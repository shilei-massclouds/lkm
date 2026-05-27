use super::state::{EventResult, Lifecycle, LifecycleEvent, State};
use crate::trace::Checkpoint;

pub struct CpuIdMap {
    lifecycle: Lifecycle,
    boot_hartid: usize,
}

impl CpuIdMap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_hartid: usize::MAX,
        }
    }

    pub fn preset(&mut self, boot_hartid: usize) -> EventResult {
        self.boot_hartid = boot_hartid;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::CpuIdMapReady,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}
