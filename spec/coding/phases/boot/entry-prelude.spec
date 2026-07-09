/*
 * EntryPreludePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in entry-prelude.md.
 */

predicate arceos_ex_must_entry_prelude_keep_early_alternatives_deferred() -> bool;

type ArceosExEntryPreludeCodingMust {
    invariant {
        /* RISC-V early alternatives boundary. */
        arceos_ex_must_entry_prelude_keep_early_alternatives_deferred();
    }
}
