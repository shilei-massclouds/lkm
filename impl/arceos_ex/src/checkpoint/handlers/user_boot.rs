use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::user_boot::{ElfError, ElfObject, USER_INIT_EXPECTED_MESSAGE},
    trace::Checkpoint,
};

#[cfg(app_user_boot)]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(app_user_boot)]
use crate::objects::user_boot::{
    user_kernel_trap_overflow_stack_base, user_kernel_trap_overflow_stack_ready,
    user_kernel_trap_overflow_stack_top, user_kernel_trap_stack_backing_order,
    user_kernel_trap_stack_backing_phys, user_kernel_trap_stack_base,
    user_kernel_trap_stack_base_aligned, user_kernel_trap_stack_guard_base,
    user_kernel_trap_stack_guard_size, user_kernel_trap_stack_guard_unmapped,
    user_kernel_trap_stack_ready, user_kernel_trap_stack_top, user_kernel_trap_stack_vmapped,
    UserStack, USER_KERNEL_IRQ_STACK_SIZE, USER_KERNEL_TRAP_ENTRY_SCRATCH_DEFERRED,
    USER_KERNEL_TRAP_GUARD_PAGE_READY, USER_KERNEL_TRAP_GUARD_SIZE, USER_KERNEL_TRAP_IRQ_STACKS,
    USER_KERNEL_TRAP_IRQ_STACK_SWITCH_DEFERRED, USER_KERNEL_TRAP_OVERFLOW_STACK_READY,
    USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE, USER_KERNEL_TRAP_STACK_ALIGN,
    USER_KERNEL_TRAP_STACK_ORDER, USER_KERNEL_TRAP_STACK_SIZE,
    USER_KERNEL_TRAP_THREAD_INFO_IN_TASK, USER_KERNEL_TRAP_VMAP_STACK, USER_PAGE_SIZE,
};

#[cfg(app_user_boot)]
use crate::objects::{
    exception_stream::execve_checkpoint_observation,
    files::{FdRef, FileBackendKind},
    state::State,
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::PayloadPhaseOnline,
    #[cfg(app_user_boot)]
    Checkpoint::UserBootMainElfReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserBootInterpreterReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserBootAddressSpaceSetupStart,
    #[cfg(app_user_boot)]
    Checkpoint::UserAddressSpaceReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserModeEntry,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableExecveArgsReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecMainElfReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecInterpreterReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecAddressSpaceReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecTrapFrameReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecSatpReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecContextReplaced,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecSatpSwitched,
    #[cfg(app_user_boot)]
    Checkpoint::UserExecReturnFrameReady,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableSetTidAddress,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableOpenAt,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableRead,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableWrite,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableClose,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableNewFstatAt,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableExit,
];
#[cfg(app_user_boot)]
pub const KUNIT_CASE_COUNT: usize = 19;
#[cfg(not(app_user_boot))]
pub const KUNIT_CASE_COUNT: usize = 5;

#[cfg(app_user_boot)]
static SYSCALL_SET_TID_ADDRESS_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_OPENAT_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_READ_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static FILES_READONLY_PATH_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_WRITE_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static FILES_WRITE_PATH_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_CLOSE_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_NEWFSTATAT_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_EXIT_REPORTED: AtomicBool = AtomicBool::new(false);

pub const HANDLER: Handler = Handler {
    name: "user_boot.elf_parser",
    priority: 120,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    #[cfg(not(app_user_boot))]
    let _ = ctx;

    let total = super::kunit_case_count();
    match checkpoint {
        Checkpoint::PayloadPhaseOnline => {
            run_valid_fixture(checkpoint, sink, total);
            run_bad_magic(checkpoint, sink, total);
            run_wrong_machine(checkpoint, sink, total);
            run_invalid_segment(checkpoint, sink, total);
            run_bss_plan(checkpoint, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserBootMainElfReady
        | Checkpoint::UserBootInterpreterReady
        | Checkpoint::UserBootAddressSpaceSetupStart => {
            run_user_boot_phase_trace(checkpoint, ctx, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserAddressSpaceReady => {
            run_user_address_space_ready(checkpoint, ctx, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserModeEntry => {
            run_user_mode_entry(checkpoint, ctx, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableExecveArgsReady
        | Checkpoint::UserExecMainElfReady
        | Checkpoint::UserExecInterpreterReady
        | Checkpoint::UserExecAddressSpaceReady
        | Checkpoint::UserExecTrapFrameReady
        | Checkpoint::UserExecSatpReady
        | Checkpoint::UserExecContextReplaced
        | Checkpoint::UserExecSatpSwitched
        | Checkpoint::UserExecReturnFrameReady => {
            emit_execve_checkpoint_diag(checkpoint, sink);
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableSetTidAddress => {
            run_once(&SYSCALL_SET_TID_ADDRESS_REPORTED, || {
                run_syscall_table_set_tid_address(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableOpenAt => {
            run_once(&SYSCALL_OPENAT_REPORTED, || {
                run_syscall_table_openat(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableRead => {
            run_once(&SYSCALL_READ_REPORTED, || {
                run_syscall_table_read(checkpoint, ctx, sink, total)
            });
            run_once(&FILES_READONLY_PATH_REPORTED, || {
                run_files_struct_readonly_path(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableWrite => {
            run_once(&SYSCALL_WRITE_REPORTED, || {
                run_syscall_table_write(checkpoint, ctx, sink, total)
            });
            run_once(&FILES_WRITE_PATH_REPORTED, || {
                run_files_struct_write_path(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableClose => {
            run_once(&SYSCALL_CLOSE_REPORTED, || {
                run_syscall_table_close(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableNewFstatAt => {
            run_once(&SYSCALL_NEWFSTATAT_REPORTED, || {
                run_syscall_table_newfstatat(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableExit => {
            run_once(&SYSCALL_EXIT_REPORTED, || {
                run_syscall_table_exit(checkpoint, ctx, sink, total)
            });
        }
        _ => {}
    }
    CheckpointOutcome::Continue
}

#[cfg(app_user_boot)]
fn run_once(reported: &AtomicBool, run_case: impl FnOnce()) {
    if !reported.swap(true, Ordering::AcqRel) {
        run_case();
    }
}

#[cfg(app_user_boot)]
fn emit_execve_checkpoint_diag(checkpoint: Checkpoint, sink: &mut dyn Sink) {
    let _ = checkpoint;
    let obs = execve_checkpoint_observation();
    sink.diag_usize("execve_stage", obs.stage);
    sink.diag_usize("execve_filename_len", obs.filename_len);
    sink.diag_usize("execve_argv0_len", obs.argv0_len);
    sink.diag_usize("execve_main_elf_type", obs.main_elf_type);
    sink.diag_usize("execve_main_input_len", obs.main_input_len);
    sink.diag_usize("execve_main_segments", obs.main_segments);
    sink.diag_usize(
        "execve_main_interpreter_required",
        obs.main_interpreter_required,
    );
    sink.diag_hex_pair(
        "execve_main_load_bias_entry",
        obs.main_load_bias,
        obs.main_entry,
    );
    sink.diag_hex_pair(
        "execve_main_entry_runtime",
        obs.main_entry,
        obs.main_runtime_entry,
    );
    sink.diag_usize("execve_interpreter_input_len", obs.interpreter_input_len);
    sink.diag_usize("execve_interpreter_segments", obs.interpreter_segments);
    sink.diag_hex_pair(
        "execve_interpreter_load_bias_entry",
        obs.interpreter_load_bias,
        obs.interpreter_entry,
    );
    sink.diag_usize("execve_address_space_state", obs.address_space_state);
    sink.diag_usize("execve_mapping_count", obs.mapping_count);
    sink.diag_usize("execve_segment_mapping_count", obs.segment_mapping_count);
    sink.diag_hex_pair("execve_satp_old_new", obs.old_satp, obs.new_satp);
    sink.diag_hex_pair(
        "execve_satp_current_token",
        obs.current_satp,
        obs.satp_token,
    );
    sink.diag_hex_pair("execve_trap_entry_sp", obs.trap_entry, obs.trap_sp);
    sink.diag_usize("execve_trap_sstatus", obs.trap_sstatus);
    sink.diag_hex_pair(
        "execve_frame_before_sepc_sp",
        obs.frame_before_sepc,
        obs.frame_before_sp,
    );
    sink.diag_hex_pair(
        "execve_frame_before_ra_sstatus",
        obs.frame_before_ra,
        obs.frame_before_sstatus,
    );
    sink.diag_hex_pair(
        "execve_frame_after_sepc_sp",
        obs.frame_after_sepc,
        obs.frame_after_sp,
    );
    sink.diag_hex_pair(
        "execve_frame_after_ra_sstatus",
        obs.frame_after_ra,
        obs.frame_after_sstatus,
    );
    sink.diag_usize("execve_kernel_sp", obs.kernel_sp);
}

#[cfg(app_user_boot)]
fn run_user_boot_phase_trace(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = match checkpoint {
        Checkpoint::UserBootMainElfReady => "user_boot.main_elf_ready",
        Checkpoint::UserBootInterpreterReady => "user_boot.interpreter_ready",
        Checkpoint::UserBootAddressSpaceSetupStart => "user_boot.address_space_setup_start",
        _ => "user_boot.phase_trace",
    };
    sink.start_case(total, "", name, checkpoint);
    emit_user_boot_phase_diag(ctx, sink);
    sink.pass(total, "", name);
}

#[cfg(app_user_boot)]
fn emit_user_boot_phase_diag(ctx: &Context, sink: &mut dyn Sink) {
    let elf = &ctx.elf_object;
    let interpreter = &ctx.elf_interpreter_object;
    sink.diag_usize("main_elf_state", elf.state() as usize);
    sink.diag_usize("main_elf_type", elf.elf_type_index());
    sink.diag_usize("main_elf_input_len", elf.input_len());
    sink.diag_usize("main_elf_load_bias", elf.load_bias());
    sink.diag_usize(
        "main_elf_pie_supported",
        elf.et_dyn_pie_main_supported() as usize,
    );
    sink.diag_usize(
        "main_elf_pie_load_bias_bound",
        elf.main_pie_load_bias_bound() as usize,
    );
    sink.diag_usize("main_elf_segments", elf.load_segment_count());
    sink.diag_usize(
        "main_elf_interpreter_required",
        elf.interpreter_required() as usize,
    );
    sink.diag_usize("main_elf_entry", elf.entry());
    sink.diag_usize("main_elf_phdr", elf.phdr_vaddr());
    sink.diag_usize("main_elf_runtime_entry", elf.runtime_entry());
    sink.diag_usize("interpreter_state", interpreter.state() as usize);
    sink.diag_usize("interpreter_input_len", interpreter.input_len());
    sink.diag_usize("interpreter_load_bias", interpreter.load_bias());
    sink.diag_usize("interpreter_segments", interpreter.load_segment_count());
    sink.diag_usize("interpreter_entry", interpreter.entry());
    emit_queue_diag(
        "blk",
        ctx.virtio_blk_runtime.device().map(|device| device.queue()),
        sink,
    );
    emit_queue_diag(
        "rng",
        ctx.virtio_rng_runtime.device().map(|device| device.queue()),
        sink,
    );
}

#[cfg(app_user_boot)]
fn emit_queue_diag(
    prefix: &str,
    queue: Option<&crate::objects::virtio_ring::VirtQueue>,
    sink: &mut dyn Sink,
) {
    let Some(queue) = queue else {
        diag_queue_usize(sink, prefix, "queue_present", 0);
        return;
    };
    let layout = queue.real_layout();
    diag_queue_usize(sink, prefix, "queue_present", 1);
    diag_queue_usize(sink, prefix, "submitted_count", queue.submitted_count());
    diag_queue_usize(sink, prefix, "kick_count", queue.kick_count());
    diag_queue_usize(sink, prefix, "completion_count", queue.completion_count());
    diag_queue_usize(sink, prefix, "get_buf_count", queue.get_buf_count());
    diag_queue_usize(
        sink,
        prefix,
        "ring_avail_idx",
        queue.ring_avail_idx() as usize,
    );
    diag_queue_usize(
        sink,
        prefix,
        "ring_used_idx",
        queue.ring_used_idx() as usize,
    );
    diag_queue_usize(
        sink,
        prefix,
        "ring_last_used_idx",
        queue.ring_last_used_idx() as usize,
    );
    diag_queue_usize(
        sink,
        prefix,
        "raw_avail_idx",
        queue.raw_avail_idx().unwrap_or(u16::MAX) as usize,
    );
    diag_queue_usize(
        sink,
        prefix,
        "raw_used_idx",
        queue.raw_used_idx().unwrap_or(u16::MAX) as usize,
    );
    diag_queue_hex_pair(
        sink,
        prefix,
        "used_virt_phys",
        layout.used_virt(),
        layout.used_phys(),
    );
}

#[cfg(app_user_boot)]
fn diag_queue_usize(sink: &mut dyn Sink, prefix: &str, field: &str, value: usize) {
    match (prefix, field) {
        ("blk", "queue_present") => sink.diag_usize("blk_queue_present", value),
        ("blk", "submitted_count") => sink.diag_usize("blk_submitted_count", value),
        ("blk", "kick_count") => sink.diag_usize("blk_kick_count", value),
        ("blk", "completion_count") => sink.diag_usize("blk_completion_count", value),
        ("blk", "get_buf_count") => sink.diag_usize("blk_get_buf_count", value),
        ("blk", "ring_avail_idx") => sink.diag_usize("blk_ring_avail_idx", value),
        ("blk", "ring_used_idx") => sink.diag_usize("blk_ring_used_idx", value),
        ("blk", "ring_last_used_idx") => sink.diag_usize("blk_ring_last_used_idx", value),
        ("blk", "raw_avail_idx") => sink.diag_usize("blk_raw_avail_idx", value),
        ("blk", "raw_used_idx") => sink.diag_usize("blk_raw_used_idx", value),
        ("rng", "queue_present") => sink.diag_usize("rng_queue_present", value),
        ("rng", "submitted_count") => sink.diag_usize("rng_submitted_count", value),
        ("rng", "kick_count") => sink.diag_usize("rng_kick_count", value),
        ("rng", "completion_count") => sink.diag_usize("rng_completion_count", value),
        ("rng", "get_buf_count") => sink.diag_usize("rng_get_buf_count", value),
        ("rng", "ring_avail_idx") => sink.diag_usize("rng_ring_avail_idx", value),
        ("rng", "ring_used_idx") => sink.diag_usize("rng_ring_used_idx", value),
        ("rng", "ring_last_used_idx") => sink.diag_usize("rng_ring_last_used_idx", value),
        ("rng", "raw_avail_idx") => sink.diag_usize("rng_raw_avail_idx", value),
        ("rng", "raw_used_idx") => sink.diag_usize("rng_raw_used_idx", value),
        _ => {}
    }
}

#[cfg(app_user_boot)]
fn diag_queue_hex_pair(
    sink: &mut dyn Sink,
    prefix: &str,
    field: &str,
    first: usize,
    second: usize,
) {
    match (prefix, field) {
        ("blk", "used_virt_phys") => sink.diag_hex_pair("blk_used_virt_phys", first, second),
        ("rng", "used_virt_phys") => sink.diag_hex_pair("rng_used_virt_phys", first, second),
        _ => {}
    }
}

#[cfg(app_user_boot)]
fn run_user_address_space_ready(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_address_space.ready";
    sink.start_case(total, "", name, checkpoint);

    let space = &ctx.user_address_space;
    let stack = &ctx.user_stack;
    let selected_path = ctx.user_boot_payload.selected_path();
    let selected_path_bytes = ctx.user_boot_payload.selected_path_bytes();
    let argv0_matches_selected =
        stack_contains_at(stack, ctx, stack.arg0_ptr(), selected_path_bytes, true);
    let mut valid = space.state() == State::Ready
        && space.page_table_view_ready()
        && space.elf_segments_mapped()
        && space.stack_mapped()
        && space.heap_mapped()
        && ctx.user_boot_payload.selected_path_bound()
        && ctx.user_boot_payload.selected_argv0_path_bound()
        && argv0_matches_selected;

    sink.diag_usize("selected_path_index", selected_path.index());
    sink.diag_usize("selected_path_len", selected_path_bytes.len());
    sink.diag_usize("user_stack_arg0_ptr", stack.arg0_ptr());
    sink.diag_usize(
        "user_stack_arg0_matches_selected",
        argv0_matches_selected as usize,
    );
    sink.diag_usize("user_mapping_count", space.mapping_count());
    sink.diag_usize("user_segment_mapping_count", space.segment_mapping_count());
    sink.diag_usize("user_heap_mapped", space.heap_mapped() as usize);
    sink.diag_usize("user_heap_base", space.heap_base());
    sink.diag_usize("user_heap_size", space.heap_size());
    if let Some(device) = ctx.virtio_blk_runtime.device() {
        let layout = device.queue().real_layout();
        let desc_overlap = user_space_contains_phys_page(space, ctx, layout.desc_phys());
        let avail_overlap = user_space_contains_phys_page(space, ctx, layout.avail_phys());
        let used_overlap = user_space_contains_phys_page(space, ctx, layout.used_phys());
        sink.diag_hex_pair(
            "virtq_desc_virt_phys",
            layout.desc_virt(),
            layout.desc_phys(),
        );
        sink.diag_hex_pair(
            "virtq_avail_virt_phys",
            layout.avail_virt(),
            layout.avail_phys(),
        );
        sink.diag_hex_pair(
            "virtq_used_virt_phys",
            layout.used_virt(),
            layout.used_phys(),
        );
        sink.diag_usize("virtq_desc_user_overlap", desc_overlap as usize);
        sink.diag_usize("virtq_avail_user_overlap", avail_overlap as usize);
        sink.diag_usize("virtq_used_user_overlap", used_overlap as usize);
        sink.diag_usize(
            "virtq_raw_used_idx",
            device.queue().raw_used_idx().unwrap_or(u16::MAX) as usize,
        );
        valid &= !desc_overlap && !avail_overlap && !used_overlap;
    }

    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "user address space ready facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_mode_entry(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.user_init_process.enter_user_mode";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let trap_stack_base = user_kernel_trap_stack_base();
    let trap_stack_top = user_kernel_trap_stack_top();
    let trap_guard_base = user_kernel_trap_stack_guard_base();
    let overflow_stack_base = user_kernel_trap_overflow_stack_base();
    let overflow_stack_top = user_kernel_trap_overflow_stack_top();
    let trap_stack_valid = USER_KERNEL_TRAP_THREAD_INFO_IN_TASK
        && USER_KERNEL_TRAP_VMAP_STACK
        && USER_KERNEL_TRAP_IRQ_STACKS
        && USER_KERNEL_TRAP_STACK_ORDER == 2
        && USER_KERNEL_TRAP_STACK_SIZE == 16 * 1024
        && USER_KERNEL_TRAP_STACK_SIZE == USER_PAGE_SIZE << USER_KERNEL_TRAP_STACK_ORDER
        && USER_KERNEL_TRAP_STACK_ALIGN == USER_KERNEL_TRAP_STACK_SIZE * 2
        && USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE == USER_PAGE_SIZE
        && USER_KERNEL_IRQ_STACK_SIZE == USER_KERNEL_TRAP_STACK_SIZE
        && USER_KERNEL_TRAP_GUARD_PAGE_READY
        && USER_KERNEL_TRAP_OVERFLOW_STACK_READY
        && USER_KERNEL_TRAP_ENTRY_SCRATCH_DEFERRED
        && USER_KERNEL_TRAP_IRQ_STACK_SWITCH_DEFERRED
        && user_kernel_trap_stack_ready()
        && user_kernel_trap_stack_vmapped()
        && user_kernel_trap_stack_guard_unmapped()
        && user_kernel_trap_stack_guard_size() == USER_KERNEL_TRAP_GUARD_SIZE
        && trap_guard_base + USER_KERNEL_TRAP_GUARD_SIZE == trap_stack_base
        && user_kernel_trap_stack_base_aligned()
        && trap_stack_top == trap_stack_base + USER_KERNEL_TRAP_STACK_SIZE
        && user_kernel_trap_stack_backing_phys() != 0
        && user_kernel_trap_stack_backing_order() == USER_KERNEL_TRAP_STACK_ORDER
        && user_kernel_trap_overflow_stack_ready()
        && overflow_stack_top == overflow_stack_base + USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE;
    let valid = process.state() == State::Online
        && process.reuses_kernel_init_task()
        && process.pid1_preserved()
        && process.exec_identity_handoff()
        && process.address_space_bound()
        && process.fs_struct_inherited()
        && process.files_struct_inherited()
        && process.trap_frame_bound()
        && process.syscall_context_bound()
        && process.trap_return_bound()
        && process.trap_return_context_used()
        && process.trap_return_sfence_vma_after_satp()
        && process.trap_return_sret_handoff()
        && process.user_entry_ready()
        && process.runtime_entered()
        && ctx.user_address_space.state() == State::Online
        && ctx.user_address_space.runtime_ready()
        && ctx.user_trap_frame.state() == State::Ready
        && ctx.user_trap_frame.user_fpu_initial()
        && ctx.user_trap_frame.fpu_context_switch_deferred()
        && ctx.user_trap_frame.sret_ready()
        && ctx.files_struct.state() == State::Ready
        && ctx.files_struct.stdio_bound()
        && ctx.files_struct.fd_bound(FdRef::Stdout)
        && ctx.files_struct.fd_bound(FdRef::Stderr)
        && trap_stack_valid;

    sink.diag_usize(
        "user_init_runtime_entered",
        process.runtime_entered() as usize,
    );
    sink.diag_usize(
        "user_trap_frame_fpu_initial",
        ctx.user_trap_frame.user_fpu_initial() as usize,
    );
    sink.diag_usize(
        "user_trap_frame_fpu_context_switch_deferred",
        ctx.user_trap_frame.fpu_context_switch_deferred() as usize,
    );
    sink.diag_usize("user_entry_ready", process.user_entry_ready() as usize);
    sink.diag_usize(
        "trap_return_context_used",
        process.trap_return_context_used() as usize,
    );
    sink.diag_usize(
        "trap_return_sfence_vma_after_satp",
        process.trap_return_sfence_vma_after_satp() as usize,
    );
    sink.diag_usize(
        "trap_return_sret_handoff",
        process.trap_return_sret_handoff() as usize,
    );
    sink.diag_usize(
        "user_address_space_runtime_ready",
        ctx.user_address_space.runtime_ready() as usize,
    );
    sink.diag_hex_pair(
        "kernel_trap_stack_base_top",
        trap_stack_base,
        trap_stack_top,
    );
    sink.diag_usize(
        "kernel_trap_stack_ready",
        user_kernel_trap_stack_ready() as usize,
    );
    sink.diag_usize(
        "kernel_trap_stack_vmapped",
        user_kernel_trap_stack_vmapped() as usize,
    );
    sink.diag_usize("kernel_trap_stack_order", USER_KERNEL_TRAP_STACK_ORDER);
    sink.diag_usize("kernel_trap_stack_size", USER_KERNEL_TRAP_STACK_SIZE);
    sink.diag_usize("kernel_trap_stack_align", USER_KERNEL_TRAP_STACK_ALIGN);
    sink.diag_usize(
        "kernel_trap_stack_base_aligned",
        user_kernel_trap_stack_base_aligned() as usize,
    );
    sink.diag_usize(
        "kernel_trap_thread_info_in_task",
        USER_KERNEL_TRAP_THREAD_INFO_IN_TASK as usize,
    );
    sink.diag_usize(
        "kernel_trap_vmap_stack_config",
        USER_KERNEL_TRAP_VMAP_STACK as usize,
    );
    sink.diag_usize(
        "kernel_trap_irq_stacks_config",
        USER_KERNEL_TRAP_IRQ_STACKS as usize,
    );
    sink.diag_hex_pair(
        "kernel_trap_guard_base_size",
        trap_guard_base,
        user_kernel_trap_stack_guard_size(),
    );
    sink.diag_usize(
        "kernel_trap_guard_unmapped",
        user_kernel_trap_stack_guard_unmapped() as usize,
    );
    sink.diag_hex_pair(
        "kernel_trap_stack_backing_phys_order",
        user_kernel_trap_stack_backing_phys(),
        user_kernel_trap_stack_backing_order(),
    );
    sink.diag_hex_pair(
        "kernel_trap_overflow_stack_base_top",
        overflow_stack_base,
        overflow_stack_top,
    );
    sink.diag_usize(
        "kernel_trap_overflow_stack_size",
        USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE,
    );
    sink.diag_usize("kernel_irq_stack_size", USER_KERNEL_IRQ_STACK_SIZE);
    sink.diag_usize(
        "kernel_trap_guard_page_ready",
        USER_KERNEL_TRAP_GUARD_PAGE_READY as usize,
    );
    sink.diag_usize(
        "kernel_trap_overflow_stack_ready",
        USER_KERNEL_TRAP_OVERFLOW_STACK_READY as usize,
    );
    sink.diag_usize(
        "kernel_trap_entry_scratch_deferred",
        USER_KERNEL_TRAP_ENTRY_SCRATCH_DEFERRED as usize,
    );
    sink.diag_usize(
        "kernel_trap_irq_stack_switch_deferred",
        USER_KERNEL_TRAP_IRQ_STACK_SWITCH_DEFERRED as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "user mode entry facts invalid");
    }
}

#[cfg(app_user_boot)]
fn user_space_contains_phys_page(
    space: &crate::objects::user_boot::UserAddressSpace,
    ctx: &Context,
    phys: usize,
) -> bool {
    if phys == 0 {
        return false;
    }
    let target = phys & !(crate::objects::user_boot::USER_PAGE_SIZE - 1);
    let mut mapping_index = 0usize;
    while mapping_index < space.mapping_count() {
        if let Some(mapping) = space.mapping(mapping_index) {
            let mut page_index = 0usize;
            while page_index < mapping.backing_page_count() {
                if let Some(page) = mapping.backing_page(page_index) {
                    if ctx
                        .page_metadata_map
                        .page_to_phys(page)
                        .is_some_and(|page_phys| page_phys.value() == target)
                    {
                        return true;
                    }
                }
                page_index += 1;
            }
        }
        mapping_index += 1;
    }
    false
}

#[cfg(app_user_boot)]
fn stack_contains_at(
    stack: &UserStack,
    ctx: &Context,
    user_addr: usize,
    expected: &[u8],
    expect_nul: bool,
) -> bool {
    let total_len = expected.len() + usize::from(expect_nul);
    if user_addr < stack.base()
        || user_addr
            .checked_add(total_len)
            .filter(|end| *end <= stack.top())
            .is_none()
    {
        return false;
    }

    let mut index = 0usize;
    while index < total_len {
        let expected_byte = if index < expected.len() {
            expected[index]
        } else {
            0
        };
        let stack_offset = user_addr - stack.base() + index;
        let page_index = stack_offset / USER_PAGE_SIZE;
        let page_offset = stack_offset % USER_PAGE_SIZE;
        let Some(page) = stack.backing_page(page_index) else {
            return false;
        };
        let Some(linear) = ctx.page_metadata_map.page_address(page) else {
            return false;
        };
        let byte = unsafe { *((linear + page_offset) as *const u8) };
        if byte != expected_byte {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(app_user_boot)]
fn run_syscall_table_set_tid_address(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.set_tid_address";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let table = &ctx.syscall_table;
    let valid = process.state() == State::Online
        && process.runtime_entered()
        && process.syscall_context_bound()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.set_tid_address_supported()
        && table.set_tid_address_observed()
        && process.clear_child_tid_bound()
        && process.clear_child_tid() != 0
        && ctx.exception_stream.syscall_state() == State::Online;

    sink.diag_usize(
        "syscall_table_set_tid_address_observed",
        table.set_tid_address_observed() as usize,
    );
    sink.diag_usize("user_init_clear_child_tid", process.clear_child_tid());
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(
            total,
            "",
            name,
            "syscall table set_tid_address facts invalid",
        );
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_openat(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.openat";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let files = &ctx.files_struct;
    let valid = ctx.user_init_process.state() == State::Online
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.openat_supported()
        && table.path_usercopy_ready()
        && table.openat_routes_to_files_struct()
        && table.openat_observed()
        && files.open_path_routes_to_vfs()
        && files.regular_fd_installed()
        && files.fd_bound(FdRef::Regular0);

    sink.diag_usize(
        "syscall_table_openat_observed",
        table.openat_observed() as usize,
    );
    sink.diag_usize("regular0_len", files.regular0_len());
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table openat facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_read(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.read";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let valid = table.state() == State::Ready
        && table.read_supported()
        && table.read_usercopy_ready()
        && table.read_routes_to_files_struct()
        && table.openat_observed()
        && table.read_observed()
        && ctx.exception_stream.syscall_state() == State::Online;

    sink.diag_usize(
        "syscall_table_read_observed",
        table.read_observed() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table read facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_files_struct_readonly_path(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.files_struct.readonly_path";
    sink.start_case(total, "", name, checkpoint);

    let files = &ctx.files_struct;
    let valid = files.state() == State::Ready
        && files.regular_file_slot_ready()
        && files.fd_bound(FdRef::Regular0)
        && files.fd_table().fd_installed()
        && files.read_fd_routes_to_table()
        && files.regular_file_read_observed()
        && files.regular0().readable()
        && files.regular0().read_dispatches_backend()
        && files.regular0().read_observed()
        && files.regular0_backend().kind() == FileBackendKind::RegularFile
        && files.regular0_backend().regular_file_bound()
        && files.regular0_backend().regular_file_read_supported()
        && files.regular0_backend().regular_file_read_returns_data()
        && files.regular0_backend().last_read_len() != 0;

    sink.diag_usize(
        "regular_file_read_observed",
        files.regular_file_read_observed() as usize,
    );
    sink.diag_usize(
        "regular0_last_read_len",
        files.regular0_backend().last_read_len(),
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "files struct readonly path facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_write(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.write";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let table = &ctx.syscall_table;
    let valid = process.state() == State::Online
        && process.runtime_entered()
        && process.syscall_context_bound()
        && process.syscall_dispatch_bound()
        && process.syscall_arguments_extracted()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.write_supported()
        && table.write_usercopy_ready()
        && table.write_routes_to_console()
        && table.write_observed()
        && ctx.exception_stream.syscall_state() == State::Online;

    sink.diag_usize(
        "syscall_table_write_observed",
        table.write_observed() as usize,
    );
    sink.diag_usize(
        "syscall_table_write_supported",
        table.write_supported() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table write facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_files_struct_write_path(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.files_struct.write_path";
    sink.start_case(total, "", name, checkpoint);

    let files = &ctx.files_struct;
    let valid = files.state() == State::Ready
        && files.fd_lookup_routes_to_table()
        && files.fd_table().lookup_returns()
        && files.stdout().write_dispatches_backend()
        && files.stdout().write_observed()
        && files.stdout_backend().kind() == FileBackendKind::CharDevice
        && files.stdout_backend().write_to_console()
        && files.stdout().last_write_len() != 0
        && files.stdout_backend().last_write_len() == files.stdout().last_write_len();

    sink.diag_usize(
        "files_struct_fd_lookup_routes_to_table",
        files.fd_lookup_routes_to_table() as usize,
    );
    sink.diag_usize("stdout_last_write_len", files.stdout().last_write_len());
    sink.diag_usize(
        "stdout_backend_last_write_len",
        files.stdout_backend().last_write_len(),
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "files struct write path facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_close(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.close";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let files = &ctx.files_struct;
    let valid = table.state() == State::Ready
        && table.close_supported()
        && table.close_routes_to_files_struct()
        && table.close_observed()
        && files.close_fd_routes_to_table()
        && files.regular_file_closed()
        && files.fd_table().fd_closed();

    sink.diag_usize(
        "syscall_table_close_observed",
        table.close_observed() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table close facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_newfstatat(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.newfstatat";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let files = &ctx.files_struct;
    let valid = table.state() == State::Ready
        && table.newfstatat_supported()
        && table.path_usercopy_ready()
        && table.stat_usercopy_ready()
        && table.newfstatat_routes_to_files_struct()
        && table.newfstatat_observed()
        && files.stat_path_routes_to_vfs()
        && files.regular_file_stat_observed()
        && files.regular0_backend().regular_file_stat_supported()
        && files
            .regular0_backend()
            .regular_file_stat_returns_metadata()
        && files.regular0_backend().last_stat_size() != 0;

    sink.diag_usize(
        "syscall_table_newfstatat_observed",
        table.newfstatat_observed() as usize,
    );
    sink.diag_usize(
        "regular0_last_stat_size",
        files.regular0_backend().last_stat_size(),
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table newfstatat facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_exit(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.exit";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let table = &ctx.syscall_table;
    let valid = process.state() == State::Online
        && process.runtime_entered()
        && process.syscall_context_bound()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.exit_supported()
        && table.exit_group_supported()
        && table.exit_records_status()
        && table.write_observed()
        && table.exit_observed()
        && ctx.exception_stream.syscall_state() == State::Online;

    sink.diag_usize(
        "syscall_table_write_observed",
        table.write_observed() as usize,
    );
    sink.diag_usize(
        "syscall_table_exit_observed",
        table.exit_observed() as usize,
    );
    sink.diag_usize(
        "syscall_table_exit_supported",
        table.exit_supported() as usize,
    );
    sink.diag_usize(
        "syscall_table_exit_group_supported",
        table.exit_group_supported() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table exit facts invalid");
    }
}

fn run_valid_fixture(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.valid_fixture";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, image.len(), image.len(), 5, 0x1000);
    image.write_message(0x80);

    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() || elf.setup(image.as_slice()).is_err() {
        sink.fail(total, "", name, "valid fixture ELF did not parse");
        return;
    }
    let Some(segment) = elf.load_segment(0) else {
        sink.fail(total, "", name, "load segment missing");
        return;
    };

    let valid = elf.input_bound()
        && elf.input_from_vfs()
        && elf.magic_valid()
        && elf.class_elf64()
        && elf.little_endian()
        && elf.machine_riscv()
        && elf.type_supported()
        && elf.static_executable()
        && elf.no_separate_loader()
        && elf.program_headers_parsed()
        && elf.pt_load_segments_bound()
        && elf.segment_permissions_bound()
        && elf.load_plan_bound()
        && elf.entry_in_executable_segment()
        && elf.init_content_observed()
        && elf.bss_zero_plan_bound()
        && elf.entry_bound()
        && elf.load_merged_into_setup()
        && elf.entry() == 0x10000
        && elf.program_header_count() == 1
        && elf.load_segment_count() == 1
        && segment.offset() == 0
        && segment.vaddr() == 0x10000
        && segment.filesz() == image.len()
        && segment.memsz() == image.len()
        && segment.readable()
        && segment.executable()
        && !segment.writable()
        && segment.align() == 0x1000;

    if !valid {
        sink.fail(total, "", name, "valid fixture ELF facts invalid");
        return;
    }

    sink.diag_usize("elf_entry", elf.entry());
    sink.diag_usize("elf_load_segments", elf.load_segment_count());
    sink.pass(total, "", name);
}

fn run_bad_magic(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.bad_magic";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.as_mut_slice()[0] = 0;
    let mut elf = ElfObject::new();
    match elf.preset_from_vfs(image.as_slice()) {
        Err(ElfError::BadMagic) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "bad magic accepted"),
    }
}

fn run_wrong_machine(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.wrong_machine";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_u16(18, 62);
    let mut elf = ElfObject::new();
    match elf.preset_from_vfs(image.as_slice()) {
        Err(ElfError::UnsupportedMachine) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "wrong machine accepted"),
    }
}

fn run_invalid_segment(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.invalid_segment";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, image.len(), image.len() - 1, 5, 0x1000);
    image.write_message(0x80);
    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() {
        sink.fail(
            total,
            "",
            name,
            "fixture preset failed before segment check",
        );
        return;
    }

    match elf.setup(image.as_slice()) {
        Err(ElfError::InvalidProgramHeader) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "invalid segment accepted"),
    }
}

fn run_bss_plan(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.bss_plan";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, 256, 512, 5, 0x1000);
    image.write_message(0x80);

    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() || elf.setup(image.as_slice()).is_err() {
        sink.fail(total, "", name, "fixture ELF did not parse");
        return;
    }
    let Some(segment) = elf.load_segment(0) else {
        sink.fail(total, "", name, "load segment missing");
        return;
    };

    if elf.bss_zero_plan_bound()
        && segment.filesz() == 256
        && segment.memsz() == 512
        && segment.memsz() - segment.filesz() == 256
    {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "bss plan facts invalid");
    }
}

struct FixtureElf {
    bytes: [u8; 256],
}

impl FixtureElf {
    const fn new() -> Self {
        Self { bytes: [0; 256] }
    }

    const fn len(&self) -> usize {
        self.bytes.len()
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    fn write_supported_header(&mut self) {
        self.bytes[0..4].copy_from_slice(b"\x7fELF");
        self.bytes[4] = 2;
        self.bytes[5] = 1;
        self.bytes[6] = 1;
        self.write_u16(16, 2);
        self.write_u16(18, 243);
        self.write_u32(20, 1);
        self.write_u64(24, 0x10000);
        self.write_u64(32, 64);
        self.write_u16(52, 64);
        self.write_u16(54, 56);
        self.write_u16(56, 1);
    }

    fn write_load_segment(
        &mut self,
        index: usize,
        offset: usize,
        vaddr: usize,
        filesz: usize,
        memsz: usize,
        flags: u32,
        align: usize,
    ) {
        let phdr = 64 + index * 56;
        self.write_u32(phdr, 1);
        self.write_u32(phdr + 4, flags);
        self.write_u64(phdr + 8, offset as u64);
        self.write_u64(phdr + 16, vaddr as u64);
        self.write_u64(phdr + 24, vaddr as u64);
        self.write_u64(phdr + 32, filesz as u64);
        self.write_u64(phdr + 40, memsz as u64);
        self.write_u64(phdr + 48, align as u64);
    }

    fn write_message(&mut self, offset: usize) {
        self.bytes[offset..offset + USER_INIT_EXPECTED_MESSAGE.len()]
            .copy_from_slice(USER_INIT_EXPECTED_MESSAGE);
    }

    fn write_u16(&mut self, offset: usize, value: u16) {
        self.bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, offset: usize, value: u32) {
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, offset: usize, value: u64) {
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
