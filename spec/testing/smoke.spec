/*
 * Smoke Test Generation Specification
 *
 * These predicates name mandatory constraints for generating smoke tests from
 * model and coding specifications. Human-facing explanations live in
 * README.md; this file is the formal entry consumed by tools and AI agents.
 */

predicate testing_smoke_category_type_behavior_defined() -> bool;
predicate testing_smoke_category_kernel_environment_bound_defined() -> bool;

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
predicate testing_smoke_must_keep_kernel_environment_bound_explicit() -> bool;
predicate testing_smoke_must_not_register_type_behavior_as_kunit_by_default() -> bool;
predicate testing_smoke_must_record_kunit_exceptions() -> bool;

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
         * Scenarios must include successful event/action paths when all
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
         * Explicit environment-bound target:
         *
         * KernelEnvironmentBound cases must make clear that they test a live
         * production object, checkpoint fact or startup-stage fact.
         */
        testing_smoke_must_keep_kernel_environment_bound_explicit();

        /*
         * KUnit default:
         *
         * TypeBehavior smoke cases must not be registered as checkpoint KUnit
         * cases by default, because checkpoint KUnit is for live environment
         * facts rather than ordinary reusable type behavior.
         */
        testing_smoke_must_not_register_type_behavior_as_kunit_by_default();

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
