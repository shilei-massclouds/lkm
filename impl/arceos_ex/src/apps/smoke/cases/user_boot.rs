use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        ext2::{EXT2_MAX_BLOCK_SIZE, EXT2_NDIR_BLOCKS},
        state::State,
        user_boot::{
            UserMappingKind, USER_INIT_EXPECTED_MESSAGE, USER_INIT_PATH, USER_PAGE_SIZE,
            USER_STACK_SIZE, USER_STACK_TOP,
        },
        virtio_blk,
    },
};

static mut USER_INIT_READ_BUFFER: [u8; USER_INIT_MAX_READ] = [0; USER_INIT_MAX_READ];
const USER_INIT_MAX_READ: usize = EXT2_MAX_BLOCK_SIZE * EXT2_NDIR_BLOCKS;
const TEMP_USER_INIT_ENTRY: usize = 0x10000;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut UserBootElfScenario::new());
    suite.result()
}

struct UserBootElfScenario;

impl UserBootElfScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for UserBootElfScenario {
    fn name(&self) -> &'static str {
        "user_boot.elf_object_from_rootfs_init"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert("rootfs online", ctx.rootfs.state() == State::Online);
        assertions.assert(
            "ext2 filesystem online",
            ctx.ext2_filesystem.state() == State::Online,
        );
        assertions.assert("vfs ready", ctx.vfs_core.state() == State::Ready);
        assertions.assert("fs_struct ready", ctx.fs_struct.state() == State::Ready);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
        let buffer = unsafe {
            let ptr = core::ptr::addr_of_mut!(USER_INIT_READ_BUFFER);
            &mut *ptr
        };
        buffer.fill(0);

        let Ok(len) = ctx.vfs_core.read_path(
            &ctx.fs_struct,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &mut provider,
            USER_INIT_PATH,
            buffer,
        ) else {
            assertions.assert("read /sbin/init", false);
            return;
        };
        let image = &buffer[..len];

        assertions.assert(
            "payload setup",
            ctx.user_boot_payload.setup(&ctx.kernel_init_task).is_ok(),
        );
        assertions.assert("elf preset", ctx.elf_object.preset_from_vfs(image).is_ok());
        assertions.assert("elf setup", ctx.elf_object.setup(image).is_ok());
        assertions.assert(
            "payload try candidate",
            ctx.user_boot_payload.try_candidate(&ctx.elf_object).is_ok(),
        );
        assertions.assert(
            "address space preset",
            ctx.user_address_space
                .preset(
                    ctx.vm.swapper_vm(),
                    &ctx.page_allocator,
                    &ctx.kernel_global_allocator,
                    &ctx.kernel_init_task,
                )
                .is_ok(),
        );
        assertions.assert(
            "user stack setup",
            ctx.user_stack
                .setup(
                    &ctx.user_address_space,
                    &mut ctx.page_allocator,
                    &ctx.page_metadata_map,
                )
                .is_ok(),
        );
        assertions.assert(
            "address space setup",
            ctx.user_address_space
                .setup(
                    &ctx.elf_object,
                    &ctx.user_stack,
                    image,
                    &mut ctx.page_allocator,
                    &ctx.page_metadata_map,
                )
                .is_ok(),
        );
        assertions.assert(
            "trap frame setup",
            ctx.user_trap_frame
                .setup(&ctx.user_address_space, &ctx.elf_object, &ctx.user_stack)
                .is_ok(),
        );
        assertions.assert(
            "address space enable",
            ctx.user_address_space
                .enable(
                    &ctx.user_trap_frame,
                    ctx.vm.swapper_vm(),
                    &ctx.kernel_image,
                    &mut ctx.page_allocator,
                    &ctx.page_metadata_map,
                )
                .is_ok(),
        );

        let elf = &ctx.elf_object;
        assertions.assert("elf ready", elf.state() == State::Ready);
        assertions.assert("elf input", elf.input_bound() && elf.input_from_vfs());
        assertions.assert(
            "elf header",
            elf.magic_valid()
                && elf.class_elf64()
                && elf.little_endian()
                && elf.machine_riscv()
                && elf.type_supported()
                && elf.static_executable(),
        );
        assertions.assert("elf no loader", elf.no_separate_loader());
        assertions.assert("elf phdr parsed", elf.program_headers_parsed());
        assertions.assert("elf load segments", elf.load_segment_count() >= 1);
        assertions.assert(
            "elf phdr count",
            elf.program_header_count() >= elf.load_segment_count(),
        );
        assertions.assert("elf load plan", elf.load_plan_bound());
        assertions.assert("elf segment perms", elf.segment_permissions_bound());
        assertions.assert(
            "elf entry bound",
            elf.entry_bound() && elf.entry() == TEMP_USER_INIT_ENTRY,
        );
        assertions.assert("elf entry executable", elf.entry_in_executable_segment());
        assertions.assert("elf init content", elf.init_content_observed());
        assertions.assert("elf bss plan", elf.bss_zero_plan_bound());
        assertions.assert("elf setup owns load", elf.load_merged_into_setup());
        assertions.assert(
            "elf direct read fit",
            elf.load_segments_fit_direct_read(USER_INIT_MAX_READ),
        );
        assertions.assert(
            "raw content",
            crate::objects::user_boot::contains_bytes(image, USER_INIT_EXPECTED_MESSAGE),
        );

        let Some(first_segment) = elf.load_segment(0) else {
            assertions.assert("first load segment", false);
            return;
        };
        assertions.assert("first segment readable", first_segment.readable());
        assertions.assert("first segment aligned", first_segment.align() == 0x1000);
        assertions.assert(
            "first segment file range",
            first_segment.file_end() <= elf.input_len(),
        );

        assertions.assert(
            "payload ready",
            ctx.user_boot_payload.state() == State::Ready,
        );
        assertions.assert("payload selected", ctx.user_boot_payload.selected());
        assertions.assert(
            "payload candidates",
            ctx.user_boot_payload.candidates_bound(),
        );
        assertions.assert(
            "payload default path",
            ctx.user_boot_payload.default_init_path_bound(),
        );
        assertions.assert("payload fs", ctx.user_boot_payload.uses_current_fs_struct());
        assertions.assert(
            "payload partition deferred",
            ctx.user_boot_payload.no_partition_dependency()
                && ctx.user_boot_payload.partition_objects_deferred(),
        );
        assertions.assert(
            "payload try candidate",
            ctx.user_boot_payload.try_candidate_bound()
                && ctx.user_boot_payload.selected_path_bound()
                && ctx.user_boot_payload.reads_init_from_vfs(),
        );
        assertions.assert(
            "payload kernel init",
            ctx.user_boot_payload.driven_by_kernel_init_task()
                && ctx.kernel_init_task.state() == State::Online,
        );

        let space = &ctx.user_address_space;
        assertions.assert("address space online", space.state() == State::Online);
        assertions.assert(
            "address space allocated",
            space.allocated() && space.first_instance(),
        );
        assertions.assert(
            "address space kernel init binding",
            space.bound_to_kernel_init_task() && ctx.kernel_init_task.state() == State::Online,
        );
        assertions.assert(
            "address space halves",
            space.low_half_private()
                && space.high_half_shares_swapper()
                && space.kernel_pages_u_disabled(),
        );
        assertions.assert(
            "address space mappings",
            space.user_pages_u_enabled()
                && space.elf_load_plan_consumed()
                && space.segment_mappings_bound()
                && space.elf_segments_mapped()
                && space.stack_mapped()
                && space.elf_mapped(),
        );
        assertions.assert(
            "address space bss",
            space.bss_zero_plan_consumed() && space.elf_bss_zeroed(),
        );
        assertions.assert(
            "address space backing",
            space.backing_pages_allocated()
                && space.elf_file_bytes_copied()
                && space.bss_bytes_zeroed()
                && space.page_table_view_ready(),
        );
        assertions.assert(
            "address space entry",
            space.entry_mapping_executable() && space.runtime_ready(),
        );
        assertions.assert(
            "address space real page table",
            space.real_page_table_allocated()
                && space.user_leaf_ptes_installed()
                && space.high_half_root_entries_shared()
                && space.satp_token_ready()
                && space.prepared_but_not_current()
                && space.satp_token() != 0
                && space.user_leaf_pte_count() >= space.mapping_count()
                && space.page_table_l0_count() >= 1,
        );
        assertions.assert(
            "address space mapping count",
            space.segment_mapping_count() == elf.load_segment_count()
                && space.mapping_count() == elf.load_segment_count() + 1,
        );

        let Some(first_mapping) = space.mapping(0) else {
            assertions.assert("first mapping", false);
            return;
        };
        assertions.assert(
            "first mapping facts",
            first_mapping.kind() == UserMappingKind::ElfSegment
                && first_mapping.vaddr() == first_segment.vaddr()
                && first_mapping.memsz() == first_segment.memsz()
                && first_mapping.filesz() == first_segment.filesz()
                && first_mapping.user_accessible()
                && first_mapping.readable()
                && first_mapping.executable()
                && first_mapping.backing_page_count() >= 1
                && first_mapping.file_bytes_copied() == first_segment.filesz()
                && first_mapping.page_table_entry_bound()
                && first_mapping.bss_zero_bytes() == first_segment.memsz() - first_segment.filesz(),
        );
        assertions.assert(
            "first mapping content",
            mapping_contains(
                first_mapping,
                &ctx.page_metadata_map,
                USER_INIT_EXPECTED_MESSAGE,
            ),
        );

        let stack = &ctx.user_stack;
        assertions.assert("user stack ready", stack.state() == State::Ready);
        assertions.assert(
            "user stack facts",
            stack.allocated()
                && stack.fixed_size_bound()
                && stack.mapped_into_address_space()
                && stack.backing_pages_allocated()
                && stack.zeroed()
                && stack.initial_sp_bound()
                && stack.minimal_arg_env_bound(),
        );
        assertions.assert(
            "user stack bounds",
            stack.size() == USER_STACK_SIZE
                && stack.top() == USER_STACK_TOP
                && stack.initial_sp() == USER_STACK_TOP
                && stack.base() + stack.size() == USER_STACK_TOP
                && stack.base() % USER_PAGE_SIZE == 0
                && stack.top() % USER_PAGE_SIZE == 0,
        );
        assertions.assert(
            "user stack pages",
            stack.backing_page_count() == USER_STACK_SIZE / USER_PAGE_SIZE
                && stack.backing_page(0).is_some(),
        );
        assertions.assert(
            "user stack zeroed",
            stack_first_page_zeroed(stack, &ctx.page_metadata_map),
        );
        let Some(stack_mapping) = space.stack_mapping() else {
            assertions.assert("stack mapping", false);
            return;
        };
        assertions.assert(
            "stack mapping facts",
            stack_mapping.kind() == UserMappingKind::Stack
                && stack_mapping.vaddr() == stack.base()
                && stack_mapping.memsz() == stack.size()
                && stack_mapping.filesz() == 0
                && stack_mapping.readable()
                && stack_mapping.writable()
                && !stack_mapping.executable()
                && stack_mapping.user_accessible(),
        );

        let trap = &ctx.user_trap_frame;
        assertions.assert("trap frame ready", trap.state() == State::Ready);
        assertions.assert(
            "trap frame facts",
            trap.allocated()
                && trap.entry_bound()
                && trap.sp_bound()
                && trap.sstatus_user_mode()
                && trap.sret_ready()
                && trap.address_space_bound()
                && trap.prepared_but_not_entered(),
        );
        assertions.assert(
            "trap frame registers",
            trap.entry() == elf.entry()
                && trap.sp() == stack.initial_sp()
                && trap.sstatus() == crate::objects::user_boot::SSTATUS_SPIE_SET,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn mapping_contains(
    mapping: crate::objects::user_boot::UserMapping,
    page_metadata_map: &crate::objects::mm_core::PageMetadataMap,
    needle: &[u8],
) -> bool {
    if needle.is_empty() {
        return true;
    }
    if mapping.filesz() < needle.len() {
        return false;
    }
    let mut offset = 0usize;
    while offset + needle.len() <= mapping.filesz() {
        let mut matched = true;
        let mut index = 0usize;
        while index < needle.len() {
            if mapping_byte_at(mapping, page_metadata_map, offset + index) != Some(needle[index]) {
                matched = false;
                break;
            }
            index += 1;
        }
        if matched {
            return true;
        }
        offset += 1;
    }
    false
}

fn mapping_byte_at(
    mapping: crate::objects::user_boot::UserMapping,
    page_metadata_map: &crate::objects::mm_core::PageMetadataMap,
    file_offset: usize,
) -> Option<u8> {
    if file_offset >= mapping.filesz() {
        return None;
    }
    let absolute = mapping.page_offset() + file_offset;
    let page_index = absolute / USER_PAGE_SIZE;
    let page_offset = absolute % USER_PAGE_SIZE;
    let page = mapping.backing_page(page_index)?;
    let linear = page_metadata_map.page_address(page)?;
    Some(unsafe { *((linear + page_offset) as *const u8) })
}

fn stack_first_page_zeroed(
    stack: &crate::objects::user_boot::UserStack,
    page_metadata_map: &crate::objects::mm_core::PageMetadataMap,
) -> bool {
    let Some(page) = stack.backing_page(0) else {
        return false;
    };
    let Some(linear) = page_metadata_map.page_address(page) else {
        return false;
    };
    let mut index = 0usize;
    while index < USER_PAGE_SIZE {
        let byte = unsafe { *((linear + index) as *const u8) };
        if byte != 0 {
            return false;
        }
        index += 1;
    }
    true
}
