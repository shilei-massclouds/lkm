use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit::Sink,
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

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
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
        "serial8250_runtime_rx_previously_deferred",
        if !ns16550a::serial8250_runtime_rx_enabled() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_runtime_rx_enabled",
        if ns16550a::serial8250_runtime_rx_enabled() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_runtime_rx_fifo_enabled",
        if ns16550a::serial8250_runtime_rx_fifo_enabled() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_runtime_rx_drain_limit",
        ns16550a::serial8250_runtime_rx_drain_limit(),
    );
    sink.diag_usize(
        "serial8250_runtime_tx_load_size",
        ns16550a::serial8250_runtime_tx_load_size(),
    );
    sink.diag_usize(
        "serial8250_rx_requests",
        ns16550a::uart8250_rx_interrupt_request_count(),
    );
    sink.diag_usize(
        "serial8250_rx_handled",
        ns16550a::uart8250_rx_interrupt_handled_count(),
    );
    sink.diag_usize("tty_flip_pushes", ns16550a::tty_flip_buffer_push_count());
    sink.diag_usize(
        "tty_flip_inserted",
        ns16550a::tty_flip_buffer_total_inserted(),
    );
    sink.diag_usize(
        "tty_flip_last_pushed_len",
        ns16550a::tty_flip_buffer_last_pushed_len(),
    );
    sink.diag_usize(
        "tty_flip_last_byte",
        ns16550a::tty_flip_buffer_last_byte() as usize,
    );
    sink.diag_usize(
        "tty_flip_overflowed",
        if ns16550a::tty_flip_buffer_overflowed() {
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
        "serial8250_tx_irq_budget_hits",
        ns16550a::serial8250_tx_irq_budget_hit_count(),
    );
    sink.diag_usize(
        "serial8250_tx_bytes_submitted",
        ns16550a::serial8250_tx_byte_count_available_for_irq_probe(),
    );
    sink.diag_usize(
        "serial8250_tx_crlf_insertions",
        ns16550a::serial8250_tx_crlf_insertion_count(),
    );
    sink.diag_usize(
        "serial8250_last_tx_byte",
        ns16550a::serial8250_last_tx_byte() as usize,
    );
    sink.diag_usize(
        "serial8250_tx_queue_len",
        ns16550a::serial8250_tx_queue_len(),
    );
    sink.diag_usize(
        "serial8250_console_burst_irq_tx_ready",
        if ctx.serial8250_console_burst_irq_tx_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_burst_write_count_matched",
        if ctx
            .serial8250_console_burst_irq_tx_probe
            .write_count_matched()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_burst_drain_count_matched",
        if ctx
            .serial8250_console_burst_irq_tx_probe
            .drain_count_matched()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_burst_tty_xmit_unchanged",
        if ctx
            .serial8250_console_burst_irq_tx_probe
            .tty_xmit_fifo_unchanged()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_irq_tx_ready",
        if ctx.serial8250_console_long_irq_tx_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_multiple_irq_rounds",
        if ctx
            .serial8250_console_long_irq_tx_probe
            .multiple_irq_rounds_observed()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_tx_load_budget",
        if ctx
            .serial8250_console_long_irq_tx_probe
            .tx_load_budget_observed()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_drain_count_matched",
        if ctx
            .serial8250_console_long_irq_tx_probe
            .drain_count_matched()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_burst_irq_tx_ready",
        if ctx.serial8250_console_long_burst_irq_tx_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_burst_records",
        if ctx
            .serial8250_console_long_burst_irq_tx_probe
            .multiple_records_submitted()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_burst_each_record_exceeds_load",
        if ctx
            .serial8250_console_long_burst_irq_tx_probe
            .each_record_exceeds_single_load()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_burst_budget",
        if ctx
            .serial8250_console_long_burst_irq_tx_probe
            .tx_load_budget_observed()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_long_burst_drain_count_matched",
        if ctx
            .serial8250_console_long_burst_irq_tx_probe
            .drain_count_matched()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_tx_quiesce_ready",
        if ctx.serial8250_console_tx_quiesce_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_tx_quiesce_no_printk_write",
        if ctx.serial8250_console_tx_quiesce_probe.no_printk_write() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_tx_quiesce_no_spurious_irq",
        if ctx
            .serial8250_console_tx_quiesce_probe
            .no_spurious_plic_claim()
            && ctx
                .serial8250_console_tx_quiesce_probe
                .no_spurious_irq_dispatch()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_tx_quiesce_no_uart_tx_drain",
        if ctx.serial8250_console_tx_quiesce_probe.no_uart_tx_drain() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "serial8250_console_tx_quiesce_no_tty_xmit_mutation",
        if ctx
            .serial8250_console_tx_quiesce_probe
            .no_tty_xmit_fifo_mutation()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_xmit_fifo_probe_ready",
        if ctx.tty_xmit_fifo_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_xmit_fifo_enqueues",
        ns16550a::tty_xmit_fifo_enqueue_count(),
    );
    sink.diag_usize(
        "tty_xmit_fifo_dequeues",
        ns16550a::tty_xmit_fifo_dequeue_count(),
    );
    sink.diag_usize(
        "tty_xmit_fifo_queue_len",
        ns16550a::tty_xmit_fifo_queue_len(),
    );
    sink.diag_usize(
        "tty_xmit_fifo_last_enqueued",
        ns16550a::tty_xmit_fifo_last_enqueued() as usize,
    );
    sink.diag_usize(
        "tty_xmit_fifo_last_dequeued",
        ns16550a::tty_xmit_fifo_last_dequeued() as usize,
    );
    sink.diag_usize(
        "tty_xmit_fifo_overflowed",
        if ns16550a::tty_xmit_fifo_overflowed() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_xmit_fifo_underflowed",
        if ns16550a::tty_xmit_fifo_underflowed() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_write_runtime_tx_ready",
        if ctx.tty_write_runtime_tx_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_xmit_fifo_runtime_tx_integrated",
        if ns16550a::tty_xmit_fifo_runtime_tx_integrated() {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_xmit_fifo_runtime_tx_kicks",
        ns16550a::tty_xmit_fifo_runtime_tx_kick_count(),
    );
    sink.diag_usize(
        "tty_xmit_fifo_runtime_tx_drains",
        ns16550a::tty_xmit_fifo_runtime_tx_drain_count(),
    );
    sink.diag_usize(
        "tty_xmit_fifo_runtime_tx_empty_stops",
        ns16550a::tty_xmit_fifo_runtime_tx_empty_stop_count(),
    );
    sink.diag_usize(
        "tty_write_batch_runtime_tx_ready",
        if ctx.tty_write_batch_runtime_tx_probe.state() == State::Ready {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_write_batch_runtime_tx_bounded_drain",
        if ctx
            .tty_write_batch_runtime_tx_probe
            .bounded_drain_observed()
        {
            1
        } else {
            0
        },
    );
    sink.diag_usize(
        "tty_write_batch_runtime_tx_count_matched",
        if ctx.tty_write_batch_runtime_tx_probe.batch_count_matched() {
            1
        } else {
            0
        },
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
        && ctx.serial8250_console_burst_irq_tx_probe.state() == State::Ready
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .printk_frontend_submitted()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .multiple_records_submitted()
        && ctx.serial8250_console_burst_irq_tx_probe.tx_queue_kicked()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .plic_claim_observed()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .irq_dispatch_observed()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .uart_handler_drained_tx()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .plic_complete_observed()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .zero_claim_loop_exit_observed()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .tx_queue_empty_after_irq()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .write_count_matched()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .drain_count_matched()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .last_byte_matched()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .local_irq_guard_observed()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .no_overflow_observed()
        && ctx
            .serial8250_console_burst_irq_tx_probe
            .tty_xmit_fifo_unchanged()
        && ctx.serial8250_console_long_irq_tx_probe.state() == State::Ready
        && ctx
            .serial8250_console_long_irq_tx_probe
            .printk_frontend_submitted()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .tx_load_size_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .message_exceeds_single_load()
        && ctx.serial8250_console_long_irq_tx_probe.tx_queue_kicked()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .plic_claim_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .irq_dispatch_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .uart_handler_drained_tx()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .multiple_irq_rounds_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .tx_load_budget_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .plic_complete_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .zero_claim_loop_exit_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .tx_queue_empty_after_irq()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .write_count_matched()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .drain_count_matched()
        && ctx.serial8250_console_long_irq_tx_probe.last_byte_matched()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .local_irq_guard_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .no_overflow_observed()
        && ctx
            .serial8250_console_long_irq_tx_probe
            .tty_xmit_fifo_unchanged()
        && ctx.serial8250_console_long_burst_irq_tx_probe.state() == State::Ready
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .printk_frontend_submitted()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .multiple_records_submitted()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .tx_load_size_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .each_record_exceeds_single_load()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .tx_queue_kicked()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .plic_claim_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .irq_dispatch_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .uart_handler_drained_tx()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .multiple_irq_rounds_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .tx_load_budget_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .plic_complete_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .zero_claim_loop_exit_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .tx_queue_empty_after_irq()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .write_count_matched()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .drain_count_matched()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .last_byte_matched()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .local_irq_guard_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .no_overflow_observed()
        && ctx
            .serial8250_console_long_burst_irq_tx_probe
            .tty_xmit_fifo_unchanged()
        && ctx.serial8250_console_tx_quiesce_probe.state() == State::Ready
        && ctx.serial8250_console_tx_quiesce_probe.no_printk_write()
        && ctx
            .serial8250_console_tx_quiesce_probe
            .tx_queue_empty_observed()
        && ctx
            .serial8250_console_tx_quiesce_probe
            .thri_stopped_observed()
        && ctx
            .serial8250_console_tx_quiesce_probe
            .no_spurious_plic_claim()
        && ctx
            .serial8250_console_tx_quiesce_probe
            .no_spurious_irq_dispatch()
        && ctx.serial8250_console_tx_quiesce_probe.no_uart_tx_drain()
        && ctx
            .serial8250_console_tx_quiesce_probe
            .no_tty_xmit_fifo_mutation()
        && ctx
            .serial8250_console_tx_quiesce_probe
            .no_overflow_observed()
        && ctx.serial8250_rx_loopback_probe.state() == State::Ready
        && ctx.serial8250_rx_loopback_probe.rx_runtime_enabled()
        && ctx
            .serial8250_rx_loopback_probe
            .loopback_stimulus_committed()
        && ctx.serial8250_rx_loopback_probe.plic_claim_observed()
        && ctx.serial8250_rx_loopback_probe.irq_dispatch_observed()
        && ctx.serial8250_rx_loopback_probe.uart_handler_received_rx()
        && ctx.serial8250_rx_loopback_probe.flip_buffer_pushed()
        && ctx.serial8250_rx_loopback_probe.plic_complete_observed()
        && ctx
            .serial8250_rx_loopback_probe
            .zero_claim_loop_exit_observed()
        && ctx.serial8250_rx_loopback_probe.irq_cycle_closed()
        && ctx.serial8250_rx_loopback_probe.last_byte_matched()
        && ctx.serial8250_rx_batch_loopback_probe.state() == State::Ready
        && ctx
            .serial8250_rx_batch_loopback_probe
            .batch_stimulus_committed()
        && ctx.serial8250_rx_batch_loopback_probe.plic_claim_observed()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .irq_dispatch_observed()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .uart_handler_received_batch()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .flip_buffer_batch_pushed()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .plic_complete_observed()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .zero_claim_loop_exit_observed()
        && ctx.serial8250_rx_batch_loopback_probe.irq_cycle_closed()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .bounded_drain_observed()
        && ctx.serial8250_rx_batch_loopback_probe.batch_count_matched()
        && ctx.serial8250_rx_batch_loopback_probe.last_byte_matched()
        && ctx
            .serial8250_rx_batch_loopback_probe
            .no_overflow_observed()
        && ns16550a::uart8250_interrupt_driven_ready()
        && ns16550a::serial8250_runtime_port_ready()
        && ns16550a::serial8250_runtime_console_tx_ready()
        && ns16550a::serial8250_runtime_rx_enabled()
        && ns16550a::serial8250_runtime_rx_fifo_enabled()
        && ns16550a::tty_port_ready()
        && ns16550a::tty_port_not_backend_owner()
        && ns16550a::tty_flip_buffer_ready()
        && ns16550a::tty_flip_buffer_pushed()
        && ns16550a::tty_flip_buffer_last_pushed_len() != 0
        && !ns16550a::tty_flip_buffer_overflowed()
        && ns16550a::tty_xmit_fifo_ready()
        && ns16550a::tty_xmit_fifo_runtime_tx_integrated()
        && ctx.tty_xmit_fifo_probe.state() == State::Ready
        && ctx.tty_xmit_fifo_probe.enqueue_committed()
        && ctx.tty_xmit_fifo_probe.dequeue_committed()
        && ctx.tty_xmit_fifo_probe.byte_round_trip()
        && ctx.tty_xmit_fifo_probe.queue_empty_after_dequeue()
        && ctx.tty_xmit_fifo_probe.distinct_from_printk_console_tx()
        && ctx.tty_xmit_fifo_probe.runtime_tx_deferred()
        && ctx.tty_xmit_fifo_probe.printk_tx_queue_unchanged()
        && ctx.tty_xmit_fifo_probe.no_uart_thri_kick()
        && ctx.tty_xmit_fifo_probe.no_overflow_observed()
        && ctx.tty_xmit_fifo_probe.no_underflow_observed()
        && ctx.tty_write_runtime_tx_probe.state() == State::Ready
        && ctx.tty_write_runtime_tx_probe.xmit_fifo_enqueued()
        && ctx.tty_write_runtime_tx_probe.start_tx_committed()
        && ctx.tty_write_runtime_tx_probe.plic_claim_observed()
        && ctx.tty_write_runtime_tx_probe.irq_dispatch_observed()
        && ctx.tty_write_runtime_tx_probe.uart_handler_observed()
        && ctx.tty_write_runtime_tx_probe.xmit_fifo_drained()
        && ctx.tty_write_runtime_tx_probe.plic_complete_observed()
        && ctx
            .tty_write_runtime_tx_probe
            .zero_claim_loop_exit_observed()
        && ctx.tty_write_runtime_tx_probe.queue_empty_after_irq()
        && ctx.tty_write_runtime_tx_probe.printk_tx_queue_unchanged()
        && ctx.tty_write_runtime_tx_probe.local_irq_guard_observed()
        && ctx.tty_write_runtime_tx_probe.last_byte_matched()
        && ctx.tty_write_batch_runtime_tx_probe.state() == State::Ready
        && ctx.tty_write_batch_runtime_tx_probe.fixed_bounded_batch()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .xmit_fifo_batch_enqueued()
        && ctx.tty_write_batch_runtime_tx_probe.start_tx_committed()
        && ctx.tty_write_batch_runtime_tx_probe.plic_claim_observed()
        && ctx.tty_write_batch_runtime_tx_probe.irq_dispatch_observed()
        && ctx.tty_write_batch_runtime_tx_probe.uart_handler_observed()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .xmit_fifo_batch_drained()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .plic_complete_observed()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .zero_claim_loop_exit_observed()
        && ctx.tty_write_batch_runtime_tx_probe.queue_empty_after_irq()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .printk_tx_queue_unchanged()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .local_irq_guard_observed()
        && ctx
            .tty_write_batch_runtime_tx_probe
            .bounded_drain_observed()
        && ctx.tty_write_batch_runtime_tx_probe.batch_count_matched()
        && ctx.tty_write_batch_runtime_tx_probe.last_byte_matched()
        && ctx.tty_write_batch_runtime_tx_probe.no_overflow_observed()
        && ctx.tty_write_batch_runtime_tx_probe.no_underflow_observed()
        && ns16550a::tty_xmit_fifo_round_trip_ready()
        && ns16550a::tty_xmit_fifo_runtime_tx_kick_count() != 0
        && ns16550a::tty_xmit_fifo_runtime_tx_drain_count() != 0
        && ns16550a::tty_xmit_fifo_runtime_tx_empty_stop_count() != 0
        && ns16550a::uart8250_rx_interrupt_request_count() != 0
        && ns16550a::uart8250_rx_interrupt_handled_count() != 0
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
