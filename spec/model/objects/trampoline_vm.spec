/* TrampolineVm model specification. */

/*
 * TrampolineVm 表示从物理地址阶段过渡到虚拟地址阶段使用的跳板虚拟内存空间。它只覆盖完成第一次切换所需的最小映射。
 */
object TrampolineVm: AddressSpaceObject {
    initial_state: State::Base;
    parent: Vm;
    source: static::linux_6_12;

    attrs {
        pg_dir: PageTableStorage;
    }

    reference linux_6_12 {
        pg_dir = symbol("trampoline_pg_dir");
    }

    /*
     * Base 表示跳板页表尚未建立。
     */
    state State::Base {
        transitions {
            /*
             * Setup 初始化跳板页表并建立第一次地址空间切换所需映射。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    Lds.state == State::Online;
                    KernelImage.state == State::Ready;
                    valid_trampoline_map(TrampolineMap);
                }

                may_change {
                    TrampolineVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
                }
            }
        }
    }

    /* Ready 是共享 controller 的稳定页表准备状态；每 CPU 激活由 ActivateOnCpu 表达。 */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
        }

        actions {
            on Action::ActivateOnCpu(cpu_ref: CpuRef) {
                depends_on {
                    KernelImage.state == State::Ready
                        || KernelImage.state == State::Online;
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::PhysicalDirect
                    );
                    translation_live_satp_for_ref_is(cpu_ref, 0);
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    trampoline_vm_translation_sync_complete(TrampolineVm, cpu_ref);
                    phys_to_virt_transition_completed(TrampolineVm.pg_dir, TrampolineMap);
                    translation_live_satp_for_ref_is(
                        cpu_ref,
                        satp_of(TrampolineVm.pg_dir, Config.satp_mode)
                    );
                    translation_stvec_borrowed_for_ref(cpu_ref, TrampolineVm);
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::TrampolineVm
                    );
                    translation_activation_kind_is(
                        cpu_ref,
                        TranslationActivationKind::Handoff
                    );
                    translation_handoff_recorded(
                        cpu_ref,
                        TranslationControllerKind::PhysicalDirect,
                        TranslationControllerKind::TrampolineVm,
                        satp_of(TrampolineVm.pg_dir, Config.satp_mode)
                    );
                    translation_activation_fence_complete(cpu_ref);
                    translation_activation_committed_atomically(cpu_ref);
                    cpu_translation_controller_matches_live_satp_for_ref(cpu_ref);
                }
            }
        }
    }
}
