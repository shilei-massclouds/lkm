type SyscallExceptionFlowRef: ExceptionChildFlowRef {
}

type SyscallExceptionFlowType: FlowObject {
    parent: SyscallExceptionType;
    initial_state: State::Base;
    state State::Base { transitions { on Transition::Preset -> State::Prepared { ensures { syscall_flow_entry_saved(self); } } } }
    state State::Prepared { transitions { on Transition::Setup -> State::Ready { ensures { syscall_flow_may_schedule_and_migrate(self); } } } }
    state State::Ready { transitions { on Transition::Enable -> State::Online { ensures { syscall_flow_return_committed(self); } } } }
    state State::Online { transitions { on Transition::Disable -> State::Offline { ensures { syscall_flow_disabled(self); } } } }
    state State::Offline { transitions { on Transition::Cleanup -> State::Destroyed { ensures { syscall_flow_released(self); } } } }
    state State::Destroyed { invariant { syscall_flow_released(self); } }
}
predicate syscall_flow_entry_saved<F: SyscallExceptionFlowType>(flow: F) -> bool;
predicate syscall_flow_may_schedule_and_migrate<F: SyscallExceptionFlowType>(flow: F) -> bool;
predicate syscall_flow_return_committed<F: SyscallExceptionFlowType>(flow: F) -> bool;
predicate syscall_flow_disabled<F: SyscallExceptionFlowType>(flow: F) -> bool;
predicate syscall_flow_released<F: SyscallExceptionFlowType>(flow: F) -> bool;
