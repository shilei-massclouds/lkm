/*
 * Kernel System Coding Specification
 *
 * This file constrains the implementation mapping for the running Kernel
 * system instance. It preserves the current startup timeline as a compatibility
 * boundary while naming the system-level mapping target.
 */

predicate kernel_system_coding_maps_to_arceos_ex_system_module() -> bool;
predicate kernel_system_coding_records_spec_chain() -> bool;
predicate kernel_system_coding_owns_runtime_phase_order() -> bool;
predicate kernel_system_coding_preserves_startup_timeline_boundary() -> bool;
predicate kernel_system_coding_does_not_reorder_phase_calls() -> bool;
predicate kernel_system_coding_does_not_reassign_phase_ownership() -> bool;
predicate kernel_system_coding_preserves_payload_handoff_boundary() -> bool;

type KernelSystemCoding {
    invariant {
        /*
         * Mapping path:
         *
         * Kernel system implementation metadata belongs under
         * impl/arceos_ex/src/systems/. The current skeleton maps the system
         * object to impl/arceos_ex/src/systems/kernel.rs.
         */
        kernel_system_coding_maps_to_arceos_ex_system_module();

        /*
         * Spec chain:
         *
         * The implementation entry must record the charter, model and coding
         * paths that define the running system object before later behavior is
         * added.
         */
        kernel_system_coding_records_spec_chain();

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
         * Compatibility boundary:
         *
         * startup_timeline_ready and startup_timeline_event are the current
         * implementation boundary for system-level startup compatibility.
         * Skeleton mapping work must not move them.
         */
        kernel_system_coding_preserves_startup_timeline_boundary();

        /*
         * Behavior preservation:
         *
         * Skeleton mapping work must not reorder existing phase calls or
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
