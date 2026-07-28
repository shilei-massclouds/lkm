/* CPU-owned public trap-entry resource. */

type TrapEntryContext {
}

type TrapType: ResourceObject {
    parent: CPU;
    initial_state: State::Base;

    owned {
        interrupt: InterruptType;
        exception: ExceptionType;
    }

    attrs {
        entry_context: TrapEntryContext;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    trap_early_fatal_prelude_ready(self);
                    trap_prelude_creates_no_flow_occurrence(self);
                    trap_children_owned(self, self.interrupt, self.exception);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            trap_early_fatal_prelude_ready(self);
            trap_children_owned(self, self.interrupt, self.exception);
        }
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    self.interrupt.state == State::Ready;
                    self.exception.state == State::Prepared;
                }
                ensures {
                    trap_formal_entry_ready(self);
                    trap_entry_context_ready(self, self.parent);
                    trap_entry_capacity_checked_per_occurrence(self);
                    trap_emergency_fail_stop_ready(self, self.parent);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            trap_formal_entry_ready(self);
            trap_entry_context_ready(self, self.parent);
            trap_entry_capacity_checked_per_occurrence(self);
            trap_emergency_fail_stop_ready(self, self.parent);
        }
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    self.interrupt.state == State::Online;
                    self.exception.state == State::Online;
                }
                ensures {
                    trap_service_online(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            trap_service_online(self);
        }
    }
}

predicate trap_early_fatal_prelude_ready<T: TrapType>(trap: T) -> bool;
predicate trap_prelude_creates_no_flow_occurrence<T: TrapType>(trap: T) -> bool;
predicate trap_children_owned<T: TrapType, I: InterruptType, E: ExceptionType>(trap: T, interrupt: I, exception: E) -> bool;
predicate trap_formal_entry_ready<T: TrapType>(trap: T) -> bool;
predicate trap_entry_context_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_entry_capacity_checked_per_occurrence<T: TrapType>(trap: T) -> bool;
predicate trap_emergency_fail_stop_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_service_online<T: TrapType>(trap: T) -> bool;
