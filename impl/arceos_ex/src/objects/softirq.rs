use super::{
    per_cpu_storage::PerCpuStorage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

const SOFTIRQ_SLOT_COUNT: usize = 10;

pub struct Softirq {
    lifecycle: Lifecycle,
    action_table_ready: bool,
    slot_count: usize,
    pending_set_ready: bool,
    rcu_action_registered: bool,
    timer_action_registered: bool,
    hrtimer_action_registered: bool,
    tasklet_queues_ready: bool,
    tasklet_actions_registered: bool,
    execution_open: bool,
}

impl Softirq {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            action_table_ready: false,
            slot_count: 0,
            pending_set_ready: false,
            rcu_action_registered: false,
            timer_action_registered: false,
            hrtimer_action_registered: false,
            tasklet_queues_ready: false,
            tasklet_actions_registered: false,
            execution_open: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn action_table_ready(&self) -> bool {
        self.action_table_ready
    }

    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    pub const fn pending_set_ready(&self) -> bool {
        self.pending_set_ready
    }

    pub const fn rcu_action_registered(&self) -> bool {
        self.rcu_action_registered
    }

    pub const fn timer_action_registered(&self) -> bool {
        self.timer_action_registered
    }

    pub const fn hrtimer_action_registered(&self) -> bool {
        self.hrtimer_action_registered
    }

    pub const fn tasklet_queues_ready(&self) -> bool {
        self.tasklet_queues_ready
    }

    pub const fn tasklet_actions_registered(&self) -> bool {
        self.tasklet_actions_registered
    }

    pub const fn execution_open(&self) -> bool {
        self.execution_open
    }

    pub fn preset(&mut self, per_cpu_storage: &PerCpuStorage) -> EventResult {
        if self.lifecycle.state() != State::Base || per_cpu_storage.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.action_table_ready = true;
        self.slot_count = SOFTIRQ_SLOT_COUNT;
        self.pending_set_ready = true;
        self.execution_open = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SoftirqPrepared,
        )
    }

    pub fn register_timer_action(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.action_table_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.timer_action_registered = true;
        Ok(())
    }

    pub fn register_rcu_action(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.action_table_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.rcu_action_registered = true;
        Ok(())
    }

    pub fn register_hrtimer_action(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.action_table_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.hrtimer_action_registered = true;
        Ok(())
    }

    pub fn setup(&mut self, per_cpu_storage: &PerCpuStorage) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || per_cpu_storage.state() != State::Ready
            || !self.timer_action_registered
            || !self.hrtimer_action_registered
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.tasklet_queues_ready = true;
        self.tasklet_actions_registered = true;
        self.execution_open = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SoftirqReady,
        )
    }
}
