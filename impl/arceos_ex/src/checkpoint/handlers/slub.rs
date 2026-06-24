use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{mm_core::SlubState, state::State},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::SlubSubsystemReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "slub",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    let slub = &ctx.slub_subsystem;
    let registry = slub.cache_registry();
    let kmalloc = slub.kmalloc_caches();
    let Some(expected_cache_count) = registry
        .kmalloc_cache_count()
        .checked_add(registry.named_cache_count())
        .and_then(|count| count.checked_add(2))
    else {
        sink.fail(total, "", HANDLER.name, "SLUB cache count overflow");
        return CheckpointOutcome::FailAndShutdown;
    };

    if slub.state() != State::Ready
        || slub.slab_state() != SlubState::Up
        || !slub.boot_kmem_cache_node_ready()
        || !slub.bootstrap_completed()
        || !slub.cpu_cache_ready()
        || !slub.cpuhp_step_registered()
        || registry.state() != State::Ready
        || !registry.boot_caches_registered()
        || !registry.global_list_ready()
        || registry.cache_count() != expected_cache_count
        || registry.kmalloc_cache_count() != kmalloc.count()
        || kmalloc.state() != State::Ready
        || !kmalloc.size_index_ready()
        || !kmalloc.default_cache_ready()
        || !kmalloc.random_caches_trimmed()
        || !kmalloc.memcg_caches_trimmed()
        || !kmalloc.has_size(8)
        || !kmalloc.has_size(1024)
        || !kmalloc.has_size(8192)
        || kmalloc.kmalloc_size(64) != Some(64)
    {
        sink.diag_usize("registry_caches", registry.cache_count());
        sink.diag_usize("registry_kmalloc_caches", registry.kmalloc_cache_count());
        sink.diag_usize("registry_named_caches", registry.named_cache_count());
        sink.diag_usize("kmalloc_refs", kmalloc.count());
        sink.fail(total, "", HANDLER.name, "SLUB subsystem facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.diag_usize("registry_caches", registry.cache_count());
    sink.diag_usize("kmalloc_refs", kmalloc.count());
    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}
