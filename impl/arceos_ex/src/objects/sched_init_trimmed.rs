use super::{
    rcu::RcuCore,
    scheduler::Scheduler,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct SchedInitTrimmedPaths {
    lifecycle: Lifecycle,
    poking_init_trimmed_noop: bool,
    ftrace_init_trimmed_noop: bool,
    ftrace_trimmed_because_mcount_record_disabled: bool,
    context_tracking_init_trimmed_noop: bool,
    context_tracking_trimmed_because_user_force_disabled: bool,
    position_preserved: bool,
}

impl SchedInitTrimmedPaths {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            poking_init_trimmed_noop: false,
            ftrace_init_trimmed_noop: false,
            ftrace_trimmed_because_mcount_record_disabled: false,
            context_tracking_init_trimmed_noop: false,
            context_tracking_trimmed_because_user_force_disabled: false,
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

    pub const fn context_tracking_init_trimmed_noop(&self) -> bool {
        self.context_tracking_init_trimmed_noop
    }

    pub const fn context_tracking_trimmed_because_user_force_disabled(&self) -> bool {
        self.context_tracking_trimmed_because_user_force_disabled
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
    }

    pub fn setup(&mut self, scheduler: &Scheduler, rcu_core: &RcuCore) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || rcu_core.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.poking_init_trimmed_noop = true;
        self.ftrace_init_trimmed_noop = true;
        self.ftrace_trimmed_because_mcount_record_disabled = true;
        self.context_tracking_init_trimmed_noop = true;
        self.context_tracking_trimmed_because_user_force_disabled = true;
        self.position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SchedInitTrimmedPathsReady,
        )
    }
}
