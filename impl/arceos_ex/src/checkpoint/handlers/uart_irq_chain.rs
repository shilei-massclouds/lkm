use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
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
    name: "uart_irq_chain.observer_baseline",
    priority: 92,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Read(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    kunit::start_case(total, "", HANDLER.name, checkpoint);

    if !observer_baseline_valid(ctx) {
        kunit::fail(
            total,
            "",
            HANDLER.name,
            "UART IRQ chain observer baseline facts invalid",
        );
        return CheckpointOutcome::FailAndShutdown;
    }

    kunit::diag_usize(
        "uart_irq_source",
        ns16550a::uart8250_port_irq_source() as usize,
    );
    kunit::diag_usize(
        "uart_logical_irq_valid",
        if ns16550a::uart8250_port_logical_irq().is_valid() {
            1
        } else {
            0
        },
    );
    kunit::diag_usize(
        "uart_irq_handler_calls",
        ns16550a::uart8250_irq_handler_call_count(),
    );
    kunit::pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn observer_baseline_valid(ctx: &Context) -> bool {
    let logical_irq = ns16550a::uart8250_port_logical_irq();
    let source = ns16550a::uart8250_port_irq_source();

    ctx.plic.state() == State::Ready
        && ctx.plic.external_irq_route_deferred()
        && ctx.plic_irq_domain.state() == State::Ready
        && ctx.plic_irq_domain.enable_deferred()
        && ctx.irq_handler_registry.state() == State::Ready
        && ctx.irq_handler_registry.source_enable_deferred()
        && ctx.irq_handler_registry.dispatch_deferred()
        && ns16550a::uart8250_port_irq_resource_ready()
        && ns16550a::uart8250_port_logical_irq_ready()
        && ns16550a::uart8250_irq_handler_registered()
        && ns16550a::uart8250_irq_handler_hardirq_context_required()
        && ns16550a::uart8250_irq_handler_dispatch_deferred()
        && ns16550a::uart8250_irq_handler_call_count() == 0
        && plic_mapping_deferred(ctx, source, logical_irq)
        && irq_action_deferred(ctx, logical_irq)
}

fn plic_mapping_deferred(ctx: &Context, source: u32, logical_irq: LogicalIrq) -> bool {
    ctx.plic_irq_domain
        .mapping_for_source(source)
        .is_some_and(|mapping| {
            mapping.logical_irq() == logical_irq
                && mapping.domain_bound()
                && mapping.source_valid()
                && mapping.source_not_enabled()
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
                && action.dispatch_deferred()
        })
}
