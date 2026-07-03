/*
 * AI/code-generator behavior specification.
 *
 * These predicates constrain AI/code-generation behavior before, during and
 * after generating artifacts from specifications. They are intentionally
 * above model, coding, compose and testing semantics.
 */

predicate guidance_agent_must_read_generation_principles_before_implementation() -> bool;
predicate guidance_agent_must_read_concrete_spec_requirements_before_implementation() -> bool;
predicate guidance_agent_must_implement_only_after_reading_requirements() -> bool;
predicate guidance_agent_must_check_generated_result_against_principles_after_implementation() -> bool;
predicate guidance_agent_must_check_generated_result_against_concrete_requirements_after_implementation() -> bool;
predicate guidance_agent_must_not_guess_fix_without_reproducible_localization() -> bool;
predicate guidance_agent_must_update_spec_before_behavior_implementation() -> bool;
predicate guidance_agent_must_run_make_test_after_code_change() -> bool;
predicate guidance_agent_must_keep_linux_checkpoint_mapping_read_only() -> bool;
predicate guidance_user_boot_codegen_must_read_user_boot_specs_first() -> bool;
predicate guidance_user_boot_codegen_must_use_consensus_object_names() -> bool;
predicate guidance_user_boot_codegen_must_not_create_test_only_or_transitional_objects() -> bool;
predicate guidance_cpu_group_codegen_must_read_cpu_model_specs_first() -> bool;
predicate guidance_cpu_group_codegen_must_generate_unified_cpu_instances() -> bool;
predicate guidance_cpu_group_codegen_must_preserve_logical_id_index_view() -> bool;
predicate guidance_cpu_group_codegen_must_not_create_live_ap_current_cpu_early() -> bool;
predicate guidance_cpu_group_codegen_must_generate_index_and_set_tests() -> bool;

type GenerationAgentWorkflow {
    invariant {
        /*
         * Step 1: read principles and concrete requirements.
         *
         * Before generating or modifying an artifact, the AI/code generator
         * must read the applicable generation principles and the concrete
         * model/coding/compose/testing requirements for the requested target.
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
         * semantics changes are spec-first work. The applicable model,
         * coding, testing or guidance specification must be updated before
         * the implementation is changed.
         */
        guidance_agent_must_update_spec_before_behavior_implementation();

        /*
         * Focused tests are allowed while locating a problem, but every code
         * change must finish with the repository root make test regression
         * gate. A focused run is not a substitute for the final regression.
         */
        guidance_agent_must_run_make_test_after_code_change();

        /*
         * Linux checkpoint mapping is read-only:
         *
         * When the requested task is a Linux checkpoint alignment mapping
         * stage, the agent must keep it as inventory/cross-reference work:
         * consume the arceos_ex checkpoint inventory, read the reference
         * Linux source tree, and emit reviewable mapping artifacts. It must
         * not silently turn the task into Linux instrumentation, runtime
         * collection, checkpoint-handler changes or behavioral changes.
         */
        guidance_agent_must_keep_linux_checkpoint_mapping_read_only();
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
         * SEM-CURRENT-CPU-MODEL-001, the entry-prelude/entry-successor model
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
         * Generated CpuGroup code must expose CpuGroup.Cpu[logical_id] as the
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
         * The generated validation surface must include checks for boot CPU
         * index 0, CpuRef target binding, possible/present/online set facts,
         * secondary not-online facts and unique logical-id/hartid boundaries.
         */
        guidance_cpu_group_codegen_must_generate_index_and_set_tests();
    }
}
