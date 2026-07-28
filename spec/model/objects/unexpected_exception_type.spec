type UnexpectedExceptionType: ResourceObject {
    parent: ExceptionType;
    initial_state: State::Base;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared { ensures { unexpected_fallback_ready(self); } }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready { ensures { unexpected_handler_ready(self); } }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online { ensures { unexpected_fail_shutdown_online(self); } }
        }
    }
    state State::Online { invariant { unexpected_fail_shutdown_online(self); } }
}
predicate unexpected_fallback_ready<U: UnexpectedExceptionType>(unexpected: U) -> bool;
predicate unexpected_handler_ready<U: UnexpectedExceptionType>(unexpected: U) -> bool;
predicate unexpected_fail_shutdown_online<U: UnexpectedExceptionType>(unexpected: U) -> bool;
