use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context_ref,
    objects::{
        rest_init::{KERNEL_INIT_PID, KTHREADD_PID},
        scheduler::{PickNextProtocol, PrevDisposition, SchedClassRef, Scheduler},
        state::State,
        task::TaskRef,
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut EmptyQueueScenario::new());
    suite.scenario(&mut KernelInitQueueScenario::new());
    suite.scenario(&mut KthreaddQueueScenario::new());
    suite.scenario(&mut MixedQueueScenario::new());
    suite.scenario(&mut InvalidTaskRefScenario::new());
    suite.scenario(&mut DynamicTaskGenerationScenario::new());
    suite.scenario(&mut ClassProtocolScenario::new());
    suite.result()
}

struct ClassProtocolScenario {
    fixture: CurrentRunQueueFixture,
}

impl ClassProtocolScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for ClassProtocolScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.class_protocol"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let rq_ref = self.fixture.selected_ref();
        assertions.assert_ok(
            "enqueue fair",
            self.fixture.runqueue.enqueue_task_in_class(
                rq_ref,
                TaskRef::KERNEL_INIT,
                SchedClassRef::Fair,
            ),
        );
        assertions.assert_ok(
            "enqueue realtime",
            self.fixture.runqueue.enqueue_task_in_class(
                rq_ref,
                TaskRef::KTHREADD,
                SchedClassRef::Realtime,
            ),
        );
        assertions.assert_ok(
            "enqueue deadline",
            self.fixture.runqueue.enqueue_task_in_class(
                rq_ref,
                TaskRef::SMOKE_SCHEDULER,
                SchedClassRef::Deadline,
            ),
        );
        assertions.assert_ok(
            "enqueue stop",
            self.fixture.runqueue.enqueue_task_in_class(
                rq_ref,
                TaskRef::SMOKE_MUTEX,
                SchedClassRef::Stop,
            ),
        );

        let current_ref = self.fixture.current_ref();
        let combined = self.fixture.runqueue.pick_next_task_for_protocol(
            current_ref,
            TaskRef::BOOT,
            PrevDisposition::Runnable,
            PickNextProtocol::Combined,
        );
        let fallback = self.fixture.runqueue.pick_next_task_for_protocol(
            current_ref,
            TaskRef::BOOT,
            PrevDisposition::Runnable,
            PickNextProtocol::Fallback,
        );
        assertions.assert(
            "combined and fallback agree",
            combined == Ok(TaskRef::SMOKE_MUTEX) && fallback == combined,
        );
        assertions.assert(
            "stop class first",
            self.fixture.runqueue.task_class(TaskRef::SMOKE_MUTEX) == Some(SchedClassRef::Stop),
        );
        assertions.assert_ok("dequeue stop", self.fixture.dequeue(TaskRef::SMOKE_MUTEX));
        assertions.assert(
            "deadline class second",
            self.fixture.pick_next() == Ok(TaskRef::SMOKE_SCHEDULER),
        );
        assertions.assert_ok(
            "dequeue deadline",
            self.fixture.dequeue(TaskRef::SMOKE_SCHEDULER),
        );
        assertions.assert(
            "realtime class third",
            self.fixture.pick_next() == Ok(TaskRef::KTHREADD),
        );
        assertions.assert_ok("dequeue realtime", self.fixture.dequeue(TaskRef::KTHREADD));
        assertions.assert(
            "fair class fourth",
            self.fixture.pick_next() == Ok(TaskRef::KERNEL_INIT),
        );
        assertions.assert_ok(
            "deactivate fair prev",
            self.fixture.dequeue(TaskRef::KERNEL_INIT),
        );
        assertions.assert(
            "blocked prev cannot be put back",
            self.fixture.runqueue.pick_next_task_for_protocol(
                current_ref,
                TaskRef::KERNEL_INIT,
                PrevDisposition::Blocked,
                PickNextProtocol::Fallback,
            ) == Ok(TaskRef::BOOT)
                && !self
                    .fixture
                    .runqueue
                    .contains_task_ref(TaskRef::KERNEL_INIT),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct CurrentRunQueueFixture {
    runqueue: Scheduler,
}

impl CurrentRunQueueFixture {
    fn new() -> Self {
        Self {
            runqueue: Scheduler::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "setup",
            self.runqueue.setup_runqueue_for_local_subject(
                &ctx.cpu_group,
                &ctx.per_cpu_storage,
                ctx.scheduler_shared.default_root_domain(),
            ),
        );
    }

    fn current_ref(&self) -> crate::objects::cpu::CpuRef {
        self.runqueue.cpu_ref()
    }

    fn selected_ref(&self) -> crate::objects::cpu::CpuRef {
        self.runqueue.cpu_ref()
    }

    fn enqueue(&mut self, task_ref: TaskRef) -> Result<(), crate::objects::state::EventError> {
        self.runqueue
            .enqueue_task_ref(self.selected_ref(), task_ref)
    }

    fn enqueue_with_ref(
        &mut self,
        runqueue_ref: crate::objects::cpu::CpuRef,
        task_ref: TaskRef,
    ) -> Result<(), crate::objects::state::EventError> {
        self.runqueue.enqueue_task_ref(runqueue_ref, task_ref)
    }

    fn pick_next(&self) -> Result<TaskRef, crate::objects::state::EventError> {
        self.runqueue.pick_next_task_for_local_subject(
            self.current_ref(),
            TaskRef::BOOT,
            crate::objects::scheduler::PrevDisposition::Runnable,
        )
    }

    fn enqueue_dynamic(
        &mut self,
        task_ref: TaskRef,
        pid: usize,
    ) -> Result<(), crate::objects::state::EventError> {
        self.runqueue
            .enqueue_task_with_id(self.selected_ref(), task_ref, pid)
    }

    fn dequeue(&mut self, task_ref: TaskRef) -> Result<(), crate::objects::state::EventError> {
        self.runqueue
            .dequeue_task_ref(self.selected_ref(), task_ref)
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
        assertions.assert(
            "ready",
            self.fixture.runqueue.runqueue_state() == State::Ready,
        );
        assertions.assert("boot cpu", self.fixture.runqueue.cpu_ref().is_boot_cpu());
        assertions.assert(
            "root domain covers runqueue cpu",
            context_ref()
                .scheduler_shared
                .default_root_domain()
                .covers_cpu_ref(self.fixture.runqueue.cpu_ref()),
        );
        assertions.assert(
            "current is idle",
            self.fixture.runqueue.curr_task_id() == self.fixture.runqueue.idle_task_id(),
        );
        assertions.assert(
            "stable idle and stop refs",
            self.fixture.runqueue.idle_ref() == TaskRef::BOOT
                && self.fixture.runqueue.stop_ref() == TaskRef::NONE,
        );
        assertions.assert("empty", self.fixture.runqueue.task_count() == 0);
        assertions.assert(
            "no runnable",
            self.fixture.runqueue.first_runnable_task_ref() == TaskRef::NONE,
        );
        assertions.assert(
            "idle selected when class queues empty",
            self.fixture.pick_next() == Ok(TaskRef::BOOT),
        );
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
        assertions.assert_ok("enqueue", self.fixture.enqueue(TaskRef::KERNEL_INIT));
        assertions.assert(
            "contains kernel init",
            self.fixture.runqueue.contains_task(KERNEL_INIT_PID),
        );
        assertions.assert("task count", self.fixture.runqueue.task_count() == 1);
        assertions.assert(
            "first runnable",
            self.fixture.runqueue.first_runnable_task_ref() == TaskRef::KERNEL_INIT,
        );
        assertions.assert(
            "pick kernel init",
            self.fixture.pick_next() == Ok(TaskRef::KERNEL_INIT),
        );
        assertions.assert_fail(
            "duplicate enqueue",
            self.fixture.enqueue(TaskRef::KERNEL_INIT),
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
        assertions.assert_ok("enqueue", self.fixture.enqueue(TaskRef::KTHREADD));
        assertions.assert(
            "contains kthreadd",
            self.fixture.runqueue.contains_task(KTHREADD_PID),
        );
        assertions.assert("task count", self.fixture.runqueue.task_count() == 1);
        assertions.assert(
            "first runnable",
            self.fixture.runqueue.first_runnable_task_ref() == TaskRef::KTHREADD,
        );
        assertions.assert(
            "pick kthreadd",
            self.fixture.pick_next() == Ok(TaskRef::KTHREADD),
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
        assertions.assert_ok("enqueue kthreadd", self.fixture.enqueue(TaskRef::KTHREADD));
        assertions.assert_ok(
            "enqueue kernel init",
            self.fixture.enqueue(TaskRef::KERNEL_INIT),
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
            self.fixture.pick_next() == Ok(TaskRef::KERNEL_INIT),
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
            self.fixture.enqueue(TaskRef::KERNEL_INIT),
        );
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail("enqueue none", self.fixture.enqueue(TaskRef::NONE));
        assertions.assert_fail("enqueue boot idle", self.fixture.enqueue(TaskRef::BOOT));
        assertions.assert_fail(
            "enqueue wrong cpu ref",
            self.fixture
                .enqueue_with_ref(crate::objects::cpu::CpuRef::invalid(), TaskRef::KERNEL_INIT),
        );
        assertions.assert("still empty", self.fixture.runqueue.task_count() == 0);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct DynamicTaskGenerationScenario {
    fixture: CurrentRunQueueFixture,
}

impl DynamicTaskGenerationScenario {
    fn new() -> Self {
        Self {
            fixture: CurrentRunQueueFixture::new(),
        }
    }
}

impl SmokeScenario for DynamicTaskGenerationScenario {
    fn name(&self) -> &'static str {
        "current_runqueue_ref.dynamic_generation"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let first = TaskRef::user(0, 1);
        let recycled = TaskRef::user(0, 2);
        assertions.assert_ok(
            "enqueue first generation",
            self.fixture.enqueue_dynamic(first, 3),
        );
        assertions.assert(
            "first generation visible",
            self.fixture.runqueue.contains_task_ref(first),
        );
        assertions.assert(
            "future generation rejected",
            !self.fixture.runqueue.contains_task_ref(recycled),
        );
        assertions.assert_fail(
            "future generation cannot dequeue",
            self.fixture.dequeue(recycled),
        );
        assertions.assert_ok("dequeue first generation", self.fixture.dequeue(first));
        assertions.assert_ok(
            "enqueue recycled slot",
            self.fixture.enqueue_dynamic(recycled, 4),
        );
        assertions.assert(
            "stale generation rejected",
            !self.fixture.runqueue.contains_task_ref(first),
        );
        assertions.assert(
            "recycled generation visible",
            self.fixture.runqueue.contains_task_ref(recycled),
        );
        assertions.assert(
            "pick exact recycled ref",
            self.fixture.pick_next() == Ok(recycled),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
