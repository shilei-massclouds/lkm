use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    config::Config,
    entry_prelude::{KernelImage, Lds},
    entry_successor::MemBlock,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
};

pub struct SwapperVm {
    lifecycle: Lifecycle,
}

impl SwapperVm {
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
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        memblock: &MemBlock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || config.state() != State::Online
            || static_objects.state() != State::Online
            || !static_objects.storage_ready(config)
            || lds.state() != State::Online
            || kernel_image.state() != State::Online
            || memblock.state() != State::Ready
        {
            return EventResult::failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !static_objects.build_swapper_pg_dir(
            config,
            lds.kernel_start(),
            lds.kernel_end(),
            memblock,
        ) {
            return EventResult::failed_condition(
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
            Checkpoint::SwapperVmReady,
        )
    }

    pub fn enable(&mut self, config: &Config, static_objects: &StaticObjects) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let Some(satp) = static_objects.swapper_satp(config) else {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        csr::write_satp(satp);
        csr::sfence_vma();

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::SwapperVmOnline,
        )
    }
}
