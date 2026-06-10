/*
 * MM Core Init Phase Specification
 *
 * This subphase starts after trap_init() and ends at the mm_core_init()
 * return boundary. It turns early MemBlock-backed memory management into
 * core page, SLUB, global heap, page-table, vmalloc and mm_struct allocation
 * foundations.
 */

/*
 * MemoryTopology 表示页分配器可见的内存节点拓扑。当前 default_config 为 UMA，
 * 因此只要求唯一 BootMemoryNode；NUMA 扩展后可增加更多 MemoryNode 实例。
 */
object MemoryTopology: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Zones.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                drives {
                    BootMemoryNode.Event::Setup;
                    BootZoneSet.Event::Setup;
                }

                ensures {
                    memory_topology_ready(MemoryTopology, Zones, CpuGroup);
                    memory_topology_uma_single_node(MemoryTopology, BootMemoryNode);
                    memory_node_zone_set_ready(BootMemoryNode, BootZoneSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootMemoryNode.state == State::Ready;
            BootZoneSet.state == State::Ready;
            memory_topology_ready(MemoryTopology, Zones, CpuGroup);
            memory_topology_uma_single_node(MemoryTopology, BootMemoryNode);
            memory_node_zone_set_ready(BootMemoryNode, BootZoneSet);
        }
    }
}

/*
 * BootMemoryNode 对应当前 UMA 配置下唯一的 pg_data_t。
 */
object BootMemoryNode: MemoryObject {
    initial_state: State::Base;
    parent: MemoryTopology;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Zones.state == State::Ready;
                }

                ensures {
                    boot_memory_node_ready(BootMemoryNode, Zones);
                    boot_memory_node_owns_zone_set(BootMemoryNode, BootZoneSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            boot_memory_node_ready(BootMemoryNode, Zones);
            boot_memory_node_owns_zone_set(BootMemoryNode, BootZoneSet);
        }
    }
}

/*
 * BootZoneSet 表示唯一 memory node 拥有的 populated zones。
 */
object BootZoneSet: MemoryObject {
    initial_state: State::Base;
    parent: BootMemoryNode;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Zones.state == State::Ready;
                }

                ensures {
                    boot_zone_set_ready(BootZoneSet, Zones);
                    boot_zone_set_contains_populated_zones(BootZoneSet, Zones);
                    zone_order_migration_free_page_hierarchy_ready(Zones);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            boot_zone_set_ready(BootZoneSet, Zones);
            boot_zone_set_contains_populated_zones(BootZoneSet, Zones);
            zone_order_migration_free_page_hierarchy_ready(Zones);
        }
    }
}

/*
 * BootZonelistSet 表示 build_all_zonelists(NULL) 后的 fallback zonelist 视图。
 */
object BootZonelistSet: MemoryObject {
    initial_state: State::Base;
    parent: BootMemoryNode;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    BootMemoryNode.state == State::Ready;
                    BootZoneSet.state == State::Ready;
                }

                ensures {
                    boot_zonelist_set_ready(BootZonelistSet, BootZoneSet);
                    zonelist_fallback_order_ready(BootZonelistSet);
                    zonelist_zonerefs_reference_zones_without_owning(BootZonelistSet, BootZoneSet);
                    zonelist_null_sentinel_ready(BootZonelistSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            boot_zonelist_set_ready(BootZonelistSet, BootZoneSet);
            zonelist_fallback_order_ready(BootZonelistSet);
            zonelist_zonerefs_reference_zones_without_owning(BootZonelistSet, BootZoneSet);
            zonelist_null_sentinel_ready(BootZonelistSet);
        }
    }
}

/*
 * PageMetadataMap 表示 Linux struct page metadata 视图：FLATMEM 下的
 * mem_map 或 SPARSEMEM_VMEMMAP 下的 vmemmap。PageRef 指向这里的具体
 * PageMetadata 项，再通过 PFN/物理地址/线性映射地址进行转换。
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
 * PageAllocatorBuddyFreePageSets 是 PageAllocator 内部的 buddy free_area
 * 集合在模型中的具名视图。coding 层仍要求它落在 PageAllocator 内部，
 * 不作为外部 heap 容器或独立 allocator。
 */
object PageAllocatorBuddyFreePageSets: BuddyFreePageSetType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    page_metadata_map_ready(PageMetadataMap, Zones);
                    page_metadata_map_indexed_by_pfn(PageMetadataMap);
                }

                ensures {
                    buddy_free_page_sets_ready(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_bound_to_allocator(PageAllocatorBuddyFreePageSets, PageAllocator);
                    buddy_free_page_sets_indexed_by_zone_and_order(PageAllocatorBuddyFreePageSets, Zones);
                    buddy_free_page_sets_use_single_migratetype(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_use_intrusive_lists(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_free_area_heads_ready(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_use_page_metadata_as_nodes(PageAllocatorBuddyFreePageSets, PageMetadataMap);
                    buddy_free_page_sets_nodes_are_block_head_metadata(PageAllocatorBuddyFreePageSets, PageMetadataMap);
                    buddy_free_page_sets_no_external_node_storage(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_populated_from_memblock(PageAllocatorBuddyFreePageSets, MemBlock, Zones);
                    buddy_free_page_sets_exclude_reserved_ranges(PageAllocatorBuddyFreePageSets, MemBlock);
                    buddy_free_page_sets_split_free_ranges_to_aligned_blocks(PageAllocatorBuddyFreePageSets);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            buddy_free_page_sets_ready(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_bound_to_allocator(PageAllocatorBuddyFreePageSets, PageAllocator);
            buddy_free_page_sets_indexed_by_zone_and_order(PageAllocatorBuddyFreePageSets, Zones);
            buddy_free_page_sets_use_single_migratetype(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_use_intrusive_lists(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_free_area_heads_ready(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_use_page_metadata_as_nodes(PageAllocatorBuddyFreePageSets, PageMetadataMap);
            buddy_free_page_sets_nodes_are_block_head_metadata(PageAllocatorBuddyFreePageSets, PageMetadataMap);
            buddy_free_page_sets_no_external_node_storage(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_populated_from_memblock(PageAllocatorBuddyFreePageSets, MemBlock, Zones);
            buddy_free_page_sets_exclude_reserved_ranges(PageAllocatorBuddyFreePageSets, MemBlock);
            buddy_free_page_sets_split_free_ranges_to_aligned_blocks(PageAllocatorBuddyFreePageSets);
        }
    }
}

/*
 * PageAllocator 覆盖 build_all_zonelists(NULL)、page_alloc_init_cpuhp()
 * 和 memblock_free_all()。Enable 保留给后续 page_alloc_init_late()。
 */
object PageAllocator: PageAllocatorType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    MemoryTopology.state == State::Ready;
                    BootZoneSet.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    BootZonelistSet.Event::Setup;
                }

                ensures {
                    page_allocator_zonelists_ready(PageAllocator, BootZonelistSet);
                    page_allocator_cpuhp_step_registered(PageAllocator, CpuHotplugState);
                    page_allocator_boot_pageset_checkpoint_ready(PageAllocator);
                    page_allocator_page_metadata_map_bound(PageAllocator, PageMetadataMap);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            BootZonelistSet.state == State::Ready;
            page_allocator_zonelists_ready(PageAllocator, BootZonelistSet);
            page_allocator_cpuhp_step_registered(PageAllocator, CpuHotplugState);
            page_allocator_boot_pageset_checkpoint_ready(PageAllocator);
            page_allocator_page_metadata_map_bound(PageAllocator, PageMetadataMap);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                    BootZonelistSet.state == State::Ready;
                    MemoryDebugHardening.state == State::Ready;
                    Swiotlb.state == State::Ready;
                }

                drives {
                    PageAllocatorBuddyFreePageSets.Event::Setup;
                    MemBlock.Event::Disable;
                }

                ensures {
                    page_allocator_ready(PageAllocator, Zones);
                    memblock_free_ranges_handed_to_page_allocator(MemBlock, PageAllocator);
                    zone_managed_pages_accounted(Zones, PageAllocator);
                    buddy_free_page_sets_ready(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_bound_to_allocator(PageAllocatorBuddyFreePageSets, PageAllocator);
                    buddy_free_page_sets_indexed_by_zone_and_order(PageAllocatorBuddyFreePageSets, Zones);
                    buddy_free_page_sets_use_single_migratetype(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_use_intrusive_lists(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_free_area_heads_ready(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_use_page_metadata_as_nodes(PageAllocatorBuddyFreePageSets, PageMetadataMap);
                    buddy_free_page_sets_nodes_are_block_head_metadata(PageAllocatorBuddyFreePageSets, PageMetadataMap);
                    buddy_free_page_sets_no_external_node_storage(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_populated_from_memblock(PageAllocatorBuddyFreePageSets, MemBlock, Zones);
                    buddy_free_page_sets_exclude_reserved_ranges(PageAllocatorBuddyFreePageSets, MemBlock);
                    buddy_free_page_sets_split_free_ranges_to_aligned_blocks(PageAllocatorBuddyFreePageSets);
                    buddy_free_page_sets_populated(PageAllocator, Zones);
                    page_allocator_buddy_free_page_sets_bound(PageAllocator, PageAllocatorBuddyFreePageSets);
                    page_allocator_free_pages_account_matches_buddy(PageAllocator, PageAllocatorBuddyFreePageSets);
                    totalram_pages_accounted(PageAllocator);
                    page_allocator_alloc_pages_api_ready(PageAllocator);
                    page_allocator_free_pages_api_ready(PageAllocator);
                    page_allocator_page_ref_conversion_api_ready(PageAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            MemBlock.state == State::Offline;
            page_allocator_ready(PageAllocator, Zones);
            memblock_free_ranges_handed_to_page_allocator(MemBlock, PageAllocator);
            zone_managed_pages_accounted(Zones, PageAllocator);
            PageAllocatorBuddyFreePageSets.state == State::Ready;
            buddy_free_page_sets_ready(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_bound_to_allocator(PageAllocatorBuddyFreePageSets, PageAllocator);
            buddy_free_page_sets_indexed_by_zone_and_order(PageAllocatorBuddyFreePageSets, Zones);
            buddy_free_page_sets_use_single_migratetype(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_use_intrusive_lists(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_free_area_heads_ready(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_use_page_metadata_as_nodes(PageAllocatorBuddyFreePageSets, PageMetadataMap);
            buddy_free_page_sets_nodes_are_block_head_metadata(PageAllocatorBuddyFreePageSets, PageMetadataMap);
            buddy_free_page_sets_no_external_node_storage(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_populated_from_memblock(PageAllocatorBuddyFreePageSets, MemBlock, Zones);
            buddy_free_page_sets_exclude_reserved_ranges(PageAllocatorBuddyFreePageSets, MemBlock);
            buddy_free_page_sets_split_free_ranges_to_aligned_blocks(PageAllocatorBuddyFreePageSets);
            buddy_free_page_sets_populated(PageAllocator, Zones);
            page_allocator_buddy_free_page_sets_bound(PageAllocator, PageAllocatorBuddyFreePageSets);
            page_allocator_free_pages_account_matches_buddy(PageAllocator, PageAllocatorBuddyFreePageSets);
            totalram_pages_accounted(PageAllocator);
            page_allocator_alloc_pages_api_ready(PageAllocator);
            page_allocator_free_pages_api_ready(PageAllocator);
            page_allocator_page_ref_conversion_api_ready(PageAllocator);
        }

    }
}

/*
 * Swiotlb 表示 mem_init() 中释放 MemBlock 前建立的 early SWIOTLB 池或
 * "当前启动不需要 early pool" 的 resolved fact。
 */
object Swiotlb: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DmaCachePolicy.state == State::Ready;
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                }

                ensures {
                    swiotlb_policy_resolved(Swiotlb, DmaCachePolicy, Zones);
                    early_swiotlb_pool_decision_ready(Swiotlb);
                    early_swiotlb_memblock_reservation_ready_if_required(Swiotlb, MemBlock);
                    swiotlb_dynamic_growth_trimmed(Swiotlb);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            swiotlb_policy_resolved(Swiotlb, DmaCachePolicy, Zones);
            early_swiotlb_pool_decision_ready(Swiotlb);
            early_swiotlb_memblock_reservation_ready_if_required(Swiotlb, MemBlock);
            swiotlb_dynamic_growth_trimmed(Swiotlb);
        }
    }
}

/*
 * MemoryDebugHardening 汇总 mem_debugging_and_hardening_init() 的策略结果。
 */
object MemoryDebugHardening: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    StaticBranch.state == State::Ready;
                    EarlyParam.state == State::Ready;
                    Config.state == State::Online;
                }

                ensures {
                    memory_debug_hardening_ready(MemoryDebugHardening, StaticBranch, EarlyParam);
                    init_on_alloc_policy_resolved(MemoryDebugHardening);
                    init_on_free_policy_resolved(MemoryDebugHardening);
                    debug_pagealloc_policy_resolved(MemoryDebugHardening);
                    debug_guardpage_policy_resolved(MemoryDebugHardening);
                    check_pages_policy_resolved(MemoryDebugHardening);
                    memory_debug_static_keys_resolved(MemoryDebugHardening, StaticBranch);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            memory_debug_hardening_ready(MemoryDebugHardening, StaticBranch, EarlyParam);
            init_on_alloc_policy_resolved(MemoryDebugHardening);
            init_on_free_policy_resolved(MemoryDebugHardening);
            debug_pagealloc_policy_resolved(MemoryDebugHardening);
            debug_guardpage_policy_resolved(MemoryDebugHardening);
            check_pages_policy_resolved(MemoryDebugHardening);
            memory_debug_static_keys_resolved(MemoryDebugHardening, StaticBranch);
        }
    }
}

/*
 * StackDepot 表示 stack_depot_early_init() 建立的调用栈存储基础。
 */
object StackDepot: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    MemoryDebugHardening.state == State::Ready;
                }

                ensures {
                    stack_depot_ready(StackDepot, MemBlock);
                    stack_depot_early_storage_ready(StackDepot);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            stack_depot_ready(StackDepot, MemBlock);
            stack_depot_early_storage_ready(StackDepot);
        }
    }
}

/*
 * SlubCacheRegistry 表示 Linux slab_caches registry。
 */
object SlubCacheRegistry: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Prepared;
                }

                ensures {
                    slub_cache_registry_ready(SlubCacheRegistry);
                    boot_slub_caches_registered(SlubCacheRegistry);
                    slub_cache_registry_global_list_ready(SlubCacheRegistry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            slub_cache_registry_ready(SlubCacheRegistry);
            boot_slub_caches_registered(SlubCacheRegistry);
            slub_cache_registry_global_list_ready(SlubCacheRegistry);
        }
    }
}

/*
 * KmallocCaches 表示 kmalloc_caches[][] 与 kmalloc_size_index[]。
 */
object KmallocCaches: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Prepared;
                    SlubCacheRegistry.state == State::Ready;
                }

                ensures {
                    kmalloc_caches_ready(KmallocCaches, SlubCacheRegistry);
                    kmalloc_size_index_ready(KmallocCaches);
                    kmalloc_caches_default_size_classes_ready(KmallocCaches);
                    default_kmalloc_cache_set_ready(KmallocCaches);
                    random_kmalloc_caches_trimmed(KmallocCaches);
                    memcg_kmalloc_caches_trimmed(KmallocCaches);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kmalloc_caches_ready(KmallocCaches, SlubCacheRegistry);
            kmalloc_size_index_ready(KmallocCaches);
            kmalloc_caches_default_size_classes_ready(KmallocCaches);
            default_kmalloc_cache_set_ready(KmallocCaches);
            random_kmalloc_caches_trimmed(KmallocCaches);
            memcg_kmalloc_caches_trimmed(KmallocCaches);
        }
    }
}

/*
 * SlubAllocator 表示 CONFIG_SLUB=y 下的 kmem_cache_init() 自举路径。
 * 本子阶段只推进到 Linux slab_state=UP 对应的 Ready。
 */
object SlubAllocator: SlubAllocatorType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    PageAllocator.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    slub_allocator_partial_ready(SlubAllocator);
                    boot_kmem_cache_node_ready(SlubAllocator);
                    slub_state_partial(SlubAllocator);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            slub_allocator_partial_ready(SlubAllocator);
            boot_kmem_cache_node_ready(SlubAllocator);
            slub_state_partial(SlubAllocator);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PageAllocator.state == State::Ready;
                    StackDepot.state == State::Ready;
                }

                drives {
                    SlubCacheRegistry.Event::Setup;
                    KmallocCaches.Event::Setup;
                }

                ensures {
                    slub_allocator_ready(SlubAllocator, SlubCacheRegistry, KmallocCaches);
                    boot_kmem_cache_bootstrap_completed(SlubAllocator, SlubCacheRegistry);
                    slub_cpu_cache_state_ready(SlubAllocator, PerCpuStorage);
                    slub_cpuhp_step_registered(SlubAllocator, CpuHotplugState);
                    slub_state_up(SlubAllocator);
                    slub_allocator_uses_page_allocator(SlubAllocator, PageAllocator);
                    slub_allocator_kmalloc_api_ready(SlubAllocator);
                    slub_allocator_kzalloc_api_ready(SlubAllocator);
                    slub_allocator_kfree_api_ready(SlubAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SlubCacheRegistry.state == State::Ready;
            KmallocCaches.state == State::Ready;
            slub_allocator_ready(SlubAllocator, SlubCacheRegistry, KmallocCaches);
            boot_kmem_cache_bootstrap_completed(SlubAllocator, SlubCacheRegistry);
            slub_cpu_cache_state_ready(SlubAllocator, PerCpuStorage);
            slub_cpuhp_step_registered(SlubAllocator, CpuHotplugState);
            slub_state_up(SlubAllocator);
            slub_allocator_uses_page_allocator(SlubAllocator, PageAllocator);
            slub_allocator_kmalloc_api_ready(SlubAllocator);
            slub_allocator_kzalloc_api_ready(SlubAllocator);
            slub_allocator_kfree_api_ready(SlubAllocator);
        }
    }
}

/*
 * KernelGlobalAllocator 表示 Rust GlobalAlloc 边界。它不是新的底层分配器，
 * 而是把普通 heap allocation 映射到已经 Ready 的 SLUB/kmalloc。
 */
object KernelGlobalAllocator: KernelGlobalAllocatorType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    kernel_global_allocator_ready(KernelGlobalAllocator, SlubAllocator);
                    kernel_global_allocator_uses_slub_allocator(KernelGlobalAllocator, SlubAllocator);
                    kernel_global_allocator_alloc_api_ready(KernelGlobalAllocator);
                    kernel_global_allocator_alloc_zeroed_api_ready(KernelGlobalAllocator);
                    kernel_global_allocator_dealloc_api_ready(KernelGlobalAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SlubAllocator.state == State::Ready;
            KmallocCaches.state == State::Ready;
            kernel_global_allocator_ready(KernelGlobalAllocator, SlubAllocator);
            kernel_global_allocator_uses_slub_allocator(KernelGlobalAllocator, SlubAllocator);
            kernel_global_allocator_alloc_api_ready(KernelGlobalAllocator);
            kernel_global_allocator_alloc_zeroed_api_ready(KernelGlobalAllocator);
            kernel_global_allocator_dealloc_api_ready(KernelGlobalAllocator);
        }
    }
}

/*
 * DynamicContainerRuntime 表示 Vec/List/Set 等普通动态容器可用的能力边界。
 * 它依赖 KernelGlobalAllocator，而不直接依赖 MemBlock、PageAllocator 或 SLUB
 * 私有结构。
 */
object DynamicContainerRuntime: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelGlobalAllocator.state == State::Ready;
                }

                ensures {
                    dynamic_container_runtime_ready(DynamicContainerRuntime, KernelGlobalAllocator);
                    dynamic_container_runtime_uses_global_allocator(DynamicContainerRuntime, KernelGlobalAllocator);
                    dynamic_container_vec_api_ready(DynamicContainerRuntime);
                    dynamic_container_list_api_ready(DynamicContainerRuntime);
                    dynamic_container_set_api_ready(DynamicContainerRuntime);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            KernelGlobalAllocator.state == State::Ready;
            dynamic_container_runtime_ready(DynamicContainerRuntime, KernelGlobalAllocator);
            dynamic_container_runtime_uses_global_allocator(DynamicContainerRuntime, KernelGlobalAllocator);
            dynamic_container_vec_api_ready(DynamicContainerRuntime);
            dynamic_container_list_api_ready(DynamicContainerRuntime);
            dynamic_container_set_api_ready(DynamicContainerRuntime);
        }
    }
}

/*
 * PageTableLockCache 对应 CONFIG_SPLIT_PTE_PTLOCKS=y 下的 "page->ptl" cache。
 */
object PageTableLockCache: MemoryObject {
    initial_state: State::Base;
    parent: PageTableCaches;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                }

                ensures {
                    page_table_lock_cache_ready(PageTableLockCache, SlubAllocator);
                    page_ptl_cache_created(PageTableLockCache);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_table_lock_cache_ready(PageTableLockCache, SlubAllocator);
            page_ptl_cache_created(PageTableLockCache);
        }
    }
}

/*
 * PageTableCaches 覆盖 ptlock_cache_init() 和 RISC-V pgtable_cache_init()。
 */
object PageTableCaches: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                    SwapperVm.state == State::Online;
                    Vm.state == State::Online;
                }

                drives {
                    PageTableLockCache.Event::Setup;
                }

                ensures {
                    page_table_caches_ready(PageTableCaches, PageTableLockCache);
                    riscv_vmalloc_pgtable_range_preallocated(PageTableCaches, SwapperVm);
                    modules_pgtable_cache_path_trimmed(PageTableCaches);
                    memory_hotplug_pgtable_cache_path_trimmed(PageTableCaches);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            PageTableLockCache.state == State::Ready;
            page_table_caches_ready(PageTableCaches, PageTableLockCache);
            riscv_vmalloc_pgtable_range_preallocated(PageTableCaches, SwapperVm);
            modules_pgtable_cache_path_trimmed(PageTableCaches);
            memory_hotplug_pgtable_cache_path_trimmed(PageTableCaches);
        }
    }
}

/*
 * VmapAreaCache 为 VmapArea 元数据提供 SLUB cache。
 */
object VmapAreaCache: MemoryObject {
    initial_state: State::Base;
    parent: VmallocAllocator;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                }

                ensures {
                    vmap_area_cache_ready(VmapAreaCache, SlubAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_area_cache_ready(VmapAreaCache, SlubAllocator);
        }
    }
}

/*
 * VmapAddressSpace 表示 vmalloc/vmap 虚拟地址资源本体。
 */
object VmapAddressSpace: AddressSpaceObject {
    initial_state: State::Base;
    parent: VmallocAllocator;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VmapAreaCache.state == State::Ready;
                    PageTableCaches.state == State::Ready;
                }

                ensures {
                    vmap_address_space_ready(VmapAddressSpace);
                    existing_vmlist_imported_as_busy_areas(VmapAddressSpace);
                    free_vmap_space_ready(VmapAddressSpace);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_address_space_ready(VmapAddressSpace);
            existing_vmlist_imported_as_busy_areas(VmapAddressSpace);
            free_vmap_space_ready(VmapAddressSpace);
        }
    }
}

/*
 * VmapNodeSet 表示 vmap address space 的索引/分片组织。
 */
object VmapNodeSet: MemoryObject {
    initial_state: State::Base;
    parent: VmallocAllocator;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VmapAddressSpace.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vmap_node_set_ready(VmapNodeSet, VmapAddressSpace);
                    vmap_node_partitions_address_space(VmapNodeSet, VmapAddressSpace);
                    vmap_addr_to_node_route_ready(VmapNodeSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_node_set_ready(VmapNodeSet, VmapAddressSpace);
            vmap_node_partitions_address_space(VmapNodeSet, VmapAddressSpace);
            vmap_addr_to_node_route_ready(VmapNodeSet);
        }
    }
}

/*
 * VmapBlockQueues 表示 per-CPU vmap block fast-path 队列基础。
 */
object VmapBlockQueues: MemoryObject {
    initial_state: State::Base;
    parent: VmallocAllocator;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VmapNodeSet.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vmap_block_queues_ready(VmapBlockQueues, PerCpuStorage);
                    vmap_block_fast_path_metadata_ready(VmapBlockQueues);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_block_queues_ready(VmapBlockQueues, PerCpuStorage);
            vmap_block_fast_path_metadata_ready(VmapBlockQueues);
        }
    }
}

/*
 * VfreeDeferredSet 表示 per-CPU deferred vfree 队列。
 */
object VfreeDeferredSet: MemoryObject {
    initial_state: State::Base;
    parent: VmallocAllocator;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vfree_deferred_set_ready(VfreeDeferredSet, PerCpuStorage);
                    vfree_deferred_work_ready(VfreeDeferredSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vfree_deferred_set_ready(VfreeDeferredSet, PerCpuStorage);
            vfree_deferred_work_ready(VfreeDeferredSet);
        }
    }
}

/*
 * VmallocAllocator 覆盖 vmalloc_init()。
 */
object VmallocAllocator: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                    PageTableCaches.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    VmapAreaCache.Event::Setup;
                    VmapAddressSpace.Event::Setup;
                    VmapNodeSet.Event::Setup;
                    VmapBlockQueues.Event::Setup;
                    VfreeDeferredSet.Event::Setup;
                }

                ensures {
                    vmalloc_allocator_ready(VmallocAllocator, VmapAddressSpace, VmapNodeSet);
                    vmap_initialized(VmallocAllocator);
                    vmap_reclaim_hook_checkpoint_ready(VmallocAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            VmapAreaCache.state == State::Ready;
            VmapAddressSpace.state == State::Ready;
            VmapNodeSet.state == State::Ready;
            VmapBlockQueues.state == State::Ready;
            VfreeDeferredSet.state == State::Ready;
            vmalloc_allocator_ready(VmallocAllocator, VmapAddressSpace, VmapNodeSet);
            vmap_initialized(VmallocAllocator);
            vmap_reclaim_hook_checkpoint_ready(VmallocAllocator);
        }
    }
}

/*
 * MmStructCache 覆盖 mm_cache_init()，只建立 "mm_struct" cache。
 */
object MmStructCache: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    mm_struct_cache_ready(MmStructCache, SlubAllocator);
                    mm_struct_cache_object_size_resolved(MmStructCache, CpuGroup);
                    mm_struct_cache_saved_auxv_usercopy_range_ready(MmStructCache);
                    vma_caches_deferred_to_proc_caches_init(MmStructCache);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mm_struct_cache_ready(MmStructCache, SlubAllocator);
            mm_struct_cache_object_size_resolved(MmStructCache, CpuGroup);
            mm_struct_cache_saved_auxv_usercopy_range_ready(MmStructCache);
            vma_caches_deferred_to_proc_caches_init(MmStructCache);
        }
    }
}

/*
 * MmCoreInitPhase 表示 BootPhase 的第四个子阶段。
 */
object MmCoreInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CorePreparePhase.state == State::Ready;
                    ExceptionStream.state == State::Ready;
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                    DmaCachePolicy.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                    StaticBranch.state == State::Ready;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                }

                drives {
                    MemoryTopology.Event::Setup;
                    PageMetadataMap.Event::Setup;
                    PageAllocator.Event::Preset;
                    MemoryDebugHardening.Event::Setup;
                    StackDepot.Event::Setup;
                    Swiotlb.Event::Setup;
                    PageAllocator.Event::Setup;
                    SlubAllocator.Event::Preset;
                    SlubAllocator.Event::Setup;
                    KernelGlobalAllocator.Event::Setup;
                    DynamicContainerRuntime.Event::Setup;
                    PageTableCaches.Event::Setup;
                    VmallocAllocator.Event::Setup;
                    MmStructCache.Event::Setup;
                }

                ensures {
                    mm_core_init_ready(MmCoreInitPhase);
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                }

                deferred {
                    "PageExt 裁剪路径：CONFIG_PAGE_EXTENSION=n，page_ext_init_flatmem()/page_ext_init_flatmem_late()/page_ext_init() 不建立主线对象。";
                    "KFENCE 裁剪路径：CONFIG_KFENCE=n，kfence_alloc_pool_and_metadata() 当前不进入 formal。";
                    "KMSAN 裁剪路径：CONFIG_KMSAN=n，kmsan_init_shadow()/kmsan_init_runtime() 当前不进入 formal。";
                    "Kmemleak 裁剪路径：CONFIG_DEBUG_KMEMLEAK=n，kmemleak_init() 当前不进入 formal。";
                    "DebugObjectsMemory 裁剪路径：CONFIG_DEBUG_OBJECTS=n，debug_objects_mem_init() 当前不进入 formal。";
                    "ExecMemory 裁剪路径：MODULES=n、BPF_JIT=n、KPROBES=n，execmem_init() 当前为 no-op。";
                    "init_espfix_bsp() 不纳入 RISC-V64 当前路径：x86 特定路径。";
                    "pti_init() 不纳入 RISC-V64 当前路径：x86 PTI 路径。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mm_core_init_ready(MmCoreInitPhase);
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
            CorePreparePhase.state == State::Ready;
            ExceptionStream.state == State::Ready;
            MemoryTopology.state == State::Ready;
            BootMemoryNode.state == State::Ready;
            BootZoneSet.state == State::Ready;
            BootZonelistSet.state == State::Ready;
            PageMetadataMap.state == State::Ready;
            PageAllocatorBuddyFreePageSets.state == State::Ready;
            PageAllocator.state == State::Ready;
            MemBlock.state == State::Offline;
            MemoryDebugHardening.state == State::Ready;
            Swiotlb.state == State::Ready;
            StackDepot.state == State::Ready;
            SlubAllocator.state == State::Ready;
            SlubCacheRegistry.state == State::Ready;
            KmallocCaches.state == State::Ready;
            PageTableCaches.state == State::Ready;
            PageTableLockCache.state == State::Ready;
            VmallocAllocator.state == State::Ready;
            VmapAreaCache.state == State::Ready;
            VmapAddressSpace.state == State::Ready;
            VmapNodeSet.state == State::Ready;
            VmapBlockQueues.state == State::Ready;
            VfreeDeferredSet.state == State::Ready;
            MmStructCache.state == State::Ready;
        }
    }
}
