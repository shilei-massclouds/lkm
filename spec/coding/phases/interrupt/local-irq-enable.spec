/*
 * LocalIrqEnablePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in local-irq-enable.md.
 */

predicate arceos_ex_must_local_irq_enable_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_local_irq_enable_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_local_irq_enable_be_separate_interrupt_subphase() -> bool;
predicate arceos_ex_must_local_irq_enable_only_open_boot_cpu_local_gate() -> bool;
predicate arceos_ex_must_local_irq_enable_clear_early_flag_before_enabling_sie() -> bool;
predicate arceos_ex_must_local_irq_enable_have_no_within_context() -> bool;
predicate arceos_ex_must_local_irq_enable_keep_runtime_gates_deferred() -> bool;

type ArceosExLocalIrqEnableCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_local_irq_enable_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_local_irq_enable_code_path_follow_interrupt_phase_tree();

        /* Separate subphase. */
        arceos_ex_must_local_irq_enable_be_separate_interrupt_subphase();

        /* Scope. */
        arceos_ex_must_local_irq_enable_only_open_boot_cpu_local_gate();

        /* Linux ordering. */
        arceos_ex_must_local_irq_enable_clear_early_flag_before_enabling_sie();

        /* Context. */
        arceos_ex_must_local_irq_enable_have_no_within_context();

        /* Deferred runtime gates. */
        arceos_ex_must_local_irq_enable_keep_runtime_gates_deferred();
    }
}
