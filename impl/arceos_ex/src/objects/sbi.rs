use super::state::{EventResult, Lifecycle, LifecycleEvent, State};
use crate::trace::Checkpoint;

pub struct Sbi {
    lifecycle: Lifecycle,
}

impl Sbi {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SbiReady,
        )
    }
}
