use crate::trace::Checkpoint;

use super::{
    entry_prelude::Lds,
    kernel_image::KernelImage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};

pub const PT_SIZE_ON_STACK: usize = 256;

unsafe extern "C" {
    static head_init_stack_sp: usize;
}

pub struct InitStack {
    lifecycle: Lifecycle,
}

impl InitStack {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self, kernel_image: &KernelImage, lds: &Lds) -> EventResult {
        let head_sp = unsafe { core::ptr::addr_of!(head_init_stack_sp).read_volatile() };
        let Some(stack_phys) = lds.init_stack_end_phys(kernel_image).and_then(|stack_end| {
            stack_end
                .checked_sub(PT_SIZE_ON_STACK)
                .filter(|stack| *stack == head_sp)
        }) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        if stack_phys < lds.init_stack_start() || stack_phys >= lds.init_stack_end() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InitStackReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::InitStackOnline,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}
