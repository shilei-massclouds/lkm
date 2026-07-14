use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        initcall::initcall_phase_ready_diagnostic,
        state::{
            EventResult, FailureDiagnostic, LifecycleEvent, State, failed_condition,
            failed_condition_with_diagnostic,
        },
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static INITCALL_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(preset_start(), "arceos_ex initcall preset start failed\n");
    crate::checkpoint::checkpoint(Checkpoint::InitcallPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| {
            transition_if_ready(
                ctx,
                LifecycleEvent::Preset,
                State::Base,
                State::Prepared,
                Checkpoint::InitcallPhasePrepared,
                "adopt_prepared",
            )
        }),
        "arceos_ex initcall preset failed\n",
    );
    setup(ctx)
}

fn preset_start() -> EventResult {
    let state = crate::phases::state::load(&INITCALL_PHASE_STATE);
    if state != State::Base || !crate::phases::smp_runtime::runtime_core::is_online() {
        return failed_condition_with_diagnostic(
            LifecycleEvent::Preset,
            state,
            State::Base,
            State::Prepared,
            initcall_failure_diagnostic(
                "preset_start.runtime_core_online",
                "RuntimeCorePhase",
                "runtime_core_phase_online",
            ),
        );
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    with_initcall_diagnostic(
        ctx.cpuset_smp_trimmed.setup(&ctx.runtime_core_boundary),
        "preset_objects.cpuset_smp_trimmed.setup",
        "CpusetSmpTrimmed",
        "cpuset_smp_trimmed.setup",
    )?;
    with_initcall_diagnostic(
        ctx.driver_core_base
            .setup(&ctx.cpuset_smp_trimmed, &ctx.page_allocator, &ctx.workqueue),
        "preset_objects.driver_core_base.setup",
        "DriverCoreBase",
        "driver_core_base.setup",
    )?;
    with_initcall_diagnostic(
        ctx.platform_bus_root_device
            .setup(&ctx.driver_core_base, &ctx.static_objects),
        "preset_objects.platform_bus_root_device.setup",
        "PlatformBusRootDevice",
        "platform_bus_root_device.setup",
    )?;
    with_initcall_diagnostic(
        ctx.platform_bus
            .setup(&ctx.driver_core_base, &ctx.platform_bus_root_device),
        "preset_objects.platform_bus.setup",
        "PlatformBus",
        "platform_bus.setup",
    )?;
    with_initcall_diagnostic(
        ctx.virtio_bus.setup(&ctx.platform_bus),
        "preset_objects.virtio_bus.setup",
        "VirtioBus",
        "virtio_bus.setup",
    )?;
    with_initcall_diagnostic(
        ctx.hwrng_core.setup(&ctx.driver_core_base),
        "preset_objects.hwrng_core.setup",
        "HwRngCore",
        "hwrng_core.setup",
    )?;
    with_initcall_diagnostic(
        ctx.block_device_registry.setup(&ctx.driver_core_base),
        "preset_objects.block_device_registry.setup",
        "BlockDeviceRegistry",
        "block_device_registry.setup",
    )?;
    with_initcall_diagnostic(
        ctx.driver_core_deferred.setup(&ctx.platform_bus),
        "preset_objects.driver_core_deferred.setup",
        "DriverCoreDeferred",
        "driver_core_deferred.setup",
    )?;
    with_initcall_diagnostic(
        ctx.irq_proc_view_deferred.setup(
            &ctx.driver_core_base,
            &ctx.platform_bus_root_device,
            &ctx.platform_bus,
            &ctx.driver_core_deferred,
            &ctx.irq_dispatch_tree,
        ),
        "preset_objects.irq_proc_view_deferred.setup",
        "IrqProcViewDeferred",
        "irq_proc_view_deferred.setup",
    )?;
    with_initcall_diagnostic(
        ctx.ctor_table
            .setup(&ctx.irq_proc_view_deferred, &ctx.static_objects),
        "preset_objects.ctor_table.setup",
        "CtorTable",
        "ctor_table.setup",
    )?;
    with_initcall_diagnostic(
        ctx.initcall_table
            .preset(&ctx.ctor_table, &ctx.static_objects),
        "preset_objects.initcall_table.preset",
        "InitcallTable",
        "initcall_table.preset",
    )?;
    with_initcall_diagnostic(
        run_initcall_table(ctx),
        "preset_objects.initcall_table.setup",
        "InitcallTable",
        "initcall_table.setup",
    )?;
    with_initcall_diagnostic(
        crate::objects::plic_provider::setup_registered_provider(&ctx.device_tree, &ctx.plic),
        "preset_objects.plic_provider.setup_registered_provider",
        "PlicProvider",
        "plic_provider.setup_registered_provider",
    )?;
    with_initcall_diagnostic(
        crate::objects::virtio_blk::setup_live_driver(
            &mut ctx.virtio_blk_runtime,
            &ctx.virtio_bus,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            &ctx.plic,
            &mut ctx.plic_irq_domain,
        ),
        "preset_objects.virtio_blk.setup_live_driver",
        "VirtioBlkDevice",
        "virtio_blk.setup_live_driver",
    )?;
    with_initcall_diagnostic(
        crate::objects::virtio_rng::setup_live_driver(
            &mut ctx.virtio_rng_runtime,
            &ctx.virtio_bus,
            &mut ctx.hwrng_core,
            &ctx.kernel_image,
            &ctx.plic,
            &mut ctx.plic_irq_domain,
            &ctx.irq_handler_registry,
        ),
        "preset_objects.virtio_rng.setup_live_driver",
        "VirtioRngDevice",
        "virtio_rng.setup_live_driver",
    )?;
    with_initcall_diagnostic(
        ctx.devfs.setup(
            &mut ctx.vfs_core,
            &ctx.fs_struct,
            &ctx.hwrng_core,
            &ctx.block_device_registry,
        ),
        "preset_objects.devfs.setup",
        "DevFs",
        "devfs.setup",
    )?;
    with_initcall_diagnostic(
        ctx.uart_external_irq_enable.setup(
            &ctx.plic,
            &mut ctx.plic_irq_domain,
            &ctx.irq_handler_registry,
            &mut ctx.interrupt_stream,
        ),
        "preset_objects.uart_external_irq_enable.setup",
        "UartExternalIrqEnable",
        "uart_external_irq_enable.setup",
    )?;
    with_initcall_diagnostic(
        crate::objects::plic_provider::exercise_uart_leaf_chip_callbacks(
            crate::objects::ns16550a::uart8250_port_irq_source(),
            crate::objects::ns16550a::uart8250_port_logical_irq(),
        ),
        "preset_objects.plic_provider.exercise_uart_leaf_chip_callbacks",
        "PlicProvider",
        "plic_provider.exercise_uart_leaf_chip_callbacks",
    )?;
    with_initcall_diagnostic(
        crate::objects::plic_provider::exercise_unmapped_irq_boundary(),
        "preset_objects.plic_provider.exercise_unmapped_irq_boundary",
        "PlicProvider",
        "plic_provider.exercise_unmapped_irq_boundary",
    )?;
    with_initcall_diagnostic(
        ctx.uart_interrupt_chain_probe.setup(
            &ctx.uart_external_irq_enable,
            &ctx.plic,
            &ctx.plic_irq_domain,
            &ctx.irq_handler_registry,
        ),
        "preset_objects.uart_interrupt_chain_probe.setup",
        "UartInterruptChainProbe",
        "uart_interrupt_chain_probe.setup",
    )?;
    with_initcall_diagnostic(
        enable_serial8250_interrupt_driven_console(),
        "preset_objects.serial8250_console.enable_interrupt_driven",
        "Serial8250Console",
        "serial8250_console.enable_interrupt_driven",
    )?;
    with_initcall_diagnostic(
        setup_uart_irq_chain_payload_probes(ctx),
        "preset_objects.uart_irq_chain_payload_probes.setup",
        "UartIrqChainPayloadProbes",
        "uart_irq_chain_payload_probes.setup",
    )?;
    with_initcall_diagnostic(
        ctx.serial8250_rx_loopback_probe.setup(
            &ctx.uart_external_irq_enable,
            &ctx.plic,
            &ctx.plic_irq_domain,
            &ctx.irq_handler_registry,
        ),
        "preset_objects.serial8250_rx_loopback_probe.setup",
        "Serial8250RxLoopbackProbe",
        "serial8250_rx_loopback_probe.setup",
    )?;
    with_initcall_diagnostic(
        ctx.serial8250_rx_batch_loopback_probe.setup(
            &ctx.serial8250_rx_loopback_probe,
            &ctx.plic,
            &ctx.plic_irq_domain,
            &ctx.irq_handler_registry,
        ),
        "preset_objects.serial8250_rx_batch_loopback_probe.setup",
        "Serial8250RxBatchLoopbackProbe",
        "serial8250_rx_batch_loopback_probe.setup",
    )?;
    with_initcall_diagnostic(
        ctx.tty_xmit_fifo_probe
            .setup(&ctx.serial8250_rx_batch_loopback_probe),
        "preset_objects.tty_xmit_fifo_probe.setup",
        "TtyXmitFifoProbe",
        "tty_xmit_fifo_probe.setup",
    )?;
    with_initcall_diagnostic(
        setup_tty_runtime_tx_payload_probes(ctx),
        "preset_objects.tty_runtime_tx_payload_probes.setup",
        "TtyRuntimeTxPayloadProbes",
        "tty_runtime_tx_payload_probes.setup",
    )?;
    with_initcall_diagnostic(
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
        ),
        "preset_objects.initcall_boundary.setup",
        "InitcallBoundary",
        "initcall_boundary.setup",
    )?;
    Ok(())
}

fn enable_serial8250_interrupt_driven_console() -> EventResult {
    if !crate::objects::ns16550a::serial8250_runtime_console_tx_ready()
        && !crate::objects::ns16550a::enable_serial8250_interrupt_driven_console()
    {
        return failed_condition(
            LifecycleEvent::Preset,
            State::Base,
            State::Base,
            State::Prepared,
        );
    }

    if !crate::objects::ns16550a::uart8250_interrupt_driven_configured() {
        return failed_condition(
            LifecycleEvent::Preset,
            State::Base,
            State::Base,
            State::Prepared,
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

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InitcallPhaseReady,
            "adopt_ready",
        ),
        "arceos_ex initcall setup failed\n",
    );
    enable(ctx)
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::InitcallPhaseOnline,
            "enable_event",
        ),
        "arceos_ex initcall enable failed\n",
    );
    crate::phases::smp_runtime::enable_after_initcall()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
    step: &'static str,
) -> EventResult {
    let state = crate::phases::state::load(&INITCALL_PHASE_STATE);
    if let Some(diagnostic) = initcall_phase_ready_diagnostic(
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
        return failed_condition_with_diagnostic(
            event,
            state,
            expected,
            target,
            initcall_failure_diagnostic(step, "InitcallPhase", diagnostic.first_failed()),
        );
    }

    if state != expected {
        return failed_condition_with_diagnostic(
            event,
            state,
            expected,
            target,
            initcall_failure_diagnostic(step, "InitcallPhase", "phase_source_state"),
        );
    }

    crate::phases::state::mark_checked(&INITCALL_PHASE_STATE, event, expected, target, checkpoint)
}

pub fn is_online() -> bool {
    crate::phases::state::load(&INITCALL_PHASE_STATE) == State::Online
}

fn with_initcall_diagnostic(
    result: EventResult,
    step: &'static str,
    object: &'static str,
    check: &'static str,
) -> EventResult {
    result.map_err(|error| {
        error.with_diagnostic_if_absent(initcall_failure_diagnostic(step, object, check))
    })
}

fn initcall_failure_diagnostic(
    step: &'static str,
    object: &'static str,
    check: &'static str,
) -> FailureDiagnostic {
    FailureDiagnostic::new("InitcallPhase", step, object, check, check)
}
