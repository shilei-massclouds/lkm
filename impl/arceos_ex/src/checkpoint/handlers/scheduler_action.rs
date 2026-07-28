use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{state::State, task::TaskRef},
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::SchedulerPickNextTaskExit,
    Checkpoint::SchedulerSwitchToEntry,
    Checkpoint::SchedulerSwitchToExit,
    Checkpoint::SchedulerScheduleExit,
];
pub const KUNIT_CASE_COUNT: usize = 4;

pub const HANDLER: Handler = Handler {
    name: "scheduler_action",
    priority: 85,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    match checkpoint {
        Checkpoint::SchedulerPickNextTaskExit => check_pick_next_task_exit(checkpoint, ctx, sink),
        Checkpoint::SchedulerSwitchToEntry => check_switch_to_entry(checkpoint, ctx, sink),
        Checkpoint::SchedulerSwitchToExit => check_switch_to_exit(checkpoint, ctx, sink),
        Checkpoint::SchedulerScheduleExit => check_schedule_exit(checkpoint, ctx, sink),
        _ => CheckpointOutcome::Continue,
    }
}

fn check_pick_next_task_exit(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "scheduler_action.pick_next_task_exit";
    sink.start_case(total, "", name, checkpoint);
    let next_ref = ctx.scheduler.pick_next_task_exit_next_ref();
    let next_is_first_boot_task = next_ref == TaskRef::KERNEL_INIT || next_ref == TaskRef::KTHREADD;

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.pick_next_task_exit_count() != 1
        || ctx.scheduler.pick_next_task_passes() != 1
        || ctx.scheduler.pick_next_task_exit_prev_ref() != TaskRef::BOOT
        || !next_is_first_boot_task
        || !ctx
            .scheduler
            .boot_cpu_owned_scheduler_view_ready(&ctx.cpu_group)
    {
        sink.fail(total, "", name, "PickNextTask exit facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", name);
    CheckpointOutcome::Continue
}

fn check_switch_to_exit(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "scheduler_action.switch_to_exit";
    sink.start_case(total, "", name, checkpoint);
    let next_ref = ctx.scheduler.switch_to_exit_next_ref();
    let current_ref = ctx.boot_cpu_current_task().current();
    let current_is_first_boot_task =
        current_ref == TaskRef::KERNEL_INIT || current_ref == TaskRef::KTHREADD;

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.switch_to_exit_count() != 1
        || ctx.scheduler.switch_to_exit_prev_ref() != TaskRef::BOOT
        || next_ref != ctx.scheduler.pick_next_task_exit_next_ref()
        || ctx.scheduler.switch_to_exit_current_ref() != next_ref
        || current_ref != next_ref
        || !current_is_first_boot_task
        || ctx.scheduler.switch_to_exit_committed_count()
            <= ctx.scheduler.switch_to_entry_committed_count()
        || ctx.boot_cpu_current_task().switch_committed_count()
            != ctx.scheduler.switch_to_exit_committed_count()
    {
        sink.fail(total, "", name, "SwitchTo exit facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", name);
    CheckpointOutcome::Continue
}

fn check_schedule_exit(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "scheduler_action.schedule_exit";
    sink.start_case(total, "", name, checkpoint);
    let next_ref = ctx.scheduler.schedule_exit_next_ref();
    let current_ref = ctx.boot_cpu_current_task().current();
    let current_is_first_boot_task =
        current_ref == TaskRef::KERNEL_INIT || current_ref == TaskRef::KTHREADD;

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.schedule_exit_count() != 1
        || ctx.scheduler.schedule_passes() != 1
        || ctx.scheduler.schedule_exit_prev_ref() != TaskRef::BOOT
        || next_ref != ctx.scheduler.pick_next_task_exit_next_ref()
        || next_ref != ctx.scheduler.switch_to_exit_next_ref()
        || ctx.scheduler.schedule_exit_current_ref() != next_ref
        || current_ref != next_ref
        || !current_is_first_boot_task
        || ctx.scheduler.schedule_exit_saved_interrupt_count()
            != ctx.boot_cpu_local_interrupt().saved_and_disabled_count()
        || ctx.scheduler.schedule_exit_restored_interrupt_count()
            != ctx.boot_cpu_local_interrupt().restored_count()
        || ctx.scheduler.schedule_exit_saved_interrupt_count()
            != ctx.scheduler.schedule_exit_restored_interrupt_count()
    {
        sink.fail(total, "", name, "Schedule exit facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", name);
    CheckpointOutcome::Continue
}

fn check_switch_to_entry(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "scheduler_action.switch_to_entry";
    sink.start_case(total, "", name, checkpoint);
    let next_ref = ctx.scheduler.switch_to_entry_next_ref();
    let next_is_first_boot_task = next_ref == TaskRef::KERNEL_INIT || next_ref == TaskRef::KTHREADD;

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.switch_to_entry_count() != 1
        || ctx.scheduler.switch_to_entry_prev_ref() != TaskRef::BOOT
        || ctx.scheduler.switch_to_entry_next_ref() != ctx.scheduler.pick_next_task_exit_next_ref()
        || !next_is_first_boot_task
        || ctx.scheduler.switch_to_entry_current_ref() != TaskRef::BOOT
        || ctx.scheduler.switch_to_entry_committed_count()
            >= ctx.boot_cpu_current_task().switch_committed_count()
    {
        sink.fail(total, "", name, "SwitchTo entry facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", name);
    CheckpointOutcome::Continue
}
