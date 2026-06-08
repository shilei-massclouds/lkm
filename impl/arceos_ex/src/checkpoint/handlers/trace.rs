use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    context::Context,
    trace::Checkpoint,
};

pub const HANDLER: Handler = Handler {
    name: "trace",
    priority: 0,
    scope: HandlerScope::All,
    run: HandlerRun::Read(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    super::early_trace::trace_name_with_context(checkpoint, ctx);
    CheckpointOutcome::Continue
}
