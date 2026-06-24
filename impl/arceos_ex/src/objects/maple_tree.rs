use super::{
    mm_core::SlubSubsystem,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct MapleTree {
    lifecycle: Lifecycle,
    node_cache_ready: bool,
    node_api_ready: bool,
}

impl MapleTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            node_cache_ready: false,
            node_api_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn node_cache_ready(&self) -> bool {
        self.node_cache_ready
    }

    pub const fn node_api_ready(&self) -> bool {
        self.node_api_ready
    }

    pub fn setup(&mut self, slub_subsystem: &SlubSubsystem) -> EventResult {
        if self.lifecycle.state() != State::Base || slub_subsystem.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.node_cache_ready = true;
        self.node_api_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::MapleTreeReady,
        )
    }
}
