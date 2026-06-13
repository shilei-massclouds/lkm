use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit::KunitSink,
    context::Context,
    objects::{
        irq_time::{IrqHandlerKind, LogicalIrq},
        ns16550a,
        state::State,
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "uart_irq_chain.observer_real_path",
    priority: 92,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn KunitSink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    if !observer_baseline_valid(ctx) {
        sink.fail(
            total,
            "",
            HANDLER.name,
            "UART IRQ chain observer baseline facts invalid",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.diag_usize(
        "uart_irq_source",
        ns16550a::uart8250_port_irq_source() as usize,
    );
    sink.diag_usize(
        "uart_logical_irq_valid",
        if ns16550a::uart8250_port_logical_irq().is_valid() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "uart_irq_handler_calls",
        ns16550a::uart8250_irq_handler_call_count(),
    );
    sink.diag_usize(
        "uart_thre_requests",
        ns16550a::uart8250_thre_interrupt_request_count(),
    );
    sink.diag_usize(
        "uart_thre_handled",
        ns16550a::uart8250_thre_interrupt_handled_count(),
    );
    sink.diag_usize("uart_last_iir", ns16550a::uart8250_last_iir());
    sink.diag_usize("uart_last_lsr", ns16550a::uart8250_last_lsr());
    sink.diag_usize("plic_claim_count", ctx.plic.claim_count());
    sink.diag_usize("plic_zero_claim_count", ctx.plic.zero_claim_count());
    sink.diag_usize("plic_complete_count", ctx.plic.complete_count());
    sink.diag_usize("plic_dispatch_count", ctx.plic.dispatch_count());
    sink.diag_usize("plic_loop_exit_count", ctx.plic.loop_exit_count());
    sink.diag_usize(
        "irq_dispatch_calls",
        ctx.irq_handler_registry.dispatch_calls(),
    );
    sink.diag_usize(
        "uart_irq_cycle_closed",
        if ctx.uart_interrupt_chain_probe.irq_cycle_closed() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_irq_tx_ready",
        if ctx
            .serial8250_console_irq_tx_probe
            .interrupt_driven_enabled()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_runtime_tx_ready",
        if ns16550a::serial8250_runtime_console_tx_ready() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_runtime_rx_deferred",
        if ns16550a::serial8250_runtime_rx_deferred() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_tx_irq_drains",
        ns16550a::serial8250_tx_irq_drain_count(),
    );
    sink.diag_usize(
        "serial8250_tx_queue_len",
        ns16550a::serial8250_tx_queue_len(),
    );
    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn observer_baseline_valid(ctx: &Context) -> bool {
    let logical_irq = ns16550a::uart8250_port_logical_irq();
    let source = ns16550a::uart8250_port_irq_source();

    ctx.plic.state() == State::Ready
        && ctx
            .interrupt_stream
            .supervisor_external_input_gate_defined()
        && ctx.interrupt_stream.supervisor_external_input_gate_open()
        && ctx.uart_external_irq_enable.state() == State::Ready
        && ctx.uart_external_irq_enable.plic_source_gate_open()
        && ctx.uart_external_irq_enable.root_external_input_gate_open()
        && ctx
            .uart_external_irq_enable
            .uart_interrupt_output_deferred()
        && ctx.uart_interrupt_chain_probe.state() == State::Ready
        && ctx.uart_interrupt_chain_probe.uart_trigger_committed()
        && ctx.uart_interrupt_chain_probe.plic_claim_observed()
        && ctx.uart_interrupt_chain_probe.irq_dispatch_observed()
        && ctx.uart_interrupt_chain_probe.uart_handler_observed()
        && ctx.uart_interrupt_chain_probe.plic_complete_observed()
        && ctx.uart_interrupt_chain_probe.plic_loop_exit_observed()
        && ctx.uart_interrupt_chain_probe.irq_cycle_closed()
        && ctx.uart_interrupt_chain_probe.console_polling_preserved()
        && ctx.serial8250_console_irq_tx_probe.state() == State::Ready
        && ctx
            .serial8250_console_irq_tx_probe
            .interrupt_driven_enabled()
        && ctx
            .serial8250_console_irq_tx_probe
            .printk_frontend_submitted()
        && ctx.serial8250_console_irq_tx_probe.tx_queue_kicked()
        && ctx
            .serial8250_console_irq_tx_probe
            .uart_handler_drained_tx()
        && ctx.serial8250_console_irq_tx_probe.plic_claim_observed()
        && ctx.serial8250_console_irq_tx_probe.plic_complete_observed()
        && ctx
            .serial8250_console_irq_tx_probe
            .zero_claim_loop_exit_observed()
        && ctx
            .serial8250_console_irq_tx_probe
            .tx_queue_empty_after_irq()
        && ctx
            .serial8250_console_irq_tx_probe
            .local_irq_guard_observed()
        && ns16550a::uart8250_interrupt_driven_ready()
        && ns16550a::serial8250_runtime_port_ready()
        && ns16550a::serial8250_runtime_console_tx_ready()
        && ns16550a::serial8250_runtime_rx_deferred()
        && ns16550a::tty_port_ready()
        && ns16550a::tty_port_not_backend_owner()
        && ns16550a::tty_port_runtime_deferred()
        && ns16550a::tty_flip_buffer_ready()
        && ns16550a::tty_flip_buffer_empty()
        && ns16550a::tty_xmit_fifo_ready()
        && ns16550a::tty_xmit_fifo_deferred_from_console_tx()
        && ns16550a::serial8250_tx_irq_drain_count() != 0
        && ns16550a::serial8250_tx_queue_len() == 0
        && ctx.plic.chained_handler_ready()
        && ctx.plic.claim_action_ready()
        && ctx.plic.complete_action_ready()
        && ctx.plic.claim_reads_claim_register()
        && ctx.plic.claim_zero_means_no_pending()
        && ctx.plic.complete_writes_claimed_source()
        && ctx.plic.claim_loop_until_zero()
        && ctx.plic.zero_claim_stops_dispatch()
        && ctx.plic.completes_each_claimed_source()
        && ctx.plic.claim_before_dispatch()
        && ctx.plic.complete_after_handler()
        && ctx.plic.claim_count() != 0
        && ctx.plic.zero_claim_count() != 0
        && ctx.plic.complete_count() != 0
        && ctx.plic.dispatch_count() != 0
        && ctx.plic.loop_exit_count() != 0
        && ctx.plic.complete_count() == ctx.plic.claim_count()
        && ctx.plic.last_claimed_source() == source
        && ctx.plic.last_completed_source() == source
        && ctx.plic_irq_domain.state() == State::Ready
        && ctx.plic_irq_domain.enable_deferred()
        && ctx.plic_irq_domain.dispatch_ops_ready()
        && ctx.irq_handler_registry.state() == State::Ready
        && ctx.irq_handler_registry.source_enable_deferred()
        && ctx.irq_handler_registry.dispatch_ready()
        && ctx.irq_handler_registry.dispatch_requires_hardirq_context()
        && ctx.irq_handler_registry.dispatch_calls() != 0
        && ns16550a::uart8250_port_irq_resource_ready()
        && ns16550a::uart8250_port_logical_irq_ready()
        && ns16550a::uart8250_irq_handler_registered()
        && ns16550a::uart8250_irq_handler_hardirq_context_required()
        && ns16550a::uart8250_irq_handler_dispatch_ready()
        && ns16550a::uart8250_interrupt_trigger_ready()
        && ns16550a::uart8250_thre_interrupt_handled()
        && ns16550a::uart8250_thre_interrupt_request_count() != 0
        && ns16550a::uart8250_thre_interrupt_handled_count() != 0
        && ns16550a::uart8250_thri_disabled_by_handler_count() != 0
        && ns16550a::uart8250_irq_handler_call_count() != 0
        && plic_mapping_enabled(ctx, source, logical_irq)
        && irq_action_deferred(ctx, logical_irq)
}

fn plic_mapping_enabled(ctx: &Context, source: u32, logical_irq: LogicalIrq) -> bool {
    ctx.plic_irq_domain
        .mapping_for_source(source)
        .is_some_and(|mapping| {
            mapping.logical_irq() == logical_irq
                && mapping.domain_bound()
                && mapping.source_valid()
                && mapping.source_gate_defined()
                && mapping.source_gate_open()
                && mapping.source_enabled()
                && mapping.handler_not_registered()
        })
}

fn irq_action_deferred(ctx: &Context, logical_irq: LogicalIrq) -> bool {
    ctx.irq_handler_registry
        .action_for_logical_irq(logical_irq)
        .is_some_and(|action| {
            action.handler_kind() == IrqHandlerKind::Ns16550aUart
                && action.handler_bound()
                && action.hardirq_context_required()
                && action.mapped_irq_required()
                && action.source_not_enabled()
                && action.dispatch_ready()
        })
}
