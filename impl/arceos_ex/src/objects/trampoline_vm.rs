use core::sync::atomic::{AtomicBool, Ordering};

use crate::checkpoint::Checkpoint;

use super::{
    config::Config,
    cpu::{
        Cpu, MAX_CPUS, TranslationActivationKind, TranslationActivationTrace, TranslationController,
    },
    kernel_image::KernelImage,
    lds::Lds,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
};

pub struct TrampolineVm {
    lifecycle: Lifecycle,
    satp: usize,
    mapping_virt_start: usize,
    mapping_virt_end: usize,
    translation_sync_complete: [AtomicBool; MAX_CPUS],
}

impl TrampolineVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            satp: 0,
            mapping_virt_start: 0,
            mapping_virt_end: 0,
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

    pub fn complete_arch_activation_on(&self, cpu: &Cpu) -> bool {
        if self.lifecycle.state() != State::Ready
            || cpu.logical_id() >= MAX_CPUS
            || self.satp == 0
            || !cpu.translation_receipt_matches(
                1,
                TranslationActivationTrace::completed(
                    TranslationActivationKind::Handoff,
                    Some(TranslationController::PhysicalDirect),
                    TranslationController::TrampolineVm,
                    self.satp,
                    2,
                ),
            )
        {
            return false;
        }
        self.translation_sync_complete[cpu.logical_id()].store(true, Ordering::Release);
        true
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

        let Some(satp) = static_objects.trampoline_satp(kernel_image) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        let mapping_virt_start = kernel_image.virt_start();
        let Some(mapping_virt_end) = mapping_virt_start.checked_add(config.pmd_size()) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        self.satp = satp;
        self.mapping_virt_start = mapping_virt_start;
        self.mapping_virt_end = mapping_virt_end;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::TrampolineVmReady,
        )
    }

    pub fn translation_state_mapped(&self, cpu: &Cpu, kernel_image: &KernelImage) -> bool {
        let Some((runtime_start, runtime_end)) = cpu.translation_state_storage_range() else {
            return false;
        };
        let Some(runtime_last) = runtime_end.checked_sub(1) else {
            return false;
        };
        let Some(virt_start) = kernel_image.runtime_to_link(runtime_start) else {
            return false;
        };
        let Some(virt_last) = kernel_image.runtime_to_link(runtime_last) else {
            return false;
        };
        let Some(virt_end) = virt_last.checked_add(1) else {
            return false;
        };
        let Some(size) = virt_end.checked_sub(virt_start) else {
            return false;
        };
        trampoline_window_contains_range(
            self.mapping_virt_start,
            self.mapping_virt_end,
            virt_start,
            size,
        )
    }
}

pub(crate) const fn trampoline_window_contains_range(
    window_start: usize,
    window_end: usize,
    range_start: usize,
    range_size: usize,
) -> bool {
    let Some(range_end) = range_start.checked_add(range_size) else {
        return false;
    };
    window_start < window_end
        && range_size != 0
        && range_start >= window_start
        && range_end <= window_end
}
