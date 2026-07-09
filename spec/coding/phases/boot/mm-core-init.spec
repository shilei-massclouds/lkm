/*
 * MmCoreInitPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in mm-core-init.md.
 */

predicate arceos_ex_must_memory_topology_project_existing_zones_only() -> bool;
predicate arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks() -> bool;
predicate arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy() -> bool;
predicate arceos_ex_must_page_allocator_buddy_lists_live_inside_page_allocator() -> bool;
predicate arceos_ex_must_page_allocator_buddy_use_zone_order_free_area() -> bool;
predicate arceos_ex_must_page_allocator_buddy_first_round_single_migratetype() -> bool;
predicate arceos_ex_must_page_allocator_buddy_split_memblock_free_ranges() -> bool;
predicate arceos_ex_must_page_allocator_buddy_use_page_metadata_nodes() -> bool;
predicate arceos_ex_must_page_allocator_buddy_use_intrusive_list() -> bool;
predicate arceos_ex_must_page_allocator_buddy_free_area_store_only_head_and_count() -> bool;
predicate arceos_ex_must_page_allocator_buddy_node_is_block_head_metadata() -> bool;
predicate arceos_ex_must_page_allocator_buddy_avoid_heap_storage() -> bool;
predicate arceos_ex_must_page_allocator_expose_linux_like_alloc_pages_api() -> bool;
predicate arceos_ex_must_page_allocator_alloc_pages_return_owned_linear_mapped_pageref() -> bool;
predicate arceos_ex_must_page_allocator_free_pages_match_alloc_order() -> bool;
predicate arceos_ex_must_page_allocator_smoke_cover_page_alloc_free_read_write() -> bool;
predicate arceos_ex_must_mm_core_init_model_sync_even_when_boot_lowering_elides_code() -> bool;
predicate arceos_ex_must_page_allocator_preset_record_zonelist_irqsave_protocol() -> bool;
predicate arceos_ex_must_page_allocator_runtime_locking_remain_explicit_deferred() -> bool;
predicate arceos_ex_must_memblock_disable_reaches_offline_not_destroyed() -> bool;
predicate arceos_ex_must_swiotlb_setup_before_memblock_disable() -> bool;
predicate arceos_ex_must_memory_debug_hardening_use_static_branch_registry() -> bool;
predicate arceos_ex_must_slub_subsystem_be_single_facade_not_cache_instance() -> bool;
predicate arceos_ex_must_slub_cache_type_name_be_slub_cache() -> bool;
predicate arceos_ex_must_slub_cache_registry_own_all_cache_instances() -> bool;
predicate arceos_ex_must_kmalloc_caches_reference_registered_slub_caches() -> bool;
predicate arceos_ex_must_kmem_cache_sites_register_named_slub_caches_or_defer() -> bool;
predicate arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready() -> bool;
predicate arceos_ex_must_slub_expose_kmalloc_kzalloc_kfree_api() -> bool;
predicate arceos_ex_must_slub_kmalloc_use_page_allocator_backing_pages() -> bool;
predicate arceos_ex_must_slub_kmalloc_use_fixed_size_classes() -> bool;
predicate arceos_ex_must_slub_kmalloc_use_slab_slot_freelist() -> bool;
predicate arceos_ex_must_slub_kzalloc_zero_returned_object() -> bool;
predicate arceos_ex_must_slub_kfree_recycle_object_to_cache() -> bool;
predicate arceos_ex_must_slub_first_round_defer_complex_linux_paths() -> bool;
predicate arceos_ex_must_slub_bootstrap_record_slab_mutex_boundary() -> bool;
predicate arceos_ex_must_slub_runtime_locking_remain_explicit_deferred() -> bool;
predicate arceos_ex_must_slub_smoke_cover_kmalloc_kzalloc_kfree() -> bool;
predicate arceos_ex_must_global_allocator_setup_after_slub_ready() -> bool;
predicate arceos_ex_must_global_allocator_implement_core_alloc_globalalloc() -> bool;
predicate arceos_ex_must_global_allocator_alloc_use_kmalloc() -> bool;
predicate arceos_ex_must_global_allocator_alloc_zeroed_use_kzalloc_or_zeroing() -> bool;
predicate arceos_ex_must_global_allocator_dealloc_recover_kmalloc_object_from_ptr() -> bool;
predicate arceos_ex_must_global_allocator_support_documented_layout_subset() -> bool;
predicate arceos_ex_must_dynamic_containers_require_global_allocator_ready() -> bool;
predicate arceos_ex_must_dynamic_container_smoke_cover_vec_growth_drop() -> bool;
predicate arceos_ex_must_dynamic_container_pressure_smoke_cover_layout_boundary() -> bool;
predicate arceos_ex_must_page_table_lock_cache_named_page_ptl() -> bool;
predicate arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_and_execute_mappings() -> bool;
predicate arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects() -> bool;
predicate arceos_ex_must_vmalloc_map_page_range_record_each_mapping_action() -> bool;
predicate arceos_ex_must_vmalloc_support_preallocated_windows_and_reject_duplicate_mapping() -> bool;
predicate arceos_ex_must_vmalloc_allocate_l0_windows_on_demand() -> bool;
predicate arceos_ex_must_vmalloc_unmap_before_free_vmap_area() -> bool;
predicate arceos_ex_must_vmalloc_sync_and_locking_contracts_remain_visible() -> bool;
predicate arceos_ex_must_ioremap_keep_physical_resource_and_mmio_policy_external_to_vmalloc() -> bool;
predicate arceos_ex_must_ioremap_iounmap_request_vmalloc_teardown_only() -> bool;
predicate arceos_ex_must_ioremap_model_mmio_attribute_policy_explicitly() -> bool;
predicate arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache() -> bool;
predicate arceos_ex_should_keep_mm_core_init_checkpoints_observable() -> bool;

type ArceosExMmCoreInitCodingMust {
    invariant {
        /* Memory topology view. */
        arceos_ex_must_memory_topology_project_existing_zones_only();

        /* PageAllocator preset/setup split. */
        arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks();

        /* MemBlock handoff. */
        arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy();

        /* Minimal buddy free lists. */
        arceos_ex_must_page_allocator_buddy_lists_live_inside_page_allocator();
        arceos_ex_must_page_allocator_buddy_use_zone_order_free_area();
        arceos_ex_must_page_allocator_buddy_first_round_single_migratetype();

        /* MemBlock range splitting. */
        arceos_ex_must_page_allocator_buddy_split_memblock_free_ranges();

        /* Metadata-backed free nodes. */
        arceos_ex_must_page_allocator_buddy_use_page_metadata_nodes();
        arceos_ex_must_page_allocator_buddy_use_intrusive_list();
        arceos_ex_must_page_allocator_buddy_free_area_store_only_head_and_count();
        arceos_ex_must_page_allocator_buddy_node_is_block_head_metadata();
        arceos_ex_must_page_allocator_buddy_avoid_heap_storage();

        /* Linux-like buddy API. */
        arceos_ex_must_page_allocator_expose_linux_like_alloc_pages_api();

        /* PageRef contract. */
        arceos_ex_must_page_allocator_alloc_pages_return_owned_linear_mapped_pageref();

        /* Free order contract. */
        arceos_ex_must_page_allocator_free_pages_match_alloc_order();

        /* mm_core_init synchronization surface. */
        arceos_ex_must_mm_core_init_model_sync_even_when_boot_lowering_elides_code();

        /* Zonelist update protocol. */
        arceos_ex_must_page_allocator_preset_record_zonelist_irqsave_protocol();

        /* Page allocator runtime locking. */
        arceos_ex_must_page_allocator_runtime_locking_remain_explicit_deferred();

        /* Page allocator smoke. */
        arceos_ex_must_page_allocator_smoke_cover_page_alloc_free_read_write();

        /* MemBlock remains Offline. */
        arceos_ex_must_memblock_disable_reaches_offline_not_destroyed();

        /* SWIOTLB ordering. */
        arceos_ex_must_swiotlb_setup_before_memblock_disable();

        /* Static branch policy. */
        arceos_ex_must_memory_debug_hardening_use_static_branch_registry();

        /* SLUB object hierarchy. */
        arceos_ex_must_slub_subsystem_be_single_facade_not_cache_instance();
        arceos_ex_must_slub_cache_type_name_be_slub_cache();
        arceos_ex_must_slub_cache_registry_own_all_cache_instances();
        arceos_ex_must_kmalloc_caches_reference_registered_slub_caches();
        arceos_ex_must_kmem_cache_sites_register_named_slub_caches_or_defer();

        /* SLUB bootstrap. */
        arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready();

        /* Linux-like kmalloc API. */
        arceos_ex_must_slub_expose_kmalloc_kzalloc_kfree_api();
        arceos_ex_must_slub_kmalloc_use_page_allocator_backing_pages();
        arceos_ex_must_slub_kmalloc_use_fixed_size_classes();
        arceos_ex_must_slub_kmalloc_use_slab_slot_freelist();
        arceos_ex_must_slub_kzalloc_zero_returned_object();
        arceos_ex_must_slub_kfree_recycle_object_to_cache();
        arceos_ex_must_slub_first_round_defer_complex_linux_paths();

        /* SLUB bootstrap synchronization. */
        arceos_ex_must_slub_bootstrap_record_slab_mutex_boundary();
        arceos_ex_must_slub_runtime_locking_remain_explicit_deferred();

        /* SLUB/kmalloc smoke. */
        arceos_ex_must_slub_smoke_cover_kmalloc_kzalloc_kfree();

        /* Global allocator. */
        arceos_ex_must_global_allocator_setup_after_slub_ready();
        arceos_ex_must_global_allocator_implement_core_alloc_globalalloc();
        arceos_ex_must_global_allocator_alloc_use_kmalloc();
        arceos_ex_must_global_allocator_alloc_zeroed_use_kzalloc_or_zeroing();
        arceos_ex_must_global_allocator_dealloc_recover_kmalloc_object_from_ptr();
        arceos_ex_must_global_allocator_support_documented_layout_subset();
        arceos_ex_must_dynamic_containers_require_global_allocator_ready();

        /* Dynamic container smoke. */
        arceos_ex_must_dynamic_container_smoke_cover_vec_growth_drop();
        arceos_ex_must_dynamic_container_pressure_smoke_cover_layout_boundary();

        /* Page table lock cache. */
        arceos_ex_must_page_table_lock_cache_named_page_ptl();

        /* Vmalloc boundary. */
        arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_and_execute_mappings();

        /* Vmap subobjects. */
        arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects();

        /* Mapping record boundary. */
        arceos_ex_must_vmalloc_map_page_range_record_each_mapping_action();

        /* Runtime mapping window pool. */
        arceos_ex_must_vmalloc_support_preallocated_windows_and_reject_duplicate_mapping();
        arceos_ex_must_vmalloc_allocate_l0_windows_on_demand();

        /* Vmalloc synchronization contracts. */
        arceos_ex_must_vmalloc_sync_and_locking_contracts_remain_visible();

        /* Unmap/free boundary. */
        arceos_ex_must_vmalloc_unmap_before_free_vmap_area();

        /* Ioremap/vmalloc split. */
        arceos_ex_must_ioremap_keep_physical_resource_and_mmio_policy_external_to_vmalloc();

        /* Iounmap boundary. */
        arceos_ex_must_ioremap_iounmap_request_vmalloc_teardown_only();

        /* MMIO attributes. */
        arceos_ex_must_ioremap_model_mmio_attribute_policy_explicitly();

        /* mm_struct only. */
        arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache();
    }
}

type ArceosExMmCoreInitCodingShould {
    invariant {
        /* Checkpoints. */
        arceos_ex_should_keep_mm_core_init_checkpoints_observable();
    }
}
