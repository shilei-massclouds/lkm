use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        process_prepare::{TaskCopyProcessInputs, TaskCreationCore, TaskCreationSetup},
        state::State,
        task::TaskEntry,
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut CopyKernelInitScenario::new());
    suite.scenario(&mut CopyKthreaddScenario::new());
    suite.scenario(&mut RejectMissingEntryScenario::new());
    suite.scenario(&mut RejectEntryMismatchScenario::new());
    suite.result()
}

struct TaskCreationCoreFixture {
    core: TaskCreationCore,
}

impl TaskCreationCoreFixture {
    fn new() -> Self {
        Self {
            core: TaskCreationCore::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "preset",
            self.core.preset(
                &ctx.slub_subsystem,
                ctx.slub_subsystem.kmalloc_caches(),
                &ctx.per_cpu_storage,
            ),
        );
        if assertions.failed() {
            return;
        }

        assertions.assert_ok(
            "setup",
            self.core.setup(TaskCreationSetup {
                root_pid_namespace: &ctx.root_pid_namespace,
                credential_core: &ctx.credential_core,
                cpu_group: &ctx.cpu_group,
                cpu_capabilities: &ctx.cpu_capabilities,
                slub_subsystem: &ctx.slub_subsystem,
                init_task: &ctx.init_task,
                exception_stream: &ctx.exception_stream,
            }),
        );
    }

    fn copy_process(&mut self, entry: TaskEntry, dst_entry: TaskEntry, dst_state: State) -> bool {
        let ctx = context_ref();
        let Ok(result) = self.core.copy_process(
            TaskCopyProcessInputs {
                src_task: &ctx.init_task,
                root_pid_namespace: &ctx.root_pid_namespace,
                credential_core: &ctx.credential_core,
                signal_core: &ctx.signal_core,
                task_file_context: &ctx.task_file_context,
                security_core: &ctx.security_core,
                scheduler: &ctx.scheduler,
                entry,
            },
            dst_state,
            dst_entry,
        ) else {
            return false;
        };

        result.entry() == entry
            && result.task_struct_allocated()
            && result.thread_context_ready()
            && result.sched_entity_ready()
            && result.task_state_new()
            && result.task_not_enqueued()
    }
}

struct CopyKernelInitScenario {
    fixture: TaskCreationCoreFixture,
}

impl CopyKernelInitScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for CopyKernelInitScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.copy_kernel_init"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "copy process",
            self.fixture.copy_process(
                TaskEntry::KernelInit,
                TaskEntry::KernelInit,
                State::Prepared,
            ),
        );
        assertions.assert(
            "kernel init created",
            self.fixture.core.kernel_init_created(),
        );
        assertions.assert(
            "kthreadd not created",
            !self.fixture.core.kthreadd_created(),
        );
        assertions.assert(
            "not system scheduling",
            !self.fixture.core.system_scheduling(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct CopyKthreaddScenario {
    fixture: TaskCreationCoreFixture,
}

impl CopyKthreaddScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for CopyKthreaddScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.copy_kthreadd"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "copy process",
            self.fixture
                .copy_process(TaskEntry::Kthreadd, TaskEntry::Kthreadd, State::Prepared),
        );
        assertions.assert(
            "kernel init not created",
            !self.fixture.core.kernel_init_created(),
        );
        assertions.assert("kthreadd created", self.fixture.core.kthreadd_created());
        assertions.assert(
            "not system scheduling",
            !self.fixture.core.system_scheduling(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RejectMissingEntryScenario {
    fixture: TaskCreationCoreFixture,
}

impl RejectMissingEntryScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for RejectMissingEntryScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.reject_missing_entry"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_fail(
            "copy process",
            self.fixture.core.copy_process(
                TaskCopyProcessInputs {
                    src_task: &ctx.init_task,
                    root_pid_namespace: &ctx.root_pid_namespace,
                    credential_core: &ctx.credential_core,
                    signal_core: &ctx.signal_core,
                    task_file_context: &ctx.task_file_context,
                    security_core: &ctx.security_core,
                    scheduler: &ctx.scheduler,
                    entry: TaskEntry::None,
                },
                State::Prepared,
                TaskEntry::None,
            ),
        );
        assertions.assert(
            "kernel init not created",
            !self.fixture.core.kernel_init_created(),
        );
        assertions.assert(
            "kthreadd not created",
            !self.fixture.core.kthreadd_created(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RejectEntryMismatchScenario {
    fixture: TaskCreationCoreFixture,
}

impl RejectEntryMismatchScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for RejectEntryMismatchScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.reject_entry_mismatch"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_fail(
            "copy process",
            self.fixture.core.copy_process(
                TaskCopyProcessInputs {
                    src_task: &ctx.init_task,
                    root_pid_namespace: &ctx.root_pid_namespace,
                    credential_core: &ctx.credential_core,
                    signal_core: &ctx.signal_core,
                    task_file_context: &ctx.task_file_context,
                    security_core: &ctx.security_core,
                    scheduler: &ctx.scheduler,
                    entry: TaskEntry::KernelInit,
                },
                State::Prepared,
                TaskEntry::Kthreadd,
            ),
        );
        assertions.assert(
            "kernel init not created",
            !self.fixture.core.kernel_init_created(),
        );
        assertions.assert(
            "kthreadd not created",
            !self.fixture.core.kthreadd_created(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
