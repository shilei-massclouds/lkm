/*
 * Kernel System Coding Specification
 *
 * This file constrains the implementation mapping for the running Kernel
 * system instance. The Kernel system module owns the system-level startup
 * lifecycle boundary while phase-local setup remains in the mapped phase files.
 */

predicate kernel_system_coding_maps_to_arceos_ex_system_module() -> bool;
predicate kernel_system_coding_implements_kernel_lifecycle_boundary() -> bool;
predicate kernel_system_coding_owns_runtime_phase_order() -> bool;
predicate kernel_system_coding_uses_kernel_system_state() -> bool;
predicate kernel_system_coding_does_not_reorder_phase_calls() -> bool;
predicate kernel_system_coding_does_not_reassign_phase_ownership() -> bool;
predicate kernel_system_coding_preserves_payload_handoff_boundary() -> bool;

type KernelSystemCoding {
    invariant {
        /*
         * Mapping path:
         *
         * Kernel system implementation belongs under impl/arceos_ex/src/systems/.
         * The Kernel object maps to impl/arceos_ex/src/systems/kernel.rs.
         */
        kernel_system_coding_maps_to_arceos_ex_system_module();

        /*
         * Kernel lifecycle boundary:
         *
         * systems/kernel.rs must provide the system lifecycle entry points that
         * bridge SMP/runtime readiness into PayloadPhase setup/enable and mark
         * Kernel online after PayloadPhase.Online.
         */
        kernel_system_coding_implements_kernel_lifecycle_boundary();

        /*
         * Runtime choreography:
         *
         * Kernel owns the ordering relationship among BootPhase,
         * InterruptPhase, UpMultitaskPhase, SmpRuntimePhase and PayloadPhase.
         * Individual phase setup/handoff code remains in the mapped phase
         * files.
         */
        kernel_system_coding_owns_runtime_phase_order();

        /*
         * Kernel state:
         *
         * The system module owns the Kernel lifecycle state and emits Kernel
         * system checkpoints. Crate-root entry code must not retain separate
         * startup timeline state.
         */
        kernel_system_coding_uses_kernel_system_state();

        /*
         * Behavior preservation:
         *
         * Kernel system migration must not reorder existing phase calls or
         * checkpoint emission order.
         */
        kernel_system_coding_does_not_reorder_phase_calls();

        /*
         * Phase ownership:
         *
         * System-level mapping records the lifecycle owner but must not absorb
         * phase-local checks, diagnostics or checkpoints out of their phase
         * modules.
         */
        kernel_system_coding_does_not_reassign_phase_ownership();

        /*
         * Payload handoff:
         *
         * The selected payload remains a sibling phase handoff after
         * SMP/runtime readiness, not a project-level build action and not a
         * nested runtime-core side effect.
         */
        kernel_system_coding_preserves_payload_handoff_boundary();
    }
}
