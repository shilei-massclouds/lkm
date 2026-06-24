/*
 * Runtime ioremap model.
 *
 * Linux reference shape:
 * - generic_ioremap_prot() obtains a VM_IOREMAP area from the vmalloc/vmap
 *   address-space manager, then asks the same vmalloc/vmap subsystem to
 *   install page-table entries with an IO pgprot.
 * - This is distinct from vmalloc(): both consume vmap virtual address space,
 *   but ioremap binds device physical MMIO ranges and returns an __iomem
 *   membase cookie instead of allocating RAM-backed kernel memory.
 * - Ioremap decides the device physical resource and IO protection policy, but
 *   the vmap area reservation and VA/PA page-table mapping record are owned by
 *   the vmalloc/vmap subsystem.
 * - Attribute policy is explicit: plain device ioremap uses the architecture
 *   IO mapping pgprot; non-cache, write-combine, and normal-memory aliases are
 *   separate policy choices and must not be implied by a successful accessible
 *   mapping.
 * - iounmap() retires the ioremap cookie by asking vmalloc/vmap to tear down
 *   the bound mapping and release the vmap area; it does not directly clear
 *   PTEs or maintain vmap free-space metadata itself.
 * - Error paths after vmap area reservation must unwind through the same
 *   vmalloc-owned records; a failed ioremap must not leak a live VM_IOREMAP
 *   area or installed mapping.
 * - This object is runtime ioremap, not EarlyIoremap/FixMap boot-time slots.
 */

type IoMemoryMappingRef {
}

type IoMemoryPhysRangeRef {
}

type MmioMappingKind {
}

type ArchMmioPageAttrRef {
}

predicate ioremap_runtime_ready<T, A, V, P>(
    ioremap: T,
    allocator: A,
    vmap_space: V,
    page_table_caches: P
) -> bool;
predicate ioremap_uses_swapper_vm<T, S>(ioremap: T, swapper_vm: S) -> bool;
predicate ioremap_uses_vmalloc_area_management<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_uses_vmalloc_mapping_execution<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_uses_vmap_address_space<T, V>(ioremap: T, vmap_space: V) -> bool;
predicate ioremap_distinct_from_vmalloc_allocation<T>(ioremap: T) -> bool;
predicate ioremap_does_not_use_fixmap<T, F>(ioremap: T, fixmap: F) -> bool;
predicate ioremap_physical_resource_policy_ready<T>(ioremap: T) -> bool;
predicate ioremap_vm_ioremap_flags_ready<T, F>(ioremap: T, flags: F) -> bool;
predicate ioremap_io_page_protection_ready<T>(ioremap: T) -> bool;
predicate ioremap_mmio_attribute_policy_ready<T>(ioremap: T) -> bool;
predicate ioremap_plain_device_attribute_supported<T, A>(ioremap: T, attr: A) -> bool;
predicate ioremap_noncached_attribute_deferred<T>(ioremap: T) -> bool;
predicate ioremap_writecombine_attribute_deferred<T>(ioremap: T) -> bool;
predicate ioremap_normal_memory_attribute_deferred<T>(ioremap: T) -> bool;
predicate ioremap_mapping_guard_contract_ready<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_unmapping_guard_contract_ready<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_mapping_sync_contract_ready<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_unmapping_flush_contract_ready<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_failure_rollback_contract_ready<T, A>(ioremap: T, allocator: A) -> bool;

predicate ioremap_mapping_created<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_owner_bound<T, M, D>(ioremap: T, mapping: M, device: D) -> bool;
predicate ioremap_mapping_owner_is_system_irqchip<T, M, C>(
    ioremap: T,
    mapping: M,
    irqchip: C
) -> bool;
predicate ioremap_mapping_phys_range_bound<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_phys_range_from_device_resource<T, M, D>(
    ioremap: T,
    mapping: M,
    device: D
) -> bool;
predicate ioremap_mapping_vmap_area_bound<T, M, A>(ioremap: T, mapping: M, area: A) -> bool;
predicate ioremap_mapping_vmalloc_mapping_bound<T, M, V>(ioremap: T, mapping: M, vmap_mapping: V) -> bool;
predicate ioremap_mapping_uses_vm_ioremap_flag<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_uses_io_page_protection<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_kind_bound<T, M, K>(ioremap: T, mapping: M, kind: K) -> bool;
predicate ioremap_mapping_arch_attr_bound<T, M, A>(ioremap: T, mapping: M, attr: A) -> bool;
predicate ioremap_mapping_uses_plain_device_attribute<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_does_not_claim_noncached<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_does_not_claim_writecombine<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_does_not_claim_normal_memory<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_page_aligned<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_membase_cookie_ready<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_not_linear_direct_map<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_sync_observed<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_unmapped<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_membase_cookie_retired<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_unmapping_flush_observed<T, M>(ioremap: T, mapping: M) -> bool;
