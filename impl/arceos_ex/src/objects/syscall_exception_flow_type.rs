use super::{
    cpu::CpuRef,
    exception_flow_type::{ExceptionChildFlowRef, ExceptionChildKind},
    state::{EventResult, LifecycleEvent, State},
    trap_flow_type::OccurrenceCore,
};

pub struct SyscallExceptionFlowType {
    core: OccurrenceCore,
    flow_ref: ExceptionChildFlowRef,
    entry_saved: bool,
    may_schedule_and_migrate: bool,
    return_committed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SyscallExceptionFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            flow_ref: ExceptionChildFlowRef::NONE,
            entry_saved: false,
            may_schedule_and_migrate: false,
            return_committed: false,
        }
    }

    pub fn declare_and_bind(
        &mut self,
        generation: u32,
        address: usize,
        parent_cpu: CpuRef,
        parent_state: State,
    ) -> EventResult {
        self.core.declare_and_bind(
            generation,
            address,
            parent_cpu,
            parent_state == State::Online,
        )?;
        self.flow_ref =
            ExceptionChildFlowRef::new(address, generation, ExceptionChildKind::Syscall);
        Ok(())
    }

    pub fn preset(&mut self) -> EventResult {
        self.entry_saved = true;
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self) -> EventResult {
        self.may_schedule_and_migrate = true;
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable_after_handler(&mut self) -> EventResult {
        self.return_committed = true;
        self.core
            .transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    pub fn disable(&mut self) -> EventResult {
        self.core
            .transition(LifecycleEvent::Disable, State::Online, State::Offline)
    }

    pub fn cleanup(&mut self) -> EventResult {
        self.core.cleanup()
    }

    pub const fn state(&self) -> State {
        self.core.state()
    }

    pub const fn flow_ref(&self) -> ExceptionChildFlowRef {
        self.flow_ref
    }

    pub const fn may_schedule_and_migrate(&self) -> bool {
        self.may_schedule_and_migrate
    }
}
