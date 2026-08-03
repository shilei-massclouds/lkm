use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        user_boot::{ElfError, ElfObject, USER_INIT_EXPECTED_MESSAGE},
    },
};

#[cfg(app_user_boot)]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(app_user_boot)]
use crate::objects::user_boot::{
    USER_COMPLETED_CHILD_RECORD_CAPACITY, USER_KERNEL_IRQ_STACK_SIZE,
    USER_KERNEL_TRAP_CURRENT_TASK_TP_READY, USER_KERNEL_TRAP_EARLY_CHECK_REGISTER_PRESERVING,
    USER_KERNEL_TRAP_EARLY_OVERFLOW_CHECK_READY, USER_KERNEL_TRAP_ENTRY_CONTEXT_SIZE,
    USER_KERNEL_TRAP_FRAME_SIZE, USER_KERNEL_TRAP_GUARD_PAGE_READY, USER_KERNEL_TRAP_GUARD_SIZE,
    USER_KERNEL_TRAP_IRQ_STACK_SWITCH_DEFERRED, USER_KERNEL_TRAP_IRQ_STACKS,
    USER_KERNEL_TRAP_OVERFLOW_FRAME_COMPLETE, USER_KERNEL_TRAP_OVERFLOW_STACK_READY,
    USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE, USER_KERNEL_TRAP_OVERFLOW_TERMINAL_PANIC,
    USER_KERNEL_TRAP_STACK_ALIGN, USER_KERNEL_TRAP_STACK_ORDER, USER_KERNEL_TRAP_STACK_SIZE,
    USER_KERNEL_TRAP_THREAD_INFO_IN_TASK, USER_KERNEL_TRAP_THREAD_SHIFT,
    USER_KERNEL_TRAP_USER_PATH_BIT_TEST_BYPASSED, USER_KERNEL_TRAP_VMAP_STACK, USER_PAGE_SIZE,
    USER_SIGCHLD_MASK, USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE,
    user_kernel_trap_entry_context, user_kernel_trap_overflow_stack_base,
    user_kernel_trap_overflow_stack_ready, user_kernel_trap_overflow_stack_top,
    user_kernel_trap_stack_backing_order, user_kernel_trap_stack_backing_phys,
    user_kernel_trap_stack_base, user_kernel_trap_stack_base_aligned,
    user_kernel_trap_stack_guard_base, user_kernel_trap_stack_guard_size,
    user_kernel_trap_stack_guard_unmapped, user_kernel_trap_stack_ready,
    user_kernel_trap_stack_top, user_kernel_trap_stack_vmapped,
};

#[cfg(app_user_boot)]
use crate::objects::{
    exception_type::{execve_checkpoint_observation, wait4_checkpoint_observation},
    files::{FdRef, FileBackendKind},
    user_stack::UserStack,
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::PayloadHandoffPreparePhaseOnline,
    Checkpoint::KernelInitFlowPayloadHandoffCommitted,
    #[cfg(app_user_boot)]
    Checkpoint::UserBootMainElfReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserBootInterpreterReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserBootAddressSpaceSetupStart,
    #[cfg(app_user_boot)]
    Checkpoint::UserAddressSpaceReady,
    #[cfg(app_user_boot)]
    Checkpoint::UserAppRuntimeEnterUserMode,
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
    Checkpoint::SyscallTableClone,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableCloneVforkVm,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableCloneVforkPidfd,
    #[cfg(app_user_boot)]
    Checkpoint::FilesStructPidfdInstall,
    #[cfg(app_user_boot)]
    Checkpoint::UserCloneVforkChildHandoff,
    #[cfg(app_user_boot)]
    Checkpoint::UserCloneVforkNextChildAccepted,
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
    Checkpoint::SyscallTableRtSigtimedwait,
    #[cfg(app_user_boot)]
    Checkpoint::UserSignalWaitSleep,
    #[cfg(app_user_boot)]
    Checkpoint::UserSignalWaitWakeSigchld,
    #[cfg(app_user_boot)]
    Checkpoint::UserPidfdReady,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableRtSigtimedwaitReturnSignal,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableWait4,
    #[cfg(app_user_boot)]
    Checkpoint::UserChildParentWaitResumed,
    #[cfg(app_user_boot)]
    Checkpoint::UserCloneVforkParentResumed,
    #[cfg(app_user_boot)]
    Checkpoint::UserChildRecordArchived,
    #[cfg(app_user_boot)]
    Checkpoint::UserTaskRecordReleased,
    #[cfg(app_user_boot)]
    Checkpoint::UserChildRecordReaped,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableExit,
    #[cfg(app_user_boot)]
    Checkpoint::UserStackGrowComplete,
    #[cfg(app_user_boot)]
    Checkpoint::UserStackGrowRejected,
];
#[cfg(app_user_boot)]
pub const KUNIT_CASE_COUNT: usize = 38;
#[cfg(not(app_user_boot))]
pub const KUNIT_CASE_COUNT: usize = 7;

#[cfg(app_user_boot)]
static SYSCALL_SET_TID_ADDRESS_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_CLONE_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_CLONE_VFORK_VM_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_CLONE_VFORK_PIDFD_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static FILES_PIDFD_INSTALL_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_CLONE_VFORK_CHILD_HANDOFF_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_CLONE_VFORK_NEXT_CHILD_ACCEPTED_REPORTED: AtomicBool = AtomicBool::new(false);
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
static SYSCALL_RT_SIGTIMEDWAIT_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_SIGNAL_WAIT_SLEEP_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_SIGNAL_WAIT_WAKE_SIGCHLD_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_PIDFD_READY_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_RT_SIGTIMEDWAIT_RETURN_SIGNAL_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static SYSCALL_WAIT4_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_CHILD_PARENT_WAIT_RESUMED_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_CLONE_VFORK_PARENT_RESUMED_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_CHILD_RECORD_ARCHIVED_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_TASK_RECORD_RELEASED_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(app_user_boot)]
static USER_CHILD_RECORD_REAPED_REPORTED: AtomicBool = AtomicBool::new(false);
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
        Checkpoint::PayloadHandoffPreparePhaseOnline => {
            run_selected_payload_handoff(checkpoint, ctx, sink, total);
            run_valid_fixture(checkpoint, sink, total);
            run_bad_magic(checkpoint, sink, total);
            run_wrong_machine(checkpoint, sink, total);
            run_invalid_segment(checkpoint, sink, total);
            run_bss_plan(checkpoint, sink, total);
        }
        Checkpoint::KernelInitFlowPayloadHandoffCommitted => {
            run_payload_handoff_committed(checkpoint, ctx, sink, total);
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
        Checkpoint::UserStackGrowComplete | Checkpoint::UserStackGrowRejected => {
            sink.diag_usize("fault_address", ctx.user_stack.last_fault_address());
            sink.diag_usize("old_vma_base", ctx.user_stack.last_old_base());
            sink.diag_usize("new_vma_base", ctx.user_stack.last_new_base());
            sink.diag_usize(
                "allocated_pages",
                ctx.user_stack.last_allocated_page_count(),
            );
            sink.diag_usize(
                "rejection_reason",
                ctx.user_stack.last_grow_rejection() as usize,
            );
            sink.diag_usize("tlb_flush", ctx.user_stack.last_tlb_flush() as usize);
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserAppRuntimeEnterUserMode => {
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
        Checkpoint::SyscallTableClone => {
            run_once(&SYSCALL_CLONE_REPORTED, || {
                run_syscall_table_clone(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableCloneVforkVm => {
            run_once(&SYSCALL_CLONE_VFORK_VM_REPORTED, || {
                run_syscall_table_clone_vfork_vm(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableCloneVforkPidfd => {
            run_once(&SYSCALL_CLONE_VFORK_PIDFD_REPORTED, || {
                run_syscall_table_clone_vfork_pidfd(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::FilesStructPidfdInstall => {
            run_once(&FILES_PIDFD_INSTALL_REPORTED, || {
                run_files_struct_pidfd_install(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserCloneVforkChildHandoff => {
            run_once(&USER_CLONE_VFORK_CHILD_HANDOFF_REPORTED, || {
                run_user_clone_vfork_child_handoff(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserCloneVforkNextChildAccepted => {
            run_once(&USER_CLONE_VFORK_NEXT_CHILD_ACCEPTED_REPORTED, || {
                run_user_clone_vfork_next_child_accepted(checkpoint, ctx, sink, total)
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
        Checkpoint::SyscallTableRtSigtimedwait => {
            run_once(&SYSCALL_RT_SIGTIMEDWAIT_REPORTED, || {
                run_syscall_table_rt_sigtimedwait(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserSignalWaitSleep => {
            run_once(&USER_SIGNAL_WAIT_SLEEP_REPORTED, || {
                run_user_signal_wait_sleep(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserSignalWaitWakeSigchld => {
            run_once(&USER_SIGNAL_WAIT_WAKE_SIGCHLD_REPORTED, || {
                run_user_signal_wait_wake_sigchld(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserPidfdReady => {
            run_once(&USER_PIDFD_READY_REPORTED, || {
                run_user_pidfd_ready(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableRtSigtimedwaitReturnSignal => {
            run_once(&SYSCALL_RT_SIGTIMEDWAIT_RETURN_SIGNAL_REPORTED, || {
                run_syscall_table_rt_sigtimedwait_return_signal(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableWait4 => {
            run_once(&SYSCALL_WAIT4_REPORTED, || {
                run_syscall_table_wait4(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserChildParentWaitResumed => {
            run_once(&USER_CHILD_PARENT_WAIT_RESUMED_REPORTED, || {
                run_user_child_parent_wait_resumed(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserCloneVforkParentResumed => {
            run_once(&USER_CLONE_VFORK_PARENT_RESUMED_REPORTED, || {
                run_user_clone_vfork_parent_resumed(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserChildRecordArchived => {
            run_once(&USER_CHILD_RECORD_ARCHIVED_REPORTED, || {
                run_user_child_record_archived(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserTaskRecordReleased => {
            run_once(&USER_TASK_RECORD_RELEASED_REPORTED, || {
                run_user_task_record_released(checkpoint, ctx, sink, total)
            });
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserChildRecordReaped => {
            run_once(&USER_CHILD_RECORD_REAPED_REPORTED, || {
                run_user_child_record_reaped(checkpoint, ctx, sink, total)
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

fn run_selected_payload_handoff(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "payload.selected_handoff.online";
    sink.start_case(total, "", name, checkpoint);

    let handoff = &ctx.selected_payload_handoff;
    let valid = crate::phases::payload::prepare::is_online()
        && crate::phases::payload::handoff_prepare::is_online()
        && crate::systems::kernel::state() == State::Ready
        && crate::systems::kernel::enable_in_progress()
        && ctx.kernel_init_task.flow_state() == State::Online
        && ctx.kernel_init_task.flow().owner().is_valid()
        && handoff.state() == State::Online
        && handoff.kind() == ctx.config.selected_payload_kind()
        && handoff.kind_bound()
        && handoff.variant_setup_ready()
        && handoff.variant_prepare_ready()
        && handoff.no_return_entry_bound()
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.kernel_init_task.task_ref()))
        && ctx.kernel_init_task.current_stack_pointer_in_range()
        && selected_variant_state_ready(ctx);

    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "selected payload handoff facts invalid");
    }
}

#[cfg(app_user_boot)]
fn selected_variant_state_ready(ctx: &Context) -> bool {
    ctx.user_boot_payload.state() == State::Ready
        && ctx.kernel_init_user_runtime.state() == State::Ready
        && !ctx.user_boot_payload.enters_user_mode()
        && !ctx.user_boot_payload.no_return_handoff()
}

#[cfg(not(app_user_boot))]
fn selected_variant_state_ready(ctx: &Context) -> bool {
    ctx.user_boot_payload.state() == State::Base
}

fn run_payload_handoff_committed(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "kernel_init_flow.payload_handoff_committed";
    sink.start_case(total, "", name, checkpoint);
    if crate::systems::kernel::is_online()
        && ctx.selected_payload_handoff.committed()
        && committed_variant_state_valid(ctx)
    {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "payload handoff commit order invalid");
    }
}

#[cfg(app_user_boot)]
fn committed_variant_state_valid(ctx: &Context) -> bool {
    ctx.kernel_init_task.flow_state() == State::Online
        && ctx.kernel_init_task.flow().owner().is_valid()
        && ctx.kernel_init_user_runtime.state() == State::Online
        && ctx.kernel_init_user_runtime.active_binding_committed()
        && ctx.kernel_init_task.application_committed()
        && ctx.user_boot_payload.state() == State::Online
}

#[cfg(not(app_user_boot))]
fn committed_variant_state_valid(ctx: &Context) -> bool {
    ctx.kernel_init_task.flow_state() == State::Online
        && ctx.kernel_init_task.flow().owner().is_valid()
        && ctx.kernel_init_task.kernel_init_flow_owned()
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
    let ctx = crate::context::context_ref();
    let (task_ref, flow_ref) = if ctx.user_task_set.active_task_ref().is_valid() {
        (
            ctx.user_task_set.active_task_ref(),
            ctx.user_task_set.flow_ref(),
        )
    } else {
        (
            ctx.kernel_init_task.task_ref(),
            ctx.kernel_init_user_runtime.flow_ref(),
        )
    };
    sink.diag_usize("execve_task_ref_slot", task_ref.slot());
    sink.diag_usize("execve_task_generation", task_ref.generation() as usize);
    sink.diag_usize("execve_flow_ref_slot", flow_ref.slot());
    sink.diag_usize("execve_flow_generation", flow_ref.generation() as usize);
    sink.diag_usize("execve_stage", obs.stage);
    sink.diag_usize("execve_filename_len", obs.filename_len);
    sink.diag_usize("execve_argv0_len", obs.argv0_len);
    sink.diag_usize("execve_argc", obs.argv_argc);
    sink.diag_usize("execve_argv_total_bytes", obs.argv_total_bytes);
    sink.diag_usize("execve_argv_capacity_exceeded", obs.argv_capacity_exceeded);
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
    sink.diag_usize("execve_stack_top_max", obs.stack_top_max);
    sink.diag_usize("execve_stack_top", obs.stack_top);
    sink.diag_usize("execve_stack_aslr_offset", obs.stack_aslr_offset);
    sink.diag_usize("execve_stack_execfn_ptr", obs.stack_execfn_ptr);
    sink.diag_usize("execve_stack_auxv_complete", obs.stack_auxv_complete);
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
    let close_report = ctx.files_struct.close_on_exec_report();
    sink.diag_usize("execve_close_on_exec_scanned", close_report.scanned);
    sink.diag_usize("execve_close_on_exec_closed", close_report.closed);
    sink.diag_usize(
        "execve_close_on_exec_first_closed_fd",
        close_report.first_closed_fd,
    );
    sink.diag_usize(
        "execve_close_on_exec_remaining_open",
        close_report.remaining_open,
    );
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

    let staging = ctx.exec_transaction.active()
        && ctx.exec_transaction.staging_address_space.state() == State::Ready
        && ctx.exec_transaction.staging_stack.state() == State::Ready;
    let space = if staging {
        &ctx.exec_transaction.staging_address_space
    } else {
        &ctx.user_address_space
    };
    let stack = if staging {
        &ctx.exec_transaction.staging_stack
    } else {
        &ctx.user_stack
    };
    let layout_stack = if ctx.exec_transaction.staging_stack.auxv_complete() {
        &ctx.exec_transaction.staging_stack
    } else {
        stack
    };
    let selected_path = ctx.user_boot_payload.selected_path();
    let selected_path_bytes = ctx
        .exec_transaction
        .active_filename()
        .unwrap_or_else(|| ctx.user_boot_payload.selected_path_bytes());
    let argv0_matches_selected =
        stack_contains_at(stack, ctx, stack.arg0_ptr(), selected_path_bytes, true);
    let execfn_matches_selected = stack_contains_at(
        layout_stack,
        ctx,
        layout_stack.execfn_ptr(),
        selected_path_bytes,
        true,
    );
    let mut valid = space.state() == State::Ready
        && space.page_table_view_ready()
        && space.elf_segments_mapped()
        && space.stack_mapped()
        && space.heap_mapped()
        && (staging
            || (ctx.user_boot_payload.selected_path_bound()
                && ctx.user_boot_payload.selected_argv0_path_bound()))
        && argv0_matches_selected
        && layout_stack.auxv_complete()
        && execfn_matches_selected;

    sink.diag_usize("selected_path_index", selected_path.index());
    sink.diag_usize("selected_path_len", selected_path_bytes.len());
    sink.diag_usize("user_stack_arg0_ptr", stack.arg0_ptr());
    sink.diag_usize(
        "user_stack_arg0_matches_selected",
        argv0_matches_selected as usize,
    );
    sink.diag_usize("user_stack_top_max", layout_stack.config().stack_top_max());
    sink.diag_usize("user_stack_selected_top", layout_stack.top());
    sink.diag_usize("user_stack_aslr_offset", layout_stack.aslr_offset());
    sink.diag_usize("user_stack_execfn_ptr", layout_stack.execfn_ptr());
    sink.diag_usize(
        "user_stack_execfn_matches_selected",
        execfn_matches_selected as usize,
    );
    sink.diag_usize(
        "user_stack_auxv_complete",
        layout_stack.auxv_complete() as usize,
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
    let name = "user_boot.kernel_init_user_state.enter_user_mode";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.kernel_init_user_state;
    let trap_stack_base = user_kernel_trap_stack_base();
    let trap_stack_top = user_kernel_trap_stack_top();
    let trap_entry_context = user_kernel_trap_entry_context();
    let trap_guard_base = user_kernel_trap_stack_guard_base();
    let overflow_stack_base = user_kernel_trap_overflow_stack_base();
    let overflow_stack_top = user_kernel_trap_overflow_stack_top();
    let cpu_trap = ctx.boot_cpu_trap();
    let entry_context = cpu_trap.entry_context();
    let trap_stack_valid = USER_KERNEL_TRAP_THREAD_INFO_IN_TASK
        && USER_KERNEL_TRAP_VMAP_STACK
        && USER_KERNEL_TRAP_IRQ_STACKS
        && USER_KERNEL_TRAP_STACK_ORDER == 2
        && USER_KERNEL_TRAP_STACK_SIZE == 16 * 1024
        && USER_KERNEL_TRAP_STACK_SIZE == USER_PAGE_SIZE << USER_KERNEL_TRAP_STACK_ORDER
        && USER_KERNEL_TRAP_STACK_ALIGN == USER_KERNEL_TRAP_STACK_SIZE * 2
        && USER_KERNEL_TRAP_FRAME_SIZE
            == core::mem::size_of::<crate::objects::trap_type::TrapFrame>()
        && USER_KERNEL_TRAP_FRAME_SIZE == 288
        && USER_KERNEL_TRAP_ENTRY_CONTEXT_SIZE == 128
        && USER_KERNEL_TRAP_THREAD_SHIFT == 14
        && USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE == USER_PAGE_SIZE
        && USER_KERNEL_IRQ_STACK_SIZE == USER_KERNEL_TRAP_STACK_SIZE
        && USER_KERNEL_TRAP_GUARD_PAGE_READY
        && USER_KERNEL_TRAP_OVERFLOW_STACK_READY
        && USER_KERNEL_TRAP_EARLY_OVERFLOW_CHECK_READY
        && USER_KERNEL_TRAP_EARLY_CHECK_REGISTER_PRESERVING
        && USER_KERNEL_TRAP_USER_PATH_BIT_TEST_BYPASSED
        && USER_KERNEL_TRAP_CURRENT_TASK_TP_READY
        && USER_KERNEL_TRAP_OVERFLOW_FRAME_COMPLETE
        && USER_KERNEL_TRAP_OVERFLOW_TERMINAL_PANIC
        && USER_KERNEL_TRAP_IRQ_STACK_SWITCH_DEFERRED
        && user_kernel_trap_stack_ready()
        && user_kernel_trap_stack_vmapped()
        && user_kernel_trap_stack_guard_unmapped()
        && user_kernel_trap_stack_guard_size() == USER_KERNEL_TRAP_GUARD_SIZE
        && trap_guard_base + USER_KERNEL_TRAP_GUARD_SIZE == trap_stack_base
        && user_kernel_trap_stack_base_aligned()
        && trap_stack_top == trap_stack_base + USER_KERNEL_TRAP_STACK_SIZE
        && trap_entry_context == cpu_trap.entry_context_address()
        && trap_entry_context.is_multiple_of(16)
        && entry_context.cpu_logical_id() == 0
        && entry_context.task_identity() == crate::arch::riscv64::csr::read_tp()
        && entry_context.kernel_stack_base() == trap_stack_base
        && entry_context.kernel_stack_top() == trap_stack_top
        && entry_context.emergency_stack_base() == overflow_stack_base
        && entry_context.emergency_stack_top() == overflow_stack_top
        && !entry_context.emergency_active()
        && user_kernel_trap_stack_backing_phys() != 0
        && user_kernel_trap_stack_backing_order() == USER_KERNEL_TRAP_STACK_ORDER
        && user_kernel_trap_overflow_stack_ready()
        && overflow_stack_top == overflow_stack_base + USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE;
    let valid = process.active_user_flow_online()
        && process.preserves_kernel_init_task()
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
        && ctx.kernel_init_user_runtime.state() == State::Online
        && ctx.kernel_init_user_runtime.task_ref_owner() == ctx.kernel_init_task.task_ref()
        && ctx.kernel_init_user_runtime.flow_generation() != 0
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
        "kernel_init_task_ref_slot",
        ctx.kernel_init_task.task_ref().slot(),
    );
    sink.diag_usize(
        "kernel_init_task_generation",
        ctx.kernel_init_task.task_ref().generation() as usize,
    );
    sink.diag_usize(
        "user_runtime_flow_ref_slot",
        ctx.kernel_init_user_runtime.flow_ref().slot(),
    );
    sink.diag_usize(
        "user_runtime_flow_generation",
        ctx.kernel_init_user_runtime.flow_generation() as usize,
    );
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
    sink.diag_usize("kernel_trap_frame_size", USER_KERNEL_TRAP_FRAME_SIZE);
    sink.diag_usize("kernel_trap_thread_shift", USER_KERNEL_TRAP_THREAD_SHIFT);
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
        "kernel_trap_early_overflow_check_ready",
        USER_KERNEL_TRAP_EARLY_OVERFLOW_CHECK_READY as usize,
    );
    sink.diag_usize(
        "kernel_trap_early_check_register_preserving",
        USER_KERNEL_TRAP_EARLY_CHECK_REGISTER_PRESERVING as usize,
    );
    sink.diag_usize(
        "kernel_trap_user_path_bit_test_bypassed",
        USER_KERNEL_TRAP_USER_PATH_BIT_TEST_BYPASSED as usize,
    );
    sink.diag_usize(
        "kernel_trap_overflow_frame_complete",
        USER_KERNEL_TRAP_OVERFLOW_FRAME_COMPLETE as usize,
    );
    sink.diag_usize(
        "kernel_trap_overflow_terminal_panic",
        USER_KERNEL_TRAP_OVERFLOW_TERMINAL_PANIC as usize,
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
        let current = user_addr + index;
        let page_vaddr = current & !(USER_PAGE_SIZE - 1);
        let page_offset = current - page_vaddr;
        let Some(page) = stack.page_for_vaddr(page_vaddr) else {
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

    let process = &ctx.kernel_init_user_state;
    let table = &ctx.syscall_table;
    let valid = process.active_user_flow_online()
        && process.runtime_entered()
        && process.syscall_context_bound()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.set_tid_address_supported()
        && table.set_tid_address_observed()
        && process.clear_child_tid_bound()
        && process.clear_child_tid() != 0
        && ctx.boot_cpu_exception().syscall_state() == State::Online;

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
fn run_syscall_table_clone(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.clone_plain_fork";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let child = &ctx.user_task_set;
    let internal_child_task = crate::objects::user_boot::USER_CHILD_PID;
    let parent_pid = crate::objects::rest_init::KERNEL_INIT_PID;
    let child_a0 = child.child_trap_frame_reg(10).unwrap_or(usize::MAX);
    let child_tp = child.child_trap_frame_reg(4).unwrap_or(0);
    let child_sepc = child.child_trap_frame_sepc().unwrap_or(0);
    let runqueue_contains_child = ctx.scheduler().contains_task_ref(child.active_task_ref());

    let valid = table.state() == State::Ready
        && table.clone_supported()
        && table.clone_routes_to_task_creation_core()
        && table.clone_routes_to_user_clone_deferred_boundaries()
        && table.clone_plain_fork_first_slice()
        && table.clone_observed()
        && ctx.task_creation_core.state() == State::Ready
        && ctx.task_creation_core.user_child_created()
        && child.active_task_state() == State::Online
        && child.pid() >= internal_child_task
        && child.parent_pid() == parent_pid
        && child.tgid() == child.pid()
        && child.exit_signal() == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && child.task_struct_allocated()
        && child.pid_allocated()
        && child.thread_context_ready()
        && child.sched_entity_ready()
        && child.task_state_new()
        && child.files_struct_copied()
        && child.fs_struct_copied()
        && child.credentials_copied()
        && child.signal_state_copied()
        && child.user_address_space_snapshot()
        && child.user_stack_snapshot_copied()
        && child.trap_frame_copied()
        && child.trap_frame_child_return_zero()
        && child_a0 == 0
        && child.tls_inherited()
        && child.enqueued()
        && ctx.scheduler().selected_runqueue_task_id() == internal_child_task
        && runqueue_contains_child
        && ctx.kernel_init_user_state.child_process_group_visible();

    sink.diag_usize("clone_observed", table.clone_observed() as usize);
    sink.diag_usize("clone_child_pid", child.pid());
    sink.diag_usize("clone_parent_pid", child.parent_pid());
    sink.diag_usize("clone_tgid", child.tgid());
    sink.diag_usize("clone_exit_signal", child.exit_signal());
    sink.diag_usize("clone_child_a0", child_a0);
    sink.diag_hex_pair("clone_child_sepc_tp", child_sepc, child_tp);
    sink.diag_usize(
        "clone_task_creation_user_child_created",
        ctx.task_creation_core.user_child_created() as usize,
    );
    sink.diag_usize(
        "clone_scheduler_selected_task",
        ctx.scheduler().selected_runqueue_task_id(),
    );
    sink.diag_usize(
        "clone_runqueue_contains_child",
        runqueue_contains_child as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table clone facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_files_struct_pidfd_install(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.files_struct.pidfd_install";
    sink.start_case(total, "", name, checkpoint);

    let files = &ctx.files_struct;
    let child = &ctx.user_task_set;
    let valid = files.state() == State::Ready
        && files.pidfd_installed()
        && files.pidfd_fd() != usize::MAX
        && files.pidfd_child_pid() == child.pid()
        && !files.pidfd_ready();

    sink.diag_usize("pidfd_installed", files.pidfd_installed() as usize);
    sink.diag_usize("pidfd_fd", files.pidfd_fd());
    sink.diag_usize("pidfd_child_pid", files.pidfd_child_pid());
    sink.diag_usize("pidfd_ready", files.pidfd_ready() as usize);
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "pidfd install facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_clone_vfork_pidfd(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.clone_vfork_pidfd";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let child = &ctx.user_task_set;
    let files = &ctx.files_struct;
    let internal_child_task = crate::objects::user_boot::USER_CHILD_PID;
    let child_a0 = child.child_trap_frame_reg(10).unwrap_or(usize::MAX);
    let child_sp = child.child_trap_frame_reg(2).unwrap_or(0);
    let child_sepc = child.child_trap_frame_sepc().unwrap_or(0);
    let runqueue_contains_child = ctx.scheduler().contains_task_ref(child.active_task_ref());

    let valid = table.state() == State::Ready
        && table.clone_supported()
        && table.clone_routes_to_task_creation_core()
        && table.clone_routes_to_user_clone_deferred_boundaries()
        && table.clone_vfork_pidfd_first_slice()
        && table.clone_legacy_pidfd_parent_tidptr_bound()
        && table.clone_vfork_parent_frame_saved()
        && table.clone_pidfd_copyout_first_slice()
        && table.clone_full_vfork_scheduler_deferred()
        && table.clone_full_pidfd_file_ops_deferred()
        && table.clone_observed()
        && ctx.task_creation_core.user_child_created()
        && child.active_task_state() == State::Online
        && child.vfork_pidfd_clone()
        && child.vfork_parent_frame_saved()
        && child.pidfd_copyout_observed()
        && child.pidfd_fd() == files.pidfd_fd()
        && child.pid() >= internal_child_task
        && child.exit_signal() == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && child.current_child_continuation()
        && child.trap_frame_child_return_zero()
        && child_a0 == 0
        && child_sp == child.vfork_child_sp()
        && child_sp != 0
        && child.enqueued()
        && files.pidfd_installed()
        && files.pidfd_child_pid() == child.pid()
        && runqueue_contains_child;

    sink.diag_usize(
        "clone_vfork_pidfd_observed",
        table.clone_observed() as usize,
    );
    sink.diag_usize("clone_vfork_child_pid", child.pid());
    sink.diag_usize("clone_vfork_exit_signal", child.exit_signal());
    sink.diag_usize("clone_vfork_pidfd_fd", child.pidfd_fd());
    sink.diag_usize(
        "clone_vfork_pidfd_copyout",
        child.pidfd_copyout_observed() as usize,
    );
    sink.diag_hex_pair("clone_vfork_child_sepc_sp", child_sepc, child_sp);
    sink.diag_usize("clone_vfork_child_a0", child_a0);
    sink.diag_usize(
        "clone_vfork_parent_frame_saved",
        child.vfork_parent_frame_saved() as usize,
    );
    sink.diag_usize(
        "clone_vfork_runqueue_contains_child",
        runqueue_contains_child as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "vfork pidfd clone facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_clone_vfork_vm(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.clone_vfork_vm";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let child = &ctx.user_task_set;
    let internal_child_task = crate::objects::user_boot::USER_CHILD_PID;
    let child_a0 = child.child_trap_frame_reg(10).unwrap_or(usize::MAX);
    let child_sp = child.child_trap_frame_reg(2).unwrap_or(0);
    let child_sepc = child.child_trap_frame_sepc().unwrap_or(0);
    let runqueue_contains_child = ctx.scheduler().contains_task_ref(child.active_task_ref());

    let valid = table.state() == State::Ready
        && table.clone_supported()
        && table.clone_routes_to_task_creation_core()
        && table.clone_routes_to_user_clone_deferred_boundaries()
        && table.clone_vfork_vm_first_slice()
        && table.clone_observed()
        && ctx.task_creation_core.user_child_created()
        && child.active_task_state() == State::Online
        && child.vfork_vm_clone()
        && !child.vfork_pidfd_clone()
        && child.vfork_parent_frame_saved()
        && !child.pidfd_copyout_observed()
        && child.pidfd_fd() == usize::MAX
        && child.pid() >= internal_child_task
        && child.exit_signal() == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && child.current_child_continuation()
        && child.trap_frame_child_return_zero()
        && child_a0 == 0
        && child_sp == child.vfork_child_sp()
        && child_sp != 0
        && child.enqueued()
        && runqueue_contains_child;

    sink.diag_usize("clone_vfork_vm_observed", table.clone_observed() as usize);
    sink.diag_usize("clone_vfork_vm_child_pid", child.pid());
    sink.diag_usize("clone_vfork_vm_exit_signal", child.exit_signal());
    sink.diag_hex_pair("clone_vfork_vm_child_sepc_sp", child_sepc, child_sp);
    sink.diag_usize("clone_vfork_vm_child_a0", child_a0);
    sink.diag_usize(
        "clone_vfork_vm_parent_frame_saved",
        child.vfork_parent_frame_saved() as usize,
    );
    sink.diag_usize(
        "clone_vfork_vm_runqueue_contains_child",
        runqueue_contains_child as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "vfork vm clone facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_clone_vfork_child_handoff(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_clone.vfork_child_handoff";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let child_task_ref = child.active_task_ref();
    let current_task_ref = ctx.current_task_ref().unwrap_or(TaskRef::NONE);
    let child_flow_ref = child.flow_ref();
    let valid = child.vfork_clone()
        && child.vfork_child_handoff()
        && child.current_child_continuation()
        && child.child_continuation_taken()
        && child.vfork_child_sp() != 0
        && !child.vfork_parent_resumed()
        && child.parent_clone_return() == 0
        && child.active_task_state() == State::OnCpu
        && current_task_ref.same_identity(child_task_ref)
        && child_flow_ref.is_valid();

    sink.diag_usize("vfork_child_handoff", child.vfork_child_handoff() as usize);
    sink.diag_usize(
        "vfork_current_child_continuation",
        child.current_child_continuation() as usize,
    );
    sink.diag_usize("vfork_child_sp", child.vfork_child_sp());
    sink.diag_usize(
        "vfork_parent_resumed",
        child.vfork_parent_resumed() as usize,
    );
    sink.diag_usize("vfork_child_task_slot", child_task_ref.slot());
    sink.diag_usize(
        "vfork_child_task_generation",
        child_task_ref.generation() as usize,
    );
    sink.diag_usize("vfork_current_task_ref_id", current_task_ref.slot());
    sink.diag_usize(
        "vfork_current_task_generation",
        current_task_ref.generation() as usize,
    );
    sink.diag_usize("vfork_child_task_state", child.active_task_state() as usize);
    sink.diag_usize("vfork_child_flow_slot", child_flow_ref.slot());
    sink.diag_usize(
        "vfork_child_flow_generation",
        child_flow_ref.generation() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "vfork child handoff facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_clone_vfork_next_child_accepted(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_clone.vfork_next_child_accepted";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let valid = child.vfork_clone()
        && child.vfork_next_child_accepted()
        && child.completed_child_record_total_archived() != 0
        && child.pid() >= crate::objects::user_boot::USER_CHILD_PID + 1
        && child.current_child_continuation()
        && child.vfork_child_handoff();

    sink.diag_usize(
        "vfork_next_child_accepted",
        child.vfork_next_child_accepted() as usize,
    );
    sink.diag_usize("vfork_next_child_pid", child.pid());
    sink.diag_usize(
        "completed_child_record_count",
        child.completed_child_record_count(),
    );
    sink.diag_usize(
        "completed_child_record_total_archived",
        child.completed_child_record_total_archived(),
    );
    sink.diag_usize(
        "completed_child_record_released_count",
        child.completed_child_record_released_count(),
    );
    sink.diag_usize("task_allocation_count", child.task_allocation_count());
    sink.diag_usize("next_child_pid", child.next_child_pid());
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "next vfork child facts invalid");
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
    let opened_regular = files.open_path_routes_to_vfs()
        && files.regular_fd_installed()
        && files.fd_bound(FdRef::Regular0);
    let opened_directory = files.open_path_routes_to_vfs() && files.directory_fd_installed();
    let opened_special = files.tty_alias_fd_installed() || files.null_fd_installed();
    let valid = ctx.kernel_init_user_state.active_user_flow_online()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.openat_supported()
        && table.path_usercopy_ready()
        && table.openat_routes_to_files_struct()
        && table.openat_observed()
        && (opened_regular || opened_directory || opened_special);

    sink.diag_usize(
        "syscall_table_openat_observed",
        table.openat_observed() as usize,
    );
    sink.diag_usize("regular0_len", files.regular0_len());
    sink.diag_usize("null_fd_installed", files.null_fd_installed() as usize);
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
        && ctx.boot_cpu_exception().syscall_state() == State::Online;

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

    let process = &ctx.kernel_init_user_state;
    let table = &ctx.syscall_table;
    let valid = process.active_user_flow_online()
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
        && ctx.boot_cpu_exception().syscall_state() == State::Online;

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
fn run_syscall_table_rt_sigtimedwait(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.rt_sigtimedwait";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let process = &ctx.kernel_init_user_state;
    let valid = table.state() == State::Ready
        && table.rt_sigtimedwait_supported()
        && table.signal_mask_usercopy_ready()
        && table.rt_sigtimedwait_routes_to_kernel_init_user_state()
        && table.rt_sigtimedwait_sigsetsize_bound()
        && table.rt_sigtimedwait_copies_wait_mask()
        && table.rt_sigtimedwait_uinfo_null_no_copyout_first_slice()
        && table.rt_sigtimedwait_uts_null_infinite_wait_first_slice()
        && table.rt_sigtimedwait_empty_pending_wait_boundary()
        && table.rt_sigtimedwait_waitqueue_sleep_first_slice()
        && table.rt_sigtimedwait_sigchld_pending_first_slice()
        && table.rt_sigtimedwait_return_signal_first_slice()
        && table.signal_delivery_deferred()
        && table.rt_sigtimedwait_observed()
        && process.pending_signal_set_empty_first_slice()
        && process.rt_sigtimedwait_observed()
        && process.rt_sigtimedwait_mask() != 0
        && process.rt_sigtimedwait_uinfo_null()
        && process.rt_sigtimedwait_uts_null()
        && process.rt_sigtimedwait_sigchld_mask_match()
        && process.rt_sigtimedwait_infinite_wait();

    sink.diag_usize(
        "rt_sigtimedwait_observed",
        table.rt_sigtimedwait_observed() as usize,
    );
    sink.diag_usize("rt_sigtimedwait_mask", process.rt_sigtimedwait_mask());
    sink.diag_usize(
        "rt_sigtimedwait_uinfo_null",
        process.rt_sigtimedwait_uinfo_null() as usize,
    );
    sink.diag_usize(
        "rt_sigtimedwait_uts_null",
        process.rt_sigtimedwait_uts_null() as usize,
    );
    sink.diag_usize(
        "rt_sigtimedwait_pending_match",
        process.rt_sigtimedwait_pending_match() as usize,
    );
    sink.diag_usize("rt_sigtimedwait_sigchld_mask", USER_SIGCHLD_MASK);
    sink.diag_usize(
        "rt_sigtimedwait_pending_sigchld",
        process.pending_sigchld() as usize,
    );
    sink.diag_usize(
        "rt_sigtimedwait_waiter_enqueued",
        process.rt_sigtimedwait_waiter_enqueued() as usize,
    );
    sink.diag_usize(
        "rt_sigtimedwait_sleep_reason",
        process.rt_sigtimedwait_sleep_reason(),
    );
    sink.diag_usize(
        "rt_sigtimedwait_wake_signal",
        process.rt_sigtimedwait_wake_signal(),
    );
    sink.diag_usize(
        "rt_sigtimedwait_dequeued_signal",
        process.rt_sigtimedwait_dequeued_signal(),
    );
    sink.diag_usize(
        "rt_sigtimedwait_return_signal",
        process.rt_sigtimedwait_return_signal(),
    );
    sink.diag_usize(
        "rt_sigtimedwait_infinite_wait",
        process.rt_sigtimedwait_infinite_wait() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "rt_sigtimedwait wait facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_signal_wait_sleep(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_signal_wait.sleep";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.kernel_init_user_state;
    let valid = process.rt_sigtimedwait_observed()
        && process.rt_sigtimedwait_sleeping()
        && process.rt_sigtimedwait_waiter_enqueued()
        && process.rt_sigtimedwait_saved_frame_bound()
        && process.rt_sigtimedwait_sleep_reason()
            == USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE
        && !process.pending_sigchld()
        && !process.rt_sigtimedwait_pending_match();

    sink.diag_usize("signal_wait_mask", process.rt_sigtimedwait_mask());
    sink.diag_usize("signal_wait_sigchld_mask", USER_SIGCHLD_MASK);
    sink.diag_usize(
        "signal_wait_pending_sigchld",
        process.pending_sigchld() as usize,
    );
    sink.diag_usize(
        "signal_wait_pending_match",
        process.rt_sigtimedwait_pending_match() as usize,
    );
    sink.diag_usize(
        "signal_wait_waiter_enqueued",
        process.rt_sigtimedwait_waiter_enqueued() as usize,
    );
    sink.diag_usize(
        "signal_wait_sleep_reason",
        process.rt_sigtimedwait_sleep_reason(),
    );

    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "signal wait sleep facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_signal_wait_wake_sigchld(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_signal_wait.wake_sigchld";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.kernel_init_user_state;
    let valid = process.rt_sigtimedwait_observed()
        && !process.rt_sigtimedwait_sleeping()
        && !process.rt_sigtimedwait_waiter_enqueued()
        && process.rt_sigtimedwait_waiter_finished()
        && process.rt_sigtimedwait_wake_sigchld_committed()
        && process.rt_sigtimedwait_wake_signal() == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && process.rt_sigtimedwait_dequeued_signal()
            == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && process.rt_sigtimedwait_return_signal() == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && !process.pending_sigchld();

    sink.diag_usize(
        "signal_wait_wake_signal",
        process.rt_sigtimedwait_wake_signal(),
    );
    sink.diag_usize(
        "signal_wait_dequeued_signal",
        process.rt_sigtimedwait_dequeued_signal(),
    );
    sink.diag_usize(
        "signal_wait_return_signal",
        process.rt_sigtimedwait_return_signal(),
    );
    sink.diag_usize(
        "signal_wait_waiter_finished",
        process.rt_sigtimedwait_waiter_finished() as usize,
    );

    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "signal wait wake facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_pidfd_ready(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.user_pidfd.ready";
    sink.start_case(total, "", name, checkpoint);

    let files = &ctx.files_struct;
    let child = &ctx.user_task_set;
    let process = &ctx.kernel_init_user_state;
    let valid = files.pidfd_installed()
        && files.pidfd_ready()
        && files.pidfd_fd() == child.pidfd_fd()
        && files.pidfd_child_pid() == child.pid()
        && files.pidfd_exit_status() == child.child_exit_status()
        && child.child_exit_status_observed()
        && child.vfork_pidfd_clone();

    sink.diag_usize("pidfd_ready", files.pidfd_ready() as usize);
    sink.diag_usize("pidfd_fd", files.pidfd_fd());
    sink.diag_usize("pidfd_child_pid", files.pidfd_child_pid());
    sink.diag_usize("pidfd_child_exit_status", files.pidfd_exit_status());
    sink.diag_usize(
        "pidfd_child_exit_status_observed",
        child.child_exit_status_observed() as usize,
    );
    sink.diag_usize("pending_sigchld", process.pending_sigchld() as usize);
    sink.diag_usize(
        "rt_sigtimedwait_wake_signal",
        process.rt_sigtimedwait_wake_signal(),
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "pidfd ready facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_rt_sigtimedwait_return_signal(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.rt_sigtimedwait_return_signal";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.kernel_init_user_state;
    let valid = ctx.syscall_table.rt_sigtimedwait_observed()
        && process.rt_sigtimedwait_observed()
        && process.rt_sigtimedwait_dequeued_signal()
            == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && process.rt_sigtimedwait_return_signal() == crate::objects::user_boot::USER_CLONE_SIGCHLD
        && !process.pending_sigchld();

    sink.diag_usize(
        "rt_sigtimedwait_dequeued_signal",
        process.rt_sigtimedwait_dequeued_signal(),
    );
    sink.diag_usize(
        "rt_sigtimedwait_return_signal",
        process.rt_sigtimedwait_return_signal(),
    );

    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(
            total,
            "",
            name,
            "rt_sigtimedwait return signal facts invalid",
        );
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_wait4(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.wait4_parent_saved";
    sink.start_case(total, "", name, checkpoint);

    let table = &ctx.syscall_table;
    let child = &ctx.user_task_set;
    let obs = wait4_checkpoint_observation();
    let child_task_ref = child.active_task_ref();
    let current_task_ref = ctx.current_task_ref().unwrap_or(TaskRef::NONE);
    let child_flow_ref = child.flow_ref();
    let child_dispatch_valid = child.active_task_state() == State::OnCpu
        && current_task_ref.same_identity(child_task_ref)
        && child_flow_ref.is_valid();
    let table_ready = table.state() == State::Ready
        && table.wait4_supported()
        && table.wait4_no_child_echild_first_slice()
        && table.wait4_observed();
    let parent_wait_valid = table_ready
        && table.wait4_parent_wait_chldexit_boundary()
        && table.wait4_yields_to_user_child_continuation()
        && table.wait4_child_exit_status_copyout_first_slice()
        && table.wait4_observed_child_reap_first_slice()
        && child.wait4_parent_wait_observed()
        && child.child_continuation_taken()
        && child.parent_wait_frame_saved()
        && child.parent_address_space_snapshot_saved()
        && child.parent_wait_register_checkpoint_bound()
        && child.parent_wait_stack_window_checkpoint_bound()
        && child.parent_wait_stack_snapshot_copied()
        && child.parent_wait_writable_page_snapshot_copied()
        && obs.saved != 0
        && obs.saved_sepc != 0
        && obs.saved_sp != 0
        && obs.saved_a7 == 260
        && obs.saved_status_ptr != 0
        && obs.saved_child_pid == crate::objects::user_boot::USER_CHILD_PID
        && obs.stack_window_saved != 0
        && obs.stack_window_start == child.parent_wait_stack_window_start()
        && obs.stack_window_len == child.parent_wait_stack_window_len()
        && obs.stack_window_len != 0
        && obs.writable_pages_saved != 0
        && obs.writable_pages_count == child.parent_wait_writable_page_count()
        && obs.writable_pages_count != 0
        && obs.writable_pages_truncated == 0
        && child_dispatch_valid;
    let wnohang_valid = table_ready
        && !child.wait4_parent_wait_observed()
        && !child.child_continuation_taken()
        && !child.parent_wait_frame_saved()
        && obs.saved == 0
        && obs.resumed == 0;
    let valid = parent_wait_valid || wnohang_valid;

    sink.diag_usize("wait4_parent_wait_path", parent_wait_valid as usize);
    sink.diag_usize("wait4_wnohang_path", wnohang_valid as usize);
    sink.diag_usize(
        "wait4_parent_wait_observed",
        child.wait4_parent_wait_observed() as usize,
    );
    sink.diag_usize("wait4_child_enqueued", child.enqueued() as usize);
    sink.diag_usize(
        "wait4_child_exit_status_observed",
        child.child_exit_status_observed() as usize,
    );
    sink.diag_hex_pair("wait4_saved_sepc_sp", obs.saved_sepc, obs.saved_sp);
    sink.diag_hex_pair("wait4_saved_s2_s4", obs.saved_s2, obs.saved_s4);
    sink.diag_hex_pair(
        "wait4_saved_satp_status",
        obs.saved_satp,
        obs.saved_status_ptr,
    );
    sink.diag_usize("wait4_saved_a7", obs.saved_a7);
    sink.diag_usize("wait4_saved_child_pid", obs.saved_child_pid);
    sink.diag_hex_pair(
        "wait4_stack_window_start_len",
        obs.stack_window_start,
        obs.stack_window_len,
    );
    sink.diag_usize("wait4_stack_window_saved", obs.stack_window_saved);
    sink.diag_usize(
        "wait4_parent_stack_snapshot_copied",
        child.parent_wait_stack_snapshot_copied() as usize,
    );
    sink.diag_usize("wait4_writable_pages_saved", obs.writable_pages_saved);
    sink.diag_usize("wait4_writable_pages_count", obs.writable_pages_count);
    sink.diag_usize(
        "wait4_writable_pages_truncated",
        obs.writable_pages_truncated,
    );
    sink.diag_usize(
        "wait4_parent_writable_snapshot_copied",
        child.parent_wait_writable_page_snapshot_copied() as usize,
    );
    sink.diag_usize("wait4_child_task_slot", child_task_ref.slot());
    sink.diag_usize(
        "wait4_child_task_generation",
        child_task_ref.generation() as usize,
    );
    sink.diag_usize("wait4_current_task_ref_id", current_task_ref.slot());
    sink.diag_usize(
        "wait4_current_task_generation",
        current_task_ref.generation() as usize,
    );
    sink.diag_usize("wait4_child_task_state", child.active_task_state() as usize);
    sink.diag_usize("wait4_child_flow_slot", child_flow_ref.slot());
    sink.diag_usize(
        "wait4_child_flow_generation",
        child_flow_ref.generation() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "wait4 parent saved facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_child_parent_wait_resumed(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_child.parent_wait_resumed";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let obs = wait4_checkpoint_observation();
    let runqueue_contains_internal_child = ctx
        .scheduler()
        .contains_task_ref(child.last_exited_task_ref());
    let valid = child.child_exit_status_observed()
        && child.wait4_status_copied()
        && child.parent_wait_resumed()
        && child.parent_wait_resume_checkpoint_bound()
        && obs.saved != 0
        && obs.resumed != 0
        && obs.resumed_sepc != 0
        && obs.resumed_sp != 0
        && obs.resumed_a0 == crate::objects::user_boot::USER_CHILD_PID
        && obs.resumed_a7 == 260
        && obs.resumed_status_ptr == obs.saved_status_ptr
        && obs.resumed_wait_status == 0
        && obs.resumed_status_copied != 0
        && obs.child_exit_status == child.child_exit_status()
        && child.parent_wait_stack_window_compared()
        && child.parent_wait_stack_snapshot_restored()
        && obs.stack_window_compared != 0
        && obs.stack_window_start == child.parent_wait_stack_window_start()
        && obs.stack_window_len == child.parent_wait_stack_window_len()
        && child.parent_wait_writable_page_compared()
        && child.parent_wait_writable_page_snapshot_restored()
        && obs.writable_pages_compared != 0
        && obs.writable_pages_restored != 0;

    sink.diag_hex_pair("wait4_resumed_sepc_sp", obs.resumed_sepc, obs.resumed_sp);
    sink.diag_hex_pair("wait4_resumed_s2_s4", obs.resumed_s2, obs.resumed_s4);
    sink.diag_hex_pair(
        "wait4_resumed_satp_status",
        obs.resumed_satp,
        obs.resumed_status_ptr,
    );
    sink.diag_usize("wait4_resumed_a0", obs.resumed_a0);
    sink.diag_usize("wait4_resumed_a7", obs.resumed_a7);
    sink.diag_usize("wait4_resumed_wait_status", obs.resumed_wait_status);
    sink.diag_usize("wait4_resumed_status_copied", obs.resumed_status_copied);
    sink.diag_usize("wait4_child_exit_status", obs.child_exit_status);
    sink.diag_usize("child_lifecycle", child.active_task_state() as usize);
    sink.diag_usize("child_task_ref_slot", child.last_exited_task_ref().slot());
    sink.diag_usize(
        "child_task_generation",
        child.last_exited_task_ref().generation() as usize,
    );
    sink.diag_usize("child_flow_ref_slot", child.last_exited_flow_ref().slot());
    sink.diag_usize(
        "child_flow_generation",
        child.last_exited_flow_ref().generation() as usize,
    );
    sink.diag_usize("child_pid", child.pid());
    sink.diag_usize("child_parent_pid", child.parent_pid());
    sink.diag_usize("child_tgid", child.tgid());
    sink.diag_usize(
        "child_continuation_taken",
        child.child_continuation_taken() as usize,
    );
    sink.diag_usize("parent_wait_resumed", child.parent_wait_resumed() as usize);
    sink.diag_usize(
        "vfork_parent_resumed",
        child.vfork_parent_resumed() as usize,
    );
    sink.diag_usize(
        "observed_plain_fork_child_active",
        child.observed_plain_fork_child_active() as usize,
    );
    sink.diag_usize(
        "observed_plain_fork_parent_restored",
        child.observed_plain_fork_parent_restored() as usize,
    );
    sink.diag_usize(
        "current_child_continuation",
        child.current_child_continuation() as usize,
    );
    sink.diag_usize("child_enqueued", child.enqueued() as usize);
    sink.diag_usize(
        "runqueue_contains_internal_child",
        runqueue_contains_internal_child as usize,
    );
    sink.diag_usize("next_child_pid", child.next_child_pid());
    sink.diag_usize(
        "completed_child_record_count",
        child.completed_child_record_count(),
    );
    sink.diag_usize("wait4_stack_window_compared", obs.stack_window_compared);
    sink.diag_usize("wait4_stack_window_diff_count", obs.stack_window_diff_count);
    sink.diag_hex_pair(
        "wait4_stack_window_first_diff",
        obs.stack_window_first_diff_addr,
        obs.stack_window_before_byte,
    );
    sink.diag_usize("wait4_stack_window_after_byte", obs.stack_window_after_byte);
    sink.diag_usize(
        "wait4_parent_stack_snapshot_restored",
        child.parent_wait_stack_snapshot_restored() as usize,
    );
    sink.diag_usize(
        "wait4_parent_writable_snapshot_restored",
        child.parent_wait_writable_page_snapshot_restored() as usize,
    );
    sink.diag_usize("wait4_writable_pages_compared", obs.writable_pages_compared);
    sink.diag_usize("wait4_writable_pages_restored", obs.writable_pages_restored);
    sink.diag_usize(
        "wait4_writable_pages_dirty_count",
        obs.writable_pages_dirty_count,
    );
    sink.diag_usize(
        "wait4_writable_pages_stack_dirty_count",
        obs.writable_pages_stack_dirty_count,
    );
    sink.diag_usize(
        "wait4_writable_pages_non_stack_dirty_count",
        obs.writable_pages_non_stack_dirty_count,
    );
    sink.diag_usize(
        "wait4_writable_pages_first_non_stack_kind",
        obs.writable_pages_first_non_stack_kind,
    );
    sink.diag_hex_pair(
        "wait4_writable_pages_first_non_stack_index_page",
        obs.writable_pages_first_non_stack_mapping_index,
        obs.writable_pages_first_non_stack_page_index,
    );
    sink.diag_hex_pair(
        "wait4_writable_pages_first_non_stack_addr_before",
        obs.writable_pages_first_non_stack_addr,
        obs.writable_pages_first_non_stack_before_checksum,
    );
    sink.diag_usize(
        "wait4_writable_pages_first_non_stack_after_checksum",
        obs.writable_pages_first_non_stack_after_checksum,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "parent wait resumed facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_clone_vfork_parent_resumed(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_clone.vfork_parent_resumed";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let files = &ctx.files_struct;
    let pidfd_ok = if child.vfork_pidfd_clone() {
        files.pidfd_ready() && files.pidfd_child_pid() == child.pid()
    } else {
        child.vfork_vm_clone() && !files.pidfd_ready()
    };
    let valid = child.vfork_clone()
        && child.child_exit_status_observed()
        && child.vfork_parent_resumed()
        && !child.current_child_continuation()
        && child.parent_clone_return() == crate::objects::user_boot::USER_CHILD_PID
        && child.parent_wait_writable_page_snapshot_restored()
        && pidfd_ok;

    sink.diag_usize(
        "vfork_parent_resumed",
        child.vfork_parent_resumed() as usize,
    );
    sink.diag_usize(
        "vfork_current_child_continuation",
        child.current_child_continuation() as usize,
    );
    sink.diag_usize("vfork_parent_clone_return", child.parent_clone_return());
    sink.diag_usize("vfork_child_exit_status", child.child_exit_status());
    sink.diag_usize(
        "vfork_writable_snapshot_restored",
        child.parent_wait_writable_page_snapshot_restored() as usize,
    );
    sink.diag_usize("vfork_pidfd_ready", files.pidfd_ready() as usize);
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "vfork parent resumed facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_child_record_archived(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_child_record.archived";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let valid = child.completed_child_record_archived()
        && child.completed_child_record_count() != 0
        && child.completed_child_record_count() <= USER_COMPLETED_CHILD_RECORD_CAPACITY
        && child.completed_child_record_total_archived() != 0
        && child.last_archived_child_pid() >= crate::objects::user_boot::USER_CHILD_PID
        && child.last_archived_child_wait_status()
            == ((child.last_archived_child_exit_status() & 0xff) << 8);

    sink.diag_usize(
        "completed_child_record_archived",
        child.completed_child_record_archived() as usize,
    );
    sink.diag_usize(
        "completed_child_record_count",
        child.completed_child_record_count(),
    );
    sink.diag_usize(
        "completed_child_record_capacity",
        USER_COMPLETED_CHILD_RECORD_CAPACITY,
    );
    sink.diag_usize(
        "completed_child_record_free_count",
        child.completed_child_record_free_count(),
    );
    sink.diag_usize(
        "completed_child_record_total_archived",
        child.completed_child_record_total_archived(),
    );
    sink.diag_usize("last_archived_child_pid", child.last_archived_child_pid());
    sink.diag_usize(
        "last_archived_child_exit_status",
        child.last_archived_child_exit_status(),
    );
    sink.diag_usize(
        "last_archived_child_wait_status",
        child.last_archived_child_wait_status(),
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "completed child archive facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_task_record_released(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_task_record.released";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let runqueue_contains_internal_child = ctx
        .scheduler()
        .contains_task_ref(child.last_exited_task_ref());
    let valid = child.state() == State::Ready
        && child.active_task_state() == State::Prepared
        && child.active_task_record_available()
        && child.completed_child_record_archived()
        && child.task_allocation_count() != 0
        && child.next_child_pid() > child.last_archived_child_pid()
        && !runqueue_contains_internal_child;

    sink.diag_usize(
        "active_task_record_available",
        child.active_task_record_available() as usize,
    );
    sink.diag_usize("task_allocation_count", child.task_allocation_count());
    sink.diag_usize("next_child_pid", child.next_child_pid());
    sink.diag_usize(
        "runqueue_contains_internal_child",
        runqueue_contains_internal_child as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "released task record facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_user_child_record_reaped(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.user_child_record.reaped";
    sink.start_case(total, "", name, checkpoint);

    let child = &ctx.user_task_set;
    let valid = child.completed_child_record_reaped()
        && child.completed_child_record_released()
        && child.completed_child_record_reaped_count() != 0
        && child.completed_child_record_released_count() != 0
        && child.last_reaped_child_pid() >= crate::objects::user_boot::USER_CHILD_PID
        && child.last_reaped_child_wait_status() != usize::MAX
        && child.last_released_child_pid() == child.last_reaped_child_pid()
        && child.last_released_child_wait_status() == child.last_reaped_child_wait_status()
        && child.completed_child_record_count() < USER_COMPLETED_CHILD_RECORD_CAPACITY;

    sink.diag_usize(
        "completed_child_record_reaped",
        child.completed_child_record_reaped() as usize,
    );
    sink.diag_usize(
        "completed_child_record_released",
        child.completed_child_record_released() as usize,
    );
    sink.diag_usize(
        "completed_child_record_count",
        child.completed_child_record_count(),
    );
    sink.diag_usize(
        "completed_child_record_free_count",
        child.completed_child_record_free_count(),
    );
    sink.diag_usize(
        "completed_child_record_reaped_count",
        child.completed_child_record_reaped_count(),
    );
    sink.diag_usize(
        "completed_child_record_released_count",
        child.completed_child_record_released_count(),
    );
    sink.diag_usize("last_reaped_child_pid", child.last_reaped_child_pid());
    sink.diag_usize(
        "last_reaped_child_wait_status",
        child.last_reaped_child_wait_status(),
    );
    sink.diag_usize("last_released_child_pid", child.last_released_child_pid());
    sink.diag_usize(
        "last_released_child_wait_status",
        child.last_released_child_wait_status(),
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "completed child reap facts invalid");
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

    let process = &ctx.kernel_init_user_state;
    let table = &ctx.syscall_table;
    let valid = process.active_user_flow_online()
        && process.runtime_entered()
        && process.syscall_context_bound()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.exit_supported()
        && table.exit_group_supported()
        && table.exit_records_status()
        && table.exit_group_pid1_shutdown_child_wait4_split()
        && table.write_observed()
        && table.exit_observed()
        && ctx.boot_cpu_exception().syscall_state() == State::Online;

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
    sink.diag_usize(
        "syscall_table_exit_group_pid1_shutdown_child_wait4_split",
        table.exit_group_pid1_shutdown_child_wait4_split() as usize,
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

    // ELF fixture construction keeps program-header fields explicit.
    #[allow(clippy::too_many_arguments)]
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
