use super::{
    fdt::{FdtFacts, PhysRangeSet},
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct PhysicalMemory {
    lifecycle: Lifecycle,
    ram: PhysRangeSet,
}

impl PhysicalMemory {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ram: PhysRangeSet::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ram(&self) -> &PhysRangeSet {
        &self.ram
    }

    pub fn preset(&mut self, raw_dtb: &RawDtb, facts: &FdtFacts) -> EventResult {
        if raw_dtb.state() != State::Ready || facts.memory.count() == 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.ram = facts.memory;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::PhysicalMemoryReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::PhysicalMemoryOnline,
        )
    }
}
