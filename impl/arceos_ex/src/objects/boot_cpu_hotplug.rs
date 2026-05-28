use super::{
    cpu_group::CpuGroup,
    per_cpu_storage::PerCpuStorage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct BootCpuHotplugState {
    lifecycle: Lifecycle,
}

impl BootCpuHotplugState {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.boot_cpu_state() != State::Online
            || per_cpu_storage.state() != State::Ready
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
            Checkpoint::BootCpuHotplugStateReady,
        )
    }
}
