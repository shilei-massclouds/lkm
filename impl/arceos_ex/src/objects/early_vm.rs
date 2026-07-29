use core::sync::atomic::{AtomicBool, Ordering};

use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    cpu::{Cpu, MAX_CPUS, TranslationOwner},
    fix_map::FixMap,
    kernel_image::KernelImage,
    lds::Lds,
    raw_dtb::RawDtb,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
};

pub struct EarlyVm {
    lifecycle: Lifecycle,
    satp: usize,
    translation_sync_complete: [AtomicBool; MAX_CPUS],
}

impl EarlyVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            satp: 0,
            translation_sync_complete: [const { AtomicBool::new(false) }; MAX_CPUS],
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn satp(&self) -> usize {
        self.satp
    }

    pub fn translation_sync_complete(&self, cpu: &Cpu) -> bool {
        cpu.logical_id() < MAX_CPUS
            && self.translation_sync_complete[cpu.logical_id()].load(Ordering::Acquire)
    }

    pub fn adopt_take_over(&self, cpu: &Cpu) -> bool {
        if self.lifecycle.state() != State::Ready
            || cpu.logical_id() >= MAX_CPUS
            || cpu.active_translation_owner() != TranslationOwner::EarlyVm
            || csr::read_satp() != self.satp
            || !cpu.adopt_translation_takeover(
                TranslationOwner::TrampolineVm,
                TranslationOwner::EarlyVm,
                self.satp,
            )
        {
            return false;
        }
        self.translation_sync_complete[cpu.logical_id()].store(true, Ordering::Release);
        true
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

        raw_dtb.preset(boot_args)?;

        raw_dtb.setup()?;

        fix_map.preset(config, raw_dtb)?;

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

        let Some(satp) = static_objects.early_satp(kernel_image) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        };
        self.satp = satp;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::EarlyVmReady,
        )
    }

    pub fn current_on_cpu(&self, cpu: &Cpu) -> bool {
        self.state() == State::Ready
            && cpu.active_translation_owner() == TranslationOwner::EarlyVm
            && csr::read_satp() == self.satp
            && self.translation_sync_complete(cpu)
    }
}
