use super::state::{
    EventError, EventErrorCode, EventResult, Lifecycle, LifecycleEvent, State, failed_condition,
};
use crate::checkpoint::Checkpoint;

#[cfg_attr(not(app_smoke), allow(dead_code))]
const COMPLETION_DONE_ALL: usize = usize::MAX;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CompletionExtState {
    Pending,
    Completed,
    CompletedAll,
}

// Extended completion observations are exercised by smoke/KUnit configurations.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct SimpleWaitQueue {
    lifecycle: Lifecycle,
    wake_one_committed: bool,
    wake_all_committed: bool,
    waiter_enqueued: bool,
    waiter_finished: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SimpleWaitQueue {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            wake_one_committed: false,
            wake_all_committed: false,
            waiter_enqueued: false,
            waiter_finished: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn wake_one_committed(&self) -> bool {
        self.wake_one_committed
    }

    pub const fn wake_all_committed(&self) -> bool {
        self.wake_all_committed
    }

    pub const fn waiter_enqueued(&self) -> bool {
        self.waiter_enqueued
    }

    pub const fn waiter_finished(&self) -> bool {
        self.waiter_finished
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SimpleWaitQueueReady,
        )
    }

    pub fn wake_one(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.wake_one_committed = true;
        Ok(())
    }

    pub fn wake_all(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.wake_all_committed = true;
        Ok(())
    }

    pub fn prepare_wait(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.waiter_enqueued = true;
        Ok(())
    }

    pub fn finish_wait(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.waiter_finished = true;
        Ok(())
    }
}

pub struct Completion {
    lifecycle: Lifecycle,
    ext_state: CompletionExtState,
    done: usize,
    wait_queue: SimpleWaitQueue,
    storage_bound: bool,
    handle_published: bool,
    complete_committed: bool,
    complete_all_committed: bool,
    waiter_enqueued: bool,
    waiter_finished: bool,
    done_observed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Completion {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ext_state: CompletionExtState::Pending,
            done: 0,
            wait_queue: SimpleWaitQueue::new(),
            storage_bound: false,
            handle_published: false,
            complete_committed: false,
            complete_all_committed: false,
            waiter_enqueued: false,
            waiter_finished: false,
            done_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ext_state(&self) -> CompletionExtState {
        self.ext_state
    }

    pub const fn done_count(&self) -> usize {
        self.done
    }

    pub const fn wait_queue(&self) -> &SimpleWaitQueue {
        &self.wait_queue
    }

    pub const fn storage_bound(&self) -> bool {
        self.storage_bound
    }

    pub const fn owns_wait_queue(&self) -> bool {
        true
    }

    pub const fn handle_published(&self) -> bool {
        self.handle_published
    }

    pub fn pending(&self) -> bool {
        self.ext_state == CompletionExtState::Pending && self.done == 0
    }

    pub fn completed(&self) -> bool {
        self.ext_state == CompletionExtState::Completed
            || self.ext_state == CompletionExtState::CompletedAll
    }

    pub fn completed_all(&self) -> bool {
        self.ext_state == CompletionExtState::CompletedAll
    }

    pub const fn token_available(&self) -> bool {
        self.done != 0
    }

    pub const fn complete_committed(&self) -> bool {
        self.complete_committed
    }

    pub const fn complete_all_committed(&self) -> bool {
        self.complete_all_committed
    }

    pub const fn wakes_one_waiter(&self) -> bool {
        self.wait_queue.wake_one_committed()
    }

    pub const fn wakes_all_waiters(&self) -> bool {
        self.wait_queue.wake_all_committed()
    }

    pub const fn waiter_enqueued(&self) -> bool {
        self.waiter_enqueued
    }

    pub const fn waiter_finished(&self) -> bool {
        self.waiter_finished
    }

    pub const fn done_observed(&self) -> bool {
        self.done_observed
    }

    pub fn preset(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.storage_bound = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CompletionPrepared,
        )
    }

    pub fn setup(&mut self) -> EventResult {
        let source = self.lifecycle.state();
        if source != State::Base && source != State::Prepared {
            return failed_condition(LifecycleEvent::Setup, source, State::Base, State::Ready);
        }
        if self.wait_queue.state() == State::Base {
            self.wait_queue.setup()?;
        }
        if self.wait_queue.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.wait_queue.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.storage_bound = true;
        self.ext_state = CompletionExtState::Pending;
        self.done = 0;
        self.handle_published = false;
        self.complete_committed = false;
        self.complete_all_committed = false;
        self.waiter_enqueued = false;
        self.waiter_finished = false;
        self.done_observed = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            source,
            State::Ready,
            Checkpoint::CompletionReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.wait_queue.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.handle_published = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::CompletionOnline,
        )
    }

    pub fn complete(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        match self.ext_state {
            CompletionExtState::Pending | CompletionExtState::Completed => {
                self.ext_state = CompletionExtState::Completed;
                self.done = self.done.saturating_add(1);
            }
            CompletionExtState::CompletedAll => {}
        }
        self.complete_committed = true;
        self.wait_queue.wake_one()
    }

    pub fn complete_all(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.ext_state = CompletionExtState::CompletedAll;
        self.done = COMPLETION_DONE_ALL;
        self.complete_all_committed = true;
        self.wait_queue.wake_all()
    }

    pub fn wait(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.wait_queue.prepare_wait()?;
        self.waiter_enqueued = true;

        match self.ext_state {
            CompletionExtState::Pending => {
                self.wait_queue.finish_wait()?;
                self.waiter_finished = true;
                Err(EventError::blocked(
                    EventErrorCode::ConditionFailed,
                    LifecycleEvent::Enable,
                    self.lifecycle.state(),
                    State::Online,
                    State::Online,
                ))
            }
            CompletionExtState::Completed => {
                self.done = 0;
                self.ext_state = CompletionExtState::Pending;
                self.wait_queue.finish_wait()?;
                self.waiter_finished = true;
                Ok(())
            }
            CompletionExtState::CompletedAll => {
                self.wait_queue.finish_wait()?;
                self.waiter_finished = true;
                Ok(())
            }
        }
    }

    pub fn try_wait(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        match self.ext_state {
            CompletionExtState::Pending => failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            ),
            CompletionExtState::Completed => {
                self.done = 0;
                self.ext_state = CompletionExtState::Pending;
                Ok(())
            }
            CompletionExtState::CompletedAll => Ok(()),
        }
    }

    pub fn reinit(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.ext_state = CompletionExtState::Pending;
        self.done = 0;
        self.complete_committed = false;
        self.complete_all_committed = false;
        Ok(())
    }

    pub fn done(&mut self) -> usize {
        self.done_observed = true;
        self.done
    }
}
