/* Shared boot/runtime exec synchronization and point-of-no-return boundary. */

predicate exec_sync_boundaries_ready<T>(boundaries: T) -> bool;
predicate exec_sync_single_active_transaction<T>(boundaries: T) -> bool;
predicate exec_sync_point_of_no_return_bound<T>(boundaries: T) -> bool;
predicate exec_sync_cloexec_precheck_bound<T>(boundaries: T) -> bool;
predicate exec_sync_current_staging_mm_handoff_bound<T>(boundaries: T) -> bool;
predicate exec_sync_retired_mm_release_bound<T>(boundaries: T) -> bool;
predicate exec_sync_boot_runtime_owner_handoff_bound<T>(boundaries: T) -> bool;

predicate payload_exec_sync_boundaries_ready<T>(boundaries: T) -> bool;
predicate payload_kernel_execve_linux_window_bound<T>(boundaries: T) -> bool;
predicate payload_binfmt_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_cred_guard_mutex_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_update_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_mmap_local_irq_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_task_siglock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_tasklist_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_fs_lock_rcu_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_mmap_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_membarrier_deferred<T>(boundaries: T) -> bool;
predicate payload_bprm_mm_init_task_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_mmap_task_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_sched_mm_cid_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_files_unshare_cloexec_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_io_uring_cancel_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_posix_timer_siglock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_namespace_switch_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_success_accounting_hooks_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_full_binfmt_deferred<T>(boundaries: T) -> bool;
predicate payload_binfmt_module_retry_trimmed_noop<T>(boundaries: T) -> bool;
predicate payload_binfmt_module_retry_trimmed_because_modules_disabled<T>(boundaries: T) -> bool;
predicate payload_ramdisk_init_branch_trimmed_noop<T>(boundaries: T) -> bool;
predicate payload_ramdisk_init_trimmed_because_config_initrd_disabled<T>(boundaries: T) -> bool;
predicate payload_default_init_branch_trimmed_noop<T>(boundaries: T) -> bool;
predicate payload_default_init_trimmed_because_config_default_init_empty<T>(boundaries: T) -> bool;
predicate payload_binfmt_script_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_panic_terminal_bound<T>(boundaries: T) -> bool;

object ExecSyncBoundaries: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    BinaryFormatRegistry.state == State::Ready;
                    PayloadParam.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    SystemState.state == State::Online;
                    system_state_running(SystemState);
                }

                ensures {
                    exec_sync_boundaries_ready(self);
                    exec_sync_single_active_transaction(self);
                    exec_sync_point_of_no_return_bound(self);
                    exec_sync_cloexec_precheck_bound(self);
                    exec_sync_current_staging_mm_handoff_bound(self);
                    exec_sync_retired_mm_release_bound(self);
                    exec_sync_boot_runtime_owner_handoff_bound(self);
                    payload_exec_sync_boundaries_ready(self);
                    payload_kernel_execve_linux_window_bound(self);
                    payload_binfmt_lock_deferred(self);
                    payload_cred_guard_mutex_deferred(self);
                    payload_exec_update_lock_deferred(self);
                    payload_exec_mmap_local_irq_deferred(self);
                    payload_exec_task_siglock_deferred(self);
                    payload_exec_tasklist_lock_deferred(self);
                    payload_exec_fs_lock_rcu_deferred(self);
                    payload_exec_mmap_lock_deferred(self);
                    payload_exec_membarrier_deferred(self);
                    payload_bprm_mm_init_task_lock_deferred(self);
                    payload_exec_mmap_task_lock_deferred(self);
                    payload_exec_sched_mm_cid_deferred(self);
                    payload_exec_files_unshare_cloexec_deferred(self);
                    payload_exec_io_uring_cancel_deferred(self);
                    payload_exec_posix_timer_siglock_deferred(self);
                    payload_exec_namespace_switch_deferred(self);
                    payload_exec_success_accounting_hooks_deferred(self);
                    payload_exec_full_binfmt_deferred(self);
                    payload_binfmt_module_retry_trimmed_noop(self);
                    payload_binfmt_module_retry_trimmed_because_modules_disabled(self);
                    payload_ramdisk_init_branch_trimmed_noop(self);
                    payload_ramdisk_init_trimmed_because_config_initrd_disabled(self);
                    payload_default_init_branch_trimmed_noop(self);
                    payload_default_init_trimmed_because_config_default_init_empty(self);
                    payload_binfmt_script_deferred(self);
                    payload_exec_panic_terminal_bound(self);
                }

                deferred {
                    "Linux 6.12 exec locks, credential/signal/LSM hooks, namespaces and accounting remain deferred; CONFIG_MODULES=n makes binfmt module retry a trimmed no-op.";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            exec_sync_boundaries_ready(self);
            exec_sync_single_active_transaction(self);
            exec_sync_point_of_no_return_bound(self);
            exec_sync_cloexec_precheck_bound(self);
            exec_sync_current_staging_mm_handoff_bound(self);
            exec_sync_retired_mm_release_bound(self);
            exec_sync_boot_runtime_owner_handoff_bound(self);
            payload_exec_sync_boundaries_ready(self);
            payload_kernel_execve_linux_window_bound(self);
            payload_binfmt_lock_deferred(self);
            payload_cred_guard_mutex_deferred(self);
            payload_exec_update_lock_deferred(self);
            payload_exec_mmap_local_irq_deferred(self);
            payload_exec_task_siglock_deferred(self);
            payload_exec_tasklist_lock_deferred(self);
            payload_exec_fs_lock_rcu_deferred(self);
            payload_exec_mmap_lock_deferred(self);
            payload_exec_membarrier_deferred(self);
            payload_bprm_mm_init_task_lock_deferred(self);
            payload_exec_mmap_task_lock_deferred(self);
            payload_exec_sched_mm_cid_deferred(self);
            payload_exec_files_unshare_cloexec_deferred(self);
            payload_exec_io_uring_cancel_deferred(self);
            payload_exec_posix_timer_siglock_deferred(self);
            payload_exec_namespace_switch_deferred(self);
            payload_exec_success_accounting_hooks_deferred(self);
            payload_exec_full_binfmt_deferred(self);
            payload_binfmt_module_retry_trimmed_noop(self);
            payload_binfmt_module_retry_trimmed_because_modules_disabled(self);
            payload_ramdisk_init_branch_trimmed_noop(self);
            payload_ramdisk_init_trimmed_because_config_initrd_disabled(self);
            payload_default_init_branch_trimmed_noop(self);
            payload_default_init_trimmed_because_config_default_init_empty(self);
            payload_binfmt_script_deferred(self);
            payload_exec_panic_terminal_bound(self);
        }
    }
}
