use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        block_device::{
            BlockDeviceProviderKind, VIRTBLK_FIRST_MINOR, VIRTBLK_MAJOR, VIRTBLK_MINORS,
        },
        state::State,
        virtio_blk,
        virtio_mmio::{self, VIRTIO_ID_BLOCK, VIRTIO_MMIO_PLATFORM_DRIVER_REF},
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::VirtioBlkReady, Checkpoint::VirtioBlkReadReady];
pub const KUNIT_CASE_COUNT: usize = 2;

pub const HANDLER: Handler = Handler {
    name: "virtio_blk.discovery_config",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    if checkpoint == Checkpoint::VirtioBlkReady {
        return run_discovery_config(checkpoint, ctx, sink, total);
    }
    run_read_completion(checkpoint, ctx, sink, total)
}

fn run_discovery_config(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) -> CheckpointOutcome {
    sink.start_case(total, "", HANDLER.name, checkpoint);
    if !blk_facts_valid(ctx) {
        sink.fail(total, "", HANDLER.name, "virtio blk facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    let runtime = &ctx.virtio_blk_runtime;
    let Some(device) = runtime.device() else {
        sink.fail(total, "", HANDLER.name, "virtio blk device missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    let virtio_device = device.virtio_device();
    let capacity = device.capacity().unwrap_or(0);
    sink.diag_usize(
        "virtio_blk_probe_attempted",
        runtime.real_probe_attempted() as usize,
    );
    sink.diag_usize(
        "virtio_blk_probe_succeeded",
        runtime.real_probe_succeeded() as usize,
    );
    sink.diag_usize("virtio_blk_device_id", virtio_device.device_id() as usize);
    sink.diag_usize("virtio_blk_capacity_sectors", capacity as usize);
    sink.diag_usize(
        "block_registry_devices",
        ctx.block_device_registry.device_count(),
    );
    sink.diag_usize(
        "block_registry_default_present",
        ctx.block_device_registry.default_device().is_some() as usize,
    );
    sink.diag_usize("virtio_blk_queue_ready", device.queue_setup_done() as usize);
    sink.diag_usize("virtio_blk_driver_ok", device.driver_ok() as usize);
    if let Some(transport) = virtio_device.mmio_transport() {
        sink.diag_usize(
            "virtio_blk_irq_source_gate_open",
            transport.irq_source_gate_open() as usize,
        );
    }
    sink.diag_usize(
        "virtio_blk_read_submitted",
        device.read_request_submitted() as usize,
    );
    sink.diag_usize(
        "virtio_blk_read_notified",
        device.read_request_notified() as usize,
    );
    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn run_read_completion(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) -> CheckpointOutcome {
    let read_name = "virtio_blk.read_completion";
    sink.start_case(total, "", read_name, checkpoint);

    if !blk_read_facts_valid(ctx) {
        sink.fail(total, "", read_name, "virtio blk read facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    let Some(device) = ctx.virtio_blk_runtime.device() else {
        sink.fail(total, "", read_name, "virtio blk device missing");
        return CheckpointOutcome::FailAndShutdown;
    };
    sink.diag_usize("virtio_blk_requests", device.request_count());
    sink.diag_usize("virtio_blk_notifies", device.notify_count());
    sink.diag_usize("virtio_blk_irq_count", device.irq_count());
    sink.diag_usize("virtio_blk_completions", device.completion_count());
    sink.diag_usize("virtio_blk_last_status", device.last_status() as usize);
    sink.diag_usize("virtio_blk_last_used_len", device.last_used_len() as usize);
    sink.diag_usize("virtio_blk_last_sector", device.last_sector() as usize);
    sink.diag_usize(
        "virtio_blk_ext2_magic",
        device.complete_ext2_magic_observed() as usize,
    );
    sink.diag_usize(
        "virtio_blk_last_irq_status",
        virtio_blk::last_irq_status() as usize,
    );
    sink.diag_usize(
        "virtio_mmio_irq_handler_calls",
        virtio_mmio::irq_handler_calls(),
    );
    sink.diag_usize(
        "virtio_blk_irq_callbacks",
        virtio_blk::irq_completion_calls(),
    );
    sink.diag_usize(
        "virtio_blk_read_ready_checkpoints",
        virtio_blk::read_ready_checkpoints(),
    );
    sink.pass(total, "", read_name);
    CheckpointOutcome::Continue
}

fn blk_facts_valid(ctx: &Context) -> bool {
    let runtime = &ctx.virtio_blk_runtime;
    let Some(device) = runtime.device() else {
        return false;
    };
    let virtio_device = device.virtio_device();
    let Some(transport) = virtio_device.mmio_transport() else {
        return false;
    };

    runtime.driver().state() == State::Ready
        && runtime.driver().name_bound()
        && runtime.driver().id_table_contains_block()
        && runtime.driver().probe_called()
        && runtime.driver().probe_return_zero()
        && runtime.driver().matched_device()
        && runtime.real_probe_attempted()
        && runtime.real_probe_succeeded()
        && ctx.virtio_bus.block_device_count() != 0
        && ctx.virtio_bus.block_device().is_some()
        && virtio_device.is_block()
        && virtio_device.device_id() == VIRTIO_ID_BLOCK
        && virtio_device.vendor_id() != 0
        && virtio_device.config_access_ready()
        && virtio_device.config_capacity_read()
        && virtio_device
            .config_capacity()
            .is_some_and(|capacity| capacity != 0)
        && virtio_device.features_read()
        && virtio_device.driver_features_written()
        && virtio_device.feature_negotiation_done()
        && virtio_device.queue_setup_done()
        && virtio_device.status_driver_ok()
        && device.state() == State::Ready
        && device.single_request_queue()
        && device.queue().state() == State::Ready
        && device.queue().real_backing_ready()
        && device.queue().queue_num_max_observed()
        && blk_block_registry_facts_valid(ctx)
        && device.capacity_read()
        && device.capacity_nonzero()
        && device.capacity().is_some_and(|capacity| capacity != 0)
        && device.queue_setup_done()
        && device.driver_ok()
        && device.request_queue_deferred()
        && device.filesystem_parse_deferred()
        && device.multi_queue_deferred()
        && device.reset_remove_deferred()
        && transport.ready_for_virtio_core()
        && transport.header_valid()
        && transport.block_candidate()
        && transport.device_id() == VIRTIO_ID_BLOCK
        && transport.config_capacity_read()
        && transport
            .config_capacity()
            .is_some_and(|capacity| capacity != 0)
        && transport.queue_setup_done()
        && transport.status_driver_ok_written()
        && transport.irq_source_gate_open()
        && ctx
            .platform_bus
            .platform_driver_registered(VIRTIO_MMIO_PLATFORM_DRIVER_REF)
        && ctx
            .platform_bus
            .platform_device_discovered(virtio_device.platform_device_ref())
        && ctx.platform_bus.platform_probe_return_zero(
            VIRTIO_MMIO_PLATFORM_DRIVER_REF,
            virtio_device.platform_device_ref(),
        )
}

fn blk_block_registry_facts_valid(ctx: &Context) -> bool {
    let Some(device) = ctx.virtio_blk_runtime.device() else {
        return false;
    };
    let block_device = device.block_device();
    let registry = &ctx.block_device_registry;
    let Some(device_ref) = block_device.device_ref() else {
        return false;
    };
    let Some(devt) = block_device.devt() else {
        return false;
    };
    let Some(entry) = registry.device(device_ref) else {
        return false;
    };
    let Some(default_entry) = registry.default_entry() else {
        return false;
    };
    let Some(lookup_entry) = registry.lookup(devt) else {
        return false;
    };

    registry.state() == State::Ready
        && registry.registry_ready()
        && registry.major_allocator_ready()
        && registry.default_device_slot_ready()
        && registry.request_queue_deferred()
        && registry.bio_page_cache_deferred()
        && registry.partition_scan_deferred()
        && registry.dev_node_deferred()
        && registry.register_blkdev_called()
        && registry.register_blkdev_returned_major()
        && registry.device_add_disk_called()
        && registry.device_add_disk_return_zero()
        && registry.major_minor_lookup_ready()
        && registry.register_count() == 1
        && registry.device_count() == 1
        && registry.default_device() == Some(device_ref)
        && block_device.state() == State::Ready
        && block_device.name_bound()
        && block_device.capacity_bound()
        && block_device.capacity_sectors() == device.capacity().unwrap_or(0)
        && block_device.sector_size() == 512
        && block_device.registered()
        && block_device.default_device()
        && block_device.major_minor_bound()
        && devt.major() == VIRTBLK_MAJOR
        && devt.minor() == VIRTBLK_FIRST_MINOR
        && VIRTBLK_MINORS != 0
        && entry.device_ref() == device_ref
        && entry.provider_kind() == BlockDeviceProviderKind::VirtioBlk
        && entry.capacity_sectors() == block_device.capacity_sectors()
        && entry.sector_size() == block_device.sector_size()
        && entry.devt() == devt
        && entry.registered()
        && entry.default_device()
        && default_entry.device_ref() == device_ref
        && lookup_entry.device_ref() == device_ref
        && entry.name() == block_device.name()
}

fn blk_read_facts_valid(ctx: &Context) -> bool {
    let runtime = &ctx.virtio_blk_runtime;
    let Some(device) = runtime.device() else {
        return false;
    };
    let virtio_device = device.virtio_device();

    blk_facts_valid(ctx)
        && device.read_header_prepared()
        && device.read_data_buffer_prepared()
        && device.read_status_buffer_prepared()
        && device.read_request_submitted()
        && device.read_request_notified()
        && !device.read_request_pending()
        && device.mmio_irq_acknowledged()
        && device.irq_callback_invoked()
        && device.complete_gets_used_buffer()
        && device.complete_status_ok()
        && device.complete_data_nonzero()
        && device.complete_ext2_magic_observed()
        && device.read_request_done()
        && device.request_count() == 1
        && device.notify_count() == 1
        && device.irq_count() == 1
        && device.completion_count() == 1
        && device.last_status() == 0
        && device.last_used_len() <= 513
        && device.last_sector() == 2
        && device.queue().out_descriptor_added()
        && device.queue().in_descriptor_added()
        && device.queue().real_used_completion_observed()
        && device.queue().descriptor_chain_released()
        && device.queue().buffer_ownership_released()
        && virtio_device.queue_notify_done()
        && virtio_blk::last_irq_status() & 1 != 0
        && virtio_blk::irq_completion_calls() != 0
        && virtio_blk::read_ready_checkpoints() != 0
        && virtio_blk::read_buffer_nonzero()
        && virtio_blk::read_buffer_ext2_magic_observed()
}
