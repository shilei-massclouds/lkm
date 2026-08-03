type InterruptFlowRef: TrapChildFlowRef {
}

type InterruptFlowType: FlowObject {
    parent: InterruptType;
    initial_state: State::Base;

    processes {
        Action::Bind(parent_interrupt: InterruptType, child_ref: InterruptFlowRef) {
            state_effect: StateEffect::None;
            structural_binding: true;
            ensures {
                interrupt_flow_parent_is(self, parent_interrupt);
                interrupt_flow_ref_targets(child_ref, self);
                interrupt_flow_ref_generation_valid(child_ref);
                interrupt_flow_occurrence_fresh(self);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    interrupt_flow_entry_saved(self);
                    hardirq_schedule_forbidden(self);
                    hardirq_ordinary_reentry_forbidden(self);
                }
            }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    interrupt_flow_handler_completed(self);
                    interrupt_flow_handler_policy_does_not_schedule(self);
                }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online { ensures { interrupt_flow_completion_committed(self); } }
        }
    }
    state State::Online {
        transitions {
            on Transition::Disable -> State::Offline { ensures { interrupt_flow_disabled(self); } }
        }
    }
    state State::Offline {
        transitions {
            on Transition::Cleanup -> State::Destroyed { ensures { interrupt_flow_occurrence_released(self); } }
        }
    }
    state State::Destroyed { invariant { interrupt_flow_occurrence_released(self); } }
}

predicate interrupt_flow_parent_is<F: InterruptFlowType, I: InterruptType>(flow: F, interrupt: I) -> bool;
predicate interrupt_flow_ref_targets<R: InterruptFlowRef, F: InterruptFlowType>(reference: R, flow: F) -> bool;
predicate interrupt_flow_ref_generation_valid<R: InterruptFlowRef>(reference: R) -> bool;
predicate interrupt_flow_occurrence_fresh<F: InterruptFlowType>(flow: F) -> bool;
predicate interrupt_flow_entry_saved<F: InterruptFlowType>(flow: F) -> bool;
predicate hardirq_schedule_forbidden<F: InterruptFlowType>(flow: F) -> bool;
predicate hardirq_ordinary_reentry_forbidden<F: InterruptFlowType>(flow: F) -> bool;
predicate interrupt_flow_handler_completed<F: InterruptFlowType>(flow: F) -> bool;
predicate interrupt_flow_handler_policy_does_not_schedule<F: InterruptFlowType>(flow: F) -> bool;
predicate interrupt_flow_completion_committed<F: InterruptFlowType>(flow: F) -> bool;
predicate interrupt_flow_disabled<F: InterruptFlowType>(flow: F) -> bool;
predicate interrupt_flow_occurrence_released<F: InterruptFlowType>(flow: F) -> bool;
