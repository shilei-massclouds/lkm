/*
 * AI/code-generator behavior specification.
 *
 * These predicates constrain AI/code-generation behavior before, during and
 * after generating artifacts from specifications. Guidance governs the
 * workflow around the core semantic refinement chain; it is not itself a
 * higher semantic authority than charter, model, coding or implementation.
 */

predicate guidance_agent_must_read_generation_principles_before_implementation() -> bool;
predicate guidance_agent_must_read_concrete_spec_requirements_before_implementation() -> bool;
predicate guidance_agent_must_implement_only_after_reading_requirements() -> bool;
predicate guidance_agent_must_check_generated_result_against_principles_after_implementation() -> bool;
predicate guidance_agent_must_check_generated_result_against_concrete_requirements_after_implementation() -> bool;
predicate guidance_agent_must_not_guess_fix_without_reproducible_localization() -> bool;
predicate guidance_agent_must_update_spec_before_behavior_implementation() -> bool;
predicate guidance_agent_must_run_make_test_after_code_change() -> bool;
predicate guidance_agent_must_keep_temporary_layer_inconsistency_uncommitted() -> bool;
predicate guidance_agent_must_review_each_applicable_layer_and_change_only_affected_layers() -> bool;
predicate guidance_agent_must_require_explicit_commit_authorization() -> bool;
predicate guidance_agent_must_treat_charter_as_human_readable_top_semantic_authority() -> bool;
predicate guidance_agent_must_make_model_only_formalize_and_refine_charter() -> bool;
predicate guidance_agent_must_make_coding_only_constrain_model_to_code_mapping() -> bool;
predicate guidance_agent_must_make_impl_only_realize_charter_model_coding_effect() -> bool;
predicate guidance_lower_core_layer_must_preserve_all_applicable_higher_constraints() -> bool;
predicate guidance_lower_core_layer_may_choose_only_when_all_higher_layers_are_unconstrained() -> bool;
predicate guidance_agent_must_promote_new_observable_contract_to_appropriate_core_specification() -> bool;
predicate guidance_agent_must_treat_guidance_compose_testing_as_supporting_responsibilities() -> bool;
predicate guidance_supporting_responsibilities_must_not_override_core_semantics() -> bool;
predicate guidance_agent_must_default_to_charter_first() -> bool;
predicate guidance_agent_must_follow_charter_first_layer_order() -> bool;
predicate guidance_agent_must_resolve_conflicts_by_charter_first_authority() -> bool;
predicate guidance_agent_must_identify_highest_affected_core_layer_before_changes() -> bool;
predicate guidance_agent_must_record_each_core_layer_closure_outcome() -> bool;
predicate guidance_agent_must_not_use_implementation_tests_roadmap_or_compose_to_override_higher_core_layers() -> bool;
predicate guidance_agent_must_stop_for_charter_decision_when_intent_is_unclear_or_conflicting() -> bool;
predicate guidance_agent_must_require_explicit_user_trigger_for_model_first() -> bool;
predicate guidance_agent_must_use_user_provided_model_first_adjustment() -> bool;
predicate guidance_agent_must_record_and_verify_model_first_baseline() -> bool;
predicate guidance_agent_must_keep_model_first_initial_phase_model_only() -> bool;
predicate guidance_agent_must_show_model_first_diff_and_validation() -> bool;
predicate guidance_agent_must_wait_for_explicit_model_first_confirmation() -> bool;
predicate guidance_agent_must_close_model_first_charter_upward_before_downward_layers() -> bool;
predicate guidance_agent_must_preserve_confirmed_model_meaning_during_charter_closure() -> bool;
predicate guidance_agent_must_return_unclosable_charter_to_model_decision() -> bool;
predicate guidance_agent_must_finish_model_first_validation() -> bool;
predicate guidance_agent_must_keep_charter_as_final_semantic_authority_in_model_first() -> bool;
predicate guidance_agent_must_treat_charter_lock_as_higher_priority_than_change_order() -> bool;
predicate guidance_agent_must_not_treat_change_request_as_implicit_unlock_authorization() -> bool;
predicate guidance_agent_must_only_suggest_changes_to_locked_charter() -> bool;
predicate guidance_agent_must_require_explicit_user_unlock_request() -> bool;
predicate guidance_agent_must_use_charter_lock_tool_for_authorized_unlock() -> bool;
predicate guidance_agent_must_not_bypass_charter_lock_protection() -> bool;
predicate guidance_agent_must_relock_authorized_charter_before_task_completion() -> bool;
predicate guidance_agent_must_not_reuse_interrupted_unlocked_state_as_authorization() -> bool;
predicate guidance_agent_must_not_weaken_charter_lock_mechanism_without_explicit_request() -> bool;
predicate guidance_agent_must_use_structured_boundary_inventory() -> bool;
predicate guidance_agent_must_not_treat_trimmed_as_unimplemented() -> bool;
predicate guidance_agent_must_close_boundary_with_facts_tests_and_archive() -> bool;
predicate guidance_agent_must_keep_linux_checkpoint_mapping_read_only() -> bool;
predicate guidance_agent_must_make_checkpoint_artifact_checks_read_only() -> bool;
predicate guidance_agent_must_keep_linux_checkpoint_coverage_mapping_only() -> bool;
predicate guidance_user_boot_codegen_must_read_user_boot_specs_first() -> bool;
predicate guidance_user_boot_codegen_must_use_consensus_object_names() -> bool;
predicate guidance_user_boot_codegen_must_not_create_test_only_or_transitional_objects() -> bool;
predicate guidance_cpu_group_codegen_must_read_cpu_model_specs_first() -> bool;
predicate guidance_cpu_group_codegen_must_generate_unified_cpu_instances() -> bool;
predicate guidance_cpu_group_codegen_must_preserve_logical_id_index_view() -> bool;
predicate guidance_cpu_group_codegen_must_not_create_live_ap_current_cpu_early() -> bool;
predicate guidance_cpu_group_codegen_must_use_ap_entry_phases_before_live_ap_current_cpu() -> bool;
predicate guidance_cpu_group_codegen_must_generate_index_and_set_tests() -> bool;

type GenerationAgentWorkflow {
    invariant {
        /*
         * Step 1: read principles and concrete requirements.
         *
         * Before generating or modifying an artifact, the AI/code generator
         * must read the applicable generation principles, the concrete core
         * Charter/Model/Coding requirements, and any applicable supporting
         * Compose/Testing requirements for the requested target.
         */
        guidance_agent_must_read_generation_principles_before_implementation();
        guidance_agent_must_read_concrete_spec_requirements_before_implementation();

        /*
         * Step 2: implement.
         *
         * Implementation may start only after the relevant principles and
         * concrete requirements have been read and understood.
         */
        guidance_agent_must_implement_only_after_reading_requirements();

        /*
         * Step 3: check the result.
         *
         * After implementation, the AI/code generator must check that the
         * generated result still satisfies the applicable principles and
         * concrete requirements. Failing this self-check means the
         * implementation is not complete.
         */
        guidance_agent_must_check_generated_result_against_principles_after_implementation();
        guidance_agent_must_check_generated_result_against_concrete_requirements_after_implementation();
    }
}

type CoreSemanticRefinementAuthority {
    invariant {
        /*
         * The only core semantic refinement chain is Charter -> Model ->
         * Coding -> Impl. Charter states design intent in human-readable
         * language and remains the top semantic authority. Model may make
         * that intent precise and checkable, but must not change its meaning.
         * Coding may constrain the mapping to data structures, algorithms,
         * memory layout, registers and other code-level choices, but must not
         * redefine Charter or Model. Impl only realizes their combined effect
         * and cannot be used to reinterpret them from current code behavior.
         */
        guidance_agent_must_treat_charter_as_human_readable_top_semantic_authority();
        guidance_agent_must_make_model_only_formalize_and_refine_charter();
        guidance_agent_must_make_coding_only_constrain_model_to_code_mapping();
        guidance_agent_must_make_impl_only_realize_charter_model_coding_effect();

        /*
         * Every lower core layer preserves all applicable higher constraints,
         * not merely the adjacent layer. A lower layer may choose a detail
         * only when every applicable higher layer leaves it unconstrained. If
         * that choice creates observable behavior, an interface or a specified
         * object boundary, the contract must first be promoted to the
         * appropriate core specification layer and closed downward again.
         */
        guidance_lower_core_layer_must_preserve_all_applicable_higher_constraints();
        guidance_lower_core_layer_may_choose_only_when_all_higher_layers_are_unconstrained();
        guidance_agent_must_promote_new_observable_contract_to_appropriate_core_specification();

        /*
         * Guidance governs workflow, Compose governs applicable assembly, and
         * Testing governs validation. These are supporting responsibilities,
         * not members of the core semantic authority chain, and none may
         * override Charter, Model, Coding or the conforming effect of Impl.
         */
        guidance_agent_must_treat_guidance_compose_testing_as_supporting_responsibilities();
        guidance_supporting_responsibilities_must_not_override_core_semantics();
    }
}

type RepositoryChangeWorkflow {
    invariant {
        /*
         * Problem fixes must be diagnosis-driven. The agent must not replace
         * a missing diagnosis with a guessed patch; it must first collect
         * reproducible observations and narrow the failing boundary until the
         * proposed change follows from the evidence.
         */
        guidance_agent_must_not_guess_fix_without_reproducible_localization();

        /*
         * Behavior, interface, object-boundary and Linux differential
         * semantics changes must follow one of the named change workflows.
         * Applicable core specifications must be updated before implementation.
         */
        guidance_agent_must_update_spec_before_behavior_implementation();

        /*
         * Focused tests are allowed while locating a problem, but every code
         * change must finish with the repository root make test regression
         * gate. A focused run is not a substitute for the final regression.
         */
        guidance_agent_must_run_make_test_after_code_change();

        /*
         * Cross-layer closure is reviewable and uncommitted until complete.
         * Each core layer must have an explicit closure outcome, and every
         * applicable supporting responsibility must be reviewed. A layer or
         * branch changes only when its semantics, mapping, composition or
         * acceptance contract is affected. A commit always requires explicit
         * user authorization.
         */
        guidance_agent_must_keep_temporary_layer_inconsistency_uncommitted();
        guidance_agent_must_review_each_applicable_layer_and_change_only_affected_layers();
        guidance_agent_must_require_explicit_commit_authorization();

        /*
         * Deferred/trimmed governance is inventory-driven.  Generators must
         * use the stable structured boundary ID, controlled category,
         * evidence and acceptance/revisit condition from the model.  They
         * must not create or preserve free-text deferred backlogs.
         */
        guidance_agent_must_use_structured_boundary_inventory();

        /*
         * A trimmed boundary is a proved configuration, architecture,
         * reference-input or compile-time no-op fact.  It must not be
         * interpreted as an unimplemented feature or emitted as a runtime
         * stub merely because the reference path is absent.
         */
        guidance_agent_must_not_treat_trimmed_as_unimplemented();

        /*
         * Closing a deferred boundary is one atomic change: remove the
         * active record, add the formal facts and tests that satisfy its
         * close_when condition, and archive the retired ID and evidence.
         * Retired IDs are never reused.
         */
        guidance_agent_must_close_boundary_with_facts_tests_and_archive();

        /*
         * Linux checkpoint mapping is read-only:
         *
         * When the requested task is a Linux checkpoint alignment mapping
         * stage, the agent must keep it as inventory/cross-reference work:
         * consume the arceos_ex checkpoint inventory, read the reference
         * Linux source tree, and emit reviewable mapping artifacts. It must
         * not silently turn the task into Linux instrumentation, runtime
         * collection, checkpoint-handler changes or behavioral changes.
         * The mapping parser may resolve C functions, SYSCALL_DEFINE* syscall
         * wrappers and assembly symbols/labels. Conditional arch/config ABI
         * wrappers must be handled conservatively: map to a shared helper or
         * keep the checkpoint unmapped, with confidence and notes explaining
         * the conditional layer.
         * Architecture-specific entry mapping, such as RISC-V64 head.S and
         * setup_vm() alignment, must keep that architecture scope explicit
         * and must classify uncertain object boundaries as range or unmapped.
         */
        guidance_agent_must_keep_linux_checkpoint_mapping_read_only();

        /*
         * Linux checkpoint coverage is mapping-only:
         *
         * When the requested task summarizes Linux checkpoint mapping
         * coverage, the agent must treat it as a review artifact derived only
         * from the tracked mapping JSON. It may aggregate counts by mapping
         * kind, confidence, Linux file and unmapped checkpoint family, but it
         * must not reinterpret mappings, read or edit Linux sources, add
         * runtime collection, or change checkpoint handlers.
         */
        guidance_agent_must_keep_linux_checkpoint_coverage_mapping_only();

        /*
         * Checkpoint artifact checks are drift detectors:
         *
         * When adding or running checkpoint inventory, Linux mapping or Linux
         * mapping coverage validation, check mode must regenerate expected
         * artifacts in memory, compare them with the tracked review artifacts,
         * and fail on drift. It must not rewrite outputs, edit Linux sources,
         * add runtime collection, or change checkpoint handlers while
         * validating.
         */
        guidance_agent_must_make_checkpoint_artifact_checks_read_only();
    }
}

type CharterFirstChangeWorkflow {
    invariant {
        /*
         * Charter-first is the default change workflow. The user or charter
         * establishes design intent, then the core semantic layers close
         * top-down in this order: Charter, Model, Coding, Impl. Applicable
         * Compose assembly and Testing validation are reviewed as supporting
         * responsibilities rather than inserted into that authority chain.
         */
        guidance_agent_must_default_to_charter_first();
        guidance_agent_must_follow_charter_first_layer_order();

        /*
         * Before changes, identify the highest affected core layer. Record a
         * changed or reviewed-and-unchanged closure outcome for Charter,
         * Model, Coding and Impl in that order, without requiring a new fixed
         * declaration artifact.
         */
        guidance_agent_must_identify_highest_affected_core_layer_before_changes();
        guidance_agent_must_record_each_core_layer_closure_outcome();

        /*
         * Cross-layer conflicts are resolved by the same core authority order.
         * Existing implementation, tests, roadmaps and Compose do not override
         * an applicable higher core layer. If Charter intent is unclear or the
         * target conflicts with it, stop downward closure and return to a
         * Charter decision.
         */
        guidance_agent_must_resolve_conflicts_by_charter_first_authority();
        guidance_agent_must_not_use_implementation_tests_roadmap_or_compose_to_override_higher_core_layers();
        guidance_agent_must_stop_for_charter_decision_when_intent_is_unclear_or_conflicting();
    }
}

type CharterLockWorkflow {
    invariant {
        /*
         * A lock recorded in the root charter lock manifest takes precedence
         * over the charter-first editing order.  Charter-first establishes
         * authority between layers; it does not implicitly authorize an agent
         * to edit a locked charter.  An ordinary request to change behavior or
         * follow charter-first therefore leaves the lock in force, and the
         * agent may only propose changes to that charter.
         */
        guidance_agent_must_treat_charter_lock_as_higher_priority_than_change_order();
        guidance_agent_must_not_treat_change_request_as_implicit_unlock_authorization();
        guidance_agent_must_only_suggest_changes_to_locked_charter();

        /*
         * Unlock is a task-scoped user authorization.  Only an explicit user
         * request to unlock the named file permits the agent to invoke the
         * repository charter-lock tool.  The agent must not chmod the target,
         * edit the manifest/hash, remove the visible notice, or otherwise
         * bypass the gate itself.
         */
        guidance_agent_must_require_explicit_user_unlock_request();
        guidance_agent_must_use_charter_lock_tool_for_authorized_unlock();
        guidance_agent_must_not_bypass_charter_lock_protection();

        /*
         * An authorized edit must finish by invoking the tool to refresh the
         * hash and restore the locked, read-only state.  If a prior task was
         * interrupted while the manifest said unlocked, that residue is a
         * failing state to repair or report; it is never authorization for a
         * later agent to continue editing.
         */
        guidance_agent_must_relock_authorized_charter_before_task_completion();
        guidance_agent_must_not_reuse_interrupted_unlocked_state_as_authorization();

        /*
         * The manifest, lock tool, visible notice, guidance rules and build
         * gate are part of the protection boundary.  An agent must not weaken
         * or edit them for the purpose of bypassing a lock unless the user
         * explicitly asks to change the protection mechanism itself.
         */
        guidance_agent_must_not_weaken_charter_lock_mechanism_without_explicit_request();
    }
}

type ModelFirstChangeWorkflow {
    invariant {
        /*
         * Model-first is selected only when the user explicitly declares the
         * round model-first and supplies the proposed model adjustment. The
         * agent must not choose the scope or behavior slice itself. This is an
         * exception to modification order only; Charter remains the final top
         * semantic authority.
         */
        guidance_agent_must_require_explicit_user_trigger_for_model_first();
        guidance_agent_must_use_user_provided_model_first_adjustment();
        guidance_agent_must_keep_charter_as_final_semantic_authority_in_model_first();

        /*
         * Before editing, record the baseline commit, worktree state, relevant
         * behavior and tests, and confirm a repository-root make test. The
         * initial phase then changes only model, runs focused validation, and
         * presents the actual diff and results to the user.
         */
        guidance_agent_must_record_and_verify_model_first_baseline();
        guidance_agent_must_keep_model_first_initial_phase_model_only();
        guidance_agent_must_show_model_first_diff_and_validation();

        /*
         * No charter, coding, compose, implementation or testing change may
         * begin until the user explicitly confirms the model adjustment.
         */
        guidance_agent_must_wait_for_explicit_model_first_confirmation();

        /*
         * After confirmation, close Charter upward without changing the
         * confirmed Model meaning, ensure Model conforms to Charter, then close
         * Coding and Impl downward. Applicable Compose and Testing are reviewed
         * as supporting responsibilities rather than semantic authorities.
         */
        guidance_agent_must_close_model_first_charter_upward_before_downward_layers();
        guidance_agent_must_preserve_confirmed_model_meaning_during_charter_closure();

        /*
         * If charter cannot close without changing the confirmed model
         * meaning, stop and return to the model decision. Otherwise finish all
         * applicable focused, specification, diff and repository-root tests.
         */
        guidance_agent_must_return_unclosable_charter_to_model_decision();
        guidance_agent_must_finish_model_first_validation();
    }
}

type UserBootGenerationWorkflow {
    invariant {
        /*
         * Before generating code for the first user-mode program path, the
         * generator must read the user boot model and the concrete coding
         * constraints that define UserBootPayload, ElfObject,
         * UserAddressSpace, UserStack, UserTrapFrame, SyscallException and
         * SyscallTable.
         */
        guidance_user_boot_codegen_must_read_user_boot_specs_first();

        /*
         * Generated code must use the agreed object names and boundaries:
         * UserBootPayload, ElfObject, UserAddressSpace, UserStack,
         * UserTrapFrame, SyscallException and SyscallTable. It must not
         * resurrect superseded names such as SyscallDispatcher, ElfLoader,
         * ExecCore or MmStruct for the first user-mode hello slice.
         */
        guidance_user_boot_codegen_must_use_consensus_object_names();

        /*
         * The user boot path must be generated from model/coding semantics,
         * not from ad hoc test helpers. A generator must not add test-only
         * object APIs, fake partition objects for a whole-disk ext2 image, or
         * transitional loader objects that are absent from the model.
         */
        guidance_user_boot_codegen_must_not_create_test_only_or_transitional_objects();
    }
}

type CpuGroupGenerationWorkflow {
    invariant {
        /*
         * Before generating CPU/CpuGroup implementation or tests, the
         * generator must read the charter CPU/CpuGroup type note,
         * SEM-CURRENT-CPU-MODEL-001, the BootInitFlow.Preset/entry-successor model
         * facts, and the arceos_ex coding constraints for unified CPU
         * instances and logical-id indexing.
         */
        guidance_cpu_group_codegen_must_read_cpu_model_specs_first();

        /*
         * Generated code must model one reusable CPU type with one instance
         * per logical CPU. BootCPU is logical-id 0 with a bootstrap role; it
         * is not a separate CPU type.
         */
        guidance_cpu_group_codegen_must_generate_unified_cpu_instances();

        /*
         * Generated CpuGroup code must expose CpuGroup.cpus[logical_id] as the
         * stable CPU reference view, and possible/present/online membership as
         * set views over those references.
         */
        guidance_cpu_group_codegen_must_preserve_logical_id_index_view();

        /*
         * Possible/present secondary CPU facts do not imply a live AP
         * CurrentCPU. Generated code must not instantiate AP CurrentCPU or
         * CPU-local control chains until the AP secondary entry path is
         * generated.
         */
        guidance_cpu_group_codegen_must_not_create_live_ap_current_cpu_early();

        /*
         * AP secondary entry is an explicit phase boundary:
         *
         * When AP current/task/stack facts are generated, they must be tied to
         * ApEntryPreludePhase/ApSmpCallinPhase/ApOnlineIdlePhase facts rather
         * than inferred from CpuGroup possible/present membership or BP HSM
         * request issuance alone.
         */
        guidance_cpu_group_codegen_must_use_ap_entry_phases_before_live_ap_current_cpu();

        /*
         * The generated validation surface must include checks for boot CPU
         * index 0, CpuRef target binding, possible/present/online set facts,
         * secondary not-online facts and unique logical-id/hartid boundaries.
         */
        guidance_cpu_group_codegen_must_generate_index_and_set_tests();
    }
}
