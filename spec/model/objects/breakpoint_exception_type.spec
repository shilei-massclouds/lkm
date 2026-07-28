type BreakpointExceptionType: ResourceObject {
    parent: ExceptionType;
    initial_state: State::Base;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared { ensures { breakpoint_fallback_ready(self); } }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    breakpoint_handler_ready(self);
                    breakpoint_nesting_requires_explicit_context(self);
                }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online { ensures { breakpoint_service_online(self); } }
        }
    }
    state State::Online { invariant { breakpoint_service_online(self); } }
}
predicate breakpoint_fallback_ready<B: BreakpointExceptionType>(breakpoint: B) -> bool;
predicate breakpoint_handler_ready<B: BreakpointExceptionType>(breakpoint: B) -> bool;
predicate breakpoint_nesting_requires_explicit_context<B: BreakpointExceptionType>(breakpoint: B) -> bool;
predicate breakpoint_service_online<B: BreakpointExceptionType>(breakpoint: B) -> bool;
