use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::state::State,
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::OfPlatformDefaultPopulateInitCalled];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "of_platform",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Read(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    kunit::start_case(total, "", HANDLER.name, checkpoint);

    let platform_bus = &ctx.platform_bus;
    if ctx.device_tree.state() != State::Ready
        || platform_bus.state() != State::Ready
        || !platform_bus.of_platform_source_tree_ready()
        || !platform_bus.of_platform_root_children_scanned()
        || !platform_bus.of_platform_strict_compatible_required()
        || !platform_bus.of_platform_default_bus_match_table_used()
        || !platform_bus.of_platform_bus_nodes_recurse()
        || !platform_bus.of_platform_candidates_identified()
        || !platform_bus.of_platform_candidates_are_available()
        || !platform_bus.of_platform_candidate_names_printed()
        || !platform_bus.of_platform_candidate_compatibles_printed()
        || !platform_bus.of_platform_device_registration_deferred()
        || platform_bus.of_platform_candidate_count() == 0
    {
        kunit::fail(
            total,
            "",
            HANDLER.name,
            "OF platform populate facts invalid",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    kunit::diag_usize(
        "of_platform_candidates",
        platform_bus.of_platform_candidate_count(),
    );
    kunit::drain_printk_diag();
    kunit::pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}
