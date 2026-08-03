type PageFaultExceptionType: ResourceObject {
    parent: ExceptionType;
    initial_state: State::Base;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures { page_fault_fallback_ready(self); }
            }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    page_fault_handler_ready(self);
                    page_fault_context_matrix_ready(self);
                    page_fault_kernel_origin_independent_of_atomic(self);
                    page_fault_exception_table_fixup_required_for_kernel(self);
                    page_fault_user_request_task_mm_bound(self);
                    page_fault_user_and_kernel_recovery_isolated(self);
                }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                ensures { page_fault_recovery_online(self); }
            }
        }
    }
    state State::Online { invariant { page_fault_recovery_online(self); } }
}
predicate page_fault_fallback_ready<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_handler_ready<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_context_matrix_ready<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_kernel_origin_independent_of_atomic<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_exception_table_fixup_required_for_kernel<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_user_request_task_mm_bound<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_user_and_kernel_recovery_isolated<P: PageFaultExceptionType>(page_fault: P) -> bool;
predicate page_fault_recovery_online<P: PageFaultExceptionType>(page_fault: P) -> bool;
