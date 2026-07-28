type BreakpointExceptionFlowRef: ExceptionChildFlowRef {
}

type BreakpointExceptionFlowType: FlowObject {
    parent: BreakpointExceptionType;
    initial_state: State::Base;
    state State::Base { transitions { on Transition::Preset -> State::Prepared { ensures { breakpoint_flow_entry_saved(self); } } } }
    state State::Prepared { transitions { on Transition::Setup -> State::Ready { ensures { breakpoint_flow_hook_completed(self); } } } }
    state State::Ready { transitions { on Transition::Enable -> State::Online { ensures { breakpoint_flow_resume_committed(self); } } } }
    state State::Online { transitions { on Transition::Disable -> State::Offline { ensures { breakpoint_flow_disabled(self); } } } }
    state State::Offline { transitions { on Transition::Cleanup -> State::Destroyed { ensures { breakpoint_flow_released(self); } } } }
    state State::Destroyed { invariant { breakpoint_flow_released(self); } }
}
predicate breakpoint_flow_entry_saved<F: BreakpointExceptionFlowType>(flow: F) -> bool;
predicate breakpoint_flow_hook_completed<F: BreakpointExceptionFlowType>(flow: F) -> bool;
predicate breakpoint_flow_resume_committed<F: BreakpointExceptionFlowType>(flow: F) -> bool;
predicate breakpoint_flow_disabled<F: BreakpointExceptionFlowType>(flow: F) -> bool;
predicate breakpoint_flow_released<F: BreakpointExceptionFlowType>(flow: F) -> bool;
