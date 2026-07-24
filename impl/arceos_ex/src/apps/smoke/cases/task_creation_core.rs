use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context_ref,
    objects::{
        cpu_control::CurrentTaskSlot,
        process_prepare::{TaskCopyProcessInputs, TaskCreationCore, TaskCreationSetup},
        state::State,
        task::{Task, TaskEntry, TaskExecutionAuthority, TaskRef},
        task_flow::TaskFlowRef,
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut CopyKernelInitScenario::new());
    suite.scenario(&mut CopyKthreaddScenario::new());
    suite.scenario(&mut RejectMissingEntryScenario::new());
    suite.scenario(&mut RejectEntryMismatchScenario::new());
    suite.scenario(&mut RejectOnlineSourceScenario::new());
    suite.scenario(&mut RejectReservedSourceScenario::new());
    suite.scenario(&mut RejectNonCurrentSourceScenario::new());
    suite.scenario(&mut RejectMismatchedSourceRefScenario::new());
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
                boot_task: &ctx.boot_task,
                exception_stream: &ctx.exception_stream,
            }),
        );
    }

    fn copy_process(&mut self, entry: TaskEntry, dst_entry: TaskEntry, dst_state: State) -> bool {
        let ctx = context_ref();
        let Ok(result) = self.core.copy_process(
            TaskCopyProcessInputs {
                src_task: ctx.kernel_init_task.task(),
                src_task_ref: ctx.kernel_init_task.task_ref(),
                current_task_slot: &ctx.boot_cpu_current_task,
                root_pid_namespace: &ctx.root_pid_namespace,
                credential_core: &ctx.credential_core,
                signal_core: &ctx.signal_core,
                task_file_context: &ctx.task_file_context,
                security_core: &ctx.security_core,
                scheduler: &ctx.scheduler,
                cpu_group: &ctx.cpu_group,
                entry,
            },
            dst_state,
            dst_entry,
        ) else {
            return false;
        };

        result.entry() == entry
            && self
                .core
                .copy_process_source()
                .same_identity(ctx.kernel_init_task.task_ref())
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
                    src_task: ctx.kernel_init_task.task(),
                    src_task_ref: ctx.kernel_init_task.task_ref(),
                    current_task_slot: &ctx.boot_cpu_current_task,
                    root_pid_namespace: &ctx.root_pid_namespace,
                    credential_core: &ctx.credential_core,
                    signal_core: &ctx.signal_core,
                    task_file_context: &ctx.task_file_context,
                    security_core: &ctx.security_core,
                    scheduler: &ctx.scheduler,
                    cpu_group: &ctx.cpu_group,
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
        assertions.assert(
            "source not committed",
            !self.fixture.core.copy_process_source().is_valid(),
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
                    src_task: ctx.kernel_init_task.task(),
                    src_task_ref: ctx.kernel_init_task.task_ref(),
                    current_task_slot: &ctx.boot_cpu_current_task,
                    root_pid_namespace: &ctx.root_pid_namespace,
                    credential_core: &ctx.credential_core,
                    signal_core: &ctx.signal_core,
                    task_file_context: &ctx.task_file_context,
                    security_core: &ctx.security_core,
                    scheduler: &ctx.scheduler,
                    cpu_group: &ctx.cpu_group,
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
        assertions.assert(
            "source not committed",
            !self.fixture.core.copy_process_source().is_valid(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RejectOnlineSourceScenario {
    fixture: TaskCreationCoreFixture,
}

impl RejectOnlineSourceScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for RejectOnlineSourceScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.reject_online_source"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        let mut current = CurrentTaskSlot::new();
        assertions.assert_ok("current setup", current.setup());
        assertions.assert_ok(
            "select online source",
            current.set_current(TaskRef::KTHREADD),
        );
        assertions.assert(
            "online source has no authority",
            ctx.kthreadd_task.task().state() == State::Online
                && ctx.kthreadd_task.task().execution_authority() == TaskExecutionAuthority::None,
        );
        assertions.assert(
            "copy rejected",
            copy_rejected(
                &mut self.fixture.core,
                ctx.kthreadd_task.task(),
                TaskRef::KTHREADD,
                &current,
            ),
        );
        assert_copy_not_committed(&self.fixture.core, assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RejectReservedSourceScenario {
    fixture: TaskCreationCoreFixture,
    source: Task,
    current: CurrentTaskSlot,
}

impl RejectReservedSourceScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
            source: Task::new_ap_idle_reserved(TaskRef::ap_idle(0), TaskFlowRef::ap_idle(0), 0),
            current: CurrentTaskSlot::new(),
        }
    }
}

impl SmokeScenario for RejectReservedSourceScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.reject_reserved_source"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
        assertions.assert_ok("current setup", self.current.setup());
        assertions.assert_ok(
            "select reserved source",
            self.current.set_current(self.source.task_ref()),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "reserved source is on cpu",
            self.source.state() == State::OnCpu
                && self.source.execution_authority() == TaskExecutionAuthority::Reserved,
        );
        assertions.assert(
            "copy rejected",
            copy_rejected(
                &mut self.fixture.core,
                &self.source,
                self.source.task_ref(),
                &self.current,
            ),
        );
        assert_copy_not_committed(&self.fixture.core, assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RejectNonCurrentSourceScenario {
    fixture: TaskCreationCoreFixture,
}

impl RejectNonCurrentSourceScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for RejectNonCurrentSourceScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.reject_non_current_source"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        let mut current = CurrentTaskSlot::new();
        assertions.assert_ok("current setup", current.setup());
        assertions.assert_ok("select another task", current.set_current(TaskRef::BOOT));
        assertions.assert(
            "copy rejected",
            copy_rejected(
                &mut self.fixture.core,
                ctx.kernel_init_task.task(),
                TaskRef::KERNEL_INIT,
                &current,
            ),
        );
        assert_copy_not_committed(&self.fixture.core, assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RejectMismatchedSourceRefScenario {
    fixture: TaskCreationCoreFixture,
}

impl RejectMismatchedSourceRefScenario {
    fn new() -> Self {
        Self {
            fixture: TaskCreationCoreFixture::new(),
        }
    }
}

impl SmokeScenario for RejectMismatchedSourceRefScenario {
    fn name(&self) -> &'static str {
        "task_creation_core.reject_mismatched_source_ref"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        let mut current = CurrentTaskSlot::new();
        assertions.assert_ok("current setup", current.setup());
        assertions.assert_ok("select supplied ref", current.set_current(TaskRef::BOOT));
        assertions.assert(
            "copy rejected",
            copy_rejected(
                &mut self.fixture.core,
                ctx.kernel_init_task.task(),
                TaskRef::BOOT,
                &current,
            ),
        );
        assert_copy_not_committed(&self.fixture.core, assertions);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn copy_rejected(
    core: &mut TaskCreationCore,
    source: &Task,
    source_ref: TaskRef,
    current: &CurrentTaskSlot,
) -> bool {
    let ctx = context_ref();
    core.copy_process(
        TaskCopyProcessInputs {
            src_task: source,
            src_task_ref: source_ref,
            current_task_slot: current,
            root_pid_namespace: &ctx.root_pid_namespace,
            credential_core: &ctx.credential_core,
            signal_core: &ctx.signal_core,
            task_file_context: &ctx.task_file_context,
            security_core: &ctx.security_core,
            scheduler: &ctx.scheduler,
            cpu_group: &ctx.cpu_group,
            entry: TaskEntry::KernelInit,
        },
        State::Prepared,
        TaskEntry::KernelInit,
    )
    .is_err()
}

fn assert_copy_not_committed(core: &TaskCreationCore, assertions: &mut SmokeAssertions) {
    assertions.assert(
        "source not committed",
        !core.copy_process_source().is_valid(),
    );
    assertions.assert("kernel init not created", !core.kernel_init_created());
    assertions.assert("kthreadd not created", !core.kthreadd_created());
}
