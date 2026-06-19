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
    ctx.virtio_bus.setup(&ctx.platform_bus)?;
    ctx.hwrng_core.setup(&ctx.driver_core_base)?;
    ctx.block_device_registry.setup(&ctx.driver_core_base)?;
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
    crate::objects::plic_provider::setup_registered_provider(&ctx.device_tree, &ctx.plic)?;
    crate::objects::virtio_blk::setup_live_driver(
        &mut ctx.virtio_blk_runtime,
        &ctx.virtio_bus,
        &mut ctx.block_device_registry,
        &ctx.kernel_image,
        &ctx.plic,
        &mut ctx.plic_irq_domain,
    )?;
    crate::objects::virtio_rng::setup_live_driver(
        &mut ctx.virtio_rng_runtime,
        &ctx.virtio_bus,
        &mut ctx.hwrng_core,
        &ctx.kernel_image,
        &ctx.plic,
        &mut ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.devfs.setup(
        &mut ctx.vfs_core,
        &ctx.hwrng_core,
        &ctx.block_device_registry,
    )?;
    ctx.uart_external_irq_enable.setup(
        &ctx.plic,
        &mut ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
        &mut ctx.interrupt_stream,
    )?;
    crate::objects::plic_provider::exercise_uart_leaf_chip_callbacks(
        crate::objects::ns16550a::uart8250_port_irq_source(),
        crate::objects::ns16550a::uart8250_port_logical_irq(),
    )?;
    crate::objects::plic_provider::exercise_unmapped_irq_boundary()?;
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
        &ctx.virtio_bus,
        &ctx.devfs,
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
    crate::checkpoint::dispatch(Checkpoint::InitcallBoundaryReady, ctx);
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

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !initcall_phase_ready(
        &ctx.cpuset_smp_trimmed,
        &ctx.driver_core_base,
        &ctx.platform_bus_root_device,
        &ctx.platform_bus,
        &ctx.virtio_bus,
        &ctx.devfs,
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
