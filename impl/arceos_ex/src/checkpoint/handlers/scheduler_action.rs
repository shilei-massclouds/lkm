use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::{cpu_control::CurrentTaskRef, state::State},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::SchedulerPickNextTaskExit,
    Checkpoint::SchedulerSwitchToEntry,
];
pub const KUNIT_CASE_COUNT: usize = 2;

pub const HANDLER: Handler = Handler {
    name: "scheduler_action",
    priority: 85,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, ctx: &mut Context) -> CheckpointOutcome {
    match checkpoint {
        Checkpoint::SchedulerPickNextTaskExit => check_pick_next_task_exit(checkpoint, ctx),
        Checkpoint::SchedulerSwitchToEntry => check_switch_to_entry(checkpoint, ctx),
        _ => CheckpointOutcome::Continue,
    }
}

fn check_pick_next_task_exit(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "scheduler_action.pick_next_task_exit";
    kunit::start_case(total, "", name, checkpoint);
    let next_ref = ctx.scheduler.pick_next_task_exit_next_ref();
    let next_is_first_boot_task =
        next_ref == CurrentTaskRef::KernelInit || next_ref == CurrentTaskRef::Kthreadd;

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.pick_next_task_exit_count() != 1
        || ctx.scheduler.pick_next_task_passes() != 1
        || ctx.scheduler.pick_next_task_exit_prev_ref() != CurrentTaskRef::BootIdle
        || !next_is_first_boot_task
        || ctx.scheduler.boot_runqueue().curr_task_id() != ctx.scheduler.boot_idle_task().task_id()
        || ctx.scheduler.boot_runqueue().idle_task_id() != ctx.scheduler.boot_idle_task().task_id()
    {
        kunit::fail(total, "", name, "PickNextTask exit facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    kunit::pass(total, "", name);
    CheckpointOutcome::Continue
}

fn check_switch_to_entry(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "scheduler_action.switch_to_entry";
    kunit::start_case(total, "", name, checkpoint);
    let next_ref = ctx.scheduler.switch_to_entry_next_ref();
    let next_is_first_boot_task =
        next_ref == CurrentTaskRef::KernelInit || next_ref == CurrentTaskRef::Kthreadd;

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.switch_to_entry_count() != 1
        || ctx.scheduler.switch_to_entry_prev_ref() != CurrentTaskRef::BootIdle
        || ctx.scheduler.switch_to_entry_next_ref() != ctx.scheduler.pick_next_task_exit_next_ref()
        || !next_is_first_boot_task
        || ctx.scheduler.switch_to_entry_current_ref() != CurrentTaskRef::BootIdle
        || ctx.scheduler.switch_to_entry_committed_count()
            >= ctx.boot_cpu_current_task.switch_committed_count()
    {
        kunit::fail(total, "", name, "SwitchTo entry facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    kunit::pass(total, "", name);
    CheckpointOutcome::Continue
}
