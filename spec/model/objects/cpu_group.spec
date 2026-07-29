/* Unique CpuGroup owner and indexed CPU collection. */

type CpuGroupObject {
    owned {
        indexed cpus[key: LogicId]: CPU;
    }

    initial_state: State::Base;

    processes {
        Action::PrepareSecondaryTrapEntry {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                self.cpus[1].state == State::Ready;
            }
            drives {
                self.cpus[1].Action::PrepareSecondaryTrapEntry;
            }
            ensures {
                cpu_group_cpu_ref_at(self, 1, ApCPURef);
                cpu_group_cpu_ref_targets(self, ApCPURef, self.cpus[1]);
                cpu_ref_targets(ApCPURef, self.cpus[1]);
                cpu_ref_ready(ApCPURef);
                cpu_active_translation_controller_absent_for_ref(ApCPURef);
                translation_live_satp_absent_for_ref(ApCPURef);
                translation_initial_activation_entry_satp_for_ref_is(ApCPURef, 0);
                self.cpus[1].trap.state == State::Ready;
                self.cpus[1].trap.interrupt.state == State::Online;
                self.cpus[1].trap.exception.state == State::Ready;
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                drives {
                    declare self.cpus[0] of CPU;
                    self.cpus[0].Transition::Preset;
                }

                ensures {
                    self.cpus[0].state == State::Prepared;
                    cpu_group_owns_indexed_cpu(self, self.cpus[0]);
                    cpu_group_boot_cpu_alias_targets(self, self.cpus[0]);
                    cpu_group_cpu_ref_at(self, 0, BootCPURef);
                    cpu_group_cpu_ref_targets(self, BootCPURef, self.cpus[0]);
                    cpu_ref_targets(BootCPURef, self.cpus[0]);
                    cpu_ref_ready(BootCPURef);
                    cpu_active_translation_controller_absent_for_ref(BootCPURef);
                    translation_live_satp_for_ref_is(BootCPURef, 0);
                    translation_initial_activation_entry_satp_for_ref_is(BootCPURef, 0);
                    cpu_group_uses_logical_id_index(self);
                    cpu_group_boot_cpu_index_zero(self, self.cpus[0]);
                    cpu_group_preset_atomic_publish(self, self.cpus[0]);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            self.cpus[0].state == State::Prepared
                || self.cpus[0].state == State::Ready
                || self.cpus[0].state == State::Online;
            cpu_group_owns_indexed_cpu(self, self.cpus[0]);
            cpu_group_boot_cpu_alias_targets(self, self.cpus[0]);
            cpu_group_cpu_ref_at(self, 0, BootCPURef);
            cpu_group_cpu_ref_targets(self, BootCPURef, self.cpus[0]);
            cpu_ref_targets(BootCPURef, self.cpus[0]);
            cpu_ref_ready(BootCPURef);
            cpu_group_preset_atomic_publish(self, self.cpus[0]);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    self.cpus[0].state == State::Online;
                    DeviceTree.state == State::Ready;
                    SBI.state == State::Ready;
                }

                drives {
                    declare self.cpus[1] of CPU;
                    self.cpus[1].Transition::Preset;
                    self.cpus[1].Action::AssignHartid(1);
                    self.cpus[1].Transition::Setup(false);
                    declare self.cpus[2] of CPU;
                    self.cpus[2].Transition::Preset;
                    self.cpus[2].Action::AssignHartid(2);
                    self.cpus[2].Transition::Setup(false);
                    declare self.cpus[3] of CPU;
                    self.cpus[3].Transition::Preset;
                    self.cpus[3].Action::AssignHartid(3);
                    self.cpus[3].Transition::Setup(false);
                    declare self.cpus[4] of CPU;
                    self.cpus[4].Transition::Preset;
                    self.cpus[4].Action::AssignHartid(4);
                    self.cpus[4].Transition::Setup(false);
                    declare self.cpus[5] of CPU;
                    self.cpus[5].Transition::Preset;
                    self.cpus[5].Action::AssignHartid(5);
                    self.cpus[5].Transition::Setup(false);
                    declare self.cpus[6] of CPU;
                    self.cpus[6].Transition::Preset;
                    self.cpus[6].Action::AssignHartid(6);
                    self.cpus[6].Transition::Setup(false);
                    declare self.cpus[7] of CPU;
                    self.cpus[7].Transition::Preset;
                    self.cpus[7].Action::AssignHartid(7);
                    self.cpus[7].Transition::Setup(false);
                }

                ensures {
                    cpu_group_topology_ready(self, DeviceTree);
                    secondary_cpus_discovered(self, DeviceTree);
                    secondary_cpus_have_unique_hartids(self);
                    secondary_cpus_exclude_boot_cpu(self, self.cpus[0]);
                    secondary_cpus_possible(self);
                    secondary_cpus_present(self);
                    secondary_cpus_not_online(self);
                    cpu_group_secondary_cpu_entries_ready(self);
                    cpu_group_hartid_logical_id_bijection(self);
                    cpu_group_sets_derived_from_owned_cpus(self);
                    cpu_group_possible_cpu_boundary_ready(self);
                    cpu_group_concurrency_closed(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            self.cpus[0].state == State::Online;
            cpu_group_owns_indexed_cpu(self, self.cpus[0]);
            cpu_group_topology_ready(self, DeviceTree);
            cpu_group_hartid_logical_id_bijection(self);
            cpu_group_sets_derived_from_owned_cpus(self);
            cpu_group_possible_cpu_boundary_ready(self);
            cpu_group_concurrency_closed(self);
        }
    }
}

object CpuGroup: CpuGroupObject {
    parent: Kernel;
}

predicate cpu_group_owns_indexed_cpu<G: CpuGroupObject, C: CPU>(group: G, cpu: C) -> bool;
predicate cpu_group_boot_cpu_alias_targets<G: CpuGroupObject, C: CPU>(group: G, cpu: C) -> bool;
predicate cpu_group_preset_atomic_publish<G: CpuGroupObject, C: CPU>(group: G, cpu: C) -> bool;
predicate cpu_group_hartid_logical_id_bijection<G: CpuGroupObject>(group: G) -> bool;
predicate cpu_group_sets_derived_from_owned_cpus<G: CpuGroupObject>(group: G) -> bool;
