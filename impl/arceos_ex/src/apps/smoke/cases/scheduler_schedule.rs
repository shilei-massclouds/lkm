use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        state::State,
        task::{Task, TaskBreakpointState, TaskEntry, TaskExecutionAuthority, TaskKind, TaskRef},
        task_flow::{TaskFlow, TaskFlowRef},
    },
};

const SCHEDULE_ATTEMPTS: usize = 4;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut CooperativeSwitchScenario::new());
    suite.result()
}

struct CooperativeSwitchScenario;

impl CooperativeSwitchScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for CooperativeSwitchScenario {
    fn name(&self) -> &'static str {
        "scheduler_schedule.cooperative_switch"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert("scheduler online", ctx.scheduler.state() == State::Online);
        assertions.assert(
            "current kernel init",
            ctx.boot_cpu_current_task.current_is_kernel_init(),
        );
        assertions.assert_ok(
            "setup smoke scheduler task",
            ctx.setup_smoke_scheduler_task(smoke_scheduler_task_entry),
        );
        assertions.assert_ok(
            "enqueue smoke scheduler task",
            ctx.enqueue_smoke_scheduler_task(),
        );
        assertions.assert_fail(
            "duplicate setup smoke scheduler task",
            ctx.setup_smoke_scheduler_task(smoke_scheduler_task_entry),
        );
        assertions.assert_fail(
            "duplicate enqueue smoke scheduler task",
            ctx.enqueue_smoke_scheduler_task(),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let mut attempts = 0usize;
        while attempts < SCHEDULE_ATTEMPTS {
            {
                let ctx = context();
                if ctx.scheduler.smoke_scheduler_task().yielded_back()
                    && ctx.boot_cpu_current_task.current_is_kernel_init()
                {
                    break;
                }
            }

            assertions.assert_ok("schedule current", context().schedule_current());
            attempts += 1;
        }

        let ctx = context();
        assertions.assert("bounded return", attempts < SCHEDULE_ATTEMPTS);
        assertions.assert(
            "smoke task ran",
            ctx.scheduler.smoke_scheduler_task().entry_ran(),
        );
        assertions.assert(
            "smoke task cpu",
            ctx.scheduler.smoke_scheduler_task().cpu_id()
                == ctx
                    .scheduler
                    .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
                    .map(|view| view.runqueue().cpu_id())
                    .unwrap_or(usize::MAX),
        );
        assertions.assert(
            "smoke task enqueued",
            ctx.scheduler
                .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
                .map(|view| {
                    view.runqueue_contains_task_id(ctx.scheduler.smoke_scheduler_task().task_id())
                })
                .unwrap_or(false),
        );
        assertions.assert(
            "smoke task yielded back",
            ctx.scheduler.smoke_scheduler_task().yielded_back(),
        );
        assertions.assert(
            "smoke task uses unified task and flow carriers",
            ctx.scheduler.smoke_scheduler_task().unified_carrier_ready()
                && ctx.scheduler.smoke_scheduler_task().task_ref() == TaskRef::SMOKE_SCHEDULER
                && ctx.scheduler.smoke_scheduler_task().flow_ref().is_valid(),
        );
        assertions.assert(
            "current returned kernel init",
            ctx.boot_cpu_current_task.current() == TaskRef::KERNEL_INIT,
        );
        assertions.assert(
            "smoke switch saved",
            ctx.scheduler
                .smoke_scheduler_task()
                .thread_context()
                .core_saved_count()
                != 0,
        );
        assertions.assert(
            "smoke switch restored",
            ctx.scheduler
                .smoke_scheduler_task()
                .thread_context()
                .core_restored_count()
                != 0,
        );
        assertions.assert(
            "assembly switch preserves ra/sp/s0-s11 sentinels",
            ctx.scheduler.task_switch_register_sentinel_passes() != 0
                && ctx.kernel_init_task.switch_context().sp() != 0
                && ctx
                    .kernel_init_task
                    .switch_context()
                    .saved_register(11)
                    .is_some()
                && ctx
                    .kernel_init_task
                    .switch_context()
                    .saved_register(12)
                    .is_none(),
        );
        assertions.assert(
            "tp follows selected Task identity outside switch context",
            ctx.scheduler.task_switch_tp_identity_passes() != 0,
        );

        let before_state = ctx.kernel_init_task.state();
        let before_authority = ctx.kernel_init_task.task().execution_authority();
        let before_breakpoint = ctx.kernel_init_task.task().breakpoint_state();
        let before_saved = ctx
            .kernel_init_task
            .task()
            .thread_context()
            .core_saved_count();
        let before_restored = ctx
            .kernel_init_task
            .task()
            .thread_context()
            .core_restored_count();
        let before_slot_commits = ctx.boot_cpu_current_task.switch_committed_count();
        let before_identity = ctx.scheduler.identity_switch_passes();
        assertions.assert_ok("identity switch accepted", ctx.smoke_identity_switch());
        assertions.assert(
            "identity switch has no lifecycle breakpoint or context event",
            ctx.scheduler.identity_switch_passes() == before_identity + 1
                && ctx.kernel_init_task.state() == before_state
                && ctx.kernel_init_task.task().execution_authority() == before_authority
                && ctx.kernel_init_task.task().breakpoint_state() == before_breakpoint
                && ctx
                    .kernel_init_task
                    .task()
                    .thread_context()
                    .core_saved_count()
                    == before_saved
                && ctx
                    .kernel_init_task
                    .task()
                    .thread_context()
                    .core_restored_count()
                    == before_restored
                && ctx.boot_cpu_current_task.switch_committed_count() == before_slot_commits,
        );
        assertions.assert(
            "prepared enable continue handoff suspend and terminal contract",
            task_breakpoint_contract_smoke(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn task_breakpoint_contract_smoke() -> bool {
    let mut task = Task::with_ref(TaskRef::user(0, 77));
    let mut initial = TaskFlow::new_user(0, 0);
    if task
        .set_identity_metadata(77, TaskEntry::UserChild, TaskKind::TestOnly)
        .is_err()
        || task.adopt_preset().is_err()
        || initial.declare().is_err()
        || initial.bind(&mut task, TaskFlowRef::NONE).is_err()
        || task.bind_initial_flow(&initial).is_err()
    {
        return false;
    }
    task.init_dummy_switch_context();
    if task.breakpoint_state() != TaskBreakpointState::Prepared
        || task.adopt_setup().is_err()
        || !task.set_task_cpu(0)
        || task.set_runtime_running().is_err()
        || task.publish_runqueue_binding().is_err()
        || task.adopt_enable().is_err()
    {
        return false;
    }
    let initial_ref = initial.flow_ref();
    let stale_initial = initial_ref.with_generation_for_test(initial_ref.generation() + 1);
    if task.state() != State::Online
        || task.execution_authority() != TaskExecutionAuthority::None
        || task.breakpoint_state() != TaskBreakpointState::Valid
        || task.breakpoint_flow() != initial_ref
        || !task.breakpoint_matches(initial_ref)
        || task.breakpoint_matches(stale_initial)
        || !task.switch_in_ready()
        || task.continue_on_cpu().is_err()
        || task.switch_in_ready()
        || initial.start_initial(&mut task, None, None).is_err()
    {
        return false;
    }

    let mut successor = TaskFlow::new_user(0, 1);
    if successor.declare().is_err()
        || successor.bind(&mut task, initial_ref).is_err()
        || successor.preset(&task, None).is_err()
        || successor.setup(&task, None).is_err()
        || initial.disable(&task, None).is_err()
        || task.commit_flow_handoff(&initial, &mut successor).is_err()
        || successor.enable(&task, None).is_err()
        || initial.cleanup(&task, None).is_err()
        || task.retire_destroyed_flow(&initial).is_err()
    {
        return false;
    }
    let successor_ref = successor.flow_ref();
    if task.breakpoint_state() != TaskBreakpointState::Invalid
        || task.suspend_from_cpu().is_err()
        || task.breakpoint_state() != TaskBreakpointState::Valid
        || task.breakpoint_flow() != successor_ref
        || !task.breakpoint_matches(successor_ref)
        || task.breakpoint_matches(initial_ref)
        || task.continue_on_cpu().is_err()
        || successor.cleanup_active_for_exit(&mut task).is_err()
        || task.disable().is_err()
        || task.state() != State::Offline
        || task.breakpoint_state() != TaskBreakpointState::Invalid
        || task.cleanup().is_err()
    {
        return false;
    }
    task.state() == State::Destroyed
}

extern "C" fn smoke_scheduler_task_entry() -> ! {
    let ctx = context();
    if ctx.mark_smoke_scheduler_entry_ran().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if ctx.mark_smoke_scheduler_yielded_back().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if ctx.schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    loop {
        core::hint::spin_loop();
    }
}
