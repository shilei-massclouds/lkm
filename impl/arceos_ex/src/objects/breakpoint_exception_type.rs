use super::{
    exception_type::install_breakpoint_policy,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

/// CPU-local breakpoint binding and explicit nesting policy.
pub struct BreakpointExceptionType {
    lifecycle: Lifecycle,
    fallback_ready: bool,
    handler_ready: bool,
    explicit_nesting_required: bool,
    service_online: bool,
}

impl BreakpointExceptionType {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fallback_ready: false,
            handler_ready: false,
            explicit_nesting_required: false,
            service_online: false,
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
        install_breakpoint_policy();
        self.handler_ready = true;
        self.explicit_nesting_required = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self, exception_state: State) -> EventResult {
        if exception_state != State::Ready
            || self.lifecycle.state() != State::Ready
            || !self.handler_ready
            || !self.explicit_nesting_required
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.service_online = true;
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

    pub const fn explicit_nesting_required(&self) -> bool {
        self.explicit_nesting_required
    }

    pub const fn service_online(&self) -> bool {
        self.service_online
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
        self.explicit_nesting_required = true;
        self.service_online = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}
