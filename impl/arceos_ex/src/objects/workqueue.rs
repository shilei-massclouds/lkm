use super::{
    cpu_group::CpuGroup,
    mm_core::{PageAllocator, SlubAllocator},
    per_cpu_storage::PerCpuStorage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct Workqueue {
    lifecycle: Lifecycle,
    system_queues_ready: bool,
    worker_pools_prepared: bool,
    unbound_cpumask_ready: bool,
    bh_pools_ready: bool,
    workers_running: bool,
    worker_creation_open: bool,
    rescuers_ready: bool,
    initial_workers_created: bool,
    watchdog_ready: bool,
    smp_topology_deferred: bool,
    possible_cpu_count: usize,
}

impl Workqueue {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            system_queues_ready: false,
            worker_pools_prepared: false,
            unbound_cpumask_ready: false,
            bh_pools_ready: false,
            workers_running: false,
            worker_creation_open: false,
            rescuers_ready: false,
            initial_workers_created: false,
            watchdog_ready: false,
            smp_topology_deferred: true,
            possible_cpu_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn system_queues_ready(&self) -> bool {
        self.system_queues_ready
    }

    pub const fn worker_pools_prepared(&self) -> bool {
        self.worker_pools_prepared
    }

    pub const fn unbound_cpumask_ready(&self) -> bool {
        self.unbound_cpumask_ready
    }

    pub const fn bh_pools_ready(&self) -> bool {
        self.bh_pools_ready
    }

    pub const fn workers_running(&self) -> bool {
        self.workers_running
    }

    pub const fn worker_creation_open(&self) -> bool {
        self.worker_creation_open
    }

    pub const fn rescuers_ready(&self) -> bool {
        self.rescuers_ready
    }

    pub const fn initial_workers_created(&self) -> bool {
        self.initial_workers_created
    }

    pub const fn watchdog_ready(&self) -> bool {
        self.watchdog_ready
    }

    pub const fn smp_topology_deferred(&self) -> bool {
        self.smp_topology_deferred
    }

    pub const fn possible_cpu_count(&self) -> usize {
        self.possible_cpu_count
    }

    pub fn preset(
        &mut self,
        page_allocator: &PageAllocator,
        slub_allocator: &SlubAllocator,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || page_allocator.state() != State::Ready
            || slub_allocator.state() != State::Ready
            || cpu_group.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.possible_cpu_count = cpu_group.possible_cpu_count();
        self.system_queues_ready = true;
        self.worker_pools_prepared = true;
        self.unbound_cpumask_ready = true;
        self.bh_pools_ready = true;
        self.workers_running = false;
        self.worker_creation_open = false;
        self.rescuers_ready = false;
        self.initial_workers_created = false;
        self.watchdog_ready = false;
        self.smp_topology_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::WorkqueuePrepared,
        )
    }

    pub fn setup(&mut self, page_allocator: &PageAllocator, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || page_allocator.state() != State::Ready
            || !page_allocator.full_gfp_mask_open()
            || cpu_group.state() != State::Ready
            || !cpu_group.pre_smp_topology_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.rescuers_ready = true;
        self.initial_workers_created = true;
        self.worker_creation_open = true;
        self.watchdog_ready = true;
        self.workers_running = false;
        self.smp_topology_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::WorkqueueReady,
        )
    }
}
