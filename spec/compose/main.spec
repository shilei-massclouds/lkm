/*
 * Composition Formal Specification Entry
 *
 * Composition starts after Object Coding Phase. It may choose crate/module/API
 * boundaries, but must not change model object semantics.
 */

predicate compose_must_not_change_object_semantics() -> bool;
predicate compose_must_preserve_object_coding_boundaries() -> bool;
predicate compose_must_record_interface_exceptions() -> bool;

type CompositionMust {
    invariant {
        compose_must_not_change_object_semantics();
        compose_must_preserve_object_coding_boundaries();
        compose_must_record_interface_exceptions();
    }
}
