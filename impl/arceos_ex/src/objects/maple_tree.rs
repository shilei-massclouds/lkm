use super::{
    mm_core::{NamedSlubCacheKind, SlubSubsystem},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

const MAPLE_NODE_CACHE_OBJECT_SIZE: usize = core::mem::size_of::<usize>() * 16;

pub struct MapleTree {
    lifecycle: Lifecycle,
    node_cache_ready: bool,
    registered_in_slub_registry: bool,
    node_cache_object_size: usize,
    node_api_ready: bool,
}

impl MapleTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            node_cache_ready: false,
            registered_in_slub_registry: false,
            node_cache_object_size: 0,
            node_api_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn node_cache_ready(&self) -> bool {
        self.node_cache_ready
    }

    pub const fn registered_in_slub_registry(&self) -> bool {
        self.registered_in_slub_registry
    }

    pub const fn node_cache_object_size(&self) -> usize {
        self.node_cache_object_size
    }

    pub const fn node_api_ready(&self) -> bool {
        self.node_api_ready
    }

    pub fn setup(&mut self, slub_subsystem: &mut SlubSubsystem) -> EventResult {
        if self.lifecycle.state() != State::Base || slub_subsystem.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let Some(cache) = slub_subsystem.register_named_cache(
            NamedSlubCacheKind::MapleNode,
            MAPLE_NODE_CACHE_OBJECT_SIZE,
            0,
            0,
        ) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        if cache.kind() != NamedSlubCacheKind::MapleNode
            || cache.object_size() != MAPLE_NODE_CACHE_OBJECT_SIZE
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.node_cache_ready = true;
        self.registered_in_slub_registry = true;
        self.node_cache_object_size = MAPLE_NODE_CACHE_OBJECT_SIZE;
        self.node_api_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::MapleTreeReady,
        )
    }
}
