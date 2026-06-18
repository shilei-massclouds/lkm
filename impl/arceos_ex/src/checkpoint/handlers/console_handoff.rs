use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        device::DeviceRef,
        irq_time::IrqHandlerKind,
        ns16550a::{self, NS16550A_PLATFORM_DRIVER_REF},
        printk,
        state::State,
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "console_handoff.real_path",
    priority: 90,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    if console_handoff_observed(ctx) {
        sink.diag_usize("serial8250_writes", ns16550a::serial8250_write_call_count());
        sink.diag_usize("serial8250_tx_bytes", ns16550a::serial8250_tx_byte_count());
        sink.pass(total, "", HANDLER.name);
        CheckpointOutcome::Continue
    } else {
        diag_observation(ctx, sink);
        sink.fail(total, "", HANDLER.name, "console handoff facts invalid");
        CheckpointOutcome::FailAndShutdown
    }
}

fn console_handoff_observed(ctx: &Context) -> bool {
    let Some(device_ref) = ctx.platform_bus.ns16550a_bound_device() else {
        return false;
    };

    platform_probe_observed(ctx, device_ref)
        && ns16550a_console_observed(device_ref)
        && printk_handoff_observed()
        && ioremap_observed(ctx, device_ref)
        && plic_mapping_observed(ctx)
        && irq_action_observed(ctx, device_ref)
}

fn platform_probe_observed(ctx: &Context, device_ref: DeviceRef) -> bool {
    ctx.platform_bus.state() == State::Ready
        && ctx.platform_bus.ns16550a_driver_registered()
        && ctx.platform_bus.ns16550a_match_table_ready()
        && ctx
            .platform_bus
            .platform_device_bound(NS16550A_PLATFORM_DRIVER_REF, device_ref)
        && ctx.platform_bus.ns16550a_device_matched()
        && ctx.platform_bus.ns16550a_probe_called()
        && ctx.platform_bus.ns16550a_probe_return_zero()
        && ctx.platform_bus.ns16550a_probe_ioremaps_uart8250_port()
        && ctx.platform_bus.ns16550a_probe_registers_uart8250_port()
        && ctx.platform_bus.ns16550a_probe_records_uart_irq_resource()
        && ctx.platform_bus.ns16550a_probe_records_uart_irq_mapping()
        && ctx.platform_bus.ns16550a_probe_registers_uart_irq_handler()
        && ctx
            .platform_bus
            .ns16550a_probe_keeps_interrupt_output_deferred()
        && ctx.platform_bus.ns16550a_probe_registers_serial_console()
        && ctx.platform_bus.ns16550a_probe_triggers_console_handoff()
}

fn ns16550a_console_observed(device_ref: DeviceRef) -> bool {
    ns16550a::stdout_path_available()
        && ns16550a::stdout_path_matched()
        && ns16550a::uart8250_port_device_ref() == Some(device_ref)
        && ns16550a::uart8250_port_ioremapped()
        && ns16550a::uart8250_port_resources_ready()
        && ns16550a::uart8250_port_registered()
        && ns16550a::uart8250_port_irq_resource_ready()
        && ns16550a::uart8250_port_logical_irq_ready()
        && ns16550a::uart8250_irq_handler_registered()
        && ns16550a::uart8250_irq_handler_hardirq_context_required()
        && ns16550a::uart8250_irq_handler_dispatch_ready()
        && ns16550a::serial8250_console_registered()
        && ns16550a::serial8250_write_uses_membase()
        && ns16550a::serial8250_write_does_not_use_sbi()
}

fn printk_handoff_observed() -> bool {
    printk::boot_console_registered()
        && !printk::boot_console_online()
        && printk::boot_console_unregistered()
        && printk::boot_console_removed_from_registry()
        && printk::serial8250_console_registered()
        && printk::preferred_console_from_stdout()
        && printk::serial8250_consdev()
        && printk::serial8250_write_ready()
        && !printk::keep_bootcon()
        && printk::console_handoff_complete()
        && printk::route() == printk::PrintkRoute::Serial8250
        && printk::boot_pending_flushed_before_serial_handoff()
        && printk::legacy_earlycon_drain_blocked_after_handoff()
        && printk::serial8250_online_trace_emitted()
        && printk::boot_console_offline_trace_emitted()
        && printk::serial8250_delivered_records_not_replayed()
}

fn ioremap_observed(ctx: &Context, device_ref: DeviceRef) -> bool {
    ctx.ioremap
        .mapping_for_device(device_ref)
        .is_some_and(|mapping| {
            mapping.device() == device_ref
                && mapping.active()
                && mapping.mapped_size() != 0
                && mapping.page_aligned()
                && mapping.membase_cookie_ready()
                && mapping.uses_vm_ioremap()
                && mapping.uses_io_page_protection()
                && mapping.vmap_mapping().installed()
        })
}

fn plic_mapping_observed(ctx: &Context) -> bool {
    ctx.plic_irq_domain
        .mapping_for_source(ns16550a::uart8250_port_irq_source())
        .is_some_and(|mapping| {
            mapping.logical_irq() == ns16550a::uart8250_port_logical_irq()
                && mapping.source_gate_defined()
                && mapping.source_gate_open()
                && mapping.source_enabled()
                && mapping.handler_not_registered()
        })
}

fn irq_action_observed(ctx: &Context, device_ref: DeviceRef) -> bool {
    ctx.irq_handler_registry
        .action_for_logical_irq(ns16550a::uart8250_port_logical_irq())
        .is_some_and(|action| {
            action.device() == device_ref
                && action.handler_kind() == IrqHandlerKind::Ns16550aUart
                && action.handler_bound()
                && action.hardirq_context_required()
                && action.mapped_irq_required()
                && action.duplicate_registration_rejected()
                && action.unmapped_registration_rejected()
                && action.source_not_enabled()
                && action.dispatch_ready()
        })
}

fn diag_observation(ctx: &Context, sink: &mut dyn Sink) {
    let bound = ctx.platform_bus.ns16550a_bound_device();
    sink.diag_usize(
        "ns16550a_bound_device",
        bound.map_or(usize::MAX, |device| device.index()),
    );

    if let Some(device_ref) = bound {
        sink.diag_usize(
            "platform_probe",
            platform_probe_observed(ctx, device_ref) as usize,
        );
        sink.diag_usize(
            "ns16550a_console",
            ns16550a_console_observed(device_ref) as usize,
        );
        sink.diag_usize("ioremap", ioremap_observed(ctx, device_ref) as usize);
        sink.diag_usize("irq_action", irq_action_observed(ctx, device_ref) as usize);
    }

    sink.diag_usize("printk_handoff", printk_handoff_observed() as usize);
    sink.diag_usize("plic_mapping", plic_mapping_observed(ctx) as usize);
}
