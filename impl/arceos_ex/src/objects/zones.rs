use super::{
    memblock::MemBlock,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

pub struct Zones {
    lifecycle: Lifecycle,
}

impl Zones {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, memblock: &MemBlock, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
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
            Checkpoint::ZonesReady,
        )
    }
}
