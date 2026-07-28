use super::{
    exception_type::install_page_fault_policy,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

/// CPU-local binding for instruction, load and store page faults.
pub struct PageFaultExceptionType {
    lifecycle: Lifecycle,
    fallback_ready: bool,
    handler_ready: bool,
    context_matrix_ready: bool,
    recovery_online: bool,
}

impl PageFaultExceptionType {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fallback_ready: false,
            handler_ready: false,
            context_matrix_ready: false,
            recovery_online: false,
        }
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
        self.fallback_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self, exception_state: State) -> EventResult {
        if exception_state != State::Prepared
            || self.lifecycle.state() != State::Prepared
            || !self.fallback_ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }
        install_page_fault_policy();
        self.handler_ready = true;
        self.context_matrix_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self, exception_state: State) -> EventResult {
        if exception_state != State::Ready
            || self.lifecycle.state() != State::Ready
            || !self.handler_ready
            || !self.context_matrix_ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.recovery_online = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fallback_ready(&self) -> bool {
        self.fallback_ready
    }

    pub const fn handler_ready(&self) -> bool {
        self.handler_ready
    }

    pub const fn context_matrix_ready(&self) -> bool {
        self.context_matrix_ready
    }

    pub const fn recovery_online(&self) -> bool {
        self.recovery_online
    }

    pub(crate) fn adopt_secondary_online(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Base,
                State::Online,
            );
        }
        self.fallback_ready = true;
        self.handler_ready = true;
        self.context_matrix_ready = true;
        self.recovery_online = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}
