use crate::trace::Checkpoint;

use super::{
    config::Config,
    entry_prelude::Lds,
    kernel_image::KernelImage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
};

pub struct TrampolineVm {
    lifecycle: Lifecycle,
}

impl TrampolineVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn enable(&mut self, kernel_image: &KernelImage) -> EventResult {
        if self.lifecycle.state() != State::Ready || kernel_image.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::TrampolineVmOnline,
        )
    }

    pub fn cleanup(&mut self, early_vm: &super::early_vm::EarlyVm) -> EventResult {
        if self.lifecycle.state() != State::Online || early_vm.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.lifecycle.state(),
                State::Online,
                State::Destroyed,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Cleanup,
            State::Online,
            State::Destroyed,
            Checkpoint::TrampolineVmDestroyed,
        )
    }

    pub fn setup(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || !config.entry_prelude_ready()
            || static_objects.state() != State::Online
            || !static_objects.storage_ready(config)
            || lds.state() != State::Online
            || kernel_image.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !static_objects.build_trampoline_pg_dir(config, kernel_image) {
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
            Checkpoint::TrampolineVmReady,
        )
    }
}
