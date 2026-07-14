use super::{
    cpu_group::CpuGroup,
    mm_core::PageAllocator,
    rcu::RcuCore,
    rest_init::{KernelInitTask, KthreaddTask},
    scheduler::Scheduler,
    softirq::Softirq,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    workqueue::Workqueue,
};
use crate::checkpoint::Checkpoint;

pub struct VmstatCore {
    lifecycle: Lifecycle,
    mm_percpu_workqueue_ready: bool,
    cpuhp_state_registered: bool,
    shepherd_work_started: bool,
    proc_exports_deferred: bool,
}

impl VmstatCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            mm_percpu_workqueue_ready: false,
            cpuhp_state_registered: false,
            shepherd_work_started: false,
            proc_exports_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn mm_percpu_workqueue_ready(&self) -> bool {
        self.mm_percpu_workqueue_ready
    }

    pub const fn cpuhp_state_registered(&self) -> bool {
        self.cpuhp_state_registered
    }

    pub const fn shepherd_work_started(&self) -> bool {
        self.shepherd_work_started
    }

    pub const fn proc_exports_deferred(&self) -> bool {
        self.proc_exports_deferred
    }

    pub fn preset(
        &mut self,
        workqueue: &Workqueue,
        page_allocator: &PageAllocator,
        kernel_init_task: &KernelInitTask,
        scheduler: &Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || workqueue.state() != State::Ready
            || !workqueue.worker_creation_open()
            || page_allocator.state() != State::Ready
            || !page_allocator.full_gfp_mask_open()
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.released_for_pre_smp_init()
            || scheduler.schedule_passes() == 0
        {
            return self.failed_preset();
        }

        self.mm_percpu_workqueue_ready = true;
        self.cpuhp_state_registered = true;
        self.shepherd_work_started = true;
        self.proc_exports_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::VmstatCorePrepared,
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

pub struct PreSmpInitcallTable {
    lifecycle: Lifecycle,
    early_level_ran: bool,
    rcu_gp_kthread_ready: bool,
    softirq_ksoftirqd_ready: bool,
    scheduler_migration_ready: bool,
    cpu_stopper_prepared: bool,
    zero_page_bound: bool,
    address_space_id_ready: bool,
}

impl PreSmpInitcallTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            early_level_ran: false,
            rcu_gp_kthread_ready: false,
            softirq_ksoftirqd_ready: false,
            scheduler_migration_ready: false,
            cpu_stopper_prepared: false,
            zero_page_bound: false,
            address_space_id_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn early_level_ran(&self) -> bool {
        self.early_level_ran
    }

    pub const fn rcu_gp_kthread_ready(&self) -> bool {
        self.rcu_gp_kthread_ready
    }

    pub const fn softirq_ksoftirqd_ready(&self) -> bool {
        self.softirq_ksoftirqd_ready
    }

    pub const fn scheduler_migration_ready(&self) -> bool {
        self.scheduler_migration_ready
    }

    pub const fn cpu_stopper_prepared(&self) -> bool {
        self.cpu_stopper_prepared
    }

    pub const fn zero_page_bound(&self) -> bool {
        self.zero_page_bound
    }

    pub const fn address_space_id_ready(&self) -> bool {
        self.address_space_id_ready
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        rcu_core: &RcuCore,
        softirq: &Softirq,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.released_for_pre_smp_init()
            || rcu_core.state() != State::Ready
            || !rcu_core.tasks_rcu().gp_threads_ready()
            || softirq.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.schedule_passes() == 0
            || cpu_group.state() != State::Ready
            || !cpu_group.pre_smp_topology_ready()
        {
            return self.failed_setup();
        }

        self.early_level_ran = true;
        self.rcu_gp_kthread_ready = true;
        self.softirq_ksoftirqd_ready = true;
        self.scheduler_migration_ready = true;
        self.cpu_stopper_prepared = true;
        self.zero_page_bound = true;
        self.address_space_id_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PreSmpInitcallsReady,
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

pub struct PreSmpInitBoundary {
    lifecycle: Lifecycle,
    mems_allowed_trimmed: bool,
    cad_pid_deferred: bool,
    lockup_detector_deferred: bool,
    smp_init_not_called: bool,
    secondary_cpus_present_not_online: bool,
}

impl PreSmpInitBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            mems_allowed_trimmed: false,
            cad_pid_deferred: false,
            lockup_detector_deferred: false,
            smp_init_not_called: true,
            secondary_cpus_present_not_online: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn mems_allowed_trimmed(&self) -> bool {
        self.mems_allowed_trimmed
    }

    pub const fn cad_pid_deferred(&self) -> bool {
        self.cad_pid_deferred
    }

    pub const fn lockup_detector_deferred(&self) -> bool {
        self.lockup_detector_deferred
    }

    pub const fn smp_init_not_called(&self) -> bool {
        self.smp_init_not_called
    }

    pub const fn secondary_cpus_present_not_online(&self) -> bool {
        self.secondary_cpus_present_not_online
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        initcalls: &PreSmpInitcallTable,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.released_for_pre_smp_init()
            || initcalls.state() != State::Ready
            || scheduler.schedule_passes() == 0
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_present_not_online()
        {
            return self.failed_setup();
        }

        self.mems_allowed_trimmed = true;
        self.cad_pid_deferred = true;
        self.lockup_detector_deferred = true;
        self.smp_init_not_called = true;
        self.secondary_cpus_present_not_online = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PreSmpBoundaryReady,
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

pub fn pre_smp_runtime_ready(
    kernel_init_task: &KernelInitTask,
    kthreadd_task: &KthreaddTask,
    scheduler: &Scheduler,
    page_allocator: &PageAllocator,
    cpu_group: &CpuGroup,
    workqueue: &Workqueue,
    rcu_core: &RcuCore,
    vmstat_core: &VmstatCore,
    initcalls: &PreSmpInitcallTable,
    boundary: &PreSmpInitBoundary,
) -> bool {
    kernel_init_task.state() == State::Online
        && kernel_init_task.released_for_pre_smp_init()
        && kthreadd_task.state() == State::Online
        && scheduler.schedule_passes() != 0
        && page_allocator.state() == State::Ready
        && page_allocator.full_gfp_mask_open()
        && cpu_group.pre_smp_topology_ready()
        && cpu_group.secondary_cpus_present_not_online()
        && workqueue.state() == State::Ready
        && workqueue.worker_creation_open()
        && !workqueue.workers_running()
        && rcu_core.tasks_rcu().state() == State::Ready
        && rcu_core.tasks_rcu().gp_threads_ready()
        && vmstat_core.state() == State::Prepared
        && initcalls.state() == State::Ready
        && boundary.state() == State::Ready
        && boundary.smp_init_not_called()
}
