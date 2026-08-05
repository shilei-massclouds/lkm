use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        virtio_mmio::{self, VIRTIO_ID_RNG, VIRTIO_MMIO_INT_VRING},
        virtio_rng,
    },
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::VirtioRngEntropyReady,
    Checkpoint::PayloadPreparePhaseOnline,
];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "virtio_rng.real_completion",
    priority: 98,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    if checkpoint == Checkpoint::PayloadPreparePhaseOnline && real_completion_facts_valid(ctx) {
        return CheckpointOutcome::Continue;
    }

    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    if !real_completion_facts_valid(ctx) {
        emit_diag(ctx, sink);
        sink.fail(
            total,
            "",
            HANDLER.name,
            "virtio rng real completion invalid",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    emit_diag(ctx, sink);
    sink.drain_printk_diag();
    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn emit_diag(ctx: &Context, sink: &mut dyn Sink) {
    let runtime = &ctx.virtio_rng_runtime;
    let Some(device) = runtime.device() else {
        sink.diag_usize("virtio_rng_device_present", 0);
        sink.diag_usize(
            "virtio_mmio_irq_handler_calls",
            virtio_mmio::irq_handler_calls(),
        );
        sink.diag_usize(
            "virtio_rng_irq_callbacks",
            virtio_rng::irq_completion_calls(),
        );
        sink.diag_usize(
            "virtio_rng_last_irq_status",
            virtio_rng::last_irq_status() as usize,
        );
        return;
    };
    sink.diag_usize(
        "virtio_rng_device_id",
        device.virtio_device().device_id() as usize,
    );
    sink.diag_usize(
        "virtio_rng_probe_attempted",
        if runtime.real_probe_attempted() { 1 } else { 0 },
    );
    sink.diag_usize(
        "virtio_rng_probe_succeeded",
        if runtime.real_probe_succeeded() { 1 } else { 0 },
    );
    sink.diag_usize("virtio_rng_requests", device.request_count());
    sink.diag_usize("virtio_rng_notifies", device.notify_count());
    sink.diag_usize("virtio_rng_irq_count", device.irq_count());
    sink.diag_usize(
        "virtio_rng_poll_completion_count",
        device.poll_completion_count(),
    );
    sink.diag_usize("virtio_rng_completions", device.completion_count());
    sink.diag_usize(
        "virtio_status_reset",
        if device.virtio_device().status_reset() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_status_driver_seen",
        if device.virtio_device().status_driver_seen() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_features_negotiated",
        if device.virtio_device().feature_negotiation_done() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_queue_setup_done",
        if device.virtio_device().queue_setup_done() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_status_driver_ok",
        if device.virtio_device().status_driver_ok() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_queue_notify_done",
        if device.virtio_device().queue_notify_done() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_rng_request_pending",
        if device.request_pending() { 1 } else { 0 },
    );
    sink.diag_usize("virtio_rng_data_avail", device.data_avail() as usize);
    sink.diag_usize("virtio_rng_queue_get_buf", device.queue().get_buf_count());
    sink.diag_usize(
        "virtio_rng_queue_last_used_len",
        device.queue().last_used_len() as usize,
    );
    sink.diag_usize(
        "virtio_rng_queue_avail_idx",
        device.queue().ring_avail_idx() as usize,
    );
    sink.diag_usize(
        "virtio_rng_queue_used_idx",
        device.queue().ring_used_idx() as usize,
    );
    sink.diag_usize(
        "virtio_rng_queue_last_used_idx",
        device.queue().ring_last_used_idx() as usize,
    );
    sink.diag_usize(
        "virtio_rng_raw_avail_idx",
        device.queue().raw_avail_idx().unwrap_or(u16::MAX) as usize,
    );
    sink.diag_usize(
        "virtio_rng_raw_used_idx",
        device.queue().raw_used_idx().unwrap_or(u16::MAX) as usize,
    );
    let layout = device.queue().real_layout();
    sink.diag_hex_pair(
        "virtio_rng_used_virt_phys",
        layout.used_virt(),
        layout.used_phys(),
    );
    sink.diag_usize(
        "virtio_rng_queue_real_used",
        if device.queue().real_used_completion_observed() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "virtio_rng_last_irq_status",
        virtio_rng::last_irq_status() as usize,
    );
    sink.diag_usize(
        "virtio_mmio_irq_handler_calls",
        virtio_mmio::irq_handler_calls(),
    );
    sink.diag_usize(
        "virtio_rng_irq_callbacks",
        virtio_rng::irq_completion_calls(),
    );
    sink.diag_usize(
        "virtio_rng_entropy_ready_checkpoints",
        virtio_rng::entropy_ready_checkpoints(),
    );
}

fn real_completion_facts_valid(ctx: &Context) -> bool {
    let runtime = &ctx.virtio_rng_runtime;
    let Some(device) = runtime.device() else {
        return false;
    };
    let Some(transport) = device.virtio_device().mmio_transport() else {
        return false;
    };
    let queue = device.queue();

    runtime.real_probe_attempted()
        && runtime.real_probe_succeeded()
        && runtime.real_completion_len() > 0
        && runtime.real_completion_len() == device.data_avail()
        && device.state() == State::Ready
        && device.virtio_device().device_id() == VIRTIO_ID_RNG
        && device.virtio_device().status_reset()
        && device.virtio_device().status_acknowledged()
        && device.virtio_device().status_driver_seen()
        && device.virtio_device().features_read()
        && device.virtio_device().driver_features_written()
        && device.virtio_device().feature_negotiation_done()
        && device.virtio_device().status_features_ok()
        && device.virtio_device().queue_setup_done()
        && device.virtio_device().status_driver_ok()
        && device.virtio_device().queue_notify_done()
        && device.real_notify_irq_ready()
        && device.probe_common_requested_entropy()
        && !device.request_pending()
        && device.request_count() >= 1
        && device.notify_count() >= 1
        && device.irq_count() >= 1
        && device.completion_count() >= 1
        && device.complete_gets_used_buffer()
        && device.data_avail_updated()
        && device.data_idx() == 0
        && queue.real_backing_ready()
        && queue.queue_num_max_observed()
        && (queue.legacy_mmio_queue_pfn_written() || queue.modern_mmio_queue_addrs_written())
        && queue.mmio_notify_written()
        && queue.real_used_completion_observed()
        && queue.get_buf_count() >= 1
        && queue.last_used_len() == device.data_avail()
        && transport.irq_handler_registered()
        && transport.irq_source_gate_open()
        && virtio_mmio::irq_handler_calls() >= 1
        && virtio_rng::irq_completion_calls() >= 1
        && virtio_rng::last_irq_status() & VIRTIO_MMIO_INT_VRING != 0
        && virtio_rng::entropy_ready_checkpoints() >= 1
        && virtio_rng::entropy_buffer_nonzero()
}
