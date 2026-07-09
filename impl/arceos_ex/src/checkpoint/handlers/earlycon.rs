use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{earlycon, printk},
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PrintkBufferReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "earlycon",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, _ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    if !earlycon::is_online() || !printk::is_ready() {
        sink.fail(
            total,
            "",
            HANDLER.name,
            "early console or printk unavailable",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}
