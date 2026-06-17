/*
 * Composition Formal Specification Entry
 *
 * Composition starts after Object Coding Phase. It may choose crate/module/API
 * boundaries, but must not change model object semantics.
 */

predicate compose_must_not_change_object_semantics() -> bool;
predicate compose_must_preserve_object_coding_boundaries() -> bool;
predicate compose_must_record_interface_exceptions() -> bool;
predicate compose_arceos_ex_crate_packages_must_use_ex_suffix() -> bool;
predicate compose_arceos_ex_component_dirs_must_not_conflict_with_arceos() -> bool;
predicate compose_arceos_ex_public_interfaces_should_remain_arceos_compatible() -> bool;
predicate compose_arceos_ex_must_map_public_api_to_objects_through_shims() -> bool;
predicate compose_arceos_ex_stage_objects_must_follow_startup_spine_components() -> bool;
predicate compose_arceos_ex_resource_objects_must_live_in_function_components() -> bool;
predicate compose_arceos_ex_build_target_must_select_ex_component_graph() -> bool;
predicate compose_arceos_ex_tests_must_not_be_modified_for_ex_target() -> bool;
predicate compose_arceos_ex_normal_tests_must_progress_from_simple_to_complex() -> bool;

type CompositionMust {
    invariant {
        compose_must_not_change_object_semantics();
        compose_must_preserve_object_coding_boundaries();
        compose_must_record_interface_exceptions();
    }
}

type ArceosExCompositionMust {
    invariant {
        /*
         * arceos_ex components are parallel implementations, not replacements
         * of existing ArceOS workspace packages. Package names must carry an
         * -ex suffix so both component families can coexist.
         */
        compose_arceos_ex_crate_packages_must_use_ex_suffix();
        compose_arceos_ex_component_dirs_must_not_conflict_with_arceos();

        /*
         * Compatibility is expressed at the public component API. Internal
         * implementation must reach LKM-derived stage/resource objects through
         * component-local shim layers rather than exposing object internals.
         */
        compose_arceos_ex_public_interfaces_should_remain_arceos_compatible();
        compose_arceos_ex_must_map_public_api_to_objects_through_shims();

        /*
         * Stage objects belong on the startup spine:
         * someboot-ex -> axplat-dyn-ex -> axruntime-ex -> payload.
         * Resource objects belong in function components such as ax-alloc-ex,
         * ax-mm-ex and ax-task-ex.
         */
        compose_arceos_ex_stage_objects_must_follow_startup_spine_components();
        compose_arceos_ex_resource_objects_must_live_in_function_components();

        /*
         * The arceos_ex build target is responsible for choosing the _ex
         * component graph. Existing tests must remain source-compatible; test
         * expansion proceeds from helloworld/simple cases toward normal suites.
         */
        compose_arceos_ex_build_target_must_select_ex_component_graph();
        compose_arceos_ex_tests_must_not_be_modified_for_ex_target();
        compose_arceos_ex_normal_tests_must_progress_from_simple_to_complex();
    }
}
