/*
 * Generic vmalloc/vmap model.
 *
 * Linux reference shape:
 * - __get_vm_area_caller() reserves a vm_struct/vmap_area from the vmalloc
 *   virtual address space.
 * - vmap/ioremap callers then ask the same subsystem to install or tear down
 *   page-table mappings for caller-supplied pages, PFNs, or physical ranges.
 * - The vmalloc layer owns virtual address area management and mapping
 *   execution. Callers still own the physical resource source and the mapping
 *   attribute policy.
 * - Successful map/unmap actions must publish architecture-visible ordering:
 *   page-table installation/removal is guarded, followed by the required
 *   kernel mapping sync/cache/TLB boundary. Full Linux lazy purge, RCU, and
 *   remote shootdown detail remains a later refinement.
 */

type VmapAreaRef {
}

type VmapMappingRef {
}

type VmapAreaFlags {
}

type PageProtectionRef {
}

type VmapPageProtectionKind {
}

predicate vmalloc_allocator_vmap_area_api_ready<T>(allocator: T) -> bool;
predicate vmalloc_allocator_page_range_mapping_api_ready<T>(allocator: T) -> bool;
predicate vmalloc_allocator_manages_vmap_address_space<T, V>(allocator: T, vmap_space: V) -> bool;
predicate vmalloc_allocator_maintains_vm_struct_metadata<T>(allocator: T) -> bool;
predicate vmalloc_allocator_maintains_vmap_area_metadata<T>(allocator: T) -> bool;
predicate vmalloc_allocator_executes_page_table_mappings<T, S>(allocator: T, swapper_vm: S) -> bool;
predicate vmalloc_allocator_runtime_page_table_mapping_ready<T, P>(allocator: T, page_table_caches: P) -> bool;
predicate vmalloc_allocator_mapping_policy_external<T>(allocator: T) -> bool;
predicate vmalloc_allocator_physical_resource_policy_external<T>(allocator: T) -> bool;
predicate vmalloc_allocator_runtime_mapping_window_bound<T, P>(allocator: T, page_table_caches: P) -> bool;
predicate vmalloc_allocator_multi_window_mapping_supported<T>(allocator: T) -> bool;
predicate vmalloc_allocator_preallocated_mapping_window_bound<T>(allocator: T) -> bool;
predicate vmalloc_allocator_dynamic_l0_window_allocation_supported<T, P>(allocator: T, page_table_caches: P) -> bool;
predicate vmalloc_pgtable_full_range_metadata_ready<P>(page_table_caches: P) -> bool;
predicate vmalloc_allocator_full_vmalloc_range_metadata_supported<T, P>(allocator: T, page_table_caches: P) -> bool;
predicate vmalloc_allocator_dynamic_record_storage_ready<T>(allocator: T) -> bool;
predicate vmalloc_allocator_rejects_duplicate_area_mapping<T>(allocator: T) -> bool;
predicate vmalloc_allocator_mapping_guard_contract_ready<T>(allocator: T) -> bool;
predicate vmalloc_allocator_unmapping_guard_contract_ready<T>(allocator: T) -> bool;
predicate vmalloc_allocator_mapping_sync_contract_ready<T, S>(allocator: T, swapper_vm: S) -> bool;
predicate vmalloc_allocator_unmapping_flush_contract_ready<T, S>(allocator: T, swapper_vm: S) -> bool;
predicate vmalloc_allocator_failure_rollback_contract_ready<T>(allocator: T) -> bool;
predicate vmalloc_allocator_setup_runtime_locking_spec_required<T>(allocator: T) -> bool;
predicate vmalloc_allocator_runtime_vmap_locking_contract_deferred<T>(allocator: T) -> bool;
predicate vmalloc_allocator_cross_cpu_vmalloc_flush_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_reusable_holes_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_augmented_tree_search_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_lazy_purge_batching_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_rcu_metadata_lifecycle_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_cross_cpu_lazy_fault_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_cache_tlb_batching_deferred<T>(allocator: T) -> bool;
predicate vmalloc_full_per_cpu_deferred_free_deferred<T>(allocator: T) -> bool;
predicate vmap_node_guard_contract_ready<T>(node_set: T) -> bool;
predicate vmap_node_runtime_spinlock_contract_deferred<T>(node_set: T) -> bool;
predicate vfree_deferred_guard_contract_ready<T>(deferred_set: T) -> bool;
predicate vfree_rcu_runtime_path_deferred<T>(deferred_set: T) -> bool;
predicate vmap_block_queue_runtime_lock_contract_deferred<T>(queues: T) -> bool;

predicate vmap_area_ref_ready<T>(area: T) -> bool;
predicate vmap_area_allocated<T, A>(area: T, allocator: A) -> bool;
predicate vmap_area_address_space_bound<T, V>(area: T, vmap_space: V) -> bool;
predicate vmap_area_size_bound<T>(area: T) -> bool;
predicate vmap_area_alignment_bound<T>(area: T) -> bool;
predicate vmap_area_flags_bound<T, F>(area: T, flags: F) -> bool;
predicate vmap_area_reserved_as_busy<T, V>(area: T, vmap_space: V) -> bool;
predicate vmap_area_range_complete_for_mapping<T, M>(area: T, mapping: M) -> bool;
predicate vmap_area_released<T, A>(area: T, allocator: A) -> bool;

predicate vmap_mapping_ref_ready<T>(mapping: T) -> bool;
predicate vmap_mapping_record_created<T, M>(allocator: T, mapping: M) -> bool;
predicate vmap_mapping_area_bound<T, A>(mapping: T, area: A) -> bool;
predicate vmap_mapping_phys_range_bound<T>(mapping: T) -> bool;
predicate vmap_mapping_page_range_installed<T, S>(mapping: T, swapper_vm: S) -> bool;
predicate vmap_mapping_page_range_removed<T, S>(mapping: T, swapper_vm: S) -> bool;
predicate vmap_mapping_kernel_mapping_synced<T, S>(mapping: T, swapper_vm: S) -> bool;
predicate vmap_mapping_kernel_tlb_flushed<T, S>(mapping: T, swapper_vm: S) -> bool;
predicate vmap_mapping_protection_bound<T, P>(mapping: T, protection: P) -> bool;
predicate vmap_mapping_protection_kind_bound<T, K>(mapping: T, kind: K) -> bool;
predicate vmap_mapping_page_aligned<T>(mapping: T) -> bool;
predicate vmap_mapping_within_runtime_mapping_window<T>(mapping: T) -> bool;
predicate vmap_area_has_no_existing_mapping<T, A>(area: T, allocator: A) -> bool;

predicate vmap_flags_vm_ioremap<T>(flags: T) -> bool;
predicate page_protection_io_memory<T>(protection: T) -> bool;
predicate page_protection_kind_io_memory<T>(kind: T) -> bool;

type VmallocAllocatorType: MemoryObject {
    processes {
        /*
         * GetVmArea corresponds to Linux __get_vm_area_caller()/get_vm_area():
         * reserve a contiguous vmap virtual-address area and attach flags.
         */
        Action::GetVmArea(area: VmapAreaRef, flags: VmapAreaFlags) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                VmapAddressSpace.state == State::Ready;
                vmalloc_allocator_vmap_area_api_ready(self);
                vmalloc_allocator_manages_vmap_address_space(self, VmapAddressSpace);
                free_vmap_space_ready(VmapAddressSpace);
                vmalloc_allocator_dynamic_record_storage_ready(self);
            }
            ensures {
                vmap_area_ref_ready(area);
                vmap_area_allocated(area, self);
                vmap_area_address_space_bound(area, VmapAddressSpace);
                vmap_area_size_bound(area);
                vmap_area_alignment_bound(area);
                vmap_area_flags_bound(area, flags);
                vmap_area_reserved_as_busy(area, VmapAddressSpace);
                vmap_area_has_no_existing_mapping(area, self);
            }
        }

        /*
         * MapPageRange corresponds to the vmap/ioremap page-table execution
         * path. It maps caller-supplied physical pages/PFNs/ranges into an
         * already reserved vmap area using caller-supplied protection flags.
         * Every successful action creates a VmapMapping record for that
         * concrete area/physical-range/protection tuple; the caller may own
         * the physical resource and attribute policy, but the vmalloc/vmap
         * subsystem owns the installed mapping record.
         */
        Action::MapPageRange(
            area: VmapAreaRef,
            mapping: VmapMappingRef,
            protection: PageProtectionRef
        ) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                SwapperVm.state == State::Online;
                PageTableCaches.state == State::Ready;
                PageAllocator.state == State::Ready;
                vmalloc_allocator_page_range_mapping_api_ready(self);
                vmalloc_allocator_executes_page_table_mappings(self, SwapperVm);
                vmalloc_allocator_runtime_page_table_mapping_ready(self, PageTableCaches);
                vmalloc_allocator_runtime_mapping_window_bound(self, PageTableCaches);
                vmalloc_allocator_dynamic_l0_window_allocation_supported(self, PageTableCaches);
                vmalloc_allocator_full_vmalloc_range_metadata_supported(self, PageTableCaches);
                vmalloc_allocator_dynamic_record_storage_ready(self);
                vmalloc_allocator_mapping_guard_contract_ready(self);
                vmalloc_allocator_mapping_sync_contract_ready(self, SwapperVm);
                vmalloc_allocator_failure_rollback_contract_ready(self);
                vmap_area_ref_ready(area);
                vmap_area_allocated(area, self);
                vmap_area_address_space_bound(area, VmapAddressSpace);
                vmap_area_has_no_existing_mapping(area, self);
            }
            ensures {
                vmap_mapping_ref_ready(mapping);
                vmap_mapping_record_created(self, mapping);
                vmap_mapping_area_bound(mapping, area);
                vmap_area_range_complete_for_mapping(area, mapping);
                vmap_mapping_phys_range_bound(mapping);
                vmap_mapping_page_range_installed(mapping, SwapperVm);
                vmap_mapping_kernel_mapping_synced(mapping, SwapperVm);
                vmap_mapping_protection_bound(mapping, protection);
                vmap_mapping_page_aligned(mapping);
                vmap_mapping_within_runtime_mapping_window(mapping);
            }
        }

        /*
         * UnmapPageRange corresponds to the vmalloc/vmap page-table teardown
         * path used by vunmap()/iounmap(): remove the installed PTEs for a
         * concrete mapping record before the vmap area metadata is released.
         */
        Action::UnmapPageRange(area: VmapAreaRef, mapping: VmapMappingRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                SwapperVm.state == State::Online;
                PageTableCaches.state == State::Ready;
                vmalloc_allocator_page_range_mapping_api_ready(self);
                vmalloc_allocator_executes_page_table_mappings(self, SwapperVm);
                vmalloc_allocator_runtime_page_table_mapping_ready(self, PageTableCaches);
                vmalloc_allocator_unmapping_guard_contract_ready(self);
                vmalloc_allocator_unmapping_flush_contract_ready(self, SwapperVm);
                vmap_area_ref_ready(area);
                vmap_area_allocated(area, self);
                vmap_mapping_ref_ready(mapping);
                vmap_mapping_area_bound(mapping, area);
                vmap_mapping_page_range_installed(mapping, SwapperVm);
            }
            ensures {
                vmap_mapping_page_range_removed(mapping, SwapperVm);
                vmap_mapping_kernel_tlb_flushed(mapping, SwapperVm);
            }
        }

        /*
         * FreeVmArea models releasing a reserved vmap area and its metadata.
         * Full TLB/cache ordering is deferred to later vmalloc teardown work.
         */
        Action::FreeVmArea(area: VmapAreaRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                vmalloc_allocator_vmap_area_api_ready(self);
                vmalloc_allocator_unmapping_guard_contract_ready(self);
                vmap_area_ref_ready(area);
                vmap_area_allocated(area, self);
            }
            ensures {
                vmap_area_released(area, self);
            }
        }
    }
}
