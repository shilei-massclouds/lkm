use crate::checkpoint::Checkpoint;

use super::{
    cpu::CpuRef,
    next_generation,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    task::{Task, TaskExecutionAuthority, TaskRef},
};

const FLOW_SLOT_BOOT_INIT: u16 = 1;
const FLOW_SLOT_KERNEL_INIT: u16 = 3;
const FLOW_SLOT_KTHREADD: u16 = 4;
const FLOW_SLOT_SMOKE_BASE: u16 = 8;
const FLOW_SLOT_AP_IDLE_BASE: u16 = 16;
const FLOW_SLOT_USER_BASE: u16 = 64;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TaskFlowRef {
    slot: u16,
    generation: u32,
}

impl TaskFlowRef {
    pub const NONE: Self = Self::new(0, 0);
    pub const BOOT_INIT: Self = Self::new(FLOW_SLOT_BOOT_INIT, 1);
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

    #[cfg(app_smoke)]
    pub(crate) const fn with_generation_for_test(self, generation: u32) -> Self {
        Self::new(self.slot, generation)
    }
}

/// The lifetime Flow core physically embedded by its unique owning Task.
pub struct TaskFlow {
    lifecycle: Lifecycle,
    flow_ref: TaskFlowRef,
    storage_slot: u16,
    generation: u32,
    declared: bool,
    owner: TaskRef,
    disabled: bool,
    cleaned: bool,
    cpu_ref: CpuRef,
    initial_context_entry_pending: bool,
    initial_context_entry_consumed: bool,
    last_entered_context_epoch: u64,
    last_entered_dispatch_ordinal: u64,
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
            disabled: false,
            cleaned: false,
            cpu_ref: CpuRef::invalid(),
            initial_context_entry_pending: false,
            initial_context_entry_consumed: false,
            last_entered_context_epoch: 0,
            last_entered_dispatch_ordinal: 0,
        }
    }

    pub const fn new_static_bound(flow_ref: TaskFlowRef, owner: TaskRef) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            flow_ref,
            storage_slot: flow_ref.slot,
            generation: flow_ref.generation,
            declared: flow_ref.is_valid(),
            owner,
            disabled: false,
            cleaned: false,
            cpu_ref: CpuRef::invalid(),
            initial_context_entry_pending: false,
            initial_context_entry_consumed: false,
            last_entered_context_epoch: 0,
            last_entered_dispatch_ordinal: 0,
        }
    }

    pub const fn new_dynamic(storage_slot: u16) -> Self {
        Self::new_dynamic_with_generation(storage_slot, 0)
    }

    pub const fn new_dynamic_with_generation(storage_slot: u16, generation: u32) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            flow_ref: TaskFlowRef::NONE,
            storage_slot,
            generation,
            declared: false,
            owner: TaskRef::NONE,
            disabled: false,
            cleaned: false,
            cpu_ref: CpuRef::invalid(),
            initial_context_entry_pending: false,
            initial_context_entry_consumed: false,
            last_entered_context_epoch: 0,
            last_entered_dispatch_ordinal: 0,
        }
    }

    pub const fn new_user(task_slot: usize) -> Self {
        Self::new_dynamic(FLOW_SLOT_USER_BASE + task_slot as u16)
    }

    pub const fn new_user_with_generation(task_slot: usize, generation: u32) -> Self {
        Self::new_dynamic_with_generation(FLOW_SLOT_USER_BASE + task_slot as u16, generation)
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

    #[allow(dead_code)]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    pub const fn cpu_ref(&self) -> Option<CpuRef> {
        if self.cpu_ref.is_valid() {
            Some(self.cpu_ref)
        } else {
            None
        }
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu_ref.logical_id()
    }

    #[cfg(app_smoke)]
    pub const fn initial_context_entry_pending(&self) -> bool {
        self.initial_context_entry_pending
    }

    #[cfg(app_smoke)]
    pub const fn initial_context_entry_consumed(&self) -> bool {
        self.initial_context_entry_consumed
    }

    pub(crate) fn prepare_initial_context_entry(&mut self) -> bool {
        if !self.declared
            || !self.owner.is_valid()
            || self.initial_context_entry_pending
            || self.initial_context_entry_consumed
        {
            return false;
        }
        self.initial_context_entry_pending = true;
        true
    }

    /// Boot/AP architecture entry consumes Start without inventing a
    /// scheduler Dispatch/Enter occurrence.
    pub(crate) fn consume_direct_initial_context_entry(&mut self) -> bool {
        if self.initial_context_entry_consumed || !self.declared || !self.owner.is_valid() {
            return false;
        }
        self.initial_context_entry_pending = false;
        self.initial_context_entry_consumed = true;
        true
    }

    pub(crate) fn enter_contextual(
        &mut self,
        owner_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
        context_epoch: u64,
        dispatch_ordinal: u64,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !owner_ref.is_valid()
            || !self.owner.same_identity(owner_ref)
            || !self.flow_ref.same_identity(flow_ref)
            || self.cpu_ref != cpu_ref
            || context_epoch == 0
            || dispatch_ordinal == 0
            || self.last_entered_dispatch_ordinal == dispatch_ordinal
            || self.last_entered_context_epoch > context_epoch
        {
            return failed_condition(
                LifecycleEvent::Dispatch,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }
        self.last_entered_context_epoch = context_epoch;
        self.last_entered_dispatch_ordinal = dispatch_ordinal;
        if self.initial_context_entry_pending {
            if self.initial_context_entry_consumed {
                return failed_condition(
                    LifecycleEvent::Dispatch,
                    self.lifecycle.state(),
                    State::Online,
                    State::Online,
                );
            }
            self.initial_context_entry_pending = false;
            self.initial_context_entry_consumed = true;
        }
        Ok(())
    }

    /// Entry/runqueue binding boundary for a Flow that has not yet acquired
    /// a CPU. Task deliberately carries no synonymous CPU assignment.
    pub fn bind_cpu_ref(&mut self, cpu_ref: CpuRef) -> bool {
        if !cpu_ref.is_valid()
            || self.cpu_ref.is_valid()
            || !matches!(
                self.lifecycle.state(),
                State::Base | State::Prepared | State::Ready
            )
        {
            return false;
        }
        self.commit_cpu_ref(cpu_ref)
    }

    /// Scheduler commit boundary. A Flow retains its previous CpuRef while it
    /// is not OnCpu, and changes it only when migration is committed.
    pub fn commit_cpu_ref(&mut self, cpu_ref: CpuRef) -> bool {
        if !cpu_ref.is_valid() || !self.declared {
            return false;
        }
        self.cpu_ref = cpu_ref;
        true
    }

    pub fn declare(&mut self) -> EventResult {
        if self.declared || !matches!(self.lifecycle.state(), State::Base | State::Destroyed) {
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
        self.disabled = false;
        self.cleaned = false;
        self.cpu_ref = CpuRef::invalid();
        self.initial_context_entry_pending = false;
        self.initial_context_entry_consumed = false;
        self.last_entered_context_epoch = 0;
        self.last_entered_dispatch_ordinal = 0;
        Ok(())
    }

    pub(crate) fn bind_owner(&mut self, owner_ref: TaskRef) -> EventResult {
        if !self.declared
            || !self.flow_ref.is_valid()
            || self.lifecycle.state() != State::Base
            || self.owner.is_valid()
            || !owner_ref.is_valid()
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Base,
            );
        }
        self.owner = owner_ref;
        Ok(())
    }

    pub(crate) fn preset_owned(
        &mut self,
        owner_ref: TaskRef,
        checkpoint: Option<Checkpoint>,
    ) -> EventResult {
        if !self.declared
            || !self.flow_ref.is_valid()
            || self.lifecycle.state() != State::Base
            || self.owner != owner_ref
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
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

    pub(crate) fn setup_owned(
        &mut self,
        owner_ref: TaskRef,
        checkpoint: Option<Checkpoint>,
    ) -> EventResult {
        if self.owner != owner_ref {
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

    pub(crate) fn enable_owned(
        &mut self,
        owner_ref: TaskRef,
        checkpoint: Option<Checkpoint>,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.owner != owner_ref {
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

    pub(crate) fn cleanup_owned(&mut self, owner_ref: TaskRef) -> EventResult {
        if self.lifecycle.state() != State::Online || self.owner != owner_ref {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Online,
                State::Offline,
            );
        }
        self.disabled = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Disable, State::Online, State::Offline)?;
        self.cleaned = true;
        self.declared = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Cleanup, State::Offline, State::Destroyed)
    }

    #[allow(dead_code)]
    pub const fn storage_slot(&self) -> usize {
        self.storage_slot as usize
    }
}

pub fn task_flow_execution_guard_satisfied(flow: &TaskFlow, owner: &Task) -> bool {
    owner.state() == State::OnCpu
        && owner.execution_authority() == TaskExecutionAuthority::Live
        && flow.owner() == owner.task_ref()
        && owner.flow().same_identity(flow.flow_ref())
}
