use crate::trace::Checkpoint;

use super::{
    boot_args::BootArgs,
    config::Config,
    fix_map::FixMap,
    raw_dtb::RawDtb,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
};

pub struct EarlyVm {
    lifecycle: Lifecycle,
}

impl EarlyVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn preset(
        &mut self,
        config: &Config,
        boot_args: &BootArgs,
        raw_dtb: &mut RawDtb,
        fix_map: &mut FixMap,
    ) -> EventResult {
        if config.state() != State::Online
            || raw_dtb.state() != State::Base
            || fix_map.state() != State::Base
        {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let result = raw_dtb.preset(boot_args);
        if !result.is_success() {
            return result;
        }

        let result = raw_dtb.setup();
        if !result.is_success() {
            return result;
        }

        let result = fix_map.preset(config, raw_dtb);
        if !result.is_success() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EarlyVmPrepared,
        )
    }
}
