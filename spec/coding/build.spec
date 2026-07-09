/*
 * Build and script formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in build.md.
 */

predicate build_must_keep_top_level_make_as_stable_entry() -> bool;
predicate build_must_delegate_kernel_specific_rules_to_kernel_dir() -> bool;
predicate build_must_keep_targets_composable() -> bool;
predicate build_must_make_disk_reproducible_input_builder() -> bool;
predicate build_must_make_disk_create_image_only_when_missing_by_default() -> bool;
predicate build_must_cache_downloaded_rootfs_inputs_under_build() -> bool;
predicate build_must_run_depend_on_required_runtime_inputs() -> bool;
predicate build_must_not_hide_model_codegen_or_verification_boundaries() -> bool;
predicate build_must_preserve_app_payload_selection_as_explicit_parameter() -> bool;
predicate build_must_stage_external_tools_as_configurable_commands() -> bool;
predicate build_must_keep_test_aggregate_decomposable() -> bool;
predicate build_must_gate_checkpoint_artifact_drift_before_runtime_tests() -> bool;
predicate build_must_not_check_in_generated_or_runtime_local_outputs() -> bool;
predicate build_must_make_clean_remove_routine_artifacts_only() -> bool;

predicate build_should_name_image_and_fs_knobs_explicitly() -> bool;
predicate build_should_make_qemu_devices_data_driven() -> bool;
predicate build_should_fail_fast_on_unknown_configuration() -> bool;
predicate build_should_document_new_targets_before_use() -> bool;

type BuildAndScriptCodingMust {
    invariant {
        /* Stable entry. */
        build_must_keep_top_level_make_as_stable_entry();

        /* Delegation. */
        build_must_delegate_kernel_specific_rules_to_kernel_dir();

        /* Composable targets. */
        build_must_keep_targets_composable();

        /* Disk image input. */
        build_must_make_disk_reproducible_input_builder();

        /* Idempotent disk creation. */
        build_must_make_disk_create_image_only_when_missing_by_default();

        /* Downloaded rootfs cache. */
        build_must_cache_downloaded_rootfs_inputs_under_build();

        /* Runtime dependencies. */
        build_must_run_depend_on_required_runtime_inputs();

        /* Visible model/codegen boundary. */
        build_must_not_hide_model_codegen_or_verification_boundaries();

        /* Payload selection. */
        build_must_preserve_app_payload_selection_as_explicit_parameter();

        /* External tool commands. */
        build_must_stage_external_tools_as_configurable_commands();

        /* Decomposable tests. */
        build_must_keep_test_aggregate_decomposable();

        /* Checkpoint artifact drift gate. */
        build_must_gate_checkpoint_artifact_drift_before_runtime_tests();

        /* Generated output hygiene. */
        build_must_not_check_in_generated_or_runtime_local_outputs();

        /* Clean scope. */
        build_must_make_clean_remove_routine_artifacts_only();
    }
}

type BuildAndScriptCodingShould {
    invariant {
        /* Explicit image and file-system knobs. */
        build_should_name_image_and_fs_knobs_explicitly();

        /* Data-driven QEMU devices. */
        build_should_make_qemu_devices_data_driven();

        /* Unknown configuration. */
        build_should_fail_fast_on_unknown_configuration();

        /* Target documentation. */
        build_should_document_new_targets_before_use();
    }
}
