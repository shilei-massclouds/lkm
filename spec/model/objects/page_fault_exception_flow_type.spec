type PageFaultExceptionFlowRef: ExceptionChildFlowRef {
}

type PageFaultExceptionFlowType: FlowObject {
    parent: PageFaultExceptionType;
    initial_state: State::Base;
    state State::Base {
        transitions { on Transition::Preset -> State::Prepared { ensures { page_fault_flow_context_classified(self); } } }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    page_fault_flow_recoverable_can_schedule(self);
                    page_fault_flow_atomic_fixup_or_fatal(self);
                }
            }
        }
    }
    state State::Ready { transitions { on Transition::Enable -> State::Online { ensures { page_fault_flow_completed(self); } } } }
    state State::Online { transitions { on Transition::Disable -> State::Offline { ensures { page_fault_flow_disabled(self); } } } }
    state State::Offline { transitions { on Transition::Cleanup -> State::Destroyed { ensures { page_fault_flow_released(self); } } } }
    state State::Destroyed { invariant { page_fault_flow_released(self); } }
}
predicate page_fault_flow_context_classified<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_recoverable_can_schedule<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_atomic_fixup_or_fatal<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_completed<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_disabled<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_released<F: PageFaultExceptionFlowType>(flow: F) -> bool;
