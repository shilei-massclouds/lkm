use super::{
    command_line::StaticCommandLine,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct Randomness {
    lifecycle: Lifecycle,
}

impl Randomness {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn preset(&mut self, static_command_line: &StaticCommandLine) -> EventResult {
        if self.lifecycle.state() != State::Base || static_command_line.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::RandomnessPrepared,
        )
    }
}
