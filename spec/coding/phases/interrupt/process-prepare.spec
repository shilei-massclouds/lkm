/*
 * ProcessPreparePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in process-prepare.md.
 */

predicate arceos_ex_must_process_prepare_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_process_prepare_run_after_irq_open_prepare() -> bool;
predicate arceos_ex_must_process_prepare_not_create_rest_init_tasks() -> bool;
predicate arceos_ex_must_process_prepare_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects() -> bool;
predicate arceos_ex_must_task_creation_core_bind_task_entry_in_copy_process() -> bool;
predicate arceos_ex_must_task_creation_core_api_smoke_use_copy_process_contract() -> bool;
predicate arceos_ex_must_process_prepare_keep_deferred_paths_explicit() -> bool;
predicate arceos_ex_must_process_prepare_record_trimmed_paths_structurally() -> bool;

type ArceosExProcessPrepareCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_process_prepare_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree();

        /* Ordering. */
        arceos_ex_must_process_prepare_run_after_irq_open_prepare();

        /* rest_init boundary. */
        arceos_ex_must_process_prepare_not_create_rest_init_tasks();

        /* Runtime services. */
        arceos_ex_must_process_prepare_keep_runtime_services_deferred();

        /* Object coverage. */
        arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects();

        /* Task entry creation contract. */
        arceos_ex_must_task_creation_core_bind_task_entry_in_copy_process();

        /* TaskCreationCore API smoke. */
        arceos_ex_must_task_creation_core_api_smoke_use_copy_process_contract();

        /* Deferred paths. */
        arceos_ex_must_process_prepare_keep_deferred_paths_explicit();

        /* Trimmed/deferred path carrier. */
        arceos_ex_must_process_prepare_record_trimmed_paths_structurally();
    }
}
