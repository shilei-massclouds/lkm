use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        rest_init::KERNEL_INIT_PID,
        state::State,
        task::{TaskEntry, TaskKind},
    },
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::KernelInitTaskReady,
    Checkpoint::KernelInitTaskOnline,
];
pub const KUNIT_CASE_COUNT: usize = 2;

pub const HANDLER: Handler = Handler {
    name: "kernel_init_task",
    priority: 80,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    match checkpoint {
        Checkpoint::KernelInitTaskReady => check_ready(checkpoint, ctx, sink),
        Checkpoint::KernelInitTaskOnline => check_online(checkpoint, ctx, sink),
        _ => CheckpointOutcome::Continue,
    }
}

fn check_ready(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "kernel_init_task.ready";
    sink.start_case(total, "", name, checkpoint);
    let Some(boot_scheduler_view) = ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        sink.fail(
            total,
            "",
            name,
            "KernelInitTask Ready CPU scheduler view missing",
        );
        return CheckpointOutcome::FailAndShutdown;
    };

    if ctx.kernel_init_task.state() != State::Ready
        || ctx.kernel_init_task.pid() != KERNEL_INIT_PID
        || ctx.kernel_init_task.entry() != TaskEntry::KernelInit
        || ctx.kernel_init_task.kind() != TaskKind::UserModeThread
        || !ctx.kernel_init_task.clone_fs()
        || ctx.kernel_init_task.user_mm_created()
        || !ctx.kernel_init_task.thread_context_ready()
        || !ctx.kernel_init_task.sched_entity_ready()
        || !ctx.kernel_init_task.waiting_for_kthreadd_done()
        || ctx.kernel_init_task.running()
        || ctx.kernel_init_task.enqueued()
        || !ctx.task_creation_core.kernel_init_created()
        || boot_scheduler_view.runqueue_contains_task_id(ctx.kernel_init_task.pid())
        || boot_scheduler_view.runqueue_task_count() != 0
    {
        sink.fail(total, "", name, "KernelInitTask Ready facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", name);
    CheckpointOutcome::Continue
}

fn check_online(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "kernel_init_task.online";
    sink.start_case(total, "", name, checkpoint);
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        sink.fail(total, "", name, "KernelInitTask Online boot CPU missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(boot_scheduler_view) = ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        sink.fail(
            total,
            "",
            name,
            "KernelInitTask Online CPU scheduler view missing",
        );
        return CheckpointOutcome::FailAndShutdown;
    };

    if ctx.kernel_init_task.state() != State::Online
        || ctx.kernel_init_task.pid() != KERNEL_INIT_PID
        || ctx.kernel_init_task.entry() != TaskEntry::KernelInit
        || !ctx.kernel_init_task.running()
        || !ctx.kernel_init_task.enqueued()
        || ctx.kernel_init_task.flow_cpu_id() != boot_cpu.logical_id()
        || !boot_scheduler_view.runqueue_contains_task_id(ctx.kernel_init_task.pid())
        || boot_scheduler_view.runqueue_task_count() != 1
    {
        sink.fail(total, "", name, "KernelInitTask Online facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", name);
    CheckpointOutcome::Continue
}
