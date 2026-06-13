use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        initcall::{InitcallLevelName, InitcallLevelState, InitcallTable, INITCALL_LEVEL_COUNT},
        printk,
        state::State,
    },
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let platform_bus_root_device = &ctx.platform_bus_root_device;
    let platform_bus = &ctx.platform_bus;

    if !phases::smp_runtime::initcall::is_ready() || !phases::smp_runtime::is_ready() {
        printk::write_str("initcall phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.cpuset_smp_trimmed.state() != State::Ready || !ctx.cpuset_smp_trimmed.trimmed_noop() {
        printk::write_str("cpuset SMP trimmed facts invalid\n");
        return SmokeResult::Failed;
    }

    let driver_core_invalid = ctx.driver_core_base.state() != State::Ready
        || !ctx.driver_core_base.device_registry_ready()
        || !ctx.driver_core_base.bus_registry_ready()
        || !ctx.driver_core_base.pre_platform_deferred()
        || !ctx.driver_core_base.pre_platform_order_preserved()
        || platform_bus_root_device.state() != State::Ready
        || !platform_bus_root_device.early_platform_cleanup_deferred()
        || !platform_bus_root_device.static_device_registered()
        || !platform_bus_root_device.device_name_bound()
        || !platform_bus_root_device.register_return_zero()
        || platform_bus.state() != State::Ready
        || !platform_bus.registered()
        || !platform_bus.devices_kset_ready()
        || !platform_bus.drivers_kset_ready()
        || !platform_bus.autoprobe_enabled()
        || !platform_bus.ops_bound()
        || !platform_bus.register_return_zero()
        || !platform_bus.of_platform_source_tree_ready()
        || !platform_bus.of_platform_root_children_scanned()
        || !platform_bus.of_platform_strict_compatible_required()
        || !platform_bus.of_platform_default_bus_match_table_used()
        || !platform_bus.of_platform_bus_nodes_recurse()
        || !platform_bus.of_platform_candidates_identified()
        || !platform_bus.of_platform_candidates_are_available()
        || !platform_bus.of_platform_candidate_names_printed()
        || !platform_bus.of_platform_candidate_compatibles_printed()
        || !platform_bus.of_platform_devices_created()
        || !platform_bus.of_platform_devices_added()
        || platform_bus.of_platform_candidate_count() == 0
        || platform_bus.platform_device_count() != platform_bus.of_platform_candidate_count()
        || platform_bus.klist_device_count() != platform_bus.of_platform_candidate_count()
        || !check_platform_device_ref_chain(platform_bus, &ctx.device_tree)
        || !platform_bus.ns16550a_driver_registered()
        || !platform_bus.ns16550a_match_table_ready()
        || !platform_bus.ns16550a_device_matched()
        || !platform_bus.ns16550a_probe_called()
        || !platform_bus.ns16550a_probe_return_zero()
        || !check_ns16550a_bound_device(platform_bus, &ctx.device_tree)
        || !platform_bus.ns16550a_probe_ioremaps_uart8250_port()
        || !platform_bus.ns16550a_probe_registers_uart8250_port()
        || !platform_bus.ns16550a_probe_records_uart_irq_resource()
        || !platform_bus.ns16550a_probe_records_uart_irq_mapping()
        || !platform_bus.ns16550a_probe_registers_uart_irq_handler()
        || !platform_bus.ns16550a_probe_keeps_interrupt_output_deferred()
        || !crate::objects::ns16550a::uart8250_port_irq_resource_ready()
        || !crate::objects::ns16550a::uart8250_port_logical_irq_ready()
        || !check_uart_irq_mapping(&ctx.plic_irq_domain)
        || !check_uart_irq_handler(&ctx.irq_handler_registry)
        || ctx.uart_external_irq_enable.state() != State::Ready
        || !ctx.uart_external_irq_enable.plic_source_gate_open()
        || !ctx.uart_external_irq_enable.root_external_input_gate_open()
        || ctx.uart_interrupt_chain_probe.state() != State::Ready
        || !ctx.uart_interrupt_chain_probe.uart_trigger_committed()
        || !ctx.uart_interrupt_chain_probe.plic_claim_observed()
        || !ctx.uart_interrupt_chain_probe.irq_dispatch_observed()
        || !ctx.uart_interrupt_chain_probe.uart_handler_observed()
        || !ctx.uart_interrupt_chain_probe.plic_complete_observed()
        || !ctx.uart_interrupt_chain_probe.plic_loop_exit_observed()
        || !ctx.uart_interrupt_chain_probe.irq_cycle_closed()
        || !ctx.uart_interrupt_chain_probe.console_polling_preserved()
        || ctx.serial8250_console_irq_tx_probe.state() != State::Ready
        || !ctx
            .serial8250_console_irq_tx_probe
            .interrupt_driven_enabled()
        || !ctx
            .serial8250_console_irq_tx_probe
            .printk_frontend_submitted()
        || !ctx.serial8250_console_irq_tx_probe.tx_queue_kicked()
        || !ctx
            .serial8250_console_irq_tx_probe
            .uart_handler_drained_tx()
        || !ctx.serial8250_console_irq_tx_probe.plic_claim_observed()
        || !ctx.serial8250_console_irq_tx_probe.plic_complete_observed()
        || !ctx
            .serial8250_console_irq_tx_probe
            .zero_claim_loop_exit_observed()
        || !ctx
            .serial8250_console_irq_tx_probe
            .tx_queue_empty_after_irq()
        || !ctx
            .serial8250_console_irq_tx_probe
            .local_irq_guard_observed()
        || ctx.serial8250_rx_loopback_probe.state() != State::Ready
        || !ctx.serial8250_rx_loopback_probe.rx_runtime_enabled()
        || !ctx
            .serial8250_rx_loopback_probe
            .loopback_stimulus_committed()
        || !ctx.serial8250_rx_loopback_probe.plic_claim_observed()
        || !ctx.serial8250_rx_loopback_probe.irq_dispatch_observed()
        || !ctx.serial8250_rx_loopback_probe.uart_handler_received_rx()
        || !ctx.serial8250_rx_loopback_probe.flip_buffer_pushed()
        || !ctx.serial8250_rx_loopback_probe.plic_complete_observed()
        || !ctx
            .serial8250_rx_loopback_probe
            .zero_claim_loop_exit_observed()
        || !ctx.serial8250_rx_loopback_probe.irq_cycle_closed()
        || !ctx.serial8250_rx_loopback_probe.last_byte_matched()
        || !crate::objects::ns16550a::uart8250_interrupt_driven_ready()
        || !crate::objects::ns16550a::serial8250_runtime_port_ready()
        || !crate::objects::ns16550a::serial8250_runtime_console_tx_ready()
        || !crate::objects::ns16550a::serial8250_runtime_rx_enabled()
        || !crate::objects::ns16550a::tty_port_ready()
        || !crate::objects::ns16550a::tty_port_not_backend_owner()
        || !crate::objects::ns16550a::tty_flip_buffer_ready()
        || !crate::objects::ns16550a::tty_flip_buffer_pushed()
        || crate::objects::ns16550a::tty_flip_buffer_last_pushed_len() == 0
        || crate::objects::ns16550a::tty_flip_buffer_overflowed()
        || !crate::objects::ns16550a::tty_xmit_fifo_ready()
        || !crate::objects::ns16550a::tty_xmit_fifo_deferred_from_console_tx()
        || crate::objects::ns16550a::uart8250_rx_interrupt_request_count() == 0
        || crate::objects::ns16550a::uart8250_rx_interrupt_handled_count() == 0
        || crate::objects::ns16550a::serial8250_tx_irq_drain_count() == 0
        || crate::objects::ns16550a::serial8250_tx_queue_len() != 0
        || !ctx.interrupt_stream.supervisor_external_input_gate_open()
        || !ctx
            .irq_handler_registry
            .has_handler_for_logical_irq(crate::objects::ns16550a::uart8250_port_logical_irq())
        || !platform_bus.ns16550a_probe_registers_serial_console()
        || !platform_bus.ns16550a_probe_triggers_console_handoff()
        || !ctx.console.registry_ready()
        || !ctx.console.boot_console_registered()
        || !ctx.console.serial_console_registered()
        || !ctx.console.preferred_console_from_stdout()
        || !ctx.console.printk_route_serial_console()
        || !ctx.console.boot_console_unregistered()
        || !ctx.console.handoff_complete()
        || ctx.driver_core_deferred.state() != State::Ready
        || !ctx.driver_core_deferred.post_platform_deferred()
        || !ctx.driver_core_deferred.entry_position_preserved()
        || ctx.irq_proc_view_deferred.state() != State::Ready
        || !ctx.irq_proc_view_deferred.setup_deferred()
        || !ctx.irq_proc_view_deferred.proc_irq_export_deferred();
    if driver_core_invalid {
        printk::write_str("initcall driver core facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.ctor_table.state() != State::Ready
        || !ctx.ctor_table.position_preserved()
        || !ctx.ctor_table.constructors_empty_or_trimmed()
    {
        printk::write_str("ctor table facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.initcall_table.state() != State::Ready
        || !ctx.initcall_table.static_ranges_ready()
        || !ctx.initcall_table.level_count_ready()
        || ctx.initcall_table.level_count() != INITCALL_LEVEL_COUNT
        || !ctx.initcall_table.all_levels_ran()
        || !ctx.initcall_table.entries_recorded_as_properties()
        || !ctx.initcall_table.registered_entries_collected()
        || !ctx.initcall_table.level_mapping_ready()
        || !ctx.initcall_table.run_levels_ready()
        || !ctx.initcall_table.entry_operation_bindings_ready()
        || !ctx.initcall_table.command_line_scratch_reused_per_level()
        || !ctx.initcall_table.param_parser_applied()
        || !ctx.initcall_table.filter_applied()
        || !ctx.initcall_table.run_context_checked()
        || ctx.initcall_table.entry_count() == 0
        || ctx.initcall_table.run_count() != ctx.initcall_table.entry_count()
        || !ctx.initcall_table.all_registered_entries_ran()
    {
        printk::write_str("initcall table summary facts invalid\n");
        return SmokeResult::Failed;
    }

    let mut index = 0usize;
    let mut level_entry_count = 0usize;
    while index < ctx.initcall_table.level_count() {
        let Some(level) = ctx.initcall_table.level(index) else {
            printk::write_str("initcall level missing\n");
            return SmokeResult::Failed;
        };
        if level.name() != expected_level_name(index) || level.state() != InitcallLevelState::Done {
            printk::write_str("initcall level facts invalid\n");
            return SmokeResult::Failed;
        }
        level_entry_count += level.entry_count();
        index += 1;
    }
    if level_entry_count != ctx.initcall_table.entry_count() {
        printk::write_str("initcall level entry count invalid\n");
        return SmokeResult::Failed;
    }

    if !check_entry_records(&ctx.initcall_table)
        || !check_static_section_entries(&ctx.initcall_table)
    {
        printk::write_str("initcall static section subset invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.initcall_boundary.state() != State::Ready || !ctx.initcall_boundary.kunit_next_boundary()
    {
        printk::write_str("initcall boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "initcall platform_bus=ready levels={} entries={} next=kunit\n",
        ctx.initcall_table.level_count(),
        ctx.initcall_table.entry_count()
    ));
    SmokeResult::Passed
}

fn check_platform_device_ref_chain(
    platform_bus: &crate::objects::initcall::PlatformBus,
    device_tree: &crate::objects::device_tree::DeviceTree,
) -> bool {
    let Some(device_ref) = platform_bus.klist_device_ref(0) else {
        return false;
    };
    let Some(platform_device) = platform_bus.platform_device(device_ref) else {
        return false;
    };
    if !platform_device.added()
        || !platform_device.dev().registered()
        || !platform_device.dev().bus_bound()
        || !platform_device.id_bound()
        || !platform_device.resources_bound()
    {
        return false;
    }
    let Some(node) = device_tree.node(platform_device.dev().node_id()) else {
        return false;
    };
    !node.name().is_empty() && node.property(b"compatible").is_some()
}

fn check_entry_records(table: &InitcallTable) -> bool {
    let mut entry_index = 0usize;
    while entry_index < table.entry_count() {
        let Some(entry) = table.entry(entry_index) else {
            return false;
        };
        let Some(level) = table.run_order_level(entry_index) else {
            return false;
        };
        if entry.level() != level
            || entry.skipped()
            || entry.return_code() != 0
            || !entry.run_context_checked()
        {
            return false;
        }
        entry_index += 1;
    }
    true
}

fn check_static_section_entries(table: &InitcallTable) -> bool {
    let expected = [
        (InitcallLevelName::Pure, "pure_smoke_initcall"),
        (InitcallLevelName::Core, "core_smoke_initcall"),
        (InitcallLevelName::Postcore, "postcore_smoke_initcall"),
        (InitcallLevelName::Arch, "of_platform_default_populate_init"),
        (InitcallLevelName::Subsys, "subsys_smoke_initcall"),
        (InitcallLevelName::Fs, "fs_smoke_initcall"),
        (InitcallLevelName::Device, "device_smoke_initcall"),
        (InitcallLevelName::Late, "late_smoke_initcall"),
    ];
    let mut expected_index = 0usize;
    let mut entry_index = 0usize;
    let mut ns16550a_seen = false;

    while entry_index < table.entry_count() {
        let Some(entry) = table.entry(entry_index) else {
            return false;
        };
        if expected_index < expected.len()
            && entry.level() == expected[expected_index].0
            && entry.name() == expected[expected_index].1
            && !entry.skipped()
            && entry.return_code() == 0
            && entry.run_context_checked()
        {
            expected_index += 1;
        }
        if entry.level() == InitcallLevelName::Device
            && entry.name() == "ns16550a_platform_driver_init"
            && !entry.skipped()
            && entry.return_code() == 0
            && entry.run_context_checked()
        {
            ns16550a_seen = true;
        }
        entry_index += 1;
    }

    expected_index == expected.len() && ns16550a_seen
}

fn check_ns16550a_bound_device(
    platform_bus: &crate::objects::initcall::PlatformBus,
    device_tree: &crate::objects::device_tree::DeviceTree,
) -> bool {
    let Some(device_ref) = platform_bus.ns16550a_bound_device() else {
        return false;
    };
    let Some(platform_device) = platform_bus.platform_device(device_ref) else {
        return false;
    };
    let Some(node) = device_tree.node(platform_device.dev().node_id()) else {
        return false;
    };
    node.has_compatible(b"ns16550a")
}

fn check_uart_irq_mapping(domain: &crate::objects::irq_time::PlicIrqDomain) -> bool {
    let source = crate::objects::ns16550a::uart8250_port_irq_source();
    let logical_irq = crate::objects::ns16550a::uart8250_port_logical_irq();
    domain.mapping_count() != 0
        && domain.mapping_for_source(source).is_some_and(|mapping| {
            mapping.logical_irq() == logical_irq
                && mapping.domain_bound()
                && mapping.source_valid()
                && mapping.source_zero_rejected()
                && mapping.source_range_checked()
                && mapping.duplicate_source_idempotent()
                && mapping.source_gate_defined()
                && mapping.source_gate_open()
                && mapping.source_enabled()
                && mapping.handler_not_registered()
        })
}

fn check_uart_irq_handler(registry: &crate::objects::irq_time::IrqHandlerRegistry) -> bool {
    let logical_irq = crate::objects::ns16550a::uart8250_port_logical_irq();
    registry.action_count() != 0
        && crate::objects::ns16550a::uart8250_irq_handler_registered()
        && crate::objects::ns16550a::uart8250_irq_handler_hardirq_context_required()
        && crate::objects::ns16550a::uart8250_irq_handler_dispatch_ready()
        && crate::objects::ns16550a::uart8250_interrupt_trigger_ready()
        && crate::objects::ns16550a::uart8250_thre_interrupt_handled()
        && crate::objects::ns16550a::uart8250_thre_interrupt_request_count() != 0
        && crate::objects::ns16550a::uart8250_thre_interrupt_handled_count() != 0
        && crate::objects::ns16550a::uart8250_thri_disabled_by_handler_count() != 0
        && crate::objects::ns16550a::uart8250_irq_handler_call_count() != 0
        && registry
            .action_for_logical_irq(logical_irq)
            .is_some_and(|action| {
                action.handler_kind() == crate::objects::irq_time::IrqHandlerKind::Ns16550aUart
                    && action.handler_bound()
                    && action.hardirq_context_required()
                    && action.mapped_irq_required()
                    && action.duplicate_registration_rejected()
                    && action.unmapped_registration_rejected()
                    && action.dispatch_ready()
            })
}

const fn expected_level_name(index: usize) -> InitcallLevelName {
    match index {
        0 => InitcallLevelName::Pure,
        1 => InitcallLevelName::Core,
        2 => InitcallLevelName::Postcore,
        3 => InitcallLevelName::Arch,
        4 => InitcallLevelName::Subsys,
        5 => InitcallLevelName::Fs,
        6 => InitcallLevelName::Device,
        _ => InitcallLevelName::Late,
    }
}
