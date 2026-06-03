use super::{
    cpu_hotplug::CpuHotplugState,
    mm_core::SlubAllocator,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

const RADIX_TREE_CPUHP_STEP: usize = 0x300;

pub struct RadixTree {
    lifecycle: Lifecycle,
    node_cache_ready: bool,
    cpuhp_step: usize,
    node_api_ready: bool,
}

impl RadixTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            node_cache_ready: false,
            cpuhp_step: 0,
            node_api_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn node_cache_ready(&self) -> bool {
        self.node_cache_ready
    }

    pub const fn cpuhp_step(&self) -> usize {
        self.cpuhp_step
    }

    pub const fn node_api_ready(&self) -> bool {
        self.node_api_ready
    }

    pub fn setup(
        &mut self,
        slub_allocator: &SlubAllocator,
        cpu_hotplug_state: &CpuHotplugState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_allocator.state() != State::Ready
            || cpu_hotplug_state.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.node_cache_ready = true;
        self.cpuhp_step = RADIX_TREE_CPUHP_STEP;
        self.node_api_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RadixTreeReady,
        )
    }
}
