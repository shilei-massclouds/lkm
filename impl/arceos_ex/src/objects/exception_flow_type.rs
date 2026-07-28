use super::{
    cpu::CpuRef,
    state::{EventResult, LifecycleEvent, State, failed_condition},
    trap_flow_type::{OccurrenceCore, TrapChildFlowRef, TrapChildKind},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExceptionChildKind {
    None,
    PageFault,
    Syscall,
    Breakpoint,
    Unexpected,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ExceptionChildFlowRef {
    address: usize,
    generation: u32,
    kind: ExceptionChildKind,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl ExceptionChildFlowRef {
    pub const NONE: Self = Self {
        address: 0,
        generation: 0,
        kind: ExceptionChildKind::None,
    };

    pub(crate) const fn new(address: usize, generation: u32, kind: ExceptionChildKind) -> Self {
        Self {
            address,
            generation,
            kind,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.address != 0 && self.generation != 0 && !matches!(self.kind, ExceptionChildKind::None)
    }

    pub const fn address(self) -> usize {
        self.address
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn kind(self) -> ExceptionChildKind {
        self.kind
    }
}

pub struct ExceptionFlowType {
    core: OccurrenceCore,
    flow_ref: TrapChildFlowRef,
    active_child: ExceptionChildFlowRef,
    entry_validated: bool,
    child_completed: bool,
    child_destroyed: bool,
    completion_committed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl ExceptionFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            flow_ref: TrapChildFlowRef::NONE,
            active_child: ExceptionChildFlowRef::NONE,
            entry_validated: false,
            child_completed: false,
            child_destroyed: false,
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
            matches!(parent_state, State::Ready | State::Online),
        )?;
        self.flow_ref = TrapChildFlowRef::new(address, generation, TrapChildKind::Exception);
        Ok(())
    }

    pub fn preset(&mut self, scause: usize) -> EventResult {
        if scause & (1usize << (usize::BITS as usize - 1)) != 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.core.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.entry_validated = true;
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn select_child(&mut self, child_ref: ExceptionChildFlowRef) -> EventResult {
        if self.core.state() != State::Prepared
            || !child_ref.is_valid()
            || child_ref.generation() != self.core.generation()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.core.state(),
                State::Prepared,
                State::Ready,
            );
        }
        self.active_child = child_ref;
        Ok(())
    }

    pub fn mark_child_completed(&mut self, child_ref: ExceptionChildFlowRef) -> EventResult {
        if self.active_child != child_ref {
            return failed_condition(
                LifecycleEvent::Setup,
                self.core.state(),
                State::Prepared,
                State::Ready,
            );
        }
        self.child_completed = true;
        self.child_destroyed = true;
        Ok(())
    }

    pub fn setup(&mut self) -> EventResult {
        if !self.entry_validated || !self.child_completed {
            return failed_condition(
                LifecycleEvent::Setup,
                self.core.state(),
                State::Prepared,
                State::Ready,
            );
        }
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self) -> EventResult {
        self.completion_committed = true;
        self.core
            .transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    pub fn disable(&mut self) -> EventResult {
        if !self.child_destroyed {
            return failed_condition(
                LifecycleEvent::Disable,
                self.core.state(),
                State::Online,
                State::Offline,
            );
        }
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

    pub const fn active_child(&self) -> ExceptionChildFlowRef {
        self.active_child
    }
}
