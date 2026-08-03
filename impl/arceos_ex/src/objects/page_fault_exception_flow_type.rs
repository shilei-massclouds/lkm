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
    nested: bool,
    hardirq_context: bool,
    entry_interrupts_enabled: bool,
    atomic_context: bool,
    schedulable: bool,
    fixup_selected: bool,
    fixup_address: usize,
    fixup_committed: bool,
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
            nested: false,
            hardirq_context: false,
            entry_interrupts_enabled: false,
            atomic_context: false,
            schedulable: false,
            fixup_selected: false,
            fixup_address: 0,
            fixup_committed: false,
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

    pub fn setup_context(
        &mut self,
        nested: bool,
        hardirq_context: bool,
        entry_interrupts_enabled: bool,
        fixup_address: Option<usize>,
    ) -> EventResult {
        if self.core.state() != State::Prepared || fixup_address == Some(0) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.core.state(),
                State::Prepared,
                State::Ready,
            );
        }
        self.nested = nested;
        self.hardirq_context = hardirq_context;
        self.entry_interrupts_enabled = entry_interrupts_enabled;
        self.atomic_context = nested || hardirq_context || !entry_interrupts_enabled;
        self.schedulable = match self.source {
            PageFaultSource::User => !self.atomic_context,
            PageFaultSource::Kernel => fixup_address.is_some() && !self.atomic_context,
        };
        self.fixup_selected =
            matches!(self.source, PageFaultSource::Kernel) && fixup_address.is_some();
        self.fixup_address = fixup_address.unwrap_or(0);
        self.fixup_committed = false;
        self.fatal_selected =
            matches!(self.source, PageFaultSource::Kernel) && fixup_address.is_none();
        self.core
            .transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn commit_kernel_fixup(&mut self, address: usize) -> EventResult {
        if self.core.state() != State::Ready
            || !matches!(self.source, PageFaultSource::Kernel)
            || !self.fixup_selected
            || address == 0
            || address != self.fixup_address
            || self.fixup_committed
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.core.state(),
                State::Ready,
                State::Online,
            );
        }
        self.fixup_committed = true;
        Ok(())
    }

    pub fn enable_after_handler(&mut self) -> EventResult {
        if matches!(self.source, PageFaultSource::Kernel) && !self.fixup_committed {
            return failed_condition(
                LifecycleEvent::Enable,
                self.core.state(),
                State::Ready,
                State::Online,
            );
        }
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

    pub const fn atomic_context(&self) -> bool {
        self.atomic_context
    }

    pub const fn fatal_selected(&self) -> bool {
        self.fatal_selected
    }

    pub const fn fixup_address(&self) -> Option<usize> {
        if self.fixup_selected {
            Some(self.fixup_address)
        } else {
            None
        }
    }

    pub const fn parent_cpu(&self) -> CpuRef {
        self.core.parent_cpu()
    }
}
