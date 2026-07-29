use core::sync::atomic::{AtomicBool, Ordering};

use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    cpu::{Cpu, MAX_CPUS, TranslationOwner},
    state::{EventResult, LifecycleEvent, State, failed_condition},
};

pub struct PhysicalDirect {
    takeover_complete: [AtomicBool; MAX_CPUS],
}

impl PhysicalDirect {
    pub const fn new() -> Self {
        Self {
            takeover_complete: [const { AtomicBool::new(false) }; MAX_CPUS],
        }
    }

    pub const fn state(&self) -> State {
        State::Ready
    }

    pub fn take_over(&self, cpu: &Cpu) -> EventResult {
        if cpu.logical_id() >= MAX_CPUS
            || cpu.active_translation_owner() != TranslationOwner::None
            || csr::read_satp() != 0
        {
            return failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Ready,
                State::Ready,
            );
        }
        csr::sfence_vma();
        if !cpu.commit_translation_takeover(
            TranslationOwner::None,
            TranslationOwner::PhysicalDirect,
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
        self.takeover_complete[cpu.logical_id()].store(true, Ordering::Release);
        crate::checkpoint::checkpoint(Checkpoint::PhysicalDirectTakeOver);
        Ok(())
    }

    pub fn complete_arch_take_over(&self, cpu: &Cpu) -> bool {
        if cpu.logical_id() >= MAX_CPUS
            || !cpu.translation_receipt_matches(
                0,
                super::cpu::TranslationTakeoverTrace::completed(
                    TranslationOwner::None,
                    TranslationOwner::PhysicalDirect,
                    0,
                    1,
                ),
            )
        {
            return false;
        }
        self.takeover_complete[cpu.logical_id()].store(true, Ordering::Release);
        true
    }

    pub fn takeover_complete_for(&self, cpu: &Cpu) -> bool {
        cpu.logical_id() < MAX_CPUS
            && self.takeover_complete[cpu.logical_id()].load(Ordering::Acquire)
    }
}
