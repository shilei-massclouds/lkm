use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    arch::riscv64::csr,
    context::context,
    objects::{
        cpu_control::{LocalInterruptControl, PreemptionControl, RawSpinLock},
        state::State,
    },
};

pub fn run() -> SmokeResult {
    // TODO: add two-task scenarios once task creation/scheduling test APIs are
    // sufficient. The current TypeBehavior smoke only exercises a single-task
    // path with local dependency objects.
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SetupReadyScenario::new());
    suite.scenario(&mut PlainAcquireReleaseScenario::new());
    suite.scenario(&mut IrqSaveRestoreScenario::new(true));
    suite.scenario(&mut IrqSaveRestoreScenario::new(false));
    suite.scenario(&mut UnpreparedOperationsScenario::new());
    suite.scenario(&mut MissingDependencyScenario::new());
    suite.scenario(&mut IllegalOperationsScenario::new());
    suite.result()
}

struct RawSpinLockFixture {
    local_interrupt: LocalInterruptControl,
    preemption: PreemptionControl,
    lock: RawSpinLock,
    restore_irq_enabled: bool,
    external_irq_enabled: bool,
}

impl RawSpinLockFixture {
    fn new(restore_irq_enabled: bool) -> Self {
        Self {
            local_interrupt: LocalInterruptControl::new(),
            preemption: PreemptionControl::new(),
            lock: RawSpinLock::new(),
            restore_irq_enabled,
            external_irq_enabled: csr::supervisor_interrupts_enabled(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert("init task online", ctx.init_task.state() == State::Online);
        assertions.assert_ok("local interrupt setup", self.local_interrupt.setup());
        assertions.assert_ok("preemption setup", self.preemption.setup(&ctx.init_task));
        assertions.assert_ok("lock setup", self.lock.setup());
    }

    fn setup_irq_precondition(&mut self, assertions: &mut SmokeAssertions) {
        if self.restore_irq_enabled {
            assertions.assert_ok("enable irq precondition", self.local_interrupt.enable());
            assertions.assert("irq precondition enabled", self.local_interrupt.enabled());
        } else {
            assertions.assert_ok("disable irq precondition", self.local_interrupt.disable());
            assertions.assert("irq precondition disabled", self.local_interrupt.disabled());
        }
    }

    fn release_if_held(&mut self, assertions: &mut SmokeAssertions) {
        if self.lock.locked() {
            if self.local_interrupt.saved_and_disabled_count()
                != self.local_interrupt.restored_count()
            {
                assertions.assert_ok(
                    "teardown unlock irqrestore",
                    self.lock
                        .unlock_irqrestore(&mut self.local_interrupt, &mut self.preemption),
                );
            } else {
                assertions.assert_ok("teardown release", self.lock.release());
            }
        }
    }

    fn restore_external_irq(&mut self, assertions: &mut SmokeAssertions) {
        if self.local_interrupt.state() != State::Ready {
            return;
        }

        if self.external_irq_enabled {
            if self.local_interrupt.disabled() {
                assertions.assert_ok(
                    "teardown restore local interrupt enabled",
                    self.local_interrupt.enable(),
                );
            }
        } else if self.local_interrupt.enabled() {
            assertions.assert_ok(
                "teardown restore local interrupt disabled",
                self.local_interrupt.disable(),
            );
        }
    }
}

struct SetupReadyScenario {
    fixture: RawSpinLockFixture,
}

impl SetupReadyScenario {
    fn new() -> Self {
        Self {
            fixture: RawSpinLockFixture::new(false),
        }
    }
}

impl SmokeScenario for SetupReadyScenario {
    fn name(&self) -> &'static str {
        "raw_spinlock.setup_ready"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("lock ready", self.fixture.lock.state() == State::Ready);
        assertions.assert("lock initially unlocked", !self.fixture.lock.locked());
        assertions.assert(
            "irq initially disabled",
            self.fixture.local_interrupt.disabled(),
        );
        assertions.assert(
            "preemption initially enabled",
            self.fixture.preemption.enabled(),
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.release_if_held(assertions);
        self.fixture.restore_external_irq(assertions);
    }
}

struct PlainAcquireReleaseScenario {
    fixture: RawSpinLockFixture,
}

impl PlainAcquireReleaseScenario {
    fn new() -> Self {
        Self {
            fixture: RawSpinLockFixture::new(false),
        }
    }
}

impl SmokeScenario for PlainAcquireReleaseScenario {
    fn name(&self) -> &'static str {
        "raw_spinlock.plain_acquire_release"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("plain acquire", self.fixture.lock.acquire());
        assertions.assert("plain lock held", self.fixture.lock.locked());
        assertions.assert(
            "plain acquire count",
            self.fixture.lock.acquired_count() == 1,
        );
        assertions.assert(
            "plain acquire keeps irq state",
            self.fixture.local_interrupt.disabled(),
        );
        assertions.assert(
            "plain acquire keeps preemption state",
            self.fixture.preemption.enabled(),
        );

        assertions.assert_ok("plain release", self.fixture.lock.release());
        assertions.assert("plain lock released", !self.fixture.lock.locked());
        assertions.assert(
            "plain release count",
            self.fixture.lock.released_count() == 1,
        );
        assertions.assert(
            "plain release keeps irq state",
            self.fixture.local_interrupt.disabled(),
        );
        assertions.assert(
            "plain release keeps preemption state",
            self.fixture.preemption.enabled(),
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.release_if_held(assertions);
        self.fixture.restore_external_irq(assertions);
    }
}

struct IrqSaveRestoreScenario {
    fixture: RawSpinLockFixture,
}

impl IrqSaveRestoreScenario {
    fn new(restore_irq_enabled: bool) -> Self {
        Self {
            fixture: RawSpinLockFixture::new(restore_irq_enabled),
        }
    }
}

impl SmokeScenario for IrqSaveRestoreScenario {
    fn name(&self) -> &'static str {
        if self.fixture.restore_irq_enabled {
            "raw_spinlock.irqsave_restore_enabled"
        } else {
            "raw_spinlock.irqsave_restore_disabled"
        }
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
        if !assertions.failed() {
            self.fixture.setup_irq_precondition(assertions);
        }
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok(
            "lock irqsave",
            self.fixture.lock.lock_irqsave(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert("lock held", self.fixture.lock.locked());
        assertions.assert(
            "irqsave count",
            self.fixture.lock.irqsave_entered_count() == 1,
        );
        assertions.assert(
            "saved and disabled count",
            self.fixture.local_interrupt.saved_and_disabled_count() == 1,
        );
        assertions.assert(
            "irq disabled after irqsave",
            self.fixture.local_interrupt.disabled(),
        );
        assertions.assert(
            "preemption disabled after irqsave",
            self.fixture.preemption.disabled(),
        );

        assertions.assert_ok(
            "unlock irqrestore",
            self.fixture.lock.unlock_irqrestore(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert("lock released", !self.fixture.lock.locked());
        assertions.assert(
            "irqrestore count",
            self.fixture.lock.irqrestore_exited_count() == 1,
        );
        assertions.assert(
            "restored count",
            self.fixture.local_interrupt.restored_count() == 1,
        );
        assertions.assert(
            "restore before preemption enable",
            self.fixture
                .lock
                .irqrestore_restored_before_preemption_enabled(),
        );
        assertions.assert(
            "irq restored to saved state",
            self.fixture.local_interrupt.enabled() == self.fixture.restore_irq_enabled,
        );
        assertions.assert(
            "preemption enabled after restore",
            self.fixture.preemption.enabled(),
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.release_if_held(assertions);
        self.fixture.restore_external_irq(assertions);
    }
}

struct UnpreparedOperationsScenario {
    fixture: RawSpinLockFixture,
}

impl UnpreparedOperationsScenario {
    fn new() -> Self {
        Self {
            fixture: RawSpinLockFixture::new(false),
        }
    }
}

impl SmokeScenario for UnpreparedOperationsScenario {
    fn name(&self) -> &'static str {
        "raw_spinlock.unprepared_operations"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "lock before setup",
            self.fixture.lock.lock_irqsave(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert_fail(
            "unlock before setup",
            self.fixture.lock.unlock_irqrestore(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert(
            "lock remains base",
            self.fixture.lock.state() == State::Base,
        );
        assertions.assert("lock remains unlocked", !self.fixture.lock.locked());
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.restore_external_irq(assertions);
    }
}

struct MissingDependencyScenario {
    fixture: RawSpinLockFixture,
}

impl MissingDependencyScenario {
    fn new() -> Self {
        Self {
            fixture: RawSpinLockFixture::new(false),
        }
    }
}

impl SmokeScenario for MissingDependencyScenario {
    fn name(&self) -> &'static str {
        "raw_spinlock.missing_dependency"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("lock setup only", self.fixture.lock.setup());
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "lock with unprepared dependencies",
            self.fixture.lock.lock_irqsave(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert("lock still unlocked", !self.fixture.lock.locked());
        assertions.assert(
            "irqsave not committed",
            self.fixture.lock.irqsave_entered_count() == 0,
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.release_if_held(assertions);
        self.fixture.restore_external_irq(assertions);
    }
}

struct IllegalOperationsScenario {
    fixture: RawSpinLockFixture,
}

impl IllegalOperationsScenario {
    fn new() -> Self {
        Self {
            fixture: RawSpinLockFixture::new(true),
        }
    }
}

impl SmokeScenario for IllegalOperationsScenario {
    fn name(&self) -> &'static str {
        "raw_spinlock.illegal_operations"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
        if !assertions.failed() {
            self.fixture.setup_irq_precondition(assertions);
        }
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("second setup", self.fixture.lock.setup());
        assertions.assert_fail(
            "unlock while unlocked",
            self.fixture.lock.unlock_irqrestore(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert_ok(
            "first lock",
            self.fixture.lock.lock_irqsave(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert_fail(
            "contended lock",
            self.fixture.lock.lock_irqsave(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert("lock still held", self.fixture.lock.locked());
        assertions.assert_ok(
            "first unlock",
            self.fixture.lock.unlock_irqrestore(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert_fail(
            "second unlock",
            self.fixture.lock.unlock_irqrestore(
                &mut self.fixture.local_interrupt,
                &mut self.fixture.preemption,
            ),
        );
        assertions.assert("lock final unlocked", !self.fixture.lock.locked());
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.release_if_held(assertions);
        self.fixture.restore_external_irq(assertions);
    }
}
