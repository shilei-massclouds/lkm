use crate::{arch::riscv64::task_switch::TaskSwitchContext, checkpoint::Checkpoint};

use super::{
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    task_flow::{TaskFlow, TaskFlowRef},
};

pub const USER_TASK_SLOT_COUNT: usize = 8;

const TASK_SLOT_BOOT: u16 = 1;
const TASK_SLOT_KERNEL_INIT: u16 = 2;
const TASK_SLOT_KTHREADD: u16 = 3;
const TASK_SLOT_SMOKE_SCHEDULER: u16 = 4;
const TASK_SLOT_SMOKE_MUTEX: u16 = 5;
const TASK_SLOT_SMOKE_RWSEM: u16 = 6;
const TASK_SLOT_SMOKE_RWLOCK: u16 = 7;
const TASK_SLOT_AP_IDLE_BASE: u16 = 16;
const TASK_SLOT_USER_BASE: u16 = 32;

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

    pub const fn is_valid(self) -> bool {
        self.slot != 0 && self.generation != 0
    }

    #[allow(dead_code)]
    pub const fn slot(self) -> usize {
        self.slot as usize
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

/// The physical core-register context and its recoverable continuation
/// metadata. This wrapper is owned by exactly one Task.
pub struct TaskThreadContext {
    arch: TaskSwitchContext,
    breakpoint_state: TaskBreakpointState,
    flow_ref: TaskFlowRef,
    core_saved_count: usize,
    core_restored_count: usize,
}

impl TaskThreadContext {
    pub const fn new() -> Self {
        Self {
            arch: TaskSwitchContext::new(),
            breakpoint_state: TaskBreakpointState::Invalid,
            flow_ref: TaskFlowRef::NONE,
            core_saved_count: 0,
            core_restored_count: 0,
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

    pub const fn core_register_set(&self) -> bool {
        self.arch.initialized()
    }

    pub const fn core_saved_count(&self) -> usize {
        self.core_saved_count
    }

    pub const fn core_restored_count(&self) -> usize {
        self.core_restored_count
    }

    fn prepare(&mut self) {
        self.breakpoint_state = TaskBreakpointState::Prepared;
        self.flow_ref = TaskFlowRef::NONE;
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
    }
}

const TASK_OWNED_FLOW_CAPACITY: usize = 4;

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
    affinity_cpu_id: usize,
    no_setaffinity: bool,
    thread_context: TaskThreadContext,
    initial_flow: TaskFlowRef,
    owned_flows: [TaskFlowRef; TASK_OWNED_FLOW_CAPACITY],
    active_flow: TaskFlowRef,
}

impl Task {
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
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            thread_context: TaskThreadContext::new(),
            initial_flow: TaskFlowRef::NONE,
            owned_flows: [TaskFlowRef::NONE; TASK_OWNED_FLOW_CAPACITY],
            active_flow: TaskFlowRef::NONE,
        }
    }

    /// Boot-only image initializer for the pre-existing PID 0 carrier.
    ///
    /// This is deliberately separate from the reusable Task lifecycle: the
    /// linker-visible init task exists before `_start` and never performs
    /// Preset/Setup/Enable.
    pub(crate) const fn new_boot_on_cpu() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Online),
            on_cpu: true,
            execution_authority: TaskExecutionAuthority::Live,
            task_ref: TaskRef::BOOT,
            entry: TaskEntry::None,
            pid: 0,
            kind: TaskKind::None,
            running: true,
            runqueue_published: false,
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            thread_context: TaskThreadContext::new(),
            initial_flow: TaskFlowRef::BOOT_INIT,
            owned_flows: [
                TaskFlowRef::BOOT_INIT,
                TaskFlowRef::NONE,
                TaskFlowRef::NONE,
                TaskFlowRef::NONE,
            ],
            active_flow: TaskFlowRef::NONE,
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
            lifecycle: Lifecycle::new(State::Online),
            on_cpu: true,
            execution_authority: TaskExecutionAuthority::Reserved,
            task_ref,
            entry: TaskEntry::ApIdle,
            pid: 0,
            kind: TaskKind::Idle,
            running: true,
            runqueue_published: false,
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            thread_context: TaskThreadContext::new(),
            initial_flow: flow_ref,
            owned_flows: [
                flow_ref,
                TaskFlowRef::NONE,
                TaskFlowRef::NONE,
                TaskFlowRef::NONE,
            ],
            active_flow: TaskFlowRef::NONE,
        }
    }

    pub const fn state(&self) -> State {
        if self.on_cpu {
            State::OnCpu
        } else {
            self.lifecycle.state()
        }
    }

    /// True for the persistent scheduler-visible lifecycle, including while
    /// the Task owns a CPU through the Task-only OnCpu projection.
    pub const fn online(&self) -> bool {
        matches!(self.lifecycle.state(), State::Online)
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

    pub const fn affinity_pinned(&self) -> bool {
        self.affinity_cpu_id != usize::MAX
    }

    pub const fn no_setaffinity(&self) -> bool {
        self.no_setaffinity
    }

    pub const fn active_flow(&self) -> TaskFlowRef {
        self.active_flow
    }

    pub const fn initial_flow(&self) -> TaskFlowRef {
        self.initial_flow
    }

    pub const fn owns_flow(&self, flow_ref: TaskFlowRef) -> bool {
        let mut index = 0;
        while index < TASK_OWNED_FLOW_CAPACITY {
            if self.owned_flows[index].same_identity(flow_ref) {
                return true;
            }
            index += 1;
        }
        false
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
            || !self.initial_flow.is_valid()
            || !self.owns_flow(self.initial_flow)
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
        self.thread_context.publish(self.initial_flow);
        Ok(())
    }

    pub fn adopt_enable(&mut self) -> EventResult {
        if !self.initial_flow.is_valid()
            || !self.owns_flow(self.initial_flow)
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
        self.thread_context.publish(self.initial_flow);
        Ok(())
    }

    /// Accept the keyed SBI HSM Startup delivery. Task lifecycle and identity
    /// do not change; only the reserved execution authority becomes Live.
    pub(crate) fn activate_hsm_authority(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.on_cpu
            || self.entry != TaskEntry::ApIdle
            || self.kind != TaskKind::Idle
            || !self.running
            || self.runqueue_published
            || !self.initial_flow.is_valid()
            || self.active_flow.is_valid()
            || self.execution_authority != TaskExecutionAuthority::Reserved
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
        {
            return failed_condition(
                LifecycleEvent::Continue,
                self.state(),
                State::Online,
                State::OnCpu,
            );
        }
        self.execution_authority = TaskExecutionAuthority::Live;
        Ok(())
    }

    /// Scheduler-only acceptance of `Task.Continue`. The caller must invoke
    /// this from the new task's real entry/resume point, after the physical
    /// context switch has transferred execution.
    pub(crate) fn continue_on_cpu(&mut self) -> EventResult {
        let expected_flow = if self.active_flow.is_valid() {
            self.active_flow
        } else {
            self.initial_flow
        };
        if self.lifecycle.state() != State::Online
            || self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::None
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Valid
            || !expected_flow.is_valid()
            || !self.owns_flow(expected_flow)
            || !self.thread_context.flow_ref().same_identity(expected_flow)
        {
            return failed_condition(
                LifecycleEvent::Continue,
                self.state(),
                State::Online,
                State::OnCpu,
            );
        }
        self.on_cpu = true;
        self.execution_authority = TaskExecutionAuthority::Live;
        self.thread_context.consume();
        Ok(())
    }

    /// Scheduler-only acceptance of `Task.Suspend`, committed on the selected
    /// task's stack after the old task's core registers have been saved.
    pub(crate) fn suspend_from_cpu(&mut self) -> EventResult {
        let flow_ref = self.active_flow;
        if self.lifecycle.state() != State::Online
            || !self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::Live
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
        self.on_cpu = false;
        self.execution_authority = TaskExecutionAuthority::None;
        self.thread_context.record_save();
        self.thread_context.publish(flow_ref);
        Ok(())
    }

    pub fn disable(&mut self) -> EventResult {
        if self.active_flow.is_valid()
            || !self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::Live
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
        {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Online,
                State::Offline,
            );
        }
        self.on_cpu = false;
        self.execution_authority = TaskExecutionAuthority::None;
        self.thread_context.record_save();
        self.running = false;
        self.runqueue_published = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Disable, State::Online, State::Offline)
    }

    pub fn cleanup(&mut self) -> EventResult {
        if self.on_cpu
            || self.execution_authority != TaskExecutionAuthority::None
            || self.thread_context.breakpoint_state() != TaskBreakpointState::Invalid
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.lifecycle.state(),
                State::Offline,
                State::Destroyed,
            );
        }
        let mut index = 0;
        while index < TASK_OWNED_FLOW_CAPACITY {
            if self.owned_flows[index].is_valid() {
                return failed_condition(
                    LifecycleEvent::Cleanup,
                    self.lifecycle.state(),
                    State::Offline,
                    State::Destroyed,
                );
            }
            index += 1;
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
        if self.lifecycle.state() != State::Ready || !self.running || !self.initial_flow.is_valid()
        {
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
        if self.lifecycle.state() != State::Online || cpu_id == usize::MAX {
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
        if self.lifecycle.state() != State::Online
            || !self.affinity_pinned()
            || !self.no_setaffinity
        {
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

    pub fn init_switch_context(&mut self, entry: extern "C" fn() -> !, stack_top: usize) {
        self.thread_context.arch_mut().init(entry, stack_top);
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

    pub const fn switch_context(&self) -> &TaskSwitchContext {
        self.thread_context.arch()
    }

    pub fn switch_context_mut(&mut self) -> &mut TaskSwitchContext {
        self.thread_context.arch_mut()
    }

    pub fn switch_in_ready(&self) -> bool {
        let expected_flow = if self.active_flow.is_valid() {
            self.active_flow
        } else {
            self.initial_flow
        };
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
        self.lifecycle.state() == State::Online
            && self.on_cpu
            && self.execution_authority == TaskExecutionAuthority::Live
            && self.thread_context.breakpoint_state() == TaskBreakpointState::Invalid
            && self.active_flow.is_valid()
            && self.owns_flow(self.active_flow)
    }

    pub fn terminal_switch_out_ready(&self) -> bool {
        self.lifecycle.state() == State::Online
            && self.on_cpu
            && self.execution_authority == TaskExecutionAuthority::Live
            && self.thread_context.breakpoint_state() == TaskBreakpointState::Invalid
            && !self.active_flow.is_valid()
    }

    pub(super) fn register_owned_flow(&mut self, flow_ref: TaskFlowRef) -> EventResult {
        if !flow_ref.is_valid() || self.owns_flow(flow_ref) {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        let mut index = 0;
        while index < TASK_OWNED_FLOW_CAPACITY {
            if !self.owned_flows[index].is_valid() {
                self.owned_flows[index] = flow_ref;
                return Ok(());
            }
            index += 1;
        }
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            self.lifecycle.state(),
            self.lifecycle.state(),
        )
    }

    pub(super) fn clear_active_flow_for_exit(&mut self, flow_ref: TaskFlowRef) -> EventResult {
        if self.active_flow != flow_ref {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        self.active_flow = TaskFlowRef::NONE;
        Ok(())
    }

    pub fn bind_initial_flow(&mut self, flow: &TaskFlow) -> EventResult {
        if self.initial_flow.is_valid()
            || flow.owner() != self.task_ref
            || flow.state() != State::Base
            || !self.owns_flow(flow.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        self.initial_flow = flow.flow_ref();
        Ok(())
    }

    pub fn activate_initial_flow(&mut self, flow: &mut TaskFlow) -> EventResult {
        if self.active_flow.is_valid()
            || self.initial_flow != flow.flow_ref()
            || flow.owner() != self.task_ref
            || flow.state() != State::Ready
            || !self.owns_flow(flow.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        self.active_flow = flow.flow_ref();
        flow.commit_active_binding(self.task_ref)
    }

    pub fn commit_flow_handoff(&mut self, old: &TaskFlow, new: &mut TaskFlow) -> EventResult {
        if self.active_flow != old.flow_ref()
            || old.owner() != self.task_ref
            || old.state() != State::Offline
            || old.active()
            || new.owner() != self.task_ref
            || new.state() != State::Ready
            || new.active()
            || !self.owns_flow(old.flow_ref())
            || !self.owns_flow(new.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        if !new.inherit_cpu_ref(old) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        self.active_flow = new.flow_ref();
        new.commit_active_binding(self.task_ref)
    }

    pub fn commit_ready_successor_handoff(
        &mut self,
        old: &mut TaskFlow,
        new: &mut TaskFlow,
    ) -> EventResult {
        if self.active_flow != old.flow_ref()
            || old.owner() != self.task_ref
            || !matches!(old.state(), State::Ready | State::Online)
            || !old.active()
            || new.owner() != self.task_ref
            || new.state() != State::Ready
            || new.active()
            || !self.owns_flow(old.flow_ref())
            || !self.owns_flow(new.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        if !new.inherit_cpu_ref(old) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        old.relinquish_active_binding(self.task_ref)?;
        self.active_flow = new.flow_ref();
        new.commit_active_binding(self.task_ref)
    }

    pub fn retire_destroyed_flow(&mut self, flow: &TaskFlow) -> EventResult {
        if flow.owner() != self.task_ref
            || flow.state() != State::Destroyed
            || self.active_flow == flow.flow_ref()
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        let mut index = 0;
        while index < TASK_OWNED_FLOW_CAPACITY {
            if self.owned_flows[index] == flow.flow_ref() {
                self.owned_flows[index] = TaskFlowRef::NONE;
                return Ok(());
            }
            index += 1;
        }
        failed_condition(
            LifecycleEvent::Cleanup,
            self.lifecycle.state(),
            self.lifecycle.state(),
            self.lifecycle.state(),
        )
    }
}
