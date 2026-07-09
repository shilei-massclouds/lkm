/*
 * DeviceTree SHOULD coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in device-tree-should.md.
 */

predicate arceos_ex_should_encapsulate_device_tree_unflatten_unsafe() -> bool;

type ArceosExDeviceTreeCodingShould {
    invariant {
        /* Unsafe encapsulation. */
        arceos_ex_should_encapsulate_device_tree_unflatten_unsafe();
    }
}
