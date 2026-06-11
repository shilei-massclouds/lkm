use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::{
        device::DeviceRef,
        ioremap::MmioMappingKind,
        mm_core::{GfpFlags, PageProtection, PageRef, VmapArea, VmapAreaFlags, VmapMapping},
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
const TEST_PAGE_COUNT: usize = 2;
pub const KUNIT_CASE_COUNT: usize = 4;

pub const HANDLER: Handler = Handler {
    name: "vmalloc_mapping",
    priority: 95,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, ctx: &mut Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    if !run_case(
        total,
        checkpoint,
        ctx,
        "vmalloc_mapping.multi_record",
        run_multi_record,
    ) || !run_case(
        total,
        checkpoint,
        ctx,
        "vmalloc_mapping.unmap_free",
        run_unmap_free,
    ) || !run_case(
        total,
        checkpoint,
        ctx,
        "vmalloc_mapping.ioremap_iounmap",
        run_ioremap_iounmap,
    ) || !run_case(
        total,
        checkpoint,
        ctx,
        "vmalloc_mapping.mmio_attributes",
        run_mmio_attributes,
    ) {
        CheckpointOutcome::FailAndShutdown
    } else {
        CheckpointOutcome::Continue
    }
}

fn run_case(
    total: usize,
    checkpoint: Checkpoint,
    ctx: &mut Context,
    name: &'static str,
    case: fn(&mut Context) -> bool,
) -> bool {
    kunit::start_case(total, "", name, checkpoint);
    let passed = case(ctx);
    if passed {
        kunit::pass(total, "", name);
    } else {
        kunit::fail(total, "", name, "vmalloc mapping facts invalid");
    }
    passed
}

fn run_multi_record(ctx: &mut Context) -> bool {
    let page_size = ctx.config.page_size();
    if page_size == 0 || !page_size.is_power_of_two() {
        return false;
    }

    let Some(first_page) = alloc_test_page(ctx) else {
        return false;
    };
    let Some(second_page) = alloc_test_page(ctx) else {
        let _ = ctx
            .page_allocator
            .free_pages(first_page.page, 0, &ctx.page_metadata_map);
        return false;
    };

    let passed = run_multi_record_with_pages(ctx, page_size, first_page.phys, second_page.phys);
    passed
        && ctx
            .page_allocator
            .free_pages(second_page.page, 0, &ctx.page_metadata_map)
        && ctx
            .page_allocator
            .free_pages(first_page.page, 0, &ctx.page_metadata_map)
}

fn run_unmap_free(ctx: &mut Context) -> bool {
    let page_size = ctx.config.page_size();
    if page_size == 0 || !page_size.is_power_of_two() {
        return false;
    };
    let Some(page) = alloc_test_page(ctx) else {
        return false;
    };

    let passed = run_unmap_free_with_page(ctx, page_size, page.phys);
    passed
        && ctx
            .page_allocator
            .free_pages(page.page, 0, &ctx.page_metadata_map)
}

fn run_ioremap_iounmap(ctx: &mut Context) -> bool {
    let page_size = ctx.config.page_size();
    if page_size <= 64 || !page_size.is_power_of_two() {
        return false;
    };
    let Some(page) = alloc_test_page(ctx) else {
        return false;
    };

    let passed = run_ioremap_iounmap_with_page(ctx, page_size, page.phys);
    passed
        && ctx
            .page_allocator
            .free_pages(page.page, 0, &ctx.page_metadata_map)
}

fn run_mmio_attributes(ctx: &mut Context) -> bool {
    let page_size = ctx.config.page_size();
    if page_size <= 128 || !page_size.is_power_of_two() {
        return false;
    };
    let Some(page) = alloc_test_page(ctx) else {
        return false;
    };

    let passed = run_mmio_attributes_with_page(ctx, page_size, page.phys);
    passed
        && ctx
            .page_allocator
            .free_pages(page.page, 0, &ctx.page_metadata_map)
}

fn run_multi_record_with_pages(
    ctx: &mut Context,
    page_size: usize,
    first_phys: usize,
    second_phys: usize,
) -> bool {
    if first_phys == second_phys {
        return false;
    }

    let allocator = &mut ctx.vmalloc_allocator;
    let base_area_count = allocator.area_count();
    let base_mapping_count = allocator.mapping_count();

    let Some(first_area) = allocator.get_vm_area(page_size, VmapAreaFlags::VmIoremap) else {
        return false;
    };
    let Some(second_area) = allocator.get_vm_area(page_size, VmapAreaFlags::VmIoremap) else {
        return false;
    };
    let Some(first_mapping) =
        allocator.map_page_range(first_area, first_phys, page_size, PageProtection::IoMemory)
    else {
        return false;
    };
    let Some(second_mapping) = allocator.map_page_range(
        second_area,
        second_phys,
        page_size,
        PageProtection::IoMemory,
    ) else {
        return false;
    };

    let records_valid = allocator.area_count() == base_area_count.saturating_add(TEST_PAGE_COUNT)
        && allocator.mapping_count() == base_mapping_count.saturating_add(TEST_PAGE_COUNT)
        && first_area.index() == base_area_count
        && second_area.index() == base_area_count.saturating_add(1)
        && first_area.busy()
        && second_area.busy()
        && first_area.vm_struct_metadata_ready()
        && first_area.vmap_area_metadata_ready()
        && second_area.vm_struct_metadata_ready()
        && second_area.vmap_area_metadata_ready()
        && first_area.is_vm_ioremap()
        && second_area.is_vm_ioremap()
        && first_area.end() <= second_area.virt_base()
        && allocator.area(first_area.index()) == Some(first_area)
        && allocator.area(second_area.index()) == Some(second_area)
        && first_mapping.index() == base_mapping_count
        && second_mapping.index() == base_mapping_count.saturating_add(1)
        && first_mapping != second_mapping
        && first_mapping.record_created()
        && second_mapping.record_created()
        && first_mapping.installed()
        && second_mapping.installed()
        && first_mapping.area() == first_area
        && second_mapping.area() == second_area
        && first_mapping.phys_base() == first_phys
        && second_mapping.phys_base() == second_phys
        && first_mapping.size() == page_size
        && second_mapping.size() == page_size
        && first_mapping.protection().is_io_memory()
        && second_mapping.protection().is_io_memory()
        && allocator.mapping(first_mapping.index()) == Some(first_mapping)
        && allocator.mapping(second_mapping.index()) == Some(second_mapping);

    records_valid
        && teardown_mapping(allocator, first_area, first_mapping, false)
        && teardown_mapping(allocator, second_area, second_mapping, false)
}

fn run_unmap_free_with_page(ctx: &mut Context, page_size: usize, phys: usize) -> bool {
    let allocator = &mut ctx.vmalloc_allocator;
    let base_area_count = allocator.area_count();
    let base_mapping_count = allocator.mapping_count();

    let Some(area) = allocator.get_vm_area(page_size, VmapAreaFlags::VmIoremap) else {
        return false;
    };
    let Some(mapping) = allocator.map_page_range(area, phys, page_size, PageProtection::IoMemory)
    else {
        return false;
    };
    if area.index() != base_area_count
        || mapping.index() != base_mapping_count
        || allocator.area(area.index()) != Some(area)
        || allocator.mapping(mapping.index()) != Some(mapping)
    {
        return false;
    }

    teardown_mapping(allocator, area, mapping, true)
}

fn run_ioremap_iounmap_with_page(ctx: &mut Context, page_size: usize, phys: usize) -> bool {
    let device = DeviceRef::new(usize::MAX - 1);
    let phys_base = phys.saturating_add(16);
    let size = 32usize;
    let base_ioremap_count = ctx.ioremap.mapping_count();
    let base_area_count = ctx.vmalloc_allocator.area_count();
    let base_mapping_count = ctx.vmalloc_allocator.mapping_count();

    let Some(mapping) =
        ctx.ioremap
            .map_device_mmio(&mut ctx.vmalloc_allocator, device, phys_base, size)
    else {
        return false;
    };
    let vmap_area = mapping.vmap_area();
    let vmap_mapping = mapping.vmap_mapping();
    let records_valid = mapping.active()
        && !mapping.unmapped()
        && mapping.device() == device
        && mapping.phys_base() == phys_base
        && mapping.page_phys_base() == phys
        && mapping.mapped_size() == page_size
        && mapping.membase() == mapping.virt_base().saturating_add(16)
        && mapping.uses_vm_ioremap()
        && mapping.uses_io_page_protection()
        && ctx.ioremap.mapping_count() == base_ioremap_count.saturating_add(1)
        && ctx.ioremap.mapping_for_device(device) == Some(mapping)
        && ctx.vmalloc_allocator.area_count() == base_area_count.saturating_add(1)
        && ctx.vmalloc_allocator.mapping_count() == base_mapping_count.saturating_add(1)
        && ctx.vmalloc_allocator.area(vmap_area.index()) == Some(vmap_area)
        && ctx.vmalloc_allocator.mapping(vmap_mapping.index()) == Some(vmap_mapping)
        && vmap_area.busy()
        && vmap_mapping.installed()
        && !vmap_mapping.removed();
    if !records_valid {
        return false;
    }

    if !ctx
        .ioremap
        .iounmap(&mut ctx.vmalloc_allocator, mapping.membase())
    {
        return false;
    }
    let Some(retired_mapping) = ctx.ioremap.mapping(base_ioremap_count) else {
        return false;
    };
    let Some(removed_vmap_mapping) = ctx.vmalloc_allocator.mapping(vmap_mapping.index()) else {
        return false;
    };
    let Some(released_area) = ctx.vmalloc_allocator.area(vmap_area.index()) else {
        return false;
    };

    retired_mapping.device() == device
        && !retired_mapping.active()
        && retired_mapping.unmapped()
        && ctx.ioremap.mapping_for_device(device).is_none()
        && removed_vmap_mapping.record_created()
        && !removed_vmap_mapping.installed()
        && removed_vmap_mapping.removed()
        && !released_area.busy()
        && released_area.released()
        && !ctx
            .ioremap
            .iounmap(&mut ctx.vmalloc_allocator, mapping.membase())
}

fn run_mmio_attributes_with_page(ctx: &mut Context, page_size: usize, phys: usize) -> bool {
    let device = DeviceRef::new(usize::MAX - 2);
    let unsupported_device = DeviceRef::new(usize::MAX - 3);

    if !ctx.ioremap.mmio_attribute_policy_ready()
        || !ctx.ioremap.plain_device_attribute_supported()
        || !ctx.ioremap.noncached_attribute_deferred()
        || !ctx.ioremap.writecombine_attribute_deferred()
        || !ctx.ioremap.normal_memory_attribute_deferred()
    {
        return false;
    }

    let base_ioremap_count = ctx.ioremap.mapping_count();
    let base_area_count = ctx.vmalloc_allocator.area_count();
    let base_mapping_count = ctx.vmalloc_allocator.mapping_count();
    let Some(mapping) = ctx.ioremap.map_device_mmio_with_kind(
        &mut ctx.vmalloc_allocator,
        device,
        phys,
        page_size,
        MmioMappingKind::PlainDevice,
    ) else {
        return false;
    };
    let vmap_mapping = mapping.vmap_mapping();

    let plain_device_valid = mapping.kind().is_plain_device()
        && mapping.arch_attr().is_riscv_page_ioremap()
        && mapping.uses_plain_device_attribute()
        && !mapping.claims_noncached()
        && !mapping.claims_writecombine()
        && !mapping.claims_normal_memory()
        && vmap_mapping.protection().is_io_memory()
        && vmap_mapping.protection_kind().is_io_memory()
        && ctx.ioremap.mapping_count() == base_ioremap_count.saturating_add(1)
        && ctx.vmalloc_allocator.area_count() == base_area_count.saturating_add(1)
        && ctx.vmalloc_allocator.mapping_count() == base_mapping_count.saturating_add(1);
    if !plain_device_valid {
        return false;
    }

    let unsupported_rejected = ctx
        .ioremap
        .map_device_mmio_with_kind(
            &mut ctx.vmalloc_allocator,
            unsupported_device,
            phys,
            page_size,
            MmioMappingKind::NonCached,
        )
        .is_none()
        && ctx
            .ioremap
            .map_device_mmio_with_kind(
                &mut ctx.vmalloc_allocator,
                unsupported_device,
                phys,
                page_size,
                MmioMappingKind::WriteCombine,
            )
            .is_none()
        && ctx
            .ioremap
            .map_device_mmio_with_kind(
                &mut ctx.vmalloc_allocator,
                unsupported_device,
                phys,
                page_size,
                MmioMappingKind::NormalMemory,
            )
            .is_none()
        && ctx.ioremap.mapping_count() == base_ioremap_count.saturating_add(1)
        && ctx.vmalloc_allocator.area_count() == base_area_count.saturating_add(1)
        && ctx.vmalloc_allocator.mapping_count() == base_mapping_count.saturating_add(1);
    if !unsupported_rejected {
        return false;
    }

    ctx.ioremap
        .iounmap(&mut ctx.vmalloc_allocator, mapping.membase())
}

struct AllocatedPage {
    page: PageRef,
    phys: usize,
}

fn alloc_test_page(ctx: &mut Context) -> Option<AllocatedPage> {
    let page = ctx
        .page_allocator
        .alloc_page(GfpFlags::kernel(), &ctx.page_metadata_map)?;
    let Some(phys) = ctx
        .page_metadata_map
        .page_to_phys(page)
        .map(|addr| addr.value())
    else {
        let _ = ctx
            .page_allocator
            .free_pages(page, 0, &ctx.page_metadata_map);
        return None;
    };
    Some(AllocatedPage { page, phys })
}

fn teardown_mapping(
    allocator: &mut crate::objects::mm_core::VmallocAllocator,
    area: VmapArea,
    mapping: VmapMapping,
    check_repeat_rejected: bool,
) -> bool {
    if !allocator.unmap_page_range(mapping) {
        return false;
    }
    let Some(removed_mapping) = allocator.mapping(mapping.index()) else {
        return false;
    };
    if !removed_mapping.record_created()
        || removed_mapping.installed()
        || !removed_mapping.removed()
        || removed_mapping.area() != area
    {
        return false;
    }
    if check_repeat_rejected && allocator.unmap_page_range(mapping) {
        return false;
    }

    if !allocator.free_vm_area(area) {
        return false;
    }
    let Some(released_area) = allocator.area(area.index()) else {
        return false;
    };
    if released_area.busy()
        || !released_area.released()
        || released_area.vm_struct_metadata_ready()
        || released_area.vmap_area_metadata_ready()
    {
        return false;
    }
    if check_repeat_rejected && allocator.free_vm_area(area) {
        return false;
    }
    true
}
