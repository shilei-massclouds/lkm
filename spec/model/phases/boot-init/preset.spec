/*
 * BootInitFlow Preset Object Specification
 *
 * These objects support BootInitFlow.Preset from kernel entry through the
 * point where EarlyVm is online and BootInitFlow.Prepared can be committed.
 */

/*
 * BootTask 的定义集中在 task.spec；本文件拥有它进入早期地址空间时所需的 binding 协议，
 * 并编排入口前导期中的迁移顺序。
 */

/*
 * BootTaskEntryBinding 是 BootInitFlow.Preset 私有的入口协调对象。它只
 * 绑定同一个静态 BootTask carrier 的物理/虚拟地址并建立入口抢占条件，
 * 不创建第二个 Task、TaskRef、Flow 或调度实体。
 */
object BootTaskEntryBinding: KernelObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    /* Base 表示 init_task 尚未绑定到当前 hart 的 tp。 */
    state State::Base {
        transitions {
            /* Preset 建立物理地址阶段的 tp 与初始抢占关闭条件。 */
            on Transition::Preset(current_task_ref: TaskRef) -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                    BootTask.state == State::OnCpu;
                    task_ref_targets(current_task_ref, BootTask);
                }

                may_change {
                    BootCpuRegisters.tp;
                }

                ensures {
                    BootCpuRegisters.tp == phys_addr(BootTask.storage);
                    valid_task_ref(BootCpuRegisters.tp);
                    task_preemption_control_ready(BootTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootTask);
                    task_preemption_disabled(BootTask);
                    current_task_resolved_target_is(BootInitFlow, current_task_ref, BootTask);
                    current_task_ref_derived_from_selector(current_task_ref, BootTask);
                }
            }
        }
    }

    /* Prepared 表示物理 tp binding 已完整建立。 */
    state State::Prepared {
        invariant {
            BootCpuRegisters.tp == phys_addr(BootTask.storage);
            valid_task_ref(BootCpuRegisters.tp);
            task_preemption_control_ready(BootTask);
            task_preempt_count_initialized_to_init_preempt_count(BootTask);
            task_preemption_disabled(BootTask);
        }

        transitions {
            /* Setup 在 EarlyVm 就绪后切换到同一 carrier 的虚拟地址。 */
            on Transition::Setup -> State::Ready {
                depends_on {
                    BootTask.state == State::OnCpu;
                    Vm.state == State::Ready;
                }

                may_change {
                    BootCpuRegisters.tp;
                }

                ensures {
                    BootCpuRegisters.tp == virt_addr(BootTask.storage, EarlyVm, KernelImageMap);
                    valid_task_ref(BootCpuRegisters.tp);
                    task_preemption_control_ready(BootTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootTask);
                    task_preemption_disabled(BootTask);
                }
            }
        }
    }

    /* Ready 表示虚拟 tp binding 已提交；BootTask 始终保持 Online。 */
    state State::Ready {
        invariant {
            BootCpuRegisters.tp == virt_addr(BootTask.storage, EarlyVm, KernelImageMap);
            valid_task_ref(BootCpuRegisters.tp);
            task_preemption_control_ready(BootTask);
            task_preempt_count_initialized_to_init_preempt_count(BootTask);
            task_preemption_disabled(BootTask);
        }
    }
}

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
                    BootCpuRegisters.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap);
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
            BootCpuRegisters.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap);
            valid_stack_pointer(BootCpuRegisters.sp);
            inside(BootCpuRegisters.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_start, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap));
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
 * KernelImage 表示入口前导期可见的内核映像对象。它跟踪映像边界、BSS 段状态和 gp-relative 寻址状态。
 */
object KernelImage: ImageObject {
    initial_state: State::Base;

    attrs {
        start: Derived<SymbolAddr, Lds.kernel_start>;
        phys_start: PhysAddr<KernelImage>;
        end: Derived<SymbolAddr, Lds.kernel_end>;
        segments: SegmentSet<KernelImageSegment>;
    }

    /*
     * Base 表示内核映像已进入模型空间，但 gp 和 BSS 状态尚未被本阶段处理。
     */
    state State::Base {
        transitions {
            /*
             * Preset 使用 global_pointer 符号建立物理地址阶段的 gp。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                    Riscv64.state == State::Online;
                }

                may_change {
                    BootCpuRegisters.gp;
                }

                ensures {
                    kernel_image_phys_start_observed_at_entry(self, phys_start);
                    phys_start == OpenSBI.kernel_load_pa;
                    BootCpuRegisters.gp == phys_addr(Lds.global_pointer);
                }
            }
        }
    }

    /*
     * Prepared 表示 BSS 边界已可识别，且 gp 指向物理地址阶段的 global_pointer。
     */
    state State::Prepared {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            kernel_image_phys_start_observed_at_entry(self, phys_start);
            phys_start == OpenSBI.kernel_load_pa;
            BootCpuRegisters.gp == phys_addr(Lds.global_pointer);
        }

        transitions {
            /*
             * Setup 清零 BSS 段，使内核映像进入早期可运行状态。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Lds.state == State::Online;
                }

                may_change {
                    memory(segments.bss.range);
                }

                ensures {
                    phys_start == OpenSBI.kernel_load_pa;
                    memory_zeroed(segments.bss.range);
                    fits_in_kernel_image_map(self, KernelImageMap);
                }
            }
        }
    }

    /*
     * Ready 表示 BSS 已清零，但 gp 尚未切换到早期虚拟地址。
     */
    state State::Ready {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == OpenSBI.kernel_load_pa;
            memory_zeroed(segments.bss.range);
            fits_in_kernel_image_map(self, KernelImageMap);
        }

        transitions {
            /*
             * Enable 在 EarlyVm 可用后重置 gp-relative 寻址。
             * 该事件通常由 VM.setup() 内部触发。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    EarlyVm.state == State::Online;
                }

                may_change {
                    BootCpuRegisters.gp;
                }

                ensures {
                    phys_start == OpenSBI.kernel_load_pa;
                    BootCpuRegisters.gp == virt_addr(Lds.global_pointer, EarlyVm, KernelImageMap);
                    gp_relative_access_ready();
                }
            }
        }
    }

    /*
     * Online 表示内核映像在早期虚拟地址空间中可按 gp-relative 方式访问。
     */
    state State::Online {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == OpenSBI.kernel_load_pa;
            BootCpuRegisters.gp == virt_addr(Lds.global_pointer, EarlyVm, KernelImageMap);
            gp_relative_access_ready();
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
                    header_range.start == BootArgs.dtb_pa;
                    header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
                    firmware_dtb_header_accessible(header_range);
                    valid_dtb_magic(header);
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
                }

                may_change {
                    RawDtb.range;
                }

                ensures {
                    valid_dtb_header(header);
                    range.start == BootArgs.dtb_pa;
                    range.end == BootArgs.dtb_pa + header.total_size;
                    firmware_dtb_range_accessible(range);
                    fits_in_fixmap_slot(range, FixMap.fdt_slot, Config.page_size);
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
            range.start == BootArgs.dtb_pa;
            range.end == BootArgs.dtb_pa + header.total_size;
            firmware_dtb_range_accessible(range);
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
object FixMap: PrepareObject {
    initial_state: State::Base;

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
                    fits_in_fixmap_slot(RawDtb.range, fdt_slot, Config.page_size);
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
    initial_state: State::Destroyed;
    parent: Vm;

    /*
     * Destroyed 表示线性映射虚拟区域已按布局预留，但尚未建立完整物理内存映射，也不提供当前地址转换服务。
     */
    state State::Destroyed {
        invariant {
            linear_map_area_reserved(self);
            fixmap_adjacent_to_linear_map(FixMap, LinearMap);
        }
    }
}

/*
 * Vm 表示入口前导期正在建立的内核虚拟内存空间抽象。它编排 TrampolineVm 和 EarlyVm，后续阶段再接入 SwapperVm。
 */
object Vm: AddressSpaceObject {
    initial_state: State::Base;

    /*
     * Base 表示页表子对象尚未完成入口前导期所需的准备。
     */
    state State::Base {
        transitions {
            /*
             * Preset 编排跳板页表和早期页表的建立。
             * Linux/RISC-V setup_vm() 在 MMU 关闭期间调用
             * apply_early_boot_alternatives()，当前模型不展开 alternatives/errata
             * text patch 细节，但必须把该同步边界标成显式 deferred。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    TrampolineVm.state == State::Base;
                    EarlyVm.state == State::Base;
                }

                drives {
                    TrampolineVm.Transition::Setup;
                    EarlyVm.Transition::Preset;
                    EarlyVm.Transition::Setup;
                }

                may_change {
                    TrampolineVm.pg_dir;
                    EarlyVm.pg_dir;
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

    /*
     * Prepared 表示 TrampolineVm 与 EarlyVm 都已具备可启用状态。
     */
    state State::Prepared {
        invariant {
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            riscv_early_boot_alternatives_deferred(Vm);
            riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
        }

        transitions {
            /*
             * Setup 启用跳板页表和早期页表，并完成进入早期虚拟地址阶段的切换。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    TrampolineVm.state == State::Ready;
                    EarlyVm.state == State::Ready;
                }

                drives {
                    TrampolineVm.Transition::Enable;
                    EarlyVm.Transition::Enable;
                    TrampolineVm.Transition::Cleanup;
                    KernelImage.Transition::Enable;
                }

                may_change {
                    BootCpuRegisters.satp;
                    BootCpuRegisters.gp;
                    BootCpuRegisters.stvec;
                }

                ensures {
                    BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
                    vm_transition_stvec_released_to_trap(BootCpuRegisters.stvec, CurrentCPU.trap);
                }
            }
        }
    }

    /*
     * Ready 表示控制流已经切换到 EarlyVm，入口前导期后续对象可使用早期虚拟地址。
     */
    state State::Ready {
        invariant {
            TrampolineVm.state == State::Destroyed;
            EarlyVm.state == State::Online;
            KernelImage.state == State::Online;
            BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
            early_vm_translation_sync_complete(EarlyVm);
            riscv_early_boot_alternatives_deferred(Vm);
            riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
        }

        transitions {
            /*
             * Enable 建立完整内核虚拟内存空间；该事件由入口后继期触发。
             */
            on Transition::Enable -> State::Online {
                drives {
                    SwapperVm.Transition::Setup;
                    SwapperVm.Transition::Enable;
                    EarlyVm.Transition::Cleanup;
                }

                may_change {
                    BootCpuRegisters.satp;
                    SwapperVm.pg_dir;
                }

                ensures {
                    BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
                }
            }
        }
    }

    /*
     * Online 表示完整内核虚拟内存空间已经由 SwapperVm 接管；该状态属于后续阶段。
     */
    state State::Online {
        invariant {
            SwapperVm.state == State::Online;
            EarlyVm.state == State::Destroyed;
            BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
            swapper_vm_translation_sync_complete(SwapperVm);
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

    /*
     * Ready 表示跳板页表已经具备执行第一次地址空间切换的条件。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
        }

        transitions {
            /*
             * Enable 切换到跳板页表，完成从物理地址阶段进入虚拟地址阶段的第一次过渡。
             * Linux/RISC-V 实现中，在写入 trampoline satp 前执行 sfence.vma，
             * 确保 setup_vm() 刚建立的页表项对新的地址转换可见。
             * 同一临界区会临时把 stvec 设置为虚拟 continuation，借用 trap 入口完成
             * 物理 PC 到虚拟 PC 的重定位；这不是正式异常/中断分发。
             * 规格层只保留地址转换同步要求，不把 sfence.vma 展开为独立事件。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    KernelImage.state == State::Ready;
                }

                may_change {
                    BootCpuRegisters.satp;
                    BootCpuRegisters.stvec;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    trampoline_vm_translation_sync_ready_before_satp(TrampolineVm);
                    phys_to_virt_transition_completed(TrampolineVm.pg_dir, TrampolineMap);
                    BootCpuRegisters.stvec == virt_addr(VmSwitchContinuation, TrampolineVm, TrampolineMap);
                    trap_stvec_temporarily_borrowed(CurrentCPU.trap, Vm);
                }
            }
        }
    }

    /*
     * Online 表示第一次物理到虚拟地址过渡已经完成，跳板映射仍处于服务状态。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            trampoline_vm_translation_sync_ready_before_satp(TrampolineVm);
            phys_to_virt_transition_completed(TrampolineVm.pg_dir, TrampolineMap);
            trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
        }

        transitions {
            /*
             * Cleanup 在 EarlyVm 接管后让跳板虚拟内存空间退出服务。
             * Destroyed 不表示 TrampolineVm.pg_dir 这块静态页表存储被释放。
             */
            on Transition::Cleanup -> State::Destroyed {
                depends_on {
                    EarlyVm.state == State::Online;
                }
            }
        }
    }

    /*
     * Destroyed 表示跳板虚拟内存空间退出服务，但其静态页表存储仍作为对象绑定保留。
     */
    state State::Destroyed {
        invariant {
            no_service(TrampolineVm);
        }
    }
}

/*
 * EarlyVm 表示入口前导期后半段使用的早期虚拟内存空间。它映射内核映像区域和 FixMap 中承载 RawDtb 的 FDT 槽位，并保留线性映射区域。
 */
object EarlyVm: AddressSpaceObject {
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
                    fits_in_kernel_image_map(KernelImage, KernelImageMap);
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }

                may_change {
                    EarlyVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    kernel_image_mapping_ready(EarlyVm.pg_dir, KernelImage, KernelImageMap);
                    fixmap_slot_mapping_ready(EarlyVm.pg_dir, FixMap.fdt_slot);
                    kernel_image_mapped_for_plain_data(KernelImage, KernelImageMap);
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
            kernel_image_mapping_ready(EarlyVm.pg_dir, KernelImage, KernelImageMap);
            fixmap_slot_mapping_ready(EarlyVm.pg_dir, FixMap.fdt_slot);
            kernel_image_mapped_for_plain_data(KernelImage, KernelImageMap);
            LinearMap.state == State::Destroyed;
        }

        transitions {
            /*
             * Enable 切换到 early_pg_dir，使早期虚拟地址空间进入服务状态。
             * Linux/RISC-V 实现中，在写入 early/kernel satp 后执行 sfence.vma，
             * 避免继续使用只覆盖首个 superpage 的 trampoline translations。
             * 规格层只保留地址转换同步要求，不把 sfence.vma 展开为独立事件。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    TrampolineVm.state == State::Online;
                }

                may_change {
                    BootCpuRegisters.satp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
                    early_vm_translation_sync_complete(EarlyVm);
                    kernel_image_accessible(KernelImage, KernelImageMap);
                    fixmap_slot_accessible(FixMap.fdt_slot);
                }
            }
        }
    }

    /*
     * Online 表示 EarlyVm 已启用，内核映像和 FDT fixmap 槽位可以通过早期虚拟地址访问。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
            early_vm_translation_sync_complete(EarlyVm);
            kernel_image_accessible(KernelImage, KernelImageMap);
            fixmap_slot_accessible(FixMap.fdt_slot);
        }

        transitions {
            /*
             * Cleanup 在 SwapperVm 接管后让早期虚拟内存空间退出服务。
             * Destroyed 不表示 EarlyVm.pg_dir 这块静态页表存储被释放。
             */
            on Transition::Cleanup -> State::Destroyed {
                depends_on {
                    SwapperVm.state == State::Online;
                }
            }
        }
    }

    /*
     * Destroyed 表示早期虚拟内存空间已被后续完整地址空间接管。
     */
    state State::Destroyed {
        invariant {
            no_service(EarlyVm);
        }
    }
}

/*
 * SwapperVm 表示后续阶段使用的完整内核虚拟内存空间。
 */
object SwapperVm: AddressSpaceObject {
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
                    swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap);
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
            swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap);
            temporary_fixmap_page_table_slots_clean(SwapperVm);
            swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm);
            swapper_vm_final_permissions_not_split_yet(SwapperVm);
        }

        transitions {
            /*
             * Enable 切换到完整内核页表。Linux/RISC-V 的 setup_vm_final()
             * 在写入 swapper_pg_dir 对应 SATP 后执行 local_flush_tlb_all()；
             * 规格层把该边界收敛为完整内核地址空间切换后的翻译同步事实。
             */
            on Transition::Enable -> State::Online {
                may_change {
                    BootCpuRegisters.satp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
                    swapper_vm_current(SwapperVm);
                    swapper_vm_translation_sync_complete(SwapperVm);
                }
            }
        }
    }

    /*
     * Online 表示完整内核虚拟内存空间已经启用。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
            swapper_vm_current(SwapperVm);
            swapper_vm_translation_sync_complete(SwapperVm);
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
