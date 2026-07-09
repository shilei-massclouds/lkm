/*
 * DeviceTree MUST coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in device-tree-must.md.
 */

predicate arceos_ex_must_device_tree_setup_allocates_from_memblock() -> bool;
predicate arceos_ex_must_device_tree_unflatten_uses_two_passes() -> bool;
predicate arceos_ex_must_device_tree_storage_uses_established_linear_mapping() -> bool;
predicate arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage() -> bool;
predicate arceos_ex_must_device_tree_checkpoint_after_validation() -> bool;

type ArceosExDeviceTreeCodingMust {
    invariant {
        /* MemBlock allocation. */
        arceos_ex_must_device_tree_setup_allocates_from_memblock();

        /* Two-pass unflatten. */
        arceos_ex_must_device_tree_unflatten_uses_two_passes();

        /* Established mapping. */
        arceos_ex_must_device_tree_storage_uses_established_linear_mapping();

        /* No heap or fixed static substitute. */
        arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage();

        /* Checkpoint after validation. */
        arceos_ex_must_device_tree_checkpoint_after_validation();
    }
}
