use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let swiotlb = &ctx.swiotlb;

    if swiotlb.state() != State::Ready {
        printk::write_str("swiotlb is not ready\n");
        return SmokeResult::Failed;
    }
    let expected_required = ctx.dma_cache_policy.noncoherent_supported()
        && ctx.dma_cache_policy.cache_alignment() > 1
        && ctx.page_allocator.totalram_pages() != 0;
    if swiotlb.pool_required() != expected_required {
        printk::write_str("swiotlb pool policy mismatch\n");
        return SmokeResult::Failed;
    }
    if !swiotlb.early_pool_ready() || !swiotlb.dynamic_growth_trimmed() {
        printk::write_str("swiotlb early pool facts missing\n");
        return SmokeResult::Failed;
    }
    if swiotlb.pool_required() {
        if !swiotlb.static_pool_area_locks_ready()
            || swiotlb.static_pool_area_count() == 0
            || swiotlb.static_pool_lock_count() != swiotlb.static_pool_area_count()
        {
            printk::write_str("swiotlb static pool area locks invalid\n");
            return SmokeResult::Failed;
        }
    } else if swiotlb.static_pool_area_locks_ready()
        || swiotlb.static_pool_area_count() != 0
        || swiotlb.static_pool_lock_count() != 0
    {
        printk::write_str("swiotlb no-pool area facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "swiotlb required={} early_pool={} areas={} locks={} dynamic_trimmed={}\n",
        swiotlb.pool_required(),
        swiotlb.early_pool_ready(),
        swiotlb.static_pool_area_count(),
        swiotlb.static_pool_lock_count(),
        swiotlb.dynamic_growth_trimmed()
    ));
    SmokeResult::Passed
}
