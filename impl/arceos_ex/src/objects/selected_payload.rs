use crate::objects::{
    config::{Config, SelectedPayloadKind},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

pub struct SelectedPayloadHandoff {
    lifecycle: Lifecycle,
    kind: SelectedPayloadKind,
    kind_bound: bool,
    variant_setup_ready: bool,
    variant_prepare_ready: bool,
    no_return_entry_bound: bool,
}

impl SelectedPayloadHandoff {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kind: SelectedPayloadKind::Hello,
            kind_bound: false,
            variant_setup_ready: false,
            variant_prepare_ready: false,
            no_return_entry_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kind(&self) -> SelectedPayloadKind {
        self.kind
    }

    pub const fn kind_bound(&self) -> bool {
        self.kind_bound
    }

    pub const fn variant_setup_ready(&self) -> bool {
        self.variant_setup_ready
    }

    pub const fn variant_prepare_ready(&self) -> bool {
        self.variant_prepare_ready
    }

    pub const fn no_return_entry_bound(&self) -> bool {
        self.no_return_entry_bound
    }

    pub fn setup(&mut self, config: &Config, variant_setup_ready: bool) -> EventResult {
        if self.lifecycle.state() != State::Base
            || config.state() != State::Online
            || !variant_setup_ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.kind = config.selected_payload_kind();
        self.kind_bound = true;
        self.variant_setup_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn enable(&mut self, config: &Config, variant_prepare_ready: bool) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || config.state() != State::Online
            || !self.kind_bound
            || self.kind != config.selected_payload_kind()
            || !self.variant_setup_ready
            || !variant_prepare_ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.variant_prepare_ready = true;
        self.no_return_entry_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}
