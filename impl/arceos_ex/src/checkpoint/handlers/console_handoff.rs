use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::{context_ref, Context},
    objects::{
        device::DeviceRef,
        device_tree::DeviceTree,
        initcall::PlatformBus,
        ioremap::Ioremap,
        irq_time::{IrqHandlerKind, IrqHandlerRegistry, LogicalIrq, PlicIrqDomain},
        mm_core::{PageAllocator, PageTableCaches, VmallocAllocator},
        ns16550a::{self, NS16550A_PLATFORM_DRIVER_REF},
        printk,
        state::State,
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
pub const KUNIT_CASE_COUNT: usize = 4;

pub const HANDLER: Handler = Handler {
    name: "console_handoff",
    priority: 90,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, _ctx: &mut Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    if !run_default_handoff(total, checkpoint)
        || !run_non_stdout_probe(total, checkpoint)
        || !run_dummy_non_match(total, checkpoint)
        || !run_keep_bootcon(total, checkpoint)
    {
        return CheckpointOutcome::FailAndShutdown;
    }
    CheckpointOutcome::Continue
}

fn run_default_handoff(total: usize, checkpoint: Checkpoint) -> bool {
    let name = "console_handoff.default";
    kunit::start_case(total, "", name, checkpoint);
    let snapshot = ConsoleProbeSnapshot::take();
    let mut fixture = ConsoleHandoffFixture::new();
    let passed = fixture.setup_ready()
        && fixture.add_ns16550a_driver()
        && fixture.probe_stdout_device()
        && fixture.assert_default_handoff()
        && fixture.assert_serial_write_path()
        && fixture.assert_duplicate_preferred_idempotent()
        && fixture.assert_irq_handler_registry_guards();
    snapshot.restore();
    finish(total, name, passed)
}

fn run_non_stdout_probe(total: usize, checkpoint: Checkpoint) -> bool {
    let name = "console_handoff.non_stdout";
    kunit::start_case(total, "", name, checkpoint);
    let snapshot = ConsoleProbeSnapshot::take();
    printk::reset_registry_for_smoke(false);
    printk::register_boot_console();
    ns16550a::reset_probe_state_for_smoke();
    let mut fixture = ConsoleHandoffFixture::new();
    let device_tree = context_ref().device_tree.without_stdout_path_for_smoke();
    let passed = fixture.setup_ready()
        && !device_tree.stdout_path_available()
        && fixture.add_ns16550a_driver()
        && fixture.probe_ns16550a_device(&device_tree)
        && fixture.assert_non_stdout_preserves_boot_route();
    snapshot.restore();
    finish(total, name, passed)
}

fn run_dummy_non_match(total: usize, checkpoint: Checkpoint) -> bool {
    let name = "console_handoff.dummy_non_match";
    kunit::start_case(total, "", name, checkpoint);
    let snapshot = ConsoleProbeSnapshot::take();
    printk::reset_registry_for_smoke(false);
    printk::register_boot_console();
    ns16550a::reset_probe_state_for_smoke();
    let registered = printk::register_serial8250_console(false);
    let passed = !registered
        && !printk::serial8250_console_registered()
        && !printk::preferred_console_from_stdout()
        && !printk::serial8250_consdev()
        && !printk::serial8250_write_ready()
        && printk::boot_console_registered()
        && printk::boot_console_online()
        && !printk::boot_console_unregistered()
        && !printk::boot_console_removed_from_registry()
        && !printk::console_handoff_complete()
        && printk::route() == printk::PrintkRoute::BootConsole;
    snapshot.restore();
    finish(total, name, passed)
}

fn run_keep_bootcon(total: usize, checkpoint: Checkpoint) -> bool {
    let name = "console_handoff.keep_bootcon";
    kunit::start_case(total, "", name, checkpoint);
    let snapshot = ConsoleProbeSnapshot::take();
    printk::reset_registry_for_smoke(true);
    printk::register_boot_console();
    ns16550a::reset_probe_state_for_smoke();
    let mut fixture = ConsoleHandoffFixture::new();
    let passed = fixture.setup_ready()
        && fixture.add_ns16550a_driver()
        && fixture.probe_stdout_device()
        && fixture.assert_keep_bootcon_handoff();
    snapshot.restore();
    finish(total, name, passed)
}

fn finish(total: usize, name: &'static str, passed: bool) -> bool {
    if passed {
        kunit::pass(total, "", name);
    } else {
        kunit::fail(total, "", name, "console handoff facts invalid");
    }
    passed
}

struct ConsoleHandoffFixture {
    bus: PlatformBus,
    page_table_caches: PageTableCaches,
    page_allocator: PageAllocator,
    vmalloc_allocator: VmallocAllocator,
    ioremap: Ioremap,
    plic_irq_domain: PlicIrqDomain,
    irq_handler_registry: IrqHandlerRegistry,
    device_ref: Option<DeviceRef>,
}

impl ConsoleHandoffFixture {
    fn new() -> Self {
        Self {
            bus: PlatformBus::new(),
            page_table_caches: PageTableCaches::new(),
            page_allocator: PageAllocator::new(),
            vmalloc_allocator: VmallocAllocator::new(),
            ioremap: Ioremap::new(),
            plic_irq_domain: PlicIrqDomain::new(),
            irq_handler_registry: IrqHandlerRegistry::new(),
            device_ref: None,
        }
    }

    fn setup_ready(&mut self) -> bool {
        let ctx = context_ref();
        self.page_table_caches
            .setup(
                &ctx.slub_allocator,
                &ctx.page_allocator,
                &ctx.page_metadata_map,
                &ctx.config,
                &ctx.vm,
                &ctx.static_objects,
                &ctx.kernel_image,
            )
            .is_ok()
            && self
                .vmalloc_allocator
                .setup(
                    &ctx.slub_allocator,
                    &self.page_table_caches,
                    &ctx.per_cpu_storage,
                )
                .is_ok()
            && self
                .ioremap
                .setup(
                    &ctx.vm,
                    &self.vmalloc_allocator,
                    &self.page_table_caches,
                    &ctx.fix_map,
                    &ctx.config,
                )
                .is_ok()
            && self
                .bus
                .setup(&ctx.driver_core_base, &ctx.platform_bus_root_device)
                .is_ok()
            && self
                .plic_irq_domain
                .preset(&ctx.plic, &ctx.irq_controller)
                .is_ok()
            && self
                .plic_irq_domain
                .setup(&ctx.plic, &ctx.irq_controller)
                .is_ok()
            && self
                .irq_handler_registry
                .setup(&ctx.irq_controller, &self.plic_irq_domain)
                .is_ok()
            && self.bus.state() == State::Ready
            && self.plic_irq_domain.state() == State::Ready
            && self.irq_handler_registry.state() == State::Ready
    }

    fn add_ns16550a_driver(&mut self) -> bool {
        self.bus.add_driver(NS16550A_PLATFORM_DRIVER_REF).is_ok()
    }

    fn probe_stdout_device(&mut self) -> bool {
        let ctx = context_ref();
        self.probe_ns16550a_device(&ctx.device_tree)
    }

    fn probe_ns16550a_device(&mut self, device_tree: &DeviceTree) -> bool {
        let ctx = context_ref();
        let Some(serial) = find_ns16550a_node(device_tree) else {
            return false;
        };
        let Ok(device_ref) = self.bus.add_smoke_platform_device(device_tree, serial.id()) else {
            return false;
        };
        self.device_ref = Some(device_ref);
        self.bus
            .probe_device(
                device_ref,
                device_tree,
                &mut self.vmalloc_allocator,
                &mut self.page_table_caches,
                &mut self.page_allocator,
                &ctx.page_metadata_map,
                &ctx.config,
                &mut self.ioremap,
                &mut self.plic_irq_domain,
                &mut self.irq_handler_registry,
            )
            .is_ok()
    }

    fn assert_default_handoff(&self) -> bool {
        let Some(device_ref) = self.device_ref else {
            return false;
        };
        self.assert_probe_and_mapping(device_ref)
            && ns16550a::stdout_path_matched()
            && ns16550a::uart8250_port_device_ref() == Some(device_ref)
            && self.bus.ns16550a_probe_registers_serial_console()
            && ns16550a::serial8250_console_registered()
            && self.bus.ns16550a_probe_triggers_console_handoff()
            && ns16550a::handoff_triggered()
            && printk::console_handoff_complete()
            && printk::route() == printk::PrintkRoute::Serial8250
            && printk::boot_pending_flushed_before_serial_handoff()
            && printk::legacy_earlycon_drain_blocked_after_handoff()
            && printk::serial8250_online_trace_emitted()
            && printk::boot_console_offline_trace_emitted()
            && !printk::keep_bootcon()
            && printk::boot_console_registered()
            && !printk::boot_console_online()
            && printk::boot_console_unregistered()
            && printk::boot_console_removed_from_registry()
    }

    fn assert_probe_and_mapping(&self, device_ref: DeviceRef) -> bool {
        let Some(mapping) = self.ioremap.mapping_for_device(device_ref) else {
            return false;
        };
        let area = mapping.vmap_area();
        let vmap_mapping = mapping.vmap_mapping();
        self.bus.probe_device_scanned_drivers()
            && self.bus.ns16550a_device_matched()
            && self.bus.ns16550a_probe_called()
            && self.bus.ns16550a_probe_return_zero()
            && self.bus.ns16550a_bound_device() == Some(device_ref)
            && self.bus.ns16550a_probe_ioremaps_uart8250_port()
            && ns16550a::uart8250_port_ioremapped()
            && ns16550a::uart8250_port_resources_ready()
            && self.vmalloc_allocator.area_count() != 0
            && self.vmalloc_allocator.mapping_count() != 0
            && area.busy()
            && area.vm_struct_metadata_ready()
            && area.vmap_area_metadata_ready()
            && area.is_vm_ioremap()
            && area.flags().is_vm_ioremap()
            && area.end() == area.virt_base().saturating_add(area.size())
            && self.vmalloc_allocator.area(area.index()) == Some(area)
            && vmap_mapping.record_created()
            && vmap_mapping.installed()
            && vmap_mapping.area() == area
            && vmap_mapping.index() == 0
            && vmap_mapping.phys_base() == mapping.page_phys_base()
            && vmap_mapping.size() == mapping.mapped_size()
            && vmap_mapping.protection().is_io_memory()
            && self.vmalloc_allocator.mapping(vmap_mapping.index()) == Some(vmap_mapping)
            && self.bus.ns16550a_probe_registers_uart8250_port()
            && self.bus.ns16550a_probe_records_uart_irq_resource()
            && self.bus.ns16550a_probe_records_uart_irq_mapping()
            && self.bus.ns16550a_probe_registers_uart_irq_handler()
            && self.bus.ns16550a_probe_keeps_interrupt_output_deferred()
            && ns16550a::uart8250_port_registered()
            && ns16550a::uart8250_port_irq_resource_ready()
            && ns16550a::uart8250_port_logical_irq_ready()
            && ns16550a::uart8250_irq_handler_registered()
            && ns16550a::uart8250_irq_handler_hardirq_context_required()
            && ns16550a::uart8250_irq_handler_dispatch_ready()
            && self
                .plic_irq_domain
                .mapping_for_source(ns16550a::uart8250_port_irq_source())
                .is_some_and(|mapping| {
                    mapping.logical_irq() == ns16550a::uart8250_port_logical_irq()
                        && mapping.source_gate_defined()
                        && mapping.source_gate_closed()
                        && mapping.source_enable_deferred()
                        && mapping.source_not_enabled()
                        && mapping.handler_not_registered()
                })
            && self
                .irq_handler_registry
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
            && printk::serial8250_write_ready()
            && ns16550a::serial8250_write_backend_ready()
            && ns16550a::serial8250_write_uses_membase()
            && ns16550a::serial8250_write_uses_lsr_thr_polling()
            && ns16550a::serial8250_write_does_not_use_sbi()
            && ns16550a::serial8250_interrupt_output_deferred()
    }

    fn assert_serial_write_path(&self) -> bool {
        let write_calls = ns16550a::serial8250_write_call_count();
        let tx_bytes = ns16550a::serial8250_tx_byte_count();
        printk::write_str("serial8250 kunit\n");
        ns16550a::serial8250_write_call_count() == write_calls.saturating_add(1)
            && ns16550a::serial8250_tx_byte_count()
                == tx_bytes.saturating_add("serial8250 kunit\n".len() + 1)
            && ns16550a::serial8250_mmio_writes_performed()
            && !ns16550a::serial8250_write_timed_out()
            && printk::serial8250_delivered_records_not_replayed()
    }

    fn assert_duplicate_preferred_idempotent(&self) -> bool {
        let area_count = self.vmalloc_allocator.area_count();
        let mapping_count = self.vmalloc_allocator.mapping_count();
        let irq_mapping_count = self.plic_irq_domain.mapping_count();
        printk::register_serial8250_console(true)
            && self.vmalloc_allocator.area_count() == area_count
            && self.vmalloc_allocator.mapping_count() == mapping_count
            && self.plic_irq_domain.mapping_count() == irq_mapping_count
            && self.irq_handler_registry.action_count() == 1
            && printk::serial8250_console_registered()
            && printk::preferred_console_from_stdout()
            && printk::serial8250_consdev()
            && printk::serial8250_write_ready()
            && printk::console_handoff_complete()
            && printk::route() == printk::PrintkRoute::Serial8250
    }

    fn assert_irq_handler_registry_guards(&mut self) -> bool {
        let Some(device_ref) = self.device_ref else {
            return false;
        };
        let action_count = self.irq_handler_registry.action_count();
        let duplicate = self.irq_handler_registry.request_irq(
            &self.plic_irq_domain,
            ns16550a::uart8250_port_logical_irq(),
            device_ref,
            IrqHandlerKind::Ns16550aUart,
        );
        let unmapped = self.irq_handler_registry.request_irq(
            &self.plic_irq_domain,
            LogicalIrq::new(usize::MAX - 1),
            device_ref,
            IrqHandlerKind::Ns16550aUart,
        );
        !duplicate
            && !unmapped
            && self.irq_handler_registry.action_count() == action_count
            && self.irq_handler_registry.duplicate_registration_rejected()
            && self.irq_handler_registry.unmapped_registration_rejected()
            && self
                .irq_handler_registry
                .has_handler_for_logical_irq(ns16550a::uart8250_port_logical_irq())
    }

    fn assert_non_stdout_preserves_boot_route(&self) -> bool {
        let Some(device_ref) = self.device_ref else {
            return false;
        };
        self.bus.ns16550a_probe_registers_uart8250_port()
            && ns16550a::uart8250_port_registered()
            && self.bus.ns16550a_probe_records_uart_irq_resource()
            && self.bus.ns16550a_probe_records_uart_irq_mapping()
            && self.bus.ns16550a_probe_registers_uart_irq_handler()
            && self.bus.ns16550a_probe_keeps_interrupt_output_deferred()
            && self.bus.ns16550a_probe_ioremaps_uart8250_port()
            && ns16550a::uart8250_port_ioremapped()
            && ns16550a::uart8250_irq_handler_registered()
            && self.ioremap.mapping_for_device(device_ref).is_some()
            && !ns16550a::stdout_path_available()
            && !ns16550a::stdout_path_matched()
            && !self.bus.ns16550a_probe_registers_serial_console()
            && !self.bus.ns16550a_probe_triggers_console_handoff()
            && !ns16550a::serial8250_console_registered()
            && !ns16550a::serial8250_write_backend_ready()
            && !ns16550a::handoff_triggered()
            && printk::boot_console_registered()
            && printk::boot_console_online()
            && !printk::serial8250_console_registered()
            && !printk::preferred_console_from_stdout()
            && !printk::console_handoff_complete()
            && !printk::boot_console_unregistered()
            && !printk::boot_console_removed_from_registry()
            && printk::route() == printk::PrintkRoute::BootConsole
    }

    fn assert_keep_bootcon_handoff(&self) -> bool {
        let Some(device_ref) = self.device_ref else {
            return false;
        };
        self.assert_probe_and_mapping(device_ref)
            && self.bus.ns16550a_probe_registers_serial_console()
            && ns16550a::serial8250_console_registered()
            && printk::serial8250_console_registered()
            && printk::preferred_console_from_stdout()
            && printk::keep_bootcon()
            && printk::boot_console_registered()
            && printk::boot_console_online()
            && !printk::boot_console_unregistered()
            && !printk::boot_console_removed_from_registry()
            && printk::console_handoff_complete()
            && printk::boot_pending_flushed_before_serial_handoff()
            && printk::legacy_earlycon_drain_blocked_after_handoff()
            && printk::serial8250_online_trace_emitted()
            && !printk::boot_console_offline_trace_emitted()
            && ns16550a::handoff_triggered()
            && printk::serial8250_consdev()
            && printk::serial8250_write_ready()
            && printk::route() == printk::PrintkRoute::Serial8250
    }
}

struct ConsoleProbeSnapshot {
    registry: printk::ConsoleRegistry,
    probe: ns16550a::Ns16550aProbeState,
}

impl ConsoleProbeSnapshot {
    fn take() -> Self {
        Self {
            registry: printk::registry_snapshot(),
            probe: ns16550a::probe_state_snapshot(),
        }
    }

    fn restore(self) {
        printk::restore_registry(self.registry);
        ns16550a::restore_probe_state(self.probe);
    }
}

fn find_ns16550a_node(
    device_tree: &DeviceTree,
) -> Option<crate::objects::device_tree::DeviceNodeRef<'_>> {
    let root = device_tree.root()?;
    find_ns16550a_node_from(root)
}

fn find_ns16550a_node_from(
    node: crate::objects::device_tree::DeviceNodeRef<'_>,
) -> Option<crate::objects::device_tree::DeviceNodeRef<'_>> {
    if node.has_compatible(b"ns16550a") {
        return Some(node);
    }

    for child in node.children() {
        if let Some(found) = find_ns16550a_node_from(child) {
            return Some(found);
        }
    }
    None
}
