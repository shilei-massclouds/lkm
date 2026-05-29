use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State};

pub struct InterruptStream {
    lifecycle: Lifecycle,
}

impl InterruptStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self) -> EventResult {
        if csr::read_sie() != 0 || csr::read_sip() != 0 {
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

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared {
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
            Checkpoint::InterruptStreamReady,
        )
    }
}

pub fn dispatch_scause(_scause: usize) -> ! {
    crate::arch::riscv64::sbi::putstr("interrupt stream not enabled\n");
    crate::arch::riscv64::sbi::system_shutdown()
}
