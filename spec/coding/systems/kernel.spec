/*
 * Kernel system coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in kernel.md.
 */

predicate kernel_system_coding_maps_to_arceos_ex_system_module() -> bool;
predicate kernel_system_coding_implements_kernel_lifecycle_boundary() -> bool;
predicate kernel_system_coding_preserves_model_lifecycle_states() -> bool;
predicate kernel_system_coding_owns_runtime_phase_order() -> bool;
predicate kernel_system_coding_uses_kernel_system_state() -> bool;
predicate kernel_system_coding_does_not_reorder_phase_calls() -> bool;
predicate kernel_system_coding_does_not_reassign_phase_ownership() -> bool;
predicate kernel_system_coding_preserves_payload_handoff_boundary() -> bool;

type KernelSystemCoding {
    invariant {
        /* Mapping path. */
        kernel_system_coding_maps_to_arceos_ex_system_module();

        /* Kernel lifecycle boundary. */
        kernel_system_coding_implements_kernel_lifecycle_boundary();
        kernel_system_coding_preserves_model_lifecycle_states();

        /* Runtime choreography. */
        kernel_system_coding_owns_runtime_phase_order();

        /* Kernel state. */
        kernel_system_coding_uses_kernel_system_state();

        /* Behavior preservation. */
        kernel_system_coding_does_not_reorder_phase_calls();

        /* Phase ownership. */
        kernel_system_coding_does_not_reassign_phase_ownership();

        /* Payload handoff. */
        kernel_system_coding_preserves_payload_handoff_boundary();
    }
}
