/*
 * Effective Context coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in effective-context.md.
 */

predicate arceos_ex_must_phase_boundary_guard_lower_to_context_contribution_only() -> bool;
predicate arceos_ex_must_preemption_guard_lower_to_counted_enter_exit() -> bool;
predicate arceos_ex_must_local_interrupt_guard_preserve_saved_flags() -> bool;
predicate arceos_ex_must_raw_spin_lock_irqsave_guard_lock_and_restore() -> bool;
predicate arceos_ex_must_raw_spin_lock_guard_use_plain_lock_unlock() -> bool;
predicate arceos_ex_must_effective_context_not_elide_protocol_guards() -> bool;
predicate arceos_ex_must_defer_rcu_lowering_until_primitive_exists() -> bool;
predicate arceos_ex_must_rcu_read_side_first_slice_remain_marked_incomplete() -> bool;
predicate arceos_ex_must_defer_full_rcu_read_side_lowering() -> bool;

type ArceosExEffectiveContextCodingMust {
    invariant {
        /* Natural phase-boundary guard. */
        arceos_ex_must_phase_boundary_guard_lower_to_context_contribution_only();

        /* PreemptionControl guard boundary. */
        arceos_ex_must_preemption_guard_lower_to_counted_enter_exit();

        /* LocalInterruptControl guard boundary. */
        arceos_ex_must_local_interrupt_guard_preserve_saved_flags();

        /* RawSpinLock irq-save guard boundary. */
        arceos_ex_must_raw_spin_lock_irqsave_guard_lock_and_restore();

        /* RawSpinLock ordinary guard boundary. */
        arceos_ex_must_raw_spin_lock_guard_use_plain_lock_unlock();

        /* Effective Context is not a license to erase protocol guards. */
        arceos_ex_must_effective_context_not_elide_protocol_guards();

        /* RCU read-side first slice. */
        arceos_ex_must_rcu_read_side_first_slice_remain_marked_incomplete();
        arceos_ex_must_defer_full_rcu_read_side_lowering();
    }
}
