use super::{
    cpu_group::CpuGroup,
    per_cpu_storage::PerCpuStorage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuHotplugLifecycleState {
    Online,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuHotplugSyncState {
    Online,
}

pub struct CpuHotplugState {
    lifecycle: Lifecycle,
    hartid: usize,
    current: CpuHotplugLifecycleState,
    target: CpuHotplugLifecycleState,
    sync: CpuHotplugSyncState,
    booted_once: bool,
}

impl CpuHotplugState {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hartid: usize::MAX,
            current: CpuHotplugLifecycleState::Online,
            target: CpuHotplugLifecycleState::Online,
            sync: CpuHotplugSyncState::Online,
            booted_once: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn hartid(&self) -> usize {
        self.hartid
    }

    #[allow(dead_code)]
    pub const fn current(&self) -> CpuHotplugLifecycleState {
        self.current
    }

    #[allow(dead_code)]
    pub const fn target(&self) -> CpuHotplugLifecycleState {
        self.target
    }

    #[allow(dead_code)]
    pub const fn sync(&self) -> CpuHotplugSyncState {
        self.sync
    }

    #[allow(dead_code)]
    pub const fn booted_once(&self) -> bool {
        self.booted_once
    }

    pub fn setup(&mut self, cpu_group: &CpuGroup, per_cpu_storage: &PerCpuStorage) -> EventResult {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        if self.lifecycle.state() != State::Base
            || cpu_group.boot_cpu_state() != State::Online
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.hartid = boot_cpu.hartid();
        self.current = CpuHotplugLifecycleState::Online;
        self.target = CpuHotplugLifecycleState::Online;
        self.sync = CpuHotplugSyncState::Online;
        self.booted_once = true;

        if !self.ready_facts_hold(cpu_group) {
            return failed_condition(
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
            Checkpoint::CpuHotplugStateReady,
        )
    }

    fn ready_facts_hold(&self, cpu_group: &CpuGroup) -> bool {
        self.hartid
            == cpu_group
                .boot_cpu()
                .map(|cpu| cpu.hartid())
                .unwrap_or(usize::MAX)
            && self.current == CpuHotplugLifecycleState::Online
            && self.target == CpuHotplugLifecycleState::Online
            && self.sync == CpuHotplugSyncState::Online
            && self.booted_once
    }
}
