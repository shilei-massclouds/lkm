use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        files::{FdRef, FileBackendKind},
        state::State,
        user_boot::{
            ElfObjectRole, UserMappingKind, USER_BOOT_READ_MAX, USER_HEAP_BASE, USER_HEAP_SIZE,
            USER_INIT_EXPECTED_MESSAGE, USER_INIT_PATH, USER_PAGE_SIZE, USER_STACK_SIZE,
            USER_STACK_TOP,
        },
        vfs::VfsError,
        virtio_blk,
    },
};

static mut USER_INIT_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];
static mut USER_INTERPRETER_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];

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
            ctx.user_boot_payload
                .setup(&ctx.kernel_init_task, &ctx.payload_exec_sync_boundaries)
                .is_ok(),
        );
        assertions.assert("elf preset", ctx.elf_object.preset_from_vfs(image).is_ok());
        assertions.assert("elf setup", ctx.elf_object.setup(image).is_ok());
        let interpreter_image = if let Some(path) = ctx.elf_object.interpreter_path() {
            let interpreter_buffer = unsafe {
                let ptr = core::ptr::addr_of_mut!(USER_INTERPRETER_READ_BUFFER);
                &mut *ptr
            };
            interpreter_buffer.fill(0);
            let len = match ctx.vfs_core.read_path(
                &ctx.fs_struct,
                &mut ctx.ext2_filesystem,
                &mut ctx.block_device_registry,
                &mut provider,
                path,
                interpreter_buffer,
            ) {
                Ok(len) => len,
                Err(error) => {
                    assertions.assert(read_interpreter_error_label(error), false);
                    return;
                }
            };
            let image = &interpreter_buffer[..len];
            assertions.assert(
                "interpreter preset",
                ctx.elf_interpreter_object
                    .preset_interpreter_from_vfs(image)
                    .is_ok(),
            );
            assertions.assert(
                "interpreter setup",
                ctx.elf_interpreter_object.setup(image).is_ok(),
            );
            assertions.assert(
                "runtime interpreter",
                ctx.elf_object
                    .bind_runtime_interpreter(&ctx.elf_interpreter_object)
                    .is_ok(),
            );
            Some(image)
        } else {
            None
        };
        let interpreter_ref = interpreter_image.map(|_| &ctx.elf_interpreter_object);
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
                    &ctx.elf_object,
                    interpreter_ref,
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
                    interpreter_ref,
                    &ctx.user_stack,
                    image,
                    interpreter_image,
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
            "elf enable",
            ctx.elf_object
                .enable(
                    &ctx.user_address_space,
                    &ctx.user_stack,
                    &ctx.user_trap_frame,
                )
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
        assertions.assert("elf online", elf.state() == State::Online);
        assertions.assert("elf input", elf.input_bound() && elf.input_from_vfs());
        assertions.assert(
            "elf header",
            elf.magic_valid()
                && elf.class_elf64()
                && elf.little_endian()
                && elf.machine_riscv()
                && elf.type_supported()
                && (elf.static_executable() || elf.dynamic_executable()),
        );
        assertions.assert("elf main role", elf.role() == ElfObjectRole::MainExecutable);
        assertions.assert(
            "elf loader mode",
            elf.no_separate_loader()
                || (elf.interpreter_required()
                    && elf.interpreter_path_bound()
                    && interpreter_ref.is_some()),
        );
        assertions.assert("elf phdr parsed", elf.program_headers_parsed());
        assertions.assert("elf load segments", elf.load_segment_count() >= 1);
        assertions.assert(
            "elf phdr count",
            elf.program_header_count() >= elf.load_segment_count(),
        );
        assertions.assert("elf load plan", elf.load_plan_bound());
        assertions.assert("elf segment perms", elf.segment_permissions_bound());
        assertions.assert("elf entry bound", elf.entry_bound() && elf.entry() != 0);
        assertions.assert(
            "elf runtime entry",
            elf.runtime_entry_bound()
                && elf.runtime_entry() != 0
                && if let Some(interpreter) = interpreter_ref {
                    elf.runtime_entry() == interpreter.entry()
                } else {
                    elf.runtime_entry() == elf.entry()
                },
        );
        if let Some(interpreter) = interpreter_ref {
            assertions.assert(
                "interpreter facts",
                interpreter.role() == ElfObjectRole::Interpreter
                    && interpreter.state() == State::Ready
                    && interpreter.et_dyn_interpreter_supported()
                    && interpreter.load_bias() != 0
                    && interpreter.entry() >= interpreter.load_bias()
                    && interpreter.load_segment_count() >= 1,
            );
        }
        assertions.assert("elf entry executable", elf.entry_in_executable_segment());
        assertions.assert("elf init content", elf.init_content_observed());
        assertions.assert("elf bss plan", elf.bss_zero_plan_bound());
        assertions.assert("elf setup owns load", elf.load_merged_into_setup());
        assertions.assert(
            "elf direct read fit",
            elf.load_segments_fit_direct_read(USER_BOOT_READ_MAX),
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
        assertions.assert(
            "payload exec sync deferred",
            ctx.user_boot_payload.exec_sync_boundaries_ready(),
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
                && space.heap_mapped()
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
            space.segment_mapping_count()
                == elf.load_segment_count() + interpreter_ref.map_or(0, |i| i.load_segment_count())
                && space.mapping_count()
                    == elf.load_segment_count()
                        + interpreter_ref.map_or(0, |i| i.load_segment_count())
                        + 2,
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
                && stack.initial_sp() >= stack.base()
                && stack.initial_sp() < stack.top()
                && stack.initial_sp() % 16 == 0
                && stack.arg0_ptr() > stack.initial_sp()
                && stack.arg0_ptr() < stack.top()
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
        assertions.assert(
            "user stack arg0",
            stack_contains_at(
                stack,
                &ctx.page_metadata_map,
                stack.arg0_ptr(),
                b"/sbin/init\0",
            ),
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
        let Some(heap_mapping) = space.mapping(space.segment_mapping_count() + 1) else {
            assertions.assert("heap mapping", false);
            return;
        };
        assertions.assert(
            "heap mapping facts",
            heap_mapping.kind() == UserMappingKind::Heap
                && heap_mapping.vaddr() == USER_HEAP_BASE
                && heap_mapping.memsz() == USER_HEAP_SIZE
                && heap_mapping.filesz() == 0
                && heap_mapping.readable()
                && heap_mapping.writable()
                && !heap_mapping.executable()
                && heap_mapping.user_accessible()
                && space.heap_base() == USER_HEAP_BASE
                && space.heap_size() == USER_HEAP_SIZE
                && space.heap_brk() == USER_HEAP_BASE,
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
            trap.entry() == elf.runtime_entry()
                && trap.sp() == stack.initial_sp()
                && trap.sstatus() == crate::objects::user_boot::SSTATUS_SPIE_SET,
        );
        assertions.assert(
            "syscall setup",
            ctx.exception_stream
                .syscall_setup(&mut ctx.syscall_table)
                .is_ok(),
        );
        assertions.assert(
            "syscall enable",
            ctx.exception_stream
                .syscall_enable(&ctx.syscall_table)
                .is_ok(),
        );
        assertions.assert(
            "dynamic memory syscalls",
            ctx.syscall_table.brk_supported()
                && ctx.syscall_table.mmap_supported()
                && ctx.syscall_table.mprotect_supported()
                && ctx.syscall_table.munmap_supported(),
        );
        assertions.assert(
            "files struct setup",
            ctx.files_struct.setup(&ctx.kernel_init_task).is_ok(),
        );
        assertions.assert(
            "files struct ready",
            ctx.files_struct.state() == State::Ready
                && ctx.files_struct.allocated()
                && ctx.files_struct.owned_by_kernel_init_task()
                && ctx.files_struct.fd_table_bound()
                && ctx.files_struct.stdio_bound()
                && ctx.files_struct.next_fd_ready()
                && ctx.files_struct.close_on_exec_ready()
                && ctx.files_struct.shared_deferred()
                && ctx.files_struct.regular_file_slot_ready(),
        );
        assertions.assert(
            "fd table stdio",
            ctx.files_struct.fd_table().state() == State::Ready
                && ctx.files_struct.fd_table().allocated()
                && ctx.files_struct.fd_table().capacity_bound()
                && ctx.files_struct.fd_table().stdio_fds_bound()
                && ctx.files_struct.fd_bound(FdRef::Stdin)
                && ctx.files_struct.fd_bound(FdRef::Stdout)
                && ctx.files_struct.fd_bound(FdRef::Stderr),
        );
        assertions.assert(
            "stdio open file descriptions",
            ctx.files_struct.stdin().state() == State::Ready
                && ctx.files_struct.stdout().state() == State::Ready
                && ctx.files_struct.stderr().state() == State::Ready
                && ctx.files_struct.stdin().backend_bound()
                && ctx.files_struct.stdout().backend_bound()
                && ctx.files_struct.stderr().backend_bound()
                && !ctx.files_struct.stdin().writable()
                && ctx.files_struct.stdout().writable()
                && ctx.files_struct.stderr().writable()
                && ctx.files_struct.stdout().offset_ready()
                && ctx.files_struct.stderr().offset_ready(),
        );
        assertions.assert(
            "stdio char device backends",
            ctx.files_struct.stdout_backend().state() == State::Ready
                && ctx.files_struct.stderr_backend().state() == State::Ready
                && ctx.files_struct.stdout_backend().kind() == FileBackendKind::CharDevice
                && ctx.files_struct.stderr_backend().kind() == FileBackendKind::CharDevice
                && ctx
                    .files_struct
                    .stdout_backend()
                    .char_device_console_bound()
                && ctx
                    .files_struct
                    .stderr_backend()
                    .char_device_console_bound()
                && ctx
                    .files_struct
                    .stdout_backend()
                    .char_device_write_supported()
                && ctx
                    .files_struct
                    .stderr_backend()
                    .char_device_write_supported(),
        );
        let fd = match ctx.files_struct.open_regular_path(
            &ctx.fs_struct,
            &mut ctx.vfs_core,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            b"/etc/alpine-release",
        ) {
            Ok(fd) => fd,
            Err(_) => {
                assertions.assert("regular open path", false);
                return;
            }
        };
        let mut regular_buffer = [0u8; 32];
        let read_len = match ctx.files_struct.read_fd(fd, &mut regular_buffer) {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("regular read fd", false);
                return;
            }
        };
        assertions.assert(
            "regular read content",
            read_len != 0 && regular_buffer[..read_len].starts_with(b"3."),
        );
        let stat = match ctx.files_struct.stat_regular_path(
            &ctx.fs_struct,
            &mut ctx.vfs_core,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            b"/etc/alpine-release",
        ) {
            Ok(stat) => stat,
            Err(_) => {
                assertions.assert("regular stat path", false);
                return;
            }
        };
        assertions.assert(
            "regular stat metadata",
            stat.size() >= read_len && stat.mode() != 0,
        );
        assertions.assert("regular close fd", ctx.files_struct.close_fd(fd).is_ok());
        assertions.assert(
            "regular files facts",
            ctx.files_struct.open_path_routes_to_vfs()
                && ctx.files_struct.read_fd_routes_to_table()
                && ctx.files_struct.close_fd_routes_to_table()
                && ctx.files_struct.stat_path_routes_to_vfs()
                && ctx.files_struct.regular_fd_installed()
                && ctx.files_struct.regular_file_read_observed()
                && ctx.files_struct.regular_file_closed()
                && ctx.files_struct.regular_file_stat_observed()
                && ctx.files_struct.fd_table().fd_installed()
                && ctx.files_struct.fd_table().fd_closed()
                && ctx.files_struct.regular0().read_observed()
                && ctx
                    .files_struct
                    .regular0_backend()
                    .regular_file_read_returns_data()
                && ctx
                    .files_struct
                    .regular0_backend()
                    .regular_file_stat_returns_metadata(),
        );
        assertions.assert(
            "user init process setup",
            ctx.user_init_process
                .setup(
                    &ctx.kernel_init_task,
                    &ctx.user_address_space,
                    &ctx.elf_object,
                    &ctx.user_trap_frame,
                    &ctx.fs_struct,
                    &ctx.files_struct,
                )
                .is_ok(),
        );
        assertions.assert(
            "user init process enable",
            ctx.user_init_process
                .enable(
                    &ctx.user_trap_frame,
                    &ctx.exception_stream,
                    &ctx.syscall_table,
                )
                .is_ok(),
        );
        let process = &ctx.user_init_process;
        assertions.assert("user init process online", process.state() == State::Online);
        assertions.assert(
            "user init identity",
            process.reuses_kernel_init_task()
                && process.pid1_preserved()
                && process.exec_identity_handoff()
                && process.no_new_task_struct()
                && process.kernel_init_not_destroyed(),
        );
        assertions.assert(
            "user init inherited context",
            process.path_bound()
                && process.path() == crate::objects::user_boot::UserInitPathRef::DefaultInit
                && process.address_space_bound()
                && process.fs_struct_inherited()
                && process.files_struct_inherited()
                && process.trap_frame_bound(),
        );
        assertions.assert(
            "kernel init attachments",
            process.kernel_init_execve_to_user_init()
                && process.kernel_init_pid1_identity_preserved()
                && process.kernel_init_user_mm_attached()
                && process.kernel_init_user_trap_frame_attached(),
        );
        assertions.assert(
            "user init syscall context",
            process.syscall_context_bound()
                && process.trap_return_bound()
                && process.syscall_dispatch_bound()
                && process.syscall_arguments_extracted(),
        );
        assertions.assert(
            "user init runtime not entered by smoke",
            !process.user_entry_ready()
                && !process.trap_return_context_used()
                && !process.trap_return_sfence_vma_after_satp()
                && !process.trap_return_sret_handoff()
                && !process.runtime_entered()
                && !ctx.syscall_table.write_observed()
                && !ctx.syscall_table.exit_observed(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn mapping_contains(
    mapping: &crate::objects::user_boot::UserMapping,
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
    mapping: &crate::objects::user_boot::UserMapping,
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

fn stack_contains_at(
    stack: &crate::objects::user_boot::UserStack,
    page_metadata_map: &crate::objects::mm_core::PageMetadataMap,
    user_addr: usize,
    expected: &[u8],
) -> bool {
    if user_addr < stack.base()
        || user_addr
            .checked_add(expected.len())
            .filter(|end| *end <= stack.top())
            .is_none()
    {
        return false;
    }

    let mut index = 0usize;
    while index < expected.len() {
        let stack_offset = user_addr - stack.base() + index;
        let page_index = stack_offset / USER_PAGE_SIZE;
        let page_offset = stack_offset % USER_PAGE_SIZE;
        let Some(page) = stack.backing_page(page_index) else {
            return false;
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            return false;
        };
        let byte = unsafe { *((linear + page_offset) as *const u8) };
        if byte != expected[index] {
            return false;
        }
        index += 1;
    }
    true
}

fn read_interpreter_error_label(error: VfsError) -> &'static str {
    match error {
        VfsError::CoreNotReady => "read interpreter core not ready",
        VfsError::FsTypeNotReady => "read interpreter fs type not ready",
        VfsError::FsTypeAlreadyRegistered => "read interpreter fs type duplicate",
        VfsError::FsTypeMissing => "read interpreter fs type missing",
        VfsError::MountMissing => "read interpreter mount missing",
        VfsError::AlreadyMounted => "read interpreter already mounted",
        VfsError::InvalidRef => "read interpreter invalid ref",
        VfsError::InvalidName => "read interpreter invalid name",
        VfsError::NameTooLong => "read interpreter name too long",
        VfsError::NotDirectory => "read interpreter not directory",
        VfsError::NotFile => "read interpreter not file",
        VfsError::AlreadyExists => "read interpreter already exists",
        VfsError::NotFound => "read interpreter not found",
        VfsError::DirectoryNotEmpty => "read interpreter dir not empty",
        VfsError::ReadOnly => "read interpreter read only",
        VfsError::ShortBuffer => "read interpreter short buffer",
        VfsError::Backend => "read interpreter backend",
        VfsError::UnsupportedPath => "read interpreter unsupported path",
    }
}
