/*
 * BootInitFlow Preset Object Specification
 *
 * These objects support BootInitFlow.Preset from kernel entry through the
 * point where EarlyVm is online and BootInitFlow.Prepared can be committed.
 */

/*
 * BootTask 的定义集中在 task.spec。入口 tp binding 是 BootInitFlow 的
 * 可重复 Action，不创建独立对象、状态或 snapshot identity。
 */
predicate boot_task_entry_bound_for_active_controller<C: CPU, T: Task>(cpu: C, task: T) -> bool;
predicate boot_task_entry_preempt_count_initialized_once<T: Task>(task: T) -> bool;
predicate boot_task_entry_preempt_count_preserved<T: Task>(task: T) -> bool;
predicate boot_task_entry_binding_diagnostic_clear() -> bool;

/*
 * BootInitStack 表示入口前导期根任务使用的静态根栈。它约束 sp 在物理地址阶段和早期虚拟地址阶段的取值。
 */
object BootInitStack: StackObject {
    initial_state: State::Base;
    parent: BootTask;

    attrs {
        range: Derived<AddrRange, range(Lds.init_stack_start, Lds.init_stack_end)>;
    }

    /*
     * Base 表示根栈范围已可由链接布局描述，但 sp 尚未指向该栈。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立物理地址阶段的根栈指针，并预留 pt_regs 区域。
             * 该动作分两步：先让 sp 指向静态根栈高端，再减去 Config.pt_size_on_stack。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                may_change {
                    BootCpuRegisters.sp;
                }

                ensures {
                    BootCpuRegisters.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack);
                }
            }
        }
    }

    /*
     * Prepared 表示 sp 已经指向根栈的物理地址阶段可用位置。
     */
    state State::Prepared {
        invariant {
            Lds.init_stack_end - Lds.init_stack_start >= Config.page_size;
            BootCpuRegisters.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack);
            valid_stack_pointer(BootCpuRegisters.sp);
            inside(BootCpuRegisters.sp, Lds.init_stack_end, Lds.init_stack_start, Lds.init_stack_end);
        }

        transitions {
            /*
             * Setup 在早期虚拟地址空间可用后，将根栈指针切换为虚拟地址。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Vm.state == State::Ready;
                }

                may_change {
                    BootCpuRegisters.sp;
                }

                ensures {
                    BootCpuRegisters.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImage.virt_range);
                    inside(BootCpuRegisters.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImage.virt_range), virt_addr(Lds.init_stack_start, EarlyVm, KernelImage.virt_range), virt_addr(Lds.init_stack_end, EarlyVm, KernelImage.virt_range));
                }
            }
        }
    }

    /*
     * Ready 表示 sp 已经使用 EarlyVm 中的内核映像虚拟区域地址。
     * Online 留给后续栈保护机制建立后的正式服务状态。
     */
    state State::Ready {
        invariant {
            Lds.init_stack_end - Lds.init_stack_start >= Config.page_size;
            BootCpuRegisters.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImage.virt_range);
            valid_stack_pointer(BootCpuRegisters.sp);
            inside(BootCpuRegisters.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImage.virt_range), virt_addr(Lds.init_stack_start, EarlyVm, KernelImage.virt_range), virt_addr(Lds.init_stack_end, EarlyVm, KernelImage.virt_range));
        }

        transitions {
            /*
             * Enable 在 start_kernel() 早期建立根栈边界和溢出保护状态。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                ensures {
                    init_stack_guard_ready(BootInitStack);
                }
            }
        }
    }

    /*
     * Online 表示根栈不仅地址可用，而且已经建立入口后继期要求的栈保护状态。
     */
    state State::Online {
        invariant {
            init_stack_guard_ready(BootInitStack);
        }
    }
}

/*
 * RawDtb 表示启动参数 dtb_pa 指向的原始设备树二进制。
 * 它分层验证原始 dtb 的起始物理地址、头部和完整物理范围。
 * 这是规格前置证明边界：Linux/RISC-V setup_vm() 主要先建立 FDT
 * fixmap 映射，后续 parse_dtb()/early_init_dt_scan() 再验证 header
 * 并扫描内容；本规格在 EarlyVm.Preset 前置收口这些安全前提。
 */
object RawDtb: ResourceObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    attrs {
        header: DtbHeader;
        header_range: PhysAddrRange<DtbHeader>;
        range: PhysAddrRange<Dtb>;
    }

    /*
     * Base 表示只知道启动参数中给出了 dtb 物理地址，尚未验证头部。
     */
    state State::Base {
        transitions {
            /*
             * Preset 读取并验证原始 dtb 头部 magic。
             * 该验证表达规格要求，不表示 Linux setup_vm() 在此处逐项执行。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                    OpenSBI.state == State::Online;
                    firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
                }

                may_change {
                    RawDtb.header;
                    RawDtb.header_range;
                }

                ensures {
                    BootArgs.dtb_pa != 0;
                    dtb_header_range_addition_safe(BootArgs.dtb_pa, size_of::<DtbHeader>());
                    header_range.start == BootArgs.dtb_pa;
                    header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
                    firmware_dtb_header_accessible(header_range);
                    valid_dtb_magic(header);
                    raw_dtb_nodes_unparsed(self);
                }
            }
        }
    }

    /*
     * Prepared 表示原始 dtb 头部 magic 有效，但完整范围尚未确定。
     */
    state State::Prepared {
        invariant {
            header_range.start == BootArgs.dtb_pa;
            header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
            firmware_dtb_header_accessible(header_range);
            valid_dtb_magic(header);
            raw_dtb_nodes_unparsed(self);
        }

        transitions {
            /*
             * Setup 读取 total_size 并确定原始 dtb 的完整物理范围。
             * 该范围证明用于后续 FDT fixmap 容量检查和映射安全性。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    OpenSBI.state == State::Online;
                    firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
                    Config.state == State::Online;
                }

                may_change {
                    RawDtb.range;
                }

                ensures {
                    valid_dtb_header(header);
                    header.total_size >= size_of::<DtbHeader>();
                    dtb_range_addition_safe(BootArgs.dtb_pa, header.total_size);
                    range.start == BootArgs.dtb_pa;
                    range.end == BootArgs.dtb_pa + header.total_size;
                    firmware_dtb_range_accessible_from_handoff_contract(range);
                    fits_in_fixmap_slot(
                        RawDtb.range,
                        Config.fixmap.fdt,
                        Config.page_size
                    );
                    raw_dtb_nodes_unparsed(self);
                }
            }
        }
    }

    /*
     * Ready 表示原始 dtb 的头部和完整物理范围均已验证。
     */
    state State::Ready {
        invariant {
            valid_dtb_header(header);
            header.total_size >= size_of::<DtbHeader>();
            dtb_range_addition_safe(BootArgs.dtb_pa, header.total_size);
            range.start == BootArgs.dtb_pa;
            range.end == BootArgs.dtb_pa + header.total_size;
            firmware_dtb_range_accessible_from_handoff_contract(range);
            fits_in_fixmap_slot(
                RawDtb.range,
                Config.fixmap.fdt,
                Config.page_size
            );
            raw_dtb_nodes_unparsed(self);
        }
    }
}

/*
 * FixMap 表示入口前导期可用的固定虚拟地址槽位集合。
 * 当前只建模 FDT 槽位，并记录 RawDtb 是否已被安排到该槽位。
 * FDT 槽位容量检查是规格侧的显式前置条件；Linux 实现侧对应
 * FIX_FDT/FIX_FDT_SIZE/MAX_FDT_SIZE 布局和 create_fdt_early_page_table()
 * 的固定映射窗口。
 */
object FixMap: AddressSpaceObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        fdt_slot: FixMapSlotRange<Fdt>;
    }

    /*
     * Base 表示 fixmap 槽位布局来自配置，但尚未把 RawDtb 安排到 FDT 槽位。
     */
    state State::Base {
        transitions {
            /*
             * Preset 检查 FDT 槽位存在且能容纳 RawDtb，并把 RawDtb 安排到该槽位。
             * 该检查把 Linux 隐含在 fixmap 布局常量中的容量前提显式化。
             */
            on Transition::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Ready;
                    has_slot(Config.fixmap, FixMapSlot::Fdt);
                    fits_in_fixmap_slot(
                        RawDtb.range,
                        Config.fixmap.fdt,
                        Config.page_size
                    );
                }

                may_change {
                    FixMap.fdt_slot;
                }

                ensures {
                    attrs_accessible(self);
                    fdt_slot == Config.fixmap.fdt;
                    slot_contains(fdt_slot, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示 FDT 槽位已经承载 RawDtb，后续页表映射可直接引用该槽位。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            fdt_slot == Config.fixmap.fdt;
            slot_contains(fdt_slot, RawDtb);
        }
    }
}

/*
 * LinearMap 表示 PAGE_OFFSET 起始的物理内存线性映射虚拟区域。
 * 入口前导期只预留该区域，完整 RAM banks 映射由后续完整页表阶段建立。
 */
object LinearMap: AddressSpaceObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        range: VirtAddrRange<PhysicalMemory>;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                }

                ensures {
                    linear_map_area_reserved(self);
                    linear_map_starts_at_page_offset(self);
                    fixmap_adjacent_to_linear_map(FixMap, self);
                }
            }
        }
    }

    /* Ready 表示区域布局已保留；映射发布是 controller 的独立事实。 */
    state State::Ready {
        invariant {
            linear_map_area_reserved(self);
            linear_map_starts_at_page_offset(self);
            fixmap_adjacent_to_linear_map(FixMap, LinearMap);
        }
    }
}

object UserSpaceReserve: AddressSpaceObject {
    initial_state: State::Base;
    parent: KernelAddrSpace;

    attrs {
        range: VirtAddrRange<User>;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                }

                ensures {
                    canonical_user_address_range_reserved(self);
                    kernel_mappings_exclude_user_reserve(KernelAddrSpace, self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            canonical_user_address_range_reserved(self);
            kernel_mappings_exclude_user_reserve(KernelAddrSpace, self);
        }
    }
}

object KernelAddrSpace: AddressSpaceObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    KernelImage.state == State::Ready;
                    LinearMap.state == State::Base;
                    UserSpaceReserve.state == State::Base;
                }

                drives {
                    LinearMap.Transition::Preset;
                    UserSpaceReserve.Transition::Preset;
                }

                ensures {
                    kernel_addr_space_region_parentage_ready(self);
                    kernel_addr_space_layout_declared(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            KernelImage.state == State::Ready;
            LinearMap.state == State::Ready;
            UserSpaceReserve.state == State::Ready;
            kernel_addr_space_region_parentage_ready(self);
            kernel_addr_space_layout_declared(self);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    FixMap.state == State::Ready;
                }

                ensures {
                    kernel_addr_space_regions_disjoint(
                        KernelImage.virt_range,
                        FixMap,
                        LinearMap,
                        UserSpaceReserve
                    );
                    kernel_mappings_exclude_user_reserve(self, UserSpaceReserve);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            KernelImage.state == State::Ready
                || KernelImage.state == State::Online;
            FixMap.state == State::Ready;
            LinearMap.state == State::Ready;
            UserSpaceReserve.state == State::Ready;
            kernel_addr_space_regions_disjoint(
                KernelImage.virt_range,
                FixMap,
                LinearMap,
                UserSpaceReserve
            );
            kernel_mappings_exclude_user_reserve(self, UserSpaceReserve);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    SwapperVm.state == State::Ready;
                }

                ensures {
                    kernel_addr_space_final_swapper_mappings_published(self, SwapperVm);
                    kernel_addr_space_online_is_not_all_cpus_switched(self);
                    translation_controller_readiness_does_not_imply_cpu_activation(SwapperVm);
                }
            }
        }
    }

    state State::Online {
        invariant {
            SwapperVm.state == State::Ready;
            kernel_addr_space_final_swapper_mappings_published(self, SwapperVm);
            kernel_addr_space_online_is_not_all_cpus_switched(self);
            translation_controller_readiness_does_not_imply_cpu_activation(SwapperVm);
            kernel_mappings_exclude_user_reserve(self, UserSpaceReserve);
        }
    }
}

/* PhysicalDirect 是 satp=0 的共享 translation controller。 */
object PhysicalDirect: PrepareObject {
    initial_state: State::Ready;
    parent: Vm;

    state State::Ready {
        actions {
            on Action::ActivateOnCpu(cpu_ref: CpuRef) {
                depends_on {
                    cpu_ref_dereference_requires_published_element(cpu_ref);
                    cpu_active_translation_controller_absent_for_ref(cpu_ref);
                    translation_initial_activation_entry_satp_for_ref_is(cpu_ref, 0);
                }

                ensures {
                    translation_live_satp_for_ref_is(cpu_ref, 0);
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::PhysicalDirect
                    );
                    translation_activation_kind_is(
                        cpu_ref,
                        TranslationActivationKind::InitialActivation
                    );
                    translation_initial_activation_recorded(
                        cpu_ref,
                        TranslationControllerKind::PhysicalDirect,
                        0
                    );
                    translation_activation_fence_complete(cpu_ref);
                    translation_activation_committed_atomically(cpu_ref);
                    cpu_translation_controller_matches_live_satp_for_ref(cpu_ref);
                }
            }
        }
    }
}

/* Vm 是控制面协调对象，不是地址空间资源。 */
object Vm: KernelObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PhysicalDirect.state == State::Ready;
                    TrampolineVm.state == State::Base;
                    EarlyVm.state == State::Base;
                    KernelAddrSpace.state == State::Prepared;
                }

                drives {
                    TrampolineVm.Transition::Setup;
                    EarlyVm.Transition::Preset;
                    KernelAddrSpace.Transition::Setup;
                    EarlyVm.Transition::Setup;
                }

                ensures {
                    riscv_early_boot_alternatives_deferred(Vm);
                    riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
                }

                deferred entry_vm.001 {
                    category: DeferredCategory::Protocol;
                    summary: "Model and implement early RISC-V alternatives/errata text patching in the MMU-off setup_vm window.";
                    evidence {
                        riscv_early_boot_alternatives_deferred(Vm);
                        riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
                    }
                    close_when: "Alternative selection, MMU-off patch ordering, synchronization and reference tests pass.";
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            PhysicalDirect.state == State::Ready;
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            KernelAddrSpace.state == State::Ready;
            riscv_early_boot_alternatives_deferred(Vm);
            riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    TrampolineVm.Action::ActivateOnCpu(BootCPURef);
                    EarlyVm.Action::ActivateOnCpu(BootCPURef);
                    KernelImage.Transition::Enable;
                }

                ensures {
                    cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::EarlyVm);
                    cpu_active_translation_controller_for_ref_is(BootCPURef, TranslationControllerKind::EarlyVm);
                    BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
                    early_vm_translation_sync_complete(EarlyVm, CpuGroup.cpus[0]);
                    vm_transition_stvec_released_to_trap(BootCpuRegisters.stvec, CurrentCPU.trap);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            PhysicalDirect.state == State::Ready;
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            SwapperVm.state == State::Base;
            KernelAddrSpace.state == State::Ready;
            KernelImage.state == State::Online;
            cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::EarlyVm);
            BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
            early_vm_translation_sync_complete(EarlyVm, CpuGroup.cpus[0]);
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    SwapperVm.Transition::Setup;
                    KernelAddrSpace.Transition::Enable;
                    SwapperVm.Action::ActivateOnCpu(BootCPURef);
                }

                ensures {
                    cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::SwapperVm);
                    cpu_active_translation_controller_for_ref_is(BootCPURef, TranslationControllerKind::SwapperVm);
                    BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
                    swapper_vm_translation_sync_complete(SwapperVm, CpuGroup.cpus[0]);
                }
            }
        }
    }

    state State::Online {
        invariant {
            PhysicalDirect.state == State::Ready;
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            SwapperVm.state == State::Ready;
            KernelAddrSpace.state == State::Online;
            cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::SwapperVm);
            BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
            swapper_vm_translation_sync_complete(SwapperVm, CpuGroup.cpus[0]);
        }
    }
}

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

/*
 * BootCpuRegisters 表示 CpuGroup.cpus[0] 天然存在且可访问的启动相关寄存器子集。
 * Online 只保证这些寄存器属性可访问；a0/a1 由 OpenSBI 交接确定，
 * 其余寄存器由内核入口阶段逐步更新。
 */
object BootCpuRegisters: HardwareObject {
    initial_state: State::Online;
    parent: CpuGroup.cpus[0];
    source: hardware::boot_cpu_registers;

    attrs {
        a0: Gpr<HartId>;
        a1: Gpr<PhysAddr<Dtb>>;
        sp: Gpr<Addr>;
        tp: Gpr<Addr>;
        gp: Gpr<Addr>;
        sstatus: Csr<Sstatus>;
        sie: Csr<Sie>;
        sip: Csr<Sip>;
        stvec: Csr<TrapVector>;
        sscratch: Csr<usize>;
        satp: Csr<Satp>;
    }

    state State::Online {
        invariant {
            attrs_accessible(self);
        }
    }
}

/*
 * Soc 表示片上系统平台对象。入口前导期只建模平台早期预置的边界。
 */
predicate soc_full_early_platform_model_deferred<T>(soc: T) -> bool;

object Soc: HardwareObject {
    initial_state: State::Base;
    parent: Kernel;

    /*
     * Base 表示 SoC 平台早期预置尚未执行。
     */
    state State::Base {
        transitions {
            /*
             * Preset 执行 SoC 平台相关的早期预置。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    CpuGroup.state == State::Prepared;
                }

                may_change {
                    // 待补充：平台相关早期状态点。
                }

                ensures {
                    soc_early_platform_ready();
                    soc_full_early_platform_model_deferred(Soc);
                }
            }
        }
    }

    /*
     * Prepared 表示 SoC 平台早期预置已完成到入口前导期所需边界。
     */
    state State::Prepared {
        invariant {
            CpuGroup.state == State::Prepared;
            soc_early_platform_ready();
            soc_full_early_platform_model_deferred(Soc);
        }

        deferred soc.001 {
            category: DeferredCategory::ModelDetail;
            summary: "Define the complete SoC-specific early platform state points beyond boot CPU organization.";
            evidence { soc_full_early_platform_model_deferred(Soc); }
            close_when: "The supported SoC early platform facts and their implementation checkpoints are explicitly modeled and tested.";
        }
    }
}
