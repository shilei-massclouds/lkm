type ExceptionChildFlowRef {
}

type ExceptionFlowRef: TrapChildFlowRef {
}

type ExceptionFlowType: FlowObject {
    parent: ExceptionType;
    initial_state: State::Base;

    associations {
        mutable active_child: ExceptionChildFlowRef;
    }

    processes {
        Action::Bind(parent_exception: ExceptionType, child_ref: ExceptionFlowRef) {
            state_effect: StateEffect::None;
            structural_binding: true;
            ensures {
                exception_flow_parent_is(self, parent_exception);
                exception_flow_ref_targets(child_ref, self);
                exception_flow_ref_generation_valid(child_ref);
                exception_flow_occurrence_fresh(self);
            }
        }

        Action::SelectChild(child_ref: ExceptionChildFlowRef) {
            state_effect: StateEffect::None;
            updates { self.active_child = child_ref; }
            ensures { exception_flow_active_child_is(self, child_ref); }
        }
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared { ensures { exception_flow_entry_validated(self); } }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on { exception_flow_has_one_active_child(self); }
                ensures { exception_flow_child_completed(self); }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online { ensures { exception_flow_completion_committed(self); } }
        }
    }
    state State::Online {
        transitions {
            on Transition::Disable -> State::Offline {
                depends_on { exception_flow_child_destroyed(self); }
                ensures { exception_flow_disabled(self); }
            }
        }
    }
    state State::Offline {
        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on { exception_flow_child_destroyed(self); }
                ensures { exception_flow_occurrence_released(self); }
            }
        }
    }
    state State::Destroyed { invariant { exception_flow_occurrence_released(self); } }
}

predicate exception_flow_parent_is<F: ExceptionFlowType, E: ExceptionType>(flow: F, exception: E) -> bool;
predicate exception_flow_ref_targets<R: ExceptionFlowRef, F: ExceptionFlowType>(reference: R, flow: F) -> bool;
predicate exception_flow_ref_generation_valid<R: ExceptionFlowRef>(reference: R) -> bool;
predicate exception_flow_occurrence_fresh<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_active_child_is<F: ExceptionFlowType, R: ExceptionChildFlowRef>(flow: F, child: R) -> bool;
predicate exception_flow_entry_validated<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_has_one_active_child<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_child_completed<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_completion_committed<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_child_destroyed<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_disabled<F: ExceptionFlowType>(flow: F) -> bool;
predicate exception_flow_occurrence_released<F: ExceptionFlowType>(flow: F) -> bool;
