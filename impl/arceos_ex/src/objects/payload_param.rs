use super::{
    boot_param::BootParam,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct PayloadParam {
    lifecycle: Lifecycle,
}

impl PayloadParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, boot_param: &BootParam) -> EventResult {
        if self.lifecycle.state() != State::Base || boot_param.state() != State::Ready {
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
            Checkpoint::PayloadParamReady,
        )
    }
}
