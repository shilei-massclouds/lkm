use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::{earlycon, printk},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PrintkBufferReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "earlycon",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, _ctx: &mut Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    kunit::start_case(total, "", HANDLER.name, checkpoint);

    if !earlycon::is_online() || !printk::is_ready() {
        kunit::fail(
            total,
            "",
            HANDLER.name,
            "early console or printk unavailable",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    printk::write_fmt(format_args!(
        "formatted value={} hex={:#x}\n",
        42usize, 42usize
    ));
    kunit::drain_printk_diag();
    kunit::pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}
