use super::{
    cpu_hotplug::CpuHotplugState,
    mm_core::{NamedSlubCacheKind, SlubSubsystem},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

const RADIX_TREE_CPUHP_STEP: usize = 0x300;
const RADIX_TREE_NODE_CACHE_OBJECT_SIZE: usize = core::mem::size_of::<usize>() * 8;

pub struct RadixTree {
    lifecycle: Lifecycle,
    node_cache_ready: bool,
    registered_in_slub_registry: bool,
    node_cache_object_size: usize,
    cpuhp_step: usize,
    node_api_ready: bool,
    node_rcu_free_callback_deferred: bool,
}

impl RadixTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            node_cache_ready: false,
            registered_in_slub_registry: false,
            node_cache_object_size: 0,
            cpuhp_step: 0,
            node_api_ready: false,
            node_rcu_free_callback_deferred: false,
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

    pub const fn cpuhp_step(&self) -> usize {
        self.cpuhp_step
    }

    pub const fn node_api_ready(&self) -> bool {
        self.node_api_ready
    }

    pub const fn node_rcu_free_callback_deferred(&self) -> bool {
        self.node_rcu_free_callback_deferred
    }

    pub fn setup(
        &mut self,
        slub_subsystem: &mut SlubSubsystem,
        cpu_hotplug_state: &CpuHotplugState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || cpu_hotplug_state.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let Some(cache) = slub_subsystem.register_named_cache(
            NamedSlubCacheKind::RadixTreeNode,
            RADIX_TREE_NODE_CACHE_OBJECT_SIZE,
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
        if cache.kind() != NamedSlubCacheKind::RadixTreeNode
            || cache.object_size() != RADIX_TREE_NODE_CACHE_OBJECT_SIZE
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
        self.node_cache_object_size = RADIX_TREE_NODE_CACHE_OBJECT_SIZE;
        self.cpuhp_step = RADIX_TREE_CPUHP_STEP;
        self.node_api_ready = true;
        self.node_rcu_free_callback_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RadixTreeReady,
        )
    }
}
