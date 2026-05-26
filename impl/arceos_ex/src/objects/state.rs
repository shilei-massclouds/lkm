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

    pub const fn code(self) -> u8 {
        match self {
            Self::Preset => b'P',
            Self::Setup => b'S',
            Self::Enable => b'E',
            Self::Cleanup => b'C',
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum EventErrorCode {
    DuplicateLifecycleEvent,
    InvalidTransition,
    ConditionFailed,
    UnexpectedState,
}

impl EventErrorCode {
    pub const fn code(self) -> u8 {
        match self {
            Self::DuplicateLifecycleEvent => b'D',
            Self::InvalidTransition => b'I',
            Self::ConditionFailed => b'C',
            Self::UnexpectedState => b'U',
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum EventErrorKind {
    Failed,
    Blocked,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct EventError {
    pub kind: EventErrorKind,
    pub code: EventErrorCode,
    pub event: LifecycleEvent,
    pub actual: State,
    pub expected: State,
    pub target: State,
}

impl EventError {
    const fn new(
        kind: EventErrorKind,
        code: EventErrorCode,
        event: LifecycleEvent,
        actual: State,
        expected: State,
        target: State,
    ) -> Self {
        Self {
            kind,
            code,
            event,
            actual,
            expected,
            target,
        }
    }

    pub const fn failed(
        code: EventErrorCode,
        event: LifecycleEvent,
        actual: State,
        expected: State,
        target: State,
    ) -> Self {
        Self::new(
            EventErrorKind::Failed,
            code,
            event,
            actual,
            expected,
            target,
        )
    }

    pub const fn blocked(
        code: EventErrorCode,
        event: LifecycleEvent,
        actual: State,
        expected: State,
        target: State,
    ) -> Self {
        Self::new(
            EventErrorKind::Blocked,
            code,
            event,
            actual,
            expected,
            target,
        )
    }

    pub const fn error_code(self) -> u8 {
        self.code.code()
    }

    pub const fn event_code(self) -> u8 {
        self.event.code()
    }

    pub const fn actual_state_code(self) -> u8 {
        self.actual.code()
    }

    pub const fn expected_state_code(self) -> u8 {
        self.expected.code()
    }

    pub const fn target_state_code(self) -> u8 {
        self.target.code()
    }
}

impl State {
    pub const fn code(self) -> u8 {
        match self {
            Self::Base => b'B',
            Self::Prepared => b'P',
            Self::Ready => b'R',
            Self::Online => b'O',
            Self::Destroyed => b'D',
        }
    }
}

pub type EventResult = Result<(), EventError>;

pub const fn failed_condition(
    event: LifecycleEvent,
    actual: State,
    expected: State,
    target: State,
) -> EventResult {
    Err(EventError::failed(
        EventErrorCode::ConditionFailed,
        event,
        actual,
        expected,
        target,
    ))
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
            return Err(EventError::failed(
                EventErrorCode::DuplicateLifecycleEvent,
                event,
                self.state,
                expected,
                target,
            ));
        }

        if self.state != expected {
            return Err(EventError::blocked(
                EventErrorCode::UnexpectedState,
                event,
                self.state,
                expected,
                target,
            ));
        }

        if !is_allowed_lifecycle_transition(expected, event, target) {
            return Err(EventError::failed(
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
        Ok(())
    }

    pub fn adopt_transition(
        &mut self,
        event: LifecycleEvent,
        expected: State,
        target: State,
    ) -> EventResult {
        if self.seen_events & event.bit() != 0 {
            return Err(EventError::failed(
                EventErrorCode::DuplicateLifecycleEvent,
                event,
                self.state,
                expected,
                target,
            ));
        }

        if self.state != expected {
            return Err(EventError::blocked(
                EventErrorCode::UnexpectedState,
                event,
                self.state,
                expected,
                target,
            ));
        }

        if !is_allowed_lifecycle_transition(expected, event, target) {
            return Err(EventError::failed(
                EventErrorCode::InvalidTransition,
                event,
                self.state,
                expected,
                target,
            ));
        }

        self.state = target;
        self.seen_events |= event.bit();
        Ok(())
    }
}

const fn is_allowed_lifecycle_transition(
    source: State,
    event: LifecycleEvent,
    target: State,
) -> bool {
    if matches_state(source, State::Base) {
        if matches_event(event, LifecycleEvent::Preset) {
            return matches_state(target, State::Prepared) || matches_state(target, State::Ready);
        }
        if matches_event(event, LifecycleEvent::Setup) {
            return matches_state(target, State::Ready);
        }
        return false;
    }

    if matches_state(source, State::Prepared) {
        if matches_event(event, LifecycleEvent::Setup) {
            return matches_state(target, State::Ready);
        }
        if matches_event(event, LifecycleEvent::Enable) {
            return matches_state(target, State::Online);
        }
        return false;
    }

    if matches_state(source, State::Ready) {
        if matches_event(event, LifecycleEvent::Enable) {
            return matches_state(target, State::Online);
        }
        if matches_event(event, LifecycleEvent::Cleanup) {
            return matches_state(target, State::Destroyed);
        }
        return false;
    }

    if matches_state(source, State::Online) && matches_event(event, LifecycleEvent::Cleanup) {
        return matches_state(target, State::Destroyed);
    }

    false
}

const fn matches_state(left: State, right: State) -> bool {
    left as u8 == right as u8
}

const fn matches_event(left: LifecycleEvent, right: LifecycleEvent) -> bool {
    left as u8 == right as u8
}
