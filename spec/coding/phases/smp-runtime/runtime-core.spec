/*
 * RuntimeCorePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in runtime-core.md.
 */


type ArceosExRuntimeCoreCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_runtime_core_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_runtime_core_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_runtime_core_run_after_smp_bringup();

        /* Scheduler SMP action. */
        arceos_ex_must_runtime_core_enable_scheduler_smp_action();
        arceos_ex_must_runtime_core_use_sched_domains_mutex_guard();

        /* Workqueue topology. */
        arceos_ex_must_runtime_core_setup_workqueue_topology_action();
        arceos_ex_must_runtime_core_use_workqueue_topology_mutex_guards();

        /* Deferred runtime cores. */
        arceos_ex_must_runtime_core_keep_async_and_padata_deferred();

        /* Page allocator late. */
        arceos_ex_must_runtime_core_setup_page_allocator_late_action();
        arceos_ex_must_runtime_core_record_page_late_trimmed_reasons();
    }
}
