use crate::trace::{self, Checkpoint};

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum State {
    Base,
    Prepared,
    Ready,
    Online,
    Destroyed,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum LifecycleEvent {
    Preset,
    Setup,
    Enable,
    Cleanup,
}

impl LifecycleEvent {
    const fn bit(self) -> u8 {
        match self {
            Self::Preset => 1 << 0,
            Self::Setup => 1 << 1,
            Self::Enable => 1 << 2,
            Self::Cleanup => 1 << 3,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum EventErrorCode {
    DuplicateLifecycleEvent,
    InvalidTransition,
    UnexpectedState,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct EventError {
    pub code: EventErrorCode,
    pub event: LifecycleEvent,
    pub actual: State,
    pub expected: State,
    pub target: State,
}

impl EventError {
    const fn new(
        code: EventErrorCode,
        event: LifecycleEvent,
        actual: State,
        expected: State,
        target: State,
    ) -> Self {
        Self {
            code,
            event,
            actual,
            expected,
            target,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum EventResult {
    Success,
    Failed(EventError),
    Blocked(EventError),
}

impl EventResult {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }
}

pub struct Lifecycle {
    state: State,
    seen_events: u8,
}

impl Lifecycle {
    pub const fn new(initial_state: State) -> Self {
        Self {
            state: initial_state,
            seen_events: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.state
    }

    pub fn transition(
        &mut self,
        event: LifecycleEvent,
        expected: State,
        target: State,
        checkpoint: Checkpoint,
    ) -> EventResult {
        if self.seen_events & event.bit() != 0 {
            return EventResult::Failed(EventError::new(
                EventErrorCode::DuplicateLifecycleEvent,
                event,
                self.state,
                expected,
                target,
            ));
        }

        if self.state != expected {
            return EventResult::Blocked(EventError::new(
                EventErrorCode::UnexpectedState,
                event,
                self.state,
                expected,
                target,
            ));
        }

        if !is_allowed_lifecycle_transition(expected, event, target) {
            return EventResult::Failed(EventError::new(
                EventErrorCode::InvalidTransition,
                event,
                self.state,
                expected,
                target,
            ));
        }

        self.state = target;
        self.seen_events |= event.bit();
        trace::checkpoint(checkpoint);
        EventResult::Success
    }
}

const fn is_allowed_lifecycle_transition(
    source: State,
    event: LifecycleEvent,
    target: State,
) -> bool {
    matches!(
        (source, event, target),
        (State::Base, LifecycleEvent::Preset, State::Prepared)
            | (State::Base, LifecycleEvent::Preset, State::Ready)
            | (State::Base, LifecycleEvent::Setup, State::Ready)
            | (State::Prepared, LifecycleEvent::Setup, State::Ready)
            | (State::Prepared, LifecycleEvent::Enable, State::Online)
            | (State::Ready, LifecycleEvent::Enable, State::Online)
            | (State::Ready, LifecycleEvent::Cleanup, State::Destroyed)
            | (State::Online, LifecycleEvent::Cleanup, State::Destroyed)
    )
}
