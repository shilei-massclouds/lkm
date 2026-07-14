use super::{
    rcu::RcuCore,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct SchedInitPreludeTrimmedPaths {
    lifecycle: Lifecycle,
    poking_init_trimmed_noop: bool,
    ftrace_init_trimmed_noop: bool,
    ftrace_trimmed_because_mcount_record_disabled: bool,
    early_trace_init_deferred: bool,
    position_preserved: bool,
}

impl SchedInitPreludeTrimmedPaths {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            poking_init_trimmed_noop: false,
            ftrace_init_trimmed_noop: false,
            ftrace_trimmed_because_mcount_record_disabled: false,
            early_trace_init_deferred: false,
            position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn poking_init_trimmed_noop(&self) -> bool {
        self.poking_init_trimmed_noop
    }

    pub const fn ftrace_init_trimmed_noop(&self) -> bool {
        self.ftrace_init_trimmed_noop
    }

    pub const fn ftrace_trimmed_because_mcount_record_disabled(&self) -> bool {
        self.ftrace_trimmed_because_mcount_record_disabled
    }

    pub const fn early_trace_init_deferred(&self) -> bool {
        self.early_trace_init_deferred
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
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

        self.poking_init_trimmed_noop = true;
        crate::checkpoint::checkpoint(Checkpoint::PokingInitNoop);
        self.ftrace_init_trimmed_noop = true;
        self.ftrace_trimmed_because_mcount_record_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::FtraceInitTrimmedNoop);
        self.early_trace_init_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::EarlyTraceInitDeferred);
        self.position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SchedInitPreludeTrimmedPathsReady,
        )
    }
}

pub struct SchedInitTraceContextBoundaries {
    lifecycle: Lifecycle,
    trace_init_deferred: bool,
    context_tracking_init_trimmed_noop: bool,
    context_tracking_trimmed_because_user_force_disabled: bool,
    position_preserved: bool,
}

impl SchedInitTraceContextBoundaries {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trace_init_deferred: false,
            context_tracking_init_trimmed_noop: false,
            context_tracking_trimmed_because_user_force_disabled: false,
            position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn trace_init_deferred(&self) -> bool {
        self.trace_init_deferred
    }

    pub const fn context_tracking_init_trimmed_noop(&self) -> bool {
        self.context_tracking_init_trimmed_noop
    }

    pub const fn context_tracking_trimmed_because_user_force_disabled(&self) -> bool {
        self.context_tracking_trimmed_because_user_force_disabled
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
    }

    pub fn setup(
        &mut self,
        prelude: &SchedInitPreludeTrimmedPaths,
        rcu_core: &RcuCore,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || prelude.state() != State::Ready
            || rcu_core.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.trace_init_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::TraceInitDeferred);
        self.context_tracking_init_trimmed_noop = true;
        self.context_tracking_trimmed_because_user_force_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::ContextTrackingInitTrimmedNoop);
        self.position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SchedInitTraceContextBoundariesReady,
        )
    }
}
