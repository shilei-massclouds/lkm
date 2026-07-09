use super::state::{EventResult, Lifecycle, LifecycleEvent, State};
use crate::checkpoint::Checkpoint;

pub struct Sbi {
    lifecycle: Lifecycle,
    hsm_available: bool,
}

impl Sbi {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hsm_available: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn hsm_available(&self) -> bool {
        self.hsm_available
    }

    pub fn setup(&mut self) -> EventResult {
        self.hsm_available = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SbiReady,
        )
    }
}
