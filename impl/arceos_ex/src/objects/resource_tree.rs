use super::{
    kernel_image::KernelImage,
    lds::Lds,
    memblock::MemBlock,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct ResourceTree {
    lifecycle: Lifecycle,
}

impl ResourceTree {
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
        memblock: &MemBlock,
        kernel_image: &KernelImage,
        lds: &Lds,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || kernel_image.state() != State::Online
            || lds.state() != State::Online
            || lds.kernel_end() <= lds.kernel_start()
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
            Checkpoint::ResourceTreeReady,
        )
    }
}
