use super::{
    command_line::SavedCommandLine,
    rcu::RcuCore,
    rest_init::{SystemState, SystemStateValue},
    rootfs::RootfsBoundary,
    runtime_core::AsyncCoreDeferred,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct AsyncFullSyncDeferred {
    lifecycle: Lifecycle,
    synchronize_full_deferred: bool,
    init_work_drain_boundary_preserved: bool,
    waitqueue_deferred: bool,
    async_lock_irqsave_deferred: bool,
    entry_count_atomic_deferred: bool,
    global_cookie_boundary_preserved: bool,
}

impl AsyncFullSyncDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            synchronize_full_deferred: false,
            init_work_drain_boundary_preserved: false,
            waitqueue_deferred: false,
            async_lock_irqsave_deferred: false,
            entry_count_atomic_deferred: false,
            global_cookie_boundary_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn synchronize_full_deferred(&self) -> bool {
        self.synchronize_full_deferred
    }

    pub const fn init_work_drain_boundary_preserved(&self) -> bool {
        self.init_work_drain_boundary_preserved
    }

    pub const fn waitqueue_deferred(&self) -> bool {
        self.waitqueue_deferred
    }

    pub const fn async_lock_irqsave_deferred(&self) -> bool {
        self.async_lock_irqsave_deferred
    }

    pub const fn entry_count_atomic_deferred(&self) -> bool {
        self.entry_count_atomic_deferred
    }

    pub const fn global_cookie_boundary_preserved(&self) -> bool {
        self.global_cookie_boundary_preserved
    }

    pub fn setup(
        &mut self,
        rootfs_boundary: &RootfsBoundary,
        async_core: &AsyncCoreDeferred,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rootfs_boundary.state() != State::Ready
            || !rootfs_boundary.finalize_next_boundary()
            || async_core.state() != State::Ready
            || !async_core.setup_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.synchronize_full_deferred = true;
        self.init_work_drain_boundary_preserved = true;
        self.waitqueue_deferred = true;
        self.async_lock_irqsave_deferred = true;
        self.entry_count_atomic_deferred = true;
        self.global_cookie_boundary_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::AsyncFullSyncDeferredReady,
        )
    }
}

pub struct InitMemoryCleanupDeferred {
    lifecycle: Lifecycle,
    system_state_freeing_window_entered: bool,
    kprobe_trimmed_noop: bool,
    ftrace_cleanup_deferred: bool,
    kgdb_trimmed_noop: bool,
    bootconfig_exit_trimmed_noop: bool,
    free_deferred: bool,
}

impl InitMemoryCleanupDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            system_state_freeing_window_entered: false,
            kprobe_trimmed_noop: false,
            ftrace_cleanup_deferred: false,
            kgdb_trimmed_noop: false,
            bootconfig_exit_trimmed_noop: false,
            free_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn system_state_freeing_window_entered(&self) -> bool {
        self.system_state_freeing_window_entered
    }

    pub const fn kprobe_trimmed_noop(&self) -> bool {
        self.kprobe_trimmed_noop
    }

    pub const fn ftrace_cleanup_deferred(&self) -> bool {
        self.ftrace_cleanup_deferred
    }

    pub const fn kgdb_trimmed_noop(&self) -> bool {
        self.kgdb_trimmed_noop
    }

    pub const fn bootconfig_exit_trimmed_noop(&self) -> bool {
        self.bootconfig_exit_trimmed_noop
    }

    pub const fn free_deferred(&self) -> bool {
        self.free_deferred
    }

    pub fn setup(
        &mut self,
        async_full_sync: &AsyncFullSyncDeferred,
        system_state: &SystemState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || async_full_sync.state() != State::Ready
            || !async_full_sync.synchronize_full_deferred()
            || system_state.state() != State::Ready
            || system_state.value() != SystemStateValue::FreeingInitmem
            || !system_state.freeing_initmem_window_entered()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.system_state_freeing_window_entered = true;
        self.kprobe_trimmed_noop = true;
        self.ftrace_cleanup_deferred = true;
        self.kgdb_trimmed_noop = true;
        self.bootconfig_exit_trimmed_noop = true;
        self.free_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::InitMemoryCleanupDeferredReady,
        )
    }
}

pub struct KernelMappingProtectionDeferred {
    lifecycle: Lifecycle,
    enable_deferred: bool,
    strict_kernel_rwx_position_preserved: bool,
    rodata_debug_test_trimmed_or_deferred: bool,
}

impl KernelMappingProtectionDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            enable_deferred: false,
            strict_kernel_rwx_position_preserved: false,
            rodata_debug_test_trimmed_or_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn enable_deferred(&self) -> bool {
        self.enable_deferred
    }

    pub const fn strict_kernel_rwx_position_preserved(&self) -> bool {
        self.strict_kernel_rwx_position_preserved
    }

    pub const fn rodata_debug_test_trimmed_or_deferred(&self) -> bool {
        self.rodata_debug_test_trimmed_or_deferred
    }

    pub fn setup(&mut self, init_memory: &InitMemoryCleanupDeferred) -> EventResult {
        if self.lifecycle.state() != State::Base
            || init_memory.state() != State::Ready
            || !init_memory.free_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.enable_deferred = true;
        self.strict_kernel_rwx_position_preserved = true;
        self.rodata_debug_test_trimmed_or_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KernelMappingProtectionDeferredReady,
        )
    }
}

pub struct PtiFinalizeTrimmed {
    lifecycle: Lifecycle,
    trimmed_noop: bool,
}

impl PtiFinalizeTrimmed {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trimmed_noop: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn trimmed_noop(&self) -> bool {
        self.trimmed_noop
    }

    pub fn setup(&mut self, mapping: &KernelMappingProtectionDeferred) -> EventResult {
        if self.lifecycle.state() != State::Base
            || mapping.state() != State::Ready
            || !mapping.enable_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.trimmed_noop = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PtiFinalizeTrimmedReady,
        )
    }
}

pub struct NumaDefaultPolicyTrimmed {
    lifecycle: Lifecycle,
    trimmed_noop: bool,
    config_numa_disabled: bool,
}

impl NumaDefaultPolicyTrimmed {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trimmed_noop: false,
            config_numa_disabled: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn trimmed_noop(&self) -> bool {
        self.trimmed_noop
    }

    pub const fn config_numa_disabled(&self) -> bool {
        self.config_numa_disabled
    }

    pub fn setup(
        &mut self,
        pti_finalize: &PtiFinalizeTrimmed,
        system_state: &SystemState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || pti_finalize.state() != State::Ready
            || !pti_finalize.trimmed_noop()
            || system_state.state() != State::Online
            || system_state.value() != SystemStateValue::Running
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.trimmed_noop = true;
        self.config_numa_disabled = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::NumaDefaultPolicyNoop,
        )
    }
}

pub struct RcuBootEnd {
    lifecycle: Lifecycle,
    rcu_boot_ended: bool,
}

impl RcuBootEnd {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            rcu_boot_ended: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn rcu_boot_ended(&self) -> bool {
        self.rcu_boot_ended
    }

    pub fn setup(
        &mut self,
        numa_default_policy: &NumaDefaultPolicyTrimmed,
        system_state: &SystemState,
        rcu_core: &mut RcuCore,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || numa_default_policy.state() != State::Ready
            || !numa_default_policy.trimmed_noop()
            || system_state.state() != State::Online
            || system_state.value() != SystemStateValue::Running
            || rcu_core.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !rcu_core.end_inkernel_boot() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.rcu_boot_ended = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RcuBootEndReady,
        )
    }
}

pub struct SysctlArgsDeferred {
    lifecycle: Lifecycle,
    apply_deferred: bool,
    command_line_position_preserved: bool,
}

impl SysctlArgsDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            apply_deferred: false,
            command_line_position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn apply_deferred(&self) -> bool {
        self.apply_deferred
    }

    pub const fn command_line_position_preserved(&self) -> bool {
        self.command_line_position_preserved
    }

    pub fn setup(
        &mut self,
        rcu_boot_end: &RcuBootEnd,
        saved_command_line: &SavedCommandLine,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rcu_boot_end.state() != State::Ready
            || !rcu_boot_end.rcu_boot_ended()
            || saved_command_line.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.apply_deferred = true;
        self.command_line_position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SysctlArgsDeferredReady,
        )
    }
}

pub struct FinalizeBoundary {
    lifecycle: Lifecycle,
    payload_next_boundary: bool,
}

impl FinalizeBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            payload_next_boundary: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn payload_next_boundary(&self) -> bool {
        self.payload_next_boundary
    }

    pub fn setup(
        &mut self,
        async_full_sync: &AsyncFullSyncDeferred,
        init_memory: &InitMemoryCleanupDeferred,
        mapping: &KernelMappingProtectionDeferred,
        pti_finalize: &PtiFinalizeTrimmed,
        numa_default_policy: &NumaDefaultPolicyTrimmed,
        rcu_boot_end: &RcuBootEnd,
        sysctl_args: &SysctlArgsDeferred,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || async_full_sync.state() != State::Ready
            || !async_full_sync.synchronize_full_deferred()
            || init_memory.state() != State::Ready
            || !init_memory.free_deferred()
            || mapping.state() != State::Ready
            || !mapping.enable_deferred()
            || pti_finalize.state() != State::Ready
            || !pti_finalize.trimmed_noop()
            || numa_default_policy.state() != State::Ready
            || !numa_default_policy.trimmed_noop()
            || rcu_boot_end.state() != State::Ready
            || !rcu_boot_end.rcu_boot_ended()
            || sysctl_args.state() != State::Ready
            || !sysctl_args.apply_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.payload_next_boundary = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::FinalizeBoundaryReady,
        )
    }
}

pub fn finalize_phase_ready(
    async_full_sync: &AsyncFullSyncDeferred,
    init_memory: &InitMemoryCleanupDeferred,
    mapping: &KernelMappingProtectionDeferred,
    pti_finalize: &PtiFinalizeTrimmed,
    numa_default_policy: &NumaDefaultPolicyTrimmed,
    system_state: &SystemState,
    rcu_core: &RcuCore,
    rcu_boot_end: &RcuBootEnd,
    sysctl_args: &SysctlArgsDeferred,
    boundary: &FinalizeBoundary,
) -> bool {
    async_full_sync.state() == State::Ready
        && async_full_sync.synchronize_full_deferred()
        && async_full_sync.init_work_drain_boundary_preserved()
        && async_full_sync.waitqueue_deferred()
        && async_full_sync.async_lock_irqsave_deferred()
        && async_full_sync.entry_count_atomic_deferred()
        && async_full_sync.global_cookie_boundary_preserved()
        && init_memory.state() == State::Ready
        && init_memory.system_state_freeing_window_entered()
        && init_memory.kprobe_trimmed_noop()
        && init_memory.ftrace_cleanup_deferred()
        && init_memory.kgdb_trimmed_noop()
        && init_memory.bootconfig_exit_trimmed_noop()
        && init_memory.free_deferred()
        && mapping.state() == State::Ready
        && mapping.enable_deferred()
        && mapping.strict_kernel_rwx_position_preserved()
        && mapping.rodata_debug_test_trimmed_or_deferred()
        && pti_finalize.state() == State::Ready
        && pti_finalize.trimmed_noop()
        && numa_default_policy.state() == State::Ready
        && numa_default_policy.trimmed_noop()
        && numa_default_policy.config_numa_disabled()
        && system_state.state() == State::Online
        && system_state.value() == SystemStateValue::Running
        && rcu_core.inkernel_boot_ended()
        && rcu_core.unexpedite_gp_atomic_decrement_recorded()
        && rcu_core.async_relax_config_lazy_trimmed()
        && rcu_core.normal_after_boot_write_once_trimmed_or_recorded()
        && rcu_core.boot_ended_publish_recorded()
        && rcu_boot_end.state() == State::Ready
        && rcu_boot_end.rcu_boot_ended()
        && sysctl_args.state() == State::Ready
        && sysctl_args.apply_deferred()
        && sysctl_args.command_line_position_preserved()
        && boundary.state() == State::Ready
        && boundary.payload_next_boundary()
}
