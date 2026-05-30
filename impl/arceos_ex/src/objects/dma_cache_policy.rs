use super::{
    cache_block_info::CacheBlockInfo,
    cpu_capabilities::CpuCapabilities,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::trace::Checkpoint;

pub struct DmaCachePolicy {
    lifecycle: Lifecycle,
    noncoherent_supported: bool,
    cache_alignment: usize,
}

impl DmaCachePolicy {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            noncoherent_supported: false,
            cache_alignment: 1,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn noncoherent_supported(&self) -> bool {
        self.noncoherent_supported
    }

    pub const fn cache_alignment(&self) -> usize {
        self.cache_alignment
    }

    pub fn setup(
        &mut self,
        cpu_capabilities: &CpuCapabilities,
        cache_block_info: &CacheBlockInfo,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_capabilities.state() != State::Ready
            || cache_block_info.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.noncoherent_supported = cpu_capabilities.common_isa().zicbom;
        self.cache_alignment = if self.noncoherent_supported {
            cache_block_info
                .cbom_block_size()
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value != 0 && value.is_power_of_two())
                .unwrap_or(1)
        } else {
            1
        };

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DmaCachePolicyReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}
