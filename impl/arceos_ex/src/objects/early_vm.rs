use crate::checkpoint::Checkpoint;

use super::{
    boot_args::BootArgs,
    config::Config,
    fix_map::FixMap,
    kernel_image::KernelImage,
    lds::Lds,
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
};

pub struct EarlyVm {
    lifecycle: Lifecycle,
    translation_sync_complete: bool,
}

impl EarlyVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            translation_sync_complete: false,
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn translation_sync_complete(&self) -> bool {
        self.translation_sync_complete
    }

    pub fn enable(&mut self, trampoline_vm: &super::trampoline_vm::TrampolineVm) -> EventResult {
        if self.lifecycle.state() != State::Ready || trampoline_vm.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.translation_sync_complete = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::EarlyVmOnline,
        )
    }

    pub fn cleanup(&mut self, swapper_vm: &super::swapper_vm::SwapperVm) -> EventResult {
        if self.lifecycle.state() != State::Online || swapper_vm.state() != State::Online {
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
            Checkpoint::EarlyVmDestroyed,
        )
    }

    pub fn preset(
        &mut self,
        config: &Config,
        boot_args: &BootArgs,
        raw_dtb: &mut RawDtb,
        fix_map: &mut FixMap,
    ) -> EventResult {
        if !config.entry_prelude_ready()
            || raw_dtb.state() != State::Base
            || fix_map.state() != State::Base
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let result = raw_dtb.preset(boot_args);
        if result.is_err() {
            return result;
        }

        let result = raw_dtb.setup();
        if result.is_err() {
            return result;
        }

        let result = fix_map.preset(config, raw_dtb);
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EarlyVmPrepared,
        )
    }

    pub fn setup(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        raw_dtb: &RawDtb,
        fix_map: &FixMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !config.entry_prelude_ready()
            || static_objects.state() != State::Online
            || !static_objects.storage_ready(config)
            || kernel_image.state() != State::Ready
            || raw_dtb.state() != State::Ready
            || fix_map.state() != State::Ready
            || !fix_map.contains_raw_dtb(raw_dtb)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        if !static_objects.build_early_pg_dir(
            config,
            kernel_image,
            lds.kernel_start(),
            lds.kernel_end(),
            raw_dtb,
            fix_map,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::EarlyVmReady,
        )
    }
}
