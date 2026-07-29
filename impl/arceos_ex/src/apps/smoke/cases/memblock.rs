use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let page_size = ctx.config.page_size();

    let addr_space = &ctx.kernel_addr_space;
    let ranges = [
        addr_space.kernel_image_range(),
        addr_space.fix_map_range(),
        addr_space.linear_map_range(),
        addr_space.user_space_reserve_range(),
    ];
    if addr_space.state() != State::Online
        || !addr_space.layout_ready()
        || !addr_space.final_swapper_mappings_published()
        || ranges[0].start() != ctx.config.kernel_link_addr()
        || ranges[2].start() != ctx.config.linear_map_virt_start()
        || ranges[3].start() != 0
        || ranges[3].end() != ctx.config.canonical_user_virt_end()
    {
        printk::write_str("kernel address-space layout facts invalid\n");
        return SmokeResult::Failed;
    }
    let mut left = 0usize;
    while left < ranges.len() {
        if !ranges[left].valid() {
            printk::write_str("kernel address-space range is invalid\n");
            return SmokeResult::Failed;
        }
        let mut right = left + 1;
        while right < ranges.len() {
            if !ranges[left].disjoint(ranges[right]) {
                printk::write_str("kernel address-space ranges overlap\n");
                return SmokeResult::Failed;
            }
            right += 1;
        }
        left += 1;
    }

    let raw_dtb = &ctx.raw_dtb;
    if raw_dtb.state() != State::Ready
        || raw_dtb.header_range().start() != raw_dtb.range().start()
        || raw_dtb.header_range().size() != 40
        || raw_dtb.total_size() < raw_dtb.header_range().size()
        || raw_dtb.range().size() != raw_dtb.total_size()
        || !raw_dtb.handoff_blob_access_contract()
        || !raw_dtb.nodes_unparsed()
        || !ctx.fix_map.contains_raw_dtb(raw_dtb)
    {
        printk::write_str("raw DTB validation boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    if !crate::flows::boot_init_flow::boot_task_entry_preemption_initialized()
        || crate::flows::boot_init_flow::boot_task_entry_bind_count() != 2
        || crate::flows::boot_init_flow::boot_task_entry_bind_diagnostic() != 0
    {
        printk::write_str("boot task entry binding facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.memblock.state() != State::Offline {
        printk::write_str("memblock is not offline after mm core init\n");
        return SmokeResult::Failed;
    }
    if ctx.memblock.usable_ranges().count() == 0 {
        printk::write_str("memblock usable metadata missing after offline\n");
        return SmokeResult::Failed;
    }
    if ctx.memblock.reserved_ranges().count() == 0 {
        printk::write_str("memblock reserved metadata missing after offline\n");
        return SmokeResult::Failed;
    }
    if !ctx.memblock.entry_successor_setup_facts_ready() {
        printk::write_str("memblock entry successor setup facts missing\n");
        return SmokeResult::Failed;
    }
    if ctx.vm.swapper_vm().state() != State::Ready
        || ctx.kernel_addr_space.state() != State::Online
        || !ctx
            .cpu_group
            .boot_cpu()
            .is_some_and(|cpu| ctx.vm.swapper_vm().translation_sync_complete(cpu))
        || !ctx.vm.swapper_vm().strict_kernel_rwx_boundary_deferred()
        || !ctx.vm.swapper_vm().final_permissions_not_split_yet()
    {
        printk::write_str("swapper vm entry successor facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.memblock.alloc_phys(page_size, page_size).is_some() {
        printk::write_str("memblock allocation succeeded while offline\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "state=offline usable={} reserved={}\n",
        ctx.memblock.usable_ranges().count(),
        ctx.memblock.reserved_ranges().count()
    ));
    SmokeResult::Passed
}
