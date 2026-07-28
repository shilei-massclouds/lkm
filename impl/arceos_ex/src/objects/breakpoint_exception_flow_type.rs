use super::{
    cpu::CpuRef,
    exception_flow_type::{ExceptionChildFlowRef, ExceptionChildKind},
    state::{EventResult, LifecycleEvent, State},
    trap_flow_type::OccurrenceCore,
};

pub struct BreakpointExceptionFlowType {
    core: OccurrenceCore,
    flow_ref: ExceptionChildFlowRef,
    entry_saved: bool,
    hook_completed: bool,
    resume_committed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl BreakpointExceptionFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            flow_ref: ExceptionChildFlowRef::NONE,
            entry_saved: false,
            hook_completed: false,
            resume_committed: false,
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
            ExceptionChildFlowRef::new(address, generation, ExceptionChildKind::Breakpoint);
        Ok(())
    }

    pub fn preset(&mut self) -> EventResult {
        self.entry_saved = true;
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup_after_hook(&mut self) -> EventResult {
        self.hook_completed = true;
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self) -> EventResult {
        self.resume_committed = true;
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
}
