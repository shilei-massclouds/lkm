use super::{
    cpu_group::CpuGroup,
    cpu_hotplug::CpuHotplugState,
    mm_core::PageAllocator,
    scheduler::Scheduler,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

pub struct AsyncCoreDeferred {
    lifecycle: Lifecycle,
    setup_deferred: bool,
    workqueue_creation_deferred: bool,
}

impl AsyncCoreDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            workqueue_creation_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn setup_deferred(&self) -> bool {
        self.setup_deferred
    }

    pub const fn workqueue_creation_deferred(&self) -> bool {
        self.workqueue_creation_deferred
    }

    pub fn setup(&mut self, workqueue: &Workqueue) -> EventResult {
        if self.lifecycle.state() != State::Base
            || workqueue.state() != State::Ready
            || !workqueue.topology_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.setup_deferred = true;
        self.workqueue_creation_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::AsyncCoreDeferredReady,
        )
    }
}

pub struct PadataCoreDeferred {
    lifecycle: Lifecycle,
    setup_deferred: bool,
    hotplug_steps_deferred: bool,
    work_array_deferred: bool,
}

impl PadataCoreDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            hotplug_steps_deferred: false,
            work_array_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn setup_deferred(&self) -> bool {
        self.setup_deferred
    }

    pub const fn hotplug_steps_deferred(&self) -> bool {
        self.hotplug_steps_deferred
    }

    pub const fn work_array_deferred(&self) -> bool {
        self.work_array_deferred
    }

    pub fn setup(
        &mut self,
        async_core: &AsyncCoreDeferred,
        cpu_hotplug_state: &CpuHotplugState,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || async_core.state() != State::Ready
            || cpu_hotplug_state.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !cpu_group.smp_concurrency_open()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.setup_deferred = true;
        self.hotplug_steps_deferred = true;
        self.work_array_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PadataCoreDeferredReady,
        )
    }
}

pub struct RuntimeCoreBoundary {
    lifecycle: Lifecycle,
    do_basic_setup_next_boundary: bool,
}

impl RuntimeCoreBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            do_basic_setup_next_boundary: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn do_basic_setup_next_boundary(&self) -> bool {
        self.do_basic_setup_next_boundary
    }

    pub fn setup(
        &mut self,
        scheduler: &Scheduler,
        workqueue: &Workqueue,
        async_core: &AsyncCoreDeferred,
        padata_core: &PadataCoreDeferred,
        page_allocator: &PageAllocator,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || !scheduler.smp_initialized()
            || workqueue.state() != State::Ready
            || !workqueue.topology_ready()
            || async_core.state() != State::Ready
            || padata_core.state() != State::Ready
            || page_allocator.state() != State::Ready
            || !page_allocator.late_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.do_basic_setup_next_boundary = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RuntimeCoreBoundaryReady,
        )
    }
}

pub fn runtime_core_ready(
    scheduler: &Scheduler,
    workqueue: &Workqueue,
    async_core: &AsyncCoreDeferred,
    padata_core: &PadataCoreDeferred,
    page_allocator: &PageAllocator,
    boundary: &RuntimeCoreBoundary,
) -> bool {
    scheduler.state() == State::Online
        && scheduler.smp_initialized()
        && scheduler.sched_domains_ready()
        && scheduler.kernel_init_affinity_released()
        && scheduler.rt_dl_smp_ready()
        && scheduler.granularity_refreshed()
        && workqueue.state() == State::Ready
        && workqueue.topology_ready()
        && workqueue.pod_types_ready()
        && workqueue.unbound_pools_rebound()
        && workqueue.max_active_topology_ready()
        && !workqueue.workers_running()
        && async_core.state() == State::Ready
        && async_core.setup_deferred()
        && async_core.workqueue_creation_deferred()
        && padata_core.state() == State::Ready
        && padata_core.setup_deferred()
        && padata_core.hotplug_steps_deferred()
        && padata_core.work_array_deferred()
        && page_allocator.state() == State::Ready
        && page_allocator.late_ready()
        && page_allocator.memory_stats_ready()
        && page_allocator.buffer_init_ready()
        && page_allocator.memblock_private_discarded()
        && page_allocator.zone_contiguous_ready()
        && page_allocator.sysctl_ready()
        && page_allocator.deferred_struct_page_init_trimmed()
        && page_allocator.page_extension_late_trimmed()
        && page_allocator.shuffle_late_trimmed()
        && boundary.state() == State::Ready
        && boundary.do_basic_setup_next_boundary()
}
