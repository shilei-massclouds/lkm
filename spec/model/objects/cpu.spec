/* Unified CPU instance and CpuRef semantics. */

type LogicId {
}

type CPU: CPUObject {
    initial_state: State::Base;

    attrs {
        hartid: HartId;
    }

    associations {
        mutable active_translation_controller: TranslationControllerRef;
    }

    owned {
        trap: TrapType;
    }

    processes {
        Action::AssignHartid(hartid: HartId) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Prepared
                    || self.state == State::Ready;
            }
            ensures {
                cpu_hartid_ready(self, hartid);
            }
        }

        /*
         * 关闭 BootCPU 的浮点运算和向量运算能力。内核态默认禁止使用这些能力，只在明确受控的执行区间内才允许临时打开，随即关闭。用户态根据任务需要和系统策略打开。
         */
        Action::DisableFpuVectorExecution {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Prepared
                    || self.state == State::Ready
                    || self.state == State::Online;
            }
            ensures {
                cpu_fpu_execution_disabled(self);
                cpu_vector_execution_disabled(self);
                cpu_kernel_fpu_vector_default_disabled(self);
                cpu_kernel_fpu_vector_temporary_enable_requires_controlled_scope(self);
                cpu_kernel_fpu_vector_disabled_after_controlled_scope(self);
                cpu_user_fpu_vector_enable_follows_task_need_and_system_policy(self);
            }
        }

        Action::PrepareSecondaryTrapEntry {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                self.trap.state == State::Base;
            }
            drives {
                self.trap.interrupt.Transition::Preset;
                self.trap.interrupt.Transition::Setup;
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
            on Transition::Preset -> State::Prepared {
                ensures {
                    cpu_logical_id_derived_from_owned_index(self);
                    cpu_ref_for_owned_index_ready(self);
                    cpu_owns_trap_resource(self, self.trap);
                    cpu_active_translation_controller_absent(self);
                    cpu_translation_controller_association_is_optional_before_kernel_entry(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            cpu_logical_id_derived_from_owned_index(self);
            cpu_ref_for_owned_index_ready(self);
            cpu_owns_trap_resource(self, self.trap);
            cpu_translation_controller_association_is_optional_before_kernel_entry(self);
        }

        transitions {
            on Transition::Setup(active: bool) -> State::Ready {
                ensures {
                    cpu_possible(self);
                    cpu_present(self);
                    cpu_active_matches_setup(self, active);
                    cpu_translation_controller_is_valid_for_execution_state(self);
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
            cpu_translation_controller_is_valid_for_execution_state(self);
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
            cpu_translation_controller_is_valid_for_execution_state(self);
        }
    }
}

predicate cpu_logical_id_derived_from_owned_index<C: CPU>(cpu: C) -> bool;
predicate cpu_ref_for_owned_index_ready<C: CPU>(cpu: C) -> bool;
predicate cpu_active_matches_setup<C: CPU>(cpu: C, active: bool) -> bool;
predicate cpu_ref_dereference_requires_published_element<R: CpuRef>(cpu_ref: R) -> bool;
predicate cpu_hartid_logical_id_bijection<C: CPU>(cpu: C) -> bool;
predicate cpu_owns_trap_resource<C: CPU, T: TrapType>(cpu: C, trap: T) -> bool;
predicate cpu_active_translation_controller_is<C: CPU>(cpu: C, controller: TranslationControllerKind) -> bool;
predicate cpu_active_translation_controller_absent<C: CPU>(cpu: C) -> bool;
predicate cpu_active_translation_controller_absent_for_ref<R: CpuRef>(cpu_ref: R) -> bool;
predicate cpu_translation_controller_association_is_optional_before_kernel_entry<C: CPU>(cpu: C) -> bool;
predicate cpu_translation_controller_is_valid_for_execution_state<C: CPU>(cpu: C) -> bool;
predicate cpu_translation_controller_matches_live_satp<C: CPU>(cpu: C) -> bool;
predicate cpu_translation_controller_matches_live_satp_for_ref<R: CpuRef>(cpu_ref: R) -> bool;
predicate cpu_translation_controller_association_replaced_atomically<C: CPU>(cpu: C) -> bool;
predicate cpu_fpu_execution_disabled<C: CPU>(cpu: C) -> bool;
predicate cpu_vector_execution_disabled<C: CPU>(cpu: C) -> bool;
predicate cpu_kernel_fpu_vector_default_disabled<C: CPU>(cpu: C) -> bool;
predicate cpu_kernel_fpu_vector_temporary_enable_requires_controlled_scope<C: CPU>(cpu: C) -> bool;
predicate cpu_kernel_fpu_vector_disabled_after_controlled_scope<C: CPU>(cpu: C) -> bool;
predicate cpu_user_fpu_vector_enable_follows_task_need_and_system_policy<C: CPU>(cpu: C) -> bool;
