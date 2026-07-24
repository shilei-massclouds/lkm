/*
 * MM Core Init Phase Specification
 *
 * This subphase starts after trap_init() and ends at the mm_core_init()
 * return boundary. It turns early MemBlock-backed memory management into
 * core page, SLUB, global heap, page-table, vmalloc and mm_struct allocation
 * foundations.
 */

/*
 * MemoryTopology 表示页分配器可见的内存节点拓扑视图。当前 `../linux-6.12/.config`
 * 为 UMA，因此只要求唯一 MemoryNode；NUMA 扩展后可增加更多 MemoryNode
 * 实例。本对象消费 CorePrepare 已建立的 Zones，不重新创建 pg_data_t、
 * node_zones 或 mem_map 本体。
 */
object MemoryTopology: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Zones.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                drives {
                    MemoryNode.Transition::Setup;
                    ZoneSet.Transition::Setup;
                }

                ensures {
                    memory_topology_ready(MemoryTopology, Zones, CpuGroup);
                    memory_topology_uma_single_node(MemoryTopology, MemoryNode);
                    memory_node_zone_set_ready(MemoryNode, ZoneSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            MemoryNode.state == State::Ready;
            ZoneSet.state == State::Ready;
            memory_topology_ready(MemoryTopology, Zones, CpuGroup);
            memory_topology_uma_single_node(MemoryTopology, MemoryNode);
            memory_node_zone_set_ready(MemoryNode, ZoneSet);
        }
    }
}

/*
 * MemoryNode 对应当前 UMA 配置下唯一的节点视图，引用已有 Zones 信息。
 */
object MemoryNode: MemoryObject {
    initial_state: State::Base;
    parent: MemoryTopology;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Zones.state == State::Ready;
                }

                ensures {
                    memory_node_ready(MemoryNode, Zones);
                    memory_node_references_zone_set(MemoryNode, ZoneSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            memory_node_ready(MemoryNode, Zones);
            memory_node_references_zone_set(MemoryNode, ZoneSet);
        }
    }
}

/*
 * ZoneSet 表示唯一 memory node 可见的 populated zone 引用集合。
 */
object ZoneSet: MemoryObject {
    initial_state: State::Base;
    parent: MemoryNode;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Zones.state == State::Ready;
                }

                ensures {
                    zone_set_ready(ZoneSet, Zones);
                    zone_set_references_populated_zones(ZoneSet, Zones);
                    zone_set_does_not_repartition_zones(ZoneSet, Zones);
                    zone_order_migration_free_page_hierarchy_ready(Zones);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            zone_set_ready(ZoneSet, Zones);
            zone_set_references_populated_zones(ZoneSet, Zones);
            zone_set_does_not_repartition_zones(ZoneSet, Zones);
            zone_order_migration_free_page_hierarchy_ready(Zones);
        }
    }
}

/*
 * ZonelistSet 表示 build_all_zonelists(NULL) 后的 fallback zonelist 视图。
 */
object ZonelistSet: MemoryObject {
    initial_state: State::Base;
    parent: MemoryNode;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    MemoryNode.state == State::Ready;
                    ZoneSet.state == State::Ready;
                }

                ensures {
                    zonelist_set_ready(ZonelistSet, ZoneSet);
                    zonelist_fallback_order_ready(ZonelistSet);
                    zonelist_zonerefs_reference_zones_without_owning(ZonelistSet, ZoneSet);
                    zonelist_null_sentinel_ready(ZonelistSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            zonelist_set_ready(ZonelistSet, ZoneSet);
            zonelist_fallback_order_ready(ZonelistSet);
            zonelist_zonerefs_reference_zones_without_owning(ZonelistSet, ZoneSet);
            zonelist_null_sentinel_ready(ZonelistSet);
        }
    }
}

/*
 * ZonelistUpdateSeq 表示 Linux zonelist_update_seq seqlock 的 boot-time
 * writer section。当前只要求 build_all_zonelists(NULL) 写侧 irqsave
 * enter/exit 可观察；完整 reader retry 运行期语义后续再展开。
 */
object ZonelistUpdateSeq: ZonelistUpdateSeqType {
    initial_state: State::Ready;

    state State::Ready {
    }
}

/*
 * ZonelistPrintkDeferredSection 表示 build_all_zonelists(NULL) 外层的
 * printk_deferred_enter()/exit() section。它不拥有 printk ring buffer，
 * 只记录本调用点的 deferred printk protocol 边界。
 */
object ZonelistPrintkDeferredSection: PrintkDeferredSectionType {
    initial_state: State::Ready;

    state State::Ready {
    }
}

context ZonelistPrintkDeferredContext: Context {
    /*
     * This context corresponds to Linux printk_deferred_enter() /
     * printk_deferred_exit() around build_all_zonelists(NULL). It records the
     * scoped printk-deferred protocol boundary; it does not own or mutate the
     * printk ring buffer itself.
     */
    guard {
        entered_by {
            ZonelistPrintkDeferredSection.Transition::Enter;
        }

        exited_by {
            ZonelistPrintkDeferredSection.Transition::Exit;
        }
    }

    obj_refs {
        PageAllocator;
        ZonelistPrintkDeferredSection;
    }
}

context ZonelistUpdateSeqWriteContext: Context {
    /*
     * This context corresponds to Linux
     * write_seqlock_irqsave(&zonelist_update_seq, flags) /
     * write_sequnlock_irqrestore(&zonelist_update_seq, flags) around
     * __build_all_zonelists(NULL). Current modeling covers the boot-time writer
     * section; full reader retry semantics remain a later seqlock refinement.
     */
    guard {
        entered_by {
            ZonelistUpdateSeq.Transition::WriteSeqLockIrqSave(BootCpuLocalInterrupt);
        }

        exited_by {
            ZonelistUpdateSeq.Transition::WriteSeqUnlockIrqRestore(BootCpuLocalInterrupt);
        }
    }

    obj_refs {
        PageAllocator;
        ZonelistUpdateSeq;
        BootCpuLocalInterrupt;
        ZonelistSet;
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PageMetadataMap.state == State::Ready;
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
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    MemoryTopology.state == State::Ready;
                    ZoneSet.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    BootCpuLocalInterrupt.state == State::Ready;
                }

                within ZonelistPrintkDeferredContext {
                    within ZonelistUpdateSeqWriteContext {
                        drives {
                            ZonelistSet.Transition::Setup;
                        }
                    }
                }

                ensures {
                    page_allocator_zonelists_ready(PageAllocator, ZonelistSet);
                    page_allocator_zonelist_update_seq_irqsave_guard_ready(PageAllocator);
                    page_allocator_zonelist_update_seq_irqsave_guard_spec_required(PageAllocator);
                    page_allocator_zonelist_update_seq_guard_used(
                        PageAllocator,
                        ZonelistUpdateSeq,
                        BootCpuLocalInterrupt
                    );
                    page_allocator_zonelist_printk_deferred_section_ready(PageAllocator);
                    page_allocator_zonelist_printk_deferred_section_spec_required(PageAllocator);
                    page_allocator_zonelist_printk_deferred_section_used(
                        PageAllocator,
                        ZonelistPrintkDeferredSection
                    );
                    page_allocator_cpuhp_step_registered(PageAllocator, CpuHotplugState);
                    page_allocator_boot_pageset_checkpoint_ready(PageAllocator);
                    page_allocator_boot_pagesets_initialized_for_possible_cpus(
                        PageAllocator,
                        PerCpuStorage
                    );
                    page_allocator_page_metadata_map_bound(PageAllocator, PageMetadataMap);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ZonelistSet.state == State::Ready;
            page_allocator_zonelists_ready(PageAllocator, ZonelistSet);
            page_allocator_zonelist_update_seq_irqsave_guard_ready(PageAllocator);
            page_allocator_zonelist_update_seq_irqsave_guard_spec_required(PageAllocator);
            page_allocator_zonelist_update_seq_guard_used(
                PageAllocator,
                ZonelistUpdateSeq,
                BootCpuLocalInterrupt
            );
            page_allocator_zonelist_printk_deferred_section_ready(PageAllocator);
            page_allocator_zonelist_printk_deferred_section_spec_required(PageAllocator);
            page_allocator_zonelist_printk_deferred_section_used(
                PageAllocator,
                ZonelistPrintkDeferredSection
            );
            page_allocator_cpuhp_step_registered(PageAllocator, CpuHotplugState);
            page_allocator_boot_pageset_checkpoint_ready(PageAllocator);
            page_allocator_boot_pagesets_initialized_for_possible_cpus(
                PageAllocator,
                PerCpuStorage
            );
            page_allocator_page_metadata_map_bound(PageAllocator, PageMetadataMap);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                    ZonelistSet.state == State::Ready;
                    MemoryDebugHardening.state == State::Ready;
                    Swiotlb.state == State::Ready;
                }

                drives {
                    PageAllocatorBuddyFreePageSets.Transition::Setup;
                    MemBlock.Transition::Disable;
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
                    page_allocator_runtime_zone_locking_contract_deferred(PageAllocator);
                    page_allocator_runtime_pcp_locking_contract_deferred(PageAllocator);
                    page_allocator_full_gfp_reclaim_deferred(PageAllocator);
                    page_allocator_full_compaction_deferred(PageAllocator);
                    page_allocator_full_oom_policy_deferred(PageAllocator);
                    page_allocator_full_failure_propagation_deferred(PageAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            MemBlock.state == State::Offline;
            ZonelistSet.state == State::Ready;
            page_allocator_zonelists_ready(PageAllocator, ZonelistSet);
            page_allocator_zonelist_update_seq_irqsave_guard_ready(PageAllocator);
            page_allocator_zonelist_update_seq_irqsave_guard_spec_required(PageAllocator);
            page_allocator_zonelist_update_seq_guard_used(
                PageAllocator,
                ZonelistUpdateSeq,
                BootCpuLocalInterrupt
            );
            page_allocator_zonelist_printk_deferred_section_ready(PageAllocator);
            page_allocator_zonelist_printk_deferred_section_spec_required(PageAllocator);
            page_allocator_zonelist_printk_deferred_section_used(
                PageAllocator,
                ZonelistPrintkDeferredSection
            );
            page_allocator_cpuhp_step_registered(PageAllocator, CpuHotplugState);
            page_allocator_boot_pageset_checkpoint_ready(PageAllocator);
            page_allocator_boot_pagesets_initialized_for_possible_cpus(
                PageAllocator,
                PerCpuStorage
            );
            page_allocator_page_metadata_map_bound(PageAllocator, PageMetadataMap);
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
            page_allocator_runtime_zone_locking_contract_deferred(PageAllocator);
            page_allocator_runtime_pcp_locking_contract_deferred(PageAllocator);
            page_allocator_full_gfp_reclaim_deferred(PageAllocator);
            page_allocator_full_compaction_deferred(PageAllocator);
            page_allocator_full_oom_policy_deferred(PageAllocator);
            page_allocator_full_failure_propagation_deferred(PageAllocator);
        }

        deferred page_alloc.001 {
            category: DeferredCategory::Feature;
            summary: "Implement complete GFP reclaim behavior for page allocation.";
            evidence { page_allocator_full_gfp_reclaim_deferred(PageAllocator); }
            close_when: "Direct/background reclaim decisions and Linux differential pressure tests pass.";
        }
        deferred page_alloc.002 {
            category: DeferredCategory::Feature;
            summary: "Implement physical-memory compaction for higher-order page allocation.";
            evidence { page_allocator_full_compaction_deferred(PageAllocator); }
            close_when: "Compaction, migration and higher-order allocation pressure tests pass.";
        }
        deferred page_alloc.003 {
            category: DeferredCategory::Protocol;
            summary: "Implement complete out-of-memory policy and victim handling.";
            evidence { page_allocator_full_oom_policy_deferred(PageAllocator); }
            close_when: "OOM selection, recovery, accounting and failure tests match Linux policy.";
        }
        deferred page_alloc.004 {
            category: DeferredCategory::Protocol;
            summary: "Propagate page-allocation failures through every supported caller contract.";
            evidence { page_allocator_full_failure_propagation_deferred(PageAllocator); }
            close_when: "NULL/ERR and retry behavior is specified and tested at all supported allocation callers.";
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    DmaCachePolicy.state == State::Ready;
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                }

                ensures {
                    swiotlb_policy_resolved(Swiotlb, DmaCachePolicy, Zones);
                    early_swiotlb_pool_decision_ready(Swiotlb);
                    early_swiotlb_memblock_reservation_ready_if_required(Swiotlb, MemBlock);
                    swiotlb_static_pool_area_locks_ready_if_required(Swiotlb);
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
            swiotlb_static_pool_area_locks_ready_if_required(Swiotlb);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    StaticBranch.state == State::Ready;
                    EarlyParam.state == State::Ready;
                    Config.state == State::Online;
                }

                ensures {
                    memory_debug_hardening_ready(MemoryDebugHardening, StaticBranch, EarlyParam);
                    memory_debug_early_params_scanned(MemoryDebugHardening, EarlyParam);
                    memory_debug_early_param_policy_trimmed(MemoryDebugHardening);
                    init_on_alloc_policy_resolved(MemoryDebugHardening);
                    init_on_free_policy_resolved(MemoryDebugHardening);
                    debug_pagealloc_policy_resolved(MemoryDebugHardening);
                    debug_guardpage_policy_resolved(MemoryDebugHardening);
                    check_pages_policy_resolved(MemoryDebugHardening);
                    memory_debug_default_policy_selected(MemoryDebugHardening);
                    memory_debug_static_keys_resolved(MemoryDebugHardening, StaticBranch);
                    memory_debug_static_keys_resolved_from_default_policy(
                        MemoryDebugHardening,
                        StaticBranch
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            memory_debug_hardening_ready(MemoryDebugHardening, StaticBranch, EarlyParam);
            memory_debug_early_params_scanned(MemoryDebugHardening, EarlyParam);
            memory_debug_early_param_policy_trimmed(MemoryDebugHardening);
            init_on_alloc_policy_resolved(MemoryDebugHardening);
            init_on_free_policy_resolved(MemoryDebugHardening);
            debug_pagealloc_policy_resolved(MemoryDebugHardening);
            debug_guardpage_policy_resolved(MemoryDebugHardening);
            check_pages_policy_resolved(MemoryDebugHardening);
            memory_debug_default_policy_selected(MemoryDebugHardening);
            memory_debug_static_keys_resolved(MemoryDebugHardening, StaticBranch);
            memory_debug_static_keys_resolved_from_default_policy(
                MemoryDebugHardening,
                StaticBranch
            );
        }
    }
}

/*
 * MmCoreTrimmedPaths 保留当前 Linux .config 下仍出现在 mm_core_init()
 * 调用序列中的裁剪/no-op 位置。它不是 PageExt/KFENCE/KMSAN 等正式
 * 子系统对象，只提供可观测的配置依据和调用位置事实。
 */
object MmCoreTrimmedPaths: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    MemoryDebugHardening.state == State::Ready;
                    StackDepot.state == State::Ready;
                    Ioremap.state == State::Ready;
                    MmStructCache.state == State::Ready;
                }

                ensures {
                    mm_core_page_ext_flatmem_trimmed(MmCoreTrimmedPaths);
                    mm_core_kfence_pool_trimmed(MmCoreTrimmedPaths);
                    mm_core_kmsan_shadow_trimmed(MmCoreTrimmedPaths);
                    mm_core_page_ext_flatmem_late_trimmed(MmCoreTrimmedPaths);
                    mm_core_kmemleak_init_trimmed(MmCoreTrimmedPaths);
                    mm_core_debug_objects_mem_trimmed(MmCoreTrimmedPaths);
                    mm_core_page_ext_final_trimmed(MmCoreTrimmedPaths);
                    mm_core_x86_espfix_not_applicable(MmCoreTrimmedPaths);
                    mm_core_x86_pti_not_applicable(MmCoreTrimmedPaths);
                    mm_core_kmsan_runtime_trimmed(MmCoreTrimmedPaths);
                    mm_core_execmem_init_trimmed_noop(MmCoreTrimmedPaths);
                    mm_core_execmem_trimmed_because_config_execmem_disabled(MmCoreTrimmedPaths);
                    mm_core_trimmed_paths_position_preserved(MmCoreTrimmedPaths);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mm_core_page_ext_flatmem_trimmed(MmCoreTrimmedPaths);
            mm_core_kfence_pool_trimmed(MmCoreTrimmedPaths);
            mm_core_kmsan_shadow_trimmed(MmCoreTrimmedPaths);
            mm_core_page_ext_flatmem_late_trimmed(MmCoreTrimmedPaths);
            mm_core_kmemleak_init_trimmed(MmCoreTrimmedPaths);
            mm_core_debug_objects_mem_trimmed(MmCoreTrimmedPaths);
            mm_core_page_ext_final_trimmed(MmCoreTrimmedPaths);
            mm_core_x86_espfix_not_applicable(MmCoreTrimmedPaths);
            mm_core_x86_pti_not_applicable(MmCoreTrimmedPaths);
            mm_core_kmsan_runtime_trimmed(MmCoreTrimmedPaths);
            mm_core_execmem_init_trimmed_noop(MmCoreTrimmedPaths);
            mm_core_execmem_trimmed_because_config_execmem_disabled(MmCoreTrimmedPaths);
            mm_core_trimmed_paths_position_preserved(MmCoreTrimmedPaths);
        }
    }
}

/*
 * StackDepot 表示 stack_depot_early_init() 的 mm_init 调用点。当前参照
 * .config 中 CONFIG_STACKDEPOT=y，但未选择 CONFIG_STACKDEPOT_ALWAYS_INIT，
 * 且 page_owner/kmemleak/kasan/kfence 等早期消费者未请求 early init。因此
 * Ready 表示该调用点已经执行并建立了后续 init/save 路径的合法边界，不表示
 * early hash table 一定已经通过 memblock 分配。
 */
object StackDepot: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    MemBlock.state == State::Online;
                    MemoryDebugHardening.state == State::Ready;
                }

                ensures {
                    stack_depot_ready(StackDepot, MemBlock);
                    stack_depot_config_enabled(StackDepot);
                    stack_depot_early_init_passed(StackDepot);
                    stack_depot_early_init_request_absent(StackDepot);
                    stack_depot_early_table_allocation_not_required(StackDepot);
                    stack_depot_early_table_not_allocated(StackDepot);
                    stack_depot_late_init_deferred(StackDepot);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            stack_depot_ready(StackDepot, MemBlock);
            stack_depot_config_enabled(StackDepot);
            stack_depot_early_init_passed(StackDepot);
            stack_depot_early_init_request_absent(StackDepot);
            stack_depot_early_table_allocation_not_required(StackDepot);
            stack_depot_early_table_not_allocated(StackDepot);
            stack_depot_late_init_deferred(StackDepot);
        }
    }
}

/*
 * SlubCacheRegistry 表示 Linux slab_caches registry。SlubSubsystem
 * 拥有 registry，registry 统一拥有和枚举所有 SlubCache 实例。
 */
object SlubCacheRegistry: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Prepared;
                }

                ensures {
                    slub_cache_registry_ready(SlubCacheRegistry);
                    slub_cache_registry_owned_by_subsystem(SlubCacheRegistry, SlubSubsystem);
                    slub_cache_registry_owns_all_slub_cache_instances(SlubCacheRegistry);
                    boot_slub_caches_registered(SlubCacheRegistry);
                    slub_cache_registry_global_list_ready(SlubCacheRegistry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            slub_cache_registry_ready(SlubCacheRegistry);
            slub_cache_registry_owned_by_subsystem(SlubCacheRegistry, SlubSubsystem);
            slub_cache_registry_owns_all_slub_cache_instances(SlubCacheRegistry);
            boot_slub_caches_registered(SlubCacheRegistry);
            slub_cache_registry_global_list_ready(SlubCacheRegistry);
        }
    }
}

/*
 * KmallocCaches 表示 kmalloc_caches[][] 与 kmalloc_size_index[]。它只维护
 * size class 到已注册 SlubCache 实例的引用/索引，不拥有这些实例。
 */
object KmallocCaches: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Prepared;
                    SlubCacheRegistry.state == State::Ready;
                }

                ensures {
                    kmalloc_caches_ready(KmallocCaches, SlubCacheRegistry);
                    kmalloc_caches_refer_to_registered_slub_caches(KmallocCaches, SlubCacheRegistry);
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
            kmalloc_caches_refer_to_registered_slub_caches(KmallocCaches, SlubCacheRegistry);
            kmalloc_size_index_ready(KmallocCaches);
            kmalloc_caches_default_size_classes_ready(KmallocCaches);
            default_kmalloc_cache_set_ready(KmallocCaches);
            random_kmalloc_caches_trimmed(KmallocCaches);
            memcg_kmalloc_caches_trimmed(KmallocCaches);
        }
    }
}

/*
 * SlubSubsystem 表示 CONFIG_SLUB=y 下的 kmem_cache_init() 自举路径和
 * 唯一 SLUB facade。SlubCache 是单个 struct kmem_cache 实例的正式类型名；
 * 不再引入 SlubCacheType。
 * 本子阶段只推进到 Linux slab_state=UP 对应的 Ready。
 */
object SlubSubsystem: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PageAllocator.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    slub_subsystem_partial_ready(SlubSubsystem);
                    slub_cache_type_name_is_slub_cache(SlubSubsystem);
                    boot_kmem_cache_node_ready(SlubSubsystem);
                    boot_kmem_cache_node_is_slub_cache_instance(SlubSubsystem);
                    slub_state_partial(SlubSubsystem);
                    slub_subsystem_slab_mutex_not_required_before_full(SlubSubsystem);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            slub_subsystem_partial_ready(SlubSubsystem);
            slub_cache_type_name_is_slub_cache(SlubSubsystem);
            boot_kmem_cache_node_ready(SlubSubsystem);
            boot_kmem_cache_node_is_slub_cache_instance(SlubSubsystem);
            slub_state_partial(SlubSubsystem);
            slub_subsystem_slab_mutex_not_required_before_full(SlubSubsystem);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PageAllocator.state == State::Ready;
                    StackDepot.state == State::Ready;
                }

                drives {
                    SlubCacheRegistry.Transition::Setup;
                    KmallocCaches.Transition::Setup;
                }

                ensures {
                    slub_subsystem_ready(SlubSubsystem, SlubCacheRegistry, KmallocCaches);
                    boot_kmem_cache_bootstrap_completed(SlubSubsystem, SlubCacheRegistry);
                    boot_kmem_cache_is_slub_cache_instance(SlubSubsystem, SlubCacheRegistry);
                    kmem_cache_and_node_registered_as_slub_cache_instances(SlubCacheRegistry);
                    slub_cpu_cache_state_ready(SlubSubsystem, PerCpuStorage);
                    slub_cpuhp_step_registered(SlubSubsystem, CpuHotplugState);
                    slub_state_up(SlubSubsystem);
                    slub_subsystem_uses_page_allocator(SlubSubsystem, PageAllocator);
                    slub_subsystem_kmalloc_api_ready(SlubSubsystem);
                    slub_subsystem_kzalloc_api_ready(SlubSubsystem);
                    slub_subsystem_kfree_api_ready(SlubSubsystem);
                    slub_subsystem_runtime_locking_contract_deferred(SlubSubsystem);
                    slub_subsystem_slab_mutex_not_required_before_full(SlubSubsystem);
                    slub_full_gfp_reclaim_deferred(SlubSubsystem);
                    slub_full_numa_policy_deferred(SlubSubsystem);
                    slub_full_memcg_accounting_deferred(SlubSubsystem);
                    slub_full_debug_redzone_deferred(SlubSubsystem);
                    slub_full_freelist_randomization_deferred(SlubSubsystem);
                    slub_full_freelist_hardening_deferred(SlubSubsystem);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SlubCacheRegistry.state == State::Ready;
            KmallocCaches.state == State::Ready;
            slub_subsystem_ready(SlubSubsystem, SlubCacheRegistry, KmallocCaches);
            boot_kmem_cache_bootstrap_completed(SlubSubsystem, SlubCacheRegistry);
            boot_kmem_cache_is_slub_cache_instance(SlubSubsystem, SlubCacheRegistry);
            kmem_cache_and_node_registered_as_slub_cache_instances(SlubCacheRegistry);
            slub_cpu_cache_state_ready(SlubSubsystem, PerCpuStorage);
            slub_cpuhp_step_registered(SlubSubsystem, CpuHotplugState);
            slub_state_up(SlubSubsystem);
            slub_subsystem_uses_page_allocator(SlubSubsystem, PageAllocator);
            slub_subsystem_kmalloc_api_ready(SlubSubsystem);
            slub_subsystem_kzalloc_api_ready(SlubSubsystem);
            slub_subsystem_kfree_api_ready(SlubSubsystem);
            slub_subsystem_runtime_locking_contract_deferred(SlubSubsystem);
            slub_subsystem_slab_mutex_not_required_before_full(SlubSubsystem);
            slub_full_gfp_reclaim_deferred(SlubSubsystem);
            slub_full_numa_policy_deferred(SlubSubsystem);
            slub_full_memcg_accounting_deferred(SlubSubsystem);
            slub_full_debug_redzone_deferred(SlubSubsystem);
            slub_full_freelist_randomization_deferred(SlubSubsystem);
            slub_full_freelist_hardening_deferred(SlubSubsystem);
        }

        deferred slub_alloc.001 {
            category: DeferredCategory::Feature;
            summary: "Implement SLUB allocation interaction with complete GFP reclaim.";
            evidence { slub_full_gfp_reclaim_deferred(SlubSubsystem); }
            close_when: "SLUB reclaim/retry behavior passes slab-pressure differential tests.";
        }
        deferred slub_alloc.002 {
            category: DeferredCategory::Feature;
            summary: "Implement NUMA-aware SLUB allocation policy.";
            evidence { slub_full_numa_policy_deferred(SlubSubsystem); }
            close_when: "Node selection, fallback and NUMA policy tests match Linux.";
        }
        deferred slub_alloc.003 {
            category: DeferredCategory::Protocol;
            summary: "Implement memory-cgroup charging and rollback for SLUB allocations.";
            evidence { slub_full_memcg_accounting_deferred(SlubSubsystem); }
            close_when: "memcg charge, failure rollback and release accounting tests pass.";
        }
        deferred slub_alloc.004 {
            category: DeferredCategory::ModelDetail;
            summary: "Implement SLUB debug redzones and their corruption checks.";
            evidence { slub_full_debug_redzone_deferred(SlubSubsystem); }
            close_when: "Redzone layout, checking and fault-report tests match enabled Linux behavior.";
        }
        deferred slub_alloc.005 {
            category: DeferredCategory::ModelDetail;
            summary: "Implement SLUB freelist randomization.";
            evidence { slub_full_freelist_randomization_deferred(SlubSubsystem); }
            close_when: "Freelist randomization state and allocation-order tests pass under the enabled configuration.";
        }
        deferred slub_alloc.006 {
            category: DeferredCategory::Protocol;
            summary: "Implement SLUB freelist pointer hardening.";
            evidence { slub_full_freelist_hardening_deferred(SlubSubsystem); }
            close_when: "Freelist encoding, validation and corruption tests pass under the enabled configuration.";
        }

        processes {
            /*
             * Kmalloc corresponds to Linux kmalloc(size, gfp). The returned
             * allocation reference is caller-owned storage from a kmalloc
             * size-class SlubCache backed by PageAllocator pages.
             */
            Action::Kmalloc(size: KmallocSize, gfp: GfpFlags) -> KmallocAllocRef {
                state_effect: StateEffect::None;
                depends_on {
                    self.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    slub_subsystem_kmalloc_api_ready(self);
                    slub_subsystem_kfree_api_ready(self);
                    slub_subsystem_uses_page_allocator(self, PageAllocator);
                    kmalloc_caches_ready_for_size(KmallocCaches, size);
                    page_allocator_gfp_allowed(PageAllocator, gfp);
                }
                ensures {
                    slub_subsystem_kmalloc_called(self, size, gfp);
                    kmalloc_cache_size_class_selected(KmallocCaches, size);
                }
                result {
                    Available: Success(kmalloc_alloc_ref_returned);
                    NoMemory: Failed(no_slab_objects_available);
                }
            }

            /*
             * Kzalloc is kmalloc plus zeroing before the reference is returned.
             */
            Action::Kzalloc(size: KmallocSize, gfp: GfpFlags) -> KmallocAllocRef {
                state_effect: StateEffect::None;
                depends_on {
                    self.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    slub_subsystem_kzalloc_api_ready(self);
                    slub_subsystem_kfree_api_ready(self);
                    slub_subsystem_uses_page_allocator(self, PageAllocator);
                    kmalloc_caches_ready_for_size(KmallocCaches, size);
                    page_allocator_gfp_allowed(PageAllocator, gfp);
                }
                ensures {
                    slub_subsystem_kzalloc_called(self, size, gfp);
                    kmalloc_cache_size_class_selected(KmallocCaches, size);
                }
                result {
                    Available: Success(kmalloc_alloc_ref_returned);
                    NoMemory: Failed(no_slab_objects_available);
                }
            }

            Action::Kfree(alloc_ref: KmallocAllocRef) {
                state_effect: StateEffect::None;
                depends_on {
                    self.state == State::Ready;
                    slub_subsystem_kfree_api_ready(self);
                    kmalloc_alloc_ref_ready(alloc_ref);
                    kmalloc_alloc_ref_exclusively_owned_by_caller(alloc_ref);
                }
                ensures {
                    slub_subsystem_kfree_called(self, alloc_ref);
                    kmalloc_alloc_ref_released_to_slub(alloc_ref, self);
                }
            }
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    kernel_global_allocator_ready(KernelGlobalAllocator, SlubSubsystem);
                    kernel_global_allocator_uses_slub_subsystem(KernelGlobalAllocator, SlubSubsystem);
                    kernel_global_allocator_alloc_api_ready(KernelGlobalAllocator);
                    kernel_global_allocator_alloc_zeroed_api_ready(KernelGlobalAllocator);
                    kernel_global_allocator_dealloc_api_ready(KernelGlobalAllocator);
                    kernel_global_allocator_runtime_sync_inherits_slub_contract(
                        KernelGlobalAllocator,
                        SlubSubsystem
                    );
                    kernel_global_allocator_large_allocation_deferred(KernelGlobalAllocator);
                    kernel_global_allocator_oom_policy_deferred(KernelGlobalAllocator);
                    kernel_global_allocator_realloc_deferred(KernelGlobalAllocator);
                    kernel_global_allocator_alignment_fallback_deferred(KernelGlobalAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SlubSubsystem.state == State::Ready;
            KmallocCaches.state == State::Ready;
            kernel_global_allocator_ready(KernelGlobalAllocator, SlubSubsystem);
            kernel_global_allocator_uses_slub_subsystem(KernelGlobalAllocator, SlubSubsystem);
            kernel_global_allocator_alloc_api_ready(KernelGlobalAllocator);
            kernel_global_allocator_alloc_zeroed_api_ready(KernelGlobalAllocator);
            kernel_global_allocator_dealloc_api_ready(KernelGlobalAllocator);
            kernel_global_allocator_runtime_sync_inherits_slub_contract(
                KernelGlobalAllocator,
                SlubSubsystem
            );
            kernel_global_allocator_large_allocation_deferred(KernelGlobalAllocator);
            kernel_global_allocator_oom_policy_deferred(KernelGlobalAllocator);
            kernel_global_allocator_realloc_deferred(KernelGlobalAllocator);
            kernel_global_allocator_alignment_fallback_deferred(KernelGlobalAllocator);
        }

        deferred global_alloc.001 {
            category: DeferredCategory::AlternatePath;
            summary: "Support heap allocations that bypass ordinary SLUB size classes.";
            evidence { kernel_global_allocator_large_allocation_deferred(KernelGlobalAllocator); }
            close_when: "Large-allocation routing, ownership and deallocation tests pass.";
        }
        deferred global_alloc.002 {
            category: DeferredCategory::Protocol;
            summary: "Implement the global allocator out-of-memory policy.";
            evidence { kernel_global_allocator_oom_policy_deferred(KernelGlobalAllocator); }
            close_when: "Allocator OOM return/abort/recovery behavior is specified and tested.";
        }
        deferred global_alloc.003 {
            category: DeferredCategory::Feature;
            summary: "Implement the global allocator realloc contract.";
            evidence { kernel_global_allocator_realloc_deferred(KernelGlobalAllocator); }
            close_when: "Grow, shrink, move, failure and ownership tests pass.";
        }
        deferred global_alloc.004 {
            category: DeferredCategory::AlternatePath;
            summary: "Implement fallback allocation for alignments unsupported by ordinary SLUB classes.";
            evidence { kernel_global_allocator_alignment_fallback_deferred(KernelGlobalAllocator); }
            close_when: "Over-aligned allocation and deallocation tests pass for all supported layouts.";
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelGlobalAllocator.state == State::Ready;
                }

                ensures {
                    dynamic_container_runtime_ready(DynamicContainerRuntime, KernelGlobalAllocator);
                    dynamic_container_runtime_uses_global_allocator(DynamicContainerRuntime, KernelGlobalAllocator);
                    dynamic_container_vec_api_ready(DynamicContainerRuntime);
                    dynamic_container_list_api_ready(DynamicContainerRuntime);
                    dynamic_container_set_api_ready(DynamicContainerRuntime);
                    dynamic_container_runtime_sync_inherits_global_allocator_contract(
                        DynamicContainerRuntime,
                        KernelGlobalAllocator
                    );
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
            dynamic_container_runtime_sync_inherits_global_allocator_contract(
                DynamicContainerRuntime,
                KernelGlobalAllocator
            );
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    SlubCacheRegistry.state == State::Ready;
                }

                ensures {
                    page_table_lock_cache_ready(PageTableLockCache, SlubSubsystem);
                    page_table_lock_cache_registered_in_slub_registry(
                        PageTableLockCache,
                        SlubCacheRegistry
                    );
                    page_ptl_cache_created(PageTableLockCache);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_table_lock_cache_ready(PageTableLockCache, SlubSubsystem);
            page_table_lock_cache_registered_in_slub_registry(
                PageTableLockCache,
                SlubCacheRegistry
            );
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    Config.state == State::Online;
                    SwapperVm.state == State::Online;
                    Vm.state == State::Online;
                }

                drives {
                    PageTableLockCache.Transition::Setup;
                }

                ensures {
                    page_table_caches_ready(PageTableCaches, PageTableLockCache);
                    riscv_vmalloc_pgtable_range_preallocated(PageTableCaches, SwapperVm);
                    vmalloc_pgtable_dynamic_allocator_ready(PageTableCaches, PageAllocator);
                    vmalloc_pgtable_full_range_metadata_ready(PageTableCaches);
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
            vmalloc_pgtable_dynamic_allocator_ready(PageTableCaches, PageAllocator);
            vmalloc_pgtable_full_range_metadata_ready(PageTableCaches);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    SlubCacheRegistry.state == State::Ready;
                }

                ensures {
                    vmap_area_cache_ready(VmapAreaCache, SlubSubsystem);
                    vmap_area_cache_registered_in_slub_registry(
                        VmapAreaCache,
                        SlubCacheRegistry
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_area_cache_ready(VmapAreaCache, SlubSubsystem);
            vmap_area_cache_registered_in_slub_registry(VmapAreaCache, SlubCacheRegistry);
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
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VmapAddressSpace.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vmap_node_set_ready(VmapNodeSet, VmapAddressSpace);
                    vmap_node_partitions_address_space(VmapNodeSet, VmapAddressSpace);
                    vmap_addr_to_node_route_ready(VmapNodeSet);
                    vmap_node_guard_contract_ready(VmapNodeSet);
                    vmap_node_runtime_spinlock_contract_deferred(VmapNodeSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_node_set_ready(VmapNodeSet, VmapAddressSpace);
            vmap_node_partitions_address_space(VmapNodeSet, VmapAddressSpace);
            vmap_addr_to_node_route_ready(VmapNodeSet);
            vmap_node_guard_contract_ready(VmapNodeSet);
            vmap_node_runtime_spinlock_contract_deferred(VmapNodeSet);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VmapNodeSet.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vmap_block_queues_ready(VmapBlockQueues, PerCpuStorage);
                    vmap_block_fast_path_metadata_ready(VmapBlockQueues);
                    vmap_block_queue_runtime_lock_contract_deferred(VmapBlockQueues);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vmap_block_queues_ready(VmapBlockQueues, PerCpuStorage);
            vmap_block_fast_path_metadata_ready(VmapBlockQueues);
            vmap_block_queue_runtime_lock_contract_deferred(VmapBlockQueues);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vfree_deferred_set_ready(VfreeDeferredSet, PerCpuStorage);
                    vfree_deferred_work_ready(VfreeDeferredSet);
                    vfree_deferred_guard_contract_ready(VfreeDeferredSet);
                    vfree_rcu_runtime_path_deferred(VfreeDeferredSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vfree_deferred_set_ready(VfreeDeferredSet, PerCpuStorage);
            vfree_deferred_work_ready(VfreeDeferredSet);
            vfree_deferred_guard_contract_ready(VfreeDeferredSet);
            vfree_rcu_runtime_path_deferred(VfreeDeferredSet);
        }
    }
}

/*
 * VmallocAllocator 覆盖 vmalloc_init()。它是 vmalloc/vmap 虚拟地址区间
 * 管理器和映射执行器；物理资源来源与 pgprot 策略由调用方决定。
 */
object VmallocAllocator: VmallocAllocatorType {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    PageTableCaches.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    VmapAreaCache.Transition::Setup;
                    VmapAddressSpace.Transition::Setup;
                    VmapNodeSet.Transition::Setup;
                    VmapBlockQueues.Transition::Setup;
                    VfreeDeferredSet.Transition::Setup;
                }

                ensures {
                    vmalloc_allocator_ready(VmallocAllocator, VmapAddressSpace, VmapNodeSet);
                    vmap_initialized(VmallocAllocator);
                    vmalloc_allocator_vmap_area_api_ready(VmallocAllocator);
                    vmalloc_allocator_page_range_mapping_api_ready(VmallocAllocator);
                    vmalloc_allocator_manages_vmap_address_space(VmallocAllocator, VmapAddressSpace);
                    vmalloc_allocator_maintains_vm_struct_metadata(VmallocAllocator);
                    vmalloc_allocator_maintains_vmap_area_metadata(VmallocAllocator);
                    vmalloc_allocator_executes_page_table_mappings(VmallocAllocator, SwapperVm);
                    vmalloc_allocator_runtime_page_table_mapping_ready(VmallocAllocator, PageTableCaches);
                    vmalloc_allocator_mapping_policy_external(VmallocAllocator);
                    vmalloc_allocator_physical_resource_policy_external(VmallocAllocator);
                    vmalloc_allocator_runtime_mapping_window_bound(VmallocAllocator, PageTableCaches);
                    vmalloc_allocator_multi_window_mapping_supported(VmallocAllocator);
                    vmalloc_allocator_preallocated_mapping_window_bound(VmallocAllocator);
                    vmalloc_allocator_dynamic_l0_window_allocation_supported(VmallocAllocator, PageTableCaches);
                    vmalloc_allocator_full_vmalloc_range_metadata_supported(VmallocAllocator, PageTableCaches);
                    vmalloc_allocator_rejects_duplicate_area_mapping(VmallocAllocator);
                    vmalloc_allocator_dynamic_record_storage_ready(VmallocAllocator);
                    vmalloc_allocator_mapping_guard_contract_ready(VmallocAllocator);
                    vmalloc_allocator_unmapping_guard_contract_ready(VmallocAllocator);
                    vmalloc_allocator_mapping_sync_contract_ready(VmallocAllocator, SwapperVm);
                    vmalloc_allocator_unmapping_flush_contract_ready(VmallocAllocator, SwapperVm);
                    vmalloc_allocator_failure_rollback_contract_ready(VmallocAllocator);
                    vmalloc_allocator_setup_runtime_locking_spec_required(VmallocAllocator);
                    vmalloc_allocator_runtime_vmap_locking_contract_deferred(VmallocAllocator);
                    vmalloc_allocator_cross_cpu_vmalloc_flush_deferred(VmallocAllocator);
                    vmalloc_full_reusable_holes_deferred(VmallocAllocator);
                    vmalloc_full_augmented_tree_search_deferred(VmallocAllocator);
                    vmalloc_full_lazy_purge_batching_deferred(VmallocAllocator);
                    vmalloc_full_rcu_metadata_lifecycle_deferred(VmallocAllocator);
                    vmalloc_full_cross_cpu_lazy_fault_deferred(VmallocAllocator);
                    vmalloc_full_cache_tlb_batching_deferred(VmallocAllocator);
                    vmalloc_full_per_cpu_deferred_free_deferred(VmallocAllocator);
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
            vmalloc_allocator_vmap_area_api_ready(VmallocAllocator);
            vmalloc_allocator_page_range_mapping_api_ready(VmallocAllocator);
            vmalloc_allocator_manages_vmap_address_space(VmallocAllocator, VmapAddressSpace);
            vmalloc_allocator_maintains_vm_struct_metadata(VmallocAllocator);
            vmalloc_allocator_maintains_vmap_area_metadata(VmallocAllocator);
            vmalloc_allocator_executes_page_table_mappings(VmallocAllocator, SwapperVm);
            vmalloc_allocator_runtime_page_table_mapping_ready(VmallocAllocator, PageTableCaches);
            vmalloc_allocator_mapping_policy_external(VmallocAllocator);
            vmalloc_allocator_physical_resource_policy_external(VmallocAllocator);
            vmalloc_allocator_runtime_mapping_window_bound(VmallocAllocator, PageTableCaches);
            vmalloc_allocator_multi_window_mapping_supported(VmallocAllocator);
            vmalloc_allocator_preallocated_mapping_window_bound(VmallocAllocator);
            vmalloc_allocator_dynamic_l0_window_allocation_supported(VmallocAllocator, PageTableCaches);
            vmalloc_allocator_full_vmalloc_range_metadata_supported(VmallocAllocator, PageTableCaches);
            vmalloc_allocator_rejects_duplicate_area_mapping(VmallocAllocator);
            vmalloc_allocator_dynamic_record_storage_ready(VmallocAllocator);
            vmalloc_allocator_mapping_guard_contract_ready(VmallocAllocator);
            vmalloc_allocator_unmapping_guard_contract_ready(VmallocAllocator);
            vmalloc_allocator_mapping_sync_contract_ready(VmallocAllocator, SwapperVm);
            vmalloc_allocator_unmapping_flush_contract_ready(VmallocAllocator, SwapperVm);
            vmalloc_allocator_failure_rollback_contract_ready(VmallocAllocator);
            vmalloc_allocator_setup_runtime_locking_spec_required(VmallocAllocator);
            vmalloc_allocator_runtime_vmap_locking_contract_deferred(VmallocAllocator);
            vmalloc_allocator_cross_cpu_vmalloc_flush_deferred(VmallocAllocator);
            vmalloc_full_reusable_holes_deferred(VmallocAllocator);
            vmalloc_full_augmented_tree_search_deferred(VmallocAllocator);
            vmalloc_full_lazy_purge_batching_deferred(VmallocAllocator);
            vmalloc_full_rcu_metadata_lifecycle_deferred(VmallocAllocator);
            vmalloc_full_cross_cpu_lazy_fault_deferred(VmallocAllocator);
            vmalloc_full_cache_tlb_batching_deferred(VmallocAllocator);
            vmalloc_full_per_cpu_deferred_free_deferred(VmallocAllocator);
            vmap_reclaim_hook_checkpoint_ready(VmallocAllocator);
        }

        deferred vmalloc_runtime.001 {
            category: DeferredCategory::ModelDetail;
            summary: "Model reusable holes in the vmalloc virtual-address space.";
            evidence { vmalloc_full_reusable_holes_deferred(VmallocAllocator); }
            close_when: "Hole creation, coalescing, reuse and fragmentation tests pass.";
        }
        deferred vmalloc_runtime.002 {
            category: DeferredCategory::ModelDetail;
            summary: "Implement Linux augmented-tree search for vmap areas.";
            evidence { vmalloc_full_augmented_tree_search_deferred(VmallocAllocator); }
            close_when: "Augmentation maintenance and address/size/alignment search tests match Linux.";
        }
        deferred vmalloc_runtime.003 {
            category: DeferredCategory::Protocol;
            summary: "Implement lazy vmalloc purge batching.";
            evidence { vmalloc_full_lazy_purge_batching_deferred(VmallocAllocator); }
            close_when: "Purge thresholds, batching, flush ordering and reuse tests pass.";
        }
        deferred vmalloc_runtime.004 {
            category: DeferredCategory::Protocol;
            summary: "Implement the RCU lifecycle for vmalloc metadata.";
            evidence { vmalloc_full_rcu_metadata_lifecycle_deferred(VmallocAllocator); }
            close_when: "Publication, lookup, retirement and grace-period tests pass.";
        }
        deferred vmalloc_runtime.005 {
            category: DeferredCategory::AlternatePath;
            summary: "Implement cross-CPU lazy vmalloc fault handling.";
            evidence { vmalloc_full_cross_cpu_lazy_fault_deferred(VmallocAllocator); }
            close_when: "Lazy-fault synchronization and remote-CPU mapping visibility tests pass.";
        }
        deferred vmalloc_runtime.006 {
            category: DeferredCategory::Protocol;
            summary: "Implement cache and TLB batching for vunmap and vfree.";
            evidence { vmalloc_full_cache_tlb_batching_deferred(VmallocAllocator); }
            close_when: "Unmap batching, architecture cache maintenance and TLB ordering tests pass.";
        }
        deferred vmalloc_runtime.007 {
            category: DeferredCategory::Protocol;
            summary: "Implement per-CPU deferred-free ordering for vmalloc areas.";
            evidence { vmalloc_full_per_cpu_deferred_free_deferred(VmallocAllocator); }
            close_when: "Per-CPU enqueue, drain, ownership and reuse ordering tests pass.";
        }
    }
}

/*
 * Ioremap 表示 runtime ioremap() 服务：它复用 vmalloc/vmap 的虚拟地址
 * area 管理，但映射对象是设备 MMIO 物理区间，且页表属性必须是 IO memory。
 * 这不是 EarlyIoremap/FixMap 的早期临时 slot。
 */
object Ioremap: AddressSpaceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SwapperVm.state == State::Online;
                    VmallocAllocator.state == State::Ready;
                    VmapAddressSpace.state == State::Ready;
                    PageTableCaches.state == State::Ready;
                    FixMap.state == State::Ready;
                }

                ensures {
                    ioremap_runtime_ready(Ioremap, VmallocAllocator, VmapAddressSpace, PageTableCaches);
                    ioremap_uses_swapper_vm(Ioremap, SwapperVm);
                    ioremap_uses_vmalloc_area_management(Ioremap, VmallocAllocator);
                    ioremap_uses_vmalloc_mapping_execution(Ioremap, VmallocAllocator);
                    ioremap_uses_vmap_address_space(Ioremap, VmapAddressSpace);
                    ioremap_distinct_from_vmalloc_allocation(Ioremap);
                    ioremap_does_not_use_fixmap(Ioremap, FixMap);
                    ioremap_physical_resource_policy_ready(Ioremap);
                    ioremap_vm_ioremap_flags_ready(Ioremap, VmapAreaFlags::VmIoremap);
                    ioremap_io_page_protection_ready(Ioremap);
                    ioremap_mmio_attribute_policy_ready(Ioremap);
                    ioremap_plain_device_attribute_supported(Ioremap, ArchMmioPageAttrRef::RiscvPageIoremap);
                    ioremap_noncached_attribute_deferred(Ioremap);
                    ioremap_writecombine_attribute_deferred(Ioremap);
                    ioremap_normal_memory_attribute_deferred(Ioremap);
                    ioremap_mapping_guard_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_unmapping_guard_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_mapping_sync_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_unmapping_flush_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_failure_rollback_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_runtime_sync_inherits_vmalloc_contract(Ioremap, VmallocAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ioremap_runtime_ready(Ioremap, VmallocAllocator, VmapAddressSpace, PageTableCaches);
            ioremap_uses_swapper_vm(Ioremap, SwapperVm);
            ioremap_uses_vmalloc_area_management(Ioremap, VmallocAllocator);
            ioremap_uses_vmalloc_mapping_execution(Ioremap, VmallocAllocator);
            ioremap_uses_vmap_address_space(Ioremap, VmapAddressSpace);
            ioremap_distinct_from_vmalloc_allocation(Ioremap);
            ioremap_does_not_use_fixmap(Ioremap, FixMap);
            ioremap_physical_resource_policy_ready(Ioremap);
            ioremap_vm_ioremap_flags_ready(Ioremap, VmapAreaFlags::VmIoremap);
            ioremap_io_page_protection_ready(Ioremap);
            ioremap_mmio_attribute_policy_ready(Ioremap);
            ioremap_plain_device_attribute_supported(Ioremap, ArchMmioPageAttrRef::RiscvPageIoremap);
            ioremap_noncached_attribute_deferred(Ioremap);
            ioremap_writecombine_attribute_deferred(Ioremap);
            ioremap_normal_memory_attribute_deferred(Ioremap);
            ioremap_mapping_guard_contract_ready(Ioremap, VmallocAllocator);
            ioremap_unmapping_guard_contract_ready(Ioremap, VmallocAllocator);
            ioremap_mapping_sync_contract_ready(Ioremap, VmallocAllocator);
            ioremap_unmapping_flush_contract_ready(Ioremap, VmallocAllocator);
            ioremap_failure_rollback_contract_ready(Ioremap, VmallocAllocator);
            ioremap_runtime_sync_inherits_vmalloc_contract(Ioremap, VmallocAllocator);
        }

        actions {
            Action::MapDeviceMmio(device: DeviceRef, mapping: IoMemoryMappingRef) {
                state_effect: StateEffect::None;
                depends_on {
                    Ioremap.state == State::Ready;
                    DeviceTree.state == State::Ready;
                    VmallocAllocator.state == State::Ready;
                    VmapAddressSpace.state == State::Ready;
                    device_ref_ready(device);
                    ioremap_physical_resource_policy_ready(Ioremap);
                    ioremap_vm_ioremap_flags_ready(Ioremap, VmapAreaFlags::VmIoremap);
                    ioremap_io_page_protection_ready(Ioremap);
                    ioremap_mmio_attribute_policy_ready(Ioremap);
                    ioremap_plain_device_attribute_supported(Ioremap, ArchMmioPageAttrRef::RiscvPageIoremap);
                    ioremap_mapping_guard_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_mapping_sync_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_failure_rollback_contract_ready(Ioremap, VmallocAllocator);
                    vmap_flags_vm_ioremap(VmapAreaFlags::VmIoremap);
                    page_protection_io_memory(PageProtectionRef::IoMemory);
                    page_protection_kind_io_memory(VmapPageProtectionKind::IoMemory);
                }

                drives {
                    VmallocAllocator.Action::GetVmArea(
                        area: VmapAreaRef::IoremapDeviceMmio,
                        flags: VmapAreaFlags::VmIoremap
                    );
                    VmallocAllocator.Action::MapPageRange(
                        area: VmapAreaRef::IoremapDeviceMmio,
                        mapping: VmapMappingRef::IoremapDeviceMmio,
                        protection: PageProtectionRef::IoMemory
                    );
                }

                ensures {
                    ioremap_mapping_created(Ioremap, mapping);
                    ioremap_mapping_owner_bound(Ioremap, mapping, device);
                    ioremap_mapping_phys_range_bound(Ioremap, mapping);
                    ioremap_mapping_phys_range_from_device_resource(Ioremap, mapping, device);
                    ioremap_mapping_vmap_area_bound(Ioremap, mapping, VmapAreaRef::IoremapDeviceMmio);
                    ioremap_mapping_vmalloc_mapping_bound(Ioremap, mapping, VmapMappingRef::IoremapDeviceMmio);
                    ioremap_mapping_uses_vm_ioremap_flag(Ioremap, mapping);
                    ioremap_mapping_uses_io_page_protection(Ioremap, mapping);
                    ioremap_mapping_kind_bound(Ioremap, mapping, MmioMappingKind::PlainDevice);
                    ioremap_mapping_arch_attr_bound(Ioremap, mapping, ArchMmioPageAttrRef::RiscvPageIoremap);
                    ioremap_mapping_uses_plain_device_attribute(Ioremap, mapping);
                    ioremap_mapping_does_not_claim_noncached(Ioremap, mapping);
                    ioremap_mapping_does_not_claim_writecombine(Ioremap, mapping);
                    ioremap_mapping_does_not_claim_normal_memory(Ioremap, mapping);
                    ioremap_mapping_page_aligned(Ioremap, mapping);
                    ioremap_mapping_membase_cookie_ready(Ioremap, mapping);
                    ioremap_mapping_not_linear_direct_map(Ioremap, mapping);
                    ioremap_mapping_sync_observed(Ioremap, mapping);
                    vmap_area_ref_ready(VmapAreaRef::IoremapDeviceMmio);
                    vmap_area_allocated(VmapAreaRef::IoremapDeviceMmio, VmallocAllocator);
                    vmap_area_address_space_bound(VmapAreaRef::IoremapDeviceMmio, VmapAddressSpace);
                    vmap_area_flags_bound(VmapAreaRef::IoremapDeviceMmio, VmapAreaFlags::VmIoremap);
                    vmap_area_reserved_as_busy(VmapAreaRef::IoremapDeviceMmio, VmapAddressSpace);
                    vmap_area_has_no_existing_mapping(VmapAreaRef::IoremapDeviceMmio, VmallocAllocator);
                    vmap_mapping_ref_ready(VmapMappingRef::IoremapDeviceMmio);
                    vmap_mapping_area_bound(VmapMappingRef::IoremapDeviceMmio, VmapAreaRef::IoremapDeviceMmio);
                    vmap_area_range_complete_for_mapping(VmapAreaRef::IoremapDeviceMmio, VmapMappingRef::IoremapDeviceMmio);
                    vmap_area_has_no_existing_mapping(VmapAreaRef::IoremapDeviceMmio, VmallocAllocator);
                    vmap_mapping_phys_range_bound(VmapMappingRef::IoremapDeviceMmio);
                    vmap_mapping_page_range_installed(VmapMappingRef::IoremapDeviceMmio, SwapperVm);
                    vmap_mapping_kernel_mapping_synced(VmapMappingRef::IoremapDeviceMmio, SwapperVm);
                    vmap_mapping_protection_bound(VmapMappingRef::IoremapDeviceMmio, PageProtectionRef::IoMemory);
                    vmap_mapping_protection_kind_bound(VmapMappingRef::IoremapDeviceMmio, VmapPageProtectionKind::IoMemory);
                    vmap_mapping_page_aligned(VmapMappingRef::IoremapDeviceMmio);
                    vmap_mapping_within_runtime_mapping_window(VmapMappingRef::IoremapDeviceMmio);
                }
            }

            Action::UnmapDeviceMmio(mapping: IoMemoryMappingRef) {
                state_effect: StateEffect::None;
                depends_on {
                    Ioremap.state == State::Ready;
                    VmallocAllocator.state == State::Ready;
                    ioremap_unmapping_guard_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_unmapping_flush_contract_ready(Ioremap, VmallocAllocator);
                    ioremap_mapping_created(Ioremap, mapping);
                    ioremap_mapping_vmap_area_bound(Ioremap, mapping, VmapAreaRef::IoremapDeviceMmio);
                    ioremap_mapping_vmalloc_mapping_bound(Ioremap, mapping, VmapMappingRef::IoremapDeviceMmio);
                    ioremap_mapping_membase_cookie_ready(Ioremap, mapping);
                    vmap_area_ref_ready(VmapAreaRef::IoremapDeviceMmio);
                    vmap_area_allocated(VmapAreaRef::IoremapDeviceMmio, VmallocAllocator);
                    vmap_mapping_ref_ready(VmapMappingRef::IoremapDeviceMmio);
                    vmap_mapping_area_bound(VmapMappingRef::IoremapDeviceMmio, VmapAreaRef::IoremapDeviceMmio);
                    vmap_mapping_page_range_installed(VmapMappingRef::IoremapDeviceMmio, SwapperVm);
                }

                drives {
                    VmallocAllocator.Action::UnmapPageRange(
                        area: VmapAreaRef::IoremapDeviceMmio,
                        mapping: VmapMappingRef::IoremapDeviceMmio
                    );
                    VmallocAllocator.Action::FreeVmArea(
                        area: VmapAreaRef::IoremapDeviceMmio
                    );
                }

                ensures {
                    ioremap_mapping_unmapped(Ioremap, mapping);
                    ioremap_mapping_membase_cookie_retired(Ioremap, mapping);
                    vmap_mapping_page_range_removed(VmapMappingRef::IoremapDeviceMmio, SwapperVm);
                    vmap_mapping_kernel_tlb_flushed(VmapMappingRef::IoremapDeviceMmio, SwapperVm);
                    ioremap_unmapping_flush_observed(Ioremap, mapping);
                    vmap_area_released(VmapAreaRef::IoremapDeviceMmio, VmallocAllocator);
                }
            }
        }
    }
}

/*
 * MmStructCache 覆盖 mm_cache_init()，只建立 "mm_struct" cache。
 */
object MmStructCache: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    SlubCacheRegistry.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    mm_struct_cache_ready(MmStructCache, SlubSubsystem);
                    mm_struct_cache_registered_in_slub_registry(
                        MmStructCache,
                        SlubCacheRegistry
                    );
                    mm_struct_cache_is_slub_cache_instance(MmStructCache);
                    mm_struct_cache_named_mm_struct(MmStructCache);
                    mm_struct_cache_object_size_resolved(MmStructCache, CpuGroup);
                    mm_struct_cache_saved_auxv_usercopy_range_ready(MmStructCache);
                    vma_caches_deferred_to_proc_caches_init(MmStructCache);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mm_struct_cache_ready(MmStructCache, SlubSubsystem);
            mm_struct_cache_registered_in_slub_registry(MmStructCache, SlubCacheRegistry);
            mm_struct_cache_is_slub_cache_instance(MmStructCache);
            mm_struct_cache_named_mm_struct(MmStructCache);
            mm_struct_cache_object_size_resolved(MmStructCache, CpuGroup);
            mm_struct_cache_saved_auxv_usercopy_range_ready(MmStructCache);
            vma_caches_deferred_to_proc_caches_init(MmStructCache);
        }
    }
}

/*
 * MmCoreInitPhase 表示 BootInitFlow.Setup 的第三个 boot 叶阶段。
 */
object MmCoreInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    CorePreparePhase.state == State::Online;
                    ExceptionStream.state == State::Ready;
                    MemBlock.state == State::Online;
                    Zones.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    DmaCachePolicy.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                    StaticBranch.state == State::Ready;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                }

                drives {
                    MemoryTopology.Transition::Setup;
                    PageAllocator.Transition::Preset;
                    MemoryDebugHardening.Transition::Setup;
                    StackDepot.Transition::Setup;
                    Swiotlb.Transition::Setup;
                    PageAllocator.Transition::Setup;
                    SlubSubsystem.Transition::Preset;
                    SlubSubsystem.Transition::Setup;
                    KernelGlobalAllocator.Transition::Setup;
                    DynamicContainerRuntime.Transition::Setup;
                    PageTableCaches.Transition::Setup;
                    VmallocAllocator.Transition::Setup;
                    Ioremap.Transition::Setup;
                    MmStructCache.Transition::Setup;
                    MmCoreTrimmedPaths.Transition::Setup;
                }

                ensures {
                    mm_core_init_ready(MmCoreInitPhase);
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                }

                trimmed mm_core.001 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "Page-extension initialization is absent because CONFIG_PAGE_EXTENSION=n.";
                    evidence {
                        mm_core_page_ext_flatmem_trimmed(MmCoreTrimmedPaths);
                        mm_core_page_ext_flatmem_late_trimmed(MmCoreTrimmedPaths);
                        mm_core_page_ext_final_trimmed(MmCoreTrimmedPaths);
                    }
                    revisit_when: "The reference configuration enables CONFIG_PAGE_EXTENSION.";
                }
                trimmed mm_core.002 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "KFENCE pool allocation is absent because CONFIG_KFENCE=n.";
                    evidence { mm_core_kfence_pool_trimmed(MmCoreTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_KFENCE.";
                }
                trimmed mm_core.003 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "KMSAN shadow and runtime initialization are absent because CONFIG_KMSAN=n.";
                    evidence {
                        mm_core_kmsan_shadow_trimmed(MmCoreTrimmedPaths);
                        mm_core_kmsan_runtime_trimmed(MmCoreTrimmedPaths);
                    }
                    revisit_when: "The reference configuration enables CONFIG_KMSAN.";
                }
                trimmed mm_core.004 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "kmemleak initialization is absent because CONFIG_DEBUG_KMEMLEAK=n.";
                    evidence { mm_core_kmemleak_init_trimmed(MmCoreTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_DEBUG_KMEMLEAK.";
                }
                trimmed mm_core.005 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "debug_objects_mem_init is absent because CONFIG_DEBUG_OBJECTS=n.";
                    evidence { mm_core_debug_objects_mem_trimmed(MmCoreTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_DEBUG_OBJECTS.";
                }
                trimmed mm_core.006 {
                    category: TrimmedCategory::CompileTimeNoOp;
                    summary: "execmem_init is an inline no-op because CONFIG_EXECMEM is disabled.";
                    evidence {
                        mm_core_execmem_init_trimmed_noop(MmCoreTrimmedPaths);
                        mm_core_execmem_trimmed_because_config_execmem_disabled(MmCoreTrimmedPaths);
                    }
                    revisit_when: "The target configuration enables CONFIG_EXECMEM.";
                }
                trimmed mm_core.007 {
                    category: TrimmedCategory::Architecture;
                    summary: "init_espfix_bsp is an x86-only path and is unreachable on RISC-V64.";
                    evidence { mm_core_x86_espfix_not_applicable(MmCoreTrimmedPaths); }
                    revisit_when: "The model target architecture changes to x86.";
                }
                trimmed mm_core.008 {
                    category: TrimmedCategory::Architecture;
                    summary: "pti_init is an x86 PTI path and is unreachable on RISC-V64.";
                    evidence { mm_core_x86_pti_not_applicable(MmCoreTrimmedPaths); }
                    revisit_when: "The model target architecture changes to an architecture with this PTI initialization path.";
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    mm_core_init_ready(MmCoreInitPhase);
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                    CorePreparePhase.state == State::Online;
                    ExceptionStream.state == State::Ready;
                    MemoryTopology.state == State::Ready;
                    MemoryNode.state == State::Ready;
                    ZoneSet.state == State::Ready;
                    ZonelistSet.state == State::Ready;
                    ZonelistUpdateSeq.state == State::Ready;
                    ZonelistPrintkDeferredSection.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    PageAllocatorBuddyFreePageSets.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    MemBlock.state == State::Offline;
                    MemoryDebugHardening.state == State::Ready;
                    Swiotlb.state == State::Ready;
                    StackDepot.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
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
                    Ioremap.state == State::Ready;
                    MmStructCache.state == State::Ready;
                    MmCoreTrimmedPaths.state == State::Ready;
                }

                emits {
                    Transition::Enable;
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
            CorePreparePhase.state == State::Online;
            ExceptionStream.state == State::Ready;
            MemoryTopology.state == State::Ready;
            MemoryNode.state == State::Ready;
            ZoneSet.state == State::Ready;
            ZonelistSet.state == State::Ready;
            ZonelistUpdateSeq.state == State::Ready;
            ZonelistPrintkDeferredSection.state == State::Ready;
            PageMetadataMap.state == State::Ready;
            PageAllocatorBuddyFreePageSets.state == State::Ready;
            PageAllocator.state == State::Ready;
            MemBlock.state == State::Offline;
            MemoryDebugHardening.state == State::Ready;
            Swiotlb.state == State::Ready;
            StackDepot.state == State::Ready;
            SlubSubsystem.state == State::Ready;
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
            Ioremap.state == State::Ready;
            MmStructCache.state == State::Ready;
            MmCoreTrimmedPaths.state == State::Ready;
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    mm_core_init_ready(MmCoreInitPhase);
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                    CorePreparePhase.state == State::Online;
                    ExceptionStream.state == State::Ready;
                    MemoryTopology.state == State::Ready;
                    MemoryNode.state == State::Ready;
                    ZoneSet.state == State::Ready;
                    ZonelistSet.state == State::Ready;
                    ZonelistUpdateSeq.state == State::Ready;
                    ZonelistPrintkDeferredSection.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                    PageAllocatorBuddyFreePageSets.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    MemBlock.state == State::Offline;
                    MemoryDebugHardening.state == State::Ready;
                    Swiotlb.state == State::Ready;
                    StackDepot.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
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
                    Ioremap.state == State::Ready;
                    MmStructCache.state == State::Ready;
                    MmCoreTrimmedPaths.state == State::Ready;
                }
            }
        }
    }

    state State::Online {
    }
}
