use crate::{
    apps::smoke::SmokeResult,
    context::{context, Context},
    objects::{printk, state::State},
};

const SMOKE_IOUNMAP_PHYS_BASE: usize = 0x0c00_0000;
const SMOKE_IOUNMAP_SIZE: usize = 0x1000;

pub fn run() -> SmokeResult {
    let ctx = context();
    let (vmap_start, vmap_end, node_count) = {
        let vmalloc = &ctx.vmalloc_allocator;
        let address_space = vmalloc.address_space();

        if vmalloc.state() != State::Ready
            || !vmalloc.initialized()
            || !vmalloc.vmap_area_api_ready()
            || !vmalloc.page_range_mapping_api_ready()
            || !vmalloc.vm_struct_metadata_ready()
            || !vmalloc.vmap_area_metadata_ready()
            || !vmalloc.mapping_policy_external()
            || !vmalloc.physical_resource_policy_external()
            || !vmalloc.runtime_page_table_mapping_ready()
            || !vmalloc.mapping_guard_contract_ready()
            || !vmalloc.unmapping_guard_contract_ready()
            || !vmalloc.mapping_sync_contract_ready()
            || !vmalloc.unmapping_flush_contract_ready()
            || !vmalloc.failure_rollback_contract_ready()
            || !vmalloc.cross_cpu_vmalloc_flush_deferred()
            || !vmalloc.reclaim_hook_ready()
        {
            printk::write_str("vmalloc allocator is not ready\n");
            return SmokeResult::Failed;
        }
        if vmalloc.area_cache().state() != State::Ready
            || address_space.state() != State::Ready
            || vmalloc.node_set().state() != State::Ready
            || vmalloc.block_queues().state() != State::Ready
            || vmalloc.deferred_set().state() != State::Ready
        {
            printk::write_str("vmap child object state invalid\n");
            return SmokeResult::Failed;
        }
        if address_space.start() >= address_space.end()
            || !address_space.free_space_ready()
            || !address_space.existing_vmlist_busy_imported()
            || vmalloc.node_set().node_count() == 0
            || !vmalloc.node_set().route_ready()
            || !vmalloc.node_set().guard_contract_ready()
            || !vmalloc.block_queues().fast_path_metadata_ready()
            || !vmalloc.deferred_set().work_ready()
            || !vmalloc.deferred_set().guard_contract_ready()
            || !vmalloc.deferred_set().rcu_runtime_path_deferred()
        {
            printk::write_str("vmap address-space facts invalid\n");
            return SmokeResult::Failed;
        }
        (
            address_space.start(),
            address_space.end(),
            vmalloc.node_set().node_count(),
        )
    };

    if !check_runtime_platform_ioremap_binding(ctx) {
        return SmokeResult::Failed;
    }
    if !check_iounmap_teardown(ctx) {
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "vmalloc={:#x}..{:#x} nodes={}\n",
        vmap_start, vmap_end, node_count
    ));
    SmokeResult::Passed
}

fn check_runtime_platform_ioremap_binding(ctx: &Context) -> bool {
    if let Some(rng_device) = ctx.virtio_bus.rng_device() {
        return check_platform_device_ioremap_binding(ctx, rng_device.platform_device_ref());
    }

    if let Some(serial_device) = crate::objects::ns16550a::uart8250_port_device_ref() {
        return check_platform_device_ioremap_binding(ctx, serial_device);
    }

    printk::write_str("virtio rng and serial ioremap bindings missing\n");
    false
}

fn check_platform_device_ioremap_binding(
    ctx: &Context,
    platform_device: crate::objects::device::DeviceRef,
) -> bool {
    let Some(mapping) = ctx.ioremap.mapping_for_device(platform_device) else {
        printk::write_str("platform ioremap mapping missing\n");
        return false;
    };
    if mapping.membase() == mapping.phys_base()
        || !mapping.page_aligned()
        || !mapping.uses_vm_ioremap()
        || !mapping.uses_io_page_protection()
    {
        printk::write_str("platform ioremap mapping facts invalid\n");
        return false;
    }
    true
}

fn check_iounmap_teardown(ctx: &mut Context) -> bool {
    let Some(mapping) = ctx.ioremap.map_system_irqchip_mmio(
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_table_caches,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &ctx.config,
        "smoke-iounmap",
        SMOKE_IOUNMAP_PHYS_BASE,
        SMOKE_IOUNMAP_SIZE,
    ) else {
        printk::write_str("smoke iounmap setup mapping failed\n");
        return false;
    };

    let membase = mapping.membase();
    if !ctx.ioremap.iounmap(&mut ctx.vmalloc_allocator, membase) {
        printk::write_str("smoke iounmap failed\n");
        return false;
    }
    if ctx.ioremap.iounmap(&mut ctx.vmalloc_allocator, membase) {
        printk::write_str("smoke duplicate iounmap succeeded\n");
        return false;
    }
    if ctx
        .ioremap
        .mapping_for_system_irqchip("smoke-iounmap")
        .is_some()
    {
        printk::write_str("smoke iounmap mapping still active\n");
        return false;
    }
    true
}
