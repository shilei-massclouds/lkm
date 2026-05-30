/*
 * arceos_ex Object Coding Specification
 *
 * This file records target-specific object-coding constraints for the current
 * arceos_ex prototype. It does not redefine model semantics; it constrains how
 * code must realize selected model events in this implementation target.
 */

predicate arceos_ex_must_device_tree_setup_allocates_from_memblock() -> bool;
predicate arceos_ex_must_device_tree_unflatten_uses_two_passes() -> bool;
predicate arceos_ex_must_device_tree_storage_uses_established_linear_mapping() -> bool;
predicate arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage() -> bool;
predicate arceos_ex_must_device_tree_checkpoint_after_validation() -> bool;
predicate arceos_ex_should_encapsulate_device_tree_unflatten_unsafe() -> bool;
predicate arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks() -> bool;
predicate arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy() -> bool;
predicate arceos_ex_must_memblock_disable_reaches_offline_not_destroyed() -> bool;
predicate arceos_ex_must_swiotlb_setup_before_memblock_disable() -> bool;
predicate arceos_ex_must_memory_debug_hardening_use_static_branch_registry() -> bool;
predicate arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready() -> bool;
predicate arceos_ex_must_page_table_lock_cache_named_page_ptl() -> bool;
predicate arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_not_page_tables() -> bool;
predicate arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects() -> bool;
predicate arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache() -> bool;
predicate arceos_ex_should_keep_mm_core_init_checkpoints_observable() -> bool;

type ArceosExDeviceTreeCodingMust {
    invariant {
        /*
         * MemBlock allocation:
         *
         * DeviceTree.setup() must allocate the expanded tree storage through
         * MemBlock early allocation. Its dependency on MemBlock.Online is an
         * allocation capability, not merely a read-only state check.
         */
        arceos_ex_must_device_tree_setup_allocates_from_memblock();

        /*
         * Two-pass unflatten:
         *
         * DeviceTree.setup() must first traverse RawDtb to validate and compute
         * the required expanded storage, then allocate storage, then traverse
         * RawDtb again to populate DeviceNode and property relations.
         */
        arceos_ex_must_device_tree_unflatten_uses_two_passes();

        /*
         * Established mapping:
         *
         * MemBlock returns physical storage. DeviceTree.setup() may write the
         * expanded tree only after resolving that storage through the already
         * established kernel linear mapping provided by SwapperVm/Config.
         */
        arceos_ex_must_device_tree_storage_uses_established_linear_mapping();

        /*
         * No heap or fixed static substitute:
         *
         * DeviceTree.setup() must not use the ordinary heap, Vec/Box, or a
         * fixed static array as the final expanded tree storage. A bounded
         * temporary stack/global workspace is allowed only for traversal state,
         * not as the DeviceTree storage itself.
         */
        arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage();

        /*
         * Checkpoint after validation:
         *
         * The DeviceTree Ready checkpoint may be emitted only after the second
         * pass has populated the tree and the model-level root, parent/child,
         * path lookup and property query facts have been checked.
         */
        arceos_ex_must_device_tree_checkpoint_after_validation();
    }
}

type ArceosExDeviceTreeCodingShould {
    invariant {
        /*
         * Unsafe encapsulation:
         *
         * Raw pointer writes into MemBlock-backed storage should be kept behind
         * a narrow internal boundary. The public DeviceTree event surface
         * should expose safe state/query operations.
         */
        arceos_ex_should_encapsulate_device_tree_unflatten_unsafe();
    }
}

type ArceosExMmCoreInitCodingMust {
    invariant {
        /*
         * PageAllocator preset/setup split:
         *
         * PageAllocator.preset() must only establish zonelist topology and
         * page allocator CPU hotplug step registration. It must not release
         * MemBlock pages to buddy/free page sets.
         */
        arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks();

        /*
         * MemBlock handoff:
         *
         * PageAllocator.setup() must perform the memblock_free_all() handoff:
         * account managed pages, populate buddy free page sets, and advance
         * MemBlock through Disable to Offline.
         */
        arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy();

        /*
         * MemBlock remains Offline:
         *
         * mm_core_init() must not destroy or discard MemBlock metadata. That
         * later discard belongs to page_alloc_init_late()/memblock_discard().
         */
        arceos_ex_must_memblock_disable_reaches_offline_not_destroyed();

        /*
         * SWIOTLB ordering:
         *
         * Swiotlb.setup() must complete before MemBlock.Disable so any early
         * default pool or resolved no-pool fact is established while MemBlock
         * allocation/reservation is still available.
         */
        arceos_ex_must_swiotlb_setup_before_memblock_disable();

        /*
         * Static branch policy:
         *
         * MemoryDebugHardening.setup() must consume the existing StaticBranch
         * registry and early parameter facts; it must not initialize a private
         * static-key mechanism.
         */
        arceos_ex_must_memory_debug_hardening_use_static_branch_registry();

        /*
         * SLUB bootstrap:
         *
         * SlubAllocator.setup() must bootstrap kmem_cache/kmem_cache_node
         * before KmallocCaches is considered Ready.
         */
        arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready();

        /*
         * Page table lock cache:
         *
         * PageTableLockCache.setup() must create the split page-table lock
         * cache corresponding to Linux "page->ptl".
         */
        arceos_ex_must_page_table_lock_cache_named_page_ptl();

        /*
         * Vmalloc boundary:
         *
         * VmallocAllocator.setup() must manage vmalloc/vmap virtual address
         * resources and metadata. It must not be used as the page-table
         * mapping implementation; page table range preparation belongs to
         * PageTableCaches/SwapperVm facts.
         */
        arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_not_page_tables();

        /*
         * Vmap subobjects:
         *
         * VmallocAllocator.setup() must establish VmapAreaCache,
         * VmapAddressSpace, VmapNodeSet, VmapBlockQueues, and VfreeDeferredSet
         * before VmallocAllocator.Ready is emitted.
         */
        arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects();

        /*
         * mm_struct only:
         *
         * MmStructCache.setup() must only establish the "mm_struct" cache.
         * vm_area_struct, vma lock cache, and mmap_init() remain later
         * proc_caches_init()/process-memory work.
         */
        arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache();
    }
}

type ArceosExMmCoreInitCodingShould {
    invariant {
        /*
         * Checkpoints:
         *
         * mm_core_init() implementation should keep compile-time, report and
         * trimmed-path checkpoints observable in trace/logging without turning
         * them into lifecycle states.
         */
        arceos_ex_should_keep_mm_core_init_checkpoints_observable();
    }
}
