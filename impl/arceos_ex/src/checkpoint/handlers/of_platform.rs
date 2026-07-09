use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::state::State,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::OfPlatformDefaultPopulateScanComplete];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "of_platform",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

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
        || !platform_bus.of_platform_devices_created()
        || !platform_bus.of_platform_devices_added()
        || platform_bus.of_platform_candidate_count() == 0
        || platform_bus.platform_device_count() != platform_bus.of_platform_candidate_count()
        || platform_bus.klist_device_count() != platform_bus.of_platform_candidate_count()
        || !first_platform_device_chain_valid(ctx)
    {
        sink.fail(
            total,
            "",
            HANDLER.name,
            "OF platform populate facts invalid",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.diag_usize(
        "of_platform_candidates",
        platform_bus.of_platform_candidate_count(),
    );
    sink.diag_usize("platform_devices", platform_bus.platform_device_count());
    sink.diag_usize("klist_devices", platform_bus.klist_device_count());
    sink.drain_printk_diag();
    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn first_platform_device_chain_valid(ctx: &Context) -> bool {
    let platform_bus = &ctx.platform_bus;
    let Some(device_ref) = platform_bus.klist_device_ref(0) else {
        return false;
    };
    let Some(platform_device) = platform_bus.platform_device(device_ref) else {
        return false;
    };
    if !platform_device.added()
        || !platform_device.dev().registered()
        || !platform_device.dev().bus_bound()
        || !platform_device.id_bound()
        || !platform_device.resources_bound()
    {
        return false;
    }
    let Some(node) = ctx.device_tree.node(platform_device.dev().node_id()) else {
        return false;
    };
    !node.name().is_empty() && node.property(b"compatible").is_some()
}
