use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{mm_core::SlubState, printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let slub = &ctx.slub_allocator;
    let registry = slub.cache_registry();
    let kmalloc = slub.kmalloc_caches();

    if slub.state() != State::Ready || slub.slab_state() != SlubState::Up {
        printk::write_str("slub allocator is not up\n");
        return SmokeResult::Failed;
    }
    if !slub.boot_kmem_cache_node_ready()
        || !slub.bootstrap_completed()
        || !slub.cpu_cache_ready()
        || !slub.cpuhp_step_registered()
    {
        printk::write_str("slub bootstrap facts missing\n");
        return SmokeResult::Failed;
    }
    if registry.state() != State::Ready
        || !registry.boot_caches_registered()
        || !registry.global_list_ready()
        || registry.cache_count() < 2
    {
        printk::write_str("slub cache registry invalid\n");
        return SmokeResult::Failed;
    }
    if kmalloc.state() != State::Ready
        || !kmalloc.size_index_ready()
        || !kmalloc.default_cache_ready()
        || !kmalloc.random_caches_trimmed()
        || !kmalloc.memcg_caches_trimmed()
        || !kmalloc.has_size(8)
        || !kmalloc.has_size(1024)
    {
        printk::write_str("kmalloc cache facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "registry_caches={} kmalloc_caches={}\n",
        registry.cache_count(),
        kmalloc.count()
    ));
    SmokeResult::Passed
}
