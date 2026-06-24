use super::{
    cpu::SecondaryCpuStore,
    cpu_group::CpuGroup,
    irq_time::SbiIpi,
    per_cpu_storage::PerCpuStorage,
    pre_smp_init::PreSmpInitBoundary,
    rest_init::{BootIdleRuntime, KernelInitTask, KthreaddTask},
    scheduler::Scheduler,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct SecondaryIdleTaskSet {
    lifecycle: Lifecycle,
    prepared_count: usize,
    inactive: bool,
}

impl SecondaryIdleTaskSet {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            prepared_count: 0,
            inactive: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn prepared_count(&self) -> usize {
        self.prepared_count
    }

    pub const fn inactive(&self) -> bool {
        self.inactive
    }

    pub fn preset(
        &mut self,
        boundary: &PreSmpInitBoundary,
        cpu_group: &CpuGroup,
        scheduler: &Scheduler,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || boundary.state() != State::Ready
            || !boundary.smp_init_not_called()
            || !boundary.secondary_cpus_present_not_online()
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_present_not_online()
            || scheduler.state() != State::Online
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.prepared_count = cpu_group.secondary_count();
        self.inactive = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SecondaryIdleTasksPrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct CpuHotplugSyncSet {
    lifecycle: Lifecycle,
    cpu_running_ready: bool,
    cpu_running_observed: bool,
    done_up_ready: bool,
    done_up_observed: bool,
    done_down_ready: bool,
    boot_cpu_hotplug_thread_online: bool,
    secondary_hotplug_threads_deferred: bool,
}

impl CpuHotplugSyncSet {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cpu_running_ready: false,
            cpu_running_observed: false,
            done_up_ready: false,
            done_up_observed: false,
            done_down_ready: false,
            boot_cpu_hotplug_thread_online: false,
            secondary_hotplug_threads_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cpu_running_ready(&self) -> bool {
        self.cpu_running_ready
    }

    pub const fn cpu_running_observed(&self) -> bool {
        self.cpu_running_observed
    }

    pub const fn done_up_ready(&self) -> bool {
        self.done_up_ready
    }

    pub const fn done_up_observed(&self) -> bool {
        self.done_up_observed
    }

    pub const fn done_down_ready(&self) -> bool {
        self.done_down_ready
    }

    pub const fn boot_cpu_hotplug_thread_online(&self) -> bool {
        self.boot_cpu_hotplug_thread_online
    }

    pub const fn secondary_hotplug_threads_deferred(&self) -> bool {
        self.secondary_hotplug_threads_deferred
    }

    pub fn preset(
        &mut self,
        cpu_group: &CpuGroup,
        boot_idle_runtime: &BootIdleRuntime,
        kthreadd_task: &KthreaddTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
            || !cpu_group.secondary_cpus_present_not_online()
            || boot_idle_runtime.state() != State::Ready
            || kthreadd_task.state() != State::Online
        {
            return self.failed_preset();
        }

        self.cpu_running_ready = true;
        self.cpu_running_observed = false;
        self.done_up_ready = true;
        self.done_up_observed = false;
        self.done_down_ready = true;
        self.boot_cpu_hotplug_thread_online = true;
        self.secondary_hotplug_threads_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CpuHotplugSyncPrepared,
        )
    }

    pub fn observe_cpu_running(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.cpu_running_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.cpu_running_observed = true;
        crate::trace::checkpoint(Checkpoint::CpuRunningObserved);
        Ok(())
    }

    pub fn observe_done_up(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.done_up_ready
            || !self.cpu_running_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.done_up_observed = true;
        crate::trace::checkpoint(Checkpoint::CpuDoneUpObserved);
        Ok(())
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct CpuStartProvider {
    lifecycle: Lifecycle,
    start_requests_issued: bool,
    ap_entry_detail_deferred: bool,
}

impl CpuStartProvider {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start_requests_issued: false,
            ap_entry_detail_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn start_requests_issued(&self) -> bool {
        self.start_requests_issued
    }

    pub const fn ap_entry_detail_deferred(&self) -> bool {
        self.ap_entry_detail_deferred
    }

    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        idle_tasks: &SecondaryIdleTaskSet,
        sync: &CpuHotplugSyncSet,
        sbi_ipi: &SbiIpi,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_present_not_online()
            || idle_tasks.state() != State::Prepared
            || sync.state() != State::Prepared
            || !sync.cpu_running_ready()
            || !sync.done_up_ready()
            || !sync.done_down_ready()
            || sbi_ipi.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.start_requests_issued = true;
        self.ap_entry_detail_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::CpuStartProviderReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct SecondaryCpuStartupAck {
    lifecycle: Lifecycle,
    acknowledged: bool,
    ap_entry_detail_deferred: bool,
}

impl SecondaryCpuStartupAck {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            acknowledged: false,
            ap_entry_detail_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub const fn ap_entry_detail_deferred(&self) -> bool {
        self.ap_entry_detail_deferred
    }

    pub fn setup(
        &mut self,
        start_provider: &CpuStartProvider,
        sync: &mut CpuHotplugSyncSet,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || start_provider.state() != State::Ready
            || !start_provider.start_requests_issued()
            || sync.state() != State::Prepared
        {
            return self.failed_setup();
        }

        sync.observe_cpu_running()?;
        self.acknowledged = true;
        self.ap_entry_detail_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SecondaryCpuStartupAckReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct SecondaryCpuOnlineAck {
    lifecycle: Lifecycle,
    acknowledged: bool,
    ap_idle_detail_deferred: bool,
}

impl SecondaryCpuOnlineAck {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            acknowledged: false,
            ap_idle_detail_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub const fn ap_idle_detail_deferred(&self) -> bool {
        self.ap_idle_detail_deferred
    }

    pub fn setup(
        &mut self,
        startup_ack: &SecondaryCpuStartupAck,
        sync: &mut CpuHotplugSyncSet,
        cpu_group: &mut CpuGroup,
        secondary_cpus: &mut SecondaryCpuStore,
        sbi_ipi: &SbiIpi,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || startup_ack.state() != State::Ready
            || !startup_ack.acknowledged()
            || sync.state() != State::Prepared
            || sbi_ipi.state() != State::Ready
        {
            return self.failed_setup();
        }

        sync.observe_done_up()?;
        cpu_group.mark_secondary_cpus_online(secondary_cpus)?;
        self.acknowledged = true;
        self.ap_idle_detail_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SecondaryCpuOnlineAckReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct SmpBringupBoundary {
    lifecycle: Lifecycle,
    smp_cpus_done_trimmed: bool,
    ap_hotplug_callbacks_deferred: bool,
}

impl SmpBringupBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            smp_cpus_done_trimmed: false,
            ap_hotplug_callbacks_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn smp_cpus_done_trimmed(&self) -> bool {
        self.smp_cpus_done_trimmed
    }

    pub const fn ap_hotplug_callbacks_deferred(&self) -> bool {
        self.ap_hotplug_callbacks_deferred
    }

    pub fn setup(
        &mut self,
        online_ack: &SecondaryCpuOnlineAck,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || online_ack.state() != State::Ready
            || !online_ack.acknowledged()
            || !cpu_group.secondary_cpus_online()
            || !cpu_group.smp_concurrency_open()
        {
            return self.failed_setup();
        }

        self.smp_cpus_done_trimmed = true;
        self.ap_hotplug_callbacks_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SmpBringupBoundaryReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub fn smp_bringup_runtime_ready(
    kernel_init_task: &KernelInitTask,
    cpu_group: &CpuGroup,
    idle_tasks: &SecondaryIdleTaskSet,
    sync: &CpuHotplugSyncSet,
    start_provider: &CpuStartProvider,
    startup_ack: &SecondaryCpuStartupAck,
    online_ack: &SecondaryCpuOnlineAck,
    boundary: &SmpBringupBoundary,
) -> bool {
    kernel_init_task.state() == State::Online
        && cpu_group.state() == State::Ready
        && cpu_group.secondary_cpus_online()
        && cpu_group.smp_concurrency_open()
        && idle_tasks.state() == State::Prepared
        && idle_tasks.inactive()
        && sync.state() == State::Prepared
        && sync.cpu_running_ready()
        && sync.cpu_running_observed()
        && sync.done_up_ready()
        && sync.done_up_observed()
        && sync.done_down_ready()
        && sync.boot_cpu_hotplug_thread_online()
        && sync.secondary_hotplug_threads_deferred()
        && start_provider.state() == State::Ready
        && start_provider.ap_entry_detail_deferred()
        && startup_ack.state() == State::Ready
        && startup_ack.ap_entry_detail_deferred()
        && online_ack.state() == State::Ready
        && online_ack.ap_idle_detail_deferred()
        && boundary.state() == State::Ready
        && boundary.smp_cpus_done_trimmed()
        && boundary.ap_hotplug_callbacks_deferred()
}
