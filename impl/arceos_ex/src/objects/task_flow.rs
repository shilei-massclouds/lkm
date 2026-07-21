use crate::checkpoint::Checkpoint;

use super::{
    next_generation,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    task::{Task, TaskRef},
};

pub const USER_FLOW_SLOTS_PER_TASK: usize = 2;

const FLOW_SLOT_BOOT_IDLE: u16 = 2;
const FLOW_SLOT_KERNEL_INIT: u16 = 3;
const FLOW_SLOT_KTHREADD: u16 = 4;
const FLOW_SLOT_SMOKE_BASE: u16 = 8;
const FLOW_SLOT_AP_IDLE_BASE: u16 = 16;
const FLOW_SLOT_KERNEL_INIT_USER_BASE: u16 = 32;
const FLOW_SLOT_USER_BASE: u16 = 64;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TaskFlowRef {
    slot: u16,
    generation: u32,
}

impl TaskFlowRef {
    pub const NONE: Self = Self::new(0, 0);
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

    pub(super) fn commit_active_binding(&mut self, owner: TaskRef) -> EventResult {
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
        owner.clear_active_flow_for_exit(self.flow_ref)?;
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
