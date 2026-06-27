#[cfg(target_arch = "riscv64")]
use core::arch::global_asm;

use crate::trace::Checkpoint;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum State {
    Base,
    Prepared,
    Ready,
    Online,
    Offline,
    Destroyed,
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum LifecycleEvent {
    Preset,
    Setup,
    Enable,
    Disable,
    Cleanup,
}

impl LifecycleEvent {
    const fn bit(self) -> u8 {
        match self {
            Self::Preset => 1 << 0,
            Self::Setup => 1 << 1,
            Self::Enable => 1 << 2,
            Self::Disable => 1 << 3,
            Self::Cleanup => 1 << 4,
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Preset => b'P',
            Self::Setup => b'S',
            Self::Enable => b'E',
            Self::Disable => b'D',
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
    pub diagnostic: Option<FailureDiagnostic>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct FailureDiagnostic {
    pub phase: &'static str,
    pub step: &'static str,
    pub object: &'static str,
    pub check: &'static str,
    pub first_failed: &'static str,
}

impl FailureDiagnostic {
    pub const fn new(
        phase: &'static str,
        step: &'static str,
        object: &'static str,
        check: &'static str,
        first_failed: &'static str,
    ) -> Self {
        Self {
            phase,
            step,
            object,
            check,
            first_failed,
        }
    }
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
            diagnostic: None,
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

    pub const fn diagnostic(self) -> Option<FailureDiagnostic> {
        self.diagnostic
    }

    pub fn with_diagnostic(mut self, diagnostic: FailureDiagnostic) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }

    pub fn with_diagnostic_if_absent(mut self, diagnostic: FailureDiagnostic) -> Self {
        if self.diagnostic.is_none() {
            self.diagnostic = Some(diagnostic);
        }
        self
    }
}

impl State {
    pub const fn code(self) -> u8 {
        match self {
            Self::Base => b'B',
            Self::Prepared => b'P',
            Self::Ready => b'R',
            Self::Online => b'O',
            Self::Offline => b'F',
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

pub fn failed_condition_with_diagnostic(
    event: LifecycleEvent,
    actual: State,
    expected: State,
    target: State,
    diagnostic: FailureDiagnostic,
) -> EventResult {
    Err(EventError::failed(
        EventErrorCode::ConditionFailed,
        event,
        actual,
        expected,
        target,
    )
    .with_diagnostic(diagnostic))
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
        crate::trace::checkpoint(checkpoint);
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

// Lifecycle is the object model's always-on guardrail: it is used from the
// pre-VM entry path through the later kernel payload handoff. Keep the
// transition legality check in hand-written assembly on RISC-V so this core
// mechanism has deterministic code generation even before EarlyVm is enabled.
// Future hot or pre-VM lifecycle operations may use the same treatment if they
// need stronger performance or control-flow guarantees.
#[cfg(target_arch = "riscv64")]
global_asm!(
    r#"
    .section .text.arceos_ex_is_allowed_lifecycle_transition, "ax"
    .align 2
    .globl arceos_ex_is_allowed_lifecycle_transition
    .type arceos_ex_is_allowed_lifecycle_transition, @function
arceos_ex_is_allowed_lifecycle_transition:
    andi a0, a0, 0xff
    andi a1, a1, 0xff
    andi a2, a2, 0xff
    slli t0, a0, 8
    slli t1, a1, 4
    or   t0, t0, t1
    or   t0, t0, a2

    li   t1, 0x001
    beq  t0, t1, 1f
    li   t1, 0x002
    beq  t0, t1, 1f
    li   t1, 0x012
    beq  t0, t1, 1f
    li   t1, 0x112
    beq  t0, t1, 1f
    li   t1, 0x123
    beq  t0, t1, 1f
    li   t1, 0x223
    beq  t0, t1, 1f
    li   t1, 0x245
    beq  t0, t1, 1f
    li   t1, 0x334
    beq  t0, t1, 1f
    li   t1, 0x345
    beq  t0, t1, 1f
    li   t1, 0x445
    beq  t0, t1, 1f

    li   a0, 0
    ret
1:
    li   a0, 1
    ret
    .size arceos_ex_is_allowed_lifecycle_transition, . - arceos_ex_is_allowed_lifecycle_transition
"#,
);

#[cfg(target_arch = "riscv64")]
unsafe extern "C" {
    fn arceos_ex_is_allowed_lifecycle_transition(source: u8, event: u8, target: u8) -> u8;
}

fn is_allowed_lifecycle_transition(source: State, event: LifecycleEvent, target: State) -> bool {
    #[cfg(target_arch = "riscv64")]
    {
        return unsafe {
            arceos_ex_is_allowed_lifecycle_transition(source as u8, event as u8, target as u8) != 0
        };
    }

    #[cfg(not(target_arch = "riscv64"))]
    {
        is_allowed_lifecycle_transition_rust(source, event, target)
    }
}

#[cfg(not(target_arch = "riscv64"))]
const fn is_allowed_lifecycle_transition_rust(
    source: State,
    event: LifecycleEvent,
    target: State,
) -> bool {
    let key = transition_key(source, event, target);

    key == transition_key(State::Base, LifecycleEvent::Preset, State::Prepared)
        || key == transition_key(State::Base, LifecycleEvent::Preset, State::Ready)
        || key == transition_key(State::Base, LifecycleEvent::Setup, State::Ready)
        || key == transition_key(State::Prepared, LifecycleEvent::Setup, State::Ready)
        || key == transition_key(State::Prepared, LifecycleEvent::Enable, State::Online)
        || key == transition_key(State::Ready, LifecycleEvent::Enable, State::Online)
        || key == transition_key(State::Ready, LifecycleEvent::Cleanup, State::Destroyed)
        || key == transition_key(State::Online, LifecycleEvent::Disable, State::Offline)
        || key == transition_key(State::Online, LifecycleEvent::Cleanup, State::Destroyed)
        || key == transition_key(State::Offline, LifecycleEvent::Cleanup, State::Destroyed)
}

#[cfg(not(target_arch = "riscv64"))]
const fn transition_key(source: State, event: LifecycleEvent, target: State) -> u16 {
    ((source as u16) << 8) | ((event as u16) << 4) | target as u16
}
