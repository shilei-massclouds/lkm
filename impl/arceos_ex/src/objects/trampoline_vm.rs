use core::sync::atomic::{AtomicBool, Ordering};

use crate::checkpoint::Checkpoint;

use super::{
    config::Config,
    cpu::{Cpu, MAX_CPUS},
    kernel_image::KernelImage,
    lds::Lds,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
};

pub struct TrampolineVm {
    lifecycle: Lifecycle,
    satp: usize,
    translation_sync_complete: [AtomicBool; MAX_CPUS],
}

impl TrampolineVm {
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

    pub fn adopt_completed_takeover(&self, cpu: &Cpu) -> bool {
        if self.lifecycle.state() != State::Ready || cpu.logical_id() >= MAX_CPUS || self.satp == 0
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
        self.satp = satp;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::TrampolineVmReady,
        )
    }
}
