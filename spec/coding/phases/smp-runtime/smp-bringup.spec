/*
 * SmpBringupPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in smp-bringup.md.
 */

predicate arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase() -> bool;
predicate arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree() -> bool;
predicate arceos_ex_must_smp_bringup_focus_bp_side_flow() -> bool;
predicate arceos_ex_must_smp_bringup_split_bp_and_ap_phase_lines() -> bool;
predicate arceos_ex_must_smp_bringup_prepare_per_ap_idle_task_and_stack() -> bool;
predicate arceos_ex_must_smp_bringup_use_sbi_hsm_hart_start() -> bool;
predicate arceos_ex_must_smp_bringup_generate_ap_entry_prelude_callin_online_idle_phases() -> bool;
predicate arceos_ex_must_smp_bringup_online_only_after_ap_done_up_ack() -> bool;
predicate arceos_ex_must_smp_bringup_checkpoint_ap_subphases() -> bool;
predicate arceos_ex_must_smp_bringup_keep_hotplug_callbacks_deferred() -> bool;
predicate arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit() -> bool;
predicate arceos_ex_must_smp_bringup_preserve_hotplug_guards() -> bool;
predicate arceos_ex_must_smp_bringup_preserve_completion_wait_locks() -> bool;
predicate arceos_ex_must_smp_bringup_preserve_sbi_boot_data_ordering() -> bool;
predicate arceos_ex_must_smp_bringup_record_ap_local_sync_summary() -> bool;
predicate arceos_ex_must_smp_bringup_make_secondary_cpus_online() -> bool;
predicate arceos_ex_must_smp_bringup_keep_ap_internals_deferred() -> bool;
predicate arceos_ex_must_smp_runtime_keep_later_subphases_deferred() -> bool;

type ArceosExSmpBringupCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree();

        /* BP/AP phase split. */
        arceos_ex_must_smp_bringup_split_bp_and_ap_phase_lines();

        /* Per-AP idle task and stack. */
        arceos_ex_must_smp_bringup_prepare_per_ap_idle_task_and_stack();

        /* SBI HSM start path. */
        arceos_ex_must_smp_bringup_use_sbi_hsm_hart_start();

        /* AP subphases. */
        arceos_ex_must_smp_bringup_generate_ap_entry_prelude_callin_online_idle_phases();

        /* Synchronization. */
        arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit();

        /* CPU hotplug guards. */
        arceos_ex_must_smp_bringup_preserve_hotplug_guards();

        /* Completion wait locks. */
        arceos_ex_must_smp_bringup_preserve_completion_wait_locks();

        /* SBI boot data ordering. */
        arceos_ex_must_smp_bringup_preserve_sbi_boot_data_ordering();

        /* AP local sync summary. */
        arceos_ex_must_smp_bringup_record_ap_local_sync_summary();

        /* Online boundary. */
        arceos_ex_must_smp_bringup_make_secondary_cpus_online();
        arceos_ex_must_smp_bringup_online_only_after_ap_done_up_ack();

        /* AP checkpoints. */
        arceos_ex_must_smp_bringup_checkpoint_ap_subphases();

        /* Deferred hotplug callbacks. */
        arceos_ex_must_smp_bringup_keep_hotplug_callbacks_deferred();

        /* Later runtime. */
        arceos_ex_must_smp_bringup_handoff_to_runtime_core();
        arceos_ex_must_smp_runtime_expand_later_subphases_explicitly();
    }
}
