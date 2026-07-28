use super::{
    cpu::CpuRef,
    state::{EventResult, LifecycleEvent, State, failed_condition},
    trap_flow_type::{OccurrenceCore, TrapChildFlowRef, TrapChildKind},
};

pub struct InterruptFlowType {
    core: OccurrenceCore,
    flow_ref: TrapChildFlowRef,
    entry_saved: bool,
    hardirq_schedule_forbidden: bool,
    ordinary_reentry_forbidden: bool,
    handler_completed: bool,
    completion_committed: bool,
}

impl InterruptFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            flow_ref: TrapChildFlowRef::NONE,
            entry_saved: false,
            hardirq_schedule_forbidden: false,
            ordinary_reentry_forbidden: false,
            handler_completed: false,
            completion_committed: false,
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
        self.flow_ref = TrapChildFlowRef::new(address, generation, TrapChildKind::Interrupt);
        Ok(())
    }

    pub fn preset(&mut self) -> EventResult {
        self.entry_saved = true;
        self.hardirq_schedule_forbidden = true;
        self.ordinary_reentry_forbidden = true;
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup_after_handler(&mut self) -> EventResult {
        self.handler_completed = true;
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self) -> EventResult {
        if !self.handler_completed {
            return failed_condition(
                LifecycleEvent::Enable,
                self.core.state(),
                State::Ready,
                State::Online,
            );
        }
        self.completion_committed = true;
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

    pub const fn flow_ref(&self) -> TrapChildFlowRef {
        self.flow_ref
    }

    pub const fn hardirq_schedule_forbidden(&self) -> bool {
        self.hardirq_schedule_forbidden
    }

    pub const fn ordinary_reentry_forbidden(&self) -> bool {
        self.ordinary_reentry_forbidden
    }
}
