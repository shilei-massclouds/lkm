type UnexpectedExceptionFlowRef: ExceptionChildFlowRef {
}

type UnexpectedExceptionFlowType: FlowObject {
    parent: UnexpectedExceptionType;
    initial_state: State::Base;
    state State::Base { transitions { on Transition::Preset -> State::Prepared { ensures { unexpected_flow_diagnostic_identity_saved(self); } } } }
    state State::Prepared { transitions { on Transition::Setup -> State::Ready { ensures { unexpected_flow_fail_shutdown_selected(self); } } } }
    state State::Ready { transitions { on Transition::Enable -> State::Online { ensures { unexpected_flow_fail_shutdown_committed(self); } } } }
    state State::Online { transitions { on Transition::Disable -> State::Offline { ensures { unexpected_flow_disabled(self); } } } }
    state State::Offline { transitions { on Transition::Cleanup -> State::Destroyed { ensures { unexpected_flow_released(self); } } } }
    state State::Destroyed { invariant { unexpected_flow_released(self); } }
}
predicate unexpected_flow_diagnostic_identity_saved<F: UnexpectedExceptionFlowType>(flow: F) -> bool;
predicate unexpected_flow_fail_shutdown_selected<F: UnexpectedExceptionFlowType>(flow: F) -> bool;
predicate unexpected_flow_fail_shutdown_committed<F: UnexpectedExceptionFlowType>(flow: F) -> bool;
predicate unexpected_flow_disabled<F: UnexpectedExceptionFlowType>(flow: F) -> bool;
predicate unexpected_flow_released<F: UnexpectedExceptionFlowType>(flow: F) -> bool;
