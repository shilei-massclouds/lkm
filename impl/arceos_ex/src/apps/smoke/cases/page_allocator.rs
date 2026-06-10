use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        mm_core::{LinearMappedPageAddr, Pfn, PhysPageAddr},
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let page_allocator = &ctx.page_allocator;
    let page_metadata_map = &ctx.page_metadata_map;
    let zonelist = page_allocator.boot_zonelist_set();

    if page_metadata_map.state() != State::Ready
        || page_metadata_map.metadata_count() == 0
        || page_metadata_map.metadata_bytes() == 0
        || page_metadata_map.metadata_storage_size() < page_metadata_map.metadata_bytes()
        || page_metadata_map.metadata_storage().size() != page_metadata_map.metadata_storage_size()
        || page_metadata_map.metadata_storage_linear() == 0
        || page_metadata_map.start_pfn().value() >= page_metadata_map.end_pfn().value()
    {
        printk::write_str("page metadata map is invalid\n");
        return SmokeResult::Failed;
    }
    if page_allocator.state() != State::Ready {
        printk::write_str("page allocator is not ready\n");
        return SmokeResult::Failed;
    }
    if !page_allocator.page_metadata_map_bound() {
        printk::write_str("page allocator is not bound to page metadata map\n");
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

    let first_pfn = first_zone.range().start() / page_metadata_map.page_size();
    let Some(page) = page_metadata_map.pfn_to_page(Pfn::new(first_pfn)) else {
        printk::write_str("pfn_to_page failed\n");
        return SmokeResult::Failed;
    };
    let Some(page_pfn) = page_metadata_map.page_to_pfn(page) else {
        printk::write_str("page_to_pfn failed\n");
        return SmokeResult::Failed;
    };
    let Some(page_phys) = page_metadata_map.page_to_phys(page) else {
        printk::write_str("page_to_phys failed\n");
        return SmokeResult::Failed;
    };
    let Some(page_linear) = page_metadata_map.page_to_virt(page) else {
        printk::write_str("page_to_virt failed\n");
        return SmokeResult::Failed;
    };
    let Some(page_address) = page_metadata_map.page_address(page) else {
        printk::write_str("page_address failed\n");
        return SmokeResult::Failed;
    };
    let Some(page_from_phys) = page_metadata_map.phys_to_page(PhysPageAddr::new(page_phys.value()))
    else {
        printk::write_str("phys_to_page failed\n");
        return SmokeResult::Failed;
    };
    let Some(page_from_virt) =
        page_metadata_map.virt_to_page(LinearMappedPageAddr::new(page_linear.value()))
    else {
        printk::write_str("virt_to_page failed\n");
        return SmokeResult::Failed;
    };
    let Some(metadata_pfn) = page_metadata_map.page_metadata_pfn(page) else {
        printk::write_str("page metadata slot pfn failed\n");
        return SmokeResult::Failed;
    };
    if page_pfn.value() != first_pfn
        || page_phys.value() != first_zone.range().start()
        || page_linear.value() != page_address
        || page_from_phys != page
        || page_from_virt != page
        || metadata_pfn.value() != first_pfn
        || page.metadata_index() != first_pfn - page_metadata_map.start_pfn().value()
    {
        printk::write_str("page metadata conversion roundtrip invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "fallback={} totalram_pages={} zone_facts={} first_zone={:#x}..{:#x} mem_map={} bytes={} first_page={:#x} metadata={:#x}\n",
        zonelist.fallback_count(),
        page_allocator.totalram_pages(),
        page_allocator.zone_fact_count(),
        first_zone.range().start(),
        first_zone.range().end(),
        page_metadata_map.metadata_count(),
        page_metadata_map.metadata_bytes(),
        page_address,
        page.metadata_linear()
    ));
    SmokeResult::Passed
}
