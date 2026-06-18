use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    trace::Checkpoint,
};

pub const HANDLER: Handler = Handler {
    name: "trace",
    priority: 0,
    scope: HandlerScope::All,
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, _sink: &mut dyn Sink) -> CheckpointOutcome {
    super::early_trace::trace_name_with_context(checkpoint, ctx);
    CheckpointOutcome::Continue
}
