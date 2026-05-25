use crate::trace::Checkpoint;

use super::{
    config::Config,
    entry_prelude::Lds,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
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

    pub fn setup(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || !config.entry_prelude_ready()
            || static_objects.state() != State::Online
            || !static_objects.storage_ready(config)
            || lds.state() != State::Online
        {
            return EventResult::failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !static_objects.build_trampoline_pg_dir(config, lds.kernel_start()) {
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
            Checkpoint::TrampolineVmReady,
        )
    }
}
