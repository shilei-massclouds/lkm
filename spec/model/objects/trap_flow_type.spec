/* Per-entry root occurrence; stored next to the architectural entry context. */

enum TrapCauseClass {
    Interrupt,
    Exception,
}

type TrapReturnToken {
}

type TrapChildFlowRef {
}

type TrapFlowRef {
    processes {
        Action::Bind(flow: TrapFlowType) {
            state_effect: StateEffect::None;
            ensures {
                trap_flow_ref_targets(self, flow);
                trap_flow_ref_generation_valid(self);
            }
        }
    }
}

type OptionalTrapFlowRef {
}

type TrapFlowType: FlowObject {
    parent: TrapType;
    initial_state: State::Base;

    associations {
        mutable active_child: TrapChildFlowRef;
    }

    processes {
        Action::Bind(parent_trap: TrapType, root_ref: TrapFlowRef) {
            state_effect: StateEffect::None;
            structural_binding: true;
            ensures {
                trap_type_is_stable_response_carrier(parent_trap);
                trap_flow_is_dynamic_response_flow_of(self, parent_trap);
                trap_flow_parent_is(self, parent_trap);
                trap_flow_parent_immutable(self);
                trap_flow_occurrence_fresh(self);
                trap_flow_generation_nonzero(self);
                trap_flow_ref_targets(root_ref, self);
                trap_flow_ref_generation_valid(root_ref);
            }
        }

        Action::SelectChild(child_ref: TrapChildFlowRef) {
            state_effect: StateEffect::None;
            depends_on { self.state == State::Prepared; }
            updates { self.active_child = child_ref; }
            ensures { trap_flow_active_child_is(self, child_ref); }
        }
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    trap_flow_occurrence_fresh(self);
                    trap_flow_generation_nonzero(self);
                }
                ensures {
                    trap_flow_entry_validated(self);
                    trap_flow_return_checkpoint_saved(self);
                    trap_flow_context_saved(self);
                    task_flow_paused_beneath_trap(self);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on { trap_flow_has_one_active_child(self); }
                ensures { trap_flow_child_completed(self); }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on { trap_flow_child_completed(self); }
                ensures {
                    trap_flow_handler_completed(self);
                    trap_return_token_created_once(self);
                    trap_formal_occurrence_never_bypassed(self);
                }
            }
        }
    }

    state State::Online {
        transitions {
            on Transition::Disable -> State::Offline {
                depends_on { trap_flow_child_destroyed(self); }
                ensures { trap_flow_disabled(self); }
            }
        }
    }

    state State::Offline {
        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on { trap_flow_child_destroyed(self); }
                ensures {
                    trap_flow_occurrence_released(self);
                    trap_return_allowed_after_cleanup(self);
                    trap_flow_return_continuation_valid(self);
                    task_flow_resumed_after_trap(self);
                }
            }
        }
    }

    state State::Destroyed {
        invariant { trap_flow_occurrence_released(self); }
    }
}

predicate trap_flow_ref_targets<R: TrapFlowRef, F: TrapFlowType>(reference: R, flow: F) -> bool;
predicate trap_flow_ref_generation_valid<R: TrapFlowRef>(reference: R) -> bool;
predicate trap_flow_is_dynamic_response_flow_of<F: TrapFlowType, T: TrapType>(flow: F, trap: T) -> bool;
predicate trap_flow_parent_is<F: TrapFlowType, T: TrapType>(flow: F, trap: T) -> bool;
predicate trap_flow_parent_immutable<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_occurrence_fresh<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_generation_nonzero<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_active_child_is<F: TrapFlowType, R: TrapChildFlowRef>(flow: F, child: R) -> bool;
predicate trap_flow_entry_validated<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_return_checkpoint_saved<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_context_saved<F: TrapFlowType>(flow: F) -> bool;
predicate task_flow_paused_beneath_trap<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_has_one_active_child<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_child_completed<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_handler_completed<F: TrapFlowType>(flow: F) -> bool;
predicate trap_return_token_created_once<F: TrapFlowType>(flow: F) -> bool;
predicate trap_formal_occurrence_never_bypassed<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_child_destroyed<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_disabled<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_occurrence_released<F: TrapFlowType>(flow: F) -> bool;
predicate trap_return_allowed_after_cleanup<F: TrapFlowType>(flow: F) -> bool;
predicate trap_flow_return_continuation_valid<F: TrapFlowType>(flow: F) -> bool;
predicate task_flow_resumed_after_trap<F: TrapFlowType>(flow: F) -> bool;
