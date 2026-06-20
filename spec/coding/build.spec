/*
 * Build and Script Coding Formal Specification
 *
 * This file defines mandatory constraints for Makefile targets and helper
 * scripts that drive object-level implementation, generated artifacts, disk
 * images, QEMU runs and validation commands.
 */

predicate build_must_keep_top_level_make_as_stable_entry() -> bool;
predicate build_must_delegate_kernel_specific_rules_to_kernel_dir() -> bool;
predicate build_must_keep_targets_composable() -> bool;
predicate build_must_make_disk_reproducible_input_builder() -> bool;
predicate build_must_run_depend_on_required_runtime_inputs() -> bool;
predicate build_must_not_hide_model_codegen_or_verification_boundaries() -> bool;
predicate build_must_preserve_app_payload_selection_as_explicit_parameter() -> bool;
predicate build_must_stage_external_tools_as_configurable_commands() -> bool;
predicate build_must_keep_test_aggregate_decomposable() -> bool;
predicate build_must_not_check_in_generated_or_runtime_local_outputs() -> bool;

predicate build_should_name_image_and_fs_knobs_explicitly() -> bool;
predicate build_should_make_qemu_devices_data_driven() -> bool;
predicate build_should_fail_fast_on_unknown_configuration() -> bool;
predicate build_should_document_new_targets_before_use() -> bool;

type BuildAndScriptCodingMust {
    invariant {
        /*
         * Stable entry:
         *
         * The repository top-level Makefile is the stable developer and CI
         * entry for object-level implementation. New routine commands should
         * be reachable from it or intentionally documented as lower-level
         * implementation details.
         */
        build_must_keep_top_level_make_as_stable_entry();

        /*
         * Delegation:
         *
         * Kernel-specific build, image, disk and QEMU rules belong to the
         * selected kernel implementation directory, for example
         * impl/arceos_ex/Makefile. The top-level Makefile should delegate with
         * explicit parameters instead of duplicating kernel-specific command
         * bodies.
         */
        build_must_delegate_kernel_specific_rules_to_kernel_dir();

        /*
         * Composable targets:
         *
         * Targets such as build, generate, disk, run, verify, test-verify,
         * test-kunit, test-smoke and test must remain separately callable.
         * An aggregate target may sequence them, but it must not hide a step
         * so that developers cannot rerun or diagnose it independently.
         */
        build_must_keep_targets_composable();

        /*
         * Disk image input:
         *
         * make disk is the canonical builder for runtime block-device images
         * used by QEMU. It must be reproducible from explicit Make variables
         * such as image path, size, file-system type, file-system block size
         * and deterministic fixture file names/content. Kernel runtime code
         * must not depend on manually prepared local disk state.
         */
        build_must_make_disk_reproducible_input_builder();

        /*
         * Runtime dependencies:
         *
         * make run must depend on runtime inputs it needs, including the disk
         * image when QEMU devices include virtio-blk. make build must not
         * create or mutate runtime disk images unless the target explicitly
         * requires it.
         */
        build_must_run_depend_on_required_runtime_inputs();

        /*
         * Visible model/codegen boundary:
         *
         * Makefiles and helper scripts must keep model derivation, generated
         * artifact creation and kernel compilation as visible target edges.
         * They must not silently bypass pyveri/codegen outputs, substitute
         * stale generated files, or turn verification failures into warnings.
         */
        build_must_not_hide_model_codegen_or_verification_boundaries();

        /*
         * Payload selection:
         *
         * Selected payload or test app must remain an explicit build parameter
         * such as APP. Scripts must not infer a different payload from local
         * files, previous runs or environment side effects.
         */
        build_must_preserve_app_payload_selection_as_explicit_parameter();

        /*
         * External tool commands:
         *
         * External tools such as rustc, rust-objcopy, QEMU, mkfs, debugfs and
         * pyveri must be configurable through Make variables or documented
         * script parameters. Hard-coded host-local absolute paths are not
         * allowed in ordinary build targets.
         */
        build_must_stage_external_tools_as_configurable_commands();

        /*
         * Decomposable tests:
         *
         * The aggregate make test target must preserve independently runnable
         * verify, KUnit/checkpoint and smoke stages. Adding a new validation
         * stage requires documenting its ordering, inputs and whether it is
         * part of the default acceptance gate.
         */
        build_must_keep_test_aggregate_decomposable();

        /*
         * Generated output hygiene:
         *
         * Generated files, runtime disk images, QEMU logs, temporary debugfs
         * scripts and trace reports must stay in build/tools/out/tmp-style
         * locations or documented artifact directories. They must not be
         * committed unless a specification explicitly classifies them as
         * stable source inputs.
         */
        build_must_not_check_in_generated_or_runtime_local_outputs();
    }
}

type BuildAndScriptCodingShould {
    invariant {
        /*
         * Explicit image and file-system knobs:
         *
         * Disk-image paths, sizes, FS_TYPE, ext2 block size and deterministic
         * fixture file knobs should have explicit variable names. Avoid
         * embedding these decisions in opaque shell fragments.
         */
        build_should_name_image_and_fs_knobs_explicitly();

        /*
         * Data-driven QEMU devices:
         *
         * QEMU devices should be composed from variables such as QEMU_DEVICES
         * and disk-image paths. The default may target QEMU virt, but the rule
         * should allow tests to remove or replace devices explicitly.
         */
        build_should_make_qemu_devices_data_driven();

        /*
         * Unknown configuration:
         *
         * Unknown provider names, file-system types, payload names or feature
         * values should fail fast at build time rather than falling back to a
         * nearby default.
         */
        build_should_fail_fast_on_unknown_configuration();

        /*
         * Target documentation:
         *
         * New Makefile targets or helper scripts should be documented in this
         * coding directory before they become part of the normal workflow.
         */
        build_should_document_new_targets_before_use();
    }
}
