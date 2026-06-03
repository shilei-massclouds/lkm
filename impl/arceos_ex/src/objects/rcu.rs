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
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn tasks_rcu(&self) -> &TasksRcu {
        &self.tasks_rcu
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
}

impl TasksRcu {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            callback_lists_ready: false,
            enabled_flavor_count: 0,
            gp_threads_deferred: true,
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
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TasksRcuPrepared,
        )
    }
}
