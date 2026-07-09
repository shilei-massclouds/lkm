/*
 * IrqOpenPreparePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in irq-open-prepare.md.
 */

predicate arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_irq_open_prepare_run_after_local_irq_enable() -> bool;
predicate arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online() -> bool;
predicate arceos_ex_must_irq_open_prepare_console_prepared_only() -> bool;
predicate arceos_ex_must_irq_open_prepare_record_trimmed_paths_structurally() -> bool;
predicate arceos_ex_must_irq_open_prepare_sched_clock_record_local_irq_guard() -> bool;
predicate arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions() -> bool;

type ArceosExIrqOpenPrepareCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree();

        /* Ordering. */
        arceos_ex_must_irq_open_prepare_run_after_local_irq_enable();

        /* Runtime services. */
        arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred();

        /* SLUB late boundary. */
        arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online();

        /* Console boundary. */
        arceos_ex_must_irq_open_prepare_console_prepared_only();

        /* Trimmed/deferred paths. */
        arceos_ex_must_irq_open_prepare_record_trimmed_paths_structurally();

        /* Sched clock local IRQ guard. */
        arceos_ex_must_irq_open_prepare_sched_clock_record_local_irq_guard();

        /* Smoke actions. */
        arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions();
    }
}
