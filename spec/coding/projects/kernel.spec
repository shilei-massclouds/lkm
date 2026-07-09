/*
 * Kernel project coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in kernel.md.
 */

predicate kernel_project_coding_maps_to_arceos_ex_project_module() -> bool;
predicate kernel_project_coding_records_spec_chain() -> bool;
predicate kernel_project_coding_uses_model_lds_and_config_inputs() -> bool;
predicate kernel_project_coding_does_not_drive_runtime_phases() -> bool;
predicate kernel_project_coding_does_not_emit_runtime_checkpoints() -> bool;
predicate kernel_project_coding_keeps_build_inputs_project_owned() -> bool;

type KernelProjectCoding {
    invariant {
        /* Mapping path. */
        kernel_project_coding_maps_to_arceos_ex_project_module();

        /* Spec chain. */
        kernel_project_coding_records_spec_chain();

        /* Build inputs. */
        kernel_project_coding_uses_model_lds_and_config_inputs();

        /* Runtime separation. */
        kernel_project_coding_does_not_drive_runtime_phases();

        /* Checkpoint ownership. */
        kernel_project_coding_does_not_emit_runtime_checkpoints();

        /* Ownership. */
        kernel_project_coding_keeps_build_inputs_project_owned();
    }
}
