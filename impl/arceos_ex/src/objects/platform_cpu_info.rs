use super::{
    fdt::{FdtFacts, HartSet},
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct PlatformCpuInfo {
    lifecycle: Lifecycle,
    harts: HartSet,
}

impl PlatformCpuInfo {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            harts: HartSet::empty(),
        }
    }

    pub fn contains(&self, hartid: usize) -> bool {
        self.lifecycle.state() == State::Online && self.harts.contains(hartid)
    }

    pub fn preset(
        &mut self,
        raw_dtb: &RawDtb,
        facts: &FdtFacts,
        boot_hartid: usize,
    ) -> EventResult {
        if raw_dtb.state() != State::Ready
            || facts.harts.count() == 0
            || !facts.harts.contains(boot_hartid)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.harts = facts.harts;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::PlatformCpuInfoReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::PlatformCpuInfoOnline,
        )
    }
}
