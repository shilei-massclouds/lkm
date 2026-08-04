use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        files::{FdRef, FileBackendKind, FileError, OpenFileDescriptionRef},
        process_prepare::TaskCopyUserProcessInputs,
        state::State,
        task::{TaskEntry, TaskRef},
        trap_type::{
            KERNEL_TRAP_OVERFLOW_STACK_SIZE, KERNEL_TRAP_THREAD_SHIFT, TRAP_FRAME_SIZE,
            TRAP_STACK_RECORD_SIZE, TrapEntryOrigin, TrapFrame, formal_trap_entry_prelude,
            kernel_trap_frame_overflows, kernel_trap_overflow_stack_base,
            kernel_trap_overflow_stack_top,
        },
        user_boot::{
            ElfObjectRole, USER_BOOT_READ_MAX, USER_CHILD_PID, USER_CLONE_SIGCHLD,
            USER_COMPLETED_CHILD_RECORD_CAPACITY, USER_HEAP_BASE, USER_HEAP_SIZE,
            USER_INIT_EXPECTED_MESSAGE, USER_PAGE_SIZE, USER_SIGCHLD_MASK,
            USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE, USER_STACK_TOP,
            USER_WAIT4_ALL_CHILDREN, UserFaultAccess, UserFaultClass, UserFaultRequest,
            UserFaultResult, UserMappingKind, UserProcessGroupLookup, UserProcessGroupUpdate,
            UserRtSigtimedwaitResult,
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
    suite.scenario(&mut BuiltinGrandchildScenario::new());
    suite.scenario(&mut ChildLifecycleScenario::new());
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
        assertions.assert(
            "user child preset",
            ctx.user_task_set.setup(&ctx.kernel_init_task).is_ok(),
        );
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
                && ctx.kernel_init_task.state() == State::OnCpu,
        );

        let space = &ctx.user_address_space;
        assertions.assert("address space online", space.state() == State::Online);
        assertions.assert(
            "address space allocated",
            space.allocated() && space.first_instance(),
        );
        assertions.assert(
            "address space kernel init binding",
            space.bound_to_kernel_init_task() && ctx.kernel_init_task.state() == State::OnCpu,
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
                && heap_mapping.memsz() == 0
                && heap_mapping.filesz() == 0
                && heap_mapping.readable()
                && heap_mapping.writable()
                && !heap_mapping.executable()
                && heap_mapping.user_accessible()
                && heap_mapping.backing_page_count() == 0
                && heap_mapping.page_table_entry_bound()
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
        exercise_user_fault_core(assertions, ctx);
        let syscall_setup_ok = ctx
            .cpu_group
            .boot_cpu_exception_mut()
            .expect("boot CPU exception resource must exist after CPU discovery")
            .syscall_setup(&mut ctx.syscall_table)
            .is_ok();
        assertions.assert("syscall setup", syscall_setup_ok);
        let syscall_enable_ok = ctx
            .cpu_group
            .boot_cpu_exception_mut()
            .expect("boot CPU exception resource must exist after CPU discovery")
            .syscall_enable(&ctx.syscall_table)
            .is_ok();
        assertions.assert("syscall enable", syscall_enable_ok);
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
            ctx.kernel_init_user_state
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
        let exception_type = ctx
            .cpu_group
            .boot_cpu_exception()
            .expect("boot CPU exception resource must exist after CPU discovery");
        assertions.assert(
            "KernelInitTask syscall context bind",
            ctx.kernel_init_user_state
                .bind_syscall_context(&ctx.user_trap_frame, exception_type, &ctx.syscall_table)
                .is_ok(),
        );
        assertions.assert(
            "user runtime declared",
            ctx.kernel_init_user_runtime.declare().is_ok(),
        );
        assertions.assert(
            "KernelInitTask user runtime preset",
            ctx.kernel_init_user_runtime
                .preset(&mut ctx.kernel_init_task)
                .is_ok(),
        );
        assertions.assert(
            "KernelInitTask user runtime setup",
            ctx.kernel_init_user_runtime
                .setup(&ctx.kernel_init_task, &ctx.kernel_init_user_state)
                .is_ok(),
        );
        assertions.assert(
            "KernelInitTask user runtime active binding",
            ctx.kernel_init_user_runtime
                .commit_active_binding(&ctx.kernel_init_task)
                .is_ok(),
        );
        assertions.assert(
            "KernelInitTask user runtime online",
            ctx.kernel_init_user_runtime
                .enable(&ctx.kernel_init_task, &ctx.kernel_init_user_state)
                .is_ok(),
        );
        assertions.assert(
            "KernelInitTask user state active",
            ctx.kernel_init_user_state
                .activate_user_flow(&ctx.kernel_init_user_runtime)
                .is_ok(),
        );
        let process = &ctx.kernel_init_user_state;
        assertions.assert(
            "KernelInitTask user runtime remains a Flow-owned child",
            process.resources_bound()
                && process.active_user_flow_online()
                && ctx.kernel_init_user_runtime.state() == State::Online
                && ctx.kernel_init_user_runtime.declared()
                && ctx.kernel_init_user_runtime.declaration_occurrence() == 1
                && ctx.kernel_init_user_runtime.owner_pid()
                    == crate::objects::rest_init::KERNEL_INIT_PID
                && ctx.kernel_init_user_runtime.application_entered()
                && !ctx.kernel_init_user_runtime.released()
                && ctx.kernel_init_task.flow_state() == State::Online
                && ctx.kernel_init_task.flow().owner().is_valid()
                && ctx.kernel_init_task.state() == State::OnCpu
                && ctx.kernel_init_task.kernel_init_flow_owned()
                && ctx.kernel_init_user_runtime.task_ref_owner() == ctx.kernel_init_task.task_ref()
                && ctx.kernel_init_user_runtime.flow_generation() != 0,
        );
        assertions.assert(
            "user init identity",
            process.preserves_kernel_init_task()
                && process.pid1_preserved()
                && process.exec_identity_handoff()
                && process.no_new_task_struct()
                && process.kernel_init_not_destroyed(),
        );
        let kernel_init_task_ref = ctx.kernel_init_task.task_ref();
        let lifetime_flow_ref = ctx.kernel_init_user_runtime.flow_ref();
        let first_runtime_handoff = ctx
            .kernel_init_user_runtime
            .replace_kernel_application(&mut ctx.kernel_init_task)
            .is_ok();
        let first_runtime_flow_ref = ctx.kernel_init_user_runtime.flow_ref();
        let second_runtime_handoff = ctx
            .kernel_init_user_runtime
            .replace_kernel_application(&mut ctx.kernel_init_task)
            .is_ok();
        let second_runtime_flow_ref = ctx.kernel_init_user_runtime.flow_ref();
        assertions.assert(
            "KernelInitTask consecutive exec preserves TaskRef and FlowRef",
            first_runtime_handoff
                && second_runtime_handoff
                && ctx.kernel_init_task.task_ref() == kernel_init_task_ref
                && lifetime_flow_ref.same_identity(first_runtime_flow_ref)
                && first_runtime_flow_ref.same_identity(second_runtime_flow_ref)
                && ctx
                    .kernel_init_user_runtime
                    .flow_ref_valid(lifetime_flow_ref)
                && ctx
                    .kernel_init_user_runtime
                    .flow_ref_valid(first_runtime_flow_ref)
                && ctx
                    .kernel_init_user_runtime
                    .flow_ref_valid(second_runtime_flow_ref)
                && ctx.kernel_init_task.task().flow() == second_runtime_flow_ref,
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
        let pid1_pid = ctx.kernel_init_user_state.read_pid(false);
        let pid1_sid = ctx.kernel_init_user_state.read_session_id(0, false);
        let pid1_explicit_sid = ctx.kernel_init_user_state.read_session_id(1, false);
        let pid1_setsid = ctx.kernel_init_user_state.set_session_id_first_slice(false);
        let child_visible = ctx
            .kernel_init_user_state
            .observe_child_process_group_visible(USER_CHILD_PID);
        let child_pid = ctx.kernel_init_user_state.read_pid(true);
        let child_initial_sid = ctx.kernel_init_user_state.read_session_id(0, true);
        let child_setsid = ctx.kernel_init_user_state.set_session_id_first_slice(true);
        let child_tty_sid_before_ctty = ctx
            .kernel_init_user_state
            .read_tty_session_id_first_slice(true);
        let child_tiocsctty = ctx
            .kernel_init_user_state
            .bind_controlling_tty_first_slice(true, 1);
        let child_tty_sid = ctx
            .kernel_init_user_state
            .read_tty_session_id_first_slice(true);
        let repeat_child_tiocsctty = ctx
            .kernel_init_user_state
            .bind_controlling_tty_first_slice(true, 1);
        let pid1_tty_sid = ctx
            .kernel_init_user_state
            .read_tty_session_id_first_slice(false);
        let pid1_tiocsctty = ctx
            .kernel_init_user_state
            .bind_controlling_tty_first_slice(false, 1);
        let child_current_sid = ctx.kernel_init_user_state.read_session_id(0, true);
        let child_explicit_sid = ctx
            .kernel_init_user_state
            .read_session_id(USER_CHILD_PID, false);
        let child_current_pgrp = ctx.kernel_init_user_state.read_process_group(0, true);
        let repeat_child_setsid = ctx.kernel_init_user_state.set_session_id_first_slice(true);
        let unknown_sid = ctx.kernel_init_user_state.read_session_id(999, false);
        assertions.assert(
            "process identity getsid and child setsid first slice",
            pid1_pid == Some(1)
                && pid1_sid == UserProcessGroupLookup::Found(1)
                && pid1_explicit_sid == UserProcessGroupLookup::Found(1)
                && pid1_setsid == UserProcessGroupUpdate::PermissionDenied
                && ctx.kernel_init_user_state.setsid_eperm_observed()
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
                && ctx.kernel_init_user_state.child_process_group() == USER_CHILD_PID
                && ctx.kernel_init_user_state.child_process_session_id() == USER_CHILD_PID
                && ctx
                    .kernel_init_user_state
                    .child_session_leader_first_slice()
                && ctx.kernel_init_user_state.child_setsid_success_observed()
                && ctx
                    .kernel_init_user_state
                    .child_controlling_tty_cleared_on_setsid()
                && ctx.kernel_init_user_state.child_controlling_tty_bound()
                && ctx.kernel_init_user_state.foreground_pgrp() == USER_CHILD_PID
                && ctx.kernel_init_user_state.tty_session_id_read_observed()
                && ctx.kernel_init_user_state.tiocsctty_observed()
                && ctx.kernel_init_user_state.session_id_read_observed(),
        );
        assertions.assert(
            "sigchld pending from child exit",
            ctx.kernel_init_user_state.record_child_exit_sigchld() == Some(false)
                && ctx.kernel_init_user_state.pending_sigchld(),
        );
        let mut immediate_wait_frame = TrapFrame::zeroed();
        immediate_wait_frame.sepc = 0x1000;
        let immediate = ctx.kernel_init_user_state.begin_rt_sigtimedwait(
            USER_SIGCHLD_MASK,
            true,
            true,
            &immediate_wait_frame,
        );
        assertions.assert(
            "rt_sigtimedwait consumes pending sigchld",
            immediate == UserRtSigtimedwaitResult::ReturnSignal(USER_CLONE_SIGCHLD)
                && !ctx.kernel_init_user_state.pending_sigchld()
                && ctx.kernel_init_user_state.rt_sigtimedwait_dequeued_signal()
                    == USER_CLONE_SIGCHLD
                && ctx.kernel_init_user_state.rt_sigtimedwait_return_signal() == USER_CLONE_SIGCHLD,
        );
        let mut sleeping_wait_frame = TrapFrame::zeroed();
        sleeping_wait_frame.sepc = 0x2000;
        sleeping_wait_frame.set_reg(17, 137);
        let sleeping = ctx.kernel_init_user_state.begin_rt_sigtimedwait(
            USER_SIGCHLD_MASK,
            true,
            true,
            &sleeping_wait_frame,
        );
        assertions.assert(
            "rt_sigtimedwait sleep waiter",
            sleeping == UserRtSigtimedwaitResult::Sleep
                && ctx.kernel_init_user_state.rt_sigtimedwait_sleeping()
                && ctx.kernel_init_user_state.rt_sigtimedwait_waiter_enqueued()
                && ctx
                    .kernel_init_user_state
                    .rt_sigtimedwait_saved_frame_bound()
                && ctx.kernel_init_user_state.rt_sigtimedwait_sleep_reason()
                    == USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE
                && !ctx.kernel_init_user_state.pending_sigchld(),
        );
        assertions.assert(
            "sigchld wakes waiting rt_sigtimedwait",
            ctx.kernel_init_user_state.record_child_exit_sigchld() == Some(true)
                && !ctx.kernel_init_user_state.rt_sigtimedwait_sleeping()
                && ctx.kernel_init_user_state.rt_sigtimedwait_waiter_finished()
                && ctx
                    .kernel_init_user_state
                    .rt_sigtimedwait_wake_sigchld_committed()
                && ctx.kernel_init_user_state.rt_sigtimedwait_wake_signal() == USER_CLONE_SIGCHLD
                && ctx.kernel_init_user_state.rt_sigtimedwait_dequeued_signal()
                    == USER_CLONE_SIGCHLD,
        );
        let mut resumed_wait_frame = TrapFrame::zeroed();
        let resumed_signal = ctx
            .kernel_init_user_state
            .complete_rt_sigtimedwait_wake(&mut resumed_wait_frame);
        assertions.assert(
            "rt_sigtimedwait wake frame returns sigchld",
            resumed_signal == Some(USER_CLONE_SIGCHLD) && resumed_wait_frame.sepc == 0x2000,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct BuiltinGrandchildScenario;

impl BuiltinGrandchildScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for BuiltinGrandchildScenario {
    fn name(&self) -> &'static str {
        "user_boot.builtin_grandchild_exec"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert(
            "builtin grandchild independent scenario prerequisites",
            ctx.kernel_init_user_state.active_user_flow_online()
                && ctx.user_address_space.state() == State::Online
                && ctx.user_trap_frame.state() == State::Ready
                && ctx.fs_struct.state() == State::Ready
                && ctx.files_struct.state() == State::Ready
                && ctx.user_task_set.active_task_record_available(),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        exercise_pid1_plain_fork_builtin_grandchild(assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct ChildLifecycleScenario;

impl ChildLifecycleScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for ChildLifecycleScenario {
    fn name(&self) -> &'static str {
        "user_boot.child_lifecycle"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "child lifecycle starts from reusable slot",
            context().user_task_set.active_task_record_available(),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        match context()
            .user_task_set
            .smoke_task_slot_generation_contract()
        {
            Ok(()) => assertions.assert(
                "user task slots reject ninth live task and invalidate stale generations",
                true,
            ),
            Err(stage) => assertions.assert(stage, false),
        }
        exercise_completed_child_record_reuse(assertions);
        exercise_nested_vfork_task_record(assertions);
        exercise_observed_child_plain_fork(assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn exercise_kernel_trap_overflow_contract(assertions: &mut SmokeAssertions) {
    const ALIGNED_STACK_BASE: usize = 0x8000_0000;
    const THREAD_SIZE: usize = 1 << KERNEL_TRAP_THREAD_SHIFT;

    let stack_top = ALIGNED_STACK_BASE + THREAD_SIZE;
    let lowest_legal_sp = ALIGNED_STACK_BASE + TRAP_STACK_RECORD_SIZE;
    let first_guard_sp = lowest_legal_sp - 1;
    let adjacent_half_sp = stack_top + TRAP_STACK_RECORD_SIZE;
    assertions.assert(
        "kernel trap frame size and overflow stack layout",
        core::mem::size_of::<TrapFrame>() == TRAP_FRAME_SIZE
            && TRAP_FRAME_SIZE == 288
            && TRAP_STACK_RECORD_SIZE > TRAP_FRAME_SIZE
            && KERNEL_TRAP_THREAD_SHIFT == 14
            && kernel_trap_overflow_stack_base().is_multiple_of(16)
            && kernel_trap_overflow_stack_top()
                == kernel_trap_overflow_stack_base() + KERNEL_TRAP_OVERFLOW_STACK_SIZE,
    );
    assertions.assert(
        "kernel trap VMAP overflow classification endpoints",
        !kernel_trap_frame_overflows(stack_top, ALIGNED_STACK_BASE, stack_top)
            && !kernel_trap_frame_overflows(lowest_legal_sp, ALIGNED_STACK_BASE, stack_top)
            && kernel_trap_frame_overflows(first_guard_sp, ALIGNED_STACK_BASE, stack_top)
            && kernel_trap_frame_overflows(adjacent_half_sp, ALIGNED_STACK_BASE, stack_top),
    );

    let entry_context = 0x9000_0000;
    let kernel_entry = formal_trap_entry_prelude(
        TrapEntryOrigin::Kernel,
        lowest_legal_sp,
        entry_context,
        ALIGNED_STACK_BASE,
        stack_top,
    );
    let kernel_overflow_entry = formal_trap_entry_prelude(
        TrapEntryOrigin::Kernel,
        first_guard_sp,
        entry_context,
        ALIGNED_STACK_BASE,
        stack_top,
    );
    let user_entry = formal_trap_entry_prelude(
        TrapEntryOrigin::User,
        first_guard_sp,
        entry_context,
        ALIGNED_STACK_BASE,
        stack_top,
    );
    assertions.assert(
        "kernel trap prelude checks complete record capacity",
        kernel_entry.origin == TrapEntryOrigin::Kernel
            && kernel_entry.stack_pointer == lowest_legal_sp
            && kernel_entry.entry_context == entry_context
            && kernel_entry.capacity_check_performed
            && !kernel_entry.overflow
            && kernel_overflow_entry.origin == TrapEntryOrigin::Kernel
            && kernel_overflow_entry.stack_pointer == first_guard_sp
            && kernel_overflow_entry.entry_context == entry_context
            && kernel_overflow_entry.capacity_check_performed
            && kernel_overflow_entry.overflow,
    );
    assertions.assert(
        "user trap prelude selects the installed kernel stack and checks capacity",
        kernel_trap_frame_overflows(first_guard_sp, ALIGNED_STACK_BASE, stack_top)
            && user_entry.origin == TrapEntryOrigin::User
            && user_entry.stack_pointer == stack_top
            && user_entry.entry_context == entry_context
            && user_entry.capacity_check_performed
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
fn exercise_user_fault_core(assertions: &mut SmokeAssertions, ctx: &mut crate::context::Context) {
    let satp = ctx.user_address_space.satp_token();
    let task_ref = ctx.current_task_ref().unwrap_or(TaskRef::NONE);
    let sepc = 0xfeed_1000usize;
    let heap_end = USER_HEAP_BASE + 2 * USER_PAGE_SIZE;
    let brk = ctx.user_address_space.user_brk(heap_end);
    let free_before = ctx.page_allocator.buddy_total_free_pages();
    let leaves_before = ctx.user_address_space.user_leaf_pte_count();
    let l0_before = ctx.user_address_space.page_table_l0_count();
    let heap_fault = ctx.user_address_space.resolve_user_fault(
        &mut ctx.user_stack,
        UserFaultRequest::new(
            task_ref,
            satp,
            USER_HEAP_BASE + USER_PAGE_SIZE,
            sepc,
            UserFaultAccess::Store,
        ),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    let heap_diag = ctx.user_address_space.last_user_fault();
    assertions.assert(
        "common fault core heap NotPresent retries same instruction",
        brk == heap_end
            && heap_fault.class() == UserFaultClass::NotPresent
            && heap_fault.result() == UserFaultResult::RetrySameInstruction
            && heap_fault.sepc() == sepc
            && heap_diag.task_ref() == task_ref
            && heap_diag.mm_satp() == satp
            && heap_diag.address() == USER_HEAP_BASE + USER_PAGE_SIZE
            && heap_diag.class() == UserFaultClass::NotPresent
            && heap_diag.result() == UserFaultResult::RetrySameInstruction
            && heap_diag.cow_shared_count() == 0
            && heap_diag.cow_copied_count() == 0
            && ctx.user_address_space.user_leaf_pte_count() == leaves_before + 1
            && ctx.page_allocator.buddy_total_free_pages()
                + 1
                + (ctx.user_address_space.page_table_l0_count() - l0_before)
                == free_before,
    );

    let mappings_before_anonymous = ctx.user_address_space.mapping_count();
    let leaves_before_anonymous = ctx.user_address_space.user_leaf_pte_count();
    let anonymous = ctx
        .user_address_space
        .user_mmap(0, 2 * USER_PAGE_SIZE, 0x3, 0x22, usize::MAX, 0)
        .ok();
    let anonymous_fault = anonymous.map(|base| {
        ctx.user_address_space.resolve_user_fault(
            &mut ctx.user_stack,
            UserFaultRequest::new(task_ref, satp, base, sepc + 4, UserFaultAccess::Load),
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    });
    assertions.assert(
        "common fault core anonymous-private demand page",
        anonymous.is_some()
            && anonymous_fault.is_some_and(|resolution| {
                resolution.class() == UserFaultClass::NotPresent
                    && resolution.result() == UserFaultResult::RetrySameInstruction
                    && resolution.sepc() == sepc + 4
            }),
    );

    let state_before_rejected_unmap = (
        ctx.user_address_space.mapping_count(),
        ctx.user_address_space.user_leaf_pte_count(),
        ctx.page_allocator.buddy_total_free_pages(),
    );
    let partial_unmap_rejected = anonymous.is_some_and(|base| {
        !ctx.user_address_space.user_munmap(
            base,
            USER_PAGE_SIZE,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    });
    let heap_unmap_rejected = !ctx.user_address_space.user_munmap(
        USER_HEAP_BASE,
        2 * USER_PAGE_SIZE,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "anonymous-private munmap rejects partial and non-anonymous atomically",
        partial_unmap_rejected
            && heap_unmap_rejected
            && state_before_rejected_unmap
                == (
                    ctx.user_address_space.mapping_count(),
                    ctx.user_address_space.user_leaf_pte_count(),
                    ctx.page_allocator.buddy_total_free_pages(),
                ),
    );

    let leaves_before_unmap = ctx.user_address_space.user_leaf_pte_count();
    let free_before_unmap = ctx.page_allocator.buddy_total_free_pages();
    let anonymous_unmapped = anonymous.is_some_and(|base| {
        ctx.user_address_space.user_munmap(
            base,
            2 * USER_PAGE_SIZE,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    });
    let unmapped_diagnostic = anonymous.map(|base| {
        ctx.user_address_space
            .fault_mapping_diagnostic(base, UserFaultAccess::Load)
    });
    let anonymous_reused = ctx
        .user_address_space
        .user_mmap(0, 2 * USER_PAGE_SIZE, 0x3, 0x22, usize::MAX, 0)
        .ok();
    let anonymous_reuse_unmapped = anonymous_reused.is_some_and(|base| {
        ctx.user_address_space.user_munmap(
            base,
            2 * USER_PAGE_SIZE,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    });
    assertions.assert(
        "anonymous-private whole-VMA munmap releases and reuses range",
        anonymous_unmapped
            && anonymous_reused == anonymous
            && anonymous_reuse_unmapped
            && unmapped_diagnostic.is_some_and(|diagnostic| !diagnostic.mapped())
            && leaves_before_unmap == leaves_before_anonymous + 1
            && ctx.user_address_space.user_leaf_pte_count() == leaves_before_anonymous
            && ctx.page_allocator.buddy_total_free_pages() == free_before_unmap + 1
            && ctx.user_address_space.mapping_count() == mappings_before_anonymous,
    );

    let read_only_addr = (0..ctx.user_address_space.segment_mapping_count()).find_map(|index| {
        let mapping = ctx.user_address_space.mapping(index)?;
        (!mapping.writable()).then_some(mapping.vaddr())
    });
    let protection = read_only_addr.map(|address| {
        ctx.user_address_space.resolve_user_fault(
            &mut ctx.user_stack,
            UserFaultRequest::new(task_ref, satp, address, sepc + 8, UserFaultAccess::Store),
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    });
    let protection_diagnostic = ctx.user_address_space.last_user_fault();
    assertions.assert(
        "common fault core protection classification",
        read_only_addr.is_some_and(|address| {
            protection.is_some_and(|resolution| {
                resolution.class() == UserFaultClass::Protection
                    && resolution.result() == UserFaultResult::SegvAccerr
                    && resolution.sepc() == sepc + 8
            }) && protection_diagnostic.address() == address
                && protection_diagnostic.sepc() == sepc + 8
                && protection_diagnostic.class() == UserFaultClass::Protection
                && protection_diagnostic.result() == UserFaultResult::SegvAccerr
        }),
    );

    let unmapped = ctx.user_address_space.resolve_user_fault(
        &mut ctx.user_stack,
        UserFaultRequest::new(
            task_ref,
            satp,
            USER_HEAP_BASE - USER_PAGE_SIZE,
            sepc + 12,
            UserFaultAccess::Load,
        ),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    let unmapped_diagnostic = ctx.user_address_space.last_user_fault();
    assertions.assert(
        "common fault core unmapped classification",
        unmapped.class() == UserFaultClass::Unmapped
            && unmapped.result() == UserFaultResult::SegvMaperr
            && unmapped.sepc() == sepc + 12
            && unmapped_diagnostic.address() == USER_HEAP_BASE - USER_PAGE_SIZE
            && unmapped_diagnostic.sepc() == sepc + 12
            && unmapped_diagnostic.class() == UserFaultClass::Unmapped
            && unmapped_diagnostic.result() == UserFaultResult::SegvMaperr,
    );

    let wrong_mm = ctx.user_address_space.resolve_user_fault(
        &mut ctx.user_stack,
        UserFaultRequest::new(
            task_ref,
            satp ^ 1,
            USER_HEAP_BASE,
            sepc + 16,
            UserFaultAccess::Load,
        ),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    assertions.assert(
        "common fault core stale mm rejected",
        wrong_mm.class() == UserFaultClass::Protection
            && wrong_mm.result() == UserFaultResult::InvalidContext,
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

fn exercise_pid1_plain_fork_builtin_grandchild(assertions: &mut SmokeAssertions) {
    let mut clone_frame = TrapFrame::zeroed();
    clone_frame.sepc = 0x9000;
    clone_frame.set_reg(2, USER_STACK_TOP - 0x900);
    clone_frame.set_reg(10, USER_PLAIN_FORK_FLAGS);
    clone_frame.set_reg(11, 0);
    clone_frame.set_reg(17, 220);

    if !copy_user_process_for_current_slot(false) {
        assertions.assert("pid1 plain fork copy gate", false);
        return;
    }
    let cow_probe_addr = context().user_stack.initial_sp();
    let readonly_probe_addr = {
        let ctx = context();
        let mut index = 0usize;
        let mut address = None;
        while index < ctx.user_address_space.mapping_count() {
            if let Some(mapping) = ctx.user_address_space.mapping(index)
                && mapping.kind() == UserMappingKind::ElfSegment
                && !mapping.writable()
                && mapping.backing_page_count() != 0
            {
                address = mapping.backing_page_vaddr(0);
                break;
            }
            index += 1;
        }
        address
    };
    let parent_leaf_before_failure = {
        let ctx = context();
        ctx.user_address_space.cow_leaf_diagnostic(
            &ctx.user_stack,
            cow_probe_addr,
            &ctx.page_metadata_map,
        )
    };
    let (free_before_dup_failure, slots_before_dup_failure, pid_before_dup_failure, satp_before) = {
        let ctx = context();
        (
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.user_task_set.free_task_slot_count(),
            ctx.user_task_set.next_child_pid(),
            ctx.user_address_space.satp_token(),
        )
    };
    context()
        .user_task_set
        .smoke_fail_next_cow_child_mm_after_prepare();
    let cow_prepare_failure = {
        let ctx = context();
        ctx.user_task_set
            .copy_plain_fork_from_parent(
                &ctx.kernel_init_user_state,
                &ctx.user_clone_deferred_boundaries,
                &mut ctx.user_address_space,
                &ctx.user_stack,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &clone_frame,
                USER_PLAIN_FORK_FLAGS,
                0,
                true,
                true,
                true,
                true,
            )
            .is_none()
    };
    {
        let ctx = context();
        assertions.assert("pid1 COW prepare failure injected", cow_prepare_failure);
        assertions.assert(
            "pid1 COW prepare failure releases pages",
            ctx.page_allocator.buddy_total_free_pages() == free_before_dup_failure,
        );
        assertions.assert(
            "pid1 COW prepare rollback validates unpublished task",
            ctx.user_task_set.smoke_last_unpublished_rollback_stage() >= 1,
        );
        assertions.assert(
            "pid1 COW prepare rollback releases mm storage",
            ctx.user_task_set.smoke_last_unpublished_rollback_stage() >= 2,
        );
        assertions.assert(
            "pid1 COW prepare rollback keeps task unpublished",
            ctx.user_task_set.smoke_last_unpublished_rollback_stage() >= 3,
        );
        assertions.assert(
            "pid1 COW prepare rollback clears reserved task",
            ctx.user_task_set.smoke_last_unpublished_rollback_stage() >= 4,
        );
        assertions.assert(
            "pid1 COW prepare rollback releases slot",
            ctx.user_task_set.smoke_last_unpublished_rollback_stage() == 5,
        );
        assertions.assert(
            "pid1 COW prepare failure releases task slot",
            ctx.user_task_set.free_task_slot_count() == slots_before_dup_failure,
        );
        assertions.assert(
            "pid1 COW prepare failure preserves pid allocation",
            ctx.user_task_set.next_child_pid() == pid_before_dup_failure,
        );
        assertions.assert(
            "pid1 COW prepare failure preserves reusable task record",
            ctx.user_task_set.active_task_record_available(),
        );
        assertions.assert(
            "pid1 COW prepare failure preserves parent mm",
            ctx.user_address_space.satp_token() == satp_before
                && ctx
                    .user_address_space
                    .owned_by(crate::objects::task::TaskRef::KERNEL_INIT),
        );
        assertions.assert(
            "pid1 COW prepare failure preserves parent leaf and reference",
            parent_leaf_before_failure.is_some()
                && ctx.user_address_space.cow_leaf_diagnostic(
                    &ctx.user_stack,
                    cow_probe_addr,
                    &ctx.page_metadata_map,
                ) == parent_leaf_before_failure
                && parent_leaf_before_failure
                    .is_some_and(|leaf| leaf.writable() && !leaf.cow() && leaf.refcount() == 1),
        );
    }
    let outer_child_pid = {
        let ctx = context();
        let Some(child_pid) = ctx.user_task_set.copy_plain_fork_from_parent(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
            &ctx.user_stack,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &clone_frame,
            USER_PLAIN_FORK_FLAGS,
            0,
            true,
            true,
            true,
            true,
        ) else {
            assertions.assert("pid1 plain fork child copy", false);
            return;
        };
        let Some(runqueue_ref) = ctx
            .scheduler_mut()
            .select_scheduler_for_task(child_pid)
            .ok()
        else {
            assertions.assert("pid1 plain fork runqueue select", false);
            return;
        };
        let task_ref = ctx.user_task_set.active_task_ref();
        if ctx
            .scheduler_mut()
            .enqueue_task_on_scheduler(child_pid, task_ref, runqueue_ref)
            .is_err()
            || !ctx.user_task_set.mark_enqueued()
            || !ctx
                .kernel_init_user_state
                .observe_child_process_group_visible(child_pid)
        {
            assertions.assert("pid1 plain fork enqueue and identity", false);
            return;
        }
        child_pid
    };

    {
        let ctx = context();
        let child_ref = ctx.user_task_set.active_task_ref();
        let parent_leaf = ctx.user_address_space.cow_leaf_diagnostic(
            &ctx.user_stack,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        let child_leaf = ctx.user_task_set.inactive_cow_leaf_diagnostic(
            child_ref,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        assertions.assert(
            "plain fork shares stack frame as parent-child RO+COW",
            parent_leaf.is_some_and(|leaf| {
                leaf.task_ref()
                    .same_identity(crate::objects::task::TaskRef::KERNEL_INIT)
                    && leaf.mm_satp() == ctx.user_address_space.satp_token()
                    && !leaf.writable()
                    && leaf.cow()
                    && leaf.refcount() == 2
            }) && child_leaf.is_some_and(|leaf| {
                parent_leaf.is_some_and(|parent| {
                    leaf.task_ref().same_identity(child_ref)
                        && leaf.mm_satp() != parent.mm_satp()
                        && leaf.phys() == parent.phys()
                        && !leaf.writable()
                        && leaf.cow()
                        && leaf.refcount() == 2
                })
            }),
        );
        let readonly_parent = readonly_probe_addr.and_then(|address| {
            ctx.user_address_space.cow_leaf_diagnostic(
                &ctx.user_stack,
                address,
                &ctx.page_metadata_map,
            )
        });
        let readonly_child = readonly_probe_addr.and_then(|address| {
            ctx.user_task_set.inactive_cow_leaf_diagnostic(
                child_ref,
                address,
                &ctx.page_metadata_map,
            )
        });
        assertions.assert(
            "plain fork shares read-only ELF frame without COW",
            readonly_parent.is_some_and(|parent| {
                !parent.writable()
                    && !parent.cow()
                    && parent.refcount() == 2
                    && readonly_child.is_some_and(|child| {
                        child.phys() == parent.phys()
                            && child.mm_satp() != parent.mm_satp()
                            && !child.writable()
                            && !child.cow()
                            && child.refcount() == 2
                    })
            }),
        );
    }

    let outer_child_frame = {
        let mut wait_frame = TrapFrame::zeroed();
        wait_frame.sepc = 0x9010;
        wait_frame.set_reg(2, USER_STACK_TOP - 0xa00);
        let ctx = context();
        let Some(child_frame) = ctx.user_task_set.wait4_yield_to_child_continuation(
            &ctx.kernel_init_user_state,
            &mut ctx.user_address_space,
            &mut ctx.user_stack,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &wait_frame,
            0,
            USER_WAIT4_ALL_CHILDREN,
            0,
            0,
        ) else {
            assertions.assert("pid1 plain fork outer wait handoff", false);
            return;
        };
        child_frame
    };
    {
        let ctx = context();
        let active = ctx.user_task_set.active_task_ref();
        if ctx.commit_user_dispatch(active).is_err() {
            assertions.assert("pid1 plain fork dispatch", false);
            return;
        }
    }
    {
        let ctx = context();
        let active = ctx.user_task_set.active_task_ref();
        let satp = ctx.user_address_space.satp_token();
        let leaf_before = ctx.user_address_space.cow_leaf_diagnostic(
            &ctx.user_stack,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        let free_before = ctx.page_allocator.buddy_total_free_pages();
        ctx.page_allocator.smoke_fail_next_user_frame_allocation();
        let resolution = ctx.user_address_space.resolve_user_fault(
            &mut ctx.user_stack,
            UserFaultRequest::new(active, satp, cow_probe_addr, 0x9014, UserFaultAccess::Store),
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        );
        assertions.assert(
            "child COW allocation failure preserves shared leaf and references",
            resolution.class() == UserFaultClass::CowWriteProtect
                && resolution.result() == UserFaultResult::TaskOom
                && resolution.sepc() == 0x9014
                && leaf_before.is_some()
                && ctx.user_address_space.cow_leaf_diagnostic(
                    &ctx.user_stack,
                    cow_probe_addr,
                    &ctx.page_metadata_map,
                ) == leaf_before
                && ctx.page_allocator.buddy_total_free_pages() == free_before,
        );
    }
    {
        let ctx = context();
        let active = ctx.user_task_set.active_task_ref();
        let satp = ctx.user_address_space.satp_token();
        let resolution = ctx.user_address_space.resolve_user_fault(
            &mut ctx.user_stack,
            UserFaultRequest::new(active, satp, cow_probe_addr, 0x9018, UserFaultAccess::Store),
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        );
        let child_leaf = ctx.user_address_space.cow_leaf_diagnostic(
            &ctx.user_stack,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        let parent_leaf = ctx.user_task_set.inactive_cow_leaf_diagnostic(
            crate::objects::task::TaskRef::KERNEL_INIT,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        let diag = ctx.user_address_space.last_user_fault();
        assertions.assert(
            "child COW write fault copies frame and retries same instruction",
            resolution.class() == UserFaultClass::CowWriteProtect
                && resolution.result() == UserFaultResult::RetrySameInstruction
                && resolution.sepc() == 0x9018
                && diag.cow_shared_count() == 2
                && diag.cow_copied_count() == 1
                && diag.cow_unique_count() == 0
                && child_leaf.is_some_and(|child| {
                    child.writable()
                        && !child.cow()
                        && child.refcount() == 1
                        && parent_leaf.is_some_and(|parent| {
                            !parent.writable()
                                && parent.cow()
                                && parent.refcount() == 1
                                && parent.phys() != child.phys()
                        })
                }),
        );
    }
    {
        let child = &context().user_task_set;
        assertions.assert(
            "pid1 plain fork owns outer continuation",
            outer_child_frame.reg(10) == 0
                && child.pid1_plain_fork_child_continuation()
                && child.parent_wait_frame_saved()
                && !child.parent_address_space_snapshot_saved()
                && child.parent_fd_snapshot_saved()
                && !child.parent_wait_stack_snapshot_copied()
                && !child.parent_wait_writable_page_snapshot_saved()
                && context()
                    .user_address_space
                    .owned_by(child.active_task_ref()),
        );
    }
    let outer_child_satp = context().user_address_space.satp_token();

    let pipe_pair = match context().files_struct.pipe2_fd_pair(0) {
        Ok(pair) => pair,
        Err(_) => {
            assertions.assert("builtin grandchild pipe pair", false);
            return;
        }
    };
    let cloexec_fd = match context().files_struct.open_null_path(
        b"/dev/null",
        USER_TEST_O_RDWR | USER_TEST_O_LARGEFILE | USER_TEST_O_CLOEXEC,
    ) {
        Ok(fd) => fd,
        Err(_) => {
            assertions.assert("builtin grandchild cloexec fd", false);
            return;
        }
    };
    let first_grandchild_pid = {
        let mut inner_clone = TrapFrame::zeroed();
        inner_clone.sepc = 0x9020;
        inner_clone.set_reg(2, USER_STACK_TOP - 0xb00);
        let ctx = context();
        let Some(child_pid) = ctx.user_task_set.copy_plain_fork_from_current_child(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
            &ctx.user_stack,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &inner_clone,
            USER_PLAIN_FORK_FLAGS,
            0,
        ) else {
            assertions.assert("builtin grandchild clone", false);
            return;
        };
        let grandchild_ref = ctx.user_task_set.pending_task_ref();
        let parent_leaf = ctx.user_address_space.cow_leaf_diagnostic(
            &ctx.user_stack,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        let child_leaf = ctx.user_task_set.inactive_cow_leaf_diagnostic(
            grandchild_ref,
            cow_probe_addr,
            &ctx.page_metadata_map,
        );
        assertions.assert(
            "nested fork re-lowers unique parent page and shares it RO+COW",
            parent_leaf.is_some_and(|parent| {
                parent.address() == cow_probe_addr & !(USER_PAGE_SIZE - 1)
                    && !parent.writable()
                    && parent.cow()
                    && parent.refcount() == 2
                    && child_leaf.is_some_and(|child| {
                        child.task_ref().same_identity(grandchild_ref)
                            && child.phys() == parent.phys()
                            && child.mm_satp() != parent.mm_satp()
                            && !child.writable()
                            && child.cow()
                            && child.refcount() == 2
                    })
            }),
        );
        if !ctx
            .kernel_init_user_state
            .observe_pending_plain_fork_child_process_group_visible(outer_child_pid, child_pid)
        {
            assertions.assert("builtin grandchild pending identity", false);
            return;
        }
        let next_pid = ctx.user_task_set.next_child_pid();
        let second_pending_rejected = ctx
            .user_task_set
            .copy_plain_fork_from_current_child(
                &ctx.kernel_init_user_state,
                &ctx.user_clone_deferred_boundaries,
                &mut ctx.user_address_space,
                &ctx.user_stack,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &inner_clone,
                USER_PLAIN_FORK_FLAGS,
                0,
            )
            .is_none();
        assertions.assert(
            "builtin grandchild rejects second pending child",
            second_pending_rejected && ctx.user_task_set.next_child_pid() == next_pid,
        );
        child_pid
    };
    if context().files_struct.close_fd(pipe_pair[1]).is_err() {
        assertions.assert("builtin grandchild parent closes pipe writer", false);
        return;
    }

    let mut parent_read_frame = TrapFrame::zeroed();
    parent_read_frame.sepc = 0x9030;
    parent_read_frame.set_reg(2, USER_STACK_TOP - 0xc00);
    parent_read_frame.set_reg(10, pipe_pair[0]);
    let free_before_capture_failure = context().page_allocator.buddy_total_free_pages();
    context()
        .user_task_set
        .smoke_fail_next_builtin_grandchild_wait_capture();
    let failed_handoff = {
        let ctx = context();
        ctx.user_task_set
            .pipe_read_yield_to_builtin_grandchild_continuation(
                &ctx.kernel_init_user_state,
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                &mut ctx.fs_struct,
                &mut ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &parent_read_frame,
            )
            .is_none()
    };
    {
        let ctx = context();
        assertions.assert(
            "builtin grandchild wait capture failure rolls back",
            failed_handoff
                && ctx.page_allocator.buddy_total_free_pages() == free_before_capture_failure
                && ctx.user_task_set.observed_plain_fork_child_pending_wait()
                && !ctx
                    .user_task_set
                    .builtin_grandchild_parent_wait_frame_saved()
                && !ctx
                    .user_task_set
                    .builtin_grandchild_parent_fd_snapshot_saved()
                && !ctx
                    .user_task_set
                    .builtin_grandchild_parent_writable_snapshot_saved(),
        );
    }

    let (inner_child_frame, inner_parent_pid, inner_child_pid) = {
        let ctx = context();
        let Some(handoff) = ctx
            .user_task_set
            .pipe_read_yield_to_builtin_grandchild_continuation(
                &ctx.kernel_init_user_state,
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                &mut ctx.fs_struct,
                &mut ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &parent_read_frame,
            )
        else {
            assertions.assert("builtin grandchild pipe-read handoff", false);
            return;
        };
        handoff
    };
    if !context()
        .kernel_init_user_state
        .switch_observed_child_process_visible(inner_parent_pid, inner_child_pid)
        || !replace_runqueue_parent_with_active()
    {
        assertions.assert("builtin grandchild visible handoff", false);
        return;
    }
    {
        let ctx = context();
        let next_pid = ctx.user_task_set.next_child_pid();
        let deeper_clone_rejected = ctx
            .user_task_set
            .copy_plain_fork_from_current_child(
                &ctx.kernel_init_user_state,
                &ctx.user_clone_deferred_boundaries,
                &mut ctx.user_address_space,
                &ctx.user_stack,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &inner_child_frame,
                USER_PLAIN_FORK_FLAGS,
                0,
            )
            .is_none();
        assertions.assert(
            "builtin grandchild handoff and depth bound",
            inner_child_frame.reg(10) == 0
                && inner_parent_pid == outer_child_pid
                && inner_child_pid == first_grandchild_pid
                && ctx.user_task_set.builtin_grandchild_active()
                && deeper_clone_rejected
                && ctx.user_task_set.next_child_pid() == next_pid
                && ctx.user_task_set.parent_wait_frame_saved()
                && !ctx.user_task_set.parent_wait_stack_snapshot_restored(),
        );
    }
    let script_satp = context().user_address_space.satp_token();
    assertions.assert(
        "builtin grandchild exec precommit failure atomic",
        crate::objects::exec_transaction::smoke_builtin_grandchild_precommit_failure_is_atomic(
            context(),
        ),
    );
    let main_len = USER_INIT_READ_LEN.load(Ordering::Acquire);
    let main_buffer = unsafe { &*core::ptr::addr_of!(USER_INIT_READ_BUFFER) };
    let interpreter_len = USER_INTERPRETER_READ_LEN.load(Ordering::Acquire);
    let interpreter_buffer = unsafe { &*core::ptr::addr_of!(USER_INTERPRETER_READ_BUFFER) };
    if main_len == 0 || main_len > main_buffer.len() || interpreter_len > interpreter_buffer.len() {
        assertions.assert("builtin grandchild exec staged images", false);
        return;
    }
    let interpreter_image =
        (interpreter_len != 0).then_some(&interpreter_buffer[..interpreter_len]);
    let exec_task_ref = context().user_task_set.active_task_ref();
    let fork_flow_ref = context().user_task_set.flow_ref();
    let first_exec = crate::objects::exec_transaction::smoke_commit_builtin_grandchild_exec_image(
        context(),
        &main_buffer[..main_len],
        interpreter_image,
    );
    let first_exec_flow_ref = context().user_task_set.flow_ref();
    let first_exec_closed_child_fd = matches!(
        context().files_struct.fcntl_getfd_fd(cloexec_fd),
        Err(FileError::BadFd)
    );
    assertions.assert(
        "builtin grandchild first exec releases only its retired image",
        first_exec.is_some_and(|success| success.retired_pages_released != 0)
            && first_exec_closed_child_fd
            && context().user_address_space.satp_token() != script_satp
            && !context()
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_saved()
            && !context()
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_ever_saved()
            && !context()
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_restored()
            && context()
                .user_task_set
                .builtin_grandchild_parent_exec_satp()
                == 0
            && context()
                .user_task_set
                .builtin_grandchild_exec_commit_count()
                == 1
            && !context()
                .user_task_set
                .parent_address_space_snapshot_saved()
            && context().user_task_set.active_task_ref() == exec_task_ref
            && fork_flow_ref.same_identity(first_exec_flow_ref)
            && context()
                .user_task_set
                .flow_ref_valid(exec_task_ref, first_exec_flow_ref),
    );
    let first_exec_satp = context().user_address_space.satp_token();
    let second_exec = crate::objects::exec_transaction::smoke_commit_builtin_grandchild_exec_image(
        context(),
        &main_buffer[..main_len],
        interpreter_image,
    );
    let second_exec_flow_ref = context().user_task_set.flow_ref();
    assertions.assert(
        "builtin grandchild consecutive exec releases intermediate image",
        second_exec.is_some_and(|success| success.retired_pages_released != 0)
            && context().user_address_space.satp_token() != first_exec_satp
            && context()
                .user_task_set
                .builtin_grandchild_parent_exec_satp()
                == 0
            && !context()
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_saved()
            && context()
                .user_task_set
                .builtin_grandchild_exec_commit_count()
                == 2
            && !context()
                .user_task_set
                .parent_address_space_snapshot_saved()
            && context().user_task_set.active_task_ref() == exec_task_ref
            && first_exec_flow_ref.same_identity(second_exec_flow_ref)
            && context()
                .user_task_set
                .flow_ref_valid(exec_task_ref, second_exec_flow_ref),
    );
    if context().files_struct.close_fd(pipe_pair[0]).is_err()
        || context().files_struct.write_fd(pipe_pair[1], b"/opt/ltp\n") != Ok(9)
        || context().files_struct.close_fd(pipe_pair[1]).is_err()
    {
        assertions.assert("builtin grandchild child pipe write", false);
        return;
    }

    let restored_parent_frame = {
        let ctx = context();
        let Ok(exiting_task) = ctx.current_task() else {
            assertions.assert("builtin grandchild CurrentTask resolves before exit", false);
            return;
        };
        let Some((parent_frame, _, child_pid, parent_pid)) =
            ctx.user_task_set.child_exit_to_observed_child_parent_wait(
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                (
                    &mut ctx.exec_transaction.retired_address_space,
                    &mut ctx.exec_transaction.retired_stack,
                ),
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                0,
            )
        else {
            assertions.assert("builtin grandchild exits to pipe-read parent", false);
            return;
        };
        if child_pid != first_grandchild_pid || parent_pid != outer_child_pid {
            assertions.assert("builtin grandchild exit identity", false);
            return;
        }
        if ctx
            .replace_terminal_user_task(
                ctx.user_task_set.last_exited_task_ref(),
                ctx.user_task_set.active_task_ref(),
                parent_pid,
                exiting_task,
            )
            .is_err()
        {
            assertions.assert("builtin grandchild restores runqueue TaskRef", false);
            return;
        }
        parent_frame
    };
    {
        let ctx = context();
        let parent_mm_restored = ctx.user_address_space.satp_token() == outer_child_satp
            && ctx
                .user_address_space
                .owned_by(ctx.user_task_set.active_task_ref())
            && !ctx
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_ever_saved()
            && !ctx
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_saved()
            && !ctx
                .user_task_set
                .builtin_grandchild_parent_exec_snapshot_restored();
        if !ctx
            .user_task_set
            .restore_observed_child_parent_fs_snapshot(&mut ctx.fs_struct)
            || !ctx
                .user_task_set
                .restore_parent_fd_snapshot(&mut ctx.files_struct)
            || !ctx
                .kernel_init_user_state
                .restore_observed_child_parent_process_visible(
                    outer_child_pid,
                    first_grandchild_pid,
                )
            || !ctx
                .user_task_set
                .mark_builtin_grandchild_exited_to_parent_read()
            || !parent_mm_restored
        {
            assertions.assert("builtin grandchild restores pipe-read parent", false);
            return;
        }
    }
    assertions.assert(
        "builtin grandchild cloexec restores script fd view",
        context().files_struct.fcntl_getfd_fd(cloexec_fd) == Ok(USER_TEST_FD_CLOEXEC),
    );
    let mut pipe_data = [0u8; 9];
    let pipe_read = context().files_struct.read_fd(pipe_pair[0], &mut pipe_data);
    let pipe_eof = context().files_struct.read_fd(pipe_pair[0], &mut pipe_data);
    if context().files_struct.close_fd(pipe_pair[0]).is_err() {
        assertions.assert("builtin grandchild parent closes pipe reader", false);
        return;
    }
    if context().files_struct.close_fd(cloexec_fd).is_err() {
        assertions.assert(
            "builtin grandchild parent closes restored cloexec fd",
            false,
        );
        return;
    }
    {
        let ctx = context();
        let wait_status = (ctx.user_task_set.child_exit_status() & 0xff) << 8;
        if !ctx
            .user_task_set
            .mark_builtin_grandchild_reaped(wait_status)
            || !ctx.user_task_set.finish_observed_child_parent_restore()
        {
            assertions.assert("builtin grandchild completed wait reap", false);
            return;
        }
    }
    {
        let child = &context().user_task_set;
        assertions.assert(
            "builtin grandchild pipe data and outer ownership",
            restored_parent_frame.sepc == 0x9030
                && pipe_read == Ok(9)
                && pipe_data == *b"/opt/ltp\n"
                && pipe_eof == Ok(0)
                && !child.builtin_grandchild_bound()
                && child.pid1_plain_fork_child_continuation()
                && child.parent_wait_frame_saved()
                && !child.parent_address_space_snapshot_saved()
                && child.parent_fd_snapshot_saved()
                && !child.parent_wait_stack_snapshot_copied()
                && !child.parent_wait_writable_page_snapshot_saved(),
        );
    }

    if !exercise_builtin_grandchild_wait4_exec(
        assertions,
        outer_child_pid,
        outer_child_satp,
        &main_buffer[..main_len],
        interpreter_image,
    ) {
        return;
    }

    let invalid_next_pid = context().user_task_set.next_child_pid();
    let mut invalid_clone = TrapFrame::zeroed();
    invalid_clone.sepc = 0x9040;
    invalid_clone.set_reg(2, USER_STACK_TOP - 0xd00);
    let invalid_flags_rejected = {
        let ctx = context();
        ctx.user_task_set
            .copy_plain_fork_from_current_child(
                &ctx.kernel_init_user_state,
                &ctx.user_clone_deferred_boundaries,
                &mut ctx.user_address_space,
                &ctx.user_stack,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &invalid_clone,
                0,
                0,
            )
            .is_none()
    };
    let invalid_newsp_rejected = {
        let ctx = context();
        ctx.user_task_set
            .copy_plain_fork_from_current_child(
                &ctx.kernel_init_user_state,
                &ctx.user_clone_deferred_boundaries,
                &mut ctx.user_address_space,
                &ctx.user_stack,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &invalid_clone,
                USER_PLAIN_FORK_FLAGS,
                USER_STACK_TOP - 0xe00,
            )
            .is_none()
    };
    assertions.assert(
        "builtin grandchild rejects invalid clone arguments",
        invalid_flags_rejected
            && invalid_newsp_rejected
            && context().user_task_set.next_child_pid() == invalid_next_pid,
    );

    let precheck_task_ref = context().user_task_set.active_task_ref();
    let precheck_flow_ref = context().user_task_set.flow_ref();
    let precheck_free_slots = context().user_task_set.free_task_slot_count();
    let invalid_identity_rejected = !context()
        .kernel_init_user_state
        .can_observe_pending_plain_fork_child_process_group_visible(
            outer_child_pid,
            outer_child_pid,
        );
    assertions.assert(
        "builtin grandchild staging precheck preserves task and flow identity",
        invalid_identity_rejected
            && context().user_task_set.next_child_pid() == invalid_next_pid
            && context().user_task_set.active_task_ref() == precheck_task_ref
            && context().user_task_set.flow_ref() == precheck_flow_ref
            && context().user_task_set.free_task_slot_count() == precheck_free_slots
            && !context().user_task_set.builtin_grandchild_bound()
            && context().user_task_set.pid1_plain_fork_child_continuation(),
    );

    {
        let ctx = context();
        let active = ctx.user_task_set.active_task_ref();
        assertions.assert(
            "task OOM terminal encodes signal 9 wait status",
            ctx.user_task_set.record_task_out_of_memory(active)
                && ctx.user_task_set.task_wait_status(active, 0) == 9,
        );
    }

    let (outer_parent_frame, outer_exit_pid, exiting_task) = {
        let ctx = context();
        let Ok(exiting_task) = ctx.current_task() else {
            assertions.assert("plain child CurrentTask resolves before exit", false);
            return;
        };
        let Some((parent_frame, _, child_pid)) = ctx.user_task_set.child_exit_to_parent_wait(
            &mut ctx.user_address_space,
            &mut ctx.user_stack,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            0,
        ) else {
            assertions.assert("pid1 plain child exits to outer parent", false);
            return;
        };
        (parent_frame, child_pid, exiting_task)
    };
    {
        let ctx = context();
        if !ctx
            .user_task_set
            .restore_parent_fd_snapshot(&mut ctx.files_struct)
            || !ctx.user_task_set.mark_parent_wait_resumed(true)
            || ctx
                .commit_terminal_kernel_init_dispatch(exiting_task)
                .is_err()
            || {
                let exited_ref = ctx.user_task_set.last_exited_task_ref();
                ctx.scheduler_mut()
                    .dequeue_user_child_from_runqueue(exited_ref)
            }
            .is_err()
            || !ctx.user_task_set.release_reaped_active_task_record()
        {
            assertions.assert("pid1 plain child restores outer parent", false);
            return;
        }
    }
    assertions.assert(
        "pid1 plain child two-level completion",
        outer_exit_pid == outer_child_pid
            && outer_parent_frame.sepc == 0x9010
            && context().user_task_set.active_task_record_available(),
    );
}

fn exercise_builtin_grandchild_wait4_exec(
    assertions: &mut SmokeAssertions,
    outer_child_pid: usize,
    script_satp: usize,
    main_image: &[u8],
    interpreter_image: Option<&[u8]>,
) -> bool {
    let wait_grandchild_pid = {
        let mut wait_clone = TrapFrame::zeroed();
        wait_clone.sepc = 0x9038;
        wait_clone.set_reg(2, USER_STACK_TOP - 0xc80);
        let ctx = context();
        let Some(pid) = ctx.user_task_set.copy_plain_fork_from_current_child(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
            &ctx.user_stack,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &wait_clone,
            USER_PLAIN_FORK_FLAGS,
            0,
        ) else {
            assertions.assert("builtin grandchild wait4 round clone", false);
            return false;
        };
        if !ctx
            .kernel_init_user_state
            .observe_pending_plain_fork_child_process_group_visible(outer_child_pid, pid)
        {
            assertions.assert("builtin grandchild wait4 round identity", false);
            return false;
        }
        pid
    };
    let wait_child_frame = {
        let mut wait_frame = TrapFrame::zeroed();
        wait_frame.sepc = 0x903c;
        wait_frame.set_reg(2, USER_STACK_TOP - 0xcc0);
        let ctx = context();
        let Some((child_frame, parent_pid, child_pid)) = ctx
            .user_task_set
            .wait4_yield_to_observed_child_continuation(
                &ctx.kernel_init_user_state,
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                &mut ctx.fs_struct,
                &mut ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                &wait_frame,
                0,
                USER_WAIT4_ALL_CHILDREN,
                0,
            )
        else {
            assertions.assert("builtin grandchild wait4 round handoff", false);
            return false;
        };
        if !ctx
            .kernel_init_user_state
            .switch_observed_child_process_visible(parent_pid, child_pid)
            || ctx
                .replace_current_user_task(
                    ctx.user_task_set.parent_task_ref(),
                    ctx.user_task_set.active_task_ref(),
                    child_pid,
                )
                .is_err()
        {
            assertions.assert("builtin grandchild wait4 round visible", false);
            return false;
        }
        child_frame
    };
    let wait_exec = crate::objects::exec_transaction::smoke_commit_builtin_grandchild_exec_image(
        context(),
        main_image,
        interpreter_image,
    );
    let wait_parent_frame = {
        let ctx = context();
        let Ok(exiting_task) = ctx.current_task() else {
            assertions.assert(
                "builtin grandchild wait4 CurrentTask resolves before exit",
                false,
            );
            return false;
        };
        let Some((parent_frame, _, child_pid, parent_pid)) =
            ctx.user_task_set.child_exit_to_observed_child_parent_wait(
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                (
                    &mut ctx.exec_transaction.retired_address_space,
                    &mut ctx.exec_transaction.retired_stack,
                ),
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                0,
            )
        else {
            assertions.assert("builtin grandchild exec exits to wait4 parent", false);
            return false;
        };
        if child_pid != wait_grandchild_pid || parent_pid != outer_child_pid {
            assertions.assert("builtin grandchild wait4 exit identity", false);
            return false;
        }
        if ctx
            .replace_terminal_user_task(
                ctx.user_task_set.last_exited_task_ref(),
                ctx.user_task_set.active_task_ref(),
                parent_pid,
                exiting_task,
            )
            .is_err()
        {
            assertions.assert("builtin grandchild wait4 restores runqueue TaskRef", false);
            return false;
        }
        parent_frame
    };
    {
        let ctx = context();
        if !ctx
            .user_task_set
            .restore_observed_child_parent_fs_snapshot(&mut ctx.fs_struct)
            || !ctx
                .user_task_set
                .restore_parent_fd_snapshot(&mut ctx.files_struct)
            || !ctx
                .kernel_init_user_state
                .restore_observed_child_parent_process_visible(outer_child_pid, wait_grandchild_pid)
            || !ctx
                .user_task_set
                .mark_observed_child_parent_wait_resumed(true, 0)
            || !ctx.user_task_set.finish_observed_child_parent_restore()
        {
            assertions.assert("builtin grandchild exec restores wait4 parent", false);
            return false;
        }
    }
    assertions.assert(
        "builtin grandchild exec wait4 resume preserves outer owner",
        wait_child_frame.reg(10) == 0
            && wait_exec.is_some_and(|success| success.retired_pages_released != 0)
            && wait_parent_frame.sepc == 0x903c
            && context().user_address_space.satp_token() == script_satp
            && context().user_task_set.pid1_plain_fork_child_continuation()
            && !context()
                .user_task_set
                .parent_address_space_snapshot_saved(),
    );
    true
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
                let signal_recorded = ctx.kernel_init_user_state.record_child_exit_sigchld();
                let signal_result = ctx.kernel_init_user_state.begin_rt_sigtimedwait(
                    USER_SIGCHLD_MASK,
                    true,
                    true,
                    &sigwait_frame,
                );
                signal_recorded == Some(false)
                    && signal_result == UserRtSigtimedwaitResult::ReturnSignal(USER_CLONE_SIGCHLD)
                    && ctx.user_task_set.completed_child_record_count() == 1
                    && ctx
                        .user_task_set
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
                    && ctx.user_task_set.completed_child_record_count() == 1
                    && ctx.user_task_set.completed_child_record_reaped_count() == 0
                    && ctx.user_task_set.completed_child_record_released_count() == 0
                    && ctx
                        .user_task_set
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
                && ctx.user_task_set.completed_child_record_count() == 0
                && ctx.user_task_set.completed_child_record_free_count()
                    == USER_COMPLETED_CHILD_RECORD_CAPACITY
                && ctx.user_task_set.last_reaped_child_pid() == child_pid
                && ctx.user_task_set.last_released_child_pid() == child_pid,
        );

        index += 1;
    }

    let ctx = context();
    assertions.assert(
        "sequential vfork exceeds completed record capacity",
        ctx.user_task_set.completed_child_record_total_archived()
            > USER_COMPLETED_CHILD_RECORD_CAPACITY
            && ctx.user_task_set.completed_child_record_released_count()
                > USER_COMPLETED_CHILD_RECORD_CAPACITY
            && ctx.user_task_set.next_child_pid()
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
            && ctx.user_task_set.completed_child_record_count() == 0
            && ctx.user_task_set.first_unreaped_completed_child().is_none(),
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
        ctx.user_task_set.copy_vfork_from_parent(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
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
        let child_pid = ctx.user_task_set.pid();
        let child_task = ctx.user_task_set.active_task_ref();
        let runqueue_ref = ctx
            .scheduler_mut()
            .select_scheduler_for_task(child_pid)
            .ok()?;
        ctx.scheduler_mut()
            .enqueue_task_on_scheduler(child_pid, child_task, runqueue_ref)
            .ok()?;
        if !ctx.user_task_set.mark_enqueued() {
            return None;
        }
        ctx.commit_user_dispatch(child_task).ok()?;
    }

    let (child_pid, exiting_task) = {
        let ctx = context();
        let exiting_task = ctx.current_task().ok()?;
        let (_parent_frame, child_pid) = ctx.user_task_set.child_exit_to_vfork_parent(
            &mut ctx.user_address_space,
            &mut ctx.user_stack,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            index,
        )?;
        (child_pid, exiting_task)
    };

    {
        let ctx = context();
        if ctx
            .commit_terminal_kernel_init_dispatch(exiting_task)
            .is_err()
        {
            return None;
        }
        let exited_ref = ctx.user_task_set.last_exited_task_ref();
        if ctx
            .scheduler_mut()
            .dequeue_user_child_from_runqueue(exited_ref)
            .is_err()
        {
            return None;
        }
        if !ctx
            .user_task_set
            .restore_parent_wait_writable_page_snapshot(
                &ctx.user_address_space,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
        {
            return None;
        }
        if !ctx
            .user_task_set
            .restore_parent_fd_snapshot(&mut ctx.files_struct)
        {
            return None;
        }
        if !ctx.user_task_set.mark_vfork_parent_resumed() {
            return None;
        }
        if !ctx.user_task_set.archive_completed_child_record() {
            return None;
        }
        if !ctx.user_task_set.release_active_task_record() {
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

fn exercise_nested_vfork_task_record(assertions: &mut SmokeAssertions) {
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
            ctx.user_task_set.copy_nested_vfork_from_current_child(
                &ctx.kernel_init_user_state,
                &ctx.user_clone_deferred_boundaries,
                &mut ctx.user_address_space,
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
        let child_pid = ctx.user_task_set.pid();
        if !ctx
            .kernel_init_user_state
            .observe_nested_child_process_group_visible(parent_pid, child_pid)
            || ctx
                .replace_current_user_task(
                    ctx.user_task_set.parent_task_ref(),
                    ctx.user_task_set.active_task_ref(),
                    child_pid,
                )
                .is_err()
        {
            assertions.assert("nested child process group visible", false);
            return;
        }
        child_pid
    };

    let ctx = context();
    assertions.assert(
        "nested vfork allocates a fresh child identity",
        ctx.user_task_set.nested_vfork_clone()
            && ctx.user_task_set.nested_vfork_parent_pid() == parent_pid
            && nested_child_pid > parent_pid
            && ctx.user_task_set.current_child_continuation()
            && ctx.user_task_set.vfork_child_handoff()
            && !ctx.user_task_set.vfork_parent_resume_on_exit()
            && ctx.user_task_set.pidfd_fd() == usize::MAX
            && ctx.user_task_set.next_child_pid() > nested_child_pid
            && ctx.user_task_set.task_allocation_count() >= 2,
    );

    let rejects_deeper_nested = !copy_user_process_for_current_slot(true);
    assertions.assert("deeper nested vfork rejected", rejects_deeper_nested);
}

fn exercise_observed_child_plain_fork(assertions: &mut SmokeAssertions) {
    let (shell_pid, shell_parent_pid, shell_tgid, shell_pgrp, shell_session_id) = {
        let ctx = context();
        (
            ctx.user_task_set.pid(),
            ctx.user_task_set.parent_pid(),
            ctx.user_task_set.tgid(),
            ctx.kernel_init_user_state.child_process_group(),
            ctx.kernel_init_user_state.child_process_session_id(),
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
        let Some(child_pid) = ctx.user_task_set.copy_plain_fork_from_current_child(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
            &ctx.user_stack,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &clone_frame,
            USER_PLAIN_FORK_FLAGS,
            0,
        ) else {
            assertions.assert("copy observed child plain fork", false);
            return;
        };
        if !ctx
            .kernel_init_user_state
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
                && ctx.user_task_set.pid() == shell_pid
                && ctx.user_task_set.parent_pid() == shell_parent_pid
                && ctx.user_task_set.observed_plain_fork_clone()
                && ctx.user_task_set.observed_plain_fork_parent_pid() == shell_pid
                && ctx.user_task_set.observed_plain_fork_child_pid() == child_pid
                && ctx.user_task_set.observed_plain_fork_child_pending_wait()
                && ctx.user_task_set.child_trap_frame_reg(10) == Some(0)
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_visible()
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_parent_pid()
                    == shell_pid
                && ctx.kernel_init_user_state.pending_plain_fork_child_pid() == child_pid
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_process_group()
                    == shell_pgrp
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_session_id()
                    == shell_session_id,
        );
    }

    let (unknown_pid, negative_pgid, invalid_pgrp, pending_update) = {
        let ctx = context();
        (
            ctx.kernel_init_user_state.set_process_group_first_slice(
                child_pid + 100,
                child_pid + 100,
                true,
            ),
            ctx.kernel_init_user_state
                .set_process_group_first_slice(child_pid, usize::MAX, true),
            ctx.kernel_init_user_state.set_process_group_first_slice(
                child_pid,
                child_pid + 1,
                true,
            ),
            ctx.kernel_init_user_state
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
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_visible()
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_process_group()
                    == child_pid
                && ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_session_id()
                    == shell_session_id
                && ctx
                    .kernel_init_user_state
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
            .user_task_set
            .wait4_yield_to_observed_child_continuation(
                &ctx.kernel_init_user_state,
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                &mut ctx.fs_struct,
                &mut ctx.files_struct,
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
            .kernel_init_user_state
            .switch_observed_child_process_visible(parent_pid, observed_child_pid)
            || ctx
                .replace_current_user_task(
                    ctx.user_task_set.parent_task_ref(),
                    ctx.user_task_set.active_task_ref(),
                    observed_child_pid,
                )
                .is_err()
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
                && ctx.user_task_set.pid() == child_pid
                && ctx.user_task_set.parent_pid() == shell_pid
                && ctx.user_task_set.observed_plain_fork_child_active()
                && ctx.kernel_init_user_state.child_process_pid() == child_pid
                && ctx.kernel_init_user_state.child_process_group() == child_pid
                && ctx.kernel_init_user_state.child_process_session_id() == shell_session_id
                && !ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_visible(),
        );
    }

    let (parent_frame, exit_child_pid, exit_parent_pid) = {
        let ctx = context();
        let Ok(exiting_task) = ctx.current_task() else {
            assertions.assert("observed child CurrentTask resolves before exit", false);
            return;
        };
        let Some((parent_frame, _status_ptr, exit_child_pid, exit_parent_pid)) =
            ctx.user_task_set.child_exit_to_observed_child_parent_wait(
                &mut ctx.user_address_space,
                &mut ctx.user_stack,
                (
                    &mut ctx.exec_transaction.retired_address_space,
                    &mut ctx.exec_transaction.retired_stack,
                ),
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                0,
            )
        else {
            assertions.assert("observed child exits to shell wait", false);
            return;
        };
        if ctx
            .replace_terminal_user_task(
                ctx.user_task_set.last_exited_task_ref(),
                ctx.user_task_set.active_task_ref(),
                exit_parent_pid,
                exiting_task,
            )
            .is_err()
        {
            assertions.assert("observed child restores shell runqueue TaskRef", false);
            return;
        }
        (parent_frame, exit_child_pid, exit_parent_pid)
    };
    if exit_child_pid != child_pid || exit_parent_pid != shell_pid {
        assertions.assert("observed child exit pids", false);
        return;
    }

    {
        let ctx = context();
        if !ctx
            .user_task_set
            .restore_parent_fd_snapshot(&mut ctx.files_struct)
            || !ctx
                .kernel_init_user_state
                .restore_observed_child_parent_process_visible(shell_pid, child_pid)
            || !ctx
                .user_task_set
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
                && ctx.user_task_set.pid() == shell_pid
                && ctx.user_task_set.parent_pid() == shell_parent_pid
                && ctx.user_task_set.tgid() == shell_tgid
                && ctx.user_task_set.current_child_continuation()
                && ctx.user_task_set.observed_plain_fork_parent_restored()
                && ctx.user_task_set.last_reaped_child_pid() == child_pid
                && ctx.kernel_init_user_state.child_process_pid() == shell_pid
                && ctx.kernel_init_user_state.child_process_group() == shell_pgrp
                && ctx.kernel_init_user_state.child_process_session_id() == shell_session_id
                && !ctx
                    .kernel_init_user_state
                    .pending_plain_fork_child_visible(),
        );
    }

    let first_round_cleared = context()
        .user_task_set
        .finish_observed_child_parent_restore();
    {
        let ctx = context();
        assertions.assert(
            "observed plain fork finishes completed round",
            first_round_cleared,
        );
        assertions.assert(
            "observed plain fork preserves shell identity",
            ctx.user_task_set.active_task_state() == State::OnCpu
                && ctx.user_task_set.pid() == shell_pid
                && ctx.user_task_set.parent_pid() == shell_parent_pid
                && ctx.user_task_set.tgid() == shell_tgid,
        );
        assertions.assert(
            "observed plain fork preserves shell continuation",
            ctx.user_task_set.current_child_continuation() && ctx.user_task_set.enqueued(),
        );
        assertions.assert(
            "observed plain fork preserves shell runqueue visibility",
            ctx.scheduler()
                .contains_task_ref(ctx.user_task_set.active_task_ref()),
        );
        assertions.assert(
            "observed plain fork clears completed round snapshots",
            !ctx.user_task_set.observed_plain_fork_clone()
                && !ctx.user_task_set.child_exit_status_observed()
                && !ctx.user_task_set.parent_wait_frame_saved()
                && !ctx.user_task_set.parent_address_space_snapshot_saved()
                && !ctx.user_task_set.parent_wait_stack_snapshot_copied(),
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
        let Some(second_child_pid) = ctx.user_task_set.copy_plain_fork_from_current_child(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
            &ctx.user_stack,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            &clone_frame,
            USER_PLAIN_FORK_FLAGS,
            0,
        ) else {
            assertions.assert("copy second observed child plain fork", false);
            return;
        };
        if !ctx
            .kernel_init_user_state
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
                && ctx.user_task_set.pid() == shell_pid
                && ctx.user_task_set.parent_pid() == shell_parent_pid
                && ctx.user_task_set.tgid() == shell_tgid
                && ctx.user_task_set.current_child_continuation()
                && ctx.user_task_set.enqueued()
                && ctx.user_task_set.observed_plain_fork_child_pid() == second_child_pid
                && ctx.user_task_set.observed_plain_fork_child_pending_wait()
                && ctx.user_task_set.next_child_pid() > second_child_pid
                && ctx.kernel_init_user_state.pending_plain_fork_child_pid() == second_child_pid,
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
        ctx.user_task_set.copy_vfork_from_parent(
            &ctx.kernel_init_user_state,
            &ctx.user_clone_deferred_boundaries,
            &mut ctx.user_address_space,
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
        ctx.user_task_set.pid()
    };

    {
        let ctx = context();
        if !ctx.scheduler().contains_task(USER_CHILD_PID) {
            let task_pid = ctx.user_task_set.pid();
            let task_ref = ctx.user_task_set.active_task_ref();
            let runqueue_ref = ctx
                .scheduler_mut()
                .select_scheduler_for_task(task_pid)
                .ok()?;
            ctx.scheduler_mut()
                .enqueue_task_on_scheduler(task_pid, task_ref, runqueue_ref)
                .ok()?;
        }
        if !ctx.user_task_set.mark_enqueued() {
            return None;
        }
        if !ctx
            .kernel_init_user_state
            .observe_child_process_group_visible(child_pid)
        {
            return None;
        }
        let child_task = ctx.user_task_set.active_task_ref();
        ctx.commit_user_dispatch(child_task).ok()?;
    }
    Some(child_pid)
}

fn copy_user_process_for_current_slot(allow_nested_vfork: bool) -> bool {
    let ctx = context();
    ctx.task_creation_core
        .copy_user_process(
            TaskCopyUserProcessInputs {
                src_process: &ctx.kernel_init_user_state,
                dst_process: &ctx.user_task_set,
                root_pid_namespace: &ctx.root_pid_namespace,
                scheduler: ctx
                    .cpu_group
                    .boot_scheduler()
                    .expect("CPU0 Scheduler must be published"),
                cpu_group: &ctx.cpu_group,
                fs_struct: &ctx.fs_struct,
                files_struct: &ctx.files_struct,
                address_space: &ctx.user_address_space,
                trap_frame: &ctx.user_trap_frame,
                boundaries: &ctx.user_clone_deferred_boundaries,
                entry: TaskEntry::UserChild,
                allow_nested_vfork,
            },
            ctx.user_task_set.active_task_state(),
            TaskEntry::UserChild,
        )
        .is_ok()
}

fn replace_runqueue_parent_with_active() -> bool {
    let ctx = context();
    ctx.replace_current_user_task(
        ctx.user_task_set.parent_task_ref(),
        ctx.user_task_set.active_task_ref(),
        ctx.user_task_set.pid(),
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
