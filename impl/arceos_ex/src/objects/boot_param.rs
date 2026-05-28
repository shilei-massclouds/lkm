use super::{
    command_line::StaticCommandLine,
    early_param::EarlyParam,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct BootParam {
    lifecycle: Lifecycle,
}

impl BootParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(
        &mut self,
        early_param: &EarlyParam,
        static_command_line: &StaticCommandLine,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || early_param.state() != State::Ready
            || static_command_line.state() != State::Ready
        {
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
            Checkpoint::BootParamReady,
        )
    }
}
