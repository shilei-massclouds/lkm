use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        state::State,
        zones::{MigrationType, ZoneKind},
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let zones = &ctx.zones;

    if zones.state() != State::Ready {
        printk::write_str("zones are not ready\n");
        return SmokeResult::Failed;
    }
    if zones.zone_count() != 3 {
        printk::write_str("zones count is not three\n");
        return SmokeResult::Failed;
    }

    let Some(dma32) = zones.zone(ZoneKind::Dma32) else {
        printk::write_str("DMA32 zone missing\n");
        return SmokeResult::Failed;
    };
    let Some(normal) = zones.zone(ZoneKind::Normal) else {
        printk::write_str("NORMAL zone missing\n");
        return SmokeResult::Failed;
    };
    let Some(movable) = zones.zone(ZoneKind::Movable) else {
        printk::write_str("MOVABLE zone missing\n");
        return SmokeResult::Failed;
    };

    if dma32.is_empty() {
        printk::write_str("DMA32 zone unexpectedly empty\n");
        return SmokeResult::Failed;
    }
    if dma32.range().end() > 0x1_0000_0000 {
        printk::write_str("DMA32 zone exceeds 32-bit address range\n");
        return SmokeResult::Failed;
    }
    if !normal.is_empty() {
        printk::write_str("NORMAL zone should be empty on current smoke platform\n");
        return SmokeResult::Failed;
    }
    if !movable.is_empty() {
        printk::write_str("MOVABLE zone should be empty before movable policy\n");
        return SmokeResult::Failed;
    }

    for zone in [dma32, normal, movable] {
        if !zone.has_migration_type(MigrationType::Unmovable)
            || !zone.has_migration_type(MigrationType::Movable)
            || !zone.has_migration_type(MigrationType::Reclaimable)
        {
            printk::write_str("zone migration layer invalid\n");
            return SmokeResult::Failed;
        }
    }

    let managed_sum = [
        ctx.page_allocator
            .zone_managed_pages(ZoneKind::Dma32)
            .unwrap_or(0),
        ctx.page_allocator
            .zone_managed_pages(ZoneKind::Normal)
            .unwrap_or(0),
        ctx.page_allocator
            .zone_managed_pages(ZoneKind::Movable)
            .unwrap_or(0),
    ]
    .iter()
    .copied()
    .sum::<usize>();
    if managed_sum == 0 || managed_sum != ctx.page_allocator.totalram_pages() {
        printk::write_str("zone managed pages do not match totalram\n");
        return SmokeResult::Failed;
    }
    if ctx
        .page_allocator
        .zone_free_pages(ZoneKind::Dma32)
        .unwrap_or(0)
        == 0
    {
        printk::write_str("DMA32 free pages not accounted\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "dma32={:#x}..{:#x} normal_bytes={} movable_bytes={} totalram_pages={}\n",
        dma32.range().start(),
        dma32.range().end(),
        normal.present_bytes(),
        movable.present_bytes(),
        ctx.page_allocator.totalram_pages()
    ));
    SmokeResult::Passed
}
