use core::sync::atomic::{AtomicBool, Ordering};

use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    config::Config,
    cpu::{
        Cpu, MAX_CPUS, TranslationActivationKind, TranslationActivationTrace, TranslationController,
    },
    kernel_addr_space::KernelAddrSpace,
    kernel_image::KernelImage,
    lds::Lds,
    memblock::MemBlock,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
};

pub struct SwapperVm {
    lifecycle: Lifecycle,
    satp: usize,
    translation_sync_complete: [AtomicBool; MAX_CPUS],
    strict_kernel_rwx_boundary_deferred: bool,
    final_permissions_not_split_yet: bool,
}

impl SwapperVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            satp: 0,
            translation_sync_complete: [const { AtomicBool::new(false) }; MAX_CPUS],
            strict_kernel_rwx_boundary_deferred: false,
            final_permissions_not_split_yet: false,
        }
    }

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

    pub const fn strict_kernel_rwx_boundary_deferred(&self) -> bool {
        self.strict_kernel_rwx_boundary_deferred
    }

    pub const fn final_permissions_not_split_yet(&self) -> bool {
        self.final_permissions_not_split_yet
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
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !static_objects.build_swapper_pg_dir(
            config,
            kernel_image,
            lds.kernel_start(),
            lds.kernel_end(),
            memblock,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.strict_kernel_rwx_boundary_deferred = true;
        self.final_permissions_not_split_yet = true;
        let Some(satp) = static_objects.swapper_satp(kernel_image) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        self.satp = satp;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SwapperVmReady,
        )
    }

    pub fn activate_on_cpu(
        &self,
        cpu: &Cpu,
        old_satp: usize,
        kernel_addr_space: &KernelAddrSpace,
    ) -> EventResult {
        let Ok(Some(old_controller)) = cpu.active_translation_controller() else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        };
        if self.lifecycle.state() != State::Ready
            || kernel_addr_space.state() != State::Online
            || cpu.logical_id() >= MAX_CPUS
            || !matches!(
                old_controller,
                TranslationController::TrampolineVm | TranslationController::EarlyVm
            )
            || !cpu.translation_activation_preflight(
                TranslationActivationKind::Handoff,
                Some(old_controller),
                TranslationController::SwapperVm,
                old_satp,
                csr::read_satp(),
            )
            || self.satp == 0
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        csr::write_satp(self.satp);
        csr::sfence_vma();
        let observed_satp = csr::read_satp();
        if !cpu.commit_translation_activation(
            TranslationActivationKind::Handoff,
            Some(old_controller),
            TranslationController::SwapperVm,
            self.satp,
            observed_satp,
        ) || !self.complete_arch_activation_on(cpu, old_controller)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        crate::checkpoint::checkpoint(Checkpoint::SwapperVmActivatedOnCpu);
        Ok(())
    }

    pub fn complete_arch_activation_on(
        &self,
        cpu: &Cpu,
        old_controller: TranslationController,
    ) -> bool {
        let (index, sequence) = match old_controller {
            TranslationController::TrampolineVm => (2, 3),
            TranslationController::EarlyVm => (3, 4),
            _ => return false,
        };
        if self.lifecycle.state() != State::Ready
            || cpu.logical_id() >= MAX_CPUS
            || cpu.active_translation_controller() != Ok(Some(TranslationController::SwapperVm))
            || csr::read_satp() != self.satp
            || !cpu.translation_receipt_matches(
                index,
                TranslationActivationTrace::completed(
                    TranslationActivationKind::Handoff,
                    Some(old_controller),
                    TranslationController::SwapperVm,
                    self.satp,
                    sequence,
                ),
            )
        {
            return false;
        }
        self.translation_sync_complete[cpu.logical_id()].store(true, Ordering::Release);
        true
    }

    pub fn activation_complete_on(&self, cpu: &Cpu) -> bool {
        self.state() == State::Ready
            && cpu.logical_id() < MAX_CPUS
            && cpu.active_translation_controller() == Ok(Some(TranslationController::SwapperVm))
            && self.translation_sync_complete(cpu)
    }

    pub fn current_on_cpu(&self, cpu: &Cpu) -> bool {
        self.state() == State::Ready
            && cpu.active_translation_controller() == Ok(Some(TranslationController::SwapperVm))
            && csr::read_satp() == self.satp
            && self.translation_sync_complete(cpu)
    }
}
