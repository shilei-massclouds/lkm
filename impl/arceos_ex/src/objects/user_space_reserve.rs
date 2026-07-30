use super::{
    config::Config,
    kernel_addr_space::VirtRange,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

pub struct UserSpaceReserve {
    lifecycle: Lifecycle,
    range: VirtRange,
}

impl UserSpaceReserve {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            range: VirtRange::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn range(&self) -> VirtRange {
        self.range
    }

    pub fn preset(&mut self, config: &Config) -> EventResult {
        if self.lifecycle.state() != State::Base || config.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let range = VirtRange::new(0, config.canonical_user_virt_end());
        if !range.valid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.range = range;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Ready)
    }
}
