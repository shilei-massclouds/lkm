use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::{mm_core::PageAllocator, raw_dtb::PhysRange, state::State},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PageAllocatorMemBlockHandoffReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "page_allocator",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Read(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    kunit::start_case(total, "", HANDLER.name, checkpoint);

    let page_allocator = &ctx.page_allocator;
    let page_metadata_map = &ctx.page_metadata_map;
    let page_size = ctx.config.page_size();
    let Some(expected_free_pages) = expected_free_pages(ctx, page_size) else {
        kunit::fail(
            total,
            "",
            HANDLER.name,
            "expected free page accounting failed",
        );
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(zone_free_pages) = zone_free_pages(page_allocator) else {
        kunit::fail(total, "", HANDLER.name, "zone free page accounting failed");
        return CheckpointOutcome::FailAndShutdown;
    };

    if ctx.memblock.state() != State::Offline
        || page_allocator.state() != State::Ready
        || page_metadata_map.state() != State::Ready
        || !page_allocator.handoff_complete()
        || !page_allocator.buddy_free_page_sets_ready()
        || page_allocator.buddy_total_free_pages() == 0
        || page_allocator.buddy_free_block_count() == 0
        || page_allocator.buddy_total_free_pages() != expected_free_pages
        || zone_free_pages != expected_free_pages
    {
        kunit::diag_usize("expected_free_pages", expected_free_pages);
        kunit::diag_usize("buddy_free_pages", page_allocator.buddy_total_free_pages());
        kunit::diag_usize("zone_free_pages", zone_free_pages);
        kunit::fail(total, "", HANDLER.name, "handoff facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    let Some(block) = page_allocator.first_buddy_free_block(page_metadata_map) else {
        kunit::fail(total, "", HANDLER.name, "first buddy block missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(is_free) = page_metadata_map.page_metadata_is_buddy_free(block.page()) else {
        kunit::fail(total, "", HANDLER.name, "buddy metadata missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(order) = page_metadata_map.page_metadata_buddy_order(block.page()) else {
        kunit::fail(total, "", HANDLER.name, "buddy order missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(order_count) = page_allocator.buddy_order_free_count(block.zone(), block.order())
    else {
        kunit::fail(total, "", HANDLER.name, "buddy order count missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    if !is_free || order != block.order() || order_count == 0 {
        kunit::fail(total, "", HANDLER.name, "buddy block metadata invalid");
        return CheckpointOutcome::FailAndShutdown;
    }
    let Some(metadata) = page_metadata_map.page_metadata(block.page()) else {
        kunit::fail(total, "", HANDLER.name, "buddy metadata read failed");
        return CheckpointOutcome::FailAndShutdown;
    };
    if metadata.buddy_prev().is_some() {
        kunit::fail(total, "", HANDLER.name, "buddy list head prev link invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    kunit::diag_usize("expected_free_pages", expected_free_pages);
    kunit::diag_usize("buddy_free_pages", page_allocator.buddy_total_free_pages());
    kunit::diag_usize("buddy_blocks", page_allocator.buddy_free_block_count());
    kunit::pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn expected_free_pages(ctx: &Context, page_size: usize) -> Option<usize> {
    if page_size == 0 || !page_size.is_power_of_two() {
        return None;
    }

    let mut pages = 0usize;
    let usable = ctx.memblock.usable_ranges();
    let mut range_index = 0usize;
    while range_index < usable.count() {
        let range = usable.get(range_index)?;
        let mut zone_index = 0usize;
        while zone_index < ctx.page_allocator.zone_fact_count() {
            let zone = ctx.page_allocator.zone_fact(zone_index)?;
            let start = range.start().max(zone.range().start());
            let end = range.end().min(zone.range().end());
            if start < end {
                pages = pages.checked_add(free_pages_in_range(ctx, start, end, page_size)?)?;
            }
            zone_index += 1;
        }
        range_index += 1;
    }
    Some(pages)
}

fn free_pages_in_range(ctx: &Context, start: usize, end: usize, page_size: usize) -> Option<usize> {
    let mut cursor = start;
    let mut pages = 0usize;
    while cursor < end {
        let Some(reserved) = next_reserved_overlap(ctx, cursor, end) else {
            return pages.checked_add(page_count(cursor, end, page_size)?);
        };
        if reserved.start() > cursor {
            pages = pages.checked_add(page_count(cursor, reserved.start(), page_size)?)?;
        }
        cursor = cursor.max(reserved.end());
    }
    Some(pages)
}

fn next_reserved_overlap(ctx: &Context, start: usize, end: usize) -> Option<PhysRange> {
    let reserved = ctx.memblock.reserved_ranges();
    let mut best: Option<PhysRange> = None;
    let mut index = 0usize;
    while index < reserved.count() {
        let range = reserved.get(index)?;
        if range.start() < end && start < range.end() {
            best = match best {
                Some(current) if current.start() <= range.start() => Some(current),
                _ => Some(range),
            };
        }
        index += 1;
    }
    best
}

fn page_count(start: usize, end: usize, page_size: usize) -> Option<usize> {
    let aligned_start = round_up(start, page_size)?;
    let aligned_end = round_down(end, page_size)?;
    if aligned_start >= aligned_end {
        return Some(0);
    }
    Some((aligned_end - aligned_start) / page_size)
}

fn round_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    value.checked_add(mask).map(|sum| sum & !mask)
}

fn round_down(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    Some(value & !(align - 1))
}

fn zone_free_pages(page_allocator: &PageAllocator) -> Option<usize> {
    let mut pages = 0usize;
    let mut index = 0usize;
    while index < page_allocator.zone_fact_count() {
        let zone = page_allocator.zone_fact(index)?;
        pages = pages.checked_add(page_allocator.zone_free_pages(zone.kind())?)?;
        if page_allocator.buddy_zone_free_pages(zone.kind())? != zone.free_pages() {
            return None;
        }
        index += 1;
    }
    Some(pages)
}
