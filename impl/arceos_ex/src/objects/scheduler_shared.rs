use super::{
    cpu_group::PossibleCpuInventory,
    default_sched_root_domain::DefaultSchedRootDomain,
    mutex::{Mutex, MutexLockOutcome, MutexOwner},
    scheduler::BitWaitQueueTable,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

/// Cross-CPU scheduling resources.  Linux keeps these outside each CPU-local
/// `struct rq`; the same ownership boundary is explicit here.
pub struct SchedulerShared {
    lifecycle: Lifecycle,
    sched_domains_mutex: Mutex,
    default_root_domain: DefaultSchedRootDomain,
    bit_wait_queue_table: BitWaitQueueTable,
    smp_initialized: bool,
    sched_domains_ready: bool,
    sched_domains_mutex_guard_used: bool,
    smp_cpu_masks_stable: bool,
    kernel_init_affinity_released: bool,
    rt_dl_smp_ready: bool,
    granularity_refreshed: bool,
}

impl SchedulerShared {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            sched_domains_mutex: Mutex::new_static(),
            default_root_domain: DefaultSchedRootDomain::new(),
            bit_wait_queue_table: BitWaitQueueTable::new(),
            smp_initialized: false,
            sched_domains_ready: false,
            sched_domains_mutex_guard_used: false,
            smp_cpu_masks_stable: false,
            kernel_init_affinity_released: false,
            rt_dl_smp_ready: false,
            granularity_refreshed: false,
        }
    }

    pub fn preset(&mut self, inventory: PossibleCpuInventory) -> EventResult {
        if self.lifecycle.state() != State::Base || inventory.count() == 0 {
            return self.failed_preset();
        }
        self.default_root_domain.setup_inventory(inventory)?;
        self.bit_wait_queue_table.preset()?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable_smp(
        &mut self,
        smp_cpu_inventory_ready: bool,
        kernel_init_affinity_released: bool,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !smp_cpu_inventory_ready
            || !kernel_init_affinity_released
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        if self.sched_domains_mutex.state() == State::Base {
            self.sched_domains_mutex.preset_static()?;
            self.sched_domains_mutex.setup()?;
        }
        if self.sched_domains_mutex.state() != State::Ready
            || self
                .sched_domains_mutex
                .lock_owner(MutexOwner::KernelInitTask)?
                != MutexLockOutcome::Acquired
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        self.sched_domains_mutex_guard_used = true;
        self.smp_cpu_masks_stable = true;
        self.sched_domains_mutex
            .unlock_owner(MutexOwner::KernelInitTask)?;

        self.smp_initialized = true;
        self.sched_domains_ready = true;
        self.kernel_init_affinity_released = true;
        self.rt_dl_smp_ready = true;
        self.granularity_refreshed = true;
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSmpReady);
        Ok(())
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn sched_domains_mutex(&self) -> &Mutex {
        &self.sched_domains_mutex
    }

    pub const fn default_root_domain(&self) -> &DefaultSchedRootDomain {
        &self.default_root_domain
    }

    pub const fn bit_wait_queue_table(&self) -> &BitWaitQueueTable {
        &self.bit_wait_queue_table
    }

    pub const fn smp_initialized(&self) -> bool {
        self.smp_initialized
    }

    pub const fn sched_domains_ready(&self) -> bool {
        self.sched_domains_ready
    }

    pub const fn sched_domains_mutex_guard_used(&self) -> bool {
        self.sched_domains_mutex_guard_used
    }

    pub const fn smp_cpu_masks_stable(&self) -> bool {
        self.smp_cpu_masks_stable
    }

    pub const fn kernel_init_affinity_released(&self) -> bool {
        self.kernel_init_affinity_released
    }

    pub const fn rt_dl_smp_ready(&self) -> bool {
        self.rt_dl_smp_ready
    }

    pub const fn granularity_refreshed(&self) -> bool {
        self.granularity_refreshed
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
