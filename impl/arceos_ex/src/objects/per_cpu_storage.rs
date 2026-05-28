use super::{
    cpu_group::CpuGroup,
    cpu_id_map::CpuIdMap,
    memblock::MemBlock,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

pub struct PerCpuStorage {
    lifecycle: Lifecycle,
}

impl PerCpuStorage {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(
        &mut self,
        memblock: &MemBlock,
        vm: &Vm,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
        {
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
            Checkpoint::PerCpuStorageReady,
        )
    }
}
