use super::{
    cpu::CpuRef,
    exception_flow_type::{ExceptionChildFlowRef, ExceptionChildKind},
    state::{EventResult, LifecycleEvent, State},
    trap_flow_type::OccurrenceCore,
};

pub struct UnexpectedExceptionFlowType {
    core: OccurrenceCore,
    flow_ref: ExceptionChildFlowRef,
    diagnostic_identity_saved: bool,
    fail_shutdown_selected: bool,
    fail_shutdown_committed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl UnexpectedExceptionFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            flow_ref: ExceptionChildFlowRef::NONE,
            diagnostic_identity_saved: false,
            fail_shutdown_selected: false,
            fail_shutdown_committed: false,
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
            ExceptionChildFlowRef::new(address, generation, ExceptionChildKind::Unexpected);
        Ok(())
    }

    pub fn preset(&mut self) -> EventResult {
        self.diagnostic_identity_saved = true;
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self) -> EventResult {
        self.fail_shutdown_selected = true;
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self) -> EventResult {
        self.fail_shutdown_committed = true;
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
