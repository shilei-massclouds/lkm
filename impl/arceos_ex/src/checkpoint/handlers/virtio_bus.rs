use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        virtio::{VirtioDevice, VirtioDeviceState, VirtioTransportKind},
        virtio_mmio::{self, VIRTIO_ID_RNG, VIRTIO_MMIO_PLATFORM_DRIVER_REF},
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::VirtioBusDeviceAdded];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "virtio_bus",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    let virtio_bus = &ctx.virtio_bus;
    let Some(device) = virtio_bus.rng_device() else {
        sink.fail(total, "", HANDLER.name, "virtio rng device not registered");
        return CheckpointOutcome::FailAndShutdown;
    };

    if virtio_bus.state() != State::Ready
        || !virtio_bus.registered()
        || virtio_bus.device_count() == 0
        || virtio_bus.mmio_transport_count() == 0
        || virtio_bus.rng_device_count() == 0
        || !virtio_device_facts_valid(device)
        || !platform_probe_facts_valid(ctx, device)
    {
        sink.fail(total, "", HANDLER.name, "virtio bus facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.diag_usize("virtio_devices", virtio_bus.device_count());
    sink.diag_usize("virtio_mmio_transports", virtio_bus.mmio_transport_count());
    sink.diag_usize("virtio_rng_devices", virtio_bus.rng_device_count());
    sink.diag_usize("virtio_rng_device_id", device.device_id() as usize);
    sink.diag_usize("virtio_rng_vendor_id", device.vendor_id() as usize);
    sink.drain_printk_diag();
    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn virtio_device_facts_valid(device: VirtioDevice) -> bool {
    let Some(transport) = device.mmio_transport() else {
        return false;
    };

    device.state() == VirtioDeviceState::Registered
        && device.transport_kind() == VirtioTransportKind::Mmio
        && device.transport_is_mmio()
        && device.is_rng()
        && device.device_id() == VIRTIO_ID_RNG
        && device.vendor_id() != 0
        && device.platform_device_ref() == transport.device_ref()
        && device.node_id() == transport.node_id()
        && transport.ready_for_virtio_core()
        && transport.header_valid()
        && transport.rng_candidate()
        && transport.device_id() == VIRTIO_ID_RNG
        && transport.vendor_id() == device.vendor_id()
        && transport.mapbase() != 0
        && transport.mapsize() != 0
        && transport.membase() != 0
        && transport.ioremapped()
        && transport.vm_ioremap()
        && transport.io_page_protection()
        && transport.irq_source().is_some()
}

fn platform_probe_facts_valid(ctx: &Context, device: VirtioDevice) -> bool {
    let platform_device_ref = device.platform_device_ref();
    ctx.platform_bus
        .platform_driver_registered(VIRTIO_MMIO_PLATFORM_DRIVER_REF)
        && virtio_mmio::is_virtio_mmio_platform_driver(VIRTIO_MMIO_PLATFORM_DRIVER_REF)
        && ctx
            .platform_bus
            .platform_device_discovered(platform_device_ref)
        && ctx
            .platform_bus
            .platform_driver_matched_device(VIRTIO_MMIO_PLATFORM_DRIVER_REF, platform_device_ref)
        && ctx
            .platform_bus
            .platform_probe_called(VIRTIO_MMIO_PLATFORM_DRIVER_REF, platform_device_ref)
        && ctx
            .platform_bus
            .platform_probe_return_zero(VIRTIO_MMIO_PLATFORM_DRIVER_REF, platform_device_ref)
        && ctx
            .platform_bus
            .platform_device_bound(VIRTIO_MMIO_PLATFORM_DRIVER_REF, platform_device_ref)
        && ctx
            .platform_bus
            .platform_device(platform_device_ref)
            .is_some()
}
