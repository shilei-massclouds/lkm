/*
 * FinalizePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in finalize.md.
 */


type ArceosExFinalizeCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_finalize_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_finalize_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_finalize_run_after_rootfs_phase();

        /* Deferred cleanup details. */
        arceos_ex_must_finalize_keep_cleanup_details_deferred();

        /* Trimmed current-config paths. */
        arceos_ex_must_finalize_record_trimmed_config_paths();

        /* System state. */
        arceos_ex_must_finalize_publish_system_running();

        /* RCU boot end. */
        arceos_ex_must_finalize_end_rcu_inkernel_boot();

        /* Boundary. */
        arceos_ex_must_finalize_boundary_handoff_to_payload();
    }
}
