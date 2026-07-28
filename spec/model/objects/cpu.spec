/* Unified CPU instance and CpuRef semantics. */

type LogicId {
}

type CPU: CPUObject {
    initial_state: State::Base;

    attrs {
        hartid: HartId;
    }

    state State::Base {
        transitions {
            on Transition::Preset(hartid: HartId) -> State::Prepared {
                ensures {
                    cpu_hartid_ready(self, hartid);
                    cpu_logical_id_derived_from_owned_index(self);
                    cpu_ref_for_owned_index_ready(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            cpu_logical_id_derived_from_owned_index(self);
            cpu_ref_for_owned_index_ready(self);
        }

        transitions {
            on Transition::Setup(active: bool) -> State::Ready {
                ensures {
                    cpu_possible(self);
                    cpu_present(self);
                    cpu_active_matches_setup(self, active);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_logical_id_derived_from_owned_index(self);
            cpu_ref_for_owned_index_ready(self);
            cpu_possible(self);
            cpu_present(self);
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
            cpu_possible(self);
            cpu_present(self);
            cpu_online(self);
        }
    }
}

predicate cpu_logical_id_derived_from_owned_index<C: CPU>(cpu: C) -> bool;
predicate cpu_ref_for_owned_index_ready<C: CPU>(cpu: C) -> bool;
predicate cpu_active_matches_setup<C: CPU>(cpu: C, active: bool) -> bool;
predicate cpu_ref_dereference_requires_published_element<R: CpuRef>(cpu_ref: R) -> bool;
predicate cpu_hartid_logical_id_bijection<C: CPU>(cpu: C) -> bool;
