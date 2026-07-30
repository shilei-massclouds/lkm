/* SwapperVm model specification. */

/*
 * SwapperVm 表示后续阶段使用的完整内核虚拟内存空间。
 */
object SwapperVm: PrepareObject {
    initial_state: State::Base;
    parent: Vm;
    source: static::linux_6_12;

    attrs {
        pg_dir: PageTableStorage;
    }

    reference linux_6_12 {
        pg_dir = symbol("swapper_pg_dir");
    }

    /*
     * Base 表示完整内核页表尚未建立。
     */
    state State::Base {
        transitions {
            /*
             * Setup 建立完整内核页表。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    MemBlock.state == State::Ready;
                }

                may_change {
                    SwapperVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap, UserSpaceReserve);
                    swapper_vm_excludes_user_reserve(SwapperVm, UserSpaceReserve);
                    temporary_fixmap_page_table_slots_clean(SwapperVm);
                    swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm);
                    swapper_vm_final_permissions_not_split_yet(SwapperVm);
                }

                deferred swapper_vm.001 {
                    category: DeferredCategory::Feature;
                    summary: "Split final kernel text, rodata and data mappings into their complete RW/RO/NX permission domains.";
                    evidence {
                        swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm);
                        swapper_vm_final_permissions_not_split_yet(SwapperVm);
                    }
                    close_when: "Final mapping permissions, mark_rodata_ro handoff and W^X tests match the reference path.";
                }
            }
        }
    }

    /*
     * Ready 表示完整内核页表已准备好等待启用。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap, UserSpaceReserve);
            swapper_vm_excludes_user_reserve(SwapperVm, UserSpaceReserve);
            temporary_fixmap_page_table_slots_clean(SwapperVm);
            swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm);
            swapper_vm_final_permissions_not_split_yet(SwapperVm);
        }

        actions {
            on Action::ActivateOnCpu(cpu_ref: CpuRef) {
                depends_on {
                    KernelAddrSpace.state == State::Online;
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::EarlyVm
                    ) || cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::TrampolineVm
                    );
                    cpu_translation_controller_matches_live_satp_for_ref(cpu_ref);
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    translation_live_satp_for_ref_is(
                        cpu_ref,
                        satp_of(SwapperVm.pg_dir, Config.satp_mode)
                    );
                    swapper_vm_current_on_cpu(SwapperVm, cpu_ref);
                    swapper_vm_translation_sync_complete(SwapperVm, cpu_ref);
                    cpu_active_translation_controller_for_ref_is(cpu_ref, TranslationControllerKind::SwapperVm);
                    translation_activation_kind_is(
                        cpu_ref,
                        TranslationActivationKind::Handoff
                    );
                    translation_handoff_to_swapper_recorded_from_active_controller(
                        cpu_ref,
                        satp_of(SwapperVm.pg_dir, Config.satp_mode)
                    );
                    translation_handoff_old_controller_recorded_from_active_association(cpu_ref);
                    translation_activation_fence_complete(cpu_ref);
                    translation_activation_committed_atomically(cpu_ref);
                    cpu_translation_controller_matches_live_satp_for_ref(cpu_ref);
                }
            }
        }
    }
}
