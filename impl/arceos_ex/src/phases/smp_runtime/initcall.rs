use crate::{
    context::Context,
    objects::{
        initcall::initcall_phase_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static INITCALL_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::InitcallPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex initcall event failed\n",
    );
    crate::phases::smp_runtime::rootfs::setup(ctx)
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::smp_runtime::runtime_core::is_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    ctx.cpuset_smp_trimmed.setup(&ctx.runtime_core_boundary)?;
    ctx.driver_core_base
        .setup(&ctx.cpuset_smp_trimmed, &ctx.page_allocator, &ctx.workqueue)?;
    ctx.platform_bus_root_device
        .setup(&ctx.driver_core_base, &ctx.static_objects)?;
    ctx.platform_bus
        .setup(&ctx.driver_core_base, &ctx.platform_bus_root_device)?;
    ctx.driver_core_deferred.setup(&ctx.platform_bus)?;
    ctx.irq_proc_view_deferred.setup(
        &ctx.driver_core_base,
        &ctx.platform_bus_root_device,
        &ctx.platform_bus,
        &ctx.driver_core_deferred,
        &ctx.irq_dispatch_tree,
    )?;
    ctx.ctor_table
        .setup(&ctx.irq_proc_view_deferred, &ctx.static_objects)?;
    ctx.initcall_table
        .preset(&ctx.ctor_table, &ctx.static_objects)?;
    run_initcall_table(ctx)?;
    ctx.uart_external_irq_enable.setup(
        &ctx.plic,
        &mut ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
        &mut ctx.interrupt_stream,
    )?;
    ctx.uart_interrupt_chain_probe.setup(
        &ctx.uart_external_irq_enable,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.serial8250_console_irq_tx_probe.setup(
        &ctx.uart_external_irq_enable,
        &ctx.uart_interrupt_chain_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.initcall_boundary.setup(
        &ctx.cpuset_smp_trimmed,
        &ctx.driver_core_base,
        &ctx.platform_bus_root_device,
        &ctx.platform_bus,
        &ctx.driver_core_deferred,
        &ctx.irq_proc_view_deferred,
        &ctx.ctor_table,
        &ctx.initcall_table,
        &ctx.uart_external_irq_enable,
        &ctx.uart_interrupt_chain_probe,
        &ctx.serial8250_console_irq_tx_probe,
    )
}

fn run_initcall_table(ctx: &mut Context) -> EventResult {
    let mut table = core::mem::replace(
        &mut ctx.initcall_table,
        crate::objects::initcall::InitcallTable::new(),
    );
    let result = table.setup(ctx);
    ctx.initcall_table = table;
    if result.is_ok() {
        ctx.console.refresh_handoff();
    }
    result
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !initcall_phase_ready(
        &ctx.cpuset_smp_trimmed,
        &ctx.driver_core_base,
        &ctx.platform_bus_root_device,
        &ctx.platform_bus,
        &ctx.driver_core_deferred,
        &ctx.irq_proc_view_deferred,
        &ctx.ctor_table,
        &ctx.initcall_table,
        &ctx.uart_external_irq_enable,
        &ctx.uart_interrupt_chain_probe,
        &ctx.serial8250_console_irq_tx_probe,
        &ctx.initcall_boundary,
    ) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&INITCALL_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &INITCALL_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::InitcallPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&INITCALL_PHASE_STATE) == State::Ready
}
