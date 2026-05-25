use crate::trace::Checkpoint;

use super::{
    boot_args::BootArgs,
    config::Config,
    early_vm::EarlyVm,
    entry_prelude::{KernelImage, Lds},
    fix_map::FixMap,
    raw_dtb::RawDtb,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    trampoline_vm::TrampolineVm,
};

pub struct Vm {
    lifecycle: Lifecycle,
    trampoline_vm: TrampolineVm,
    early_vm: EarlyVm,
}

impl Vm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trampoline_vm: TrampolineVm::new(),
            early_vm: EarlyVm::new(),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn preset(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        boot_args: &BootArgs,
        raw_dtb: &mut RawDtb,
        fix_map: &mut FixMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.trampoline_vm.state() != State::Base
            || self.early_vm.state() != State::Base
        {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let result = self.trampoline_vm.setup(config, static_objects, lds);
        if !result.is_success() {
            return result;
        }

        let result = self.early_vm.preset(config, boot_args, raw_dtb, fix_map);
        if !result.is_success() {
            return result;
        }

        let result =
            self.early_vm
                .setup(config, static_objects, lds, kernel_image, raw_dtb, fix_map);
        if !result.is_success() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::VmPrepared,
        )
    }
}
