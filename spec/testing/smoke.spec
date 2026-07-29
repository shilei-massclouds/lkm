/*
 * Smoke Test Generation Specification
 *
 * These predicates name mandatory constraints for generating smoke tests from
 * model and coding specifications. Human-facing explanations live in
 * README.md; this file is the formal entry consumed by tools and AI agents.
 */

predicate testing_smoke_category_type_behavior_defined() -> bool;
predicate testing_smoke_category_object_api_behavior_defined() -> bool;
predicate testing_smoke_category_kernel_environment_bound_defined() -> bool;

predicate testing_tests_must_not_add_test_only_subject_api() -> bool;
predicate testing_tests_must_not_modify_functional_api_for_tests() -> bool;
predicate testing_kunit_checkpoint_must_be_read_only_observer_by_default() -> bool;
predicate testing_kunit_checkpoint_must_not_add_test_only_subject_api() -> bool;
predicate testing_action_level_kunit_must_record_capability() -> bool;
predicate testing_checkpoint_handler_run_must_have_read_only_context_and_sink() -> bool;
predicate testing_checkpoint_handler_run_must_not_have_write_variant() -> bool;
predicate testing_new_smoke_tests_must_not_join_kunit_by_default() -> bool;
predicate testing_new_kunit_tests_must_not_join_smoke_by_default() -> bool;
predicate testing_cross_registration_must_record_reason() -> bool;
predicate testing_smoke_must_classify_target_before_generation() -> bool;
predicate testing_smoke_must_generate_scenarios() -> bool;
predicate testing_smoke_must_cover_normal_paths() -> bool;
predicate testing_smoke_must_cover_invalid_state_paths() -> bool;
predicate testing_smoke_must_cover_repeated_operation_paths() -> bool;
predicate testing_smoke_must_cover_resource_boundary_paths() -> bool;
predicate testing_smoke_must_cover_ordering_sensitive_paths() -> bool;
predicate testing_smoke_must_cover_missing_dependency_paths() -> bool;
predicate testing_smoke_must_use_setup_run_teardown() -> bool;
predicate testing_smoke_must_restore_environment_in_teardown() -> bool;
predicate testing_smoke_must_isolate_scenarios() -> bool;
predicate testing_smoke_must_use_assertions_as_oracle() -> bool;
predicate testing_smoke_must_not_use_logs_as_oracle() -> bool;
predicate testing_smoke_must_support_assert_fail() -> bool;
predicate testing_smoke_must_support_assert_timeout() -> bool;
predicate testing_smoke_must_not_halt_on_assertion_failure_by_default() -> bool;
predicate testing_smoke_must_run_teardown_after_assertion_failure() -> bool;
predicate testing_smoke_must_keep_type_behavior_local() -> bool;
predicate testing_smoke_must_not_bind_type_behavior_to_production_singleton() -> bool;
predicate testing_smoke_must_exercise_formal_object_api() -> bool;
predicate testing_smoke_must_not_add_test_only_subject_api() -> bool;
predicate testing_smoke_must_keep_kernel_environment_bound_explicit() -> bool;
predicate testing_smoke_must_not_register_type_behavior_as_kunit_by_default() -> bool;
predicate testing_smoke_must_not_register_object_api_behavior_as_kunit_by_default() -> bool;
predicate testing_smoke_must_record_kunit_exceptions() -> bool;
predicate testing_cpu_group_smoke_must_be_kernel_environment_bound() -> bool;
predicate testing_cpu_group_smoke_must_cover_boot_cpu_index_zero() -> bool;
predicate testing_cpu_group_smoke_must_cover_cpu_ref_targets() -> bool;
predicate testing_cpu_group_smoke_must_cover_possible_present_online_sets() -> bool;
predicate testing_cpu_group_smoke_must_cover_secondary_not_online_before_bringup() -> bool;
predicate testing_cpu_group_smoke_must_cover_unique_logical_ids_and_hartids() -> bool;

type TestImplementationPrinciples {
    invariant {
        /*
         * No test-only subject API:
         *
         * Tests must not add test_* or otherwise test-only APIs to the subject
         * under test. If an action needs a callable implementation boundary,
         * that boundary must be modeled and implemented as a formal object API.
         */
        testing_tests_must_not_add_test_only_subject_api();

        /*
         * Do not bend functional APIs for tests:
         *
         * Tests must not change existing functional API signatures,
         * visibility, semantics or error behavior merely to make a test easier
         * to write. API changes must be justified by model/coding semantics.
         */
        testing_tests_must_not_modify_functional_api_for_tests();

        /*
         * Checkpoint KUnit is read-only by default:
         *
         * A checkpoint/KUnit handler observes live checkpoint facts. It must
         * not drive probe, IRQ, ring, completion or other production actions,
         * mutate object state, or synthesize events unless the handler is
         * explicitly specified as an action-level probe with a minimal named
         * capability.
         */
        testing_kunit_checkpoint_must_be_read_only_observer_by_default();

        /*
         * HandlerRun type boundary:
         *
         * MUST: checkpoint KUnit handlers must use a read-only Context plus an
         * explicit Sink capability. The ordinary handler enum must not expose
         * a Write variant, &mut Context, or an equivalent mutable Context
         * escape hatch.
         */
        testing_checkpoint_handler_run_must_have_read_only_context_and_sink();
        testing_checkpoint_handler_run_must_not_have_write_variant();

        /*
         * KUnit still cannot add subject APIs:
         *
         * Checkpoint/KUnit coverage must not cause test-only methods,
         * snapshot/reset hooks, fake-completion hooks or mock-injection
         * methods to be added to the subject object. If the capability is real,
         * model and implement it as a formal object API or probe object.
         */
        testing_kunit_checkpoint_must_not_add_test_only_subject_api();

        /*
         * Explicit action-level KUnit:
         *
         * If a checkpoint KUnit handler must execute a mutating action, the
         * testing and coding specs must name the action-level boundary,
         * allowed capability and cleanup expectations. Silence means observer
         * only.
         */
        testing_action_level_kunit_must_record_capability();

        /*
         * Smoke registration default:
         *
         * A newly added smoke test must remain registered only in smoke by
         * default. It must not be added to checkpoint KUnit unless a recorded
         * reason names the checkpoint fact that requires KUnit coverage.
         */
        testing_new_smoke_tests_must_not_join_kunit_by_default();

        /*
         * KUnit registration default:
         *
         * A newly added KUnit/checkpoint test must remain registered only in
         * KUnit by default. It must not be added to smoke unless a recorded
         * reason names the payload or end-to-end observable behavior that
         * requires smoke coverage.
         */
        testing_new_kunit_tests_must_not_join_smoke_by_default();

        /*
         * Explicit cross-registration:
         *
         * Any test shared between smoke and KUnit must record why both
         * execution carriers are required. Silence means no cross-registration.
         */
        testing_cross_registration_must_record_reason();
    }
}

type SmokeTestCategories {
    invariant {
        /*
         * TypeBehavior:
         *
         * A reusable abstract type whose behavior can be tested through local
         * subject instances and minimal dependency instances. RawSpinLock and
         * Completion are current examples.
         */
        testing_smoke_category_type_behavior_defined();

        /*
         * ObjectApiBehavior:
         *
         * A formal object API or action boundary whose contract can be tested
         * with a local subject object and read-only live prerequisites.
         * TaskCreationCore.copy_process() and CurrentRunQueueRef/RunQueue
         * enqueue/pick actions are current examples.
         */
        testing_smoke_category_object_api_behavior_defined();

        /*
         * KernelEnvironmentBound:
         *
         * A production kernel object or fact set that is meaningful only in the
         * live startup environment. MemBlock, CpuGroup and Scheduler are
         * current examples.
         */
        testing_smoke_category_kernel_environment_bound_defined();
    }
}

type SmokeTestGenerationMust {
    invariant {
        /*
         * Classify before generation:
         *
         * A generated smoke case must first classify its target as
         * TypeBehavior, KernelEnvironmentBound or a later formal category.
         */
        testing_smoke_must_classify_target_before_generation();

        /*
         * Scenario generation:
         *
         * A smoke case must be generated as one or more explicit scenarios,
         * not as an unclassified sequence of convenient calls.
         */
        testing_smoke_must_generate_scenarios();

        /*
         * Normal paths:
         *
         * Scenarios must include successful transition/action paths when all
         * modeled preconditions are satisfied.
         */
        testing_smoke_must_cover_normal_paths();

        /*
         * Invalid-state paths:
         *
         * Scenarios must include calls where the subject or dependency object
         * is in a state that the model does not allow.
         */
        testing_smoke_must_cover_invalid_state_paths();

        /*
         * Repeated operations:
         *
         * Scenarios must cover repeated operations such as repeated setup,
         * acquire, release, wait, consume or reinit when those operations are
         * exposed by the type or object under test.
         */
        testing_smoke_must_cover_repeated_operation_paths();

        /*
         * Resource boundaries:
         *
         * Scenarios must cover modeled count, depth, queue, token, capacity,
         * empty and full boundaries when such resources are present.
         */
        testing_smoke_must_cover_resource_boundary_paths();

        /*
         * Ordering-sensitive paths:
         *
         * Scenarios must observe or indirectly assert ordering constraints
         * derived from drives, within or equivalent formal sequencing rules.
         */
        testing_smoke_must_cover_ordering_sensitive_paths();

        /*
         * Missing dependencies:
         *
         * Scenarios must cover absent dependencies, dependency state mismatch
         * or missing prerequisite facts when the formal model exposes them.
         */
        testing_smoke_must_cover_missing_dependency_paths();

        /*
         * Three-part scenario body:
         *
         * Each scenario must have setup, run and teardown sections. setup
         * establishes the pre-state, run executes checks, and teardown restores
         * the environment.
         */
        testing_smoke_must_use_setup_run_teardown();

        /*
         * Environment restoration:
         *
         * teardown must restore every external state snapshot affected by the
         * scenario, including interrupt state, preemption state, held locks,
         * queues, tokens and counters that may affect later scenarios.
         */
        testing_smoke_must_restore_environment_in_teardown();

        /*
         * Scenario isolation:
         *
         * A scenario must not depend on side effects left by a previous
         * scenario. Intentional multi-step sequences belong inside one
         * scenario and must be cleaned by that scenario.
         */
        testing_smoke_must_isolate_scenarios();

        /*
         * Assertion oracle:
         *
         * Smoke tests must decide pass/fail through assertions. Diagnostics are
         * allowed, but a human reading logs must not be the oracle.
         */
        testing_smoke_must_use_assertions_as_oracle();

        /*
         * Logs are diagnostic only:
         *
         * Log output may explain a failure or summarize measured facts, but it
         * must not be the only mechanism that validates the expected behavior.
         */
        testing_smoke_must_not_use_logs_as_oracle();

        /*
         * Expected failure assertions:
         *
         * Generated tests must support assertions that an operation fails, so
         * invalid-state and repeated-operation scenarios are tested directly.
         */
        testing_smoke_must_support_assert_fail();

        /*
         * Timeout assertions:
         *
         * Generated tests must support bounded wait assertions for scenarios
         * that depend on eventual facts. An unmet timeout is a test failure,
         * not an infinite wait.
         */
        testing_smoke_must_support_assert_timeout();

        /*
         * Non-halting failure default:
         *
         * Assertion failure must not halt, panic or spin forever by default.
         * The default behavior is to record failure and return a failed test
         * outcome after required teardown. A strict/debug policy may explicitly
         * upgrade failure to stop.
         */
        testing_smoke_must_not_halt_on_assertion_failure_by_default();

        /*
         * Teardown after failure:
         *
         * Assertion failure must not bypass teardown. Generated tests that
         * modify external environment state must use a runner, guard or
         * equivalent structure that executes teardown on both success and
         * failure paths.
         */
        testing_smoke_must_run_teardown_after_assertion_failure();

        /*
         * Local type behavior:
         *
         * TypeBehavior cases must use local subject instances and minimal
         * dependency instances by default.
         */
        testing_smoke_must_keep_type_behavior_local();

        /*
         * No production singleton binding:
         *
         * TypeBehavior cases must not bind the test target to a production
         * singleton or startup-context field. Live context may be read only as
         * minimal support for preconditions.
         */
        testing_smoke_must_not_bind_type_behavior_to_production_singleton();

        /*
         * Formal object API:
         *
         * ObjectApiBehavior cases must call the same formal object API or
         * action boundary that production code uses. The smoke case may
         * construct a local subject, but the exercised method must remain a
         * real implementation boundary.
         */
        testing_smoke_must_exercise_formal_object_api();

        /*
         * No test-only subject API:
         *
         * Smoke generation must not add test_* or otherwise test-only methods
         * to the subject object. If an action is not externally callable enough
         * to test, the implementation must expose a formal object API instead.
         */
        testing_smoke_must_not_add_test_only_subject_api();

        /*
         * Explicit environment-bound target:
         *
         * KernelEnvironmentBound cases must make clear that they test a live
         * production object, checkpoint fact or startup-stage fact.
         */
        testing_smoke_must_keep_kernel_environment_bound_explicit();

        /*
         * CPU/CpuGroup classification:
         *
         * CpuGroup smoke coverage is KernelEnvironmentBound. It observes the
         * live startup CpuGroup, CPU facts and checkpoint facts; it must not
         * create a fake production CpuGroup or add test-only CPU APIs.
         */
        testing_cpu_group_smoke_must_be_kernel_environment_bound();

        /*
         * CPU index and reference facts:
         *
         * CpuGroup smoke coverage must assert that logical id 0 is the boot
         * CPU entry, that CpuGroup.cpus[0] resolves through BootCPURef, and that
         * the reference targets canonical CpuGroup.cpus[0]. BootCPU is only a
         * role alias and owns no separate state.
         */
        testing_cpu_group_smoke_must_cover_boot_cpu_index_zero();
        testing_cpu_group_smoke_must_cover_cpu_ref_targets();

        /*
         * CPU set views:
         *
         * CpuGroup smoke coverage must assert possible/present/online set
         * views over CPU references. BootCPU must be possible, present and
         * online; secondary CPUs discovered before bringup must be possible
         * and present but not online.
         */
        testing_cpu_group_smoke_must_cover_possible_present_online_sets();
        testing_cpu_group_smoke_must_cover_secondary_not_online_before_bringup();

        /*
         * CPU identity uniqueness:
         *
         * CpuGroup smoke coverage must assert unique logical CPU ids and
         * unique hartids for entries discovered from topology.
         */
        testing_cpu_group_smoke_must_cover_unique_logical_ids_and_hartids();

        /*
         * Per-CPU translation ownership:
         *
         * CpuGroup smoke coverage must observe the live owner and final
         * takeover trace independently for every CPU. The boot CPU reaches
         * SwapperVm from EarlyVm, while APs reach it from TrampolineVm; every
         * trace must record the installed SATP and completed synchronization.
         */
        testing_cpu_group_smoke_must_cover_per_cpu_translation_owner();
        testing_cpu_group_smoke_must_cover_bp_and_ap_takeover_paths();
        testing_cpu_group_smoke_must_cover_translation_takeover_trace();

        /*
         * KUnit default:
         *
         * TypeBehavior smoke cases must not be registered as checkpoint KUnit
         * cases by default, because checkpoint KUnit is for live environment
         * facts rather than ordinary reusable type behavior.
         */
        testing_smoke_must_not_register_type_behavior_as_kunit_by_default();

        /*
         * Object API KUnit default:
         *
         * ObjectApiBehavior smoke cases must not be registered as checkpoint
         * KUnit cases by default. They validate API contracts, not necessarily
         * checkpoint-specific production facts.
         */
        testing_smoke_must_not_register_object_api_behavior_as_kunit_by_default();

        /*
         * KUnit exceptions:
         *
         * If a TypeBehavior case is registered as checkpoint KUnit, the
         * exception must record the live fact that cannot be tested as an app
         * smoke scenario alone.
         */
        testing_smoke_must_record_kunit_exceptions();
    }
}
