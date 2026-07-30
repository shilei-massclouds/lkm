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
        invariant {
            trap_flow_type_is_common_interrupt_exception_entry(self);
            trap_common_entry_routes_by_event_class(
                self,
                self.interrupt,
                self.exception
            );
        }

        transitions {
            /* 为所属 CPU 建立临时保护入口，用于处理意外事件并支持测试和缺陷定位。 */
            on Transition::Preset -> State::Prepared {
                ensures {
                    trap_flow_type_is_common_interrupt_exception_entry(self);
                    trap_common_entry_routes_by_event_class(
                        self,
                        self.interrupt,
                        self.exception
                    );
                    trap_temporary_protection_entry_ready(self, self.parent);
                    trap_temporary_protection_handles_unexpected_events(self);
                    trap_temporary_protection_supports_testing_and_defect_localization(self);
                    trap_children_owned(self, self.interrupt, self.exception);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            trap_flow_type_is_common_interrupt_exception_entry(self);
            trap_common_entry_routes_by_event_class(
                self,
                self.interrupt,
                self.exception
            );
            trap_temporary_protection_entry_ready(self, self.parent);
            trap_temporary_protection_handles_unexpected_events(self);
            trap_temporary_protection_supports_testing_and_defect_localization(self);
            trap_children_owned(self, self.interrupt, self.exception);
        }
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    self.interrupt.state == State::Ready;
                    self.exception.state == State::Prepared;
                }
                ensures {
                    trap_flow_type_is_common_interrupt_exception_entry(self);
                    trap_common_entry_routes_by_event_class(
                        self,
                        self.interrupt,
                        self.exception
                    );
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
            trap_flow_type_is_common_interrupt_exception_entry(self);
            trap_common_entry_routes_by_event_class(
                self,
                self.interrupt,
                self.exception
            );
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
                    trap_flow_type_is_common_interrupt_exception_entry(self);
                    trap_common_entry_routes_by_event_class(
                        self,
                        self.interrupt,
                        self.exception
                    );
                    trap_service_online(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            trap_flow_type_is_common_interrupt_exception_entry(self);
            trap_common_entry_routes_by_event_class(
                self,
                self.interrupt,
                self.exception
            );
            trap_service_online(self);
        }
    }
}

predicate trap_flow_type_is_common_interrupt_exception_entry<T: TrapType>(trap: T) -> bool;
predicate trap_common_entry_routes_by_event_class<T: TrapType, I: InterruptType, E: ExceptionType>(trap: T, interrupt: I, exception: E) -> bool;
predicate trap_temporary_protection_entry_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_temporary_protection_handles_unexpected_events<T: TrapType>(trap: T) -> bool;
predicate trap_temporary_protection_supports_testing_and_defect_localization<T: TrapType>(trap: T) -> bool;
predicate trap_children_owned<T: TrapType, I: InterruptType, E: ExceptionType>(trap: T, interrupt: I, exception: E) -> bool;
predicate trap_formal_entry_ready<T: TrapType>(trap: T) -> bool;
predicate trap_entry_context_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_entry_capacity_checked_per_occurrence<T: TrapType>(trap: T) -> bool;
predicate trap_emergency_fail_stop_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_service_online<T: TrapType>(trap: T) -> bool;
