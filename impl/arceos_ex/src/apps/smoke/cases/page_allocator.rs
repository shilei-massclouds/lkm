use crate::{
    apps::smoke::SmokeResult,
    context::{context, Context},
    objects::{
        mm_core::{GfpFlags, LinearMappedPageAddr, PageRef, Pfn, PhysPageAddr},
        printk,
        state::State,
    },
};

const PAGE_ALLOCATOR_API_TEST_WORD: usize = 0x5041_4745_4150_4930;
const PAGE_ALLOCATOR_CONVENIENCE_TEST_WORD: usize = 0x5041_4745_4150_4931;
const PAGE_ALLOCATOR_ORDER_TEST_WORD: usize = 0x5041_4745_4150_4940;
const PAGE_ALLOCATOR_MAX_TEST_ORDER: usize = 2;

pub fn run() -> SmokeResult {
    let ctx = context();

    let Some(diag) = check_handoff_and_conversions(ctx) else {
        return SmokeResult::Failed;
    };
    let Some((allocated_page_address, allocated_page_phys, tested_max_order)) =
        run_alloc_free_api_smoke(ctx)
    else {
        return SmokeResult::Failed;
    };

    let page_allocator = &ctx.page_allocator;
    let page_metadata_map = &ctx.page_metadata_map;
    let zonelist = page_allocator.boot_zonelist_set();

    printk::write_fmt(format_args!(
        "fallback={} totalram_pages={} buddy_free={} buddy_blocks={} first_block_pages={} zone_facts={} first_zone={:#x}..{:#x} mem_map={} bytes={} first_page={:#x} metadata={:#x} alloc_page={:#x} alloc_phys={:#x} max_order={} gfp={:#x}\n",
        zonelist.fallback_count(),
        page_allocator.totalram_pages(),
        page_allocator.buddy_total_free_pages(),
        page_allocator.buddy_free_block_count(),
        diag.first_buddy_block_pages,
        page_allocator.zone_fact_count(),
        diag.first_zone_start,
        diag.first_zone_end,
        page_metadata_map.metadata_count(),
        page_metadata_map.metadata_bytes(),
        diag.first_page_address,
        diag.first_page_metadata,
        allocated_page_address,
        allocated_page_phys,
        tested_max_order,
        GfpFlags::kernel().bits()
    ));
    SmokeResult::Passed
}

struct PageAllocatorDiag {
    first_buddy_block_pages: usize,
    first_zone_start: usize,
    first_zone_end: usize,
    first_page_address: usize,
    first_page_metadata: usize,
}

fn check_handoff_and_conversions(ctx: &Context) -> Option<PageAllocatorDiag> {
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
        return None;
    }
    if page_allocator.state() != State::Ready {
        printk::write_str("page allocator is not ready\n");
        return None;
    }
    if !page_allocator.page_metadata_map_bound() {
        printk::write_str("page allocator is not bound to page metadata map\n");
        return None;
    }
    if zonelist.state() != State::Ready
        || zonelist.fallback_count() == 0
        || !zonelist.null_sentinel_ready()
        || zonelist.fallback_zone_kind(0).is_none()
    {
        printk::write_str("boot zonelist fallback invalid\n");
        return None;
    }
    if !page_allocator.cpuhp_step_registered()
        || !page_allocator.boot_pageset_checkpoint_ready()
        || !page_allocator.handoff_complete()
        || !page_allocator.buddy_free_page_sets_ready()
        || page_allocator.buddy_total_free_pages() == 0
        || page_allocator.buddy_free_block_count() == 0
    {
        printk::write_str("page allocator handoff facts missing\n");
        return None;
    }
    if ctx.memblock.state() != State::Offline || page_allocator.totalram_pages() == 0 {
        printk::write_str("page allocator accounting/offline state invalid\n");
        return None;
    }

    let Some(first_zone) = page_allocator.zone_fact(0) else {
        printk::write_str("page allocator first zone fact missing\n");
        return None;
    };
    if first_zone.range().start() >= first_zone.range().end()
        || first_zone.managed_pages() == 0
        || first_zone.free_pages() == 0
        || page_allocator.buddy_zone_free_pages(first_zone.kind()) != Some(first_zone.free_pages())
    {
        printk::write_str("page allocator first zone fact invalid\n");
        return None;
    }
    let mut zone_free_pages = 0usize;
    let mut zone_index = 0usize;
    while zone_index < page_allocator.zone_fact_count() {
        let Some(zone) = page_allocator.zone_fact(zone_index) else {
            printk::write_str("page allocator zone fact missing\n");
            return None;
        };
        let Some(next) = zone_free_pages.checked_add(zone.free_pages()) else {
            printk::write_str("page allocator zone free pages overflow\n");
            return None;
        };
        zone_free_pages = next;
        zone_index += 1;
    }
    if zone_free_pages != page_allocator.buddy_total_free_pages() {
        printk::write_str("page allocator buddy free page accounting invalid\n");
        return None;
    }
    let Some(first_buddy_block) = page_allocator.first_buddy_free_block(page_metadata_map) else {
        printk::write_str("first buddy free block missing\n");
        return None;
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
        return None;
    }
    let Some(first_buddy_metadata) = page_metadata_map.page_metadata(first_buddy_block.page())
    else {
        printk::write_str("first buddy free block metadata missing\n");
        return None;
    };
    let order_count = page_allocator
        .buddy_order_free_count(first_buddy_block.zone(), first_buddy_block.order())
        .unwrap_or(0);
    if first_buddy_metadata.buddy_prev().is_some()
        || (first_buddy_metadata.buddy_next().is_some() && order_count <= 1)
    {
        printk::write_str("first buddy free block list links invalid\n");
        return None;
    }

    let first_pfn = first_zone.range().start() / page_metadata_map.page_size();
    let Some(page) = page_metadata_map.pfn_to_page(Pfn::new(first_pfn)) else {
        printk::write_str("pfn_to_page failed\n");
        return None;
    };
    let Some(page_pfn) = page_metadata_map.page_to_pfn(page) else {
        printk::write_str("page_to_pfn failed\n");
        return None;
    };
    let Some(page_phys) = page_metadata_map.page_to_phys(page) else {
        printk::write_str("page_to_phys failed\n");
        return None;
    };
    let Some(page_linear) = page_metadata_map.page_to_virt(page) else {
        printk::write_str("page_to_virt failed\n");
        return None;
    };
    let Some(page_address) = page_metadata_map.page_address(page) else {
        printk::write_str("page_address failed\n");
        return None;
    };
    let Some(page_from_phys) = page_metadata_map.phys_to_page(PhysPageAddr::new(page_phys.value()))
    else {
        printk::write_str("phys_to_page failed\n");
        return None;
    };
    let Some(page_from_virt) =
        page_metadata_map.virt_to_page(LinearMappedPageAddr::new(page_linear.value()))
    else {
        printk::write_str("virt_to_page failed\n");
        return None;
    };
    let Some(metadata_pfn) = page_metadata_map.page_metadata_pfn(page) else {
        printk::write_str("page metadata slot pfn failed\n");
        return None;
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
        return None;
    }

    Some(PageAllocatorDiag {
        first_buddy_block_pages: first_buddy_block.pages(),
        first_zone_start: first_zone.range().start(),
        first_zone_end: first_zone.range().end(),
        first_page_address: page_address,
        first_page_metadata: page.metadata_linear(),
    })
}

fn run_alloc_free_api_smoke(ctx: &mut Context) -> Option<(usize, usize, usize)> {
    let page = ctx
        .page_allocator
        .alloc_pages(0, GfpFlags::kernel(), &ctx.page_metadata_map)?;
    let page_address = ctx.page_metadata_map.page_address(page)?;
    let page_phys = ctx.page_metadata_map.page_to_phys(page)?.value();
    if !write_read_word(page_address, PAGE_ALLOCATOR_API_TEST_WORD) {
        printk::write_str("alloc_pages linear mapping readback failed\n");
        return None;
    }
    if !ctx
        .page_allocator
        .free_pages(page, 0, &ctx.page_metadata_map)
    {
        printk::write_str("free_pages failed\n");
        return None;
    }

    let convenience_page = ctx
        .page_allocator
        .alloc_page(GfpFlags::empty(), &ctx.page_metadata_map)?;
    let convenience_address = ctx.page_metadata_map.page_address(convenience_page)?;
    if !write_read_word(convenience_address, PAGE_ALLOCATOR_CONVENIENCE_TEST_WORD) {
        printk::write_str("alloc_page linear mapping readback failed\n");
        return None;
    }
    if !ctx
        .page_allocator
        .free_pages(convenience_page, 0, &ctx.page_metadata_map)
    {
        printk::write_str("free_pages for alloc_page failed\n");
        return None;
    }

    if !run_order_alloc_free_smoke(ctx, 1, true) || !run_order_alloc_free_smoke(ctx, 2, false) {
        return None;
    }

    Some((page_address, page_phys, PAGE_ALLOCATOR_MAX_TEST_ORDER))
}

fn run_order_alloc_free_smoke(ctx: &mut Context, order: usize, check_wrong_order: bool) -> bool {
    let Some(page) =
        ctx.page_allocator
            .alloc_pages(order, GfpFlags::kernel(), &ctx.page_metadata_map)
    else {
        printk::write_fmt(format_args!("alloc_pages order {} failed\n", order));
        return false;
    };

    if check_wrong_order
        && ctx
            .page_allocator
            .free_pages(page, order - 1, &ctx.page_metadata_map)
    {
        printk::write_str("free_pages accepted mismatched order\n");
        return false;
    }

    if !check_allocated_block(ctx, page, order) {
        return false;
    }
    if !ctx
        .page_allocator
        .free_pages(page, order, &ctx.page_metadata_map)
    {
        printk::write_fmt(format_args!("free_pages order {} failed\n", order));
        return false;
    }
    true
}

fn check_allocated_block(ctx: &Context, page: PageRef, order: usize) -> bool {
    let page_count = 1usize << order;
    let page_size = ctx.page_metadata_map.page_size();
    let Some(base_phys) = ctx
        .page_metadata_map
        .page_to_phys(page)
        .map(|addr| addr.value())
    else {
        printk::write_str("allocated block base phys conversion failed\n");
        return false;
    };
    if !page.pfn().value().is_multiple_of(page_count) {
        printk::write_fmt(format_args!(
            "allocated block order {} pfn alignment invalid\n",
            order
        ));
        return false;
    }

    let mut offset = 0usize;
    while offset < page_count {
        let Some(pfn) = page.pfn().value().checked_add(offset) else {
            printk::write_str("allocated block pfn overflow\n");
            return false;
        };
        let Some(current_page) = ctx.page_metadata_map.pfn_to_page(Pfn::new(pfn)) else {
            printk::write_str("allocated block pfn_to_page failed\n");
            return false;
        };
        let Some(current_phys) = ctx
            .page_metadata_map
            .page_to_phys(current_page)
            .map(|addr| addr.value())
        else {
            printk::write_str("allocated block page_to_phys failed\n");
            return false;
        };
        let Some(phys_offset) = offset.checked_mul(page_size) else {
            printk::write_str("allocated block phys offset overflow\n");
            return false;
        };
        let Some(expected_phys) = base_phys.checked_add(phys_offset) else {
            printk::write_str("allocated block expected phys overflow\n");
            return false;
        };
        if current_phys != expected_phys {
            printk::write_fmt(format_args!(
                "allocated block order {} phys continuity invalid\n",
                order
            ));
            return false;
        }
        let Some(address) = ctx.page_metadata_map.page_address(current_page) else {
            printk::write_str("allocated block page_address failed\n");
            return false;
        };
        if !write_read_word(
            address,
            PAGE_ALLOCATOR_ORDER_TEST_WORD ^ (order << 8) ^ offset,
        ) {
            printk::write_fmt(format_args!(
                "allocated block order {} readback failed\n",
                order
            ));
            return false;
        }
        offset += 1;
    }
    true
}

fn write_read_word(address: usize, value: usize) -> bool {
    if !address.is_multiple_of(core::mem::align_of::<usize>()) {
        return false;
    }
    let word = address as *mut usize;
    unsafe {
        core::ptr::write_volatile(word, value);
        core::ptr::read_volatile(word) == value
    }
}
