use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let boundaries = &ctx.payload_exec_sync_boundaries;

    if boundaries.state() != State::Ready
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
        || !boundaries.exec_full_binfmt_deferred()
        || !boundaries.ramdisk_init_branch_trimmed_noop()
        || !boundaries.ramdisk_init_trimmed_because_config_initrd_disabled()
        || !boundaries.default_init_branch_trimmed_noop()
        || !boundaries.default_init_trimmed_because_config_default_init_empty()
        || !boundaries.binfmt_script_deferred()
        || !boundaries.exec_panic_terminal_bound()
    {
        printk::write_str("payload exec sync boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_str("payload exec sync deferred\n");
    SmokeResult::Passed
}
