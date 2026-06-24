/*
 * Generic allocator model.
 *
 * PageAllocatorType is the reusable boundary for Linux-like buddy page
 * allocation. Its lifecycle is still provided by the concrete PageAllocator
 * instance in mm_core_init(); the runtime actions here are only valid after
 * that instance reaches Ready.
 */

type PageOrder {
}

type GfpFlags {
}

type Pfn {
}

type PhysPageAddr {
}

type LinearMappedPageAddr {
}

type KmallocSize {
}

type KmallocAllocRef {
}

/*
 * SlubCache is the formal type name for a single Linux struct kmem_cache
 * instance. The model intentionally does not define SlubCacheType.
 */
type SlubCache {
}

type AllocLayout {
}

type HeapAllocRef {
}

type PageMetadata {
}

type BuddyListLink {
}

type BuddyFreeListHead {
}

type BuddyFreeArea {
    order: PageOrder;
    free_list_head: BuddyFreeListHead;
}

type BuddyFreePageSetType {
    lifecycle {
        /*
         * Setup corresponds to the free_area population done by Linux
         * memblock_free_all(): each released free range is split into
         * pageblock-aligned buddy blocks and linked through the block head's
         * PageMetadata.
         */
        Event::Setup {
            state_effect: StateEffect::Always;
            depends_on {
                MemBlock.state == State::Online;
                Zones.state == State::Ready;
                page_metadata_map_ready(PageMetadataMap, Zones);
                page_metadata_map_indexed_by_pfn(PageMetadataMap);
            }
            ensures {
                buddy_free_page_sets_ready(self);
                buddy_free_page_sets_bound_to_allocator(self, owner);
                buddy_free_page_sets_indexed_by_zone_and_order(self, Zones);
                buddy_free_page_sets_use_single_migratetype(self);
                buddy_free_page_sets_use_intrusive_lists(self);
                buddy_free_page_sets_free_area_heads_ready(self);
                buddy_free_page_sets_use_page_metadata_as_nodes(self, PageMetadataMap);
                buddy_free_page_sets_nodes_are_block_head_metadata(self, PageMetadataMap);
                buddy_free_page_sets_no_external_node_storage(self);
                buddy_free_page_sets_populated_from_memblock(self, MemBlock, Zones);
                buddy_free_page_sets_exclude_reserved_ranges(self, MemBlock);
                buddy_free_page_sets_split_free_ranges_to_aligned_blocks(self);
            }
        }
    }
}

type PageRef {
    processes {
        /*
         * PageToPfn corresponds to Linux page_to_pfn(page).
         */
        Action::PageToPfn -> Pfn {
            state_effect: StateEffect::None;
            depends_on {
                page_ref_ready(self);
                page_ref_targets_page_metadata(self);
                page_ref_has_valid_pfn(self);
            }
            ensures {
                page_ref_page_to_pfn_called(self);
            }
        }

        /*
         * PageToPhys corresponds to page_to_phys(page), i.e.
         * pfn_to_phys(page_to_pfn(page)).
         */
        Action::PageToPhys -> PhysPageAddr {
            state_effect: StateEffect::None;
            depends_on {
                page_ref_ready(self);
                page_ref_targets_page_metadata(self);
                page_ref_has_valid_pfn(self);
            }
            ensures {
                page_ref_page_to_phys_called(self);
            }
        }

        /*
         * PageToVirt/PageAddress correspond to Linux page_to_virt() and the
         * lowmem/direct-map form of page_address().
         */
        Action::PageToVirt -> LinearMappedPageAddr {
            state_effect: StateEffect::None;
            depends_on {
                page_ref_ready(self);
                page_ref_linear_mapped(self);
            }
            ensures {
                page_ref_page_to_virt_called(self);
            }
        }

        Action::PageAddress -> LinearMappedPageAddr {
            state_effect: StateEffect::None;
            depends_on {
                page_ref_ready(self);
                page_ref_linear_mapped(self);
            }
            ensures {
                page_ref_page_address_called(self);
            }
        }
    }
}

predicate page_metadata_map_ready<T, Z>(metadata_map: T, zones: Z) -> bool;
predicate page_metadata_map_covers_managed_pfns<T, Z>(metadata_map: T, zones: Z) -> bool;
predicate page_metadata_map_uses_mem_map_or_vmemmap<T>(metadata_map: T) -> bool;
predicate page_metadata_map_storage_allocated_from_memblock<T, M>(metadata_map: T, memblock: M) -> bool;
predicate page_metadata_map_indexed_by_pfn<T>(metadata_map: T) -> bool;
predicate page_metadata_map_item_ready<T>(page_metadata: T) -> bool;
predicate page_metadata_map_item_in_map<T, M>(page_metadata: T, metadata_map: M) -> bool;
predicate page_metadata_map_item_pfn_bound<T, P>(page_metadata: T, pfn: P) -> bool;
predicate page_metadata_buddy_fields_ready<T>(page_metadata: T) -> bool;
predicate page_metadata_embeds_buddy_list_link<T, L>(page_metadata: T, link: L) -> bool;
predicate page_metadata_buddy_free_node_ready<T>(page_metadata: T) -> bool;
predicate page_metadata_buddy_order_recorded<T, O>(page_metadata: T, order: O) -> bool;
predicate page_metadata_buddy_allocated_or_free_state_ready<T>(page_metadata: T) -> bool;
predicate page_metadata_buddy_link_not_external_storage<T>(page_metadata: T) -> bool;
predicate buddy_free_area_ready<T>(area: T) -> bool;
predicate buddy_free_area_order_bound<T, O>(area: T, order: O) -> bool;
predicate buddy_free_area_zone_bound<T, Z>(area: T, zone: Z) -> bool;
predicate buddy_free_area_list_head_ready<T, H>(area: T, head: H) -> bool;
predicate buddy_free_area_list_head_not_page_metadata_node<T, H>(area: T, head: H) -> bool;
predicate buddy_free_area_nr_free_accounted<T>(area: T) -> bool;
predicate buddy_free_page_sets_ready<T>(sets: T) -> bool;
predicate buddy_free_page_sets_bound_to_allocator<T, A>(sets: T, allocator: A) -> bool;
predicate buddy_free_page_sets_indexed_by_zone_and_order<T, Z>(sets: T, zones: Z) -> bool;
predicate buddy_free_page_sets_use_single_migratetype<T>(sets: T) -> bool;
predicate buddy_free_page_sets_use_intrusive_lists<T>(sets: T) -> bool;
predicate buddy_free_page_sets_free_area_heads_ready<T>(sets: T) -> bool;
predicate buddy_free_page_sets_use_page_metadata_as_nodes<T, M>(sets: T, metadata_map: M) -> bool;
predicate buddy_free_page_sets_nodes_are_block_head_metadata<T, M>(sets: T, metadata_map: M) -> bool;
predicate buddy_free_page_sets_no_external_node_storage<T>(sets: T) -> bool;
predicate buddy_free_page_sets_populated_from_memblock<T, M, Z>(sets: T, memblock: M, zones: Z) -> bool;
predicate buddy_free_page_sets_exclude_reserved_ranges<T, M>(sets: T, memblock: M) -> bool;
predicate buddy_free_page_sets_split_free_ranges_to_aligned_blocks<T>(sets: T) -> bool;
predicate page_allocator_page_metadata_map_bound<T, M>(allocator: T, metadata_map: M) -> bool;
predicate page_allocator_zonelist_update_seq_irqsave_guard_ready<T>(allocator: T) -> bool;
predicate page_allocator_zonelist_update_seq_irqsave_guard_spec_required<T>(allocator: T) -> bool;
predicate page_allocator_zonelist_printk_deferred_section_ready<T>(allocator: T) -> bool;
predicate page_allocator_zonelist_printk_deferred_section_spec_required<T>(allocator: T) -> bool;
predicate page_allocator_boot_pagesets_initialized_for_possible_cpus<T, P>(
    allocator: T,
    per_cpu_storage: P
) -> bool;
predicate page_allocator_runtime_zone_locking_contract_deferred<T>(allocator: T) -> bool;
predicate page_allocator_runtime_pcp_locking_contract_deferred<T>(allocator: T) -> bool;
predicate page_allocator_buddy_free_page_sets_bound<T, S>(allocator: T, sets: S) -> bool;
predicate page_allocator_free_pages_account_matches_buddy<T, S>(allocator: T, sets: S) -> bool;
predicate page_allocator_alloc_pages_api_ready<T>(allocator: T) -> bool;
predicate page_allocator_free_pages_api_ready<T>(allocator: T) -> bool;
predicate page_allocator_page_ref_conversion_api_ready<T>(allocator: T) -> bool;
predicate page_allocator_can_allocate_order<T, O>(allocator: T, order: O) -> bool;
predicate page_allocator_gfp_allowed<T, G>(allocator: T, gfp: G) -> bool;
predicate page_allocator_alloc_pages_called<T, O, G>(allocator: T, order: O, gfp: G) -> bool;
predicate page_allocator_alloc_pages_returns<T, R, O, G>(
    allocator: T,
    page_ref: R,
    order: O,
    gfp: G
) -> bool;
predicate page_allocator_free_pages_called<T, R, O>(allocator: T, page_ref: R, order: O) -> bool;
predicate page_allocator_free_pages_committed<T, R, O>(allocator: T, page_ref: R, order: O) -> bool;
predicate page_allocator_pfn_to_page_called<T, P>(allocator: T, pfn: P) -> bool;
predicate page_allocator_phys_to_page_called<T, A>(allocator: T, phys_addr: A) -> bool;
predicate page_allocator_virt_to_page_called<T, A>(allocator: T, linear_addr: A) -> bool;
predicate page_ref_ready<T>(page_ref: T) -> bool;
predicate page_ref_targets_buddy_pages<T, A>(page_ref: T, allocator: A) -> bool;
predicate page_ref_targets_page_metadata<T>(page_ref: T) -> bool;
predicate page_ref_metadata_in_map<T, M>(page_ref: T, metadata_map: M) -> bool;
predicate page_ref_order_bound<T, O>(page_ref: T, order: O) -> bool;
predicate page_ref_has_valid_pfn<T>(page_ref: T) -> bool;
predicate page_ref_pfn_bound<T, P>(page_ref: T, pfn: P) -> bool;
predicate page_ref_phys_addr_bound<T, A>(page_ref: T, phys_addr: A) -> bool;
predicate page_ref_linear_addr_bound<T, A>(page_ref: T, linear_addr: A) -> bool;
predicate page_ref_linear_mapped<T>(page_ref: T) -> bool;
predicate page_ref_exclusively_owned_by_caller<T>(page_ref: T) -> bool;
predicate page_ref_released_to_allocator<T, A>(page_ref: T, allocator: A) -> bool;
predicate page_ref_page_to_pfn_called<T>(page_ref: T) -> bool;
predicate page_ref_page_to_phys_called<T>(page_ref: T) -> bool;
predicate page_ref_page_to_virt_called<T>(page_ref: T) -> bool;
predicate page_ref_page_address_called<T>(page_ref: T) -> bool;
predicate pfn_valid_for_page_allocator<P, A>(pfn: P, allocator: A) -> bool;
predicate pfn_has_page_metadata<P, M>(pfn: P, metadata_map: M) -> bool;
predicate phys_page_addr_page_aligned<T>(phys_addr: T) -> bool;
predicate phys_page_addr_pfn_bound<T, P>(phys_addr: T, pfn: P) -> bool;
predicate linear_page_addr_in_linear_map<T>(linear_addr: T) -> bool;
predicate linear_page_addr_pfn_bound<T, P>(linear_addr: T, pfn: P) -> bool;
predicate slub_cache_type_name_is_slub_cache<T>(subsystem: T) -> bool;
predicate slub_cache_registry_owned_by_subsystem<T, S>(registry: T, subsystem: S) -> bool;
predicate slub_cache_registry_owns_all_slub_cache_instances<T>(registry: T) -> bool;
predicate boot_kmem_cache_node_is_slub_cache_instance<T>(subsystem: T) -> bool;
predicate boot_kmem_cache_is_slub_cache_instance<T, R>(subsystem: T, registry: R) -> bool;
predicate kmem_cache_and_node_registered_as_slub_cache_instances<T>(registry: T) -> bool;
predicate kmalloc_caches_refer_to_registered_slub_caches<T, R>(caches: T, registry: R) -> bool;
predicate page_table_lock_cache_registered_in_slub_registry<T, R>(cache: T, registry: R) -> bool;
predicate vmap_area_cache_registered_in_slub_registry<T, R>(cache: T, registry: R) -> bool;
predicate mm_struct_cache_is_slub_cache_instance<T>(cache: T) -> bool;
predicate radix_tree_node_cache_registered_in_slub_registry<T, R>(tree: T, registry: R) -> bool;
predicate maple_tree_node_cache_registered_in_slub_registry<T, R>(tree: T, registry: R) -> bool;
predicate slub_subsystem_kmalloc_api_ready<T>(allocator: T) -> bool;
predicate slub_subsystem_kzalloc_api_ready<T>(allocator: T) -> bool;
predicate slub_subsystem_kfree_api_ready<T>(allocator: T) -> bool;
predicate slub_subsystem_uses_page_allocator<T, P>(allocator: T, page_allocator: P) -> bool;
predicate slub_subsystem_runtime_locking_contract_deferred<T>(allocator: T) -> bool;
predicate slub_subsystem_slab_mutex_not_required_before_full<T>(allocator: T) -> bool;
predicate slub_subsystem_kmalloc_called<T, S, G>(allocator: T, size: S, gfp: G) -> bool;
predicate slub_subsystem_kzalloc_called<T, S, G>(allocator: T, size: S, gfp: G) -> bool;
predicate slub_subsystem_kfree_called<T, R>(allocator: T, alloc_ref: R) -> bool;
predicate kmalloc_caches_ready_for_size<T, S>(caches: T, size: S) -> bool;
predicate kmalloc_caches_default_size_classes_ready<T>(caches: T) -> bool;
predicate kmalloc_cache_size_class_selected<T, S>(caches: T, size: S) -> bool;
predicate kmalloc_slab_backed_by_page_ref<T, P>(alloc_ref: T, page_ref: P) -> bool;
predicate kmalloc_alloc_ref_ready<T>(alloc_ref: T) -> bool;
predicate kmalloc_alloc_ref_size_bound<T, S>(alloc_ref: T, size: S) -> bool;
predicate kmalloc_alloc_ref_linear_mapped<T>(alloc_ref: T) -> bool;
predicate kmalloc_alloc_ref_zeroed<T>(alloc_ref: T) -> bool;
predicate kmalloc_alloc_ref_exclusively_owned_by_caller<T>(alloc_ref: T) -> bool;
predicate kmalloc_alloc_ref_released_to_slub<T, A>(alloc_ref: T, allocator: A) -> bool;
predicate kernel_global_allocator_ready<T, S>(allocator: T, slub_subsystem: S) -> bool;
predicate kernel_global_allocator_uses_slub_subsystem<T, S>(allocator: T, slub_subsystem: S) -> bool;
predicate kernel_global_allocator_alloc_api_ready<T>(allocator: T) -> bool;
predicate kernel_global_allocator_alloc_zeroed_api_ready<T>(allocator: T) -> bool;
predicate kernel_global_allocator_dealloc_api_ready<T>(allocator: T) -> bool;
predicate kernel_global_allocator_runtime_sync_inherits_slub_contract<T, S>(
    allocator: T,
    slub_subsystem: S
) -> bool;
predicate kernel_global_allocator_layout_supported<T, L>(allocator: T, layout: L) -> bool;
predicate alloc_layout_size_nonzero<T>(layout: T) -> bool;
predicate alloc_layout_size_bound<T, S>(layout: T, size: S) -> bool;
predicate alloc_layout_align_bound<T, A>(layout: T, align: A) -> bool;
predicate heap_alloc_ref_ready<T>(alloc_ref: T) -> bool;
predicate heap_alloc_ref_layout_bound<T, L>(alloc_ref: T, layout: L) -> bool;
predicate heap_alloc_ref_backed_by_kmalloc<T, K>(alloc_ref: T, kmalloc_ref: K) -> bool;
predicate heap_alloc_ref_linear_mapped<T>(alloc_ref: T) -> bool;
predicate heap_alloc_ref_zeroed<T>(alloc_ref: T) -> bool;
predicate heap_alloc_ref_exclusively_owned_by_caller<T>(alloc_ref: T) -> bool;
predicate heap_alloc_ref_released_to_global_allocator<T, A>(alloc_ref: T, allocator: A) -> bool;
predicate kernel_global_allocator_alloc_called<T, L>(allocator: T, layout: L) -> bool;
predicate kernel_global_allocator_alloc_zeroed_called<T, L>(allocator: T, layout: L) -> bool;
predicate kernel_global_allocator_dealloc_called<T, R, L>(allocator: T, alloc_ref: R, layout: L) -> bool;
predicate dynamic_container_runtime_ready<T, A>(runtime: T, allocator: A) -> bool;
predicate dynamic_container_runtime_uses_global_allocator<T, A>(runtime: T, allocator: A) -> bool;
predicate dynamic_container_vec_api_ready<T>(runtime: T) -> bool;
predicate dynamic_container_list_api_ready<T>(runtime: T) -> bool;
predicate dynamic_container_set_api_ready<T>(runtime: T) -> bool;
predicate dynamic_container_storage_backed_by_heap_alloc_ref<T, R>(runtime: T, alloc_ref: R) -> bool;
predicate dynamic_container_runtime_sync_inherits_global_allocator_contract<T, A>(
    runtime: T,
    allocator: A
) -> bool;

type PageAllocatorType: MemoryObject {
    processes {
        /*
         * AllocPages corresponds to Linux alloc_pages(gfp, order), expressed
         * with the model's preferred argument order (order, gfp). The returned
         * PageRef is a caller-owned reference to 2^order contiguous buddy pages.
         */
        Action::AllocPages(order: PageOrder, gfp: GfpFlags) -> PageRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_alloc_pages_api_ready(self);
                page_allocator_page_metadata_map_bound(self, PageMetadataMap);
                buddy_free_page_sets_populated(self, Zones);
                page_allocator_can_allocate_order(self, order);
                page_allocator_gfp_allowed(self, gfp);
            }
            ensures {
                page_allocator_alloc_pages_called(self, order, gfp);
            }
            result {
                Available: Success(page_ref_returned);
                NoMemory: Failed(no_buddy_pages_available);
            }
            deferred {
                "当前 AllocPages 规格只展开成功返回 PageRef 的使用路径；GFP reclaim/compaction/oom 和 NULL/ERR 失败传播后续随完整内存压力模型展开。";
            }
        }

        /*
         * FreePages corresponds to Linux free_pages()/__free_pages(). The
         * order must match the order used when the PageRef was allocated.
         */
        Action::FreePages(page_ref: PageRef, order: PageOrder) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_free_pages_api_ready(self);
                page_ref_ready(page_ref);
                page_ref_targets_buddy_pages(page_ref, self);
                page_ref_metadata_in_map(page_ref, PageMetadataMap);
                page_ref_order_bound(page_ref, order);
                page_ref_exclusively_owned_by_caller(page_ref);
            }
            ensures {
                page_allocator_free_pages_called(self, page_ref, order);
                page_allocator_free_pages_committed(self, page_ref, order);
                page_ref_released_to_allocator(page_ref, self);
            }
        }

        /*
         * PfnToPage corresponds to Linux pfn_to_page(pfn). The returned
         * PageRef targets the page metadata entry associated with the PFN.
         */
        Action::PfnToPage(pfn: Pfn) -> PageRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_page_ref_conversion_api_ready(self);
                page_allocator_page_metadata_map_bound(self, PageMetadataMap);
                pfn_valid_for_page_allocator(pfn, self);
                pfn_has_page_metadata(pfn, PageMetadataMap);
            }
            ensures {
                page_allocator_pfn_to_page_called(self, pfn);
            }
        }

        /*
         * PhysToPage corresponds to Linux phys_to_page(paddr).
         */
        Action::PhysToPage(phys_addr: PhysPageAddr) -> PageRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_page_ref_conversion_api_ready(self);
                phys_page_addr_page_aligned(phys_addr);
                page_allocator_page_metadata_map_bound(self, PageMetadataMap);
            }
            ensures {
                page_allocator_phys_to_page_called(self, phys_addr);
            }
        }

        /*
         * VirtToPage corresponds to Linux virt_to_page(vaddr) for addresses
         * that are known to be in the established linear mapping.
         */
        Action::VirtToPage(linear_addr: LinearMappedPageAddr) -> PageRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_page_ref_conversion_api_ready(self);
                linear_page_addr_in_linear_map(linear_addr);
                page_allocator_page_metadata_map_bound(self, PageMetadataMap);
            }
            ensures {
                page_allocator_virt_to_page_called(self, linear_addr);
            }
        }
    }
}

type KernelGlobalAllocatorType: MemoryObject {
    processes {
        /*
         * Alloc corresponds to the Rust GlobalAlloc::alloc(Layout) boundary
         * used by Vec/Box and other ordinary heap-backed containers.
         */
        Action::Alloc(layout: AllocLayout) -> HeapAllocRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                SlubSubsystem.state == State::Ready;
                kernel_global_allocator_alloc_api_ready(self);
                kernel_global_allocator_uses_slub_subsystem(self, SlubSubsystem);
                kernel_global_allocator_layout_supported(self, layout);
                alloc_layout_size_nonzero(layout);
            }
            ensures {
                kernel_global_allocator_alloc_called(self, layout);
            }
            result {
                Available: Success(heap_alloc_ref_returned);
                NoMemory: Failed(no_heap_storage_available);
            }
            deferred {
                "当前 GlobalAlloc 规格只展开 SLUB-backed 成功路径；large allocation、OOM policy、realloc 和特殊 alignment fallback 后续随完整 heap adapter 展开。";
            }
        }

        /*
         * AllocZeroed corresponds to GlobalAlloc::alloc_zeroed(Layout). It is
         * the heap-facing equivalent of kzalloc for ordinary Rust containers.
         */
        Action::AllocZeroed(layout: AllocLayout) -> HeapAllocRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                SlubSubsystem.state == State::Ready;
                kernel_global_allocator_alloc_zeroed_api_ready(self);
                kernel_global_allocator_uses_slub_subsystem(self, SlubSubsystem);
                kernel_global_allocator_layout_supported(self, layout);
                alloc_layout_size_nonzero(layout);
            }
            ensures {
                kernel_global_allocator_alloc_zeroed_called(self, layout);
            }
            result {
                Available: Success(heap_alloc_ref_returned);
                NoMemory: Failed(no_heap_storage_available);
            }
        }

        /*
         * Dealloc corresponds to GlobalAlloc::dealloc(ptr, Layout). The
         * implementation must recover the owning kmalloc slab/cache from the
         * heap allocation reference before returning storage to SLUB.
         */
        Action::Dealloc(alloc_ref: HeapAllocRef, layout: AllocLayout) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                SlubSubsystem.state == State::Ready;
                kernel_global_allocator_dealloc_api_ready(self);
                heap_alloc_ref_ready(alloc_ref);
                heap_alloc_ref_layout_bound(alloc_ref, layout);
                heap_alloc_ref_exclusively_owned_by_caller(alloc_ref);
            }
            ensures {
                kernel_global_allocator_dealloc_called(self, alloc_ref, layout);
                heap_alloc_ref_released_to_global_allocator(alloc_ref, self);
            }
        }
    }
}
