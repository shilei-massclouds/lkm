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
        || !page_allocator.buddy_free_page_sets_ready()
        || page_allocator.buddy_total_free_pages() == 0
        || page_allocator.buddy_free_block_count() == 0
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
        || page_allocator.buddy_zone_free_pages(first_zone.kind()) != Some(first_zone.free_pages())
    {
        printk::write_str("page allocator first zone fact invalid\n");
        return SmokeResult::Failed;
    }
    let mut zone_free_pages = 0usize;
    let mut zone_index = 0usize;
    while zone_index < page_allocator.zone_fact_count() {
        let Some(zone) = page_allocator.zone_fact(zone_index) else {
            printk::write_str("page allocator zone fact missing\n");
            return SmokeResult::Failed;
        };
        let Some(next) = zone_free_pages.checked_add(zone.free_pages()) else {
            printk::write_str("page allocator zone free pages overflow\n");
            return SmokeResult::Failed;
        };
        zone_free_pages = next;
        zone_index += 1;
    }
    if zone_free_pages != page_allocator.buddy_total_free_pages() {
        printk::write_str("page allocator buddy free page accounting invalid\n");
        return SmokeResult::Failed;
    }
    let Some(first_buddy_block) = page_allocator.first_buddy_free_block(page_metadata_map) else {
        printk::write_str("first buddy free block missing\n");
        return SmokeResult::Failed;
    };
    if page_metadata_map.page_metadata_is_buddy_free(first_buddy_block.page()) != Some(true)
        || page_metadata_map.page_metadata_buddy_order(first_buddy_block.page())
            != Some(first_buddy_block.order())
        || page_allocator
            .buddy_order_free_count(first_buddy_block.zone(), first_buddy_block.order())
            .unwrap_or(0)
            == 0
    {
        printk::write_str("first buddy free block metadata invalid\n");
        return SmokeResult::Failed;
    }
    let Some(first_buddy_metadata) = page_metadata_map.page_metadata(first_buddy_block.page())
    else {
        printk::write_str("first buddy free block metadata missing\n");
        return SmokeResult::Failed;
    };
    let order_count = page_allocator
        .buddy_order_free_count(first_buddy_block.zone(), first_buddy_block.order())
        .unwrap_or(0);
    if first_buddy_metadata.buddy_prev().is_some()
        || (first_buddy_metadata.buddy_next().is_some() && order_count <= 1)
    {
        printk::write_str("first buddy free block list links invalid\n");
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
        "fallback={} totalram_pages={} buddy_free={} buddy_blocks={} first_block_pages={} zone_facts={} first_zone={:#x}..{:#x} mem_map={} bytes={} first_page={:#x} metadata={:#x}\n",
        zonelist.fallback_count(),
        page_allocator.totalram_pages(),
        page_allocator.buddy_total_free_pages(),
        page_allocator.buddy_free_block_count(),
        first_buddy_block.pages(),
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
