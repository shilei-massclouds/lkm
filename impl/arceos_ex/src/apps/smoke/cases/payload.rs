use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{config::SelectedPayloadKind, printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let boundaries = &ctx.payload_exec_sync_boundaries;

    if crate::phases::payload::state() != State::Online
        || ctx.config.selected_payload_kind() != SelectedPayloadKind::Smoke
        || ctx.selected_payload_handoff.state() != State::Online
        || ctx.selected_payload_handoff.kind() != SelectedPayloadKind::Smoke
        || !ctx.selected_payload_handoff.kind_bound()
        || !ctx.selected_payload_handoff.variant_setup_ready()
        || !ctx.selected_payload_handoff.variant_prepare_ready()
        || !ctx.selected_payload_handoff.no_return_entry_bound()
        || ctx.user_clone_deferred_boundaries.state() != State::Ready
        || !ctx
            .user_clone_deferred_boundaries
            .plain_fork_first_slice_bound()
        || !ctx
            .user_clone_deferred_boundaries
            .vfork_vm_first_slice_bound()
        || !ctx
            .user_clone_deferred_boundaries
            .vfork_pidfd_first_slice_bound()
        || boundaries.state() != State::Ready
        || !boundaries.kernel_execve_linux_window_bound()
        || !boundaries.binfmt_lock_deferred()
        || !boundaries.cred_guard_mutex_deferred()
        || !boundaries.exec_update_lock_deferred()
        || !boundaries.exec_mmap_local_irq_deferred()
        || !boundaries.exec_task_siglock_deferred()
        || !boundaries.exec_tasklist_lock_deferred()
        || !boundaries.exec_fs_lock_rcu_deferred()
        || !boundaries.exec_mmap_lock_deferred()
        || !boundaries.exec_membarrier_deferred()
        || !boundaries.bprm_mm_init_task_lock_deferred()
        || !boundaries.exec_mmap_task_lock_deferred()
        || !boundaries.exec_sched_mm_cid_deferred()
        || !boundaries.exec_files_unshare_cloexec_deferred()
        || !boundaries.exec_io_uring_cancel_deferred()
        || !boundaries.exec_posix_timer_siglock_deferred()
        || !boundaries.exec_namespace_switch_deferred()
        || !boundaries.exec_success_accounting_hooks_deferred()
        || !boundaries.exec_full_binfmt_deferred()
        || !boundaries.binfmt_module_retry_trimmed_noop()
        || !boundaries.binfmt_module_retry_trimmed_because_modules_disabled()
        || !boundaries.ramdisk_init_branch_trimmed_noop()
        || !boundaries.ramdisk_init_trimmed_because_config_initrd_disabled()
        || !boundaries.default_init_branch_trimmed_noop()
        || !boundaries.default_init_trimmed_because_config_default_init_empty()
        || !boundaries.binfmt_script_deferred()
        || !boundaries.exec_panic_terminal_bound()
    {
        printk::write_str("payload lifecycle or deferred boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_str("payload exec sync deferred\n");
    SmokeResult::Passed
}
