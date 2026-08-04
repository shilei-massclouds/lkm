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
                    page_fault_flow_user_recovery_can_schedule(self);
                    page_fault_flow_kernel_task_context_fixup_can_schedule(self);
                    page_fault_flow_atomic_fixup_or_fatal(self);
                    page_fault_flow_nested_hardirq_or_irqoff_never_schedules(self);
                    page_fault_flow_missing_or_stale_fixup_is_terminal(self);
                    page_fault_flow_user_request_contains_task_mm_addr_sepc_access(self);
                    page_fault_flow_user_vma_classification_exclusive(self);
                    page_fault_flow_user_cow_write_protect_bound(self);
                    page_fault_flow_user_cow_refcount_checked(self, PageMetadataMap);
                    page_fault_flow_user_cow_commit_atomic(self);
                    page_fault_flow_user_cow_oom_terminal_disjoint(self);
                    page_fault_flow_user_retry_same_instruction(self);
                    page_fault_flow_user_kernel_state_disjoint(self);
                }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    page_fault_flow_root_leaf_revalidated_after_schedule(self);
                    page_fault_flow_completed(self);
                }
            }
        }
    }
    state State::Online { transitions { on Transition::Disable -> State::Offline { ensures { page_fault_flow_disabled(self); } } } }
    state State::Offline { transitions { on Transition::Cleanup -> State::Destroyed { ensures { page_fault_flow_released(self); } } } }
    state State::Destroyed { invariant { page_fault_flow_released(self); } }
}
predicate page_fault_flow_context_classified<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_recovery_can_schedule<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_kernel_task_context_fixup_can_schedule<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_atomic_fixup_or_fatal<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_nested_hardirq_or_irqoff_never_schedules<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_missing_or_stale_fixup_is_terminal<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_request_contains_task_mm_addr_sepc_access<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_vma_classification_exclusive<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_cow_write_protect_bound<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_cow_refcount_checked<F: PageFaultExceptionFlowType, M>(flow: F, metadata_map: M) -> bool;
predicate page_fault_flow_user_cow_commit_atomic<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_cow_oom_terminal_disjoint<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_retry_same_instruction<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_user_kernel_state_disjoint<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_root_leaf_revalidated_after_schedule<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_completed<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_disabled<F: PageFaultExceptionFlowType>(flow: F) -> bool;
predicate page_fault_flow_released<F: PageFaultExceptionFlowType>(flow: F) -> bool;
