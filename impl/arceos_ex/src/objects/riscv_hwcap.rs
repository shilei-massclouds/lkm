use super::{
    cache_block_info::CacheBlockInfo,
    cpu_group::CpuGroup,
    device_tree::DeviceTree,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct RiscvHwCap {
    lifecycle: Lifecycle,
}

impl RiscvHwCap {
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
        device_tree: &DeviceTree,
        cpu_group: &CpuGroup,
        cache_block_info: &CacheBlockInfo,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || cpu_group.state() != State::Ready
            || cache_block_info.state() != State::Ready
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
            Checkpoint::RiscvHwCapReady,
        )
    }
}
