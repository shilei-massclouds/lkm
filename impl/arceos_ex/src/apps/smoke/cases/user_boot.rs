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
            assertions.assert("read /init", false);
            return;
        };
        let image = &buffer[..len];

        assertions.assert("payload setup", ctx.user_boot_payload.setup().is_ok());
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
                )
                .is_ok(),
        );
        assertions.assert(
            "user stack setup",
            ctx.user_stack
                .setup(&ctx.user_address_space, &ctx.page_allocator)
                .is_ok(),
        );
        assertions.assert(
            "address space setup",
            ctx.user_address_space
                .setup(&ctx.elf_object, &ctx.user_stack)
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

        let space = &ctx.user_address_space;
        assertions.assert("address space ready", space.state() == State::Ready);
        assertions.assert("address space allocated", space.allocated());
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
            "address space entry",
            space.entry_mapping_executable() && !space.runtime_ready(),
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
                && first_mapping.bss_zero_bytes() == first_segment.memsz() - first_segment.filesz(),
        );

        let stack = &ctx.user_stack;
        assertions.assert("user stack ready", stack.state() == State::Ready);
        assertions.assert(
            "user stack facts",
            stack.allocated()
                && stack.fixed_size_bound()
                && stack.mapped_into_address_space()
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
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
