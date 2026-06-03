use super::{
    per_cpu_storage::PerCpuStorage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

const SOFTIRQ_SLOT_COUNT: usize = 10;

pub struct Softirq {
    lifecycle: Lifecycle,
    action_table_ready: bool,
    slot_count: usize,
    pending_set_ready: bool,
    execution_open: bool,
}

impl Softirq {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            action_table_ready: false,
            slot_count: 0,
            pending_set_ready: false,
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
}
