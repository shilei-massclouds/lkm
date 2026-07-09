use super::{
    fdt::{BootCommandLine, FdtFacts},
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::checkpoint::Checkpoint;

pub struct KernelCmdline {
    lifecycle: Lifecycle,
    cmdline: BootCommandLine,
}

impl KernelCmdline {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cmdline: BootCommandLine::empty(),
        }
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cmdline(&self) -> BootCommandLine {
        self.cmdline
    }

    pub fn has_earlycon_sbi(&self) -> bool {
        self.cmdline.contains(b"earlycon=sbi")
    }

    pub fn preset(&mut self, raw_dtb: &RawDtb, facts: &FdtFacts) -> EventResult {
        if raw_dtb.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.cmdline = facts.cmdline;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::KernelCmdlineReady,
        )
    }
}
