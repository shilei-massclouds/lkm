/*
 * Core Prepare Phase Specification
 *
 * This subphase starts after paging_init() has completed and ends at the
 * trap_init() boundary. It prepares core kernel mechanisms that require the
 * full swapper virtual address space but precede the final selected payload.
 */

/*
 * DeviceTree 表示正式运行期设备树对象。它不同于 EarlyDtb：EarlyDtb 只服务早期事实提取，
 * DeviceTree 则建立运行期可遍历和可查询的 OF/DeviceTree 结构。展开后的
 * DeviceTree 存储由 MemBlock 早期分配提供；这里描述的是结果语义，具体的两次遍历
 * 和内存写入策略由 coding 规格约束。
 *
 * DeviceNode 是 DeviceTree 中的抽象节点。规格层描述的是逻辑关系：每个节点有 name、
 * properties、可选 parent 引用和 children 集合；这些关系不要求实现必须使用指针、数组下标、
 * sibling 链表或其它具体内存布局。
 */
object DeviceTree: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示正式 DeviceTree 尚未从 RawDtb 展开。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 unflatten_device_tree() / unflatten_and_copy_device_tree()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    RawDtb.state == State::Ready;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                    MemBlock.state == State::Online;
                }

                ensures {
                    device_tree_unflattened_from_raw_dtb(DeviceTree, RawDtb);
                    device_tree_storage_allocated_from_memblock(DeviceTree, MemBlock);
                    device_tree_storage_accessible_in_swapper_vm(DeviceTree, SwapperVm);
                    device_tree_root_exists(DeviceTree);
                    device_tree_root_unique(DeviceTree);
                    device_tree_root_has_no_parent(DeviceTree);
                    device_tree_nodes_have_name(DeviceTree);
                    device_tree_nodes_have_properties(DeviceTree);
                    device_tree_non_root_nodes_have_unique_parent(DeviceTree);
                    device_tree_parent_children_consistent(DeviceTree);
                    device_tree_root_reaches_all_nodes(DeviceTree);
                    device_tree_acyclic(DeviceTree);
                    device_tree_node_property_names_unique(DeviceTree);
                    device_tree_path_lookup_ready(DeviceTree);
                    device_tree_parent_name_lookup_ready(DeviceTree);
                    device_tree_properties_queryable(DeviceTree);
                    device_tree_property_raw_values_queryable(DeviceTree);
                }
            }
        }
    }

    /*
     * Ready 表示正式 DeviceTree 结构和查询语义已经可供后续对象依赖。
     */
    state State::Ready {
        invariant {
            device_tree_unflattened_from_raw_dtb(DeviceTree, RawDtb);
            device_tree_storage_allocated_from_memblock(DeviceTree, MemBlock);
            device_tree_storage_accessible_in_swapper_vm(DeviceTree, SwapperVm);
            device_tree_root_exists(DeviceTree);
            device_tree_root_unique(DeviceTree);
            device_tree_root_has_no_parent(DeviceTree);
            device_tree_nodes_have_name(DeviceTree);
            device_tree_nodes_have_properties(DeviceTree);
            device_tree_non_root_nodes_have_unique_parent(DeviceTree);
            device_tree_parent_children_consistent(DeviceTree);
            device_tree_root_reaches_all_nodes(DeviceTree);
            device_tree_acyclic(DeviceTree);
            device_tree_node_property_names_unique(DeviceTree);
            device_tree_path_lookup_ready(DeviceTree);
            device_tree_parent_name_lookup_ready(DeviceTree);
            device_tree_properties_queryable(DeviceTree);
            device_tree_property_raw_values_queryable(DeviceTree);
        }
    }
}

/*
 * Zones 表示页分配器建立前的 zone / migration type / free page set 层级。
 * 当前规格固定三类 ZoneKind：DMA32、NORMAL 和 MOVABLE；它们代表不同物理内存
 * 区段，允许某类 zone 在当前平台或策略下为空。ZoneKind::MOVABLE 不等同于
 * MigrationType::MOVABLE：前者是物理内存 zone，后者是每个 Zone 下 buddy/pageblock
 * 层级的迁移类型。
 *
 * 当前 DSL 还没有专门集合类型，因此 Zone、MigrationType 和 FreePageSet 先作为
 * Zones 内部的抽象元素关系由谓词表达。
 */
object Zones: MemoryObject {
    initial_state: State::Base;

    /*
     * Base 表示 zone 层级尚未由 memblock 事实计算出来。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 misc_mem_init() 中的 zone_sizes_init()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                }

                ensures {
                    zones_ready(Zones, MemBlock);
                    zones_zone_level_ready(Zones);
                    zones_have_dma32_normal_movable_kinds(Zones);
                    zones_dma32_normal_movable_ranges_ordered(Zones);
                    zones_empty_zone_ranges_allowed(Zones);
                    zones_zone_ranges_derived_from_memblock(Zones, MemBlock);
                    zones_dma32_covers_32bit_dma_range(Zones, MemBlock);
                    zones_normal_covers_regular_managed_range(Zones, MemBlock);
                    zones_movable_reserved_for_movable_policy(Zones);
                    zones_migration_type_level_ready(Zones);
                    zones_migration_type_layer_distinct_from_zone_kind(Zones);
                    zones_free_page_set_level_ready(Zones);
                    zones_free_page_sets_initially_empty(Zones);
                }
            }
        }
    }

    /*
     * Ready 表示 zone 层级和空闲页集合边界已建立，但完整页分配器尚未进入 Online。
     */
    state State::Ready {
        invariant {
            zones_ready(Zones, MemBlock);
            zones_zone_level_ready(Zones);
            zones_have_dma32_normal_movable_kinds(Zones);
            zones_dma32_normal_movable_ranges_ordered(Zones);
            zones_empty_zone_ranges_allowed(Zones);
            zones_zone_ranges_derived_from_memblock(Zones, MemBlock);
            zones_dma32_covers_32bit_dma_range(Zones, MemBlock);
            zones_normal_covers_regular_managed_range(Zones, MemBlock);
            zones_movable_reserved_for_movable_policy(Zones);
            zones_migration_type_level_ready(Zones);
            zones_migration_type_layer_distinct_from_zone_kind(Zones);
            zones_free_page_set_level_ready(Zones);
            zones_free_page_sets_initially_empty(Zones);
        }
    }
}

/*
 * ResourceTree 表示系统资源树。它把 MemBlock 中的 memory/reserved 区段和内核镜像段
 * 登记为可查询、可嵌套的 Resource 关系。
 */
object ResourceTree: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示系统资源树尚未建立。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 init_resources()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    KernelImage.state == State::Online;
                    Lds.state == State::Online;
                }

                ensures {
                    resource_tree_ready(ResourceTree, MemBlock);
                    system_ram_resources_ready(ResourceTree, MemBlock);
                    reserved_resources_ready(ResourceTree, MemBlock);
                    kernel_image_resources_ready(ResourceTree, KernelImage, Lds);
                    resource_parent_child_ranges_valid(ResourceTree);
                    resource_overlap_policy_valid(ResourceTree);
                }
            }
        }
    }

    /*
     * Ready 表示系统 RAM、保留区和内核镜像段资源已经登记完成。
     */
    state State::Ready {
        invariant {
            resource_tree_ready(ResourceTree, MemBlock);
            system_ram_resources_ready(ResourceTree, MemBlock);
            reserved_resources_ready(ResourceTree, MemBlock);
            kernel_image_resources_ready(ResourceTree, KernelImage, Lds);
            resource_parent_child_ranges_valid(ResourceTree);
            resource_overlap_policy_valid(ResourceTree);
        }
    }
}

/*
 * CacheBlockInfo 表示 RISC-V cache block operation 的平台级块大小事实。
 * 它是独立事实对象；CorePreparePhase 只编排其 Setup，不拥有该对象。
 * 当前最小模型按 Linux 6.12.37 的 riscv_init_cbo_blocksizes() 建模：
 * 从 DeviceTree CPU nodes 收集 CBOM/CBOZ block size，并发布为系统级事实。
 */
object CacheBlockInfo: HardwareObject {
    initial_state: State::Base;

    /*
     * Base 表示 CBOM/CBOZ block size 尚未从平台描述中确认。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 riscv_init_cbo_blocksizes()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    DeviceTree.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    cache_block_info_ready(CacheBlockInfo, DeviceTree, CpuGroup);
                    cache_block_info_source_is_cpu_nodes(CacheBlockInfo, DeviceTree);
                    cbom_block_size_fact_ready(CacheBlockInfo);
                    cboz_block_size_fact_ready(CacheBlockInfo);
                    cache_block_info_values_platform_wide(CacheBlockInfo);
                    cache_block_info_missing_property_allowed(CacheBlockInfo);
                    cache_block_info_mismatch_diagnostic_nonfatal(CacheBlockInfo);
                }
            }
        }
    }

    /*
     * Ready 表示 CBOM/CBOZ block size 已收敛为可供后续机制依赖的系统级事实。
     */
    state State::Ready {
        invariant {
            cache_block_info_ready(CacheBlockInfo, DeviceTree, CpuGroup);
            cache_block_info_source_is_cpu_nodes(CacheBlockInfo, DeviceTree);
            cbom_block_size_fact_ready(CacheBlockInfo);
            cboz_block_size_fact_ready(CacheBlockInfo);
            cache_block_info_values_platform_wide(CacheBlockInfo);
            cache_block_info_missing_property_allowed(CacheBlockInfo);
            cache_block_info_mismatch_diagnostic_nonfatal(CacheBlockInfo);
        }
    }
}

/*
 * RiscvHwCap 表示 RISC-V ISA/hwcap 能力汇总。
 */
object RiscvHwCap: HardwareObject {
    initial_state: State::Base;

    /*
     * Base 表示硬件能力尚未汇总。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 riscv_fill_hwcap()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    DeviceTree.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    CacheBlockInfo.state == State::Ready;
                }

                ensures {
                    riscv_hwcap_ready(RiscvHwCap, DeviceTree, CpuGroup);
                    riscv_isa_facts_ready(RiscvHwCap);
                }
            }
        }
    }

    /*
     * Ready 表示 ISA/hwcap 能力事实已经可供 alternatives、DMA/cache 和用户态能力路径依赖。
     */
    state State::Ready {
        invariant {
            riscv_hwcap_ready(RiscvHwCap, DeviceTree, CpuGroup);
            riscv_isa_facts_ready(RiscvHwCap);
        }
    }
}

/*
 * SavedCommandLine 表示 setup_command_line() 保存下来的稳定命令行副本。
 */
object SavedCommandLine: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelCmdline.state == State::Ready;
                    MemBlock.state == State::Online;
                }

                ensures {
                    saved_command_line_ready(SavedCommandLine, KernelCmdline);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            saved_command_line_ready(SavedCommandLine, KernelCmdline);
        }
    }
}

/*
 * StaticCommandLine 表示 parse_args() 使用的可修改命令行工作副本。
 */
object StaticCommandLine: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelCmdline.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                    MemBlock.state == State::Online;
                }

                ensures {
                    static_command_line_ready(StaticCommandLine, KernelCmdline);
                    static_command_line_preserves_saved_copy(StaticCommandLine, SavedCommandLine);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            static_command_line_ready(StaticCommandLine, KernelCmdline);
            static_command_line_preserves_saved_copy(StaticCommandLine, SavedCommandLine);
        }
    }
}

/*
 * PerCpuStorage 表示 per-cpu 存储的顶层抽象。后续可以继续细化为 first chunk、
 * static/dynamic per-cpu 区和 offset 表。
 */
object PerCpuStorage: MemoryObject {
    initial_state: State::Base;

    /*
     * Base 表示 per-cpu 存储尚未建立。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 setup_per_cpu_areas()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    SwapperVm.state == State::Online;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                }

                ensures {
                    per_cpu_storage_ready(PerCpuStorage, CpuGroup);
                    per_cpu_addressing_ready(PerCpuStorage);
                    per_cpu_static_instances_ready(PerCpuStorage);
                    per_cpu_dynamic_reserve_ready(PerCpuStorage);
                }
            }
        }
    }

    /*
     * Ready 表示 per-cpu 存储和寻址事实已经可供后续 CPU/hotplug/logging 路径使用。
     */
    state State::Ready {
        invariant {
            per_cpu_storage_ready(PerCpuStorage, CpuGroup);
            per_cpu_addressing_ready(PerCpuStorage);
            per_cpu_static_instances_ready(PerCpuStorage);
            per_cpu_dynamic_reserve_ready(PerCpuStorage);
        }
    }
}

/*
 * BootCpuHotplugState 表示 BootCPU 下的 hotplug 状态对象。后续 SecondaryCPU 也应具备同类子对象。
 */
object BootCpuHotplugState: HardwareObject {
    initial_state: State::Base;
    parent: BootCPU;

    /*
     * Base 表示 boot CPU 的 hotplug 状态尚未初始化。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 boot_cpu_hotplug_init()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    BootCPU.state == State::Online;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    cpu_hotplug_state_ready(BootCpuHotplugState, BootCPU);
                    cpu_hotplug_state_current(BootCpuHotplugState, BootCPU, Online);
                    cpu_hotplug_state_target(BootCpuHotplugState, BootCPU, Online);
                    boot_cpu_recorded_booted_once(BootCpuHotplugState, BootCPU);
                }
            }
        }
    }

    /*
     * Ready 表示 boot CPU 的 hotplug 当前态和目标态已经初始化为 online。
     */
    state State::Ready {
        invariant {
            cpu_hotplug_state_ready(BootCpuHotplugState, BootCPU);
            cpu_hotplug_state_current(BootCpuHotplugState, BootCPU, Online);
            cpu_hotplug_state_target(BootCpuHotplugState, BootCPU, Online);
            boot_cpu_recorded_booted_once(BootCpuHotplugState, BootCPU);
        }
    }
}

/*
 * BootParam 表示普通内核启动参数解析。它不同于 EntrySuccessor 中的 EarlyParam。
 */
object BootParam: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示普通启动参数尚未解析。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 parse_args("Booting kernel", ...)。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EarlyParam.state == State::Ready;
                    StaticCommandLine.state == State::Ready;
                }

                ensures {
                    boot_params_dispatched(BootParam, StaticCommandLine);
                    unknown_boot_options_collected(BootParam);
                    payload_param_boundary_ready(BootParam);
                }
            }
        }
    }

    /*
     * Ready 表示普通启动参数已经解析，未知参数和 "--" 边界已经被记录。
     */
    state State::Ready {
        invariant {
            boot_params_dispatched(BootParam, StaticCommandLine);
            unknown_boot_options_collected(BootParam);
            payload_param_boundary_ready(BootParam);
        }
    }
}

/*
 * PayloadParam 表示传递给最终 Payload 的参数。Linux-like 路径中它对应传给第一个用户态
 * init 的参数；Unikernel 路径中它可作为 kernel-mode payload 的参数来源。
 */
object PayloadParam: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示 payload 参数尚未从 BootParam 边界中建立。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 parse_args("Setting init args", after_dashes, ...)。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    BootParam.state == State::Ready;
                }

                ensures {
                    payload_param_ready(PayloadParam, BootParam);
                    payload_param_may_be_empty(PayloadParam);
                }
            }
        }
    }

    /*
     * Ready 表示最终 payload 参数已经建立；没有 "--" 时允许为空。
     */
    state State::Ready {
        invariant {
            payload_param_ready(PayloadParam, BootParam);
            payload_param_may_be_empty(PayloadParam);
        }
    }
}

/*
 * Randomness 表示内核随机性对象。核心准备期只执行 random_init_early() 对应的 preset，
 * 不承诺完整 RNG 已 ready。
 */
object Randomness: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示随机性对象尚未吸收早期熵源。
     */
    state State::Base {
        events {
            /*
             * Preset 对应 random_init_early(command_line)。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                    StaticCommandLine.state == State::Ready;
                }

                ensures {
                    randomness_early_entropy_mixed(Randomness, StaticCommandLine);
                    randomness_not_fully_ready(Randomness);
                }
            }
        }
    }

    /*
     * Prepared 表示早期随机性混入已经完成，完整 Randomness.Setup 留给后续阶段。
     */
    state State::Prepared {
        invariant {
            randomness_early_entropy_mixed(Randomness, StaticCommandLine);
            randomness_not_fully_ready(Randomness);
        }
    }
}

/*
 * ExceptionTable 表示内核异常表准备对象。它为后续异常恢复和故障定位提供排序后的表结构。
 */
object ExceptionTable: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示异常表尚未完成启动期整理。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 sort_main_extable()，整理并排序内核主异常表。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelImage.state == State::Online;
                    Vm.state == State::Online;
                }

                ensures {
                    main_exception_table_sorted(ExceptionTable, KernelImage);
                    exception_table_lookup_ready(ExceptionTable, KernelImage);
                    exception_table_ready(ExceptionTable, KernelImage);
                }
            }
        }
    }

    /*
     * Ready 表示异常表已排序并可供后续异常处理路径查询。
     */
    state State::Ready {
        invariant {
            main_exception_table_sorted(ExceptionTable, KernelImage);
            exception_table_lookup_ready(ExceptionTable, KernelImage);
            exception_table_ready(ExceptionTable, KernelImage);
        }
    }
}

/*
 * CorePreparePhase 表示从 paging_init() 完成后到 trap_init() 完成的核心准备子阶段。
 * 它补齐正式异常分发之前的核心机制准备，并以 ExceptionStream.Setup 作为阶段末尾边界。
 */
object CorePreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    /*
     * Base 表示核心准备期尚未开始，仍处于系统独占上下文。
     */
    state State::Base {
        events {
            /*
             * Setup 按 paging_init() 后到 trap_init() 的核心路径编排对象推进。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EntrySuccessorPhase.state == State::Ready;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                    MemBlock.state == State::Online;
                    EarlyParam.state == State::Ready;
                    KernelCmdline.state == State::Ready;
                    BootCPU.state == State::Online;
                    CpuIdMap.state == State::Prepared;
                    PrintkBuffer.state == State::Prepared;
                    ExceptionStream.state == State::Prepared;
                }

                drives {
                    DeviceTree.Event::Setup;
                    Zones.Event::Setup;
                    ResourceTree.Event::Setup;
                    CpuGroup.Event::Setup;
                    CpuIdMap.Event::Setup;
                    CacheBlockInfo.Event::Setup;
                    RiscvHwCap.Event::Setup;
                    SavedCommandLine.Event::Setup;
                    StaticCommandLine.Event::Setup;
                    PerCpuStorage.Event::Setup;
                    BootCpuHotplugState.Event::Setup;
                    BootParam.Event::Setup;
                    PayloadParam.Event::Setup;
                    Randomness.Event::Preset;
                    PrintkBuffer.Event::Setup;
                    ExceptionTable.Event::Setup;
                    ExceptionStream.Event::Setup;
                    PageFaultException.Event::Setup;
                    BreakpointException.Event::Setup;
                    UnexpectedException.Event::Setup;
                }

                ensures {
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                }

                deferred {
                    "acpi_boot_table_init() 暂缓：当前最小路径以 FDT/SBI 为主，ACPI 引导路径后续单独建模。"
                    "early_memtest() 暂缓：属于可选内存测试路径，不改变当前最小启动语义。"
                    "sparse_init() 暂缓：SPARSEMEM 元数据初始化后续随完整内存模型展开。"
                    "local_flush_tlb_kernel_range(VMEMMAP_START, VMEMMAP_END) 暂缓：SPARSEMEM_VMEMMAP 条件路径。"
                    "arch_reserve_crashkernel() 暂缓：crashkernel 资源保留路径后续展开。"
                    "kasan_init() 暂缓：CONFIG_KASAN 条件路径。"
                    "acpi_init_rintc_map() / acpi_map_cpus_to_nodes() 暂缓：依赖 ACPI CPU 拓扑和 NUMA 模型。"
                    "CBOP block size 暂缓：DeviceTree binding 定义 riscv,cbop-block-size，但 Linux 6.12.37 的 riscv_init_cbo_blocksizes() 当前只发布 CBOM/CBOZ。"
                    "apply_boot_alternatives() 暂缓：启动期 alternatives patch 后续抽象为代码补丁设施。"
                    "init_rt_signal_env() 暂缓：用户态信号环境不属于当前最小核心准备路径。"
                    "riscv_noncoherent_supported() / riscv_set_dma_cache_alignment() 暂缓：DMA/cache policy 后续建模。"
                    "riscv_user_isa_enable() 暂缓：用户态 ISA 暴露路径后续建模。"
                    "jump_label_init() 在 setup_arch() 返回后再次出现；保留该调用点，后续抽象为 StaticKey/静态分支对象。"
                    "static_call_init() 暂缓：静态调用是调用目标代码补丁设施，当前不进入核心模型。"
                    "early_security_init() 暂缓：LSM/security 框架依赖后续任务、凭据和安全对象。"
                    "setup_boot_config() 暂缓：bootconfig/XBC/initrd 派生参数后续作为参数来源展开。"
                    "setup_nr_cpu_ids() 作为实现 checkpoint：只验证 CpuIdMap[0] == BootCPU 和 possible CPU 映射边界，不推进 CpuIdMap 状态。"
                    "smp_prepare_boot_cpu() 暂缓：RISC-V64 当前弱实现为空，不建立额外对象。"
                    "parse_early_param() 第二次调用作为实现 checkpoint：不得重新推进 EarlyParam 或 EarlyCon。"
                    "print_unknown_bootoptions() 作为实现 checkpoint：只输出 BootParam 收集到的未知选项，不改变 BootParam 状态。"
                    "parse_args(\"Setting extra init args\", ...) 暂缓：bootconfig init.* 路径随 BootConfig 展开。"
                    "vfs_caches_init_early() 暂缓：VFS/inode/dentry 缓存不属于当前核心启动模型。"
                }
            }
        }
    }

    /*
     * Ready 表示 trap_init() 边界已完成，正式异常分类和分发框架已建立。
     */
    state State::Ready {
        invariant {
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
            EntrySuccessorPhase.state == State::Ready;
            DeviceTree.state == State::Ready;
            Zones.state == State::Ready;
            ResourceTree.state == State::Ready;
            CpuGroup.state == State::Ready;
            CpuIdMap.state == State::Ready;
            CacheBlockInfo.state == State::Ready;
            RiscvHwCap.state == State::Ready;
            SavedCommandLine.state == State::Ready;
            StaticCommandLine.state == State::Ready;
            PerCpuStorage.state == State::Ready;
            BootCpuHotplugState.state == State::Ready;
            BootParam.state == State::Ready;
            PayloadParam.state == State::Ready;
            Randomness.state == State::Prepared;
            PrintkBuffer.state == State::Ready;
            ExceptionTable.state == State::Ready;
            ExceptionStream.state == State::Ready;
            PageFaultException.state == State::Ready;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Ready;
            UnexpectedException.state == State::Ready;
        }
    }
}
