use super::{
    fix_map::FixMap,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct EarlyIoremap {
    lifecycle: Lifecycle,
}

impl EarlyIoremap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn setup(&mut self, fix_map: &FixMap) -> EventResult {
        if fix_map.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::EarlyIoremapReady,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}
