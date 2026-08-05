use super::{
    breakpoint_exception_flow_type::BreakpointExceptionFlowType,
    cpu::CpuRef,
    exception_flow_type::ExceptionFlowType,
    interrupt_flow_type::InterruptFlowType,
    page_fault_exception_flow_type::PageFaultExceptionFlowType,
    state::{EventError, EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    syscall_exception_flow_type::SyscallExceptionFlowType,
    task::TaskRef,
    task_flow::TaskFlowRef,
    unexpected_exception_flow_type::UnexpectedExceptionFlowType,
};

#[cfg(app_smoke)]
use super::exception_flow_type::{ExceptionChildFlowRef, ExceptionChildKind};

pub const TRAP_RETURN_TOKEN_MAGIC: usize = 0x5452_4150_5245_5455;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrapCauseClass {
    Interrupt,
    Exception,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrapChildKind {
    None,
    Interrupt,
    Exception,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TrapFlowRef {
    address: usize,
    generation: u32,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl TrapFlowRef {
    pub const NONE: Self = Self {
        address: 0,
        generation: 0,
    };

    pub(crate) const fn new(address: usize, generation: u32) -> Self {
        Self {
            address,
            generation,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.address != 0 && self.generation != 0
    }

    pub const fn address(self) -> usize {
        self.address
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn same_identity(self, other: Self) -> bool {
        self.address == other.address && self.generation == other.generation
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TrapChildFlowRef {
    address: usize,
    generation: u32,
    kind: TrapChildKind,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl TrapChildFlowRef {
    pub const NONE: Self = Self {
        address: 0,
        generation: 0,
        kind: TrapChildKind::None,
    };

    pub(crate) const fn new(address: usize, generation: u32, kind: TrapChildKind) -> Self {
        Self {
            address,
            generation,
            kind,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.address != 0 && self.generation != 0 && !matches!(self.kind, TrapChildKind::None)
    }

    pub const fn address(self) -> usize {
        self.address
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn kind(self) -> TrapChildKind {
        self.kind
    }
}

pub struct TrapReturnToken {
    root_ref: TrapFlowRef,
}

impl TrapReturnToken {
    fn new(root_ref: TrapFlowRef) -> Self {
        Self { root_ref }
    }

    /// Consumes the one-shot token only after the root occurrence is destroyed.
    pub fn consume_after_cleanup(self, root: &TrapFlowType) -> Option<usize> {
        if root.state() == State::Destroyed
            && root.identity_matches(self.root_ref)
            && root.return_allowed_after_cleanup
        {
            Some(TRAP_RETURN_TOKEN_MAGIC)
        } else {
            None
        }
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
#[derive(Clone, Copy)]
pub struct TrapEntrySnapshot {
    pub cause_class: TrapCauseClass,
    pub scause: usize,
    pub sepc: usize,
    pub sstatus: usize,
    pub stval: usize,
    pub entry_task: TaskRef,
    pub effective_task_flow: TaskFlowRef,
    pub context_epoch: u64,
}

pub(crate) struct OccurrenceCore {
    lifecycle: Lifecycle,
    generation: u32,
    address: usize,
    parent_cpu: CpuRef,
    declared: bool,
    parent_bound: bool,
    alive: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl OccurrenceCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            generation: 0,
            address: 0,
            parent_cpu: CpuRef::invalid(),
            declared: false,
            parent_bound: false,
            alive: false,
        }
    }

    pub fn declare_and_bind(
        &mut self,
        generation: u32,
        address: usize,
        parent_cpu: CpuRef,
        parent_ready: bool,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.declared
            || generation == 0
            || address == 0
            || !parent_cpu.is_valid()
            || !parent_ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.generation = generation;
        self.address = address;
        self.parent_cpu = parent_cpu;
        self.declared = true;
        self.parent_bound = true;
        self.alive = true;
        Ok(())
    }

    pub fn transition(
        &mut self,
        event: LifecycleEvent,
        expected: State,
        target: State,
    ) -> EventResult {
        if !self.declared || !self.parent_bound || !self.alive {
            return failed_condition(event, self.lifecycle.state(), expected, target);
        }
        self.lifecycle.adopt_transition(event, expected, target)
    }

    pub fn cleanup(&mut self) -> EventResult {
        self.transition(LifecycleEvent::Cleanup, State::Offline, State::Destroyed)?;
        self.alive = false;
        Ok(())
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn generation(&self) -> u32 {
        self.generation
    }

    pub const fn address(&self) -> usize {
        self.address
    }

    pub const fn parent_cpu(&self) -> CpuRef {
        self.parent_cpu
    }

    pub const fn alive(&self) -> bool {
        self.alive
    }

    pub const fn identity_matches(&self, address: usize, generation: u32) -> bool {
        self.address() == address && self.generation == generation
    }
}

pub struct TrapFlowType {
    core: OccurrenceCore,
    root_ref: TrapFlowRef,
    active_child: TrapChildFlowRef,
    snapshot: Option<TrapEntrySnapshot>,
    child_completed: bool,
    child_destroyed: bool,
    handler_completed: bool,
    return_token_created: bool,
    return_allowed_after_cleanup: bool,
    task_flow_paused: bool,
    task_flow_resumed: bool,
    return_cpu: CpuRef,
    suspended_context_epoch: u64,
    resumed_context_epoch: u64,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl TrapFlowType {
    pub const fn new() -> Self {
        Self {
            core: OccurrenceCore::new(),
            root_ref: TrapFlowRef::NONE,
            active_child: TrapChildFlowRef::NONE,
            snapshot: None,
            child_completed: false,
            child_destroyed: false,
            handler_completed: false,
            return_token_created: false,
            return_allowed_after_cleanup: false,
            task_flow_paused: false,
            task_flow_resumed: false,
            return_cpu: CpuRef::invalid(),
            suspended_context_epoch: 0,
            resumed_context_epoch: 0,
        }
    }

    pub fn declare_and_bind(
        &mut self,
        generation: u32,
        address: usize,
        parent_cpu: CpuRef,
        parent_ready: bool,
    ) -> EventResult {
        self.core
            .declare_and_bind(generation, address, parent_cpu, parent_ready)?;
        self.root_ref = TrapFlowRef::new(address, generation);
        Ok(())
    }

    pub fn preset(&mut self, snapshot: TrapEntrySnapshot) -> EventResult {
        if !snapshot.entry_task.is_valid()
            || !snapshot.effective_task_flow.is_valid()
            || snapshot.scause == 0 && matches!(snapshot.cause_class, TrapCauseClass::Interrupt)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.core.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.snapshot = Some(snapshot);
        self.task_flow_paused = true;
        self.core
            .transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn select_child(&mut self, child_ref: TrapChildFlowRef) -> EventResult {
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

    pub fn mark_child_completed(&mut self, child_ref: TrapChildFlowRef) -> EventResult {
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
        if !self.active_child.is_valid() || !self.child_completed {
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

    pub fn enable(&mut self) -> Result<TrapReturnToken, EventError> {
        if !self.child_completed || self.return_token_created {
            return Err(EventError::failed(
                super::state::EventErrorCode::ConditionFailed,
                LifecycleEvent::Enable,
                self.core.state(),
                State::Ready,
                State::Online,
            ));
        }
        self.handler_completed = true;
        self.return_token_created = true;
        self.core
            .transition(LifecycleEvent::Enable, State::Ready, State::Online)?;
        Ok(TrapReturnToken::new(self.root_ref))
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

    pub fn cleanup(&mut self, return_cpu: CpuRef) -> EventResult {
        if !self.child_destroyed || !return_cpu.is_valid() || return_cpu != self.core.parent_cpu() {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.core.state(),
                State::Offline,
                State::Destroyed,
            );
        }
        self.return_cpu = return_cpu;
        self.task_flow_resumed = true;
        self.return_allowed_after_cleanup = true;
        self.core.cleanup()
    }

    pub const fn state(&self) -> State {
        self.core.state()
    }

    pub const fn flow_ref(&self) -> TrapFlowRef {
        self.root_ref
    }

    pub const fn active_child(&self) -> TrapChildFlowRef {
        self.active_child
    }

    pub const fn parent_cpu(&self) -> CpuRef {
        self.core.parent_cpu()
    }

    pub const fn return_cpu(&self) -> CpuRef {
        self.return_cpu
    }

    pub const fn alive(&self) -> bool {
        self.core.alive()
    }

    pub const fn identity_matches(&self, flow_ref: TrapFlowRef) -> bool {
        self.core
            .identity_matches(flow_ref.address(), flow_ref.generation())
    }

    pub const fn resolves(&self, flow_ref: TrapFlowRef) -> bool {
        self.alive() && self.identity_matches(flow_ref)
    }

    pub const fn task_flow_paused(&self) -> bool {
        self.task_flow_paused
    }

    pub const fn task_flow_resumed(&self) -> bool {
        self.task_flow_resumed
    }

    pub fn record_suspended_context(
        root_ref: TrapFlowRef,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
        context_epoch: u64,
    ) -> bool {
        let Some(root) = (unsafe { root_mut(root_ref) }) else {
            return false;
        };
        if !root.context_identity_matches(task_ref, flow_ref, cpu_ref) || context_epoch == 0 {
            return false;
        }
        root.suspended_context_epoch = context_epoch;
        true
    }

    pub fn active_leaf_preflight(
        root_ref: TrapFlowRef,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
        context_epoch: u64,
    ) -> bool {
        let Some(record) = (unsafe { record_ref(root_ref) }) else {
            return false;
        };
        record
            .root
            .context_identity_matches(task_ref, flow_ref, cpu_ref)
            && record.root.suspended_context_epoch == context_epoch
            && record.active_leaf_alive(root_ref.generation())
    }

    pub fn active_leaf_matches(
        root_ref: TrapFlowRef,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
    ) -> bool {
        let Some(record) = (unsafe { record_ref(root_ref) }) else {
            return false;
        };
        record
            .root
            .context_identity_matches(task_ref, flow_ref, cpu_ref)
            && record.active_leaf_alive(root_ref.generation())
    }

    pub fn resume_active_leaf(
        root_ref: TrapFlowRef,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
        context_epoch: u64,
    ) -> bool {
        if !Self::active_leaf_preflight(root_ref, task_ref, flow_ref, cpu_ref, context_epoch) {
            return false;
        }
        let Some(root) = (unsafe { root_mut(root_ref) }) else {
            return false;
        };
        if root.resumed_context_epoch == context_epoch {
            return false;
        }
        root.resumed_context_epoch = context_epoch;
        true
    }

    pub fn active_leaf_resumed(
        root_ref: TrapFlowRef,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
        context_epoch: u64,
    ) -> bool {
        let Some(record) = (unsafe { record_ref(root_ref) }) else {
            return false;
        };
        record
            .root
            .context_identity_matches(task_ref, flow_ref, cpu_ref)
            && record.root.suspended_context_epoch == context_epoch
            && record.root.resumed_context_epoch == context_epoch
            && record.active_leaf_alive(root_ref.generation())
    }

    pub fn active_child_is_interrupt(root_ref: TrapFlowRef) -> bool {
        let Some(record) = (unsafe { record_ref(root_ref) }) else {
            return false;
        };
        record.root.active_child().kind() == TrapChildKind::Interrupt
            && record.active_leaf_alive(root_ref.generation())
    }

    fn context_identity_matches(
        &self,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
    ) -> bool {
        self.resolves(self.root_ref)
            && self.core.state() == State::Prepared
            && self.parent_cpu() == cpu_ref
            && self.snapshot.is_some_and(|snapshot| {
                snapshot.entry_task.same_identity(task_ref)
                    && snapshot.effective_task_flow.same_identity(flow_ref)
                    && snapshot.context_epoch != 0
            })
    }
}

#[repr(C)]
pub struct TrapExecutionRecord {
    pub root: TrapFlowType,
    pub interrupt: InterruptFlowType,
    pub exception: ExceptionFlowType,
    pub page_fault: PageFaultExceptionFlowType,
    pub syscall: SyscallExceptionFlowType,
    pub breakpoint: BreakpointExceptionFlowType,
    pub unexpected: UnexpectedExceptionFlowType,
}

impl TrapExecutionRecord {
    pub const fn new() -> Self {
        Self {
            root: TrapFlowType::new(),
            interrupt: InterruptFlowType::new(),
            exception: ExceptionFlowType::new(),
            page_fault: PageFaultExceptionFlowType::new(),
            syscall: SyscallExceptionFlowType::new(),
            breakpoint: BreakpointExceptionFlowType::new(),
            unexpected: UnexpectedExceptionFlowType::new(),
        }
    }

    fn active_leaf_alive(&self, generation: u32) -> bool {
        let child = self.root.active_child();
        if child.generation() != generation {
            return false;
        }
        match child.kind() {
            TrapChildKind::Interrupt => {
                child == self.interrupt.flow_ref()
                    && matches!(
                        self.interrupt.state(),
                        State::Prepared | State::Ready | State::Online | State::Offline
                    )
            }
            TrapChildKind::Exception => {
                child == self.exception.flow_ref()
                    && self.exception.state() == State::Prepared
                    && self.active_exception_leaf_alive(generation)
            }
            TrapChildKind::None => false,
        }
    }

    fn active_exception_leaf_alive(&self, generation: u32) -> bool {
        let leaf = self.exception.active_child();
        if leaf.generation() != generation {
            return false;
        }
        let alive = |state| {
            matches!(
                state,
                State::Prepared | State::Ready | State::Online | State::Offline
            )
        };
        match leaf.kind() {
            super::exception_flow_type::ExceptionChildKind::PageFault => {
                leaf == self.page_fault.flow_ref() && alive(self.page_fault.state())
            }
            super::exception_flow_type::ExceptionChildKind::Syscall => {
                leaf == self.syscall.flow_ref() && alive(self.syscall.state())
            }
            super::exception_flow_type::ExceptionChildKind::Breakpoint => {
                leaf == self.breakpoint.flow_ref() && alive(self.breakpoint.state())
            }
            super::exception_flow_type::ExceptionChildKind::Unexpected => {
                leaf == self.unexpected.flow_ref() && alive(self.unexpected.state())
            }
            super::exception_flow_type::ExceptionChildKind::None => false,
        }
    }
}

unsafe fn record_ref(root_ref: TrapFlowRef) -> Option<&'static TrapExecutionRecord> {
    if !root_ref.is_valid()
        || !root_ref
            .address()
            .is_multiple_of(core::mem::align_of::<TrapExecutionRecord>())
    {
        return None;
    }
    let record = unsafe { &*(root_ref.address() as *const TrapExecutionRecord) };
    (core::ptr::addr_of!(record.root) as usize == root_ref.address()
        && record.root.resolves(root_ref))
    .then_some(record)
}

unsafe fn root_mut(root_ref: TrapFlowRef) -> Option<&'static mut TrapFlowType> {
    if !root_ref.is_valid()
        || !root_ref
            .address()
            .is_multiple_of(core::mem::align_of::<TrapFlowType>())
    {
        return None;
    }
    let root = unsafe { &mut *(root_ref.address() as *mut TrapFlowType) };
    root.resolves(root_ref).then_some(root)
}

/// Exercises the dynamic occurrence and generation contracts without relying
/// on architecture entry. The live breakpoint smoke that calls this helper
/// then covers the same record through the real trap assembly path.
#[cfg(app_smoke)]
pub fn smoke_occurrence_contract() -> bool {
    let cpu_ref = CpuRef::new(0);
    let snapshot = TrapEntrySnapshot {
        cause_class: TrapCauseClass::Interrupt,
        scause: 1usize << (usize::BITS as usize - 1),
        sepc: 0x1000,
        sstatus: 0x2000,
        stval: 0x3000,
        entry_task: TaskRef::BOOT,
        effective_task_flow: TaskFlowRef::BOOT_INIT,
        context_epoch: 1,
    };

    let mut interrupt_record = TrapExecutionRecord::new();
    let root_address = core::ptr::addr_of!(interrupt_record.root) as usize;
    let interrupt_address = core::ptr::addr_of!(interrupt_record.interrupt) as usize;
    if interrupt_record
        .root
        .declare_and_bind(7, root_address, cpu_ref, true)
        .is_err()
        || interrupt_record.root.preset(snapshot).is_err()
        || interrupt_record
            .interrupt
            .declare_and_bind(7, interrupt_address, cpu_ref, State::Online)
            .is_err()
        || interrupt_record.interrupt.preset().is_err()
    {
        return false;
    }
    let interrupt_ref = interrupt_record.interrupt.flow_ref();
    if interrupt_record.root.select_child(interrupt_ref).is_err()
        || interrupt_record.interrupt.setup_after_handler().is_err()
        || interrupt_record.interrupt.enable().is_err()
        || interrupt_record.interrupt.disable().is_err()
        || interrupt_record.interrupt.cleanup().is_err()
        || interrupt_record
            .root
            .mark_child_completed(interrupt_ref)
            .is_err()
        || interrupt_record.root.setup().is_err()
    {
        return false;
    }
    let Ok(token) = interrupt_record.root.enable() else {
        return false;
    };
    if interrupt_record.root.enable().is_ok()
        || interrupt_record.root.disable().is_err()
        || interrupt_record.root.cleanup(CpuRef::new(1)).is_ok()
        || interrupt_record.root.state() != State::Offline
        || interrupt_record.root.cleanup(cpu_ref).is_err()
    {
        return false;
    }
    let root_ref = interrupt_record.root.flow_ref();
    let snapshot_matches = interrupt_record
        .root
        .snapshot
        .map(|saved| {
            saved.sepc == snapshot.sepc
                && saved.sstatus == snapshot.sstatus
                && saved.stval == snapshot.stval
        })
        .unwrap_or(false);
    if !root_ref.is_valid()
        || !root_ref.same_identity(TrapFlowRef::new(root_address, 7))
        || interrupt_record.root.active_child().kind() != TrapChildKind::Interrupt
        || interrupt_record.root.parent_cpu() != cpu_ref
        || interrupt_record.root.return_cpu() != cpu_ref
        || interrupt_record.root.alive()
        || interrupt_record.root.resolves(root_ref)
        || !interrupt_record.root.task_flow_paused()
        || !interrupt_record.root.task_flow_resumed()
        || !snapshot_matches
        || token.consume_after_cleanup(&interrupt_record.root) != Some(TRAP_RETURN_TOKEN_MAGIC)
    {
        return false;
    }

    let mut renewed_root = TrapFlowType::new();
    if renewed_root
        .declare_and_bind(8, root_address, cpu_ref, true)
        .is_err()
        || renewed_root.resolves(root_ref)
        || !renewed_root.resolves(TrapFlowRef::new(root_address, 8))
    {
        return false;
    }

    smoke_active_leaf_resume_contract(cpu_ref)
        && smoke_page_fault_occurrence(cpu_ref)
        && smoke_syscall_occurrence(cpu_ref)
        && smoke_breakpoint_occurrence(cpu_ref)
        && smoke_unexpected_occurrence(cpu_ref)
}

#[cfg(app_smoke)]
fn smoke_active_leaf_resume_contract(cpu_ref: CpuRef) -> bool {
    let generation = 15;
    let context_epoch = 9;
    let task_ref = TaskRef::BOOT;
    let flow_ref = TaskFlowRef::BOOT_INIT;
    let mut record = TrapExecutionRecord::new();
    let root_address = core::ptr::addr_of!(record.root) as usize;
    let exception_address = core::ptr::addr_of!(record.exception) as usize;
    let leaf_address = core::ptr::addr_of!(record.page_fault) as usize;
    if record
        .root
        .declare_and_bind(generation, root_address, cpu_ref, true)
        .is_err()
        || record
            .root
            .preset(TrapEntrySnapshot {
                cause_class: TrapCauseClass::Exception,
                scause: 13,
                sepc: 0x5000,
                sstatus: 0,
                stval: 0x6000,
                entry_task: task_ref,
                effective_task_flow: flow_ref,
                context_epoch,
            })
            .is_err()
        || record
            .exception
            .declare_and_bind(generation, exception_address, cpu_ref, State::Ready)
            .is_err()
        || record.exception.preset(13).is_err()
    {
        return false;
    }
    let exception_ref = record.exception.flow_ref();
    if record.root.select_child(exception_ref).is_err()
        || record
            .page_fault
            .declare_and_bind(generation, leaf_address, cpu_ref, State::Online)
            .is_err()
        || record.page_fault.preset(13, 0x6000, true).is_err()
    {
        return false;
    }
    let leaf_ref = record.page_fault.flow_ref();
    if record.exception.select_child(leaf_ref).is_err()
        || record
            .page_fault
            .setup_context(false, false, true, None)
            .is_err()
    {
        return false;
    }

    let root_ref = record.root.flow_ref();
    let stale_root = TrapFlowRef::new(root_address, generation + 1);
    let stale_flow = flow_ref.with_generation_for_test(flow_ref.generation() + 1);
    if TrapFlowType::record_suspended_context(
        root_ref,
        TaskRef::KERNEL_INIT,
        flow_ref,
        cpu_ref,
        context_epoch,
    ) || TrapFlowType::record_suspended_context(
        root_ref,
        task_ref,
        stale_flow,
        cpu_ref,
        context_epoch,
    ) || TrapFlowType::record_suspended_context(
        root_ref,
        task_ref,
        flow_ref,
        CpuRef::new(1),
        context_epoch,
    ) || TrapFlowType::record_suspended_context(root_ref, task_ref, flow_ref, cpu_ref, 0)
        || !TrapFlowType::record_suspended_context(
            root_ref,
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch,
        )
        || !TrapFlowType::active_leaf_preflight(
            root_ref,
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch,
        )
        || TrapFlowType::active_leaf_preflight(
            stale_root,
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch,
        )
        || TrapFlowType::active_leaf_preflight(
            root_ref,
            TaskRef::KERNEL_INIT,
            flow_ref,
            cpu_ref,
            context_epoch,
        )
        || TrapFlowType::active_leaf_preflight(
            root_ref,
            task_ref,
            stale_flow,
            cpu_ref,
            context_epoch,
        )
        || TrapFlowType::active_leaf_preflight(
            root_ref,
            task_ref,
            flow_ref,
            CpuRef::new(1),
            context_epoch,
        )
        || TrapFlowType::active_leaf_preflight(
            root_ref,
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch + 1,
        )
        || !TrapFlowType::resume_active_leaf(root_ref, task_ref, flow_ref, cpu_ref, context_epoch)
        || TrapFlowType::resume_active_leaf(root_ref, task_ref, flow_ref, cpu_ref, context_epoch)
        || !TrapFlowType::active_leaf_resumed(root_ref, task_ref, flow_ref, cpu_ref, context_epoch)
    {
        return false;
    }

    record.page_fault.enable_after_handler().is_ok()
        && record.page_fault.disable().is_ok()
        && record.page_fault.cleanup().is_ok()
        && !TrapFlowType::active_leaf_preflight(
            root_ref,
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch,
        )
}

#[cfg(app_smoke)]
fn smoke_page_fault_occurrence(cpu_ref: CpuRef) -> bool {
    let mut record = TrapExecutionRecord::new();
    let exception_address = core::ptr::addr_of!(record.exception) as usize;
    let child_address = core::ptr::addr_of!(record.page_fault) as usize;
    if record
        .exception
        .declare_and_bind(11, exception_address, cpu_ref, State::Ready)
        .is_err()
        || record.exception.preset(13).is_err()
        || record
            .page_fault
            .declare_and_bind(11, child_address, cpu_ref, State::Online)
            .is_err()
        || record.page_fault.preset(13, 0x4000, true).is_err()
    {
        return false;
    }
    let child_ref = record.page_fault.flow_ref();
    record.exception.select_child(child_ref).is_ok()
        && record
            .page_fault
            .setup_context(false, false, true, None)
            .is_ok()
        && record.page_fault.schedulable()
        && record.page_fault.parent_cpu() == cpu_ref
        && record.page_fault.enable_after_handler().is_ok()
        && record.page_fault.disable().is_ok()
        && record.page_fault.cleanup().is_ok()
        && record.page_fault.state() == State::Destroyed
        && child_ref.kind() == ExceptionChildKind::PageFault
        && finish_smoke_exception(&mut record.exception, child_ref)
}

#[cfg(app_smoke)]
fn smoke_syscall_occurrence(cpu_ref: CpuRef) -> bool {
    let mut record = TrapExecutionRecord::new();
    let exception_address = core::ptr::addr_of!(record.exception) as usize;
    let child_address = core::ptr::addr_of!(record.syscall) as usize;
    if record
        .exception
        .declare_and_bind(12, exception_address, cpu_ref, State::Ready)
        .is_err()
        || record.exception.preset(8).is_err()
        || record
            .syscall
            .declare_and_bind(12, child_address, cpu_ref, State::Online)
            .is_err()
        || record.syscall.preset().is_err()
    {
        return false;
    }
    let child_ref = record.syscall.flow_ref();
    record.exception.select_child(child_ref).is_ok()
        && record.syscall.setup().is_ok()
        && record.syscall.may_schedule_and_migrate()
        && record.syscall.enable_after_handler().is_ok()
        && record.syscall.disable().is_ok()
        && record.syscall.cleanup().is_ok()
        && record.syscall.state() == State::Destroyed
        && child_ref.kind() == ExceptionChildKind::Syscall
        && finish_smoke_exception(&mut record.exception, child_ref)
}

#[cfg(app_smoke)]
fn smoke_breakpoint_occurrence(cpu_ref: CpuRef) -> bool {
    let mut record = TrapExecutionRecord::new();
    let exception_address = core::ptr::addr_of!(record.exception) as usize;
    let child_address = core::ptr::addr_of!(record.breakpoint) as usize;
    if record
        .exception
        .declare_and_bind(13, exception_address, cpu_ref, State::Ready)
        .is_err()
        || record.exception.preset(3).is_err()
        || record
            .breakpoint
            .declare_and_bind(13, child_address, cpu_ref, State::Online)
            .is_err()
        || record.breakpoint.preset().is_err()
    {
        return false;
    }
    let child_ref = record.breakpoint.flow_ref();
    record.exception.select_child(child_ref).is_ok()
        && record.breakpoint.setup_after_hook().is_ok()
        && record.breakpoint.enable().is_ok()
        && record.breakpoint.disable().is_ok()
        && record.breakpoint.cleanup().is_ok()
        && record.breakpoint.state() == State::Destroyed
        && child_ref.kind() == ExceptionChildKind::Breakpoint
        && finish_smoke_exception(&mut record.exception, child_ref)
}

#[cfg(app_smoke)]
fn smoke_unexpected_occurrence(cpu_ref: CpuRef) -> bool {
    let mut record = TrapExecutionRecord::new();
    let exception_address = core::ptr::addr_of!(record.exception) as usize;
    let child_address = core::ptr::addr_of!(record.unexpected) as usize;
    if record
        .exception
        .declare_and_bind(14, exception_address, cpu_ref, State::Ready)
        .is_err()
        || record.exception.preset(2).is_err()
        || record
            .unexpected
            .declare_and_bind(14, child_address, cpu_ref, State::Online)
            .is_err()
        || record.unexpected.preset().is_err()
    {
        return false;
    }
    let child_ref = record.unexpected.flow_ref();
    record.exception.select_child(child_ref).is_ok()
        && record.unexpected.setup().is_ok()
        && record.unexpected.enable().is_ok()
        && record.unexpected.disable().is_ok()
        && record.unexpected.cleanup().is_ok()
        && record.unexpected.state() == State::Destroyed
        && child_ref.kind() == ExceptionChildKind::Unexpected
        && finish_smoke_exception(&mut record.exception, child_ref)
}

#[cfg(app_smoke)]
fn finish_smoke_exception(
    exception: &mut ExceptionFlowType,
    child_ref: ExceptionChildFlowRef,
) -> bool {
    exception.mark_child_completed(child_ref).is_ok()
        && exception.setup().is_ok()
        && exception.enable().is_ok()
        && exception.disable().is_ok()
        && exception.cleanup().is_ok()
        && exception.state() == State::Destroyed
        && exception.active_child() == child_ref
}
