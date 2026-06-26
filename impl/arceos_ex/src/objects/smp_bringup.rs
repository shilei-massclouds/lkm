use super::{
    cpu::SecondaryCpuStore,
    cpu_control::{LocalInterruptControl, RawSpinLock},
    cpu_group::CpuGroup,
    irq_time::SbiIpi,
    mutex::{Mutex, MutexLockOutcome, MutexOwner},
    per_cpu_storage::PerCpuStorage,
    percpu_rw_semaphore::{
        PerCpuRwSemaphore, PerCpuRwSemaphoreOwner, PerCpuRwSemaphoreReadOutcome,
        PerCpuRwSemaphoreWriteOutcome,
    },
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
    cpus_read_guard_used: bool,
    smpboot_threads_mutex_guard_used: bool,
    cpu_running_wait_lock_guard_used: bool,
    done_up_wait_lock_guard_used: bool,
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
            cpus_read_guard_used: false,
            smpboot_threads_mutex_guard_used: false,
            cpu_running_wait_lock_guard_used: false,
            done_up_wait_lock_guard_used: false,
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

    pub const fn cpus_read_guard_used(&self) -> bool {
        self.cpus_read_guard_used
    }

    pub const fn smpboot_threads_mutex_guard_used(&self) -> bool {
        self.smpboot_threads_mutex_guard_used
    }

    pub const fn cpu_running_wait_lock_guard_used(&self) -> bool {
        self.cpu_running_wait_lock_guard_used
    }

    pub const fn done_up_wait_lock_guard_used(&self) -> bool {
        self.done_up_wait_lock_guard_used
    }

    pub fn preset(
        &mut self,
        cpu_group: &CpuGroup,
        boot_idle_runtime: &BootIdleRuntime,
        kthreadd_task: &KthreaddTask,
        cpu_hotplug_lock: &mut PerCpuRwSemaphore,
        smpboot_threads_lock: &mut Mutex,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
            || !cpu_group.secondary_cpus_present_not_online()
            || boot_idle_runtime.state() != State::Ready
            || kthreadd_task.state() != State::Online
            || !cpu_hotplug_lock.ready()
            || !smpboot_threads_lock.ready()
        {
            return self.failed_preset();
        }

        if cpu_hotplug_lock.read_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, 0)?
            != PerCpuRwSemaphoreReadOutcome::AcquiredFast
        {
            return self.failed_preset();
        }
        crate::trace::checkpoint(Checkpoint::CpuHotplugReadGuardUsed);

        if smpboot_threads_lock.lock_owner(MutexOwner::KernelInitTask)?
            != MutexLockOutcome::Acquired
        {
            return self.failed_preset();
        }
        crate::trace::checkpoint(Checkpoint::SmpbootThreadsMutexGuardUsed);

        self.cpu_running_ready = true;
        self.cpu_running_observed = false;
        self.done_up_ready = true;
        self.done_up_observed = false;
        self.done_down_ready = true;
        self.boot_cpu_hotplug_thread_online = true;
        self.secondary_hotplug_threads_deferred = true;
        self.cpus_read_guard_used = true;
        self.smpboot_threads_mutex_guard_used = true;

        smpboot_threads_lock.unlock_owner(MutexOwner::KernelInitTask)?;
        cpu_hotplug_lock.read_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, 0)?;

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CpuHotplugSyncPrepared,
        )
    }

    pub fn observe_cpu_running(
        &mut self,
        wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.cpu_running_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        wait_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.cpu_running_observed = true;
        wait_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.cpu_running_wait_lock_guard_used = true;
        crate::trace::checkpoint(Checkpoint::CpuRunningObserved);
        crate::trace::checkpoint(Checkpoint::CpuRunningWaitLockGuardUsed);
        Ok(())
    }

    pub fn observe_done_up(
        &mut self,
        wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
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

        wait_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.done_up_observed = true;
        wait_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.done_up_wait_lock_guard_used = true;
        crate::trace::checkpoint(Checkpoint::CpuDoneUpObserved);
        crate::trace::checkpoint(Checkpoint::CpuDoneUpWaitLockGuardUsed);
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
    cpu_add_remove_mutex_guard_used: bool,
    cpu_hotplug_write_guard_used: bool,
    sbi_boot_data_publish_barriers_observed: bool,
}

impl CpuStartProvider {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start_requests_issued: false,
            ap_entry_detail_deferred: true,
            cpu_add_remove_mutex_guard_used: false,
            cpu_hotplug_write_guard_used: false,
            sbi_boot_data_publish_barriers_observed: false,
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

    pub const fn cpu_add_remove_mutex_guard_used(&self) -> bool {
        self.cpu_add_remove_mutex_guard_used
    }

    pub const fn cpu_hotplug_write_guard_used(&self) -> bool {
        self.cpu_hotplug_write_guard_used
    }

    pub const fn sbi_boot_data_publish_barriers_observed(&self) -> bool {
        self.sbi_boot_data_publish_barriers_observed
    }

    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        idle_tasks: &SecondaryIdleTaskSet,
        sync: &CpuHotplugSyncSet,
        sbi_ipi: &SbiIpi,
        cpu_add_remove_lock: &mut Mutex,
        cpu_hotplug_lock: &mut PerCpuRwSemaphore,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_present_not_online()
            || idle_tasks.state() != State::Prepared
            || sync.state() != State::Prepared
            || !sync.cpu_running_ready()
            || !sync.done_up_ready()
            || !sync.done_down_ready()
            || !sync.cpus_read_guard_used()
            || !sync.smpboot_threads_mutex_guard_used()
            || sbi_ipi.state() != State::Ready
            || !cpu_add_remove_lock.ready()
            || !cpu_hotplug_lock.ready()
        {
            return self.failed_setup();
        }

        if cpu_add_remove_lock.lock_owner(MutexOwner::KernelInitTask)? != MutexLockOutcome::Acquired
        {
            return self.failed_setup();
        }

        if cpu_hotplug_lock.write_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask)?
            != PerCpuRwSemaphoreWriteOutcome::Acquired
        {
            return self.failed_setup();
        }

        self.start_requests_issued = true;
        self.ap_entry_detail_deferred = true;
        self.cpu_add_remove_mutex_guard_used = true;
        self.cpu_hotplug_write_guard_used = true;
        self.sbi_boot_data_publish_barriers_observed = true;
        crate::trace::checkpoint(Checkpoint::CpuHotplugWriteGuardUsed);
        crate::trace::checkpoint(Checkpoint::CpuStartProviderBootDataPublished);

        cpu_hotplug_lock.write_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask)?;
        cpu_add_remove_lock.unlock_owner(MutexOwner::KernelInitTask)?;

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
        cpu_running_wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || start_provider.state() != State::Ready
            || !start_provider.start_requests_issued()
            || !start_provider.cpu_add_remove_mutex_guard_used()
            || !start_provider.cpu_hotplug_write_guard_used()
            || !start_provider.sbi_boot_data_publish_barriers_observed()
            || sync.state() != State::Prepared
        {
            return self.failed_setup();
        }

        sync.observe_cpu_running(cpu_running_wait_lock, local_interrupt, scheduler)?;
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
    ap_local_irq_enable_deferred: bool,
    ap_cache_tlb_flush_summary_observed: bool,
    ap_ipi_enable_observed: bool,
    ap_hotplug_thread_mb_pair_deferred: bool,
}

impl SecondaryCpuOnlineAck {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            acknowledged: false,
            ap_idle_detail_deferred: true,
            ap_local_irq_enable_deferred: true,
            ap_cache_tlb_flush_summary_observed: false,
            ap_ipi_enable_observed: false,
            ap_hotplug_thread_mb_pair_deferred: true,
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

    pub const fn ap_local_irq_enable_deferred(&self) -> bool {
        self.ap_local_irq_enable_deferred
    }

    pub const fn ap_cache_tlb_flush_summary_observed(&self) -> bool {
        self.ap_cache_tlb_flush_summary_observed
    }

    pub const fn ap_ipi_enable_observed(&self) -> bool {
        self.ap_ipi_enable_observed
    }

    pub const fn ap_hotplug_thread_mb_pair_deferred(&self) -> bool {
        self.ap_hotplug_thread_mb_pair_deferred
    }

    pub fn setup(
        &mut self,
        startup_ack: &SecondaryCpuStartupAck,
        sync: &mut CpuHotplugSyncSet,
        cpu_group: &mut CpuGroup,
        secondary_cpus: &mut SecondaryCpuStore,
        sbi_ipi: &SbiIpi,
        done_up_wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || startup_ack.state() != State::Ready
            || !startup_ack.acknowledged()
            || sync.state() != State::Prepared
            || sbi_ipi.state() != State::Ready
        {
            return self.failed_setup();
        }

        sync.observe_done_up(done_up_wait_lock, local_interrupt, scheduler)?;
        cpu_group.mark_secondary_cpus_online(secondary_cpus)?;
        self.acknowledged = true;
        self.ap_idle_detail_deferred = true;
        self.ap_local_irq_enable_deferred = true;
        self.ap_cache_tlb_flush_summary_observed = true;
        self.ap_ipi_enable_observed = true;
        self.ap_hotplug_thread_mb_pair_deferred = true;
        crate::trace::checkpoint(Checkpoint::SecondaryCpuApLocalSyncSummary);
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
    smpboot_threads_lock: &Mutex,
    sync: &CpuHotplugSyncSet,
    cpu_add_remove_lock: &Mutex,
    cpu_hotplug_lock: &PerCpuRwSemaphore,
    cpu_running_wait_lock: &RawSpinLock,
    done_up_wait_lock: &RawSpinLock,
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
        && smpboot_threads_lock.state() == State::Ready
        && smpboot_threads_lock.ready()
        && sync.state() == State::Prepared
        && sync.cpu_running_ready()
        && sync.cpu_running_observed()
        && sync.done_up_ready()
        && sync.done_up_observed()
        && sync.done_down_ready()
        && sync.boot_cpu_hotplug_thread_online()
        && sync.secondary_hotplug_threads_deferred()
        && sync.cpus_read_guard_used()
        && sync.smpboot_threads_mutex_guard_used()
        && sync.cpu_running_wait_lock_guard_used()
        && sync.done_up_wait_lock_guard_used()
        && cpu_add_remove_lock.state() == State::Ready
        && cpu_add_remove_lock.ready()
        && cpu_hotplug_lock.state() == State::Ready
        && cpu_hotplug_lock.ready()
        && cpu_hotplug_lock.read_lock_count() != 0
        && cpu_hotplug_lock.read_unlock_count() == cpu_hotplug_lock.read_lock_count()
        && cpu_hotplug_lock.write_lock_count() != 0
        && cpu_hotplug_lock.write_unlock_count() == cpu_hotplug_lock.write_lock_count()
        && cpu_running_wait_lock.state() == State::Ready
        && cpu_running_wait_lock.irqsave_entered_count() != 0
        && cpu_running_wait_lock.irqrestore_exited_count()
            == cpu_running_wait_lock.irqsave_entered_count()
        && done_up_wait_lock.state() == State::Ready
        && done_up_wait_lock.irqsave_entered_count() != 0
        && done_up_wait_lock.irqrestore_exited_count() == done_up_wait_lock.irqsave_entered_count()
        && start_provider.state() == State::Ready
        && start_provider.ap_entry_detail_deferred()
        && start_provider.cpu_add_remove_mutex_guard_used()
        && start_provider.cpu_hotplug_write_guard_used()
        && start_provider.sbi_boot_data_publish_barriers_observed()
        && startup_ack.state() == State::Ready
        && startup_ack.ap_entry_detail_deferred()
        && online_ack.state() == State::Ready
        && online_ack.ap_idle_detail_deferred()
        && online_ack.ap_local_irq_enable_deferred()
        && online_ack.ap_cache_tlb_flush_summary_observed()
        && online_ack.ap_ipi_enable_observed()
        && online_ack.ap_hotplug_thread_mb_pair_deferred()
        && boundary.state() == State::Ready
        && boundary.smp_cpus_done_trimmed()
        && boundary.ap_hotplug_callbacks_deferred()
}
