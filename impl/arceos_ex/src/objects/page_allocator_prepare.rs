use super::{
    memblock::MemBlock,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
    zones::Zones,
};
use crate::trace::Checkpoint;

pub struct PageAllocatorPrepare {
    lifecycle: Lifecycle,
}

impl PageAllocatorPrepare {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, memblock: &MemBlock, vm: &Vm, zones: &Zones) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || zones.state() != State::Ready
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
            Checkpoint::PageAllocatorPrepareReady,
        )
    }
}
