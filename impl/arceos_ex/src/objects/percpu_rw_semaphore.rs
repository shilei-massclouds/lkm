use super::{
    per_cpu_storage::PerCpuStorage,
    state::{
        EventError, EventErrorCode, EventResult, Lifecycle, LifecycleEvent, State, failed_condition,
    },
    task::TaskRef,
};

const OWNER_SLOTS: usize = 4;
const BOOT_CPU_ID: usize = 0;

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RcuSyncExtState {
    Idle,
    WriterActive,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PerCpuRwSemaphoreInitKind {
    StaticInitializer,
    RuntimeInit,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PerCpuRwSemaphoreExtState {
    ReadersFast,
    WriterBlocked,
    WriterActive,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PerCpuRwSemaphoreOwner {
    None,
    BootTask,
    KernelInitTask,
    SmokeRwsemTask,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PerCpuRwSemaphoreReadOutcome {
    AcquiredFast,
    Blocked,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PerCpuRwSemaphoreWriteOutcome {
    Acquired,
    Blocked,
}

pub struct RcuSync {
    lifecycle: Lifecycle,
    ext_state: RcuSyncExtState,
    entered_count: usize,
    exited_count: usize,
    grace_period_count: usize,
}

#[allow(dead_code)]
impl RcuSync {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ext_state: RcuSyncExtState::Idle,
            entered_count: 0,
            exited_count: 0,
            grace_period_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ext_state(&self) -> RcuSyncExtState {
        self.ext_state
    }

    pub const fn entered_count(&self) -> usize {
        self.entered_count
    }

    pub const fn exited_count(&self) -> usize {
        self.exited_count
    }

    pub const fn grace_period_count(&self) -> usize {
        self.grace_period_count
    }

    pub const fn idle(&self) -> bool {
        matches!(self.ext_state, RcuSyncExtState::Idle)
    }

    pub fn preset(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return self.failed_transition(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        self.ext_state = RcuSyncExtState::Idle;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.idle() {
            return self.failed_transition(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enter(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.idle() {
            return self.failed_transition(LifecycleEvent::Enable, State::Ready, State::Ready);
        }

        self.ext_state = RcuSyncExtState::WriterActive;
        self.entered_count = self.entered_count.wrapping_add(1);
        Ok(())
    }

    pub fn exit(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !matches!(self.ext_state, RcuSyncExtState::WriterActive)
        {
            return self.failed_transition(LifecycleEvent::Disable, State::Ready, State::Ready);
        }

        self.ext_state = RcuSyncExtState::Idle;
        self.exited_count = self.exited_count.wrapping_add(1);
        self.grace_period_count = self.grace_period_count.wrapping_add(1);
        Ok(())
    }

    fn failed_transition(
        &self,
        event: LifecycleEvent,
        expected: State,
        target: State,
    ) -> EventResult {
        failed_condition(event, self.lifecycle.state(), expected, target)
    }
}

pub struct PerCpuRwSemaphore {
    lifecycle: Lifecycle,
    init_kind: PerCpuRwSemaphoreInitKind,
    ext_state: PerCpuRwSemaphoreExtState,
    rcu_sync: RcuSync,
    storage_bound: bool,
    percpu_read_counter_bound: bool,
    boot_cpu_read_available: bool,
    writer_wait_ready: bool,
    wait_queue_ready: bool,
    block: bool,
    active_readers: usize,
    reader_counts: [usize; OWNER_SLOTS],
    writer: PerCpuRwSemaphoreOwner,
    pending_writer: PerCpuRwSemaphoreOwner,
    read_lock_count: usize,
    read_unlock_count: usize,
    read_blocked_count: usize,
    read_try_count: usize,
    read_try_failed_count: usize,
    write_lock_count: usize,
    write_unlock_count: usize,
    writer_blocked_count: usize,
    reader_drain_count: usize,
    wake_count: usize,
}

#[allow(dead_code)]
impl PerCpuRwSemaphore {
    pub const fn new_static() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            init_kind: PerCpuRwSemaphoreInitKind::StaticInitializer,
            ext_state: PerCpuRwSemaphoreExtState::ReadersFast,
            rcu_sync: RcuSync::new(),
            storage_bound: false,
            percpu_read_counter_bound: false,
            boot_cpu_read_available: false,
            writer_wait_ready: false,
            wait_queue_ready: false,
            block: false,
            active_readers: 0,
            reader_counts: [0; OWNER_SLOTS],
            writer: PerCpuRwSemaphoreOwner::None,
            pending_writer: PerCpuRwSemaphoreOwner::None,
            read_lock_count: 0,
            read_unlock_count: 0,
            read_blocked_count: 0,
            read_try_count: 0,
            read_try_failed_count: 0,
            write_lock_count: 0,
            write_unlock_count: 0,
            writer_blocked_count: 0,
            reader_drain_count: 0,
            wake_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn init_kind(&self) -> PerCpuRwSemaphoreInitKind {
        self.init_kind
    }

    pub const fn ext_state(&self) -> PerCpuRwSemaphoreExtState {
        self.ext_state
    }

    pub const fn rcu_sync(&self) -> &RcuSync {
        &self.rcu_sync
    }

    pub const fn storage_bound(&self) -> bool {
        self.storage_bound
    }

    pub const fn percpu_read_counter_bound(&self) -> bool {
        self.percpu_read_counter_bound
    }

    pub const fn boot_cpu_read_available(&self) -> bool {
        self.boot_cpu_read_available
    }

    pub const fn wait_queue_ready(&self) -> bool {
        self.wait_queue_ready
    }

    pub const fn block_flag_clear(&self) -> bool {
        !self.block
    }

    pub const fn active_readers(&self) -> usize {
        self.active_readers
    }

    pub const fn writer(&self) -> PerCpuRwSemaphoreOwner {
        self.writer
    }

    pub const fn pending_writer(&self) -> PerCpuRwSemaphoreOwner {
        self.pending_writer
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

    pub const fn writer_blocked_count(&self) -> usize {
        self.writer_blocked_count
    }

    pub const fn reader_drain_count(&self) -> usize {
        self.reader_drain_count
    }

    pub const fn wake_count(&self) -> usize {
        self.wake_count
    }

    pub fn ready(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.storage_bound
            && self.percpu_read_counter_bound
            && self.boot_cpu_read_available
            && self.writer_wait_ready
            && self.wait_queue_ready
            && !self.block
            && self.active_readers == 0
            && self.writer == PerCpuRwSemaphoreOwner::None
            && self.pending_writer == PerCpuRwSemaphoreOwner::None
            && self.rcu_sync.state() == State::Ready
            && self.rcu_sync.idle()
    }

    pub fn readers_fast(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.ext_state == PerCpuRwSemaphoreExtState::ReadersFast
            && !self.block
            && self.writer == PerCpuRwSemaphoreOwner::None
            && self.rcu_sync.idle()
    }

    pub fn boot_init_task_read_guard_completed(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.read_lock_count > 0
            && self.read_unlock_count == self.read_lock_count
            && self.active_readers == 0
            && self.readers_fast()
    }

    pub fn preset_static(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.init_kind != PerCpuRwSemaphoreInitKind::StaticInitializer
        {
            return self.failed_transition(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        self.rcu_sync.preset()?;
        self.storage_bound = true;
        self.percpu_read_counter_bound = true;
        self.writer_wait_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn preset_static_with_per_cpu_storage(
        &mut self,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if per_cpu_storage.state() != State::Prepared && per_cpu_storage.state() != State::Ready {
            return self.failed_transition(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        self.preset_static()
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.storage_bound
            || !self.percpu_read_counter_bound
            || !self.writer_wait_ready
        {
            return self.failed_transition(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }

        self.rcu_sync.setup()?;
        self.wait_queue_ready = true;
        self.boot_cpu_read_available = true;
        self.block = false;
        self.ext_state = PerCpuRwSemaphoreExtState::ReadersFast;
        self.writer = PerCpuRwSemaphoreOwner::None;
        self.pending_writer = PerCpuRwSemaphoreOwner::None;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn read_lock_owner(
        &mut self,
        owner: PerCpuRwSemaphoreOwner,
        cpu_id: usize,
    ) -> Result<PerCpuRwSemaphoreReadOutcome, EventError> {
        if self.lifecycle.state() != State::Ready
            || !self.boot_cpu_read_available
            || !owner.valid()
            || cpu_id != BOOT_CPU_ID
        {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        }

        if self.block || self.writer != PerCpuRwSemaphoreOwner::None {
            self.read_blocked_count = self.read_blocked_count.wrapping_add(1);
            return Ok(PerCpuRwSemaphoreReadOutcome::Blocked);
        }

        let Some(index) = owner.reader_index() else {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        };
        self.reader_counts[index] = self.reader_counts[index].wrapping_add(1);
        self.active_readers = self.active_readers.wrapping_add(1);
        self.read_lock_count = self.read_lock_count.wrapping_add(1);
        self.ext_state = PerCpuRwSemaphoreExtState::ReadersFast;
        Ok(PerCpuRwSemaphoreReadOutcome::AcquiredFast)
    }

    pub fn read_try_lock_owner(
        &mut self,
        owner: PerCpuRwSemaphoreOwner,
        cpu_id: usize,
    ) -> Result<PerCpuRwSemaphoreReadOutcome, EventError> {
        self.read_try_count = self.read_try_count.wrapping_add(1);
        let outcome = self.read_lock_owner(owner, cpu_id)?;
        if outcome == PerCpuRwSemaphoreReadOutcome::Blocked {
            self.read_try_failed_count = self.read_try_failed_count.wrapping_add(1);
        }
        Ok(outcome)
    }

    pub fn read_unlock_owner(
        &mut self,
        owner: PerCpuRwSemaphoreOwner,
        cpu_id: usize,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || !owner.valid() || cpu_id != BOOT_CPU_ID {
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
        if self.block && self.active_readers == 0 {
            self.reader_drain_count = self.reader_drain_count.wrapping_add(1);
            self.wake_count = self.wake_count.wrapping_add(1);
        }
        Ok(())
    }

    pub fn write_lock_owner(
        &mut self,
        owner: PerCpuRwSemaphoreOwner,
    ) -> Result<PerCpuRwSemaphoreWriteOutcome, EventError> {
        if self.lifecycle.state() != State::Ready || !owner.valid() {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        }

        if self.writer == owner {
            return Err(self.failed_error(LifecycleEvent::Enable, State::Ready, State::Ready));
        }

        if self.writer != PerCpuRwSemaphoreOwner::None {
            self.writer_blocked_count = self.writer_blocked_count.wrapping_add(1);
            return Ok(PerCpuRwSemaphoreWriteOutcome::Blocked);
        }

        if self.block {
            if self.pending_writer == owner && self.active_readers == 0 {
                self.pending_writer = PerCpuRwSemaphoreOwner::None;
                self.writer = owner;
                self.ext_state = PerCpuRwSemaphoreExtState::WriterActive;
                self.write_lock_count = self.write_lock_count.wrapping_add(1);
                return Ok(PerCpuRwSemaphoreWriteOutcome::Acquired);
            }

            self.writer_blocked_count = self.writer_blocked_count.wrapping_add(1);
            return Ok(PerCpuRwSemaphoreWriteOutcome::Blocked);
        }

        self.block = true;
        self.rcu_sync.enter()?;
        if self.active_readers == 0 {
            self.writer = owner;
            self.ext_state = PerCpuRwSemaphoreExtState::WriterActive;
            self.write_lock_count = self.write_lock_count.wrapping_add(1);
            Ok(PerCpuRwSemaphoreWriteOutcome::Acquired)
        } else {
            self.pending_writer = owner;
            self.ext_state = PerCpuRwSemaphoreExtState::WriterBlocked;
            self.writer_blocked_count = self.writer_blocked_count.wrapping_add(1);
            Ok(PerCpuRwSemaphoreWriteOutcome::Blocked)
        }
    }

    pub fn write_unlock_owner(&mut self, owner: PerCpuRwSemaphoreOwner) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.writer != owner || !owner.valid() {
            return self.failed_transition(LifecycleEvent::Disable, State::Ready, State::Ready);
        }

        self.writer = PerCpuRwSemaphoreOwner::None;
        self.block = false;
        self.ext_state = PerCpuRwSemaphoreExtState::ReadersFast;
        self.write_unlock_count = self.write_unlock_count.wrapping_add(1);
        self.wake_count = self.wake_count.wrapping_add(1);
        self.rcu_sync.exit()
    }

    pub fn read_lock_task(
        &mut self,
        task_ref: TaskRef,
    ) -> Result<PerCpuRwSemaphoreReadOutcome, EventError> {
        self.read_lock_owner(owner_from_task_ref(task_ref), BOOT_CPU_ID)
    }

    pub fn read_unlock_task(&mut self, task_ref: TaskRef) -> EventResult {
        self.read_unlock_owner(owner_from_task_ref(task_ref), BOOT_CPU_ID)
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

impl PerCpuRwSemaphoreOwner {
    const fn valid(self) -> bool {
        !matches!(self, Self::None)
    }

    const fn reader_index(self) -> Option<usize> {
        match self {
            Self::BootTask => Some(0),
            Self::KernelInitTask => Some(1),
            Self::SmokeRwsemTask => Some(2),
            Self::None => None,
        }
    }
}

const fn owner_from_task_ref(task_ref: TaskRef) -> PerCpuRwSemaphoreOwner {
    match task_ref {
        TaskRef::KERNEL_INIT => PerCpuRwSemaphoreOwner::KernelInitTask,
        TaskRef::SMOKE_RWSEM => PerCpuRwSemaphoreOwner::SmokeRwsemTask,
        _ => PerCpuRwSemaphoreOwner::None,
    }
}
