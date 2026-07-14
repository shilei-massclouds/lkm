use super::{
    init_task::InitTask,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum MutexInitKind {
    StaticInitializer,
    RuntimeInit,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum MutexOwner {
    None,
    BootInitTask,
    KernelInitTask,
    #[cfg_attr(not(app_smoke), allow(dead_code))]
    SmokeMutexTask,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum MutexLockOutcome {
    Acquired,
    Blocked,
}

pub struct Mutex {
    lifecycle: Lifecycle,
    init_kind: MutexInitKind,
    storage_bound: bool,
    wait_queue_ready: bool,
    wait_lock_internal_deferred: bool,
    locked: bool,
    owner: MutexOwner,
    lock_entered_count: usize,
    unlock_exited_count: usize,
    contended_count: usize,
    wake_count: usize,
}

#[allow(dead_code)]
impl Mutex {
    pub const fn new_static() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            init_kind: MutexInitKind::StaticInitializer,
            storage_bound: false,
            wait_queue_ready: false,
            wait_lock_internal_deferred: false,
            locked: false,
            owner: MutexOwner::None,
            lock_entered_count: 0,
            unlock_exited_count: 0,
            contended_count: 0,
            wake_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn init_kind(&self) -> MutexInitKind {
        self.init_kind
    }

    pub const fn storage_bound(&self) -> bool {
        self.storage_bound
    }

    pub const fn wait_queue_ready(&self) -> bool {
        self.wait_queue_ready
    }

    pub const fn wait_lock_internal_deferred(&self) -> bool {
        self.wait_lock_internal_deferred
    }

    pub const fn locked(&self) -> bool {
        self.locked
    }

    pub const fn unlocked(&self) -> bool {
        !self.locked
    }

    pub const fn owner(&self) -> MutexOwner {
        self.owner
    }

    pub const fn lock_entered_count(&self) -> usize {
        self.lock_entered_count
    }

    pub const fn unlock_exited_count(&self) -> usize {
        self.unlock_exited_count
    }

    pub const fn contended_count(&self) -> usize {
        self.contended_count
    }

    pub const fn wake_count(&self) -> usize {
        self.wake_count
    }

    pub fn ready(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.storage_bound
            && self.wait_queue_ready
            && self.wait_lock_internal_deferred
            && !self.locked
            && self.owner == MutexOwner::None
    }

    pub fn boot_init_task_guard_completed(&self) -> bool {
        self.ready()
            && self.lock_entered_count > 0
            && self.unlock_exited_count == self.lock_entered_count
    }

    pub fn preset_static(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.init_kind != MutexInitKind::StaticInitializer
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.storage_bound = true;
        self.wait_lock_internal_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.storage_bound {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.wait_queue_ready = true;
        self.locked = false;
        self.owner = MutexOwner::None;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn lock_boot_init_task(&mut self, init_task: &InitTask) -> EventResult {
        if self.lifecycle.state() != State::Ready || init_task.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        match self.lock_owner(MutexOwner::BootInitTask)? {
            MutexLockOutcome::Acquired => Ok(()),
            MutexLockOutcome::Blocked => failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            ),
        }
    }

    pub fn unlock_boot_init_task(&mut self, init_task: &InitTask) -> EventResult {
        if self.lifecycle.state() != State::Ready || init_task.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.unlock_owner(MutexOwner::BootInitTask)
    }

    pub fn lock_owner(
        &mut self,
        owner: MutexOwner,
    ) -> Result<MutexLockOutcome, super::state::EventError> {
        if self.lifecycle.state() != State::Ready || matches!(owner, MutexOwner::None) {
            return Err(super::state::EventError::failed(
                super::state::EventErrorCode::ConditionFailed,
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            ));
        }

        if self.locked {
            self.contended_count = self.contended_count.wrapping_add(1);
            return Ok(MutexLockOutcome::Blocked);
        }

        self.locked = true;
        self.owner = owner;
        self.lock_entered_count = self.lock_entered_count.wrapping_add(1);
        Ok(MutexLockOutcome::Acquired)
    }

    pub fn unlock_owner(&mut self, owner: MutexOwner) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.locked || self.owner != owner {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.locked = false;
        self.owner = MutexOwner::None;
        self.unlock_exited_count = self.unlock_exited_count.wrapping_add(1);
        if self.wake_count < self.contended_count {
            self.wake_count = self.wake_count.wrapping_add(1);
        }
        Ok(())
    }
}
