use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::mm_core::{GfpFlags, PageProtection, PageRef, VmapAreaFlags},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
const TEST_PAGE_COUNT: usize = 2;
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "vmalloc_mapping",
    priority: 95,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, ctx: &mut Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let name = "vmalloc_mapping.multi_record";
    kunit::start_case(total, "", name, checkpoint);

    if run_multi_record(ctx) {
        kunit::pass(total, "", name);
        CheckpointOutcome::Continue
    } else {
        kunit::fail(total, "", name, "multi mapping record facts invalid");
        CheckpointOutcome::FailAndShutdown
    }
}

fn run_multi_record(ctx: &mut Context) -> bool {
    let page_size = ctx.config.page_size();
    if page_size == 0 || !page_size.is_power_of_two() {
        return false;
    }

    let Some(first_page) = ctx
        .page_allocator
        .alloc_page(GfpFlags::kernel(), &ctx.page_metadata_map)
    else {
        return false;
    };
    let Some(second_page) = ctx
        .page_allocator
        .alloc_page(GfpFlags::kernel(), &ctx.page_metadata_map)
    else {
        let _ = ctx
            .page_allocator
            .free_pages(first_page, 0, &ctx.page_metadata_map);
        return false;
    };

    // Keep the pages owned by these mappings until unmap/vfree exists.
    run_multi_record_with_pages(ctx, page_size, first_page, second_page)
}

fn run_multi_record_with_pages(
    ctx: &mut Context,
    page_size: usize,
    first_page: PageRef,
    second_page: PageRef,
) -> bool {
    let Some(first_phys) = ctx
        .page_metadata_map
        .page_to_phys(first_page)
        .map(|addr| addr.value())
    else {
        return false;
    };
    let Some(second_phys) = ctx
        .page_metadata_map
        .page_to_phys(second_page)
        .map(|addr| addr.value())
    else {
        return false;
    };
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

    allocator.area_count() == base_area_count.saturating_add(TEST_PAGE_COUNT)
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
        && allocator.mapping(second_mapping.index()) == Some(second_mapping)
}
