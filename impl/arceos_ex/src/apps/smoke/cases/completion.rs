use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    objects::{
        completion::{Completion, CompletionExtState},
        state::State,
    },
};

pub fn run() -> SmokeResult {
    // TODO: add two-task scenarios once task creation/scheduling test APIs are
    // sufficient. The current TypeBehavior smoke only exercises local
    // completion state transitions and wait-queue effects.
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SetupEnableScenario::new());
    suite.scenario(&mut PresetSetupScenario::new());
    suite.scenario(&mut CompleteWaitScenario::new());
    suite.scenario(&mut CompleteAllReinitScenario::new());
    suite.scenario(&mut PendingFailureScenario::new());
    suite.scenario(&mut UnpreparedOperationsScenario::new());
    suite.scenario(&mut IllegalLifecycleScenario::new());
    suite.result()
}

struct CompletionFixture {
    completion: Completion,
}

impl CompletionFixture {
    fn new() -> Self {
        Self {
            completion: Completion::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("setup", self.completion.setup());
    }

    fn setup_online(&mut self, assertions: &mut SmokeAssertions) {
        self.setup_ready(assertions);
        if !assertions.failed() {
            assertions.assert_ok("enable", self.completion.enable());
        }
    }

    fn assert_ready_pending(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("state ready", self.completion.state() == State::Ready);
        assertions.assert(
            "ext pending",
            self.completion.ext_state() == CompletionExtState::Pending,
        );
        assertions.assert("pending", self.completion.pending());
        assertions.assert("done zero", self.completion.done_count() == 0);
        assertions.assert(
            "wait queue ready",
            self.completion.wait_queue().state() == State::Ready,
        );
        assertions.assert("storage bound", self.completion.storage_bound());
        assertions.assert("owns wait queue", self.completion.owns_wait_queue());
    }

    fn assert_online(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("state online", self.completion.state() == State::Online);
        assertions.assert("handle published", self.completion.handle_published());
    }
}

struct SetupEnableScenario {
    fixture: CompletionFixture,
}

impl SetupEnableScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for SetupEnableScenario {
    fn name(&self) -> &'static str {
        "completion.setup_enable"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.assert_ready_pending(assertions);
        assertions.assert_ok("enable", self.fixture.completion.enable());
        self.fixture.assert_online(assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct PresetSetupScenario {
    fixture: CompletionFixture,
}

impl PresetSetupScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for PresetSetupScenario {
    fn name(&self) -> &'static str {
        "completion.preset_setup"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("preset", self.fixture.completion.preset());
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "prepared",
            self.fixture.completion.state() == State::Prepared,
        );
        assertions.assert("storage bound", self.fixture.completion.storage_bound());
        assertions.assert_ok("setup after preset", self.fixture.completion.setup());
        self.fixture.assert_ready_pending(assertions);
        assertions.assert_ok("enable", self.fixture.completion.enable());
        self.fixture.assert_online(assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct CompleteWaitScenario {
    fixture: CompletionFixture,
}

impl CompleteWaitScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for CompleteWaitScenario {
    fn name(&self) -> &'static str {
        "completion.complete_wait"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_online(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("complete", self.fixture.completion.complete());
        assertions.assert(
            "completed",
            self.fixture.completion.ext_state() == CompletionExtState::Completed,
        );
        assertions.assert("done count one", self.fixture.completion.done_count() == 1);
        assertions.assert(
            "complete committed",
            self.fixture.completion.complete_committed(),
        );
        assertions.assert("token available", self.fixture.completion.token_available());
        assertions.assert("wake one", self.fixture.completion.wakes_one_waiter());
        assertions.assert("no wake all", !self.fixture.completion.wakes_all_waiters());
        assertions.assert("done observed value", self.fixture.completion.done() == 1);
        assertions.assert("done observed", self.fixture.completion.done_observed());

        assertions.assert_ok("wait consumes token", self.fixture.completion.wait());
        assertions.assert("pending after wait", self.fixture.completion.pending());
        assertions.assert("done consumed", self.fixture.completion.done_count() == 0);
        assertions.assert("waiter enqueued", self.fixture.completion.waiter_enqueued());
        assertions.assert("waiter finished", self.fixture.completion.waiter_finished());
        assertions.assert(
            "queue waiter enqueued",
            self.fixture.completion.wait_queue().waiter_enqueued(),
        );
        assertions.assert(
            "queue waiter finished",
            self.fixture.completion.wait_queue().waiter_finished(),
        );
        assertions.assert_fail("try wait without token", self.fixture.completion.try_wait());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct CompleteAllReinitScenario {
    fixture: CompletionFixture,
}

impl CompleteAllReinitScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for CompleteAllReinitScenario {
    fn name(&self) -> &'static str {
        "completion.complete_all_reinit"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_online(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("complete all", self.fixture.completion.complete_all());
        assertions.assert("completed all", self.fixture.completion.completed_all());
        assertions.assert("completed", self.fixture.completion.completed());
        assertions.assert("token available", self.fixture.completion.token_available());
        assertions.assert(
            "complete all committed",
            self.fixture.completion.complete_all_committed(),
        );
        assertions.assert("wake all", self.fixture.completion.wakes_all_waiters());
        assertions.assert_ok("try wait completed all", self.fixture.completion.try_wait());
        assertions.assert(
            "still completed all",
            self.fixture.completion.completed_all(),
        );
        assertions.assert_ok("wait completed all", self.fixture.completion.wait());
        assertions.assert(
            "still completed all after wait",
            self.fixture.completion.completed_all(),
        );

        assertions.assert_ok("reinit", self.fixture.completion.reinit());
        assertions.assert("pending after reinit", self.fixture.completion.pending());
        assertions.assert(
            "done zero after reinit",
            self.fixture.completion.done_count() == 0,
        );
        assertions.assert(
            "complete flag cleared",
            !self.fixture.completion.complete_committed(),
        );
        assertions.assert(
            "complete all flag cleared",
            !self.fixture.completion.complete_all_committed(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct PendingFailureScenario {
    fixture: CompletionFixture,
}

impl PendingFailureScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for PendingFailureScenario {
    fn name(&self) -> &'static str {
        "completion.pending_failure"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_online(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("try wait pending", self.fixture.completion.try_wait());
        assertions.assert_fail("wait pending blocks", self.fixture.completion.wait());
        assertions.assert("still pending", self.fixture.completion.pending());
        assertions.assert("waiter enqueued", self.fixture.completion.waiter_enqueued());
        assertions.assert("waiter finished", self.fixture.completion.waiter_finished());
        assertions.assert(
            "queue waiter enqueued",
            self.fixture.completion.wait_queue().waiter_enqueued(),
        );
        assertions.assert(
            "queue waiter finished",
            self.fixture.completion.wait_queue().waiter_finished(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct UnpreparedOperationsScenario {
    fixture: CompletionFixture,
}

impl UnpreparedOperationsScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for UnpreparedOperationsScenario {
    fn name(&self) -> &'static str {
        "completion.unprepared_operations"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("complete before setup", self.fixture.completion.complete());
        assertions.assert_fail(
            "complete all before setup",
            self.fixture.completion.complete_all(),
        );
        assertions.assert_fail("wait before setup", self.fixture.completion.wait());
        assertions.assert_fail("try wait before setup", self.fixture.completion.try_wait());
        assertions.assert_fail("reinit before setup", self.fixture.completion.reinit());
        assertions.assert("state base", self.fixture.completion.state() == State::Base);
        assertions.assert("still pending", self.fixture.completion.pending());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct IllegalLifecycleScenario {
    fixture: CompletionFixture,
}

impl IllegalLifecycleScenario {
    fn new() -> Self {
        Self {
            fixture: CompletionFixture::new(),
        }
    }
}

impl SmokeScenario for IllegalLifecycleScenario {
    fn name(&self) -> &'static str {
        "completion.illegal_lifecycle"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_online(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("second setup", self.fixture.completion.setup());
        assertions.assert_fail("second enable", self.fixture.completion.enable());
        assertions.assert_fail("preset after online", self.fixture.completion.preset());
        self.fixture.assert_online(assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
