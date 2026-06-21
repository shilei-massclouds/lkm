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
predicate guidance_user_boot_codegen_must_read_user_boot_specs_first() -> bool;
predicate guidance_user_boot_codegen_must_use_consensus_object_names() -> bool;
predicate guidance_user_boot_codegen_must_not_create_test_only_or_transitional_objects() -> bool;

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
