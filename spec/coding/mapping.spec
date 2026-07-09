/*
 * Object coding mapping formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in mapping.md.
 */

predicate coding_rule_level_must_defined() -> bool;
predicate coding_rule_level_should_defined() -> bool;
predicate coding_rule_level_may_defined() -> bool;
predicate coding_rule_level_note_defined() -> bool;

predicate coding_must_model_priority() -> bool;
predicate coding_must_map_before_coding() -> bool;
predicate coding_must_preserve_object_ownership() -> bool;
predicate coding_must_phase_path_matches_tree() -> bool;
predicate coding_must_phase_dfs_generation() -> bool;
predicate coding_must_phase_not_resource_lifecycle() -> bool;
predicate coding_must_keep_transition_boundaries() -> bool;
predicate coding_must_check_before_checkpoint() -> bool;
predicate coding_must_checkpoint_owner_matches_transition() -> bool;
predicate coding_must_checkpoint_hook_be_observation_timing_only() -> bool;
predicate coding_must_observation_facts_live_on_objects_or_providers() -> bool;
predicate coding_must_failure_diagnostic_not_be_checkpoint_handler() -> bool;
predicate coding_must_linker_script_driven_by_model_lds() -> bool;
predicate coding_must_record_exceptions() -> bool;
predicate coding_must_run_verification_gates() -> bool;
predicate linux_checkpoint_mapping_must_be_static_source_only() -> bool;
predicate linux_checkpoint_mapping_must_keep_unproven_unmapped() -> bool;
predicate linux_checkpoint_mapping_should_mark_partial_boundaries() -> bool;
predicate linux_checkpoint_mapping_must_distinguish_user_boot_from_user_exec() -> bool;
predicate linux_checkpoint_mapping_must_distinguish_shared_call_site_semantics() -> bool;

predicate coding_should_use_global_context() -> bool;
predicate coding_should_name_context_parameters_ctx() -> bool;
predicate coding_should_group_context_resources_by_category() -> bool;
predicate coding_should_avoid_phase_local_context_accessors() -> bool;

type CodingRuleLevels {
    invariant {
        /* MUST. */
        coding_rule_level_must_defined();

        /* SHOULD. */
        coding_rule_level_should_defined();

        /* MAY. */
        coding_rule_level_may_defined();

        /* NOTE. */
        coding_rule_level_note_defined();
    }
}

type CodingMappingMust {
    invariant {
        /* Model priority. */
        coding_must_model_priority();

        /* Map before coding. */
        coding_must_map_before_coding();

        /* Object ownership. */
        coding_must_preserve_object_ownership();

        /* Phase tree path. */
        coding_must_phase_path_matches_tree();

        /* DFS generation. */
        coding_must_phase_dfs_generation();

        /* Phase shape. */
        coding_must_phase_not_resource_lifecycle();

        /* Transition boundaries. */
        coding_must_keep_transition_boundaries();

        /* Checks before checkpoints. */
        coding_must_check_before_checkpoint();

        /* Checkpoint ownership. */
        coding_must_checkpoint_owner_matches_transition();

        /* Checkpoint as observation timing. */
        coding_must_checkpoint_hook_be_observation_timing_only();

        /* Observation facts. */
        coding_must_observation_facts_live_on_objects_or_providers();

        /* Failure diagnostic separation. */
        coding_must_failure_diagnostic_not_be_checkpoint_handler();

        /* Linker script mapping. */
        coding_must_linker_script_driven_by_model_lds();

        /* Explicit exceptions. */
        coding_must_record_exceptions();

        /* Verification gates. */
        coding_must_run_verification_gates();
    }
}

type CodingMappingShould {
    invariant {
        /* Global context. */
        coding_should_use_global_context();

        /* Context parameter names. */
        coding_should_name_context_parameters_ctx();

        /* Context resource layout. */
        coding_should_group_context_resources_by_category();

        /* Context accessor placement. */
        coding_should_avoid_phase_local_context_accessors();
    }
}

type LinuxCheckpointMappingRules {
    invariant {
        /* Static source only. */
        linux_checkpoint_mapping_must_be_static_source_only();

        /* Unproven stays unmapped. */
        linux_checkpoint_mapping_must_keep_unproven_unmapped();

        /* Partial boundaries. */
        linux_checkpoint_mapping_should_mark_partial_boundaries();

        /* UserBoot versus UserExec. */
        linux_checkpoint_mapping_must_distinguish_user_boot_from_user_exec();

        /* Shared call-site semantics. */
        linux_checkpoint_mapping_must_distinguish_shared_call_site_semantics();
    }
}
