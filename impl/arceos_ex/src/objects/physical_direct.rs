use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::riscv64::csr;

use super::{
    cpu::{
        Cpu, MAX_CPUS, TranslationActivationKind, TranslationActivationTrace, TranslationController,
    },
    state::{EventResult, LifecycleEvent, State, failed_condition},
};

pub struct PhysicalDirect {
    activation_complete: [AtomicBool; MAX_CPUS],
}

impl PhysicalDirect {
    pub const fn new() -> Self {
        Self {
            activation_complete: [const { AtomicBool::new(false) }; MAX_CPUS],
        }
    }

    pub const fn state(&self) -> State {
        State::Ready
    }

    pub(crate) fn adopt_head_activation_on(&self, cpu: &Cpu) -> EventResult {
        if cpu.logical_id() >= MAX_CPUS
            || cpu.active_translation_controller() != Ok(None)
            || !cpu.translation_activation_preflight(
                TranslationActivationKind::InitialActivation,
                None,
                TranslationController::PhysicalDirect,
                0,
                csr::read_satp(),
            )
        {
            return failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Ready,
                State::Ready,
            );
        }
        if !cpu.commit_translation_activation(
            TranslationActivationKind::InitialActivation,
            None,
            TranslationController::PhysicalDirect,
            0,
            csr::read_satp(),
        ) {
            return failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Ready,
                State::Ready,
            );
        }
        self.activation_complete[cpu.logical_id()].store(true, Ordering::Release);
        Ok(())
    }

    pub fn complete_arch_activation_on(&self, cpu: &Cpu) -> bool {
        if cpu.logical_id() >= MAX_CPUS
            || !cpu.translation_receipt_matches(
                0,
                TranslationActivationTrace::completed(
                    TranslationActivationKind::InitialActivation,
                    None,
                    TranslationController::PhysicalDirect,
                    0,
                    1,
                ),
            )
        {
            return false;
        }
        self.activation_complete[cpu.logical_id()].store(true, Ordering::Release);
        true
    }

    pub fn activation_complete_on(&self, cpu: &Cpu) -> bool {
        cpu.logical_id() < MAX_CPUS
            && self.activation_complete[cpu.logical_id()].load(Ordering::Acquire)
    }
}
