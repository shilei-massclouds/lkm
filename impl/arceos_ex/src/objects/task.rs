use crate::{arch::riscv64::task_switch::TaskSwitchContext, checkpoint::Checkpoint};

use super::state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition};

pub const USER_TASK_SLOT_COUNT: usize = 8;
pub const USER_FLOW_SLOTS_PER_TASK: usize = 2;

const TASK_SLOT_BOOT: u16 = 1;
const TASK_SLOT_KERNEL_INIT: u16 = 2;
const TASK_SLOT_KTHREADD: u16 = 3;
const TASK_SLOT_SMOKE_SCHEDULER: u16 = 4;
const TASK_SLOT_SMOKE_MUTEX: u16 = 5;
const TASK_SLOT_SMOKE_RWSEM: u16 = 6;
const TASK_SLOT_SMOKE_RWLOCK: u16 = 7;
const TASK_SLOT_AP_IDLE_BASE: u16 = 16;
const TASK_SLOT_USER_BASE: u16 = 32;

const FLOW_SLOT_ROOT_STREAM: u16 = 1;
const FLOW_SLOT_BOOT_IDLE: u16 = 2;
const FLOW_SLOT_KERNEL_INIT: u16 = 3;
const FLOW_SLOT_KTHREADD: u16 = 4;
const FLOW_SLOT_SMOKE_BASE: u16 = 8;
const FLOW_SLOT_AP_IDLE_BASE: u16 = 16;
const FLOW_SLOT_KERNEL_INIT_USER_BASE: u16 = 32;
const FLOW_SLOT_USER_BASE: u16 = 64;

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
pub struct TaskFlowRef {
    slot: u16,
    generation: u32,
}

impl TaskFlowRef {
    pub const NONE: Self = Self::new(0, 0);
    pub const ROOT_STREAM: Self = Self::new(FLOW_SLOT_ROOT_STREAM, 1);
    pub const BOOT_IDLE: Self = Self::new(FLOW_SLOT_BOOT_IDLE, 1);
    pub const KERNEL_INIT: Self = Self::new(FLOW_SLOT_KERNEL_INIT, 1);
    pub const KTHREADD: Self = Self::new(FLOW_SLOT_KTHREADD, 1);

    const fn new(slot: u16, generation: u32) -> Self {
        Self { slot, generation }
    }

    pub(crate) const fn smoke(index: usize) -> Self {
        Self::new(FLOW_SLOT_SMOKE_BASE + index as u16, 1)
    }

    pub(crate) const fn ap_idle(logical_id: usize) -> Self {
        Self::new(FLOW_SLOT_AP_IDLE_BASE + logical_id as u16, 1)
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

pub struct TaskCpuState {
    cpu_id: usize,
}

impl TaskCpuState {
    pub const fn new() -> Self {
        Self { cpu_id: usize::MAX }
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu_id
    }

    pub fn set_task_cpu(&mut self, cpu_id: usize) -> bool {
        if cpu_id == usize::MAX {
            return false;
        }

        self.cpu_id = cpu_id;
        true
    }
}

const TASK_OWNED_FLOW_CAPACITY: usize = 4;

/// The one lifecycle/identity/PID/CPU/context/Flow-ownership carrier.
pub struct Task {
    lifecycle: Lifecycle,
    task_ref: TaskRef,
    entry: TaskEntry,
    pid: usize,
    kind: TaskKind,
    cpu: TaskCpuState,
    running: bool,
    runqueue_published: bool,
    affinity_cpu_id: usize,
    no_setaffinity: bool,
    switch_ctx: TaskSwitchContext,
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
            task_ref,
            entry: TaskEntry::None,
            pid: 0,
            kind: TaskKind::None,
            cpu: TaskCpuState::new(),
            running: false,
            runqueue_published: false,
            affinity_cpu_id: usize::MAX,
            no_setaffinity: false,
            switch_ctx: TaskSwitchContext::new(),
            owned_flows: [TaskFlowRef::NONE; TASK_OWNED_FLOW_CAPACITY],
            active_flow: TaskFlowRef::NONE,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
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

    pub const fn cpu_id(&self) -> usize {
        self.cpu.cpu_id()
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
        if self.runqueue_published {
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
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self, checkpoint: Checkpoint) -> EventResult {
        if !self.running || !self.runqueue_published || !self.active_flow.is_valid() {
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
        )
    }

    pub fn enable_from_prepared(&mut self, checkpoint: Checkpoint) -> EventResult {
        self.running = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Prepared,
            State::Online,
            checkpoint,
        )
    }

    pub fn adopt_enable(&mut self) -> EventResult {
        self.running = true;
        self.runqueue_published = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    pub fn disable(&mut self) -> EventResult {
        if self.active_flow.is_valid() {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Online,
                State::Offline,
            );
        }
        self.running = false;
        self.runqueue_published = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Disable, State::Online, State::Offline)
    }

    pub fn cleanup(&mut self) -> EventResult {
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

    pub fn set_task_cpu(&mut self, cpu_id: usize) -> bool {
        self.cpu.set_task_cpu(cpu_id)
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
        if self.lifecycle.state() != State::Ready
            || !self.running
            || self.cpu.cpu_id() == usize::MAX
            || !self.active_flow.is_valid()
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
        if self.lifecycle.state() != State::Online
            || cpu_id == usize::MAX
            || self.cpu.cpu_id() != cpu_id
        {
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
        let tp = self as *const Task as usize;
        self.switch_ctx.init(entry, stack_top, tp);
    }

    pub fn init_dummy_switch_context(&mut self) {
        self.switch_ctx.init_with_dummy();
    }

    pub const fn switch_context(&self) -> &TaskSwitchContext {
        &self.switch_ctx
    }

    pub fn switch_context_mut(&mut self) -> &mut TaskSwitchContext {
        &mut self.switch_ctx
    }

    fn register_owned_flow(&mut self, flow_ref: TaskFlowRef) -> EventResult {
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

    pub fn bind_initial_flow(&mut self, flow: &mut TaskFlow) -> EventResult {
        if self.active_flow.is_valid()
            || flow.owner() != self.task_ref
            || flow.state() != State::Ready
            || !self.owns_flow(flow.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                self.lifecycle.state(),
                self.lifecycle.state(),
            );
        }
        self.active_flow = flow.flow_ref();
        flow.commit_active_binding(self.task_ref)
    }

    pub fn bind_initial_prepared_flow(&mut self, flow: &mut TaskFlow) -> EventResult {
        if self.active_flow.is_valid()
            || flow.owner() != self.task_ref
            || flow.state() != State::Prepared
            || !self.owns_flow(flow.flow_ref())
        {
            return failed_condition(
                LifecycleEvent::Setup,
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

/// Independently-lived Flow core shared by boot, kernel, user, AP and smoke
/// role wrappers.
pub struct TaskFlow {
    lifecycle: Lifecycle,
    flow_ref: TaskFlowRef,
    storage_slot: u16,
    generation: u32,
    declared: bool,
    owner: TaskRef,
    active: bool,
    predecessor: TaskFlowRef,
    disabled: bool,
    cleaned: bool,
}

impl TaskFlow {
    pub const fn new_static(flow_ref: TaskFlowRef) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            flow_ref,
            storage_slot: flow_ref.slot,
            generation: flow_ref.generation,
            declared: flow_ref.is_valid(),
            owner: TaskRef::NONE,
            active: false,
            predecessor: TaskFlowRef::NONE,
            disabled: false,
            cleaned: false,
        }
    }

    pub const fn new_dynamic(storage_slot: u16) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            flow_ref: TaskFlowRef::NONE,
            storage_slot,
            generation: 0,
            declared: false,
            owner: TaskRef::NONE,
            active: false,
            predecessor: TaskFlowRef::NONE,
            disabled: false,
            cleaned: false,
        }
    }

    pub const fn new_kernel_init_user(slot: usize) -> Self {
        Self::new_dynamic(FLOW_SLOT_KERNEL_INIT_USER_BASE + slot as u16)
    }

    pub const fn new_user(task_slot: usize, flow_slot: usize) -> Self {
        Self::new_dynamic(
            FLOW_SLOT_USER_BASE + (task_slot * USER_FLOW_SLOTS_PER_TASK + flow_slot) as u16,
        )
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.flow_ref
    }

    pub const fn declared(&self) -> bool {
        self.declared
    }

    pub const fn owner(&self) -> TaskRef {
        self.owner
    }

    pub const fn active(&self) -> bool {
        self.active
    }

    #[allow(dead_code)]
    pub const fn predecessor(&self) -> TaskFlowRef {
        self.predecessor
    }

    #[allow(dead_code)]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    pub const fn cleaned(&self) -> bool {
        self.cleaned
    }

    pub fn declare(&mut self) -> EventResult {
        if self.declared
            || !matches!(self.lifecycle.state(), State::Base | State::Destroyed)
            || self.active
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                self.lifecycle.state(),
                State::Base,
            );
        }
        self.generation = next_generation(self.generation);
        self.flow_ref = TaskFlowRef::new(self.storage_slot, self.generation);
        self.lifecycle = Lifecycle::new(State::Base);
        self.declared = true;
        self.owner = TaskRef::NONE;
        self.predecessor = TaskFlowRef::NONE;
        self.disabled = false;
        self.cleaned = false;
        Ok(())
    }

    pub fn preset(
        &mut self,
        owner: &mut Task,
        predecessor: TaskFlowRef,
        checkpoint: Option<Checkpoint>,
    ) -> EventResult {
        if !self.declared
            || !self.flow_ref.is_valid()
            || self.lifecycle.state() != State::Base
            || !owner.task_ref().is_valid()
            || (predecessor.is_valid() && !owner.owns_flow(predecessor))
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        owner.register_owned_flow(self.flow_ref)?;
        self.owner = owner.task_ref();
        self.predecessor = predecessor;
        match checkpoint {
            Some(checkpoint) => self.lifecycle.transition(
                LifecycleEvent::Preset,
                State::Base,
                State::Prepared,
                checkpoint,
            ),
            None => self.lifecycle.adopt_transition(
                LifecycleEvent::Preset,
                State::Base,
                State::Prepared,
            ),
        }
    }

    pub fn setup(&mut self, checkpoint: Option<Checkpoint>) -> EventResult {
        if !self.owner.is_valid() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }
        match checkpoint {
            Some(checkpoint) => self.lifecycle.transition(
                LifecycleEvent::Setup,
                State::Prepared,
                State::Ready,
                checkpoint,
            ),
            None => self.lifecycle.adopt_transition(
                LifecycleEvent::Setup,
                State::Prepared,
                State::Ready,
            ),
        }
    }

    fn commit_active_binding(&mut self, owner: TaskRef) -> EventResult {
        if !matches!(self.lifecycle.state(), State::Prepared | State::Ready)
            || self.owner != owner
            || self.active
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        self.active = true;
        Ok(())
    }

    pub fn enable(&mut self, owner: &Task, checkpoint: Option<Checkpoint>) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.owner != owner.task_ref()
            || owner.active_flow() != self.flow_ref
            || !self.active
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        match checkpoint {
            Some(checkpoint) => self.lifecycle.transition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Online,
                checkpoint,
            ),
            None => {
                self.lifecycle
                    .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
            }
        }
    }

    pub fn disable(&mut self, owner: &Task, checkpoint: Option<Checkpoint>) -> EventResult {
        if self.lifecycle.state() != State::Online
            || self.owner != owner.task_ref()
            || owner.active_flow() != self.flow_ref
            || !self.active
        {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Online,
                State::Offline,
            );
        }
        self.active = false;
        self.disabled = true;
        match checkpoint {
            Some(checkpoint) => self.lifecycle.transition(
                LifecycleEvent::Disable,
                State::Online,
                State::Offline,
                checkpoint,
            ),
            None => self.lifecycle.adopt_transition(
                LifecycleEvent::Disable,
                State::Online,
                State::Offline,
            ),
        }
    }

    pub fn cleanup(&mut self, owner: &Task, checkpoint: Option<Checkpoint>) -> EventResult {
        if self.lifecycle.state() != State::Offline
            || self.owner != owner.task_ref()
            || owner.active_flow() == self.flow_ref
            || self.active
            || !self.disabled
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.lifecycle.state(),
                State::Offline,
                State::Destroyed,
            );
        }
        self.cleaned = true;
        self.declared = false;
        match checkpoint {
            Some(checkpoint) => self.lifecycle.transition(
                LifecycleEvent::Cleanup,
                State::Offline,
                State::Destroyed,
                checkpoint,
            ),
            None => self.lifecycle.adopt_transition(
                LifecycleEvent::Cleanup,
                State::Offline,
                State::Destroyed,
            ),
        }
    }

    pub fn cleanup_active_for_exit(&mut self, owner: &mut Task) -> EventResult {
        self.disable(owner, None)?;
        owner.active_flow = TaskFlowRef::NONE;
        self.cleaned = true;
        self.declared = false;
        self.lifecycle.adopt_transition(
            LifecycleEvent::Cleanup,
            State::Offline,
            State::Destroyed,
        )?;
        owner.retire_destroyed_flow(self)
    }

    #[allow(dead_code)]
    pub const fn storage_slot(&self) -> usize {
        self.storage_slot as usize
    }

    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

pub const fn next_generation(current: u32) -> u32 {
    let next = current.wrapping_add(1);
    if next == 0 { 1 } else { next }
}
