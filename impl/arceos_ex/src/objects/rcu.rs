use super::{
    cpu_group::CpuGroup,
    per_cpu_storage::PerCpuStorage,
    scheduler::Scheduler,
    softirq::Softirq,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

pub struct RcuCore {
    lifecycle: Lifecycle,
    tasks_rcu: TasksRcu,
    boot_cpu_online_ready: bool,
    softirq_registered: bool,
    workqueues_ready: bool,
    gp_threads_deferred: bool,
    scheduler_starting_ready: bool,
    scheduler_active_init: bool,
    scheduler_start_single_online_cpu: bool,
    gp_seq_baseline_synced: bool,
    inkernel_boot_ended: bool,
}

impl RcuCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            tasks_rcu: TasksRcu::new(),
            boot_cpu_online_ready: false,
            softirq_registered: false,
            workqueues_ready: false,
            gp_threads_deferred: true,
            scheduler_starting_ready: false,
            scheduler_active_init: false,
            scheduler_start_single_online_cpu: false,
            gp_seq_baseline_synced: false,
            inkernel_boot_ended: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn tasks_rcu(&self) -> &TasksRcu {
        &self.tasks_rcu
    }

    pub fn tasks_rcu_mut(&mut self) -> &mut TasksRcu {
        &mut self.tasks_rcu
    }

    pub const fn boot_cpu_online_ready(&self) -> bool {
        self.boot_cpu_online_ready
    }

    pub const fn softirq_registered(&self) -> bool {
        self.softirq_registered
    }

    pub const fn workqueues_ready(&self) -> bool {
        self.workqueues_ready
    }

    pub const fn gp_threads_deferred(&self) -> bool {
        self.gp_threads_deferred
    }

    pub const fn scheduler_starting_ready(&self) -> bool {
        self.scheduler_starting_ready
    }

    pub const fn scheduler_active_init(&self) -> bool {
        self.scheduler_active_init
    }

    pub const fn scheduler_start_single_online_cpu(&self) -> bool {
        self.scheduler_start_single_online_cpu
    }

    pub const fn gp_seq_baseline_synced(&self) -> bool {
        self.gp_seq_baseline_synced
    }

    pub const fn inkernel_boot_ended(&self) -> bool {
        self.inkernel_boot_ended
    }

    pub fn setup(
        &mut self,
        scheduler: &Scheduler,
        workqueue: &Workqueue,
        softirq: &Softirq,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || workqueue.state() != State::Prepared
            || softirq.state() != State::Prepared
            || cpu_group.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.tasks_rcu.preset(per_cpu_storage)?;
        self.boot_cpu_online_ready = cpu_group.boot_cpu_state() == State::Online;
        self.softirq_registered = softirq.action_table_ready();
        self.workqueues_ready = workqueue.system_queues_ready();
        self.gp_threads_deferred = true;
        self.scheduler_starting_ready = false;
        self.scheduler_active_init = false;
        self.scheduler_start_single_online_cpu = false;
        self.gp_seq_baseline_synced = false;
        self.inkernel_boot_ended = false;
        if !self.boot_cpu_online_ready || !self.softirq_registered || !self.workqueues_ready {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RcuCoreReady,
        )
    }

    pub fn scheduler_start(&mut self, scheduler: &Scheduler, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || scheduler.state() != State::Online
            || cpu_group.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.scheduler_start_single_online_cpu = cpu_group.boot_cpu_state() == State::Online;
        if !self.scheduler_start_single_online_cpu || !self.gp_threads_deferred {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.scheduler_starting_ready = true;
        self.scheduler_active_init = true;
        self.gp_seq_baseline_synced = true;
        crate::trace::checkpoint(Checkpoint::RcuSchedulerStartingReady);
        Ok(())
    }

    pub fn end_inkernel_boot(&mut self) -> bool {
        if self.lifecycle.state() != State::Ready || self.inkernel_boot_ended {
            return false;
        }

        self.inkernel_boot_ended = true;
        crate::trace::checkpoint(Checkpoint::RcuInkernelBootEnded);
        true
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

pub struct TasksRcu {
    lifecycle: Lifecycle,
    callback_lists_ready: bool,
    enabled_flavor_count: usize,
    gp_threads_deferred: bool,
    gp_threads_ready: bool,
}

impl TasksRcu {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            callback_lists_ready: false,
            enabled_flavor_count: 0,
            gp_threads_deferred: true,
            gp_threads_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn callback_lists_ready(&self) -> bool {
        self.callback_lists_ready
    }

    pub const fn enabled_flavor_count(&self) -> usize {
        self.enabled_flavor_count
    }

    pub const fn gp_threads_deferred(&self) -> bool {
        self.gp_threads_deferred
    }

    pub const fn gp_threads_ready(&self) -> bool {
        self.gp_threads_ready
    }

    fn preset(&mut self, per_cpu_storage: &PerCpuStorage) -> EventResult {
        if self.lifecycle.state() != State::Base || per_cpu_storage.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.callback_lists_ready = true;
        self.enabled_flavor_count = 2;
        self.gp_threads_deferred = true;
        self.gp_threads_ready = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TasksRcuPrepared,
        )
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.callback_lists_ready
            || self.enabled_flavor_count == 0
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.gp_threads_deferred = false;
        self.gp_threads_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::TasksRcuReady,
        )
    }
}
