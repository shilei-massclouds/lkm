use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        virtio_mmio::{VIRTIO_ID_BLOCK, VIRTIO_MMIO_PLATFORM_DRIVER_REF},
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::VirtioBlkReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "virtio_blk.discovery_config",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
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
    sink.diag_usize("virtio_blk_queue_ready", device.queue_setup_done() as usize);
    sink.diag_usize("virtio_blk_driver_ok", device.driver_ok() as usize);
    sink.pass(total, "", HANDLER.name);
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
        && device.capacity_read()
        && device.capacity_nonzero()
        && device.capacity().is_some_and(|capacity| capacity != 0)
        && device.queue_setup_done()
        && device.driver_ok()
        && device.request_io_deferred()
        && device.block_layer_deferred()
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
