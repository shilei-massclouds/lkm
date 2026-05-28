use super::{
    memblock::MemBlock,
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

pub struct DeviceTree {
    lifecycle: Lifecycle,
}

impl DeviceTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, raw_dtb: &RawDtb, vm: &Vm, memblock: &MemBlock) -> EventResult {
        if self.lifecycle.state() != State::Base
            || raw_dtb.state() != State::Ready
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || memblock.state() != State::Online
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
            Checkpoint::DeviceTreeReady,
        )
    }
}
