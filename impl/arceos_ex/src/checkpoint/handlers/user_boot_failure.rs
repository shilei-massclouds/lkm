use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        user_boot::{ElfError, UserInitAttemptReason, UserInitAttemptStage, UserInitPathRef},
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::UserBootInitAttemptFailed];

pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "user_boot.init_attempt_failure",
    priority: 121,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "user_boot.init_attempt_failed";
    sink.start_case(total, "", name, checkpoint);

    let payload = &ctx.user_boot_payload;
    let failure = payload.last_init_attempt_failure();
    let valid = payload.state() == State::Ready
        && payload.init_attempt_failure_trace_defined()
        && payload.init_attempt_failure_recorded()
        && payload.init_attempt_failure_checkpoint_bound()
        && failure.stage() != UserInitAttemptStage::None
        && failure.reason() != UserInitAttemptReason::None
        && failure.path_actual_len() != 0
        && failure.path_len() <= failure.path_actual_len()
        && failure.path_len() == failure.path_bytes().len()
        && if failure.path() == UserInitPathRef::RequestedInit {
            failure.requested_terminal() && !failure.default_nonfatal()
        } else {
            !failure.requested_terminal() && failure.default_nonfatal()
        };

    sink.diag_usize("init_attempt_path_index", failure.path().index());
    sink.diag_usize("init_attempt_path_len", failure.path_len());
    sink.diag_usize("init_attempt_path_actual_len", failure.path_actual_len());
    sink.diag_usize(
        "init_attempt_path_truncated",
        failure.path_truncated() as usize,
    );
    sink.diag_usize("init_attempt_stage", failure.stage().index());
    sink.diag_usize("init_attempt_reason", failure.reason().index());
    sink.diag_usize(
        "init_attempt_requested_terminal",
        failure.requested_terminal() as usize,
    );
    sink.diag_usize(
        "init_attempt_default_nonfatal",
        failure.default_nonfatal() as usize,
    );
    sink.diag_usize(
        "init_attempt_elf_error",
        failure.elf_error().map_or(0, ElfError::index),
    );

    if valid {
        sink.pass(total, "", name);
        CheckpointOutcome::StopAndShutdown
    } else {
        sink.fail(total, "", name, "init attempt failure facts invalid");
        CheckpointOutcome::FailAndShutdown
    }
}
