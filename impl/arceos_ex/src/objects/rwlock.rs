use super::{
    cpu_control::CurrentTaskRef,
    state::{
        EventError, EventErrorCode, EventResult, Lifecycle, LifecycleEvent, State, failed_condition,
    },
};

const OWNER_SLOTS: usize = 4;

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RwLockInitKind {
    StaticInitializer,
    RuntimeInit,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RwLockExtState {
    Unlocked,
    ReadHeld,
    WriteHeld,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RwLockOwner {
    None,
    BootInitTask,
    KernelInitTask,
    SmokeRwLockTask,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RwLockReadOutcome {
    Acquired,
    Shared,
    Blocked,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RwLockWriteOutcome {
    Acquired,
    Blocked,
}

pub struct RwLock {
    lifecycle: Lifecycle,
    init_kind: RwLockInitKind,
    ext_state: RwLockExtState,
    storage_bound: bool,
    arch_raw_lock_internal: bool,
    debug_lockdep_internal_deferred: bool,
    active_readers: usize,
    reader_counts: [usize; OWNER_SLOTS],
    writer: RwLockOwner,
    read_lock_count: usize,
    read_unlock_count: usize,
    read_blocked_count: usize,
    read_try_count: usize,
    read_try_failed_count: usize,
    write_lock_count: usize,
    write_unlock_count: usize,
    write_blocked_count: usize,
    write_try_count: usize,
    write_try_failed_count: usize,
}

#[allow(dead_code)]
impl RwLock {
    pub const fn new_static() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            init_kind: RwLockInitKind::StaticInitializer,
            ext_state: RwLockExtState::Unlocked,
            storage_bound: false,
            arch_raw_lock_internal: false,
            debug_lockdep_internal_deferred: false,
            active_readers: 0,
            reader_counts: [0; OWNER_SLOTS],
            writer: RwLockOwner::None,
            read_lock_count: 0,
            read_unlock_count: 0,
            read_blocked_count: 0,
            read_try_count: 0,
            read_try_failed_count: 0,
            write_lock_count: 0,
            write_unlock_count: 0,
            write_blocked_count: 0,
            write_try_count: 0,
            write_try_failed_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn init_kind(&self) -> RwLockInitKind {
        self.init_kind
    }

    pub const fn ext_state(&self) -> RwLockExtState {
        self.ext_state
    }

    pub const fn storage_bound(&self) -> bool {
        self.storage_bound
    }

    pub const fn arch_raw_lock_internal(&self) -> bool {
        self.arch_raw_lock_internal
    }

    pub const fn debug_lockdep_internal_deferred(&self) -> bool {
        self.debug_lockdep_internal_deferred
    }

    pub const fn active_readers(&self) -> usize {
        self.active_readers
    }

    pub const fn writer(&self) -> RwLockOwner {
        self.writer
    }

    pub const fn read_lock_count(&self) -> usize {
        self.read_lock_count
    }

    pub const fn read_unlock_count(&self) -> usize {
        self.read_unlock_count
    }

    pub const fn read_blocked_count(&self) -> usize {
        self.read_blocked_count
    }

    pub const fn read_try_count(&self) -> usize {
        self.read_try_count
    }

    pub const fn read_try_failed_count(&self) -> usize {
        self.read_try_failed_count
    }

    pub const fn write_lock_count(&self) -> usize {
        self.write_lock_count
    }

    pub const fn write_unlock_count(&self) -> usize {
        self.write_unlock_count
    }

    pub const fn write_blocked_count(&self) -> usize {
        self.write_blocked_count
    }

    pub const fn write_try_count(&self) -> usize {
        self.write_try_count
    }

    pub const fn write_try_failed_count(&self) -> usize {
        self.write_try_failed_count
    }

    pub const fn unlocked(&self) -> bool {
        matches!(self.ext_state, RwLockExtState::Unlocked)
            && self.active_readers == 0
            && matches!(self.writer, RwLockOwner::None)
    }

    pub fn ready(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.storage_bound
            && self.arch_raw_lock_internal
            && self.debug_lockdep_internal_deferred
            && self.unlocked()
    }

    pub fn boot_init_task_write_guard_completed(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.write_lock_count > 0
            && self.write_unlock_count == self.write_lock_count
            && self.unlocked()
    }

    pub fn preset_static(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.init_kind != RwLockInitKind::StaticInitializer
        {
            return self.failed_transition(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        self.storage_bound = true;
        self.arch_raw_lock_internal = true;
        self.debug_lockdep_internal_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.storage_bound {
            return self.failed_transition(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }

        self.ext_state = RwLockExtState::Unlocked;
        self.active_readers = 0;
        self.reader_counts = [0; OWNER_SLOTS];
        self.writer = RwLockOwner::None;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn read_lock_owner(&mut self, owner: RwLockOwner) -> Result<RwLockReadOutcome, EventError> {
        if self.lifecycle.state() != State::Ready || !owner.valid() {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        }

        if self.writer != RwLockOwner::None {
            self.read_blocked_count = self.read_blocked_count.wrapping_add(1);
            return Ok(RwLockReadOutcome::Blocked);
        }

        let Some(index) = owner.reader_index() else {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        };
        let outcome = if self.active_readers == 0 {
            RwLockReadOutcome::Acquired
        } else {
            RwLockReadOutcome::Shared
        };
        self.reader_counts[index] = self.reader_counts[index].wrapping_add(1);
        self.active_readers = self.active_readers.wrapping_add(1);
        self.read_lock_count = self.read_lock_count.wrapping_add(1);
        self.ext_state = RwLockExtState::ReadHeld;
        Ok(outcome)
    }

    pub fn read_try_lock_owner(
        &mut self,
        owner: RwLockOwner,
    ) -> Result<RwLockReadOutcome, EventError> {
        self.read_try_count = self.read_try_count.wrapping_add(1);
        let outcome = self.read_lock_owner(owner)?;
        if outcome == RwLockReadOutcome::Blocked {
            self.read_try_failed_count = self.read_try_failed_count.wrapping_add(1);
        }
        Ok(outcome)
    }

    pub fn read_unlock_owner(&mut self, owner: RwLockOwner) -> EventResult {
        if self.lifecycle.state() != State::Ready || !owner.valid() {
            return self.failed_transition(LifecycleEvent::Disable, State::Ready, State::Ready);
        }

        let Some(index) = owner.reader_index() else {
            return self.failed_transition(LifecycleEvent::Disable, State::Ready, State::Ready);
        };
        if self.reader_counts[index] == 0 || self.active_readers == 0 {
            return self.failed_transition(LifecycleEvent::Disable, State::Ready, State::Ready);
        }

        self.reader_counts[index] -= 1;
        self.active_readers -= 1;
        self.read_unlock_count = self.read_unlock_count.wrapping_add(1);
        if self.active_readers == 0 {
            self.ext_state = RwLockExtState::Unlocked;
        }
        Ok(())
    }

    pub fn write_lock_owner(
        &mut self,
        owner: RwLockOwner,
    ) -> Result<RwLockWriteOutcome, EventError> {
        if self.lifecycle.state() != State::Ready || !owner.valid() || self.writer == owner {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        }

        if self.writer != RwLockOwner::None || self.active_readers != 0 {
            self.write_blocked_count = self.write_blocked_count.wrapping_add(1);
            return Ok(RwLockWriteOutcome::Blocked);
        }

        self.writer = owner;
        self.ext_state = RwLockExtState::WriteHeld;
        self.write_lock_count = self.write_lock_count.wrapping_add(1);
        Ok(RwLockWriteOutcome::Acquired)
    }

    pub fn write_try_lock_owner(
        &mut self,
        owner: RwLockOwner,
    ) -> Result<RwLockWriteOutcome, EventError> {
        self.write_try_count = self.write_try_count.wrapping_add(1);
        let outcome = self.write_lock_owner(owner)?;
        if outcome == RwLockWriteOutcome::Blocked {
            self.write_try_failed_count = self.write_try_failed_count.wrapping_add(1);
        }
        Ok(outcome)
    }

    pub fn write_unlock_owner(&mut self, owner: RwLockOwner) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.writer != owner || !owner.valid() {
            return self.failed_transition(LifecycleEvent::Disable, State::Ready, State::Ready);
        }

        self.writer = RwLockOwner::None;
        self.ext_state = RwLockExtState::Unlocked;
        self.write_unlock_count = self.write_unlock_count.wrapping_add(1);
        Ok(())
    }

    pub fn read_lock_task(
        &mut self,
        task_ref: CurrentTaskRef,
    ) -> Result<RwLockReadOutcome, EventError> {
        self.read_lock_owner(owner_from_task_ref(task_ref))
    }

    pub fn read_unlock_task(&mut self, task_ref: CurrentTaskRef) -> EventResult {
        self.read_unlock_owner(owner_from_task_ref(task_ref))
    }

    pub fn write_lock_task(
        &mut self,
        task_ref: CurrentTaskRef,
    ) -> Result<RwLockWriteOutcome, EventError> {
        self.write_lock_owner(owner_from_task_ref(task_ref))
    }

    pub fn write_unlock_task(&mut self, task_ref: CurrentTaskRef) -> EventResult {
        self.write_unlock_owner(owner_from_task_ref(task_ref))
    }

    fn failed_transition(
        &self,
        event: LifecycleEvent,
        expected: State,
        target: State,
    ) -> EventResult {
        failed_condition(event, self.lifecycle.state(), expected, target)
    }

    fn failed_error(&self, event: LifecycleEvent, expected: State, target: State) -> EventError {
        EventError::failed(
            EventErrorCode::ConditionFailed,
            event,
            self.lifecycle.state(),
            expected,
            target,
        )
    }
}

impl RwLockOwner {
    const fn valid(self) -> bool {
        !matches!(self, Self::None)
    }

    const fn reader_index(self) -> Option<usize> {
        match self {
            Self::BootInitTask => Some(0),
            Self::KernelInitTask => Some(1),
            Self::SmokeRwLockTask => Some(2),
            Self::None => None,
        }
    }
}

const fn owner_from_task_ref(task_ref: CurrentTaskRef) -> RwLockOwner {
    match task_ref {
        CurrentTaskRef::KernelInit => RwLockOwner::KernelInitTask,
        CurrentTaskRef::SmokeRwLock => RwLockOwner::SmokeRwLockTask,
        CurrentTaskRef::None
        | CurrentTaskRef::BootIdle
        | CurrentTaskRef::Kthreadd
        | CurrentTaskRef::UserChild
        | CurrentTaskRef::SmokeScheduler
        | CurrentTaskRef::SmokeMutex
        | CurrentTaskRef::SmokeRwsem => RwLockOwner::None,
    }
}
