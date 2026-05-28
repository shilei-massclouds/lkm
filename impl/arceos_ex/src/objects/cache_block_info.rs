use super::{
    device_tree::DeviceTree,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct CacheBlockInfo {
    lifecycle: Lifecycle,
}

impl CacheBlockInfo {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, device_tree: &DeviceTree) -> EventResult {
        if self.lifecycle.state() != State::Base || device_tree.state() != State::Ready {
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
            Checkpoint::CacheBlockInfoReady,
        )
    }
}
