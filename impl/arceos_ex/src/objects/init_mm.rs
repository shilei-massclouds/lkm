use super::{
    lds::Lds,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::checkpoint::Checkpoint;

pub struct InitMm {
    lifecycle: Lifecycle,
    kernel_start: usize,
    kernel_end: usize,
}

impl InitMm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kernel_start: 0,
            kernel_end: 0,
        }
    }

    pub fn setup(&mut self, lds: &Lds) -> EventResult {
        if lds.state() != State::Online || lds.kernel_start() >= lds.kernel_end() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.kernel_start = lds.kernel_start();
        self.kernel_end = lds.kernel_end();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::InitMmReady,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}
