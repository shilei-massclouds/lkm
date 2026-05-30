use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let vmalloc = &ctx.vmalloc_allocator;
    let address_space = vmalloc.address_space();

    if vmalloc.state() != State::Ready || !vmalloc.initialized() || !vmalloc.reclaim_hook_ready() {
        printk::write_str("vmalloc allocator is not ready\n");
        return SmokeResult::Failed;
    }
    if vmalloc.area_cache().state() != State::Ready
        || address_space.state() != State::Ready
        || vmalloc.node_set().state() != State::Ready
        || vmalloc.block_queues().state() != State::Ready
        || vmalloc.deferred_set().state() != State::Ready
    {
        printk::write_str("vmap child object state invalid\n");
        return SmokeResult::Failed;
    }
    if address_space.start() >= address_space.end()
        || !address_space.free_space_ready()
        || !address_space.existing_vmlist_busy_imported()
        || vmalloc.node_set().node_count() == 0
        || !vmalloc.node_set().route_ready()
        || !vmalloc.block_queues().fast_path_metadata_ready()
        || !vmalloc.deferred_set().work_ready()
    {
        printk::write_str("vmap address-space facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "vmalloc={:#x}..{:#x} nodes={}\n",
        address_space.start(),
        address_space.end(),
        vmalloc.node_set().node_count()
    ));
    SmokeResult::Passed
}
