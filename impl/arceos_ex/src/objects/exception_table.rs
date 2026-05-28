use super::{
    kernel_image::KernelImage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

pub struct ExceptionTable {
    lifecycle: Lifecycle,
}

impl ExceptionTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, kernel_image: &KernelImage, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_image.state() != State::Online
            || vm.state() != State::Online
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
            Checkpoint::ExceptionTableReady,
        )
    }
}
