use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::{context_ref, Context},
    objects::{
        device::DeviceRef,
        device_tree::DeviceTree,
        initcall::PlatformBus,
        ioremap::Ioremap,
        mm_core::VmallocAllocator,
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
        && fixture.assert_duplicate_preferred_idempotent();
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
    vmalloc_allocator: VmallocAllocator,
    ioremap: Ioremap,
    device_ref: Option<DeviceRef>,
}

impl ConsoleHandoffFixture {
    fn new() -> Self {
        Self {
            bus: PlatformBus::new(),
            vmalloc_allocator: VmallocAllocator::new(),
            ioremap: Ioremap::new(),
            device_ref: None,
        }
    }

    fn setup_ready(&mut self) -> bool {
        let ctx = context_ref();
        self.vmalloc_allocator
            .setup(
                &ctx.slub_allocator,
                &ctx.page_table_caches,
                &ctx.per_cpu_storage,
            )
            .is_ok()
            && self
                .ioremap
                .setup(
                    &ctx.vm,
                    &self.vmalloc_allocator,
                    &ctx.page_table_caches,
                    &ctx.fix_map,
                    &ctx.config,
                )
                .is_ok()
            && self
                .bus
                .setup(&ctx.driver_core_base, &ctx.platform_bus_root_device)
                .is_ok()
            && self.bus.state() == State::Ready
    }

    fn add_ns16550a_driver(&mut self) -> bool {
        self.bus.add_driver(NS16550A_PLATFORM_DRIVER_REF).is_ok()
    }

    fn probe_stdout_device(&mut self) -> bool {
        let ctx = context_ref();
        self.probe_ns16550a_device(&ctx.device_tree)
    }

    fn probe_ns16550a_device(&mut self, device_tree: &DeviceTree) -> bool {
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
                &mut self.ioremap,
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
            && area.is_vm_ioremap()
            && area.flags().is_vm_ioremap()
            && area.end() == area.virt_base().saturating_add(area.size())
            && self.vmalloc_allocator.area(area.index()) == Some(area)
            && vmap_mapping.installed()
            && vmap_mapping.area() == area
            && vmap_mapping.index() == 0
            && vmap_mapping.phys_base() == mapping.page_phys_base()
            && vmap_mapping.size() == mapping.mapped_size()
            && vmap_mapping.protection().is_io_memory()
            && self.vmalloc_allocator.mapping(vmap_mapping.index()) == Some(vmap_mapping)
            && self.bus.ns16550a_probe_registers_uart8250_port()
            && ns16550a::uart8250_port_registered()
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
        printk::register_serial8250_console(true)
            && self.vmalloc_allocator.area_count() == area_count
            && self.vmalloc_allocator.mapping_count() == mapping_count
            && printk::serial8250_console_registered()
            && printk::preferred_console_from_stdout()
            && printk::serial8250_consdev()
            && printk::serial8250_write_ready()
            && printk::console_handoff_complete()
            && printk::route() == printk::PrintkRoute::Serial8250
    }

    fn assert_non_stdout_preserves_boot_route(&self) -> bool {
        let Some(device_ref) = self.device_ref else {
            return false;
        };
        self.bus.ns16550a_probe_registers_uart8250_port()
            && ns16550a::uart8250_port_registered()
            && self.bus.ns16550a_probe_ioremaps_uart8250_port()
            && ns16550a::uart8250_port_ioremapped()
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
