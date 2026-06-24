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
                    device_tree_chosen_node_ready(DeviceTree);
                    device_tree_stdout_path_property_ready(DeviceTree);
                    device_tree_stdout_path_options_preserved(DeviceTree);
                    device_tree_stdout_path_resolves_to_node(DeviceTree, DeviceNodeRef::Ns16550aSerial);
                    device_tree_stdout_path_node_id_stable(DeviceTree, DeviceNodeId::Ns16550aSerial);
                    device_tree_stdout_path_selects_console_node(DeviceTree, DeviceNodeRef::Ns16550aSerial);
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
            device_tree_chosen_node_ready(DeviceTree);
            device_tree_stdout_path_property_ready(DeviceTree);
            device_tree_stdout_path_options_preserved(DeviceTree);
            device_tree_stdout_path_resolves_to_node(DeviceTree, DeviceNodeRef::Ns16550aSerial);
            device_tree_stdout_path_node_id_stable(DeviceTree, DeviceNodeId::Ns16550aSerial);
            device_tree_stdout_path_selects_console_node(DeviceTree, DeviceNodeRef::Ns16550aSerial);
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
                    zones_order_class_level_ready(Zones);
                    zones_migration_type_level_ready(Zones);
                    zones_free_page_sets_partitioned_by_order_and_migration_type(Zones);
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
            zones_order_class_level_ready(Zones);
            zones_migration_type_level_ready(Zones);
            zones_free_page_sets_partitioned_by_order_and_migration_type(Zones);
            zones_migration_type_layer_distinct_from_zone_kind(Zones);
            zones_free_page_set_level_ready(Zones);
            zones_free_page_sets_initially_empty(Zones);
        }
    }
}

/*
 * PageMetadataMap 表示 Linux struct page metadata 视图。当前 default_config
 * 选择 CONFIG_FLATMEM=y，因此它对应 misc_mem_init() 中
 * zone_sizes_init() -> free_area_init() -> alloc_node_mem_map()/memmap_init()
 * 建立的 mem_map；SPARSEMEM/VMEMMAP 路径在当前配置下被裁剪。
 * PageRef 指向这里的具体 PageMetadata 项，再通过 PFN/物理地址/线性映射地址
 * 进行转换。
 */
object PageMetadataMap: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                    SwapperVm.state == State::Online;
                }

                ensures {
                    page_metadata_map_ready(PageMetadataMap, Zones);
                    page_metadata_map_covers_managed_pfns(PageMetadataMap, Zones);
                    page_metadata_map_uses_mem_map_or_vmemmap(PageMetadataMap);
                    page_metadata_map_storage_allocated_from_memblock(PageMetadataMap, MemBlock);
                    page_metadata_map_indexed_by_pfn(PageMetadataMap);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_metadata_map_ready(PageMetadataMap, Zones);
            page_metadata_map_covers_managed_pfns(PageMetadataMap, Zones);
            page_metadata_map_uses_mem_map_or_vmemmap(PageMetadataMap);
            page_metadata_map_storage_allocated_from_memblock(PageMetadataMap, MemBlock);
            page_metadata_map_indexed_by_pfn(PageMetadataMap);
        }
    }
}

/*
 * ResourceLock 表示 Linux kernel/resource.c 中的
 * static DEFINE_RWLOCK(resource_lock)。它保护 resource tree 的读写遍历和
 * 插入/删除路径；CorePrepare 中的 ResourceTree.setup() 只使用写侧 guard。
 */
object ResourceLock: RwLock {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                ensures {
                    resource_lock_static_initializer(ResourceLock);
                    rwlock_storage_bound(ResourceLock);
                    rwlock_init_kind_recorded(ResourceLock);
                    rwlock_preset_respects_init_kind(ResourceLock);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            resource_lock_static_initializer(ResourceLock);
            rwlock_storage_bound(ResourceLock);
            rwlock_init_kind_recorded(ResourceLock);
        }

        events {
            on Event::Setup -> State::Ready {
                ensures {
                    resource_lock_ready(ResourceLock);
                    rwlock_ready(ResourceLock);
                    rwlock_unlocked(ResourceLock);
                    rwlock_no_readers(ResourceLock);
                    rwlock_no_writer(ResourceLock);
                    rwlock_write_side_exclusive(ResourceLock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            resource_lock_ready(ResourceLock);
            rwlock_ready(ResourceLock);
            rwlock_unlocked(ResourceLock);
            rwlock_no_readers(ResourceLock);
            rwlock_no_writer(ResourceLock);
        }
    }
}

context ResourceTreeWriteContext: ResourceExclusiveContext {
    /*
     * This context corresponds to Linux write_lock(&resource_lock) /
     * write_unlock(&resource_lock) around insert_resource_conflict() and
     * insert_resource() mutations during init_resources().
     */
    guard {
        lock_ref: ResourceLock;

        entered_by {
            ResourceLock.Event::WriteLock(BootInitTaskRef);
        }

        exited_by {
            ResourceLock.Event::WriteUnlock(BootInitTaskRef);
        }
    }

    obj_refs {
        ResourceTree;
        ResourceLock;
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
                    ResourceLock.state == State::Ready;
                    task_ref_ready(BootInitTaskRef);
                }

                within ResourceTreeWriteContext only-once {
                    ensures {
                        resource_tree_ready(ResourceTree, MemBlock);
                        system_ram_resources_ready(ResourceTree, MemBlock);
                        reserved_resources_ready(ResourceTree, MemBlock);
                        kernel_image_resources_ready(ResourceTree, KernelImage, Lds);
                        resource_tree_write_lock_guard_used(ResourceTree);
                        resource_tree_resource_lock_write_guard_used(ResourceTree, ResourceLock);
                        resource_parent_child_ranges_valid(ResourceTree);
                        resource_overlap_policy_valid(ResourceTree);
                    }
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
            resource_tree_write_lock_guard_used(ResourceTree);
            resource_tree_resource_lock_write_guard_used(ResourceTree, ResourceLock);
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
 * CpuCapabilities 表示 CPU 集合的能力事实视图。它是独立事实对象；
 * CorePreparePhase 只编排其 Setup，不拥有该对象。
 * 当前最小模型对应 Linux 6.12.37 的 riscv_fill_hwcap()：
 * 从 DeviceTree CPU nodes 收集 per-hart ISA facts，结合 CpuGroup 形成
 * all-harts common capability facts，并用 CacheBlockInfo 校验 Zicbom/Zicboz。
 */
object CpuCapabilities: HardwareObject {
    initial_state: State::Base;

    /*
     * Base 表示 CPU 能力事实尚未汇总。
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
                    cpu_capabilities_ready(CpuCapabilities, DeviceTree, CpuGroup);
                    cpu_capabilities_source_is_cpu_nodes(CpuCapabilities, DeviceTree);
                    per_hart_isa_facts_ready(CpuCapabilities, CpuGroup);
                    common_cpu_isa_facts_ready(CpuCapabilities, CpuGroup);
                    elf_hwcap_facts_ready(CpuCapabilities);
                    fpu_capability_facts_ready(CpuCapabilities);
                    vector_capability_facts_ready(CpuCapabilities);
                    zicbom_capability_validated(CpuCapabilities, CacheBlockInfo);
                    zicboz_capability_validated(CpuCapabilities, CacheBlockInfo);
                }
            }
        }
    }

    /*
     * Ready 表示 CPU 能力事实已经可供 alternatives、DMA/cache、上下文管理
     * 和用户态能力路径依赖。FPU/VECTOR 的支持事实属于本对象；入口期禁用
     * FPU/VECTOR 的执行状态属于 CPU execution context，不由本对象推进。
     */
    state State::Ready {
        invariant {
            cpu_capabilities_ready(CpuCapabilities, DeviceTree, CpuGroup);
            cpu_capabilities_source_is_cpu_nodes(CpuCapabilities, DeviceTree);
            per_hart_isa_facts_ready(CpuCapabilities, CpuGroup);
            common_cpu_isa_facts_ready(CpuCapabilities, CpuGroup);
            elf_hwcap_facts_ready(CpuCapabilities);
            fpu_capability_facts_ready(CpuCapabilities);
            vector_capability_facts_ready(CpuCapabilities);
            zicbom_capability_validated(CpuCapabilities, CacheBlockInfo);
            zicboz_capability_validated(CpuCapabilities, CacheBlockInfo);
        }
    }
}

/*
 * DmaCachePolicy 表示 RISC-V non-coherent DMA 与 cache maintenance 策略事实。
 * 它消费 CPU 能力和 cache block size 事实，供后续 mm_core_init() 中的 SWIOTLB
 * 与未对齐 kmalloc DMA bounce 决策使用；它不是 DMA API 本体。
 */
object DmaCachePolicy: HardwareObject {
    initial_state: State::Base;

    /*
     * Base 表示 DMA/cache 策略事实尚未从 CPU 能力中收敛。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 riscv_noncoherent_supported() 与
             * riscv_set_dma_cache_alignment() 的抽象结果。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    CpuCapabilities.state == State::Ready;
                    CacheBlockInfo.state == State::Ready;
                }

                ensures {
                    dma_cache_policy_ready(DmaCachePolicy, CpuCapabilities, CacheBlockInfo);
                    noncoherent_dma_policy_resolved(DmaCachePolicy, CpuCapabilities);
                    dma_cache_alignment_resolved(DmaCachePolicy, CacheBlockInfo);
                    dma_cache_policy_available_for_swiotlb(DmaCachePolicy);
                }
            }
        }
    }

    /*
     * Ready 表示后续 SWIOTLB 和 DMA bounce 选择可以消费稳定策略事实。
     */
    state State::Ready {
        invariant {
            dma_cache_policy_ready(DmaCachePolicy, CpuCapabilities, CacheBlockInfo);
            noncoherent_dma_policy_resolved(DmaCachePolicy, CpuCapabilities);
            dma_cache_alignment_resolved(DmaCachePolicy, CacheBlockInfo);
            dma_cache_policy_available_for_swiotlb(DmaCachePolicy);
        }
    }
}

/*
 * JumpLabelMutex 表示 Linux kernel/jump_label.c 中的静态
 * DEFINE_MUTEX(jump_label_mutex)。它是通用 Mutex 类型的具名实例，保护
 * jump_label table 的 coming/going 和 jump_label_init() 期间的 registry
 * 构造边界。当前步骤只建立该静态 mutex 实例本身；StaticBranch.setup()
 * 对 Lock/Unlock 的正式驱动在下一步接入。
 */
object JumpLabelMutex: Mutex {
    initial_state: State::Base;

    /*
     * Base 表示 jump_label_mutex 的静态定义尚未纳入模型事实。
     */
    state State::Base {
        events {
            /*
             * Preset 对应 static DEFINE_MUTEX(jump_label_mutex) 提供的
             * 静态存储和 __MUTEX_INITIALIZER 初值。
             */
            on Event::Preset -> State::Prepared {
                ensures {
                    jump_label_mutex_static_initializer(JumpLabelMutex);
                    jump_label_mutex_storage_bound(JumpLabelMutex);
                    jump_label_mutex_init_kind_static(JumpLabelMutex);
                    mutex_storage_bound(JumpLabelMutex);
                    mutex_init_kind_recorded(JumpLabelMutex);
                    mutex_preset_respects_init_kind(JumpLabelMutex);
                    mutex_owns_wait_queue(JumpLabelMutex);
                    mutex_wait_lock_internal_deferred(JumpLabelMutex);
                }
            }
        }
    }

    /*
     * Prepared 表示静态 initializer 已确认，但尚未作为 CorePrepare 中的
     * early mutex 实例发布给 StaticBranch.setup() 使用。
     */
    state State::Prepared {
        invariant {
            jump_label_mutex_static_initializer(JumpLabelMutex);
            jump_label_mutex_storage_bound(JumpLabelMutex);
            jump_label_mutex_init_kind_static(JumpLabelMutex);
        }

        events {
            /*
             * Setup 让 jump_label_mutex 进入 unlocked/ready 状态，并建立
             * 其 wait queue 抽象；Linux wait_lock、handoff 和 lockdep 细节
             * 仍保持为通用 Mutex 的 deferred/internal 边界。
             */
            on Event::Setup -> State::Ready {
                ensures {
                    jump_label_mutex_ready(JumpLabelMutex);
                    jump_label_mutex_unlocked(JumpLabelMutex);
                    jump_label_mutex_wait_queue_ready(JumpLabelMutex);
                    jump_label_mutex_wait_lock_internal_deferred(JumpLabelMutex);
                    mutex_initialized(JumpLabelMutex);
                    mutex_ready(JumpLabelMutex);
                    mutex_unlocked(JumpLabelMutex);
                    mutex_wait_queue_ready(JumpLabelMutex);
                    mutex_recursive_locking_forbidden(JumpLabelMutex);
                    mutex_unlock_requires_owner(JumpLabelMutex);
                }
            }
        }
    }

    /*
     * Ready 表示 jump_label_mutex 已可作为 StaticBranch.setup() 的同步实例。
     */
    state State::Ready {
        invariant {
            jump_label_mutex_ready(JumpLabelMutex);
            jump_label_mutex_unlocked(JumpLabelMutex);
            jump_label_mutex_wait_queue_ready(JumpLabelMutex);
            jump_label_mutex_wait_lock_internal_deferred(JumpLabelMutex);
            mutex_ready(JumpLabelMutex);
            mutex_unlocked(JumpLabelMutex);
            mutex_wait_queue_ready(JumpLabelMutex);
        }
    }
}

/*
 * CpuHotplugLock 表示 Linux kernel/cpu.c 中的
 * DEFINE_STATIC_PERCPU_RWSEM(cpu_hotplug_lock)。它是通用
 * PerCpuRwSemaphore 类型的具名静态实例，cpus_read_lock() /
 * cpus_read_unlock() 通过它排斥 CPU hotplug writer。
 */
object CpuHotplugLock: PerCpuRwSemaphore {
    initial_state: State::Base;

    /*
     * Base 表示 cpu_hotplug_lock 静态定义尚未纳入模型事实。
     */
    state State::Base {
        events {
            /*
             * Preset 对应 DEFINE_STATIC_PERCPU_RWSEM(cpu_hotplug_lock) 提供的
             * 静态 struct、per-cpu read_count 和 RcuSync/waiter/block 初值。
             * 该实例只要求 PerCpuStorage.Prepared，表示 boot CPU 早期
             * per-cpu 静态区已经可访问；通用类型不绑定这个依赖。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    PerCpuStorage.state == State::Prepared;
                }

                ensures {
                    cpu_hotplug_lock_static_initializer(CpuHotplugLock);
                    cpu_hotplug_lock_storage_bound(CpuHotplugLock);
                    cpu_hotplug_lock_percpu_read_counter_bound(CpuHotplugLock, PerCpuStorage);
                    percpu_rwsem_storage_bound(CpuHotplugLock);
                    percpu_rwsem_init_kind_recorded(CpuHotplugLock);
                    percpu_rwsem_percpu_read_counter_bound(CpuHotplugLock);
                    percpu_rwsem_block_flag_clear(CpuHotplugLock);
                    percpu_rwsem_writer_wait_ready(CpuHotplugLock);
                    percpu_rwsem_wait_queue_storage_bound(CpuHotplugLock);
                    percpu_rwsem_rcu_sync_bound(CpuHotplugLock);
                }
            }
        }
    }

    /*
     * Prepared 表示静态 initializer 和 boot CPU early per-cpu counter 已确认，
     * 但尚未作为 CorePrepare 中的 CPU hotplug read guard 实例发布。
     */
    state State::Prepared {
        invariant {
            cpu_hotplug_lock_static_initializer(CpuHotplugLock);
            cpu_hotplug_lock_storage_bound(CpuHotplugLock);
            cpu_hotplug_lock_percpu_read_counter_bound(CpuHotplugLock, PerCpuStorage);
        }

        events {
            /*
             * Setup 发布 boot CPU early read-lock 可用状态。完整多 CPU
             * counter scope 由后续 Enable/Online 表达，不是 CorePrepare 前置。
             */
            on Event::Setup -> State::Ready {
                ensures {
                    cpu_hotplug_lock_ready(CpuHotplugLock);
                    cpu_hotplug_lock_boot_cpu_read_available(CpuHotplugLock);
                    percpu_rwsem_ready(CpuHotplugLock);
                    percpu_rwsem_readers_fast(CpuHotplugLock);
                    percpu_rwsem_percpu_read_count_zero(CpuHotplugLock);
                    percpu_rwsem_wait_queue_ready(CpuHotplugLock);
                    percpu_rwsem_writer_inactive(CpuHotplugLock);
                    percpu_rwsem_reader_fast_path_available(CpuHotplugLock);
                    rcu_sync_ready(CpuHotplugLock);
                    rcu_sync_idle(CpuHotplugLock);
                    rcu_sync_reader_fast_path_available(CpuHotplugLock);
                }
            }
        }
    }

    /*
     * Ready 表示 cpu_hotplug_lock 已可作为 StaticBranch.setup() 的
     * cpus_read_lock()/cpus_read_unlock() 同步实例。
     */
    state State::Ready {
        invariant {
            cpu_hotplug_lock_ready(CpuHotplugLock);
            cpu_hotplug_lock_boot_cpu_read_available(CpuHotplugLock);
            percpu_rwsem_ready(CpuHotplugLock);
            percpu_rwsem_readers_fast(CpuHotplugLock);
            percpu_rwsem_percpu_read_count_zero(CpuHotplugLock);
            percpu_rwsem_reader_fast_path_available(CpuHotplugLock);
        }
    }
}

context CpuHotplugReadContext: ResourceExclusiveContext {
    /*
     * This context corresponds to Linux cpus_read_lock() /
     * cpus_read_unlock() around jump_label_init(). In arceos_ex boot code the
     * outer BootPhaseContext may lower this guard to proof-only, but the
     * source model still records the real Linux guard boundary.
     */
    guard {
        lock_ref: CpuHotplugLock;

        entered_by {
            CpuHotplugLock.Event::ReadLock(BootInitTaskRef);
        }

        exited_by {
            CpuHotplugLock.Event::ReadUnlock(BootInitTaskRef);
        }
    }

    obj_refs {
        StaticBranch;
        CpuHotplugLock;
    }
}

context StaticBranchJumpLabelContext: ResourceExclusiveContext {
    /*
     * This context corresponds to Linux jump_label_lock() /
     * jump_label_unlock() around jump_label_init(). The current task is the
     * early boot init_task represented by BootInitTaskRef.
     */
    guard {
        lock_ref: JumpLabelMutex;

        entered_by {
            JumpLabelMutex.Event::Lock(BootInitTaskRef);
        }

        exited_by {
            JumpLabelMutex.Event::Unlock(BootInitTaskRef);
        }
    }

    obj_refs {
        StaticBranch;
    }
}

/*
 * StaticBranch 表示 static key / static branch 的启动期基础设施。
 * 本阶段只建立 registry 与分支项关系，后续 set(key, value) 是状态内 action，
 * 不推进 StaticBranch 生命周期。
 *
 * Linux jump_label_init() 在建立 registry 时持有 cpus_read_lock() 和
 * jump_label_mutex。本阶段只记录这两个启动期保护事实。运行期 static key
 * text patch 需要的 text_mutex、stop_machine 或 icache 同步不由本 Setup
 * 隐式完成，后续在 StaticBranch.set/action 语义中单独展开。
 */
object StaticBranch: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示 static branch registry 尚未建立。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 RISC-V setup_arch() 中的 jump_label_init()。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelImage.state == State::Online;
                    SwapperVm.state == State::Online;
                    CpuHotplugLock.state == State::Ready;
                    JumpLabelMutex.state == State::Ready;
                    task_ref_ready(BootInitTaskRef);
                }

                within CpuHotplugReadContext only-once {
                    within StaticBranchJumpLabelContext only-once {
                        ensures {
                            static_branch_registry_ready(StaticBranch, KernelImage);
                            static_branch_entries_sorted(StaticBranch);
                            static_key_to_branch_sites_ready(StaticBranch);
                            static_branch_cpu_hotplug_read_guard_used(StaticBranch);
                            static_branch_jump_label_mutex_guard_used(StaticBranch);
                            static_branch_text_patch_sync_deferred(StaticBranch);
                            static_branch_set_action_ready(StaticBranch);
                        }
                    }
                }
            }
        }
    }

    /*
     * Ready 表示后续对象可以通过 StaticBranch.set(key, value) action 收敛 static key。
     */
    state State::Ready {
        invariant {
            static_branch_registry_ready(StaticBranch, KernelImage);
            static_branch_entries_sorted(StaticBranch);
            static_key_to_branch_sites_ready(StaticBranch);
            static_branch_cpu_hotplug_read_guard_used(StaticBranch);
            static_branch_jump_label_mutex_guard_used(StaticBranch);
            static_branch_text_patch_sync_deferred(StaticBranch);
            static_branch_set_action_ready(StaticBranch);
        }
    }
}

/*
 * SavedCommandLine 表示 CommandLine 的 saved view，即 setup_command_line()
 * 保存下来的稳定命令行副本。
 */
object SavedCommandLine: ResourceObject {
    initial_state: State::Base;
    parent: CommandLine;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CommandLine.state == State::Prepared;
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
 * StaticCommandLine 表示 CommandLine 的 static view，即 parse_args() 使用的
 * 可修改命令行工作副本。
 */
object StaticCommandLine: ResourceObject {
    initial_state: State::Base;
    parent: CommandLine;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CommandLine.state == State::Prepared;
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
 * PerCpuStaticImage 表示链接脚本中 .data..percpu 的静态模板。
 * __per_cpu_start / __per_cpu_end 定义模板大小和运行地址范围；
 * __per_cpu_load 定义 first chunk 初始化时复制静态内容的加载地址。
 */
object PerCpuStaticImage: MemoryObject {
    initial_state: State::Base;
    parent: PerCpuStorage;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Lds.state == State::Online;
                }

                ensures {
                    per_cpu_static_image_ready(PerCpuStaticImage, Lds);
                    per_cpu_static_image_size_from_lds(PerCpuStaticImage, Lds);
                    per_cpu_static_image_load_source_ready(PerCpuStaticImage, Lds);
                    static_per_cpu_objects_in_static_image(PerCpuStaticImage);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            per_cpu_static_image_ready(PerCpuStaticImage, Lds);
            per_cpu_static_image_size_from_lds(PerCpuStaticImage, Lds);
            per_cpu_static_image_load_source_ready(PerCpuStaticImage, Lds);
            static_per_cpu_objects_in_static_image(PerCpuStaticImage);
        }
    }
}

/*
 * PerCpuFirstChunk 表示 setup_per_cpu_areas() 为 possible CPU 集合建立的
 * first percpu chunk。每个 possible CPU 对应一个 unit；unit 内部布局为
 * static | [reserved] | dynamic | unused tail。reserved 段当前只作为布局事实，
 * 其 allocator 能力后续在模块/保留 percpu 分配路径展开。
 */
object PerCpuFirstChunk: MemoryObject {
    initial_state: State::Base;
    parent: PerCpuStorage;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStaticImage.state == State::Ready;
                    MemBlock.state == State::Online;
                    SwapperVm.state == State::Online;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                }

                ensures {
                    per_cpu_first_chunk_ready(PerCpuFirstChunk, PerCpuStaticImage, CpuGroup);
                    per_cpu_first_chunk_allocated_from_memblock(PerCpuFirstChunk, MemBlock);
                    per_cpu_first_chunk_linearly_mapped(PerCpuFirstChunk, SwapperVm);
                    per_cpu_first_chunk_unit_count_matches_possible_cpus(PerCpuFirstChunk, CpuGroup);
                    per_cpu_first_chunk_units_follow_cpu_id_map(PerCpuFirstChunk, CpuIdMap);
                    per_cpu_unit_static_area_initialized_from_load(PerCpuFirstChunk, PerCpuStaticImage);
                    per_cpu_unit_layout_ready(PerCpuFirstChunk);
                    per_cpu_reserved_area_layout_ready(PerCpuFirstChunk);
                    per_cpu_dynamic_reserve_ready(PerCpuFirstChunk);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            per_cpu_first_chunk_ready(PerCpuFirstChunk, PerCpuStaticImage, CpuGroup);
            per_cpu_first_chunk_allocated_from_memblock(PerCpuFirstChunk, MemBlock);
            per_cpu_first_chunk_linearly_mapped(PerCpuFirstChunk, SwapperVm);
            per_cpu_first_chunk_unit_count_matches_possible_cpus(PerCpuFirstChunk, CpuGroup);
            per_cpu_first_chunk_units_follow_cpu_id_map(PerCpuFirstChunk, CpuIdMap);
            per_cpu_unit_static_area_initialized_from_load(PerCpuFirstChunk, PerCpuStaticImage);
            per_cpu_unit_layout_ready(PerCpuFirstChunk);
            per_cpu_reserved_area_layout_ready(PerCpuFirstChunk);
            per_cpu_dynamic_reserve_ready(PerCpuFirstChunk);
        }
    }
}

/*
 * PerCpuOffsetTable 表示 logical CPU 到 percpu unit/base offset 的映射表。
 * 其边界基于 CpuGroup 中 possible CPU 的个数，而不是 online CPU 的个数。
 * 运行期 per_cpu_ptr(symbol, logical_cpu) 属于 action：它锚定在 Ready 状态，
 * 依赖 Ready 状态提供的 offset table 和 static image 事实；成功访问后仍停留在
 * Ready 状态，不推进任何生命周期状态。
 */
object PerCpuOffsetTable: MemoryObject {
    initial_state: State::Base;
    parent: PerCpuStorage;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuFirstChunk.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                }

                ensures {
                    per_cpu_offset_table_ready(PerCpuOffsetTable, PerCpuFirstChunk, CpuIdMap);
                    per_cpu_offset_entries_match_possible_cpus(PerCpuOffsetTable, CpuGroup);
                    per_cpu_offset_entries_follow_cpu_id_map(PerCpuOffsetTable, CpuIdMap);
                    per_cpu_addressing_ready(PerCpuOffsetTable);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            per_cpu_offset_table_ready(PerCpuOffsetTable, PerCpuFirstChunk, CpuIdMap);
            per_cpu_offset_entries_match_possible_cpus(PerCpuOffsetTable, CpuGroup);
            per_cpu_offset_entries_follow_cpu_id_map(PerCpuOffsetTable, CpuIdMap);
            per_cpu_addressing_ready(PerCpuOffsetTable);
        }
    }
}

/*
 * PerCpuStorage 表示 per-cpu 存储的顶层抽象。它聚合静态 percpu 模板、
 * first chunk 和 offset table。Prepared 表示静态定义和 boot CPU 早期
 * per-cpu 可用性已经建立；Ready 表示完整 possible CPU first chunk 和
 * offset table 可用。完整动态 percpu allocator 后续再展开。
 */
object PerCpuStorage: MemoryObject {
    initial_state: State::Base;

    /*
     * Base 表示 per-cpu 静态模板尚未纳入模型事实。
     */
    state State::Base {
        events {
            /*
             * Preset 对应静态 .data..percpu 模板和 boot CPU 早期访问事实。
             * 这一步足以支撑 DEFINE_STATIC_PERCPU_RWSEM(cpu_hotplug_lock)
             * 的 read_count 存储绑定，但不表示完整 per-cpu allocator 在线。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                }

                drives {
                    PerCpuStaticImage.Event::Setup;
                }

                ensures {
                    PerCpuStaticImage.state == State::Ready;
                    per_cpu_storage_static_template_ready(PerCpuStorage, PerCpuStaticImage);
                    per_cpu_storage_boot_cpu_early_access_ready(PerCpuStorage, PerCpuStaticImage);
                }
            }
        }
    }

    /*
     * Prepared 表示静态 percpu 模板和 boot CPU 早期 per-cpu 静态实例可用，
     * 但 first chunk 和 offset table 尚未完成。
     */
    state State::Prepared {
        invariant {
            PerCpuStaticImage.state == State::Ready;
            per_cpu_storage_static_template_ready(PerCpuStorage, PerCpuStaticImage);
            per_cpu_storage_boot_cpu_early_access_ready(PerCpuStorage, PerCpuStaticImage);
        }

        events {
            /*
             * Setup 对应 setup_per_cpu_areas() 的完整 possible CPU first chunk
             * 和 offset table 建立。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStaticImage.state == State::Ready;
                    MemBlock.state == State::Online;
                    SwapperVm.state == State::Online;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                }

                drives {
                    PerCpuFirstChunk.Event::Setup;
                    PerCpuOffsetTable.Event::Setup;
                }

                ensures {
                    per_cpu_storage_ready(PerCpuStorage, CpuGroup, PerCpuFirstChunk, PerCpuOffsetTable);
                    per_cpu_static_instances_ready(PerCpuStorage, PerCpuStaticImage, PerCpuFirstChunk);
                    per_cpu_dynamic_reserve_ready(PerCpuStorage, PerCpuFirstChunk);
                    per_cpu_addressing_ready(PerCpuStorage, PerCpuOffsetTable);
                    per_cpu_storage_unit_count_uses_possible_cpus(PerCpuStorage, CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示 per-cpu 存储、静态 percpu 变量寻址事实和基础 offset table
     * 已经可供后续 CPU/hotplug/logging 路径使用。
     */
    state State::Ready {
        invariant {
            PerCpuStaticImage.state == State::Ready;
            PerCpuFirstChunk.state == State::Ready;
            PerCpuOffsetTable.state == State::Ready;
            per_cpu_storage_ready(PerCpuStorage, CpuGroup, PerCpuFirstChunk, PerCpuOffsetTable);
            per_cpu_static_instances_ready(PerCpuStorage, PerCpuStaticImage, PerCpuFirstChunk);
            per_cpu_dynamic_reserve_ready(PerCpuStorage, PerCpuFirstChunk);
            per_cpu_addressing_ready(PerCpuStorage, PerCpuOffsetTable);
            per_cpu_storage_unit_count_uses_possible_cpus(PerCpuStorage, CpuGroup);
        }
    }
}

/*
 * CpuHotplugState 表示 CPUObject 下的通用 hotplug 状态对象类型。
 * 当前 DSL 尚未表达“每个 CPUObject 一份”的实例集合，因此此处先声明
 * BootCPU 实例；后续 SecondaryCPU bringup/teardown 复用同类子状态。
 */
object CpuHotplugState: HardwareObject {
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
                    cpu_hotplug_state_ready(CpuHotplugState, BootCPU);
                    cpu_hotplug_state_current(CpuHotplugState, BootCPU, Online);
                    cpu_hotplug_state_target(CpuHotplugState, BootCPU, Online);
                    cpu_hotplug_ap_sync_state_online(CpuHotplugState, BootCPU);
                    boot_cpu_recorded_booted_once(CpuHotplugState, BootCPU);
                }
            }
        }
    }

    /*
     * Ready 表示 boot CPU 的 hotplug 当前态和目标态已经初始化为 online。
     */
    state State::Ready {
        invariant {
            cpu_hotplug_state_ready(CpuHotplugState, BootCPU);
            cpu_hotplug_state_current(CpuHotplugState, BootCPU, Online);
            cpu_hotplug_state_target(CpuHotplugState, BootCPU, Online);
            cpu_hotplug_ap_sync_state_online(CpuHotplugState, BootCPU);
            boot_cpu_recorded_booted_once(CpuHotplugState, BootCPU);
        }
    }
}

/*
 * BootParam 表示普通内核启动参数解析。它不同于 EntrySuccessor 中的 EarlyParam。
 */
object BootParam: KernelObject {
    initial_state: State::Base;
    parent: Params;

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
    parent: Params;

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
             * Preset 对应 random_init_early(command_line)。Linux 主线直接
             * 使用内部 _mix_pool_bytes() 混入早期材料，不经过带
             * input_pool.lock 的 mix_pool_bytes()；末尾 crng_ready() /
             * trust_cpu 条件路径可能进入 crng_reseed() 或 _credit_init_bits()
             * 并使用 base_crng.lock 的 irqsave 自旋锁，当前最小路径只记录为
             * deferred。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                    StaticCommandLine.state == State::Ready;
                }

                ensures {
                    randomness_early_seed_material_ready(Randomness);
                    static_command_line_mixed_into_randomness(Randomness, StaticCommandLine);
                    arch_entropy_accounted(Randomness, Riscv64);
                    randomness_early_mix_uses_internal_mix_pool_bytes_without_input_pool_lock(Randomness);
                    randomness_early_conditional_reseed_or_credit_path_deferred(Randomness);
                    randomness_base_crng_irqsave_lock_required_if_conditional_reseed_enabled(Randomness);
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
            randomness_early_seed_material_ready(Randomness);
            static_command_line_mixed_into_randomness(Randomness, StaticCommandLine);
            arch_entropy_accounted(Randomness, Riscv64);
            randomness_early_mix_uses_internal_mix_pool_bytes_without_input_pool_lock(Randomness);
            randomness_early_conditional_reseed_or_credit_path_deferred(Randomness);
            randomness_base_crng_irqsave_lock_required_if_conditional_reseed_enabled(Randomness);
            randomness_not_fully_ready(Randomness);
        }

        events {
            /*
             * Setup 对应 random_init()。它在中断打开前补齐完整 RNG 基础，
             * 但不要求熵池已经达到运行期强随机可用状态。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Timekeeper.state == State::Ready;
                    StaticCommandLine.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    randomness_ready(Randomness);
                    randomness_time_seed_material_ready(Randomness, Timekeeper);
                    randomness_boot_cpu_mix_ready(Randomness, CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示 random_init() 已完成，后续代码可依赖完整 RNG 对象壳；
     * 熵质量和阻塞随机语义留给后续运行期模型。
     */
    state State::Ready {
        invariant {
            randomness_ready(Randomness);
            randomness_time_seed_material_ready(Randomness, Timekeeper);
            randomness_boot_cpu_mix_ready(Randomness, CpuGroup);
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
                    Params.state == State::Prepared;
                    EarlyParam.state == State::Ready;
                    CommandLine.state == State::Prepared;
                    BootCPU.state == State::Online;
                    BootCpuLocalInterrupt.state == State::Ready;
                    CpuIdMap.state == State::Prepared;
                    PrintkBuffer.state == State::Prepared;
                    ExceptionStream.state == State::Prepared;
                }

                drives {
                    DeviceTree.Event::Setup;
                    Zones.Event::Setup;
                    PageMetadataMap.Event::Setup;
                    ResourceLock.Event::Preset;
                    ResourceLock.Event::Setup;
                    ResourceTree.Event::Setup;
                    CpuGroup.Event::Setup;
                    CpuIdMap.Event::Setup;
                    CacheBlockInfo.Event::Setup;
                    CpuCapabilities.Event::Setup;
                    DmaCachePolicy.Event::Setup;
                    PerCpuStorage.Event::Preset;
                    CpuHotplugLock.Event::Preset;
                    CpuHotplugLock.Event::Setup;
                    JumpLabelMutex.Event::Preset;
                    JumpLabelMutex.Event::Setup;
                    StaticBranch.Event::Setup;
                    CommandLine.Event::Setup;
                    PerCpuStorage.Event::Setup;
                    CpuHotplugState.Event::Setup;
                    Params.Event::Setup;
                    Randomness.Event::Preset;
                    PrintkBuffer.Event::Setup;
                    ExceptionTable.Event::Setup;
                    ExceptionStream.Event::Setup;
                }

                ensures {
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    context_is(SystemExclusive);
                    early_boot_irqs_disabled_true();
                }

                deferred {
                    "acpi_boot_table_init() 暂缓：当前最小路径以 FDT/SBI 为主，ACPI 引导路径后续单独建模。"
                    "early_memtest() 暂缓：属于可选内存测试路径，不改变当前最小启动语义。"
                    "sparse_init() 裁剪路径：当前 default_config 为 CONFIG_FLATMEM=y、CONFIG_SPARSEMEM=n，该调用展开为空操作。"
                    "local_flush_tlb_kernel_range(VMEMMAP_START, VMEMMAP_END) 暂缓：SPARSEMEM_VMEMMAP 条件路径。"
                    "arch_reserve_crashkernel() 暂缓：crashkernel 资源保留路径后续展开。"
                    "kasan_init() 暂缓：CONFIG_KASAN 条件路径。"
                    "acpi_init_rintc_map() / acpi_map_cpus_to_nodes() 暂缓：依赖 ACPI CPU 拓扑和 NUMA 模型。"
                    "CBOP block size 暂缓：DeviceTree binding 定义 riscv,cbop-block-size，但 Linux 6.12.37 的 riscv_init_cbo_blocksizes() 当前只发布 CBOM/CBOZ。"
                    "apply_boot_alternatives() 暂缓：启动期 alternatives patch 后续抽象为代码补丁设施。"
                    "init_rt_signal_env() 暂缓：用户态信号环境不属于当前最小核心准备路径。"
                    "riscv_user_isa_enable() 暂缓：用户态 ISA 暴露路径后续建模。"
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
            smp_concurrency_closed();
            context_is(SystemExclusive);
            early_boot_irqs_disabled_true();
            EntrySuccessorPhase.state == State::Ready;
            DeviceTree.state == State::Ready;
            Zones.state == State::Ready;
            PageMetadataMap.state == State::Ready;
            ResourceLock.state == State::Ready;
            ResourceTree.state == State::Ready;
            CpuGroup.state == State::Ready;
            CpuIdMap.state == State::Ready;
            CacheBlockInfo.state == State::Ready;
            CpuCapabilities.state == State::Ready;
            DmaCachePolicy.state == State::Ready;
            CpuHotplugLock.state == State::Ready;
            JumpLabelMutex.state == State::Ready;
            StaticBranch.state == State::Ready;
            CommandLine.state == State::Ready;
            SavedCommandLine.state == State::Ready;
            StaticCommandLine.state == State::Ready;
            PerCpuStorage.state == State::Ready;
            PerCpuStaticImage.state == State::Ready;
            PerCpuFirstChunk.state == State::Ready;
            PerCpuOffsetTable.state == State::Ready;
            CpuHotplugState.state == State::Ready;
            Params.state == State::Ready;
            BootParam.state == State::Ready;
            PayloadParam.state == State::Ready;
            Randomness.state == State::Prepared;
            PrintkBuffer.state == State::Ready;
            printk_buffer_setup_local_irq_guard_used(PrintkBuffer, BootCpuLocalInterrupt);
            ExceptionTable.state == State::Ready;
            ExceptionStream.state == State::Ready;
            PageFaultException.state == State::Ready;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Ready;
            UnexpectedException.state == State::Ready;
        }
    }
}
