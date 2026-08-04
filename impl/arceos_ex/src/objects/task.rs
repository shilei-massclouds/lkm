use crate::{arch::riscv64::task_switch::TaskSwitchContext, checkpoint::Checkpoint};

use super::{
    state::{EventResult, FailureDiagnostic, Lifecycle, LifecycleEvent, State, failed_condition},
    task_flow::{TaskFlow, TaskFlowRef},
    trap_flow_type::{TrapFlowRef, TrapFlowType},
};

pub const USER_TASK_SLOT_COUNT: usize = 8;
pub const KERNEL_TASK_SLOT_COUNT: usize = 8;
pub const TASK_STACK_GUARD_VALUE: usize = 0x57AC6E9D;

const TASK_SLOT_BOOT: u16 = 1;
const TASK_SLOT_KERNEL_INIT: u16 = 2;
const TASK_SLOT_KTHREADD: u16 = 3;
const TASK_SLOT_SMOKE_SCHEDULER: u16 = 4;
const TASK_SLOT_SMOKE_MUTEX: u16 = 5;
const TASK_SLOT_SMOKE_RWSEM: u16 = 6;
const TASK_SLOT_SMOKE_RWLOCK: u16 = 7;
const TASK_SLOT_AP_IDLE_BASE: u16 = 16;
const TASK_SLOT_USER_BASE: u16 = 32;
const TASK_SLOT_KERNEL_BASE: u16 = 48;

/// Stable identity for a Task storage occurrence.
///
/// Slot numbers are deliberately private.  Callers can observe them for
/// diagnostics, but only the storage owner can construct a dynamic ref.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TaskRef {
    slot: u16,
    generation: u32,
}

impl TaskRef {
    pub const NONE: Self = Self::new(0, 0);
    pub const BOOT: Self = Self::new(TASK_SLOT_BOOT, 1);
    pub const KERNEL_INIT: Self = Self::new(TASK_SLOT_KERNEL_INIT, 1);
    pub const KTHREADD: Self = Self::new(TASK_SLOT_KTHREADD, 1);
    pub const SMOKE_SCHEDULER: Self = Self::new(TASK_SLOT_SMOKE_SCHEDULER, 1);
    pub const SMOKE_MUTEX: Self = Self::new(TASK_SLOT_SMOKE_MUTEX, 1);
    pub const SMOKE_RWSEM: Self = Self::new(TASK_SLOT_SMOKE_RWSEM, 1);
    pub const SMOKE_RWLOCK: Self = Self::new(TASK_SLOT_SMOKE_RWLOCK, 1);

    const fn new(slot: u16, generation: u32) -> Self {
        Self { slot, generation }
    }

    pub(crate) const fn ap_idle(logical_id: usize) -> Self {
        Self::new(TASK_SLOT_AP_IDLE_BASE + logical_id as u16, 1)
    }

    pub(crate) const fn user(slot: usize, generation: u32) -> Self {
        Self::new(TASK_SLOT_USER_BASE + slot as u16, generation)
    }

    pub(crate) const fn kernel(slot: usize, generation: u32) -> Self {
        Self::new(TASK_SLOT_KERNEL_BASE + slot as u16, generation)
    }

    pub const fn is_valid(self) -> bool {
        self.slot != 0 && self.generation != 0
    }

    #[allow(dead_code)]
    pub const fn slot(self) -> usize {
        self.slot as usize
    }

    #[cfg(app_smoke)]
    pub(crate) const fn with_generation_for_test(self, generation: u32) -> Self {
        Self::new(self.slot, generation)
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn same_identity(self, other: Self) -> bool {
        self.slot == other.slot && self.generation == other.generation
    }

    pub const fn is_user(self) -> bool {
        self.slot >= TASK_SLOT_USER_BASE
            && self.slot < TASK_SLOT_USER_BASE + USER_TASK_SLOT_COUNT as u16
    }

    pub const fn is_kernel(self) -> bool {
        self.slot >= TASK_SLOT_KERNEL_BASE
            && self.slot < TASK_SLOT_KERNEL_BASE + KERNEL_TASK_SLOT_COUNT as u16
    }

    pub const fn is_ap_idle(self) -> bool {
        self.slot >= TASK_SLOT_AP_IDLE_BASE && self.slot < TASK_SLOT_AP_IDLE_BASE + 8
    }

    pub const fn is_scheduler_ref(self) -> bool {
        self.is_boot_scheduler_ref() || self.is_ap_idle() || self.is_kernel()
    }

    pub const fn is_boot_scheduler_ref(self) -> bool {
        self.same_identity(Self::BOOT)
            || self.same_identity(Self::KERNEL_INIT)
            || self.same_identity(Self::KTHREADD)
            || self.same_identity(Self::SMOKE_SCHEDULER)
            || self.same_identity(Self::SMOKE_MUTEX)
            || self.same_identity(Self::SMOKE_RWSEM)
            || self.same_identity(Self::SMOKE_RWLOCK)
            || self.is_user()
    }

    pub const fn user_slot(self) -> Option<usize> {
        if self.is_user() {
            Some((self.slot - TASK_SLOT_USER_BASE) as usize)
        } else {
            None
        }
    }

    pub const fn kernel_slot(self) -> Option<usize> {
        if self.is_kernel() {
            Some((self.slot - TASK_SLOT_KERNEL_BASE) as usize)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub const fn name(self) -> &'static str {
        match self.slot {
            0 => "None",
            TASK_SLOT_BOOT => "BootTask",
            TASK_SLOT_KERNEL_INIT => "KernelInitTask",
            TASK_SLOT_KTHREADD => "KthreaddTask",
            TASK_SLOT_SMOKE_SCHEDULER => "SmokeSchedulerTask",
            TASK_SLOT_SMOKE_MUTEX => "SmokeMutexTask",
            TASK_SLOT_SMOKE_RWSEM => "SmokeRwsemTask",
            TASK_SLOT_SMOKE_RWLOCK => "SmokeRwLockTask",
            TASK_SLOT_AP_IDLE_BASE..=23 => "ApIdleTask",
            TASK_SLOT_USER_BASE..=39 => "UserTask",
            TASK_SLOT_KERNEL_BASE..=55 => "KernelTask",
            _ => "UnknownTask",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskEntry {
    None,
    BootIdle,
    KernelInit,
    Kthreadd,
    UserChild,
    ApIdle,
    SmokeScheduler,
    #[cfg_attr(not(app_smoke), allow(dead_code))]
    KernelTask,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskKind {
    None,
    Idle,
    UserModeThread,
    KernelThread,
    TestOnly,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskExecutionAuthority {
    None,
    Reserved,
    Live,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskBreakpointState {
    Invalid,
    Prepared,
    Valid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskTerminalReason {
    None,
    OutOfMemory,
    SegmentationFault,
}

pub const TASK_SIGNAL_SEGMENTATION_FAULT: usize = 11;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskSegvCode {
    Maperr,
    Accerr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskSegvInfo {
    code: TaskSegvCode,
    address: usize,
}

impl TaskSegvInfo {
    pub const fn new(code: TaskSegvCode, address: usize) -> Self {
        Self { code, address }
    }

    pub const fn signal(self) -> usize {
        TASK_SIGNAL_SEGMENTATION_FAULT
    }

    pub const fn code(self) -> TaskSegvCode {
        self.code
    }

    pub const fn address(self) -> usize {
        self.address
    }
}

/// The physical core-register context and its recoverable continuation
/// metadata. This wrapper is owned by exactly one Task.
pub struct TaskThreadContext {
    arch: TaskSwitchContext,
    breakpoint_state: TaskBreakpointState,
    flow_ref: TaskFlowRef,
    core_saved_count: usize,
    core_restored_count: usize,
    context_epoch: u64,
    next_dispatch_ordinal: u64,
    dispatch_record: Option<DispatchRecord>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct DispatchRecord {
    task_ref: TaskRef,
    flow_ref: TaskFlowRef,
    cpu_ref: super::cpu::CpuRef,
    context_epoch: u64,
    ordinal: u64,
    entered: bool,
}

/// Single-use capability created only by `Task.Dispatch` and consumed by the
/// immediately following contextual TaskFlow Enter.
pub(crate) struct TaskFlowEnterProof {
    record: DispatchRecord,
}

impl TaskThreadContext {
    pub const fn new() -> Self {
        Self {
            arch: TaskSwitchContext::new(),
            breakpoint_state: TaskBreakpointState::Invalid,
            flow_ref: TaskFlowRef::NONE,
            core_saved_count: 0,
            core_restored_count: 0,
            context_epoch: 0,
            next_dispatch_ordinal: 0,
            dispatch_record: None,
        }
    }

    pub const fn arch(&self) -> &TaskSwitchContext {
        &self.arch
    }

    pub fn arch_mut(&mut self) -> &mut TaskSwitchContext {
        &mut self.arch
    }

    pub const fn breakpoint_state(&self) -> TaskBreakpointState {
        self.breakpoint_state
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.flow_ref
    }

    pub const fn root_trap_flow_ref(&self) -> TrapFlowRef {
        self.arch.root_trap_flow_ref()
    }

    pub const fn core_register_set(&self) -> bool {
        self.arch.initialized()
    }

    pub const fn core_saved_count(&self) -> usize {
        self.core_saved_count
    }

    pub const fn core_restored_count(&self) -> usize {
        self.core_restored_count
    }

    pub const fn context_epoch(&self) -> u64 {
        self.context_epoch
    }

    fn prepare(&mut self) {
        self.breakpoint_state = TaskBreakpointState::Prepared;
        self.flow_ref = TaskFlowRef::NONE;
        self.context_epoch = 1;
        self.dispatch_record = None;
    }

    fn publish(&mut self, flow_ref: TaskFlowRef) {
        self.breakpoint_state = TaskBreakpointState::Valid;
        self.flow_ref = flow_ref;
    }

    fn consume(&mut self) {
        self.breakpoint_state = TaskBreakpointState::Invalid;
        self.flow_ref = TaskFlowRef::NONE;
        self.core_restored_count = self.core_restored_count.wrapping_add(1);
    }

    fn record_save(&mut self) {
        self.core_saved_count = self.core_saved_count.wrapping_add(1);
        self.context_epoch = self.context_epoch.wrapping_add(1).max(1);
        self.dispatch_record = None;
    }

    fn create_dispatch_record(
        &mut self,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: super::cpu::CpuRef,
    ) -> DispatchRecord {
        self.next_dispatch_ordinal = self.next_dispatch_ordinal.wrapping_add(1).max(1);
        let record = DispatchRecord {
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch: self.context_epoch,
            ordinal: self.next_dispatch_ordinal,
            entered: false,
        };
        self.dispatch_record = Some(record);
        record
    }

    fn root_trap_flow_resolves(&self) -> bool {
        let root_ref = self.root_trap_flow_ref();
        if !root_ref.is_valid()
            || !root_ref
                .address()
                .is_multiple_of(core::mem::align_of::<TrapFlowType>())
        {
            return false;
        }

        // A root ref is installed before the stack-local occurrence becomes
        // reachable from a scheduling point and is cleared before that stack
        // object is released. Therefore a still-installed ref may be
        // generation-checked while its Task is awaiting dispatch or handling a
        // nested trap.
        let root = unsafe { &*(root_ref.address() as *const TrapFlowType) };
        root.resolves(root_ref)
    }

    /// Returns `Some(true)` when this occurrence installed the Task root and
    /// `Some(false)` when it is nested beneath an already-live root.
    fn bind_root_trap_flow(&mut self, root_ref: TrapFlowRef) -> Result<bool, &'static str> {
        if !root_ref.is_valid() {
            return Err("root_ref_invalid");
        }
        if self.root_trap_flow_ref().is_valid() {
            if self.root_trap_flow_ref().same_identity(root_ref) {
                return Err("root_ref_already_installed");
            }
            if !self.root_trap_flow_resolves() {
                return Err("installed_root_stale");
            }
            return Ok(false);
        }
        self.arch.set_root_trap_flow_ref(root_ref);
        Ok(true)
    }

    fn clear_root_trap_flow(&mut self, root_ref: TrapFlowRef) -> bool {
        if !root_ref.is_valid() || !self.root_trap_flow_ref().same_identity(root_ref) {
            return false;
        }
        self.arch.set_root_trap_flow_ref(TrapFlowRef::NONE);
        true
    }
}

/// The one lifecycle/identity/PID/CPU/context/Flow-ownership carrier.
pub struct Task {
    lifecycle: Lifecycle,
    on_cpu: bool,
    execution_authority: TaskExecutionAuthority,
    task_ref: TaskRef,
    entry: TaskEntry,
    pid: usize,
    kind: TaskKind,
    running: bool,
    runqueue_published: bool,
    scheduler_sleep_declared: bool,
    pending_wake_signal: bool,
    core_save_pending_suspend: bool,
    affinity_cpu_id: usize,
    no_setaffinity: bool,
    stack_guard_installed: bool,
    terminal_reason: TaskTerminalReason,
    segv_info: Option<TaskSegvInfo>,
    thread_context: TaskThreadContext,
    flow: TaskFlow,
}

impl Task {
    const fn initial_flow(task_ref: TaskRef) -> TaskFlow {
        match task_ref.slot {
            TASK_SLOT_BOOT => TaskFlow::new_static_bound(TaskFlowRef::BOOT_INIT, task_ref),
            TASK_SLOT_KERNEL_INIT => TaskFlow::new_static_bound(TaskFlowRef::KERNEL_INIT, task_ref),
            TASK_SLOT_KTHREADD => TaskFlow::new_static_bound(TaskFlowRef::KTHREADD, task_ref),
            TASK_SLOT_SMOKE_SCHEDULER => {
                TaskFlow::new_static_bound(TaskFlowRef::smoke(0), task_ref)
            }
            TASK_SLOT_SMOKE_MUTEX => TaskFlow::new_static_bound(TaskFlowRef::smoke(1), task_ref),
            TASK_SLOT_SMOKE_RWSEM => TaskFlow::new_static_bound(TaskFlowRef::smoke(2), task_ref),
            TASK_SLOT_SMOKE_RWLOCK => TaskFlow::new_static_bound(TaskFlowRef::smoke(3), task_ref),
            TASK_SLOT_USER_BASE..=39 => {
                TaskFlow::new_user((task_ref.slot - TASK_SLOT_USER_BASE) as usize)
            }
            TASK_SLOT_KERNEL_BASE..=55 => {
                TaskFlow::new_kernel((task_ref.slot - TASK_SLOT_KERNEL_BASE) as usize)
            }
            _ => TaskFlow::new_static(TaskFlowRef::NONE),
        }
    }

    pub const fn new() -> Self {
        Self::with_ref(TaskRef::NONE)
    }

    pub const fn with_ref(task_ref: TaskRef) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            on_cpu: false,
            execution_authority: TaskExecutionAuthority::None,
            task_ref,
            entry: TaskEntry::None,
            pid: 0,
            kind: TaskKind::None,
            running: false,
            runqueue_published: false,
            scheduler_sleep_declared: false,
            pending_wake_signal: false,
            core_save_pending_suspend: false,
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            stack_guard_installed: false,
            terminal_reason: TaskTerminalReason::None,
            segv_info: None,
            thread_context: TaskThreadContext::new(),
            flow: Self::initial_flow(task_ref),
        }
    }

    pub const fn new_user(task_ref: TaskRef, flow_generation: u32) -> Self {
        let mut task = Self::with_ref(task_ref);
        let slot = match task_ref.user_slot() {
            Some(slot) => slot,
            None => 0,
        };
        task.flow = TaskFlow::new_user_with_generation(slot, flow_generation);
        task
    }

    pub const fn new_kernel(task_ref: TaskRef, flow_generation: u32) -> Self {
        let mut task = Self::with_ref(task_ref);
        let slot = match task_ref.kernel_slot() {
            Some(slot) => slot,
            None => 0,
        };
        task.flow = TaskFlow::new_kernel_with_generation(slot, flow_generation);
        task
    }

    /// Boot-only image initializer for the pre-existing PID 0 carrier.
    ///
    /// This is deliberately separate from the reusable Task lifecycle: the
    /// linker-visible init task exists before `_start` and never performs
    /// Preset/Setup/Enable.
    pub(crate) const fn new_boot_on_cpu() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::OnCpu),
            on_cpu: true,
            execution_authority: TaskExecutionAuthority::Live,
            task_ref: TaskRef::BOOT,
            entry: TaskEntry::None,
            pid: 0,
            kind: TaskKind::None,
            running: true,
            runqueue_published: false,
            scheduler_sleep_declared: false,
            pending_wake_signal: false,
            core_save_pending_suspend: false,
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            stack_guard_installed: false,
            terminal_reason: TaskTerminalReason::None,
            segv_info: None,
            thread_context: TaskThreadContext::new(),
            flow: TaskFlow::new_static_bound(TaskFlowRef::BOOT_INIT, TaskRef::BOOT),
        }
    }

    /// init_idle()-equivalent AP reservation. The carrier is already the
    /// CPU's current/idle Task, while HSM has not yet granted Live authority.
    pub(crate) const fn new_ap_idle_reserved(
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        _logical_id: usize,
    ) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::OnCpu),
            on_cpu: true,
            execution_authority: TaskExecutionAuthority::Reserved,
            task_ref,
            entry: TaskEntry::ApIdle,
            pid: 0,
            kind: TaskKind::Idle,
            running: true,
            runqueue_published: false,
            scheduler_sleep_declared: false,
            pending_wake_signal: false,
            core_save_pending_suspend: false,
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            stack_guard_installed: false,
            terminal_reason: TaskTerminalReason::None,
            segv_info: None,
            thread_context: TaskThreadContext::new(),
            flow: TaskFlow::new_static_bound(flow_ref, task_ref),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    /// True while the Task remains scheduler-published, independently of its
    /// current execution ownership.
    pub const fn online(&self) -> bool {
        matches!(self.lifecycle.state(), State::Online | State::OnCpu)
    }

    pub const fn execution_authority(&self) -> TaskExecutionAuthority {
        self.execution_authority
    }

    pub const fn breakpoint_state(&self) -> TaskBreakpointState {
        self.thread_context.breakpoint_state()
    }

    pub const fn breakpoint_flow(&self) -> TaskFlowRef {
        self.thread_context.flow_ref()
    }

    pub const fn breakpoint_matches(&self, flow_ref: TaskFlowRef) -> bool {
        matches!(
            self.thread_context.breakpoint_state(),
            TaskBreakpointState::Valid
        ) && self.breakpoint_flow().same_identity(flow_ref)
    }

    pub const fn thread_context(&self) -> &TaskThreadContext {
        &self.thread_context
    }

    pub const fn root_trap_flow_ref(&self) -> TrapFlowRef {
        self.thread_context.root_trap_flow_ref()
    }

    pub(crate) fn bind_root_trap_flow(
        &mut self,
        effective_flow_ref: TaskFlowRef,
        root_ref: TrapFlowRef,
    ) -> Result<bool, &'static str> {
        if self.lifecycle.state() != State::OnCpu {
            return Err("task_not_on_cpu");
        }
        if !self.on_cpu {
            return Err("task_on_cpu_flag_false");
        }
        if self.execution_authority != TaskExecutionAuthority::Live {
            return Err("task_authority_not_live");
        }
        if self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid {
            return Err("task_breakpoint_not_invalid");
        }
        if !effective_flow_ref.is_valid() || !self.owns_flow(effective_flow_ref) {
            return Err("task_effective_flow_invalid_or_not_owned");
        }
        if !self.flow.flow_ref().same_identity(effective_flow_ref) {
            return Err("task_effective_flow_not_fixed");
        }
        self.thread_context.bind_root_trap_flow(root_ref)
    }

    pub(crate) fn root_trap_flow_resolves(&self) -> bool {
        self.root_trap_flow_ref().is_valid() && self.thread_context.root_trap_flow_resolves()
    }

    pub(crate) fn clear_root_trap_flow(&mut self, root_ref: TrapFlowRef) -> bool {
        self.thread_context.clear_root_trap_flow(root_ref)
    }

    pub const fn task_ref(&self) -> TaskRef {
        self.task_ref
    }

    pub const fn entry(&self) -> TaskEntry {
        self.entry
    }

    pub const fn pid(&self) -> usize {
        self.pid
    }

    pub const fn kind(&self) -> TaskKind {
        self.kind
    }

    pub const fn running(&self) -> bool {
        self.running
    }

    pub const fn runqueue_published(&self) -> bool {
        self.runqueue_published
    }

    pub const fn terminal_reason(&self) -> TaskTerminalReason {
        self.terminal_reason
    }

    pub const fn segv_info(&self) -> Option<TaskSegvInfo> {
        self.segv_info
    }

    pub const fn terminal_wait_status(&self, exit_status: usize) -> usize {
        match self.terminal_reason {
            TaskTerminalReason::OutOfMemory => 9,
            TaskTerminalReason::SegmentationFault => TASK_SIGNAL_SEGMENTATION_FAULT,
            TaskTerminalReason::None => (exit_status & 0xff) << 8,
        }
    }

    pub fn record_out_of_memory_terminal(&mut self) -> bool {
        if self.lifecycle.state() != State::OnCpu
            || self.execution_authority != TaskExecutionAuthority::Live
            || self.terminal_reason != TaskTerminalReason::None
        {
            return false;
        }
        self.terminal_reason = TaskTerminalReason::OutOfMemory;
        true
    }

    pub fn record_segmentation_fault_terminal(
        &mut self,
        code: TaskSegvCode,
        address: usize,
    ) -> bool {
        if self.lifecycle.state() != State::OnCpu
            || self.execution_authority != TaskExecutionAuthority::Live
            || self.terminal_reason != TaskTerminalReason::None
            || self.segv_info.is_some()
        {
            return false;
        }
        self.terminal_reason = TaskTerminalReason::SegmentationFault;
        self.segv_info = Some(TaskSegvInfo::new(code, address));
        true
    }

    pub const fn scheduler_sleep_declared(&self) -> bool {
        self.scheduler_sleep_declared
    }

    pub const fn pending_wake_signal(&self) -> bool {
        self.pending_wake_signal
    }

    /// Publish the task state consumed by the next non-preemptive
    /// `PreparePrev`.  Lifecycle and execution authority remain unchanged.
    pub(crate) fn declare_scheduler_sleep(&mut self) -> EventResult {
        if self.state() != State::OnCpu || !self.running {
            return failed_condition(
                LifecycleEvent::Setup,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            );
        }
        self.scheduler_sleep_declared = true;
        self.running = false;
        Ok(())
    }

    /// Record a matching wake signal. `PreparePrev` consumes it exactly once.
    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub(crate) fn post_pending_wake_signal(&mut self) -> EventResult {
        if self.lifecycle.state() != State::OnCpu || self.pending_wake_signal {
            return failed_condition(
                LifecycleEvent::Dispatch,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            );
        }
        self.pending_wake_signal = true;
        Ok(())
    }

    /// Returns true when `prev` must retain runnable eligibility.
    pub(crate) fn prepare_prev_runnable(&mut self) -> bool {
        if !self.scheduler_sleep_declared {
            return self.running;
        }
        if self.pending_wake_signal {
            self.pending_wake_signal = false;
            self.scheduler_sleep_declared = false;
            self.running = true;
            return true;
        }
        false
    }

    pub(crate) fn deactivate_from_scheduler(&mut self) -> EventResult {
        if self.state() != State::OnCpu || !self.scheduler_sleep_declared || self.running {
            return failed_condition(
                LifecycleEvent::Setup,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            );
        }
        self.runqueue_published = false;
        Ok(())
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub(crate) fn wake_for_scheduler_enqueue(&mut self) -> EventResult {
        if self.state() != State::Online || self.on_cpu || !self.scheduler_sleep_declared {
            return failed_condition(
                LifecycleEvent::Dispatch,
                self.state(),
                State::Online,
                State::Online,
            );
        }
        self.scheduler_sleep_declared = false;
        self.pending_wake_signal = false;
        self.running = true;
        self.runqueue_published = true;
        Ok(())
    }

    pub const fn affinity_pinned(&self) -> bool {
        self.affinity_cpu_id != usize::MAX
    }

    pub const fn no_setaffinity(&self) -> bool {
        self.no_setaffinity
    }

    pub const fn flow(&self) -> TaskFlowRef {
        self.flow.flow_ref()
    }

    pub const fn owns_flow(&self, flow_ref: TaskFlowRef) -> bool {
        self.flow.flow_ref().same_identity(flow_ref)
            && self.flow.owner().same_identity(self.task_ref)
    }

    pub const fn embedded_flow(&self) -> &TaskFlow {
        &self.flow
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.flow.flow_ref()
    }

    pub const fn flow_state(&self) -> State {
        self.flow.state()
    }

    pub const fn flow_cpu_ref(&self) -> Option<super::cpu::CpuRef> {
        self.flow.cpu_ref()
    }

    pub fn bind_flow_cpu_ref(&mut self, cpu_ref: super::cpu::CpuRef) -> bool {
        self.flow.bind_cpu_ref(cpu_ref)
    }

    pub fn commit_flow_cpu_ref(&mut self, cpu_ref: super::cpu::CpuRef) -> bool {
        self.flow.commit_cpu_ref(cpu_ref)
    }

    pub fn declare_and_bind_embedded_flow(&mut self) -> EventResult {
        if !self.flow.declared() {
            self.flow.declare()?;
        }
        if self.flow.owner().is_valid() {
            return if self.flow.owner().same_identity(self.task_ref) {
                Ok(())
            } else {
                failed_condition(
                    LifecycleEvent::Preset,
                    self.state(),
                    self.state(),
                    self.state(),
                )
            };
        }
        self.flow.bind_owner(self.task_ref)
    }

    pub fn publish_embedded_flow(&mut self) -> EventResult {
        let owner_ref = self.task_ref;
        self.flow.preset_owned(owner_ref, None)?;
        self.flow.setup_owned(owner_ref, None)?;
        self.flow.enable_owned(owner_ref, None)
    }

    pub fn preset_embedded_flow(&mut self, checkpoint: Checkpoint) -> EventResult {
        self.flow.preset_owned(self.task_ref, Some(checkpoint))
    }

    pub fn setup_embedded_flow(&mut self, checkpoint: Checkpoint) -> EventResult {
        self.flow.setup_owned(self.task_ref, Some(checkpoint))
    }

    pub fn enable_embedded_flow(&mut self, checkpoint: Checkpoint) -> EventResult {
        self.flow.enable_owned(self.task_ref, Some(checkpoint))
    }

    pub fn cleanup_embedded_flow(&mut self) -> EventResult {
        self.flow.cleanup_owned(self.task_ref)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub(crate) fn disable_embedded_flow_for_exit(&mut self) -> EventResult {
        if self.state() != State::OnCpu
            || self.execution_authority != TaskExecutionAuthority::Live
            || !self.owns_flow(self.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Disable,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            );
        }
        self.flow.disable_owned(self.task_ref)
    }

    pub const fn stack_guard_installed(&self) -> bool {
        self.stack_guard_installed
    }

    pub fn stack_guard_intact(&self) -> bool {
        let base = self.kernel_stack_base();
        let top = self.kernel_stack_top();
        self.stack_guard_installed
            && self.stack_guard_range_valid(base, top)
            && unsafe { core::ptr::read_volatile(base as *const usize) == TASK_STACK_GUARD_VALUE }
    }

    pub fn enable_stack_guard(&mut self, base: usize, top: usize) -> EventResult {
        if !self.stack_guard_range_valid(base, top) {
            return self.stack_guard_failed("stack_range_invalid_or_not_owned");
        }
        if self.stack_guard_installed {
            return if self.stack_guard_intact() {
                Ok(())
            } else {
                self.stack_guard_failed("stack_guard_corrupted")
            };
        }

        unsafe { core::ptr::write_volatile(base as *mut usize, TASK_STACK_GUARD_VALUE) };
        self.stack_guard_installed = true;
        Ok(())
    }

    fn stack_guard_range_valid(&self, base: usize, top: usize) -> bool {
        let word_size = core::mem::size_of::<usize>();
        let word_align = core::mem::align_of::<usize>();
        base != 0
            && top != 0
            && top.checked_sub(base).is_some_and(|size| size >= word_size)
            && base.is_multiple_of(word_align)
            && top.is_multiple_of(word_align)
            && self.kernel_stack_base() == base
            && self.kernel_stack_top() == top
    }

    fn stack_guard_failed(&self, first_failed: &'static str) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            self.lifecycle.state(),
            self.lifecycle.state(),
        )
        .map_err(|error| {
            error.with_diagnostic(FailureDiagnostic::new(
                "Task",
                "EnableStackGuard",
                "Task.stack",
                "owned valid range and intact installed guard",
                first_failed,
            ))
        })
    }

    pub fn set_identity_metadata(
        &mut self,
        pid: usize,
        entry: TaskEntry,
        kind: TaskKind,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base || !self.task_ref.is_valid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.pid = pid;
        self.entry = entry;
        self.kind = kind;
        Ok(())
    }

    pub fn preset(&mut self, checkpoint: Checkpoint) -> EventResult {
        if !self.task_ref.is_valid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            checkpoint,
        )
    }

    pub fn adopt_preset(&mut self) -> EventResult {
        if !self.task_ref.is_valid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self, checkpoint: Checkpoint) -> EventResult {
        if self.runqueue_published
            || self.execution_authority != TaskExecutionAuthority::None
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Prepared
            || !self.thread_context.core_register_set()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            checkpoint,
        )
    }

    pub fn adopt_setup(&mut self) -> EventResult {
        if self.execution_authority != TaskExecutionAuthority::None
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Prepared
            || !self.thread_context.core_register_set()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self, checkpoint: Checkpoint) -> EventResult {
        if !self.running
            || !self.runqueue_published
            || !self.flow_ref().is_valid()
            || !self.flow.owner().same_identity(self.task_ref)
            || self.execution_authority != TaskExecutionAuthority::None
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Prepared
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            checkpoint,
        )?;
        self.thread_context.publish(self.flow_ref());
        Ok(())
    }

    pub fn adopt_enable(&mut self) -> EventResult {
        if !self.flow_ref().is_valid()
            || !self.flow.owner().same_identity(self.task_ref)
            || self.execution_authority != TaskExecutionAuthority::None
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Prepared
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.running = true;
        self.runqueue_published = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)?;
        self.thread_context.publish(self.flow_ref());
        Ok(())
    }

    /// Accept the keyed SBI HSM Startup delivery. Task lifecycle and identity
    /// do not change; only the reserved execution authority becomes Live.
    pub(crate) fn activate_hsm_authority(&mut self) -> EventResult {
        if self.lifecycle.state() != State::OnCpu
            || !self.on_cpu
            || self.entry != TaskEntry::ApIdle
            || self.kind != TaskKind::Idle
            || !self.running
            || self.runqueue_published
            || !self.flow_ref().is_valid()
            || self.execution_authority != TaskExecutionAuthority::Reserved
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
        {
            return failed_condition(
                LifecycleEvent::Dispatch,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            );
        }
        self.execution_authority = TaskExecutionAuthority::Live;
        Ok(())
    }

    /// Scheduler-only acceptance of every `Task.Dispatch` delivery. The
    /// returned capability can be consumed only by this Task's embedded Flow.
    pub(crate) fn dispatch_on_cpu(
        &mut self,
        expected_flow: TaskFlowRef,
        expected_context_epoch: u64,
        cpu_ref: super::cpu::CpuRef,
    ) -> Result<TaskFlowEnterProof, super::state::EventError> {
        let first_failed = if self.lifecycle.state() != State::Online {
            Some("lifecycle_online")
        } else if self.on_cpu {
            Some("not_already_on_cpu")
        } else if self.execution_authority != TaskExecutionAuthority::None {
            Some("authority_none")
        } else if self.core_save_pending_suspend {
            Some("core_save_not_pending")
        } else if self.thread_context.breakpoint_state() != TaskBreakpointState::Valid {
            Some("breakpoint_valid")
        } else if !expected_flow.is_valid() {
            Some("flow_valid")
        } else if !self.owns_flow(expected_flow) {
            Some("owns_flow")
        } else if !self.thread_context.flow_ref().same_identity(expected_flow) {
            Some("breakpoint_flow_matches")
        } else if self.thread_context.context_epoch() != expected_context_epoch {
            Some("context_epoch_matches_preflight")
        } else if self.flow.cpu_ref() != Some(cpu_ref) {
            Some("flow_cpu_matches_dispatch_cpu")
        } else if self.thread_context.dispatch_record.is_some() {
            Some("no_unconsumed_dispatch_record")
        } else {
            None
        };
        if let Some(first_failed) = first_failed {
            let error = failed_condition(
                LifecycleEvent::Dispatch,
                self.state(),
                State::Online,
                State::OnCpu,
            )
            .map_err(|error| {
                error.with_diagnostic(FailureDiagnostic::new(
                    "SchedulerTaskSwitch",
                    "dispatch_task_after_switch",
                    "Task",
                    "Task.Dispatch prerequisites",
                    first_failed,
                ))
            })
            .unwrap_err();
            return Err(error);
        }
        self.lifecycle.adopt_repeating_transition(
            LifecycleEvent::Dispatch,
            State::Online,
            State::OnCpu,
        )?;
        self.on_cpu = true;
        self.execution_authority = TaskExecutionAuthority::Live;
        self.thread_context.consume();
        let record =
            self.thread_context
                .create_dispatch_record(self.task_ref, expected_flow, cpu_ref);
        Ok(TaskFlowEnterProof { record })
    }

    /// Transitional object-level helper used by focused tests while all
    /// production scheduling goes through the explicit Dispatch/Enter pair.
    #[cfg(app_smoke)]
    pub(crate) fn dispatch_and_enter_for_test(&mut self) -> EventResult {
        let flow_ref = self.flow_ref();
        let cpu_ref = self.flow.cpu_ref().unwrap_or(super::cpu::CpuRef::invalid());
        let root_ref = self.root_trap_flow_ref();
        let proof = self.dispatch_on_cpu(flow_ref, self.context_epoch(), cpu_ref)?;
        self.enter_flow_contextual(proof, self.task_ref, true, flow_ref, root_ref)
    }

    #[cfg(app_smoke)]
    pub(crate) fn dispatch_rejected_for_test(
        &mut self,
        flow_ref: TaskFlowRef,
        context_epoch: u64,
        cpu_ref: super::cpu::CpuRef,
    ) -> bool {
        self.dispatch_on_cpu(flow_ref, context_epoch, cpu_ref)
            .is_err()
    }

    #[cfg(app_smoke)]
    pub(crate) fn duplicate_enter_rejected_for_test(&mut self) -> bool {
        let Some(mut record) = self.thread_context.dispatch_record else {
            return false;
        };
        if !record.entered {
            return false;
        }
        record.entered = false;
        let task_ref = self.task_ref;
        let flow_ref = self.flow_ref();
        let root_ref = self.root_trap_flow_ref();
        self.enter_flow_contextual(
            TaskFlowEnterProof { record },
            task_ref,
            true,
            flow_ref,
            root_ref,
        )
        .is_err()
    }

    pub(crate) fn enter_flow_contextual(
        &mut self,
        proof: TaskFlowEnterProof,
        current_task_ref: TaskRef,
        current_stack_matches: bool,
        effective_flow_ref: TaskFlowRef,
        expected_root_ref: TrapFlowRef,
    ) -> EventResult {
        let record = proof.record;
        let first_failed = if !record.task_ref.same_identity(self.task_ref) {
            Some("proof_task_ref")
        } else if !record.flow_ref.same_identity(self.flow_ref()) {
            Some("proof_flow_ref_generation")
        } else if record.context_epoch != self.thread_context.context_epoch() {
            Some("proof_context_epoch")
        } else if !current_task_ref.same_identity(self.task_ref) {
            Some("current_task_binding")
        } else if !current_stack_matches {
            Some("current_stack_binding")
        } else if !effective_flow_ref.same_identity(self.flow_ref()) {
            Some("effective_flow_binding")
        } else if self.root_trap_flow_ref() != expected_root_ref {
            Some("root_trap_flow_binding")
        } else if expected_root_ref.is_valid()
            && !TrapFlowType::active_leaf_preflight(
                expected_root_ref,
                self.task_ref,
                self.flow_ref(),
                record.cpu_ref,
                record.context_epoch,
            )
        {
            Some("active_trap_leaf_preflight")
        } else if self.thread_context.dispatch_record != Some(record) {
            Some("dispatch_record")
        } else if record.entered {
            Some("dispatch_record_not_entered")
        } else {
            None
        };
        if let Some(first_failed) = first_failed {
            return failed_condition(
                LifecycleEvent::Dispatch,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            )
            .map_err(|error| {
                error.with_diagnostic(FailureDiagnostic::new(
                    "SchedulerTaskSwitch",
                    "enter_task_flow_after_dispatch",
                    "TaskFlow",
                    "contextual Enter proof and execution bindings",
                    first_failed,
                ))
            });
        }

        if expected_root_ref.is_valid()
            && !TrapFlowType::resume_active_leaf(
                expected_root_ref,
                self.task_ref,
                self.flow_ref(),
                record.cpu_ref,
                record.context_epoch,
            )
        {
            return failed_condition(
                LifecycleEvent::Dispatch,
                self.state(),
                State::OnCpu,
                State::OnCpu,
            );
        }

        self.flow.enter_contextual(
            record.task_ref,
            record.flow_ref,
            record.cpu_ref,
            record.context_epoch,
            record.ordinal,
        )?;
        let mut entered_record = record;
        entered_record.entered = true;
        self.thread_context.dispatch_record = Some(entered_record);
        Ok(())
    }

    /// Scheduler-only acceptance of `Task.Suspend`, committed on the selected
    /// task's stack after the old task's core registers have been saved.
    pub(crate) fn suspend_from_cpu(&mut self) -> EventResult {
        self.save_core_context_for_suspend()?;
        self.suspend_after_core_context_save()
    }

    /// First half of the Scheduler SaveCoreContext -> Suspend protocol. The
    /// architecture switch completes the callee-register store immediately
    /// after this semantic save reservation and lifecycle handoff.
    pub(crate) fn save_core_context_for_suspend(&mut self) -> EventResult {
        if !self.switch_out_ready() {
            return failed_condition(
                LifecycleEvent::Suspend,
                self.state(),
                State::OnCpu,
                State::Online,
            );
        }
        self.thread_context.record_save();
        let root_ref = self.root_trap_flow_ref();
        if root_ref.is_valid()
            && !self.flow.cpu_ref().is_some_and(|cpu_ref| {
                TrapFlowType::record_suspended_context(
                    root_ref,
                    self.task_ref,
                    self.flow_ref(),
                    cpu_ref,
                    self.context_epoch(),
                )
            })
        {
            return failed_condition(
                LifecycleEvent::Suspend,
                self.state(),
                State::OnCpu,
                State::Online,
            );
        }
        self.core_save_pending_suspend = true;
        Ok(())
    }

    /// Second half of the Scheduler SaveCoreContext -> Suspend protocol.
    pub(crate) fn suspend_after_core_context_save(&mut self) -> EventResult {
        let flow_ref = self.flow_ref();
        if self.lifecycle.state() != State::OnCpu
            || !self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::Live
            || !self.core_save_pending_suspend
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
            || !flow_ref.is_valid()
            || !self.owns_flow(flow_ref)
        {
            return failed_condition(
                LifecycleEvent::Suspend,
                self.state(),
                State::OnCpu,
                State::Online,
            );
        }
        self.lifecycle.adopt_repeating_transition(
            LifecycleEvent::Suspend,
            State::OnCpu,
            State::Online,
        )?;
        self.on_cpu = false;
        self.execution_authority = TaskExecutionAuthority::None;
        self.core_save_pending_suspend = false;
        self.thread_context.publish(flow_ref);
        Ok(())
    }

    pub fn disable(&mut self) -> EventResult {
        if !self.flow_ref().is_valid()
            || !self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::Live
            || self.core_save_pending_suspend
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
        {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::OnCpu,
                State::Offline,
            );
        }
        self.on_cpu = false;
        self.execution_authority = TaskExecutionAuthority::None;
        self.thread_context.record_save();
        self.running = false;
        self.runqueue_published = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Disable, State::OnCpu, State::Offline)
    }

    pub fn cleanup(&mut self) -> EventResult {
        if self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::None
            || self.core_save_pending_suspend
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.lifecycle.state(),
                State::Offline,
                State::Destroyed,
            );
        }
        self.lifecycle
            .adopt_transition(LifecycleEvent::Cleanup, State::Offline, State::Destroyed)
    }

    pub fn set_runtime_running(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.running = true;
        Ok(())
    }

    pub fn publish_runqueue_binding(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.running || !self.flow_ref().is_valid() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.runqueue_published = true;
        Ok(())
    }

    pub fn pin_to_cpu(&mut self, cpu_id: usize) -> EventResult {
        if !self.online() || cpu_id == usize::MAX {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }
        self.affinity_cpu_id = cpu_id;
        self.no_setaffinity = true;
        Ok(())
    }

    pub fn clear_cpu_pin(&mut self) -> bool {
        if !self.online() || !self.affinity_pinned() || !self.no_setaffinity {
            return false;
        }
        self.affinity_cpu_id = usize::MAX;
        self.no_setaffinity = false;
        true
    }

    pub fn set_role_metadata(&mut self, entry: TaskEntry, kind: TaskKind) {
        self.entry = entry;
        self.kind = kind;
    }

    pub fn init_switch_context(
        &mut self,
        entry: extern "C" fn() -> !,
        stack_base: usize,
        stack_top: usize,
    ) {
        self.thread_context
            .arch_mut()
            .init(entry, stack_base, stack_top);
        if !self.on_cpu && self.lifecycle.state() == State::Prepared {
            self.thread_context.prepare();
        }
    }

    pub fn init_dummy_switch_context(&mut self) {
        self.thread_context.arch_mut().init_with_dummy();
        if !self.on_cpu && self.lifecycle.state() == State::Prepared {
            self.thread_context.prepare();
        }
    }

    pub fn set_kernel_stack_bounds(&mut self, base: usize, top: usize) -> bool {
        if self.stack_guard_installed
            && (self.kernel_stack_base() != base || self.kernel_stack_top() != top)
        {
            return false;
        }
        let installed = self
            .thread_context
            .arch_mut()
            .set_kernel_stack_bounds(base, top);
        installed && self.kernel_stack_base() == base && self.kernel_stack_top() == top
    }

    pub const fn kernel_stack_base(&self) -> usize {
        self.thread_context.arch().kernel_stack_base()
    }

    pub const fn kernel_stack_top(&self) -> usize {
        self.thread_context.arch().kernel_stack_top()
    }

    pub const fn switch_context(&self) -> &TaskSwitchContext {
        self.thread_context.arch()
    }

    pub const fn context_epoch(&self) -> u64 {
        self.thread_context.context_epoch()
    }

    pub fn current_stack_matches(&self) -> bool {
        let (stack_pointer, base, top) = self.current_stack_observation();
        base != 0 && top > base && stack_pointer >= base && stack_pointer <= top
    }

    pub(crate) fn current_stack_observation(&self) -> (usize, usize, usize) {
        (
            crate::arch::riscv64::csr::read_sp(),
            self.kernel_stack_base(),
            self.kernel_stack_top(),
        )
    }

    pub fn switch_context_mut(&mut self) -> &mut TaskSwitchContext {
        self.thread_context.arch_mut()
    }

    pub fn switch_in_ready(&self) -> bool {
        let expected_flow = self.flow_ref();
        self.lifecycle.state() == State::Online
            && !self.on_cpu
            && self.execution_authority == TaskExecutionAuthority::None
            && self.thread_context.breakpoint_state() == TaskBreakpointState::Valid
            && self.thread_context.core_register_set()
            && expected_flow.is_valid()
            && self.owns_flow(expected_flow)
            && self.breakpoint_matches(expected_flow)
    }

    pub fn switch_out_ready(&self) -> bool {
        self.lifecycle.state() == State::OnCpu
            && self.on_cpu
            && self.execution_authority == TaskExecutionAuthority::Live
            && !self.core_save_pending_suspend
            && self.thread_context.breakpoint_state() == TaskBreakpointState::Invalid
            && self.flow_ref().is_valid()
    }

    pub fn terminal_switch_out_ready(&self) -> bool {
        self.lifecycle.state() == State::OnCpu
            && self.on_cpu
            && self.execution_authority == TaskExecutionAuthority::Live
            && !self.core_save_pending_suspend
            && self.thread_context.breakpoint_state() == TaskBreakpointState::Invalid
            && self.flow_ref().is_valid()
    }
}
