use super::{
    cpu::CpuRef,
    exception_flow_type::{ExceptionChildFlowRef, ExceptionChildKind},
    state::{EventResult, LifecycleEvent, State, failed_condition},
    trap_flow_type::OccurrenceCore,
};

const EXC_INSTRUCTION_PAGE_FAULT: usize = 12;
const EXC_LOAD_PAGE_FAULT: usize = 13;
const EXC_STORE_PAGE_FAULT: usize = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageFaultAccess {
    Execute,
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageFaultSource {
    User,
    Kernel,
}

pub struct PageFaultExceptionFlowType {
    core: OccurrenceCore,
    flow_ref: ExceptionChildFlowRef,
    fault_address: usize,
    access: PageFaultAccess,
    source: PageFaultSource,
    atomic_context: bool,
    schedulable: bool,
    fixup_selected: bool,
    fatal_selected: bool,
    handler_completed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl PageFaultExceptionFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            flow_ref: ExceptionChildFlowRef::NONE,
            fault_address: 0,
            access: PageFaultAccess::Read,
            source: PageFaultSource::Kernel,
            atomic_context: false,
            schedulable: false,
            fixup_selected: false,
            fatal_selected: false,
            handler_completed: false,
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
            ExceptionChildFlowRef::new(address, generation, ExceptionChildKind::PageFault);
        Ok(())
    }

    pub fn preset(&mut self, scause: usize, stval: usize, from_user: bool) -> EventResult {
        self.access = match scause {
            EXC_INSTRUCTION_PAGE_FAULT => PageFaultAccess::Execute,
            EXC_LOAD_PAGE_FAULT => PageFaultAccess::Read,
            EXC_STORE_PAGE_FAULT => PageFaultAccess::Write,
            _ => {
                return failed_condition(
                    LifecycleEvent::Preset,
                    self.core.state(),
                    State::Base,
                    State::Prepared,
                );
            }
        };
        self.fault_address = stval;
        self.source = if from_user {
            PageFaultSource::User
        } else {
            PageFaultSource::Kernel
        };
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup_context(&mut self, atomic_context: bool, fixup_available: bool) -> EventResult {
        self.atomic_context = atomic_context;
        self.schedulable = !atomic_context && matches!(self.source, PageFaultSource::User);
        self.fixup_selected = atomic_context && fixup_available;
        self.fatal_selected = atomic_context && !fixup_available;
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable_after_handler(&mut self) -> EventResult {
        self.handler_completed = true;
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

    pub const fn schedulable(&self) -> bool {
        self.schedulable
    }

    pub const fn parent_cpu(&self) -> CpuRef {
        self.core.parent_cpu()
    }
}
