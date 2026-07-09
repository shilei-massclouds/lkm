/*
 * Kernel Project Coding Specification
 *
 * This file constrains the implementation mapping for the KernelProject
 * engineering product. It does not own runtime phase choreography.
 */

predicate kernel_project_coding_maps_to_arceos_ex_project_module() -> bool;
predicate kernel_project_coding_records_spec_chain() -> bool;
predicate kernel_project_coding_uses_model_lds_and_config_inputs() -> bool;
predicate kernel_project_coding_does_not_drive_runtime_phases() -> bool;
predicate kernel_project_coding_does_not_emit_runtime_checkpoints() -> bool;
predicate kernel_project_coding_keeps_build_inputs_project_owned() -> bool;

type KernelProjectCoding {
    invariant {
        /*
         * Mapping path:
         *
         * KernelProject implementation metadata belongs under
         * impl/arceos_ex/src/projects/. The current skeleton maps the project
         * object to impl/arceos_ex/src/projects/kernel.rs.
         */
        kernel_project_coding_maps_to_arceos_ex_project_module();

        /*
         * Spec chain:
         *
         * The implementation entry must record the charter, model and coding
         * paths that define the project object before later behavior is added.
         */
        kernel_project_coding_records_spec_chain();

        /*
         * Build inputs:
         *
         * Project-level construction may depend on model Lds and Config facts,
         * architecture/firmware/platform facts, and build configuration facts.
         */
        kernel_project_coding_uses_model_lds_and_config_inputs();

        /*
         * Runtime separation:
         *
         * KernelProject implementation must not drive BootPhase,
         * InterruptPhase, UpMultitaskPhase, SmpRuntimePhase or PayloadPhase.
         * Those transitions belong to the Kernel system and phase mappings.
         */
        kernel_project_coding_does_not_drive_runtime_phases();

        /*
         * Checkpoint ownership:
         *
         * Project mapping code must not emit runtime checkpoints or synthetic
         * phase observations merely to represent system progress.
         */
        kernel_project_coding_does_not_emit_runtime_checkpoints();

        /*
         * Ownership:
         *
         * Build inputs, image construction facts and project/product metadata
         * remain project-owned unless a later model/coding update records a
         * narrower object owner.
         */
        kernel_project_coding_keeps_build_inputs_project_owned();
    }
}
