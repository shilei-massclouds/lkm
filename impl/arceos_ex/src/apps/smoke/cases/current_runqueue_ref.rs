use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        cpu_control::CurrentTaskRef,
        rest_init::{KERNEL_INIT_PID, KTHREADD_PID},
        scheduler::{BootRunQueue, CurrentRunQueueRef},
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut EmptyQueueScenario::new());
    suite.scenario(&mut KernelInitQueueScenario::new());
    suite.scenario(&mut KthreaddQueueScenario::new());
    suite.scenario(&mut MixedQueueScenario::new());
    suite.scenario(&mut InvalidTaskRefScenario::new());
    suite.result()
}

struct CurrentRunQueueFixture {
    runqueue: BootRunQueue,
}

impl CurrentRunQueueFixture {
    fn new() -> Self {
        Self {
            runqueue: BootRunQueue::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "setup",
            self.runqueue.setup(
                &ctx.cpu_group,
                &ctx.cpu_id_map,
                &ctx.per_cpu_storage,
                ctx.scheduler.default_root_domain(),
            ),
        );
    }

    fn enqueue(
        &mut self,
        task_ref: CurrentTaskRef,
    ) -> Result<(), crate::objects::state::EventError> {
        self.runqueue
            .enqueue_task_ref(CurrentRunQueueRef::BootRunQueue, task_ref)
    }

    fn pick_next(&self) -> Result<CurrentTaskRef, crate::objects::state::EventError> {
        self.runqueue
            .pick_next_task(CurrentRunQueueRef::BootRunQueue, CurrentTaskRef::BootIdle)
    }
}

struct EmptyQueueScenario {
    fixture: CurrentRunQueueFixture,
}

impl EmptyQueueScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for EmptyQueueScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.empty_queue"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("pick before setup", self.fixture.pick_next());
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("ready", self.fixture.runqueue.state() == State::Ready);
        assertions.assert("boot cpu", self.fixture.runqueue.cpu_ref().is_boot_cpu());
        assertions.assert(
            "root domain covers runqueue cpu",
            context_ref()
                .scheduler
                .default_root_domain()
                .covers_cpu_ref(self.fixture.runqueue.cpu_ref()),
        );
        assertions.assert(
            "current is idle",
            self.fixture.runqueue.curr_task_id() == self.fixture.runqueue.idle_task_id(),
        );
        assertions.assert("empty", self.fixture.runqueue.task_count() == 0);
        assertions.assert(
            "no runnable",
            self.fixture.runqueue.first_runnable_task_ref() == CurrentTaskRef::None,
        );
        assertions.assert_fail("pick empty", self.fixture.pick_next());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct KernelInitQueueScenario {
    fixture: CurrentRunQueueFixture,
}

impl KernelInitQueueScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for KernelInitQueueScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.kernel_init"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("enqueue", self.fixture.enqueue(CurrentTaskRef::KernelInit));
        assertions.assert(
            "contains kernel init",
            self.fixture.runqueue.contains_task(KERNEL_INIT_PID),
        );
        assertions.assert("task count", self.fixture.runqueue.task_count() == 1);
        assertions.assert(
            "first runnable",
            self.fixture.runqueue.first_runnable_task_ref() == CurrentTaskRef::KernelInit,
        );
        assertions.assert(
            "pick kernel init",
            self.fixture.pick_next() == Ok(CurrentTaskRef::KernelInit),
        );
        assertions.assert_fail(
            "duplicate enqueue",
            self.fixture.enqueue(CurrentTaskRef::KernelInit),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct KthreaddQueueScenario {
    fixture: CurrentRunQueueFixture,
}

impl KthreaddQueueScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for KthreaddQueueScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.kthreadd"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("enqueue", self.fixture.enqueue(CurrentTaskRef::Kthreadd));
        assertions.assert(
            "contains kthreadd",
            self.fixture.runqueue.contains_task(KTHREADD_PID),
        );
        assertions.assert("task count", self.fixture.runqueue.task_count() == 1);
        assertions.assert(
            "first runnable",
            self.fixture.runqueue.first_runnable_task_ref() == CurrentTaskRef::Kthreadd,
        );
        assertions.assert(
            "pick kthreadd",
            self.fixture.pick_next() == Ok(CurrentTaskRef::Kthreadd),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct MixedQueueScenario {
    fixture: CurrentRunQueueFixture,
}

impl MixedQueueScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for MixedQueueScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.mixed_queue"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok(
            "enqueue kthreadd",
            self.fixture.enqueue(CurrentTaskRef::Kthreadd),
        );
        assertions.assert_ok(
            "enqueue kernel init",
            self.fixture.enqueue(CurrentTaskRef::KernelInit),
        );
        assertions.assert("task count", self.fixture.runqueue.task_count() == 2);
        assertions.assert(
            "contains kernel init",
            self.fixture.runqueue.contains_task(KERNEL_INIT_PID),
        );
        assertions.assert(
            "contains kthreadd",
            self.fixture.runqueue.contains_task(KTHREADD_PID),
        );
        assertions.assert(
            "pick kernel init",
            self.fixture.pick_next() == Ok(CurrentTaskRef::KernelInit),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct InvalidTaskRefScenario {
    fixture: CurrentRunQueueFixture,
}

impl InvalidTaskRefScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for InvalidTaskRefScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.invalid_task_ref"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "enqueue before setup",
            self.fixture.enqueue(CurrentTaskRef::KernelInit),
        );
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("enqueue none", self.fixture.enqueue(CurrentTaskRef::None));
        assertions.assert_fail(
            "enqueue boot idle",
            self.fixture.enqueue(CurrentTaskRef::BootIdle),
        );
        assertions.assert("still empty", self.fixture.runqueue.task_count() == 0);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
