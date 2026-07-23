use super::{
    binary_format_registry::BinaryFormatRegistry,
    rest_init::{KernelInitTask, SystemState, SystemStateValue},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

pub struct ExecSyncBoundaries {
    lifecycle: Lifecycle,
    kernel_execve_linux_window_bound: bool,
    binfmt_lock_deferred: bool,
    cred_guard_mutex_deferred: bool,
    exec_update_lock_deferred: bool,
    exec_mmap_local_irq_deferred: bool,
    exec_task_siglock_deferred: bool,
    exec_tasklist_lock_deferred: bool,
    exec_fs_lock_rcu_deferred: bool,
    exec_mmap_lock_deferred: bool,
    exec_membarrier_deferred: bool,
    bprm_mm_init_task_lock_deferred: bool,
    exec_mmap_task_lock_deferred: bool,
    exec_sched_mm_cid_deferred: bool,
    exec_files_unshare_cloexec_deferred: bool,
    exec_io_uring_cancel_deferred: bool,
    exec_posix_timer_siglock_deferred: bool,
    exec_namespace_switch_deferred: bool,
    exec_success_accounting_hooks_deferred: bool,
    exec_full_binfmt_deferred: bool,
    binfmt_module_retry_trimmed_noop: bool,
    binfmt_module_retry_trimmed_because_modules_disabled: bool,
    ramdisk_init_branch_trimmed_noop: bool,
    ramdisk_init_trimmed_because_config_initrd_disabled: bool,
    default_init_branch_trimmed_noop: bool,
    default_init_trimmed_because_config_default_init_empty: bool,
    binfmt_script_deferred: bool,
    exec_panic_terminal_bound: bool,
    single_active_transaction: bool,
    point_of_no_return_bound: bool,
    cloexec_precheck_bound: bool,
    current_staging_mm_handoff_bound: bool,
    retired_mm_release_bound: bool,
    boot_runtime_owner_handoff_bound: bool,
}

#[allow(dead_code)]
impl ExecSyncBoundaries {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kernel_execve_linux_window_bound: false,
            binfmt_lock_deferred: false,
            cred_guard_mutex_deferred: false,
            exec_update_lock_deferred: false,
            exec_mmap_local_irq_deferred: false,
            exec_task_siglock_deferred: false,
            exec_tasklist_lock_deferred: false,
            exec_fs_lock_rcu_deferred: false,
            exec_mmap_lock_deferred: false,
            exec_membarrier_deferred: false,
            bprm_mm_init_task_lock_deferred: false,
            exec_mmap_task_lock_deferred: false,
            exec_sched_mm_cid_deferred: false,
            exec_files_unshare_cloexec_deferred: false,
            exec_io_uring_cancel_deferred: false,
            exec_posix_timer_siglock_deferred: false,
            exec_namespace_switch_deferred: false,
            exec_success_accounting_hooks_deferred: false,
            exec_full_binfmt_deferred: false,
            binfmt_module_retry_trimmed_noop: false,
            binfmt_module_retry_trimmed_because_modules_disabled: false,
            ramdisk_init_branch_trimmed_noop: false,
            ramdisk_init_trimmed_because_config_initrd_disabled: false,
            default_init_branch_trimmed_noop: false,
            default_init_trimmed_because_config_default_init_empty: false,
            binfmt_script_deferred: false,
            exec_panic_terminal_bound: false,
            single_active_transaction: false,
            point_of_no_return_bound: false,
            cloexec_precheck_bound: false,
            current_staging_mm_handoff_bound: false,
            retired_mm_release_bound: false,
            boot_runtime_owner_handoff_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kernel_execve_linux_window_bound(&self) -> bool {
        self.kernel_execve_linux_window_bound
    }

    pub const fn binfmt_lock_deferred(&self) -> bool {
        self.binfmt_lock_deferred
    }

    pub const fn cred_guard_mutex_deferred(&self) -> bool {
        self.cred_guard_mutex_deferred
    }

    pub const fn exec_update_lock_deferred(&self) -> bool {
        self.exec_update_lock_deferred
    }

    pub const fn exec_mmap_local_irq_deferred(&self) -> bool {
        self.exec_mmap_local_irq_deferred
    }

    pub const fn exec_task_siglock_deferred(&self) -> bool {
        self.exec_task_siglock_deferred
    }

    pub const fn exec_tasklist_lock_deferred(&self) -> bool {
        self.exec_tasklist_lock_deferred
    }

    pub const fn exec_fs_lock_rcu_deferred(&self) -> bool {
        self.exec_fs_lock_rcu_deferred
    }

    pub const fn exec_mmap_lock_deferred(&self) -> bool {
        self.exec_mmap_lock_deferred
    }

    pub const fn exec_membarrier_deferred(&self) -> bool {
        self.exec_membarrier_deferred
    }

    pub const fn bprm_mm_init_task_lock_deferred(&self) -> bool {
        self.bprm_mm_init_task_lock_deferred
    }

    pub const fn exec_mmap_task_lock_deferred(&self) -> bool {
        self.exec_mmap_task_lock_deferred
    }

    pub const fn exec_sched_mm_cid_deferred(&self) -> bool {
        self.exec_sched_mm_cid_deferred
    }

    pub const fn exec_files_unshare_cloexec_deferred(&self) -> bool {
        self.exec_files_unshare_cloexec_deferred
    }

    pub const fn exec_io_uring_cancel_deferred(&self) -> bool {
        self.exec_io_uring_cancel_deferred
    }

    pub const fn exec_posix_timer_siglock_deferred(&self) -> bool {
        self.exec_posix_timer_siglock_deferred
    }

    pub const fn exec_namespace_switch_deferred(&self) -> bool {
        self.exec_namespace_switch_deferred
    }

    pub const fn exec_success_accounting_hooks_deferred(&self) -> bool {
        self.exec_success_accounting_hooks_deferred
    }

    pub const fn exec_full_binfmt_deferred(&self) -> bool {
        self.exec_full_binfmt_deferred
    }

    pub const fn binfmt_module_retry_trimmed_noop(&self) -> bool {
        self.binfmt_module_retry_trimmed_noop
    }

    pub const fn binfmt_module_retry_trimmed_because_modules_disabled(&self) -> bool {
        self.binfmt_module_retry_trimmed_because_modules_disabled
    }

    pub const fn ramdisk_init_branch_trimmed_noop(&self) -> bool {
        self.ramdisk_init_branch_trimmed_noop
    }

    pub const fn ramdisk_init_trimmed_because_config_initrd_disabled(&self) -> bool {
        self.ramdisk_init_trimmed_because_config_initrd_disabled
    }

    pub const fn default_init_branch_trimmed_noop(&self) -> bool {
        self.default_init_branch_trimmed_noop
    }

    pub const fn default_init_trimmed_because_config_default_init_empty(&self) -> bool {
        self.default_init_trimmed_because_config_default_init_empty
    }

    pub const fn binfmt_script_deferred(&self) -> bool {
        self.binfmt_script_deferred
    }

    pub const fn exec_panic_terminal_bound(&self) -> bool {
        self.exec_panic_terminal_bound
    }

    pub const fn single_active_transaction(&self) -> bool {
        self.single_active_transaction
    }

    pub const fn point_of_no_return_bound(&self) -> bool {
        self.point_of_no_return_bound
    }

    pub const fn cloexec_precheck_bound(&self) -> bool {
        self.cloexec_precheck_bound
    }

    pub const fn current_staging_mm_handoff_bound(&self) -> bool {
        self.current_staging_mm_handoff_bound
    }

    pub const fn retired_mm_release_bound(&self) -> bool {
        self.retired_mm_release_bound
    }

    pub const fn boot_runtime_owner_handoff_bound(&self) -> bool {
        self.boot_runtime_owner_handoff_bound
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        system_state: &SystemState,
        registry: &BinaryFormatRegistry,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::OnCpu
            || system_state.state() != State::Online
            || system_state.value() != SystemStateValue::Running
            || registry.state() != State::Ready
            || !registry.only_elf_handler()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.kernel_execve_linux_window_bound = true;
        self.binfmt_lock_deferred = true;
        self.cred_guard_mutex_deferred = true;
        self.exec_update_lock_deferred = true;
        self.exec_mmap_local_irq_deferred = true;
        self.exec_task_siglock_deferred = true;
        self.exec_tasklist_lock_deferred = true;
        self.exec_fs_lock_rcu_deferred = true;
        self.exec_mmap_lock_deferred = true;
        self.exec_membarrier_deferred = true;
        self.bprm_mm_init_task_lock_deferred = true;
        self.exec_mmap_task_lock_deferred = true;
        self.exec_sched_mm_cid_deferred = true;
        self.exec_files_unshare_cloexec_deferred = true;
        self.exec_io_uring_cancel_deferred = true;
        self.exec_posix_timer_siglock_deferred = true;
        self.exec_namespace_switch_deferred = true;
        self.exec_success_accounting_hooks_deferred = true;
        self.exec_full_binfmt_deferred = true;
        self.binfmt_module_retry_trimmed_noop = true;
        self.binfmt_module_retry_trimmed_because_modules_disabled = true;
        self.ramdisk_init_branch_trimmed_noop = true;
        self.ramdisk_init_trimmed_because_config_initrd_disabled = true;
        self.default_init_branch_trimmed_noop = true;
        self.default_init_trimmed_because_config_default_init_empty = true;
        self.binfmt_script_deferred = true;
        self.exec_panic_terminal_bound = true;
        self.single_active_transaction = true;
        self.point_of_no_return_bound = true;
        self.cloexec_precheck_bound = true;
        self.current_staging_mm_handoff_bound = true;
        self.retired_mm_release_bound = true;
        self.boot_runtime_owner_handoff_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}
