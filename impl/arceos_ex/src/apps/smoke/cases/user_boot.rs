use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        event_stream::{
            KERNEL_TRAP_OVERFLOW_STACK_SIZE, KERNEL_TRAP_THREAD_SHIFT, TRAP_FRAME_SIZE,
            TrapEntryOrigin, TrapFrame, formal_trap_entry_prelude, kernel_trap_frame_overflows,
            kernel_trap_overflow_stack_base, kernel_trap_overflow_stack_top,
        },
        files::{FdRef, FileBackendKind, FileError, OpenFileDescriptionRef},
        process_prepare::TaskCopyUserProcessInputs,
        state::State,
        task::TaskEntry,
        user_boot::{
            ElfObjectRole, USER_BOOT_READ_MAX, USER_CHILD_PID, USER_CLONE_SIGCHLD,
            USER_COMPLETED_CHILD_RECORD_CAPACITY, USER_HEAP_BASE, USER_HEAP_SIZE,
            USER_INIT_EXPECTED_MESSAGE, USER_PAGE_SIZE, USER_SIGCHLD_MASK,
            USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE, USER_STACK_TOP,
            USER_WAIT4_ALL_CHILDREN, UserMappingKind, UserProcessGroupLookup,
            UserProcessGroupUpdate, UserRtSigtimedwaitResult,
        },
        virtio_blk,
    },
};

const USER_BUSYBOX_INIT_VFORK_FLAGS: usize = 0x4111;
const USER_SMOKE_INIT_PATH: &[u8] = b"/opt/lkm/tests/user-smoke";
const USER_SMOKE_INIT_PATH_NUL: &[u8] = b"/opt/lkm/tests/user-smoke\0";
const USER_PLAIN_FORK_FLAGS: usize = 0x11;
const USER_WAIT4_WNOHANG: usize = 1;
const USER_EFAULT_RETURN: usize = usize::MAX - 13;
const USER_EINVAL_RETURN: usize = usize::MAX - 21;
const USER_ECHILD_RETURN: usize = usize::MAX - 9;
const USER_TEST_WAIT4_WUNTRACED: usize = 2;
const USER_TEST_O_RDWR: u32 = 0o2;
const USER_TEST_O_NONBLOCK: u32 = 0o4000;
const USER_TEST_O_LARGEFILE: u32 = 0o100000;
const USER_TEST_O_CLOEXEC: u32 = 0o2000000;
const USER_TEST_FD_CLOEXEC: u32 = 1;

static mut USER_INIT_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];
static mut USER_INTERPRETER_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];
static USER_INIT_READ_LEN: AtomicUsize = AtomicUsize::new(0);
static USER_INTERPRETER_READ_LEN: AtomicUsize = AtomicUsize::new(0);
static mut USER_INTERPRETER_PATH: [u8; 128] = [0; 128];
static USER_INTERPRETER_PATH_LEN: AtomicUsize = AtomicUsize::new(0);

#[inline(never)]
pub(super) fn stage_user_elf_images() -> bool {
    USER_INIT_READ_LEN.store(0, Ordering::Release);
    USER_INTERPRETER_READ_LEN.store(0, Ordering::Release);
    USER_INTERPRETER_PATH_LEN.store(0, Ordering::Release);
    stage_main_image() && discover_interpreter_path() && stage_interpreter_image()
}

#[inline(never)]
fn stage_main_image() -> bool {
    let ctx = context();
    let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
    let main_buffer = unsafe { &mut *core::ptr::addr_of_mut!(USER_INIT_READ_BUFFER) };
    main_buffer.fill(0);
    let Ok(main_len) = ctx.vfs_core.read_path(
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &mut provider,
        USER_SMOKE_INIT_PATH,
        main_buffer,
    ) else {
        return false;
    };
    USER_INIT_READ_LEN.store(main_len, Ordering::Release);
    true
}

#[inline(never)]
fn discover_interpreter_path() -> bool {
    let main_len = USER_INIT_READ_LEN.load(Ordering::Acquire);
    let main_buffer = unsafe { &*core::ptr::addr_of!(USER_INIT_READ_BUFFER) };
    if main_len == 0 || main_len > main_buffer.len() {
        return false;
    }
    let ctx = context();
    let mut probe_elf = crate::objects::elf_object::ElfObject::new();
    if ctx
        .binary_format_registry
        .prepare_main(&main_buffer[..main_len], &mut probe_elf)
        .is_err()
    {
        return false;
    }
    if let Some(path) = probe_elf.interpreter_path() {
        if path.len() > 128 {
            return false;
        }
        let output = unsafe { &mut *core::ptr::addr_of_mut!(USER_INTERPRETER_PATH) };
        output[..path.len()].copy_from_slice(path);
        USER_INTERPRETER_PATH_LEN.store(path.len(), Ordering::Release);
    }
    true
}

#[inline(never)]
fn stage_interpreter_image() -> bool {
    let path_len = USER_INTERPRETER_PATH_LEN.load(Ordering::Acquire);
    if path_len == 0 {
        return true;
    }
    let path_buffer = unsafe { &*core::ptr::addr_of!(USER_INTERPRETER_PATH) };
    if path_len > path_buffer.len() {
        return false;
    }
    let ctx = context();
    let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
    let interpreter_buffer = unsafe { &mut *core::ptr::addr_of_mut!(USER_INTERPRETER_READ_BUFFER) };
    interpreter_buffer.fill(0);
    let Ok(interpreter_len) = ctx.vfs_core.read_path(
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &mut provider,
        &path_buffer[..path_len],
        interpreter_buffer,
    ) else {
        return false;
    };
    USER_INTERPRETER_READ_LEN.store(interpreter_len, Ordering::Release);
    true
}

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
        "user_boot.elf_object_from_canonical_fixture"
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
        assertions.assert(
            "user ELF images staged",
            USER_INIT_READ_LEN.load(Ordering::Acquire) != 0,
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        let buffer = unsafe {
            let ptr = core::ptr::addr_of_mut!(USER_INIT_READ_BUFFER);
            &mut *ptr
        };
        let len = USER_INIT_READ_LEN.load(Ordering::Acquire);
        if len == 0 || len > buffer.len() {
            assertions.assert("staged canonical user-smoke", false);
            return;
        }
        let image = &buffer[..len];

        assertions.assert(
            "payload setup",
            ctx.user_boot_payload
                .setup(&ctx.kernel_init_task, &ctx.exec_sync_boundaries)
                .is_ok(),
        );
        assertions.assert("user child preset", ctx.user_child_process.preset().is_ok());
        assertions.assert("elf preset", ctx.elf_object.preset_from_vfs(image).is_ok());
        assertions.assert("elf setup", ctx.elf_object.setup(image).is_ok());
        let interpreter_image = if let Some(_path) = ctx.elf_object.interpreter_path() {
            let interpreter_buffer = unsafe {
                let ptr = core::ptr::addr_of_mut!(USER_INTERPRETER_READ_BUFFER);
                &mut *ptr
            };
            let len = USER_INTERPRETER_READ_LEN.load(Ordering::Acquire);
            if len == 0 || len > interpreter_buffer.len() {
                assertions.assert("staged interpreter", false);
                return;
            }
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
            ctx.user_boot_payload
                .try_candidate(
                    crate::objects::user_boot::UserInitPathRef::RequestedInit,
                    USER_SMOKE_INIT_PATH,
                    &ctx.elf_object,
                )
                .is_ok(),
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
        let init_argv = [USER_SMOKE_INIT_PATH];
        assertions.assert(
            "user stack setup",
            ctx.user_stack
                .setup(
                    &ctx.user_address_space,
                    &ctx.elf_object,
                    interpreter_ref,
                    &init_argv,
                    ctx.config.user_stack(),
                    USER_SMOKE_INIT_PATH,
                    &[0x3c; crate::objects::user_stack::USER_STACK_RANDOM_BYTES],
                    &[0; crate::objects::user_stack::USER_STACK_ASLR_BYTES],
                    crate::objects::user_stack::UserStackAuxv::root(
                        ctx.cpu_capabilities.elf_hwcap(),
                    ),
                    &mut ctx.page_allocator,
                    &ctx.page_metadata_map,
                )
                .is_ok(),
        );
        exercise_stack_argument_bounds(
            assertions,
            &ctx.user_address_space,
            &ctx.elf_object,
            interpreter_ref,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
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
                    &ctx.user_stack,
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
            "elf main pie bias",
            !elf.et_dyn_pie_main_supported()
                || (elf.main_pie_load_bias_bound()
                    && elf.load_bias() == crate::objects::user_boot::USER_MAIN_PIE_LOAD_BIAS),
        );
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
            ctx.user_boot_payload.default_init_path_bound()
                && ctx.user_boot_payload.default_init_fallback_order_bound(),
        );
        assertions.assert(
            "payload selection facts",
            ctx.user_boot_payload
                .candidate_failure_nonfatal_for_fallback()
                && ctx.user_boot_payload.first_successful_candidate_selected()
                && ctx.user_boot_payload.success_stops_fallback_chain()
                && ctx
                    .user_boot_payload
                    .success_no_return_to_startup_orchestration()
                && ctx.user_boot_payload.no_working_init_panic_terminal_bound()
                && ctx.user_boot_payload.selected_path()
                    == crate::objects::user_boot::UserInitPathRef::RequestedInit,
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
                && ctx.user_boot_payload.selected_argv0_path_bound()
                && bytes_eq(
                    ctx.user_boot_payload.selected_path_bytes(),
                    USER_SMOKE_INIT_PATH,
                )
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
                && stack.mapped_into_address_space()
                && stack.backing_pages_allocated()
                && stack.zeroed()
                && stack.initial_sp_bound()
                && stack.minimal_arg_env_bound()
                && stack.auxv_complete(),
        );
        assertions.assert(
            "user stack bounds",
            stack.size() >= ctx.config.user_stack().initial_expand()
                && stack.size() <= ctx.config.user_stack().initial_expand() + 2 * USER_PAGE_SIZE
                && stack.top() == USER_STACK_TOP
                && stack.initial_sp() >= stack.base()
                && stack.initial_sp() < stack.top()
                && stack.initial_sp().is_multiple_of(16)
                && stack.arg0_ptr() > stack.initial_sp()
                && stack.arg0_ptr() < stack.top()
                && stack.execfn_ptr() > stack.initial_sp()
                && stack.execfn_ptr() < stack.top()
                && stack.base() + stack.size() == stack.top()
                && stack.base().is_multiple_of(USER_PAGE_SIZE)
                && stack.top().is_multiple_of(USER_PAGE_SIZE),
        );
        assertions.assert(
            "user stack Linux config",
            ctx.config.user_stack().initial_expand() == crate::objects::user_stack::USER_STACK_SIZE
                && ctx.config.user_stack().rlimit_stack()
                    == crate::objects::user_stack::USER_STACK_RLIMIT
                && ctx.config.user_stack().guard_gap()
                    == crate::objects::user_stack::USER_STACK_GUARD_GAP
                && ctx.config.user_stack().stack_top_max()
                    == crate::objects::user_stack::USER_STACK_TOP_MAX
                && ctx.config.user_stack().aslr_window()
                    == crate::objects::user_stack::USER_STACK_ASLR_WINDOW
                && ctx.config.user_stack().random_bytes()
                    == crate::objects::user_stack::USER_STACK_RANDOM_BYTES,
        );
        assertions.assert(
            "user stack pages",
            stack.backing_page_count() >= 1
                && stack.backing_page_count() < stack.size() / USER_PAGE_SIZE
                && stack.backing_page(0).is_some(),
        );
        assertions.assert(
            "user stack sparse initial vma",
            stack.page_for_vaddr(stack.base()).is_none(),
        );
        assertions.assert(
            "user stack arg0",
            stack_contains_at(
                stack,
                &ctx.page_metadata_map,
                stack.arg0_ptr(),
                USER_SMOKE_INIT_PATH_NUL,
            ),
        );
        assertions.assert(
            "user stack independent execfn",
            stack.execfn_ptr() != stack.arg0_ptr()
                && stack_contains_at(
                    stack,
                    &ctx.page_metadata_map,
                    stack.execfn_ptr(),
                    USER_SMOKE_INIT_PATH_NUL,
                ),
        );
        let word = core::mem::size_of::<usize>();
        let init_argc = stack_usize_at(stack, &ctx.page_metadata_map, stack.initial_sp());
        let init_argv0 = stack_usize_at(stack, &ctx.page_metadata_map, stack.initial_sp() + word);
        let init_argv_null =
            stack_usize_at(stack, &ctx.page_metadata_map, stack.initial_sp() + 2 * word);
        let init_envp_null =
            stack_usize_at(stack, &ctx.page_metadata_map, stack.initial_sp() + 3 * word);
        assertions.assert(
            "user stack argc argv words",
            init_argc == Some(1)
                && init_argv0 == Some(stack.arg0_ptr())
                && init_argv_null == Some(0)
                && init_envp_null == Some(0),
        );
        let expected_auxv = [
            (
                crate::objects::user_stack::AT_HWCAP,
                ctx.cpu_capabilities.elf_hwcap(),
            ),
            (crate::objects::user_stack::AT_PAGESZ, USER_PAGE_SIZE),
            (crate::objects::user_stack::AT_CLKTCK, 100),
            (crate::objects::user_stack::AT_PHDR, elf.phdr_vaddr()),
            (crate::objects::user_stack::AT_PHENT, elf.phentsize()),
            (
                crate::objects::user_stack::AT_PHNUM,
                elf.program_header_count(),
            ),
            (
                crate::objects::user_stack::AT_BASE,
                interpreter_ref.map_or(0, |interp| interp.load_bias()),
            ),
            (crate::objects::user_stack::AT_FLAGS, 0),
            (crate::objects::user_stack::AT_ENTRY, elf.entry()),
            (crate::objects::user_stack::AT_UID, 0),
            (crate::objects::user_stack::AT_EUID, 0),
            (crate::objects::user_stack::AT_GID, 0),
            (crate::objects::user_stack::AT_EGID, 0),
            (crate::objects::user_stack::AT_SECURE, 0),
            (crate::objects::user_stack::AT_RANDOM, stack.random_ptr()),
            (crate::objects::user_stack::AT_EXECFN, stack.execfn_ptr()),
            (crate::objects::user_stack::AT_NULL, 0),
        ];
        let mut auxv_matches = true;
        let mut aux_index = 0usize;
        while aux_index < expected_auxv.len() {
            let entry = stack.auxv_ptr() + aux_index * 2 * word;
            auxv_matches &= stack_usize_at(stack, &ctx.page_metadata_map, entry)
                == Some(expected_auxv[aux_index].0)
                && stack_usize_at(stack, &ctx.page_metadata_map, entry + word)
                    == Some(expected_auxv[aux_index].1);
            aux_index += 1;
        }
        assertions.assert(
            "user stack Linux RISC-V auxv",
            auxv_matches
                && expected_auxv.len() == crate::objects::user_stack::USER_INITIAL_AUXV_ENTRIES
                && stack_contains_at(
                    stack,
                    &ctx.page_metadata_map,
                    stack.random_ptr(),
                    &[0x3c; crate::objects::user_stack::USER_STACK_RANDOM_BYTES],
                ),
        );
        let config = ctx.config.user_stack();
        let zero_seed = [0; crate::objects::user_stack::USER_STACK_ASLR_BYTES];
        let max_seed = [0xff; crate::objects::user_stack::USER_STACK_ASLR_BYTES];
        let zero_top = crate::objects::user_stack::select_stack_top(config, &zero_seed);
        let max_top = crate::objects::user_stack::select_stack_top(config, &max_seed);
        assertions.assert(
            "user stack ASLR endpoints and layout",
            stack.aslr_offset() == 0
                && stack.top() == USER_STACK_TOP
                && zero_top == Some((config.stack_top_max(), 0))
                && max_top
                    == Some((
                        config.stack_top_max() - config.aslr_window() + USER_PAGE_SIZE,
                        config.aslr_window() - USER_PAGE_SIZE,
                    ))
                && max_top.is_some_and(|(top, _)| {
                    top - config.rlimit_stack()
                        >= USER_HEAP_BASE + USER_HEAP_SIZE + config.guard_gap()
                }),
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
                && stack_mapping.user_accessible()
                && stack_mapping.backing_page_count() == 0
                && stack_mapping.stack_ownership_token() == stack.top(),
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
                && trap.user_fpu_initial()
                && trap.fpu_context_switch_deferred()
                && trap.sret_ready()
                && trap.address_space_bound()
                && trap.prepared_but_not_entered(),
        );
        assertions.assert(
            "trap frame registers",
            trap.entry() == elf.runtime_entry()
                && trap.sp() == stack.initial_sp()
                && trap.sstatus() == crate::objects::user_boot::USER_SSTATUS_INITIAL,
        );
        exercise_kernel_trap_overflow_contract(assertions);
        exercise_user_stack_growth(assertions, ctx);
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
            "dup3 syscall",
            ctx.syscall_table.dup3_supported() && ctx.syscall_table.dup3_routes_to_files_struct(),
        );
        assertions.assert(
            "pipe2 syscall first slice",
            ctx.syscall_table.pipe2_supported()
                && ctx.syscall_table.pipe2_routes_to_files_struct()
                && ctx.syscall_table.pipe2_flags_zero_first_slice()
                && ctx.syscall_table.pipe2_atomic_fd_usercopy_rollback()
                && ctx.syscall_table.pipe2_full_linux_model_deferred(),
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
            0,
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
        let stat = match ctx.files_struct.stat_path(
            &ctx.fs_struct,
            &mut ctx.vfs_core,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            b"/etc/alpine-release",
            false,
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
        let null_fd = match ctx.files_struct.open_null_path(
            b"/dev/null",
            USER_TEST_O_RDWR | USER_TEST_O_NONBLOCK | USER_TEST_O_LARGEFILE | USER_TEST_O_CLOEXEC,
        ) {
            Ok(fd) => fd,
            Err(_) => {
                assertions.assert("null open path", false);
                return;
            }
        };
        let Some(null_diag) = ctx.files_struct.fd_table_entry_diagnostic(null_fd) else {
            assertions.assert("null fd diagnostic", false);
            return;
        };
        let null_status_flags = match ctx.files_struct.fcntl_getfl_fd(null_fd) {
            Ok(flags) => flags,
            Err(_) => {
                assertions.assert("null fgetfl", false);
                return;
            }
        };
        let null_fd_flags = match ctx.files_struct.fcntl_getfd_fd(null_fd) {
            Ok(flags) => flags,
            Err(_) => {
                assertions.assert("null fgetfd", false);
                return;
            }
        };
        let mut null_buffer = [0u8; 4];
        let null_read = match ctx.files_struct.read_fd(null_fd, &mut null_buffer) {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("null read eof", false);
                return;
            }
        };
        let null_written = match ctx.files_struct.write_fd(null_fd, b"drop") {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("null write discard", false);
                return;
            }
        };
        let null_stat = match ctx.files_struct.fstat_fd(null_fd, &ctx.vfs_core) {
            Ok(stat) => stat,
            Err(_) => {
                assertions.assert("null fstat", false);
                return;
            }
        };
        assertions.assert(
            "null tty ioctl enotty",
            matches!(
                ctx.files_struct.ioctl_tiocgwinsz_fd(null_fd),
                Err(FileError::NotTty)
            ) && matches!(
                ctx.files_struct.ioctl_tiocgsid_fd(null_fd),
                Err(FileError::NotTty)
            ) && matches!(
                ctx.files_struct.ioctl_tiocsctty_fd(null_fd),
                Err(FileError::NotTty)
            ),
        );
        assertions.assert(
            "null fd semantics",
            null_fd == 3
                && null_diag.ofd == OpenFileDescriptionRef::Null
                && null_diag.readable
                && null_diag.writable
                && null_diag.close_on_exec
                && null_status_flags & USER_TEST_O_NONBLOCK != 0
                && null_status_flags & USER_TEST_O_LARGEFILE != 0
                && null_status_flags & USER_TEST_O_CLOEXEC == 0
                && null_fd_flags == USER_TEST_FD_CLOEXEC
                && null_read == 0
                && null_written == 4
                && null_stat.size() == 0
                && null_stat.mode() & 0o170000 == 0o020000
                && ctx.files_struct.null_fd_installed()
                && ctx.files_struct.null_device_read_eof_observed()
                && ctx.files_struct.null_device_write_discard_observed()
                && ctx.files_struct.null_device_fstat_device_node()
                && ctx.files_struct.null_device_tty_ioctl_enotty()
                && ctx.files_struct.null().read_observed()
                && ctx.files_struct.null().write_observed()
                && ctx.files_struct.null_backend().char_device_null_bound()
                && ctx
                    .files_struct
                    .null_backend()
                    .null_device_read_returns_eof()
                && ctx
                    .files_struct
                    .null_backend()
                    .null_device_write_discards_data()
                && !ctx.files_struct.null_backend().write_to_console(),
        );
        let fchmod_snapshot = match ctx.files_struct.save_parent_fd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                assertions.assert("fchown fchmod fd snapshot save", false);
                return;
            }
        };
        let metadata_set = ctx.files_struct.fchown_fd(0, 1000, 100).is_ok()
            && ctx.files_struct.fchmod_fd(0, 0o600).is_ok();
        let changed_fd0_diag = ctx.files_struct.fd_table_entry_diagnostic(0);
        let fchmod_snapshot_restored = ctx
            .files_struct
            .restore_parent_fd_snapshot(&fchmod_snapshot)
            .is_ok();
        let restored_fd0_diag = ctx.files_struct.fd_table_entry_diagnostic(0);
        assertions.assert(
            "fchown fchmod fd snapshot restore",
            metadata_set
                && changed_fd0_diag
                    .map(|entry| {
                        entry.owner_valid
                            && entry.owner_uid == 1000
                            && entry.owner_gid == 100
                            && entry.mode_override_valid
                            && entry.mode_override == 0o600
                    })
                    .unwrap_or(false)
                && fchmod_snapshot_restored
                && restored_fd0_diag
                    .map(|entry| !entry.owner_valid && !entry.mode_override_valid)
                    .unwrap_or(false),
        );
        assertions.assert(
            "fchown fchmod bad fd semantics",
            matches!(
                ctx.files_struct.fchown_fd(99, 1000, 100),
                Err(FileError::BadFd)
            ) && matches!(ctx.files_struct.fchmod_fd(99, 0o600), Err(FileError::BadFd)),
        );
        let owner_recorded = ctx.files_struct.fchown_fd(0, 1000, 100).is_ok();
        let mode_recorded = ctx.files_struct.fchmod_fd(0, 0o600).is_ok();
        let fd0_metadata = ctx.files_struct.fd_table_entry_diagnostic(0);
        let fd0_stat = ctx.files_struct.fstat_fd(0, &ctx.vfs_core);
        assertions.assert(
            "fchown fchmod fd metadata",
            owner_recorded
                && mode_recorded
                && fd0_metadata
                    .map(|entry| {
                        entry.owner_valid
                            && entry.owner_uid == 1000
                            && entry.owner_gid == 100
                            && entry.mode_override_valid
                            && entry.mode_override == 0o600
                    })
                    .unwrap_or(false)
                && fd0_stat
                    .map(|stat| stat.mode() & 0o170000 == 0o020000 && stat.mode() & 0o7777 == 0o600)
                    .unwrap_or(false)
                && ctx.files_struct.fchown_fd_routes_to_table()
                && ctx.files_struct.fchmod_fd_routes_to_table()
                && ctx.files_struct.fd_owner_recorded()
                && ctx.files_struct.fd_mode_override_recorded()
                && ctx.files_struct.fchmod_mode_visible_to_fstat(),
        );
        let dup3_snapshot = match ctx.files_struct.save_parent_fd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                assertions.assert("dup3 fd snapshot save", false);
                return;
            }
        };
        assertions.assert("dup3 close stdin", ctx.files_struct.close_fd(0).is_ok());
        let dup3_null_fd = match ctx
            .files_struct
            .open_null_path(b"/dev/null", USER_TEST_O_RDWR | USER_TEST_O_LARGEFILE)
        {
            Ok(fd) => fd,
            Err(_) => {
                assertions.assert("dup3 null fd0 open", false);
                return;
            }
        };
        let dup3_stdout = ctx.files_struct.dup3_fd(dup3_null_fd, 1, false);
        let dup3_stderr = ctx.files_struct.dup3_fd(dup3_null_fd, 2, true);
        let dup3_stdout_flags = ctx.files_struct.fcntl_getfd_fd(1);
        let dup3_stderr_flags = ctx.files_struct.fcntl_getfd_fd(2);
        let dup3_stdout_diag = ctx.files_struct.fd_table_entry_diagnostic(1);
        let dup3_stderr_diag = ctx.files_struct.fd_table_entry_diagnostic(2);
        let dup3_stdout_write = ctx.files_struct.write_fd(1, b"o");
        let dup3_stderr_write = ctx.files_struct.write_fd(2, b"e");
        assertions.assert(
            "dup3 null stdio semantics",
            dup3_null_fd == 0
                && dup3_stdout == Ok(1)
                && dup3_stderr == Ok(2)
                && dup3_stdout_flags == Ok(0)
                && dup3_stderr_flags == Ok(USER_TEST_FD_CLOEXEC)
                && dup3_stdout_diag
                    .map(|entry| {
                        entry.ofd == OpenFileDescriptionRef::Null
                            && entry.readable
                            && entry.writable
                            && !entry.close_on_exec
                    })
                    .unwrap_or(false)
                && dup3_stderr_diag
                    .map(|entry| {
                        entry.ofd == OpenFileDescriptionRef::Null
                            && entry.readable
                            && entry.writable
                            && entry.close_on_exec
                    })
                    .unwrap_or(false)
                && dup3_stdout_write == Ok(1)
                && dup3_stderr_write == Ok(1)
                && ctx.files_struct.dup3_routes_to_table()
                && ctx.files_struct.fd_table().fd_duplicated()
                && ctx.files_struct.fd_table().dup3_close_on_exec_bound(),
        );
        let dup3_capacity = ctx.files_struct.fd_table_capacity();
        assertions.assert(
            "dup3 negative fd semantics",
            matches!(
                ctx.files_struct.dup3_fd(dup3_null_fd, dup3_null_fd, false),
                Err(FileError::InvalidArgument)
            ) && matches!(
                ctx.files_struct.dup3_fd(99, 1, false),
                Err(FileError::BadFd)
            ) && matches!(
                ctx.files_struct.dup3_fd(dup3_null_fd, dup3_capacity, false),
                Err(FileError::BadFd)
            ),
        );
        assertions.assert(
            "dup3 fd snapshot restore",
            ctx.files_struct
                .restore_parent_fd_snapshot(&dup3_snapshot)
                .is_ok(),
        );
        let pipe_baseline_open = ctx.files_struct.fd_table_open_count();
        assertions.assert(
            "pipe2 invalid flags rollback",
            matches!(
                ctx.files_struct.pipe2_fd_pair(USER_TEST_O_CLOEXEC),
                Err(FileError::InvalidArgument)
            ) && ctx.files_struct.fd_table_open_count() == pipe_baseline_open
                && !ctx.files_struct.pipe_read_end_open()
                && !ctx.files_struct.pipe_write_end_open(),
        );
        let rollback_pair = match ctx.files_struct.pipe2_fd_pair(0) {
            Ok(pair) => pair,
            Err(_) => {
                assertions.assert("pipe2 efault rollback allocate", false);
                return;
            }
        };
        assertions.assert(
            "pipe2 efault rollback",
            rollback_pair == [4, 5]
                && ctx
                    .files_struct
                    .rollback_pipe2_usercopy(rollback_pair)
                    .is_ok()
                && ctx.files_struct.fd_table_open_count() == pipe_baseline_open
                && ctx.files_struct.pipe_buffer_len() == 0
                && !ctx.files_struct.pipe_read_end_open()
                && !ctx.files_struct.pipe_write_end_open()
                && ctx.files_struct.pipe_usercopy_rollback_observed(),
        );
        let emfile_snapshot = match ctx.files_struct.save_parent_fd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                assertions.assert("pipe2 emfile snapshot", false);
                return;
            }
        };
        while ctx.files_struct.fd_table_open_count() + 1 < ctx.files_struct.fd_table_capacity() {
            if ctx.files_struct.fcntl_dupfd_fd(1, 0, false).is_err() {
                assertions.assert("pipe2 emfile fill", false);
                return;
            }
        }
        let before_emfile = ctx.files_struct.fd_table_open_count();
        let pipe_emfile = ctx.files_struct.pipe2_fd_pair(0);
        assertions.assert(
            "pipe2 emfile atomic rollback",
            matches!(pipe_emfile, Err(FileError::TooManyOpenFiles))
                && ctx.files_struct.fd_table_open_count() == before_emfile
                && !ctx.files_struct.pipe_read_end_open()
                && !ctx.files_struct.pipe_write_end_open(),
        );
        assertions.assert(
            "pipe2 emfile restore",
            ctx.files_struct
                .restore_parent_fd_snapshot(&emfile_snapshot)
                .is_ok()
                && ctx.files_struct.fd_table_open_count() == pipe_baseline_open,
        );
        let pipe_pair = match ctx.files_struct.pipe2_fd_pair(0) {
            Ok(pair) => pair,
            Err(_) => {
                assertions.assert("pipe2 pair allocate", false);
                return;
            }
        };
        let pipe_read_diag = ctx.files_struct.fd_table_entry_diagnostic(pipe_pair[0]);
        let pipe_write_diag = ctx.files_struct.fd_table_entry_diagnostic(pipe_pair[1]);
        let mut pipe_roundtrip = [0u8; 4];
        let pipe_written = ctx.files_struct.write_fd(pipe_pair[1], b"pipe");
        let pipe_read = ctx.files_struct.read_fd(pipe_pair[0], &mut pipe_roundtrip);
        assertions.assert(
            "pipe2 fd pair direction roundtrip",
            pipe_pair == [4, 5]
                && pipe_read_diag
                    .map(|entry| {
                        entry.ofd == OpenFileDescriptionRef::PipeRead0
                            && entry.readable
                            && !entry.writable
                    })
                    .unwrap_or(false)
                && pipe_write_diag
                    .map(|entry| {
                        entry.ofd == OpenFileDescriptionRef::PipeWrite0
                            && !entry.readable
                            && entry.writable
                    })
                    .unwrap_or(false)
                && matches!(
                    ctx.files_struct.write_fd(pipe_pair[0], b"x"),
                    Err(FileError::NotWritable)
                )
                && matches!(
                    ctx.files_struct.read_fd(pipe_pair[1], &mut pipe_roundtrip),
                    Err(FileError::NotReadable)
                )
                && pipe_written == Ok(4)
                && pipe_read == Ok(4)
                && pipe_roundtrip == *b"pipe"
                && ctx.files_struct.pipe_pair_installed(),
        );
        let pipe_parent_snapshot = match ctx.files_struct.save_parent_fd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                assertions.assert("pipe2 snapshot save", false);
                return;
            }
        };
        let child_pipe_handoff = ctx.files_struct.dup3_fd(pipe_pair[1], 1, false).is_ok()
            && ctx.files_struct.close_fd(pipe_pair[0]).is_ok()
            && ctx.files_struct.close_fd(pipe_pair[1]).is_ok()
            && ctx.files_struct.write_fd(1, b"/\n") == Ok(2)
            && ctx.files_struct.close_fd(1).is_ok()
            && ctx.files_struct.pipe_buffer_len() == 2;
        let parent_pipe_restored = ctx
            .files_struct
            .restore_parent_fd_snapshot(&pipe_parent_snapshot)
            .is_ok();
        let parent_writer_closed = ctx.files_struct.close_fd(pipe_pair[1]).is_ok();
        let parent_pipe_ends_restored =
            ctx.files_struct.pipe_read_end_open() && !ctx.files_struct.pipe_write_end_open();
        let mut snapshot_pipe_data = [0u8; 2];
        let snapshot_pipe_read = ctx
            .files_struct
            .read_fd(pipe_pair[0], &mut snapshot_pipe_data);
        let snapshot_pipe_eof = ctx
            .files_struct
            .read_fd(pipe_pair[0], &mut snapshot_pipe_data);
        let parent_reader_closed = ctx.files_struct.close_fd(pipe_pair[0]).is_ok();
        assertions.assert(
            "pipe2 child dup close writes shared data",
            child_pipe_handoff,
        );
        assertions.assert(
            "pipe2 parent snapshot restores ends",
            parent_pipe_restored && parent_writer_closed && parent_pipe_ends_restored,
        );
        assertions.assert(
            "pipe2 parent reads shared data then eof",
            snapshot_pipe_read == Ok(2)
                && snapshot_pipe_data == *b"/\n"
                && snapshot_pipe_eof == Ok(0)
                && parent_reader_closed
                && ctx.files_struct.pipe_buffer_len() == 0
                && !ctx.files_struct.pipe_read_end_open()
                && !ctx.files_struct.pipe_write_end_open(),
        );
        let pipe_syscall_open_baseline = ctx.files_struct.fd_table_open_count();
        let mut pipe_efault_frame = TrapFrame::zeroed();
        pipe_efault_frame.set_reg(10, 0);
        pipe_efault_frame.set_reg(11, 0);
        pipe_efault_frame.set_reg(17, 59);
        ctx.syscall_table.pipe2(&mut pipe_efault_frame);
        assertions.assert(
            "pipe2 syscall efault rolls back pair",
            pipe_efault_frame.reg(10) == USER_EFAULT_RETURN
                && ctx.files_struct.fd_table_open_count() == pipe_syscall_open_baseline
                && !ctx.files_struct.pipe_read_end_open()
                && !ctx.files_struct.pipe_write_end_open(),
        );
        let mut pipe_einval_frame = TrapFrame::zeroed();
        pipe_einval_frame.set_reg(10, 0);
        pipe_einval_frame.set_reg(11, USER_TEST_O_CLOEXEC as usize);
        pipe_einval_frame.set_reg(17, 59);
        ctx.syscall_table.pipe2(&mut pipe_einval_frame);
        assertions.assert(
            "pipe2 syscall rejects nonzero flags",
            pipe_einval_frame.reg(10) == USER_EINVAL_RETURN
                && ctx.files_struct.fd_table_open_count() == pipe_syscall_open_baseline,
        );
        assertions.assert("null close fd", ctx.files_struct.close_fd(null_fd).is_ok());
        assertions.assert(
            "invalid close badfd",
            matches!(ctx.files_struct.close_fd(usize::MAX), Err(FileError::BadFd)),
        );
        assertions.assert("stdio close fd0", ctx.files_struct.close_fd(0).is_ok());
        assertions.assert(
            "stdio fd0 fgetfl badfd",
            matches!(ctx.files_struct.fcntl_getfl_fd(0), Err(FileError::BadFd)),
        );
        let tty_fd = match ctx.files_struct.open_tty_path(
            b"/dev/tty1",
            USER_TEST_O_RDWR | USER_TEST_O_NONBLOCK | USER_TEST_O_LARGEFILE | USER_TEST_O_CLOEXEC,
        ) {
            Ok(fd) => fd,
            Err(_) => {
                assertions.assert("tty reopen fd0", false);
                return;
            }
        };
        let tty_status_flags = match ctx.files_struct.fcntl_getfl_fd(tty_fd) {
            Ok(flags) => flags,
            Err(_) => {
                assertions.assert("tty fgetfl after reopen", false);
                return;
            }
        };
        assertions.assert(
            "tty fd0 reuses stdio slot",
            tty_fd == 0
                && tty_status_flags & USER_TEST_O_NONBLOCK != 0
                && tty_status_flags & USER_TEST_O_CLOEXEC == 0,
        );
        assertions.assert(
            "tty fd0 cloexec bit",
            ctx.files_struct.fcntl_getfd_fd(tty_fd) == Ok(USER_TEST_FD_CLOEXEC),
        );
        let parent_fd_snapshot = match ctx.files_struct.save_parent_fd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                assertions.assert("parent fd snapshot save", false);
                return;
            }
        };
        let child_close_on_exec = match ctx.files_struct.close_on_exec() {
            Ok(report) => report,
            Err(_) => {
                assertions.assert("child close on exec scan", false);
                return;
            }
        };
        let parent_fd_restored = ctx
            .files_struct
            .restore_parent_fd_snapshot(&parent_fd_snapshot)
            .is_ok();
        assertions.assert(
            "child close on exec rollback keeps parent tty fd",
            child_close_on_exec.scanned == ctx.files_struct.fd_table_capacity()
                && child_close_on_exec.closed == 1
                && child_close_on_exec.first_closed_fd == tty_fd
                && child_close_on_exec.remaining_open == 2
                && parent_fd_restored
                && ctx.files_struct.parent_fd_snapshot_saved()
                && ctx.files_struct.parent_fd_snapshot_restored()
                && ctx.files_struct.close_on_exec_observed()
                && ctx.files_struct.close_on_exec_report().closed == 1
                && ctx.files_struct.fcntl_getfl_fd(tty_fd).is_ok()
                && ctx.files_struct.fcntl_getfl_fd(1).is_ok()
                && ctx.files_struct.fcntl_getfl_fd(2).is_ok(),
        );
        let close_on_exec = match ctx.files_struct.close_on_exec() {
            Ok(report) => report,
            Err(_) => {
                assertions.assert("close on exec scan", false);
                return;
            }
        };
        assertions.assert(
            "close on exec closes tty fd",
            close_on_exec.scanned == ctx.files_struct.fd_table_capacity()
                && close_on_exec.closed == 1
                && close_on_exec.first_closed_fd == tty_fd
                && close_on_exec.remaining_open == 2
                && matches!(
                    ctx.files_struct.fcntl_getfl_fd(tty_fd),
                    Err(FileError::BadFd)
                )
                && ctx.files_struct.fcntl_getfl_fd(1).is_ok()
                && ctx.files_struct.fcntl_getfl_fd(2).is_ok(),
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
                    crate::objects::user_boot::UserInitPathRef::DefaultInit,
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
        let pid1_pid = ctx.user_init_process.read_pid(false);
        let pid1_sid = ctx.user_init_process.read_session_id(0, false);
        let pid1_explicit_sid = ctx.user_init_process.read_session_id(1, false);
        let pid1_setsid = ctx.user_init_process.set_session_id_first_slice(false);
        let child_visible = ctx
            .user_init_process
            .observe_child_process_group_visible(USER_CHILD_PID);
        let child_pid = ctx.user_init_process.read_pid(true);
        let child_initial_sid = ctx.user_init_process.read_session_id(0, true);
        let child_setsid = ctx.user_init_process.set_session_id_first_slice(true);
        let child_tty_sid_before_ctty = ctx.user_init_process.read_tty_session_id_first_slice(true);
        let child_tiocsctty = ctx
            .user_init_process
            .bind_controlling_tty_first_slice(true, 1);
        let child_tty_sid = ctx.user_init_process.read_tty_session_id_first_slice(true);
        let repeat_child_tiocsctty = ctx
            .user_init_process
            .bind_controlling_tty_first_slice(true, 1);
        let pid1_tty_sid = ctx.user_init_process.read_tty_session_id_first_slice(false);
        let pid1_tiocsctty = ctx
            .user_init_process
            .bind_controlling_tty_first_slice(false, 1);
        let child_current_sid = ctx.user_init_process.read_session_id(0, true);
        let child_explicit_sid = ctx.user_init_process.read_session_id(USER_CHILD_PID, false);
        let child_current_pgrp = ctx.user_init_process.read_process_group(0, true);
        let repeat_child_setsid = ctx.user_init_process.set_session_id_first_slice(true);
        let unknown_sid = ctx.user_init_process.read_session_id(999, false);
        assertions.assert(
            "process identity getsid and child setsid first slice",
            pid1_pid == Some(1)
                && pid1_sid == UserProcessGroupLookup::Found(1)
                && pid1_explicit_sid == UserProcessGroupLookup::Found(1)
                && pid1_setsid == UserProcessGroupUpdate::PermissionDenied
                && ctx.user_init_process.setsid_eperm_observed()
                && child_visible
                && child_pid == Some(USER_CHILD_PID)
                && child_initial_sid == UserProcessGroupLookup::Found(1)
                && child_setsid == UserProcessGroupUpdate::Updated(USER_CHILD_PID)
                && child_tty_sid_before_ctty == UserProcessGroupLookup::NotReady
                && child_tiocsctty == UserProcessGroupUpdate::Updated(USER_CHILD_PID)
                && child_tty_sid == UserProcessGroupLookup::Found(USER_CHILD_PID)
                && repeat_child_tiocsctty == UserProcessGroupUpdate::PermissionDenied
                && pid1_tty_sid == UserProcessGroupLookup::Found(1)
                && pid1_tiocsctty == UserProcessGroupUpdate::PermissionDenied
                && child_current_sid == UserProcessGroupLookup::Found(USER_CHILD_PID)
                && child_explicit_sid == UserProcessGroupLookup::Found(USER_CHILD_PID)
                && child_current_pgrp == UserProcessGroupLookup::Found(USER_CHILD_PID)
                && repeat_child_setsid == UserProcessGroupUpdate::PermissionDenied
                && unknown_sid == UserProcessGroupLookup::NoSuchProcess
                && ctx.user_init_process.child_process_group() == USER_CHILD_PID
                && ctx.user_init_process.child_process_session_id() == USER_CHILD_PID
                && ctx.user_init_process.child_session_leader_first_slice()
                && ctx.user_init_process.child_setsid_success_observed()
                && ctx
                    .user_init_process
                    .child_controlling_tty_cleared_on_setsid()
                && ctx.user_init_process.child_controlling_tty_bound()
                && ctx.user_init_process.foreground_pgrp() == USER_CHILD_PID
                && ctx.user_init_process.tty_session_id_read_observed()
                && ctx.user_init_process.tiocsctty_observed()
                && ctx.user_init_process.session_id_read_observed(),
        );
        assertions.assert(
            "sigchld pending from child exit",
            ctx.user_init_process.record_child_exit_sigchld() == Some(false)
                && ctx.user_init_process.pending_sigchld(),
        );
        let mut immediate_wait_frame = TrapFrame::zeroed();
        immediate_wait_frame.sepc = 0x1000;
        let immediate = ctx.user_init_process.begin_rt_sigtimedwait(
            USER_SIGCHLD_MASK,
            true,
            true,
            &immediate_wait_frame,
        );
        assertions.assert(
            "rt_sigtimedwait consumes pending sigchld",
            immediate == UserRtSigtimedwaitResult::ReturnSignal(USER_CLONE_SIGCHLD)
                && !ctx.user_init_process.pending_sigchld()
                && ctx.user_init_process.rt_sigtimedwait_dequeued_signal() == USER_CLONE_SIGCHLD
                && ctx.user_init_process.rt_sigtimedwait_return_signal() == USER_CLONE_SIGCHLD,
        );
        let mut sleeping_wait_frame = TrapFrame::zeroed();
        sleeping_wait_frame.sepc = 0x2000;
        sleeping_wait_frame.set_reg(17, 137);
        let sleeping = ctx.user_init_process.begin_rt_sigtimedwait(
            USER_SIGCHLD_MASK,
            true,
            true,
            &sleeping_wait_frame,
        );
        assertions.assert(
            "rt_sigtimedwait sleep waiter",
            sleeping == UserRtSigtimedwaitResult::Sleep
                && ctx.user_init_process.rt_sigtimedwait_sleeping()
                && ctx.user_init_process.rt_sigtimedwait_waiter_enqueued()
                && ctx.user_init_process.rt_sigtimedwait_saved_frame_bound()
                && ctx.user_init_process.rt_sigtimedwait_sleep_reason()
                    == USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE
                && !ctx.user_init_process.pending_sigchld(),
        );
        assertions.assert(
            "sigchld wakes waiting rt_sigtimedwait",
            ctx.user_init_process.record_child_exit_sigchld() == Some(true)
                && !ctx.user_init_process.rt_sigtimedwait_sleeping()
                && ctx.user_init_process.rt_sigtimedwait_waiter_finished()
                && ctx
                    .user_init_process
                    .rt_sigtimedwait_wake_sigchld_committed()
                && ctx.user_init_process.rt_sigtimedwait_wake_signal() == USER_CLONE_SIGCHLD
                && ctx.user_init_process.rt_sigtimedwait_dequeued_signal() == USER_CLONE_SIGCHLD,
        );
        let mut resumed_wait_frame = TrapFrame::zeroed();
        let resumed_signal = ctx
            .user_init_process
            .complete_rt_sigtimedwait_wake(&mut resumed_wait_frame);
        assertions.assert(
            "rt_sigtimedwait wake frame returns sigchld",
            resumed_signal == Some(USER_CLONE_SIGCHLD) && resumed_wait_frame.sepc == 0x2000,
        );
        exercise_completed_child_record_reuse(assertions);
        exercise_nested_vfork_child_slot(assertions);
        exercise_observed_child_plain_fork(assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn exercise_kernel_trap_overflow_contract(assertions: &mut SmokeAssertions) {
    const ALIGNED_STACK_BASE: usize = 0x8000_0000;
    const THREAD_SIZE: usize = 1 << KERNEL_TRAP_THREAD_SHIFT;

    let stack_top = ALIGNED_STACK_BASE + THREAD_SIZE;
    let lowest_legal_sp = ALIGNED_STACK_BASE + TRAP_FRAME_SIZE;
    let first_guard_sp = lowest_legal_sp - 1;
    let adjacent_half_sp = stack_top + TRAP_FRAME_SIZE;
    assertions.assert(
        "kernel trap frame size and overflow stack layout",
        core::mem::size_of::<TrapFrame>() == TRAP_FRAME_SIZE
            && TRAP_FRAME_SIZE == 288
            && KERNEL_TRAP_THREAD_SHIFT == 14
            && kernel_trap_overflow_stack_base().is_multiple_of(16)
            && kernel_trap_overflow_stack_top()
                == kernel_trap_overflow_stack_base() + KERNEL_TRAP_OVERFLOW_STACK_SIZE,
    );
    assertions.assert(
        "kernel trap VMAP overflow classification endpoints",
        !kernel_trap_frame_overflows(stack_top)
            && !kernel_trap_frame_overflows(lowest_legal_sp)
            && kernel_trap_frame_overflows(first_guard_sp)
            && kernel_trap_frame_overflows(adjacent_half_sp),
    );

    let kernel_entry = formal_trap_entry_prelude(lowest_legal_sp, 0);
    let kernel_overflow_entry = formal_trap_entry_prelude(first_guard_sp, 0);
    let user_entry = formal_trap_entry_prelude(first_guard_sp, stack_top);
    assertions.assert(
        "kernel trap prelude restores sp and sscratch",
        kernel_entry.origin == TrapEntryOrigin::Kernel
            && kernel_entry.stack_pointer == lowest_legal_sp
            && kernel_entry.scratch == 0
            && kernel_entry.early_check_performed
            && !kernel_entry.overflow
            && kernel_overflow_entry.origin == TrapEntryOrigin::Kernel
            && kernel_overflow_entry.stack_pointer == first_guard_sp
            && kernel_overflow_entry.scratch == 0
            && kernel_overflow_entry.early_check_performed
            && kernel_overflow_entry.overflow,
    );
    assertions.assert(
        "user trap prelude bypasses overflow bit test",
        kernel_trap_frame_overflows(first_guard_sp)
            && user_entry.origin == TrapEntryOrigin::User
            && user_entry.stack_pointer == stack_top
            && user_entry.scratch == first_guard_sp
            && !user_entry.early_check_performed
            && !user_entry.overflow,
    );

    let mut regs = [0usize; 32];
    let mut index = 0usize;
    while index < regs.len() {
        regs[index] = 0x1000 + index;
        index += 1;
    }
    let original_t6 = regs[31];
    let frame =
        TrapFrame::from_kernel_stack_overflow(regs, first_guard_sp, 0x2000, 0x3000, 0x4000, 0x5000);
    let mut complete = frame.reg(0) == 0
        && frame.reg(2) == first_guard_sp
        && frame.reg(31) == original_t6
        && frame.sstatus == 0x2000
        && frame.sepc == 0x3000
        && frame.scause == 0x4000
        && frame.stval == 0x5000;
    index = 1;
    while index < 32 {
        if index != 2 {
            complete &= frame.reg(index) == regs[index];
        }
        index += 1;
    }
    assertions.assert("kernel trap overflow frame complete", complete);
}

#[inline(never)]
fn exercise_user_stack_growth(assertions: &mut SmokeAssertions, ctx: &mut crate::context::Context) {
    use crate::objects::{user_boot::UserFaultAccess, user_stack::UserStackGrowReject};

    let satp = ctx.user_address_space.satp_token();
    let initial_base = ctx.user_stack.base();
    let initial_pages = ctx.user_stack.backing_page_count();
    let inside = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        initial_base,
        UserFaultAccess::Load,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack VMA demand page",
        inside.is_ok_and(|resolution| {
            resolution.fault_page() == initial_base
                && resolution.old_base() == initial_base
                && resolution.new_base() == initial_base
                && !resolution.expanded()
        }) && ctx.user_stack.base() == initial_base
            && ctx.user_stack.backing_page_count() == initial_pages + 1,
    );

    let pages_before_jump = ctx.user_stack.backing_page_count();
    let jump_addr = initial_base - 4 * USER_PAGE_SIZE;
    let jump = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        jump_addr,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack sparse multi-page growth",
        jump.is_ok_and(|resolution| resolution.expanded() && resolution.new_base() == jump_addr)
            && ctx.user_stack.base() == jump_addr
            && ctx.user_stack.backing_page_count() == pages_before_jump + 1
            && ctx
                .user_stack
                .page_for_vaddr(initial_base - USER_PAGE_SIZE)
                .is_none(),
    );

    let l0_before = ctx.user_address_space.page_table_l0_count();
    let boundary_addr = (ctx.user_stack.base() & !((2 * 1024 * 1024) - 1)) - USER_PAGE_SIZE;
    let boundary = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        boundary_addr,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack 2MiB page-table growth",
        boundary.is_ok()
            && ctx.user_stack.base() == boundary_addr
            && ctx.user_address_space.page_table_l0_count() == l0_before + 1,
    );

    let base_before_reject = ctx.user_stack.base();
    let pages_before_reject = ctx.user_stack.backing_page_count();
    let free_before_reject = ctx.page_allocator.buddy_total_free_pages();
    let below_rlimit = ctx.user_stack.rlimit_base() - USER_PAGE_SIZE;
    let rlimit = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        below_rlimit,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack rlimit rejection rollback",
        rlimit == Err(UserStackGrowReject::Rlimit)
            && ctx.user_stack.base() == base_before_reject
            && ctx.user_stack.backing_page_count() == pages_before_reject
            && ctx.page_allocator.buddy_total_free_pages() == free_before_reject,
    );

    let base_before_alloc_fail = ctx.user_stack.base();
    let pages_before_alloc_fail = ctx.user_stack.backing_page_count();
    let free_before_alloc_fail = ctx.page_allocator.buddy_total_free_pages();
    ctx.user_stack.smoke_fail_next_backing_allocation();
    let alloc_fail_addr = base_before_alloc_fail - USER_PAGE_SIZE;
    let alloc_fail = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        alloc_fail_addr,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack backing failure rollback",
        alloc_fail == Err(UserStackGrowReject::BackingAllocation)
            && ctx.user_stack.base() == base_before_alloc_fail
            && ctx.user_stack.backing_page_count() == pages_before_alloc_fail
            && ctx.page_allocator.buddy_total_free_pages() == free_before_alloc_fail,
    );

    let pte_fail_addr = base_before_alloc_fail + USER_PAGE_SIZE;
    let pages_before_pte_fail = ctx.user_stack.backing_page_count();
    let free_before_pte_fail = ctx.page_allocator.buddy_total_free_pages();
    let l0_before_pte_fail = ctx.user_address_space.page_table_l0_count();
    ctx.user_address_space.smoke_fail_next_stack_pte_install();
    let pte_fail = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        pte_fail_addr,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack PTE failure rollback",
        pte_fail == Err(UserStackGrowReject::PteInstall)
            && ctx.user_stack.base() == base_before_alloc_fail
            && ctx.user_stack.page_for_vaddr(pte_fail_addr).is_none()
            && ctx.user_stack.backing_page_count() == pages_before_pte_fail
            && ctx.user_address_space.page_table_l0_count() == l0_before_pte_fail
            && ctx.page_allocator.buddy_total_free_pages() == free_before_pte_fail,
    );

    let guard_base = ctx.user_stack.base();
    let guard_end = guard_base - ctx.user_stack.config().guard_gap() + USER_PAGE_SIZE;
    let old_heap_vaddr = ctx.user_address_space.smoke_set_heap_mapping_end(guard_end);
    let guard = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        guard_base - USER_PAGE_SIZE,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack guard-gap rejection",
        old_heap_vaddr.is_some()
            && guard == Err(UserStackGrowReject::GuardGap)
            && ctx.user_stack.base() == guard_base,
    );
    if let Some(old_heap_vaddr) = old_heap_vaddr {
        ctx.user_address_space
            .smoke_restore_heap_mapping_vaddr(old_heap_vaddr);
    }

    let collision_end = guard_base + USER_PAGE_SIZE;
    let old_heap_vaddr = ctx
        .user_address_space
        .smoke_set_heap_mapping_end(collision_end);
    let collision = ctx.user_address_space.resolve_user_stack_fault(
        &mut ctx.user_stack,
        guard_base - USER_PAGE_SIZE,
        UserFaultAccess::Store,
        satp,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "user stack adjacent mapping collision",
        old_heap_vaddr.is_some()
            && collision == Err(UserStackGrowReject::MappingCollision)
            && ctx.user_stack.base() == guard_base,
    );
    if let Some(old_heap_vaddr) = old_heap_vaddr {
        ctx.user_address_space
            .smoke_restore_heap_mapping_vaddr(old_heap_vaddr);
    }

    let nx = ctx
        .user_address_space
        .fault_mapping_diagnostic(ctx.user_stack.top() - 1, UserFaultAccess::Instruction);
    assertions.assert(
        "user stack NX instruction fault",
        nx.mapped()
            && nx.kind() == UserMappingKind::Stack
            && !nx.executable()
            && !nx.permission_satisfied(),
    );
}

#[inline(never)]
fn exercise_stack_argument_bounds(
    assertions: &mut SmokeAssertions,
    address_space: &crate::objects::user_boot::UserAddressSpace,
    elf: &crate::objects::elf_object::ElfObject,
    interpreter: Option<&crate::objects::elf_object::ElfObject>,
    page_allocator: &mut crate::objects::mm_core::PageAllocator,
    page_metadata_map: &crate::objects::mm_core::PageMetadataMap,
) {
    let word = core::mem::size_of::<usize>();
    let getty_argv: [&[u8]; 3] = [b"/sbin/getty", b"38400", b"tty1"];
    let mut getty_stack = crate::objects::user_stack::UserStack::new();
    let getty_stack_ready = getty_stack
        .setup(
            address_space,
            elf,
            interpreter,
            &getty_argv,
            crate::objects::config::UserStackConfig::linux_default(),
            b"/usr/libexec/getty",
            &[0xa5; crate::objects::user_stack::USER_STACK_RANDOM_BYTES],
            &[0xff; crate::objects::user_stack::USER_STACK_ASLR_BYTES],
            crate::objects::user_stack::UserStackAuxv::new(0x55aa, 1000, 1001, 1002, 1003),
            page_allocator,
            page_metadata_map,
        )
        .is_ok();
    let getty_sp = getty_stack.initial_sp();
    let getty_argc = stack_usize_at(&getty_stack, page_metadata_map, getty_sp);
    let getty_argv0 = stack_usize_at(&getty_stack, page_metadata_map, getty_sp + word);
    let getty_argv1 = stack_usize_at(&getty_stack, page_metadata_map, getty_sp + 2 * word);
    let getty_argv2 = stack_usize_at(&getty_stack, page_metadata_map, getty_sp + 3 * word);
    let getty_argv_null = stack_usize_at(&getty_stack, page_metadata_map, getty_sp + 4 * word);
    let getty_envp_null = stack_usize_at(&getty_stack, page_metadata_map, getty_sp + 5 * word);
    assertions.assert(
        "user stack bounded argv words",
        getty_stack_ready
            && getty_argc == Some(3)
            && getty_argv0 == Some(getty_stack.arg0_ptr())
            && getty_argv1.is_some_and(|ptr| {
                stack_contains_at(&getty_stack, page_metadata_map, ptr, b"38400\0")
            })
            && getty_argv2.is_some_and(|ptr| {
                stack_contains_at(&getty_stack, page_metadata_map, ptr, b"tty1\0")
            })
            && getty_argv_null == Some(0)
            && getty_envp_null == Some(0),
    );
    assertions.assert(
        "user stack bounded argv strings",
        getty_argv0.is_some_and(|ptr| {
            stack_contains_at(&getty_stack, page_metadata_map, ptr, b"/sbin/getty\0")
        }) && getty_stack.execfn_ptr() != getty_stack.arg0_ptr()
            && stack_contains_at(
                &getty_stack,
                page_metadata_map,
                getty_stack.execfn_ptr(),
                b"/usr/libexec/getty\0",
            )
            && getty_stack.aslr_offset()
                == crate::objects::user_stack::USER_STACK_ASLR_WINDOW - USER_PAGE_SIZE
            && getty_stack.top()
                == crate::objects::user_stack::USER_STACK_TOP_MAX
                    - crate::objects::user_stack::USER_STACK_ASLR_WINDOW
                    + USER_PAGE_SIZE,
    );
    getty_stack.release_exec_backing(page_allocator, page_metadata_map);
}

fn exercise_completed_child_record_reuse(assertions: &mut SmokeAssertions) {
    let mut index = 0usize;
    while index <= USER_COMPLETED_CHILD_RECORD_CAPACITY {
        let Some(child_pid) = archive_completed_vfork_child(index) else {
            assertions.assert("archive sequential vfork completed record", false);
            return;
        };

        if index == 0 {
            let mut sigwait_frame = TrapFrame::zeroed();
            sigwait_frame.sepc = 0x3000;
            let signal_consumed_without_release = {
                let ctx = context();
                let signal_recorded = ctx.user_init_process.record_child_exit_sigchld();
                let signal_result = ctx.user_init_process.begin_rt_sigtimedwait(
                    USER_SIGCHLD_MASK,
                    true,
                    true,
                    &sigwait_frame,
                );
                signal_recorded == Some(false)
                    && signal_result == UserRtSigtimedwaitResult::ReturnSignal(USER_CLONE_SIGCHLD)
                    && ctx.user_child_process.completed_child_record_count() == 1
                    && ctx
                        .user_child_process
                        .first_unreaped_completed_child()
                        .map(|record| record.0)
                        == Some(child_pid)
            };
            assertions.assert(
                "rt_sigtimedwait does not release completed record",
                signal_consumed_without_release,
            );

            let fault_ret = wait4_completed_record(usize::MAX, 0);
            let ctx = context();
            assertions.assert(
                "wait4 EFAULT keeps completed record unreleased",
                fault_ret == USER_EFAULT_RETURN
                    && ctx.user_child_process.completed_child_record_count() == 1
                    && ctx.user_child_process.completed_child_record_reaped_count() == 0
                    && ctx
                        .user_child_process
                        .completed_child_record_released_count()
                        == 0
                    && ctx
                        .user_child_process
                        .first_unreaped_completed_child()
                        .map(|record| record.0)
                        == Some(child_pid),
            );
        }

        let wait_ret = wait4_completed_record(0, 0);
        let ctx = context();
        assertions.assert(
            "wait4 releases completed record slot",
            wait_ret == child_pid
                && ctx.user_child_process.completed_child_record_count() == 0
                && ctx.user_child_process.completed_child_record_free_count()
                    == USER_COMPLETED_CHILD_RECORD_CAPACITY
                && ctx.user_child_process.last_reaped_child_pid() == child_pid
                && ctx.user_child_process.last_released_child_pid() == child_pid,
        );

        index += 1;
    }

    let ctx = context();
    assertions.assert(
        "sequential vfork exceeds completed record capacity",
        ctx.user_child_process
            .completed_child_record_total_archived()
            > USER_COMPLETED_CHILD_RECORD_CAPACITY
            && ctx
                .user_child_process
                .completed_child_record_released_count()
                > USER_COMPLETED_CHILD_RECORD_CAPACITY
            && ctx.user_child_process.next_child_pid()
                > USER_CHILD_PID + USER_COMPLETED_CHILD_RECORD_CAPACITY,
    );

    let no_child_ret = wait4_completed_record(0, USER_WAIT4_WNOHANG);
    let no_child_blocking_ret = wait4_completed_record(0, 0);
    let no_child_combo_ret =
        wait4_completed_record(0, USER_WAIT4_WNOHANG | USER_TEST_WAIT4_WUNTRACED);
    let ctx = context();
    assertions.assert(
        "wait4 no completed child returns ECHILD",
        no_child_ret == USER_ECHILD_RETURN
            && no_child_blocking_ret == USER_ECHILD_RETURN
            && no_child_combo_ret == USER_ECHILD_RETURN
            && ctx.user_child_process.completed_child_record_count() == 0
            && ctx
                .user_child_process
                .first_unreaped_completed_child()
                .is_none(),
    );
}

fn archive_completed_vfork_child(index: usize) -> Option<usize> {
    let mut parent_frame = TrapFrame::zeroed();
    parent_frame.sepc = 0x4000 + index * 4;
    parent_frame.set_reg(2, USER_STACK_TOP - 0x100 - index * 16);
    parent_frame.set_reg(10, USER_BUSYBOX_INIT_VFORK_FLAGS);
    parent_frame.set_reg(11, USER_STACK_TOP - 0x200 - index * 16);
    parent_frame.set_reg(17, 220);

    {
        let ctx = context();
        ctx.user_child_process.copy_vfork_from_parent(
            &ctx.user_init_process,
            &ctx.user_clone_deferred_boundaries,
            &ctx.user_address_space,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &parent_frame,
            USER_BUSYBOX_INIT_VFORK_FLAGS,
            USER_STACK_TOP - 0x200 - index * 16,
            usize::MAX,
            false,
            true,
            true,
            true,
            true,
        )?;
    }

    let child_pid = {
        let ctx = context();
        let (_parent_frame, child_pid) = ctx.user_child_process.child_exit_to_vfork_parent(
            &mut ctx.user_address_space,
            &mut ctx.user_stack,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            index,
        )?;
        child_pid
    };

    {
        let ctx = context();
        if !ctx
            .user_child_process
            .restore_parent_wait_writable_page_snapshot(
                &ctx.user_address_space,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
        {
            return None;
        }
        if !ctx
            .user_child_process
            .restore_parent_fd_snapshot(&mut ctx.files_struct)
        {
            return None;
        }
        if !ctx.user_child_process.mark_vfork_parent_resumed() {
            return None;
        }
        if !ctx.user_child_process.archive_completed_child_record() {
            return None;
        }
        if !ctx.user_child_process.mark_active_slot_reusable() {
            return None;
        }
    }

    Some(child_pid)
}

fn wait4_completed_record(stat_addr: usize, options: usize) -> usize {
    let mut frame = TrapFrame::zeroed();
    frame.sepc = 0x5000;
    frame.set_reg(10, USER_WAIT4_ALL_CHILDREN);
    frame.set_reg(11, stat_addr);
    frame.set_reg(12, options);
    frame.set_reg(13, 0);
    frame.set_reg(17, 260);
    context().syscall_table.wait4(&mut frame);
    frame.reg(10)
}

fn exercise_nested_vfork_child_slot(assertions: &mut SmokeAssertions) {
    let Some(parent_pid) = start_active_vfork_child(0x80) else {
        assertions.assert("start active vfork child for nested clone", false);
        return;
    };

    let rejected_without_nested_gate = !copy_user_process_for_current_slot(false);
    assertions.assert(
        "active child vfork rejects without nested gate",
        rejected_without_nested_gate,
    );

    let accepted_with_nested_gate = copy_user_process_for_current_slot(true);
    assertions.assert(
        "nested vfork passes copy_user_process gate",
        accepted_with_nested_gate,
    );

    let nested_child_pid = {
        let mut nested_frame = TrapFrame::zeroed();
        nested_frame.sepc = 0x6000;
        nested_frame.set_reg(2, USER_STACK_TOP - 0x480);
        nested_frame.set_reg(10, USER_BUSYBOX_INIT_VFORK_FLAGS);
        nested_frame.set_reg(11, USER_STACK_TOP - 0x580);
        nested_frame.set_reg(17, 220);

        let ctx = context();
        let Some((_child_frame, nested_parent_pid)) =
            ctx.user_child_process.copy_nested_vfork_from_current_child(
                &ctx.user_init_process,
                &ctx.user_clone_deferred_boundaries,
                &ctx.user_address_space,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &nested_frame,
                USER_BUSYBOX_INIT_VFORK_FLAGS,
                USER_STACK_TOP - 0x580,
                true,
                true,
                true,
                true,
            )
        else {
            assertions.assert("copy nested vfork child", false);
            return;
        };
        if nested_parent_pid != parent_pid {
            assertions.assert("nested vfork parent pid preserved", false);
            return;
        }
        let child_pid = ctx.user_child_process.pid();
        if !ctx
            .user_init_process
            .observe_nested_child_process_group_visible(parent_pid, child_pid)
        {
            assertions.assert("nested child process group visible", false);
            return;
        }
        child_pid
    };

    let ctx = context();
    assertions.assert(
        "nested vfork reuses single child slot",
        ctx.user_child_process.nested_vfork_clone()
            && ctx.user_child_process.nested_vfork_parent_pid() == parent_pid
            && nested_child_pid > parent_pid
            && ctx.user_child_process.current_child_continuation()
            && ctx.user_child_process.vfork_child_handoff()
            && !ctx.user_child_process.vfork_parent_resume_on_exit()
            && ctx.user_child_process.pidfd_fd() == usize::MAX
            && ctx.user_child_process.next_child_pid() > nested_child_pid,
    );

    let rejects_deeper_nested = !copy_user_process_for_current_slot(true);
    assertions.assert("deeper nested vfork rejected", rejects_deeper_nested);
}

fn exercise_observed_child_plain_fork(assertions: &mut SmokeAssertions) {
    let (shell_pid, shell_parent_pid, shell_tgid, shell_pgrp, shell_session_id) = {
        let ctx = context();
        (
            ctx.user_child_process.pid(),
            ctx.user_child_process.parent_pid(),
            ctx.user_child_process.tgid(),
            ctx.user_init_process.child_process_group(),
            ctx.user_init_process.child_process_session_id(),
        )
    };
    if shell_pid == 0
        || shell_parent_pid == 0
        || shell_tgid == 0
        || shell_pgrp == 0
        || shell_session_id == 0
    {
        assertions.assert("observed plain fork starts from shell child", false);
        return;
    }

    let child_pid = {
        let mut clone_frame = TrapFrame::zeroed();
        clone_frame.sepc = 0x8000;
        clone_frame.set_reg(2, USER_STACK_TOP - 0x680);
        clone_frame.set_reg(10, USER_PLAIN_FORK_FLAGS);
        clone_frame.set_reg(11, 0);
        clone_frame.set_reg(17, 220);

        let ctx = context();
        let Some(child_pid) = ctx.user_child_process.copy_plain_fork_from_current_child(
            &ctx.user_init_process,
            &ctx.user_clone_deferred_boundaries,
            &ctx.user_address_space,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &ctx.page_metadata_map,
            &clone_frame,
            USER_PLAIN_FORK_FLAGS,
            0,
        ) else {
            assertions.assert("copy observed child plain fork", false);
            return;
        };
        if !ctx
            .user_init_process
            .observe_pending_plain_fork_child_process_group_visible(shell_pid, child_pid)
        {
            assertions.assert("observe pending plain fork child identity", false);
            return;
        }
        child_pid
    };

    {
        let ctx = context();
        assertions.assert(
            "observed plain fork parent continues",
            child_pid > shell_pid
                && ctx.user_child_process.pid() == shell_pid
                && ctx.user_child_process.parent_pid() == shell_parent_pid
                && ctx.user_child_process.observed_plain_fork_clone()
                && ctx.user_child_process.observed_plain_fork_parent_pid() == shell_pid
                && ctx.user_child_process.observed_plain_fork_child_pid() == child_pid
                && ctx
                    .user_child_process
                    .observed_plain_fork_child_pending_wait()
                && ctx.user_child_process.child_trap_frame_reg(10) == Some(0)
                && ctx.user_init_process.pending_plain_fork_child_visible()
                && ctx.user_init_process.pending_plain_fork_child_parent_pid() == shell_pid
                && ctx.user_init_process.pending_plain_fork_child_pid() == child_pid
                && ctx
                    .user_init_process
                    .pending_plain_fork_child_process_group()
                    == shell_pgrp
                && ctx.user_init_process.pending_plain_fork_child_session_id() == shell_session_id,
        );
    }

    let (unknown_pid, negative_pgid, invalid_pgrp, pending_update) = {
        let ctx = context();
        (
            ctx.user_init_process.set_process_group_first_slice(
                child_pid + 100,
                child_pid + 100,
                true,
            ),
            ctx.user_init_process
                .set_process_group_first_slice(child_pid, usize::MAX, true),
            ctx.user_init_process
                .set_process_group_first_slice(child_pid, child_pid + 1, true),
            ctx.user_init_process
                .set_process_group_first_slice(child_pid, child_pid, true),
        )
    };
    {
        let ctx = context();
        assertions.assert(
            "pending plain fork child parent setpgid first slice",
            unknown_pid == UserProcessGroupUpdate::NoSuchProcess
                && negative_pgid == UserProcessGroupUpdate::Invalid
                && invalid_pgrp == UserProcessGroupUpdate::PermissionDenied
                && pending_update == UserProcessGroupUpdate::UpdatedPendingChild(child_pid)
                && ctx.user_init_process.pending_plain_fork_child_visible()
                && ctx
                    .user_init_process
                    .pending_plain_fork_child_process_group()
                    == child_pid
                && ctx.user_init_process.pending_plain_fork_child_session_id() == shell_session_id
                && ctx
                    .user_init_process
                    .pending_plain_fork_child_process_group_set_observed(),
        );
    }

    let child_frame = {
        let mut wait_frame = TrapFrame::zeroed();
        wait_frame.sepc = 0x8100;
        wait_frame.set_reg(2, USER_STACK_TOP - 0x780);
        wait_frame.set_reg(10, USER_WAIT4_ALL_CHILDREN);
        wait_frame.set_reg(11, 0);
        wait_frame.set_reg(12, USER_TEST_WAIT4_WUNTRACED);
        wait_frame.set_reg(13, 0);
        wait_frame.set_reg(17, 260);

        let ctx = context();
        let Some((child_frame, parent_pid, observed_child_pid)) = ctx
            .user_child_process
            .wait4_yield_to_observed_child_continuation(
                &ctx.user_init_process,
                &ctx.user_address_space,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &wait_frame,
                0,
                USER_WAIT4_ALL_CHILDREN,
                0,
            )
        else {
            assertions.assert("observed child wait4 handoff", false);
            return;
        };
        if !ctx
            .user_init_process
            .switch_observed_child_process_visible(parent_pid, observed_child_pid)
        {
            assertions.assert("observed child process visible", false);
            return;
        }
        child_frame
    };

    {
        let ctx = context();
        assertions.assert(
            "observed child handoff uses grandchild pid",
            child_frame.reg(10) == 0
                && ctx.user_child_process.pid() == child_pid
                && ctx.user_child_process.parent_pid() == shell_pid
                && ctx.user_child_process.observed_plain_fork_child_active()
                && ctx.user_init_process.child_process_pid() == child_pid
                && ctx.user_init_process.child_process_group() == child_pid
                && ctx.user_init_process.child_process_session_id() == shell_session_id
                && !ctx.user_init_process.pending_plain_fork_child_visible(),
        );
    }

    let (parent_frame, exit_child_pid, exit_parent_pid) = {
        let ctx = context();
        let Some((parent_frame, _status_ptr, exit_child_pid, exit_parent_pid)) = ctx
            .user_child_process
            .child_exit_to_observed_child_parent_wait(
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                0,
            )
        else {
            assertions.assert("observed child exits to shell wait", false);
            return;
        };
        (parent_frame, exit_child_pid, exit_parent_pid)
    };
    if exit_child_pid != child_pid || exit_parent_pid != shell_pid {
        assertions.assert("observed child exit pids", false);
        return;
    }

    {
        let ctx = context();
        let _ = ctx
            .user_child_process
            .compare_parent_wait_stack_window(&ctx.user_address_space, &ctx.page_metadata_map);
        let _ = ctx
            .user_child_process
            .compare_parent_wait_writable_pages(&ctx.user_address_space, &ctx.page_metadata_map);
        if !ctx
            .user_child_process
            .restore_parent_wait_stack_snapshot(&ctx.user_address_space, &ctx.page_metadata_map)
            || !ctx
                .user_child_process
                .restore_parent_wait_writable_page_snapshot(
                    &ctx.user_address_space,
                    &mut ctx.page_allocator,
                    &ctx.page_metadata_map,
                )
            || !ctx
                .user_child_process
                .restore_parent_fd_snapshot(&mut ctx.files_struct)
            || !ctx
                .user_init_process
                .restore_observed_child_parent_process_visible(shell_pid, child_pid)
            || !ctx
                .user_child_process
                .mark_observed_child_parent_wait_resumed(true, 0)
        {
            assertions.assert("observed child parent restored", false);
            return;
        }
    }

    {
        let ctx = context();
        assertions.assert(
            "observed plain fork restores shell continuation",
            parent_frame.sepc == 0x8100
                && ctx.user_child_process.pid() == shell_pid
                && ctx.user_child_process.parent_pid() == shell_parent_pid
                && ctx.user_child_process.tgid() == shell_tgid
                && ctx.user_child_process.current_child_continuation()
                && ctx.user_child_process.observed_plain_fork_parent_restored()
                && ctx.user_child_process.last_reaped_child_pid() == child_pid
                && ctx.user_init_process.child_process_pid() == shell_pid
                && ctx.user_init_process.child_process_group() == shell_pgrp
                && ctx.user_init_process.child_process_session_id() == shell_session_id
                && !ctx.user_init_process.pending_plain_fork_child_visible(),
        );
    }

    let first_round_cleared = context()
        .user_child_process
        .finish_observed_child_parent_restore();
    {
        let ctx = context();
        assertions.assert(
            "observed plain fork finishes completed round",
            first_round_cleared,
        );
        assertions.assert(
            "observed plain fork preserves shell identity",
            ctx.user_child_process.state() == State::Ready
                && ctx.user_child_process.pid() == shell_pid
                && ctx.user_child_process.parent_pid() == shell_parent_pid
                && ctx.user_child_process.tgid() == shell_tgid,
        );
        assertions.assert(
            "observed plain fork preserves shell continuation",
            ctx.user_child_process.current_child_continuation()
                && ctx.user_child_process.enqueued(),
        );
        assertions.assert(
            "observed plain fork preserves shell runqueue visibility",
            ctx.scheduler
                .boot_runqueue()
                .contains_task(crate::objects::user_boot::USER_CHILD_PID),
        );
        assertions.assert(
            "observed plain fork clears completed round snapshots",
            !ctx.user_child_process.observed_plain_fork_clone()
                && !ctx.user_child_process.child_exit_status_observed()
                && !ctx.user_child_process.parent_wait_frame_saved()
                && !ctx.user_child_process.parent_address_space_snapshot_saved()
                && !ctx.user_child_process.parent_wait_stack_snapshot_copied(),
        );
    }

    let second_child_pid = {
        let mut clone_frame = TrapFrame::zeroed();
        clone_frame.sepc = 0x8200;
        clone_frame.set_reg(2, USER_STACK_TOP - 0x880);
        clone_frame.set_reg(10, USER_PLAIN_FORK_FLAGS);
        clone_frame.set_reg(11, 0);
        clone_frame.set_reg(17, 220);

        let ctx = context();
        let Some(second_child_pid) = ctx.user_child_process.copy_plain_fork_from_current_child(
            &ctx.user_init_process,
            &ctx.user_clone_deferred_boundaries,
            &ctx.user_address_space,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &ctx.page_metadata_map,
            &clone_frame,
            USER_PLAIN_FORK_FLAGS,
            0,
        ) else {
            assertions.assert("copy second observed child plain fork", false);
            return;
        };
        if !ctx
            .user_init_process
            .observe_pending_plain_fork_child_process_group_visible(shell_pid, second_child_pid)
        {
            assertions.assert("observe second pending plain fork child", false);
            return;
        }
        second_child_pid
    };

    {
        let ctx = context();
        assertions.assert(
            "observed plain fork reuses shell for second child",
            second_child_pid > child_pid
                && ctx.user_child_process.pid() == shell_pid
                && ctx.user_child_process.parent_pid() == shell_parent_pid
                && ctx.user_child_process.tgid() == shell_tgid
                && ctx.user_child_process.current_child_continuation()
                && ctx.user_child_process.enqueued()
                && ctx.user_child_process.observed_plain_fork_child_pid() == second_child_pid
                && ctx
                    .user_child_process
                    .observed_plain_fork_child_pending_wait()
                && ctx.user_child_process.next_child_pid() > second_child_pid
                && ctx.user_init_process.pending_plain_fork_child_pid() == second_child_pid,
        );
    }
}

fn start_active_vfork_child(index: usize) -> Option<usize> {
    let mut parent_frame = TrapFrame::zeroed();
    parent_frame.sepc = 0x7000 + index * 4;
    parent_frame.set_reg(2, USER_STACK_TOP - 0x300 - index * 16);
    parent_frame.set_reg(10, USER_BUSYBOX_INIT_VFORK_FLAGS);
    parent_frame.set_reg(11, USER_STACK_TOP - 0x400 - index * 16);
    parent_frame.set_reg(17, 220);

    let child_pid = {
        let ctx = context();
        ctx.user_child_process.copy_vfork_from_parent(
            &ctx.user_init_process,
            &ctx.user_clone_deferred_boundaries,
            &ctx.user_address_space,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &parent_frame,
            USER_BUSYBOX_INIT_VFORK_FLAGS,
            USER_STACK_TOP - 0x400 - index * 16,
            usize::MAX,
            false,
            true,
            true,
            true,
            true,
        )?;
        ctx.user_child_process.pid()
    };

    {
        let ctx = context();
        if !ctx.scheduler.boot_runqueue().contains_task(USER_CHILD_PID) {
            let runqueue_ref = ctx
                .scheduler
                .select_runqueue_for_task(USER_CHILD_PID, &ctx.cpu_group)
                .ok()?;
            ctx.scheduler
                .enqueue_task_on_runqueue(USER_CHILD_PID, runqueue_ref)
                .ok()?;
        }
        if !ctx.user_child_process.mark_enqueued() {
            return None;
        }
        if !ctx
            .user_init_process
            .observe_child_process_group_visible(child_pid)
        {
            return None;
        }
    }
    Some(child_pid)
}

fn copy_user_process_for_current_slot(allow_nested_vfork: bool) -> bool {
    let ctx = context();
    ctx.task_creation_core
        .copy_user_process(
            TaskCopyUserProcessInputs {
                src_process: &ctx.user_init_process,
                dst_process: &ctx.user_child_process,
                root_pid_namespace: &ctx.root_pid_namespace,
                scheduler: &ctx.scheduler,
                cpu_group: &ctx.cpu_group,
                fs_struct: &ctx.fs_struct,
                files_struct: &ctx.files_struct,
                address_space: &ctx.user_address_space,
                trap_frame: &ctx.user_trap_frame,
                boundaries: &ctx.user_clone_deferred_boundaries,
                entry: TaskEntry::UserChild,
                allow_nested_vfork,
            },
            ctx.user_child_process.state(),
            TaskEntry::UserChild,
        )
        .is_ok()
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

fn stack_contains_at(
    stack: &crate::objects::user_stack::UserStack,
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
        let current = user_addr + index;
        let page_vaddr = current & !(USER_PAGE_SIZE - 1);
        let page_offset = current - page_vaddr;
        let Some(page) = stack.page_for_vaddr(page_vaddr) else {
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

fn stack_usize_at(
    stack: &crate::objects::user_stack::UserStack,
    page_metadata_map: &crate::objects::mm_core::PageMetadataMap,
    user_addr: usize,
) -> Option<usize> {
    let mut bytes = [0u8; core::mem::size_of::<usize>()];
    if user_addr < stack.base()
        || user_addr
            .checked_add(bytes.len())
            .filter(|end| *end <= stack.top())
            .is_none()
    {
        return None;
    }

    let mut index = 0usize;
    while index < bytes.len() {
        let current = user_addr + index;
        let page_vaddr = current & !(USER_PAGE_SIZE - 1);
        let page_offset = current - page_vaddr;
        let page = stack.page_for_vaddr(page_vaddr)?;
        let linear = page_metadata_map.page_address(page)?;
        bytes[index] = unsafe { *((linear + page_offset) as *const u8) };
        index += 1;
    }

    Some(usize::from_ne_bytes(bytes))
}

fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0usize;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
