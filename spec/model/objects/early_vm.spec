/* EarlyVm model specification. */

/*
 * EarlyVm 表示入口前导期后半段使用的早期虚拟内存空间。它映射内核映像区域和 FixMap 中承载 RawDtb 的 FDT 槽位，并保留线性映射区域。
 */
object EarlyVm: PrepareObject {
    initial_state: State::Base;
    parent: Vm;
    source: static::linux_6_12;

    attrs {
        pg_dir: PageTableStorage;
    }

    reference linux_6_12 {
        pg_dir = symbol("early_pg_dir");
    }

    /*
     * Base 表示早期虚拟内存空间尚未发现 RawDtb，也尚未准备 FDT fixmap 槽位。
     */
    state State::Base {
        transitions {
            /*
             * Preset 发现并验证原始 dtb，并把 RawDtb 安排到 FDT fixmap 槽位。
             * 这是规格前置证明边界，强于 Linux setup_vm() 的直接实现顺序。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Base;
                    FixMap.state == State::Base;
                }

                drives {
                    RawDtb.Transition::Preset;
                    RawDtb.Transition::Setup;
                    FixMap.Transition::Preset;
                }

                ensures {
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }
            }
        }
    }

    /*
     * Prepared 表示原始 dtb 已验证，且已被安排到 FDT fixmap 槽位。
     */
    state State::Prepared {
        invariant {
            RawDtb.state == State::Ready;
            FixMap.state == State::Ready;
            slot_contains(FixMap.fdt_slot, RawDtb);
        }

        transitions {
            /*
             * Setup 初始化 early_pg_dir，建立内核映像映射和原始 dtb 的 fixmap 映射。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    KernelImage.state == State::Ready;
                    RawDtb.state == State::Ready;
                    FixMap.state == State::Ready;
                    KernelAddrSpace.state == State::Ready;
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }

                may_change {
                    EarlyVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    kernel_image_mapping_ready(EarlyVm.pg_dir, KernelImage, KernelImage.virt_range);
                    fixmap_slot_mapping_ready(EarlyVm.pg_dir, FixMap.fdt_slot);
                    kernel_image_mapped_for_plain_data(KernelImage, KernelImage.virt_range);
                }
            }
        }
    }

    /*
     * Ready 表示 early_pg_dir 已建立内核映像映射和 FDT fixmap 槽位映射，并保留 PAGE_OFFSET 起始的线性映射区域。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            kernel_image_mapping_ready(EarlyVm.pg_dir, KernelImage, KernelImage.virt_range);
            fixmap_slot_mapping_ready(EarlyVm.pg_dir, FixMap.fdt_slot);
            kernel_image_mapped_for_plain_data(KernelImage, KernelImage.virt_range);
            LinearMap.state == State::Ready;
        }

        actions {
            on Action::ActivateOnCpu(cpu_ref: CpuRef) {
                depends_on {
                    TrampolineVm.state == State::Ready;
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::TrampolineVm
                    );
                    translation_live_satp_for_ref_is(
                        cpu_ref,
                        satp_of(TrampolineVm.pg_dir, Config.satp_mode)
                    );
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    translation_live_satp_for_ref_is(
                        cpu_ref,
                        satp_of(EarlyVm.pg_dir, Config.satp_mode)
                    );
                    early_vm_translation_sync_complete(EarlyVm, cpu_ref);
                    kernel_image_accessible(KernelImage, KernelImage.virt_range);
                    fixmap_slot_accessible(FixMap.fdt_slot);
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::EarlyVm
                    );
                    translation_activation_kind_is(
                        cpu_ref,
                        TranslationActivationKind::Handoff
                    );
                    translation_handoff_recorded(
                        cpu_ref,
                        TranslationControllerKind::TrampolineVm,
                        TranslationControllerKind::EarlyVm,
                        satp_of(EarlyVm.pg_dir, Config.satp_mode)
                    );
                    translation_activation_fence_complete(cpu_ref);
                    translation_activation_committed_atomically(cpu_ref);
                    cpu_translation_controller_matches_live_satp_for_ref(cpu_ref);
                    translation_controller_retired_for_cpu(TrampolineVm, cpu_ref);
                }
            }
        }
    }
}
