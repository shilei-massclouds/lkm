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
    run_linux_plic_initcall6(ctx)?;
    ctx.uart_external_irq_enable.setup(
        &ctx.plic,
        &mut ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
        &mut ctx.interrupt_stream,
    )?;
    exercise_linux_plic_uart_chip_callbacks()?;
    ctx.uart_interrupt_chain_probe.setup(
        &ctx.uart_external_irq_enable,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    enable_serial8250_interrupt_driven_console()?;
    setup_uart_irq_chain_payload_probes(ctx)?;
    ctx.serial8250_rx_loopback_probe.setup(
        &ctx.uart_external_irq_enable,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.serial8250_rx_batch_loopback_probe.setup(
        &ctx.serial8250_rx_loopback_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.tty_xmit_fifo_probe
        .setup(&ctx.serial8250_rx_batch_loopback_probe)?;
    setup_tty_runtime_tx_payload_probes(ctx)?;
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
        &ctx.serial8250_rx_loopback_probe,
        &ctx.serial8250_rx_batch_loopback_probe,
        &ctx.tty_xmit_fifo_probe,
    )?;
    crate::checkpoint::dispatch_mut(Checkpoint::InitcallBoundaryReady, ctx);
    Ok(())
}

fn enable_serial8250_interrupt_driven_console() -> EventResult {
    if !crate::objects::ns16550a::serial8250_runtime_console_tx_ready()
        && !crate::objects::ns16550a::enable_serial8250_interrupt_driven_console()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    if !crate::objects::ns16550a::uart8250_interrupt_driven_configured() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    Ok(())
}

#[cfg(plic_provider_linux_object)]
fn exercise_linux_plic_uart_chip_callbacks() -> EventResult {
    if !crate::objects::linux_plic_shim::exercise_uart_leaf_chip_callbacks(
        crate::objects::ns16550a::uart8250_port_irq_source(),
        crate::objects::ns16550a::uart8250_port_logical_irq(),
    ) || !crate::objects::linux_plic_shim::exercise_unmapped_irq_boundary()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    Ok(())
}

#[cfg(not(plic_provider_linux_object))]
fn exercise_linux_plic_uart_chip_callbacks() -> EventResult {
    Ok(())
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn setup_uart_irq_chain_payload_probes(ctx: &mut Context) -> EventResult {
    ctx.serial8250_console_irq_tx_probe.setup(
        &ctx.uart_external_irq_enable,
        &ctx.uart_interrupt_chain_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.serial8250_console_burst_irq_tx_probe.setup(
        &ctx.serial8250_console_irq_tx_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.serial8250_console_long_irq_tx_probe.setup(
        &ctx.serial8250_console_burst_irq_tx_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.serial8250_console_long_burst_irq_tx_probe.setup(
        &ctx.serial8250_console_long_irq_tx_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.serial8250_console_tx_quiesce_probe.setup(
        &ctx.serial8250_console_long_burst_irq_tx_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )
}

#[cfg(not(checkpoint_handler_uart_irq_chain))]
fn setup_uart_irq_chain_payload_probes(_ctx: &mut Context) -> EventResult {
    Ok(())
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn setup_tty_runtime_tx_payload_probes(ctx: &mut Context) -> EventResult {
    ctx.tty_write_runtime_tx_probe.setup(
        &ctx.tty_xmit_fifo_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.tty_write_batch_runtime_tx_probe.setup(
        &ctx.tty_write_runtime_tx_probe,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )
}

#[cfg(not(checkpoint_handler_uart_irq_chain))]
fn setup_tty_runtime_tx_payload_probes(_ctx: &mut Context) -> EventResult {
    Ok(())
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

#[cfg(plic_provider_linux_object)]
fn run_linux_plic_initcall6(ctx: &Context) -> EventResult {
    crate::objects::linux_plic_shim::run_linux_initcall6()?;
    if !crate::objects::linux_plic_shim::platform_driver_registered()
        || crate::objects::linux_plic_shim::platform_driver_probe_ptr() == 0
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    crate::objects::linux_plic_shim::platform_driver_match_and_probe(&ctx.device_tree, &ctx.plic)
}

#[cfg(not(plic_provider_linux_object))]
fn run_linux_plic_initcall6(_ctx: &Context) -> EventResult {
    Ok(())
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
        &ctx.serial8250_rx_loopback_probe,
        &ctx.serial8250_rx_batch_loopback_probe,
        &ctx.tty_xmit_fifo_probe,
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
