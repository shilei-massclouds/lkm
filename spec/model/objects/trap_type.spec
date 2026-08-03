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
            trap_type_is_stable_response_carrier(self);
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
                    trap_type_is_stable_response_carrier(self);
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
            trap_type_is_stable_response_carrier(self);
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
            /* 把所属 CPU 的异常/中断响应入口重置为正式的 TrapFlowType 响应流入口。 */
            on Transition::Setup -> State::Ready {
                depends_on {
                    self.interrupt.state == State::Ready;
                    self.exception.state == State::Base;
                }
                drives {
                    self.exception.Transition::Preset;
                }
                ensures {
                    trap_type_is_stable_response_carrier(self);
                    trap_flow_type_is_common_interrupt_exception_entry(self);
                    trap_common_entry_routes_by_event_class(
                        self,
                        self.interrupt,
                        self.exception
                    );
                    trap_formal_entry_ready(self);
                    trap_response_entry_reset_to_formal_trap_flow(self, self.parent);
                    trap_formal_entry_creates_fresh_trap_flow(self);
                    exception_fallback_covers_all_causes(self.exception);
                    trap_entry_context_ready(self, self.parent);
                    trap_entry_capacity_checked_per_occurrence(self);
                    trap_emergency_fail_stop_ready(self, self.parent);
                    trap_runtime_lease_cpu_local(self, self.parent);
                    trap_runtime_lease_has_narrow_task_access(self);
                    trap_exception_table_access_read_only(self);
                    trap_observation_per_cpu_stable(self, self.parent);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            trap_type_is_stable_response_carrier(self);
            trap_flow_type_is_common_interrupt_exception_entry(self);
            trap_common_entry_routes_by_event_class(
                self,
                self.interrupt,
                self.exception
            );
            trap_formal_entry_ready(self);
            trap_response_entry_reset_to_formal_trap_flow(self, self.parent);
            trap_formal_entry_creates_fresh_trap_flow(self);
            exception_fallback_covers_all_causes(self.exception);
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
            trap_type_is_stable_response_carrier(self);
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

predicate trap_type_is_stable_response_carrier<T: TrapType>(trap: T) -> bool;
predicate trap_flow_type_is_common_interrupt_exception_entry<T: TrapType>(trap: T) -> bool;
predicate trap_common_entry_routes_by_event_class<T: TrapType, I: InterruptType, E: ExceptionType>(trap: T, interrupt: I, exception: E) -> bool;
predicate trap_temporary_protection_entry_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_temporary_protection_handles_unexpected_events<T: TrapType>(trap: T) -> bool;
predicate trap_temporary_protection_supports_testing_and_defect_localization<T: TrapType>(trap: T) -> bool;
predicate trap_children_owned<T: TrapType, I: InterruptType, E: ExceptionType>(trap: T, interrupt: I, exception: E) -> bool;
predicate trap_formal_entry_ready<T: TrapType>(trap: T) -> bool;
predicate trap_response_entry_reset_to_formal_trap_flow<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_formal_entry_creates_fresh_trap_flow<T: TrapType>(trap: T) -> bool;
predicate trap_entry_context_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_entry_capacity_checked_per_occurrence<T: TrapType>(trap: T) -> bool;
predicate trap_emergency_fail_stop_ready<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_runtime_lease_cpu_local<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_runtime_lease_has_narrow_task_access<T: TrapType>(trap: T) -> bool;
predicate trap_exception_table_access_read_only<T: TrapType>(trap: T) -> bool;
predicate trap_observation_per_cpu_stable<T: TrapType, P: CPU>(trap: T, cpu: P) -> bool;
predicate trap_service_online<T: TrapType>(trap: T) -> bool;
