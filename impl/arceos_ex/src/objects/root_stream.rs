use crate::arch::riscv64::csr;

use super::state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition};

pub struct RootStream {
    lifecycle: Lifecycle,
}

impl RootStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self) -> EventResult {
        if !csr::kernel_fpu_vector_disabled() {
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
}
