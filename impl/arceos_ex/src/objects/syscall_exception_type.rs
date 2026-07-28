use super::{
    exception_type::{SyscallTable, install_syscall_disabled_policy, install_syscall_policy},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

/// CPU-local supervisor-call binding.
pub struct SyscallExceptionType {
    lifecycle: Lifecycle,
    fallback_ready: bool,
    handler_ready: bool,
    service_online: bool,
}

impl SyscallExceptionType {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fallback_ready: false,
            handler_ready: false,
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
        install_syscall_disabled_policy();
        self.fallback_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(
        &mut self,
        exception_state: State,
        syscall_table: &mut SyscallTable,
    ) -> EventResult {
        if exception_state != State::Ready
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
        install_syscall_policy();
        self.handler_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)?;
        syscall_table.setup(self.lifecycle.state())
    }

    pub fn enable(&mut self, exception_state: State, syscall_table: &SyscallTable) -> EventResult {
        if exception_state != State::Ready
            || self.lifecycle.state() != State::Ready
            || !self.handler_ready
            || syscall_table.state() != State::Ready
            || !syscall_table.bound_to_exception()
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

    pub const fn service_online(&self) -> bool {
        self.service_online
    }

    pub(crate) fn adopt_secondary_prepared(&mut self) -> EventResult {
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

    #[cfg(app_user_boot)]
    pub(crate) fn enable_secondary(&mut self, syscall_table: &SyscallTable) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.fallback_ready
            || syscall_table.state() != State::Ready
            || !syscall_table.bound_to_exception()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        }
        self.handler_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)?;
        self.service_online = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}
