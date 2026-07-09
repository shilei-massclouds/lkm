/*
 * Completion coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in completion.md.
 */

predicate arceos_ex_must_completion_map_to_reusable_object() -> bool;
predicate arceos_ex_must_completion_own_simple_wait_queue() -> bool;
predicate arceos_ex_must_completion_processes_keep_state_effects() -> bool;
predicate arceos_ex_must_completion_instances_drive_type_processes() -> bool;
predicate arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow() -> bool;

type ArceosExCompletionCodingMust {
    invariant {
        /* Reusable object. */
        arceos_ex_must_completion_map_to_reusable_object();

        /* Owned wait queue. */
        arceos_ex_must_completion_own_simple_wait_queue();

        /* Event/action boundary. */
        arceos_ex_must_completion_processes_keep_state_effects();

        /* Instance inheritance. */
        arceos_ex_must_completion_instances_drive_type_processes();

        /* Smoke coverage. */
        arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow();
    }
}
