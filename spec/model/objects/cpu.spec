/* Unified CPU instance and CpuRef semantics. */

type LogicId {
}

type CPU: CPUObject {
    initial_state: State::Base;

    attrs {
        hartid: HartId;
        active_translation_owner: TranslationOwnerKind;
    }

    owned {
        trap: TrapType;
    }

    processes {
        Action::PrepareSecondaryTrapEntry {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                self.trap.state == State::Base;
            }
            drives {
                self.trap.interrupt.Transition::Preset;
                self.trap.interrupt.Transition::Setup;
                self.trap.exception.Transition::Preset;
                self.trap.Transition::Preset;
                self.trap.Transition::Setup;
                self.trap.interrupt.Transition::Enable;
                self.trap.exception.Transition::Setup;
                self.trap.exception.page_fault.Transition::Enable;
                self.trap.exception.breakpoint.Transition::Enable;
                self.trap.exception.unexpected.Transition::Enable;
            }
            ensures {
                self.trap.state == State::Ready;
                self.trap.interrupt.state == State::Online;
                self.trap.exception.state == State::Ready;
                self.trap.exception.page_fault.state == State::Online;
                self.trap.exception.syscall.state == State::Prepared;
                self.trap.exception.breakpoint.state == State::Online;
                self.trap.exception.unexpected.state == State::Online;
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Preset(hartid: HartId) -> State::Prepared {
                ensures {
                    cpu_hartid_ready(self, hartid);
                    cpu_logical_id_derived_from_owned_index(self);
                    cpu_ref_for_owned_index_ready(self);
                    cpu_owns_trap_resource(self, self.trap);
                    cpu_active_translation_owner_is(self, TranslationOwnerKind::None);
                    cpu_translation_owner_is_optional_before_kernel_entry(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            cpu_logical_id_derived_from_owned_index(self);
            cpu_ref_for_owned_index_ready(self);
            cpu_owns_trap_resource(self, self.trap);
            cpu_translation_owner_is_optional_before_kernel_entry(self);
        }

        transitions {
            on Transition::Setup(active: bool) -> State::Ready {
                ensures {
                    cpu_possible(self);
                    cpu_present(self);
                    cpu_active_matches_setup(self, active);
                    cpu_translation_owner_is_valid_for_execution_state(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_logical_id_derived_from_owned_index(self);
            cpu_ref_for_owned_index_ready(self);
            cpu_owns_trap_resource(self, self.trap);
            cpu_possible(self);
            cpu_present(self);
            cpu_translation_owner_is_valid_for_execution_state(self);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    cpu_online(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            cpu_logical_id_derived_from_owned_index(self);
            cpu_ref_for_owned_index_ready(self);
            cpu_owns_trap_resource(self, self.trap);
            cpu_possible(self);
            cpu_present(self);
            cpu_online(self);
            cpu_translation_owner_is_valid_for_execution_state(self);
        }
    }
}

predicate cpu_logical_id_derived_from_owned_index<C: CPU>(cpu: C) -> bool;
predicate cpu_ref_for_owned_index_ready<C: CPU>(cpu: C) -> bool;
predicate cpu_active_matches_setup<C: CPU>(cpu: C, active: bool) -> bool;
predicate cpu_ref_dereference_requires_published_element<R: CpuRef>(cpu_ref: R) -> bool;
predicate cpu_hartid_logical_id_bijection<C: CPU>(cpu: C) -> bool;
predicate cpu_owns_trap_resource<C: CPU, T: TrapType>(cpu: C, trap: T) -> bool;
predicate cpu_active_translation_owner_is<C: CPU>(cpu: C, owner: TranslationOwnerKind) -> bool;
predicate cpu_translation_owner_is_optional_before_kernel_entry<C: CPU>(cpu: C) -> bool;
predicate cpu_translation_owner_is_valid_for_execution_state<C: CPU>(cpu: C) -> bool;
predicate cpu_translation_owner_matches_live_satp<C: CPU>(cpu: C) -> bool;
predicate cpu_translation_owner_matches_live_satp_for_ref<R: CpuRef>(cpu_ref: R) -> bool;
predicate cpu_translation_owner_replaced_atomically<C: CPU>(cpu: C) -> bool;
