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
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::WorkqueuePrepared,
        )
    }
}
