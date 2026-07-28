type SyscallExceptionType: ResourceObject {
    parent: ExceptionType;
    initial_state: State::Base;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared { ensures { syscall_fallback_ready(self); } }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready { ensures { syscall_handler_ready(self); } }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online { ensures { syscall_service_online(self); } }
        }
    }
    state State::Online { invariant { syscall_service_online(self); } }
}
predicate syscall_fallback_ready<S: SyscallExceptionType>(syscall: S) -> bool;
predicate syscall_handler_ready<S: SyscallExceptionType>(syscall: S) -> bool;
predicate syscall_service_online<S: SyscallExceptionType>(syscall: S) -> bool;
