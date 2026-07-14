use super::{
    cpu_group::CpuGroup,
    init_task::InitTask,
    mm_core::{NamedSlubCacheKind, PageAllocator, SlubSubsystem},
    mutex::{Mutex, MutexLockOutcome, MutexOwner},
    per_cpu_storage::PerCpuStorage,
    rest_init::KernelInitTask,
    scheduler::Scheduler,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

const EARLY_SYSTEM_WORKQUEUE_COUNT: usize = 9;
const POOL_WORKQUEUE_CACHE_OBJECT_SIZE: usize = core::mem::size_of::<usize>() * 12;

pub struct Workqueue {
    lifecycle: Lifecycle,
    pool_mutex: Mutex,
    struct_mutex: Mutex,
    system_queues_ready: bool,
    system_queue_count: usize,
    worker_pools_prepared: bool,
    cpu_worker_pools_ready: bool,
    unbound_cpumask_ready: bool,
    bh_pools_ready: bool,
    pool_workqueue_cache_ready: bool,
    pool_workqueue_cache_object_size: usize,
    registered_in_slub_registry: bool,
    attrs_ready: bool,
    system_affinity_pods_ready: bool,
    pool_attach_mutex_deferred: bool,
    mayday_lock_deferred: bool,
    manager_wait_deferred: bool,
    pool_mutex_guard_used: bool,
    struct_mutex_guard_used: bool,
    pre_smp_pool_mutex_guard_used: bool,
    topology_pool_mutex_guard_used: bool,
    topology_struct_mutex_guard_used: bool,
    workers_running: bool,
    worker_creation_open: bool,
    rescuers_ready: bool,
    initial_workers_created: bool,
    watchdog_ready: bool,
    smp_topology_deferred: bool,
    topology_ready: bool,
    pod_types_ready: bool,
    unbound_pools_rebound: bool,
    max_active_topology_ready: bool,
    possible_cpu_count: usize,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Workqueue {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pool_mutex: Mutex::new_static(),
            struct_mutex: Mutex::new_static(),
            system_queues_ready: false,
            system_queue_count: 0,
            worker_pools_prepared: false,
            cpu_worker_pools_ready: false,
            unbound_cpumask_ready: false,
            bh_pools_ready: false,
            pool_workqueue_cache_ready: false,
            pool_workqueue_cache_object_size: 0,
            registered_in_slub_registry: false,
            attrs_ready: false,
            system_affinity_pods_ready: false,
            pool_attach_mutex_deferred: false,
            mayday_lock_deferred: false,
            manager_wait_deferred: false,
            pool_mutex_guard_used: false,
            struct_mutex_guard_used: false,
            pre_smp_pool_mutex_guard_used: false,
            topology_pool_mutex_guard_used: false,
            topology_struct_mutex_guard_used: false,
            workers_running: false,
            worker_creation_open: false,
            rescuers_ready: false,
            initial_workers_created: false,
            watchdog_ready: false,
            smp_topology_deferred: true,
            topology_ready: false,
            pod_types_ready: false,
            unbound_pools_rebound: false,
            max_active_topology_ready: false,
            possible_cpu_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pool_mutex(&self) -> &Mutex {
        &self.pool_mutex
    }

    pub const fn struct_mutex(&self) -> &Mutex {
        &self.struct_mutex
    }

    pub const fn system_queues_ready(&self) -> bool {
        self.system_queues_ready
    }

    pub const fn system_queue_count_matches_linux_early(&self) -> bool {
        self.system_queue_count == EARLY_SYSTEM_WORKQUEUE_COUNT
    }

    pub const fn worker_pools_prepared(&self) -> bool {
        self.worker_pools_prepared
    }

    pub const fn cpu_worker_pools_ready(&self) -> bool {
        self.cpu_worker_pools_ready
    }

    pub const fn unbound_cpumask_ready(&self) -> bool {
        self.unbound_cpumask_ready
    }

    pub const fn bh_pools_ready(&self) -> bool {
        self.bh_pools_ready
    }

    pub const fn pool_workqueue_cache_ready(&self) -> bool {
        self.pool_workqueue_cache_ready
    }

    pub const fn pool_workqueue_cache_object_size(&self) -> usize {
        self.pool_workqueue_cache_object_size
    }

    pub const fn registered_in_slub_registry(&self) -> bool {
        self.registered_in_slub_registry
    }

    pub const fn attrs_ready(&self) -> bool {
        self.attrs_ready
    }

    pub const fn system_affinity_pods_ready(&self) -> bool {
        self.system_affinity_pods_ready
    }

    pub const fn pool_attach_mutex_deferred(&self) -> bool {
        self.pool_attach_mutex_deferred
    }

    pub const fn mayday_lock_deferred(&self) -> bool {
        self.mayday_lock_deferred
    }

    pub const fn manager_wait_deferred(&self) -> bool {
        self.manager_wait_deferred
    }

    pub const fn pool_mutex_guard_used(&self) -> bool {
        self.pool_mutex_guard_used
    }

    pub const fn struct_mutex_guard_used(&self) -> bool {
        self.struct_mutex_guard_used
    }

    pub const fn pre_smp_pool_mutex_guard_used(&self) -> bool {
        self.pre_smp_pool_mutex_guard_used
    }

    pub const fn topology_pool_mutex_guard_used(&self) -> bool {
        self.topology_pool_mutex_guard_used
    }

    pub const fn topology_struct_mutex_guard_used(&self) -> bool {
        self.topology_struct_mutex_guard_used
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

    pub const fn topology_ready(&self) -> bool {
        self.topology_ready
    }

    pub const fn pod_types_ready(&self) -> bool {
        self.pod_types_ready
    }

    pub const fn unbound_pools_rebound(&self) -> bool {
        self.unbound_pools_rebound
    }

    pub const fn max_active_topology_ready(&self) -> bool {
        self.max_active_topology_ready
    }

    pub const fn possible_cpu_count(&self) -> usize {
        self.possible_cpu_count
    }

    pub fn preset(
        &mut self,
        page_allocator: &PageAllocator,
        slub_subsystem: &mut SlubSubsystem,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
        init_task: &InitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || page_allocator.state() != State::Ready
            || slub_subsystem.state() != State::Ready
            || cpu_group.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || init_task.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.pool_mutex.preset_static()?;
        self.pool_mutex.setup()?;
        self.struct_mutex.preset_static()?;
        self.struct_mutex.setup()?;
        let Some(cache) = slub_subsystem.register_named_cache(
            NamedSlubCacheKind::PoolWorkqueue,
            POOL_WORKQUEUE_CACHE_OBJECT_SIZE,
            0,
            0,
        ) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        if cache.kind() != NamedSlubCacheKind::PoolWorkqueue
            || cache.object_size() != POOL_WORKQUEUE_CACHE_OBJECT_SIZE
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.pool_mutex.lock_boot_init_task(init_task)?;
        self.pool_mutex_guard_used = true;
        self.struct_mutex.lock_boot_init_task(init_task)?;
        self.struct_mutex_guard_used = true;
        self.struct_mutex.unlock_boot_init_task(init_task)?;
        self.pool_mutex.unlock_boot_init_task(init_task)?;

        self.possible_cpu_count = cpu_group.possible_cpu_count();
        self.system_queues_ready = true;
        self.system_queue_count = EARLY_SYSTEM_WORKQUEUE_COUNT;
        self.worker_pools_prepared = true;
        self.cpu_worker_pools_ready = true;
        self.unbound_cpumask_ready = true;
        self.bh_pools_ready = true;
        self.pool_workqueue_cache_ready = true;
        self.pool_workqueue_cache_object_size = POOL_WORKQUEUE_CACHE_OBJECT_SIZE;
        self.registered_in_slub_registry = true;
        self.attrs_ready = true;
        self.system_affinity_pods_ready = true;
        self.pool_attach_mutex_deferred = true;
        self.mayday_lock_deferred = true;
        self.manager_wait_deferred = true;
        self.workers_running = false;
        self.pre_smp_pool_mutex_guard_used = false;
        self.topology_pool_mutex_guard_used = false;
        self.topology_struct_mutex_guard_used = false;
        self.worker_creation_open = false;
        self.rescuers_ready = false;
        self.initial_workers_created = false;
        self.watchdog_ready = false;
        self.smp_topology_deferred = true;
        self.topology_ready = false;
        self.pod_types_ready = false;
        self.unbound_pools_rebound = false;
        self.max_active_topology_ready = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::WorkqueuePrepared,
        )
    }

    pub fn setup(
        &mut self,
        page_allocator: &PageAllocator,
        cpu_group: &CpuGroup,
        kernel_init_task: &KernelInitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || page_allocator.state() != State::Ready
            || !page_allocator.full_gfp_mask_open()
            || cpu_group.state() != State::Ready
            || !cpu_group.pre_smp_topology_ready()
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.released_for_pre_smp_init()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        match self.pool_mutex.lock_owner(MutexOwner::KernelInitTask)? {
            MutexLockOutcome::Acquired => {}
            MutexLockOutcome::Blocked => {
                return failed_condition(
                    LifecycleEvent::Setup,
                    self.lifecycle.state(),
                    State::Prepared,
                    State::Ready,
                );
            }
        }
        self.rescuers_ready = true;
        self.initial_workers_created = true;
        self.pool_mutex.unlock_owner(MutexOwner::KernelInitTask)?;
        self.pre_smp_pool_mutex_guard_used = true;
        self.topology_pool_mutex_guard_used = false;
        self.topology_struct_mutex_guard_used = false;
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

    pub fn setup_topology(&mut self, scheduler: &Scheduler, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.worker_creation_open
            || scheduler.state() != State::Online
            || !scheduler.smp_initialized()
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_online()
            || !cpu_group.smp_concurrency_open()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        if self.pool_mutex.lock_owner(MutexOwner::KernelInitTask)? != MutexLockOutcome::Acquired {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        if self.struct_mutex.lock_owner(MutexOwner::KernelInitTask)? != MutexLockOutcome::Acquired {
            self.pool_mutex.unlock_owner(MutexOwner::KernelInitTask)?;
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.topology_ready = true;
        self.pod_types_ready = true;
        self.unbound_pools_rebound = true;
        self.max_active_topology_ready = true;
        self.smp_topology_deferred = false;
        self.workers_running = false;
        self.struct_mutex.unlock_owner(MutexOwner::KernelInitTask)?;
        self.pool_mutex.unlock_owner(MutexOwner::KernelInitTask)?;
        self.topology_pool_mutex_guard_used = true;
        self.topology_struct_mutex_guard_used = true;
        crate::checkpoint::checkpoint(Checkpoint::WorkqueueTopologyReady);
        Ok(())
    }
}
