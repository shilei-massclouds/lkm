/*
 * Runtime ioremap model.
 *
 * Linux reference shape:
 * - generic_ioremap_prot() obtains a VM_IOREMAP area from the vmalloc/vmap
 *   address-space manager, then installs page-table entries with an IO pgprot.
 * - This is distinct from vmalloc(): both consume vmap virtual address space,
 *   but ioremap binds device physical MMIO ranges and returns an __iomem
 *   membase cookie instead of allocating RAM-backed kernel memory.
 * - This object is runtime ioremap, not EarlyIoremap/FixMap boot-time slots.
 */

type IoMemoryMappingRef {
}

predicate ioremap_runtime_ready<T, A, V, P>(
    ioremap: T,
    allocator: A,
    vmap_space: V,
    page_table_caches: P
) -> bool;
predicate ioremap_uses_swapper_vm<T, S>(ioremap: T, swapper_vm: S) -> bool;
predicate ioremap_uses_vmalloc_area_management<T, A>(ioremap: T, allocator: A) -> bool;
predicate ioremap_uses_vmap_address_space<T, V>(ioremap: T, vmap_space: V) -> bool;
predicate ioremap_distinct_from_vmalloc_allocation<T>(ioremap: T) -> bool;
predicate ioremap_does_not_use_fixmap<T, F>(ioremap: T, fixmap: F) -> bool;
predicate ioremap_io_page_protection_ready<T>(ioremap: T) -> bool;

predicate ioremap_mapping_created<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_owner_bound<T, M, D>(ioremap: T, mapping: M, device: D) -> bool;
predicate ioremap_mapping_phys_range_bound<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_vmap_area_bound<T, M, V>(ioremap: T, mapping: M, vmap_space: V) -> bool;
predicate ioremap_mapping_uses_vm_ioremap_flag<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_uses_io_page_protection<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_page_aligned<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_membase_cookie_ready<T, M>(ioremap: T, mapping: M) -> bool;
predicate ioremap_mapping_not_linear_direct_map<T, M>(ioremap: T, mapping: M) -> bool;
