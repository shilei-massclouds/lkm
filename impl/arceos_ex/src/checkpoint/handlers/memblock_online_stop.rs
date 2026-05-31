use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    context::Context,
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::MemBlockOnline];

pub const HANDLER: Handler = Handler {
    name: "memblock-online-stop",
    priority: 200,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Read(run),
};

fn run(_checkpoint: Checkpoint, _ctx: &Context) -> CheckpointOutcome {
    CheckpointOutcome::StopAndShutdown
}
