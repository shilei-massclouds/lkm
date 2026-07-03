/*
 * Object Coding Mapping Specification
 *
 * These predicates name mandatory constraints for generating object-level code
 * from spec/model. They are intentionally separate from mapping.md so tools and
 * AI agents can consume the hard rules and strong recommendations through the
 * formal *.spec entry.
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

predicate coding_should_use_global_context() -> bool;
predicate coding_should_name_context_parameters_ctx() -> bool;
predicate coding_should_group_context_resources_by_category() -> bool;
predicate coding_should_avoid_phase_local_context_accessors() -> bool;

type CodingRuleLevels {
    invariant {
        /*
         * MUST:
         *
         * A mandatory rule. Violating a MUST rule blocks implementation until
         * the model, coding spec, tool or implementation is corrected.
         */
        coding_rule_level_must_defined();

        /*
         * SHOULD:
         *
         * A strong recommendation. Generated or handwritten code is expected to
         * follow it by default. A deviation is allowed only with an explicit
         * recorded reason, scope and future convergence path.
         */
        coding_rule_level_should_defined();

        /*
         * MAY:
         *
         * An optional technique. It is permitted when useful but must not be
         * required by later code unless promoted to SHOULD or MUST.
         */
        coding_rule_level_may_defined();

        /*
         * NOTE:
         *
         * Explanatory guidance. It carries no direct implementation obligation
         * and cannot override MUST or SHOULD rules.
         */
        coding_rule_level_note_defined();
    }
}

type CodingMappingMust {
    invariant {
        /*
         * Model priority:
         *
         * Implementation must strictly follow objects, states, transitions,
         * dependencies, drive order, phase boundaries and proof obligations
         * derived from spec/model. Implementation convenience, directory habit,
         * reused code, early Rust entry, reduced assembly or build-tool limits
         * are not valid reasons to diverge from model semantics.
         */
        coding_must_model_priority();

        /*
         * Map before coding:
         *
         * Code generation or modification must first identify the model object
         * to source-file mapping. Code may only be written to the mapped file or
         * its explicit submodules unless a recorded exception exists.
         */
        coding_must_map_before_coding();

        /*
         * Object ownership:
         *
         * The main implementation of a model object must live in that object's
         * mapped file. Phase setup/handoff/checkpoint code belongs to the Phase
         * file; resource-object state and event methods belong to the resource
         * object file.
         */
        coding_must_preserve_object_ownership();

        /*
         * Phase tree path:
         *
         * Phase file paths must reflect the model parent/child phase tree.
         */
        coding_must_phase_path_matches_tree();

        /*
         * DFS generation:
         *
         * Phase code must be generated from the phase tree using depth-first
         * traversal. Runtime control between sibling phases is connected by
         * handoff() -> next.setup(), not by flattening all child phase bodies
         * into the parent phase.
         */
        coding_must_phase_dfs_generation();

        /*
         * Phase shape:
         *
         * Phase objects must not be implemented as resource-object style
         * struct + impl lifecycle state machines. They are process modules with
         * setup(), handoff(), boundary checks and checkpoints.
         */
        coding_must_phase_not_resource_lifecycle();

        /*
         * Transition boundaries:
         *
         * Each model transition must keep a locatable code boundary. If low-level
         * code must complete adjacent transitions without a hardware-visible gap, the
         * mapped source must still preserve transition functions, state adoption,
         * checks and checkpoint boundaries.
         */
        coding_must_keep_transition_boundaries();

        /*
         * Checks before checkpoints:
         *
         * depends_on, ensures and invariant facts must be checked before the
         * target state is committed, a checkpoint is emitted or later code uses
         * the fact as an established dependency.
         */
        coding_must_check_before_checkpoint();

        /*
         * Checkpoint ownership:
         *
         * A checkpoint may only be emitted by the mapped implementation of the
         * object event or phase boundary whose fact it reports. A lower-level
         * phase, resource object, assembly entry point or continuation must not
         * emit synthetic checkpoints for parent phases, sibling phases,
         * preparation phases or other objects merely to make the runtime trace
         * visually match the derived trace.
         */
        coding_must_checkpoint_owner_matches_transition();

        /*
         * Checkpoint as observation timing:
         *
         * A checkpoint is a stable observation timing boundary. It may trigger
         * trace backends or observer handlers, but it is not ordinary logging,
         * does not advance object state by itself, and must not become a
         * dependency of the transition whose boundary it observes.
         */
        coding_must_checkpoint_hook_be_observation_timing_only();

        /*
         * Observation facts:
         *
         * Structured observation content must live on the owning object,
         * provider or explicit context object as long-term facts. Handlers may
         * read, classify or emit those facts; they must not manufacture model
         * facts through handler-local debug state.
         */
        coding_must_observation_facts_live_on_objects_or_providers();

        /*
         * Failure diagnostic separation:
         *
         * Failure diagnostics are structured error payloads collected on a
         * failing predicate/check path and emitted when the error is reported.
         * They are not additional checkpoints and are not checkpoint handlers
         * registered on a failure point. Supporting them must not change the
         * successful checkpoint sequence.
         */
        coding_must_failure_diagnostic_not_be_checkpoint_handler();

        /*
         * Linker script mapping:
         *
         * A generated or maintained linker script is the coding artifact that
         * realizes the model Lds object. It must be driven by the PreparePhase
         * Lds attributes and by the Config attributes that Lds depends on, such
         * as kernel addresses, page size, section alignment, head-text layout
         * and boot-stack size. Hard-coded linker constants are only permitted
         * as recorded transitional exceptions with the corresponding model
         * Lds/Config source named.
         */
        coding_must_linker_script_driven_by_model_lds();

        /*
         * Explicit exceptions:
         *
         * Architecture, linker, Rust-language or boot-ABI constraints that force
         * code outside the default mapped file must be recorded with reason,
         * scope and the preserved model boundary.
         */
        coding_must_record_exceptions();

        /*
         * Verification gates:
         *
         * Changes touching model semantics, phase call chains, object states or
         * entry paths must pass the configured build/verify/run gates before
         * they are treated as complete.
         */
        coding_must_run_verification_gates();
    }
}

type CodingMappingShould {
    invariant {
        /*
         * Global context:
         *
         * Object Coding Phase should maintain long-lived resource objects in a
         * single implementation Context rather than in phase-local object
         * carriers. Phase modules should borrow this Context and use it to
         * drive resource-object transitions.
         */
        coding_should_use_global_context();

        /*
         * Context parameter names:
         *
         * Function parameters and local variables that carry the implementation
         * Context should be named ctx or context. They should not be named
         * objects, because objects has a formal model meaning.
         */
        coding_should_name_context_parameters_ctx();

        /*
         * Context resource layout:
         *
         * Context fields should be grouped by resource-object category, matching
         * the source directory hierarchy as it evolves. They should not be
         * grouped by Phase except as an explicitly recorded transitional step.
         */
        coding_should_group_context_resources_by_category();

        /*
         * Context accessor placement:
         *
         * Context accessors should be centralized, for example as
         * crate::context::context() and crate::context::context_ref(). Phase
         * files should not define their own local objects()/context() accessors.
         */
        coding_should_avoid_phase_local_context_accessors();
    }
}

type LinuxCheckpointMappingRules {
    invariant {
        /*
         * Static source only:
         *
         * Linux checkpoint mapping artifacts record reviewable anchors in the
         * read-only Linux reference tree. The mapping pass must not modify
         * Linux source, add probes, or depend on runtime collection to justify
         * a mapping.
         */
        linux_checkpoint_mapping_must_be_static_source_only();

        /*
         * Unproven stays unmapped:
         *
         * If a checkpoint cannot be tied to a stable Linux source boundary from
         * the reference tree, it must remain unmapped and the notes must record
         * why no reliable boundary was claimed.
         */
        linux_checkpoint_mapping_must_keep_unproven_unmapped();

        /*
         * Partial boundaries:
         *
         * When Linux lacks a local object boundary matching the arceos_ex
         * checkpoint, the mapping should use a range or medium confidence to
         * make the partial alignment explicit.
         */
        linux_checkpoint_mapping_should_mark_partial_boundaries();

        /*
         * UserBoot versus UserExec:
         *
         * Boot-time init exec checkpoints may reuse Linux exec/binfmt/return
         * anchors that also describe runtime exec syscall checkpoints, but the
         * notes must distinguish the boot-time init exec view from the runtime
         * exec syscall view.
         */
        linux_checkpoint_mapping_must_distinguish_user_boot_from_user_exec();
    }
}
