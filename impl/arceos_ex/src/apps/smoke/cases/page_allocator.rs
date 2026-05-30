use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let page_allocator = &ctx.page_allocator;
    let zonelist = page_allocator.boot_zonelist_set();

    if page_allocator.state() != State::Ready {
        printk::write_str("page allocator is not ready\n");
        return SmokeResult::Failed;
    }
    if zonelist.state() != State::Ready
        || zonelist.fallback_count() == 0
        || !zonelist.null_sentinel_ready()
        || zonelist.fallback_zone_kind(0).is_none()
    {
        printk::write_str("boot zonelist fallback invalid\n");
        return SmokeResult::Failed;
    }
    if !page_allocator.cpuhp_step_registered()
        || !page_allocator.boot_pageset_checkpoint_ready()
        || !page_allocator.handoff_complete()
    {
        printk::write_str("page allocator handoff facts missing\n");
        return SmokeResult::Failed;
    }
    if ctx.memblock.state() != State::Offline || page_allocator.totalram_pages() == 0 {
        printk::write_str("page allocator accounting/offline state invalid\n");
        return SmokeResult::Failed;
    }

    let Some(first_zone) = page_allocator.zone_fact(0) else {
        printk::write_str("page allocator first zone fact missing\n");
        return SmokeResult::Failed;
    };
    if first_zone.range().start() >= first_zone.range().end()
        || first_zone.managed_pages() == 0
        || first_zone.free_pages() == 0
    {
        printk::write_str("page allocator first zone fact invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "fallback={} totalram_pages={} zone_facts={} first_zone={:#x}..{:#x}\n",
        zonelist.fallback_count(),
        page_allocator.totalram_pages(),
        page_allocator.zone_fact_count(),
        first_zone.range().start(),
        first_zone.range().end()
    ));
    SmokeResult::Passed
}
