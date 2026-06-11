use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        device::DeviceRef,
        device_tree::DeviceTree,
        driver::MOCK_PLATFORM_DRIVER_REF,
        initcall::PlatformBus,
        ioremap::Ioremap,
        mm_core::VmallocAllocator,
        ns16550a::{self, NS16550A_PLATFORM_DRIVER_REF},
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut AddDeviceProbeDriverScenario::new());
    suite.scenario(&mut AddDriverProbeDeviceScenario::new());
    suite.scenario(&mut Ns16550aProbeDeviceScenario::new());
    suite.scenario(&mut DummyConsoleNonStdoutScenario::new());
    suite.scenario(&mut KeepBootconScenario::new());
    suite.result()
}

struct PlatformBusActionsFixture {
    bus: PlatformBus,
    vmalloc_allocator: VmallocAllocator,
    ioremap: Ioremap,
}

impl PlatformBusActionsFixture {
    fn new() -> Self {
        Self {
            bus: PlatformBus::new(),
            vmalloc_allocator: VmallocAllocator::new(),
            ioremap: Ioremap::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "setup vmalloc allocator",
            self.vmalloc_allocator.setup(
                &ctx.slub_allocator,
                &ctx.page_table_caches,
                &ctx.per_cpu_storage,
            ),
        );
        assertions.assert_ok(
            "setup ioremap",
            self.ioremap.setup(
                &ctx.vm,
                &self.vmalloc_allocator,
                &ctx.page_table_caches,
                &ctx.fix_map,
                &ctx.config,
            ),
        );
        assertions.assert_ok(
            "setup platform bus",
            self.bus
                .setup(&ctx.driver_core_base, &ctx.platform_bus_root_device),
        );
        assertions.assert("bus ready", self.bus.state() == State::Ready);
        assertions.assert("bus registered", self.bus.registered());
        assertions.assert("devices kset ready", self.bus.devices_kset_ready());
        assertions.assert("drivers kset ready", self.bus.drivers_kset_ready());
    }

    fn add_smoke_device(&mut self, assertions: &mut SmokeAssertions) -> Option<DeviceRef> {
        let ctx = context_ref();
        let Some(root) = ctx.device_tree.root() else {
            assertions.assert("device tree root available", false);
            return None;
        };
        let result = self
            .bus
            .add_smoke_platform_device(&ctx.device_tree, root.id());
        assertions.assert("add smoke platform device", result.is_ok());
        result.ok()
    }
}

struct AddDeviceProbeDriverScenario {
    fixture: PlatformBusActionsFixture,
}

impl AddDeviceProbeDriverScenario {
    fn new() -> Self {
        Self {
            fixture: PlatformBusActionsFixture::new(),
        }
    }
}

impl SmokeScenario for AddDeviceProbeDriverScenario {
    fn name(&self) -> &'static str {
        "platform_bus_actions.add_device_probe_driver"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "add device before setup",
            self.fixture.bus.add_device(DeviceRef::new(usize::MAX)),
        );
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let Some(device_ref) = self.fixture.add_smoke_device(assertions) else {
            return;
        };
        assertions.assert(
            "device listed",
            self.fixture.bus.contains_device(device_ref),
        );
        assertions.assert("device count", self.fixture.bus.device_count() == 1);
        assertions.assert(
            "platform device count",
            self.fixture.bus.platform_device_count() == 1,
        );
        assertions.assert(
            "klist device count",
            self.fixture.bus.klist_device_count() == 1,
        );
        assertions.assert_fail("duplicate device", self.fixture.bus.add_device(device_ref));

        assertions.assert_ok(
            "add deferred driver",
            self.fixture.bus.add_driver(MOCK_PLATFORM_DRIVER_REF),
        );
        assertions.assert_ok(
            "probe driver deferred",
            self.fixture.bus.probe_driver(
                MOCK_PLATFORM_DRIVER_REF,
                &context_ref().device_tree,
                &mut self.fixture.vmalloc_allocator,
                &mut self.fixture.ioremap,
            ),
        );
        assertions.assert(
            "probe driver scanned devices",
            self.fixture.bus.probe_driver_scanned_devices(),
        );
        assertions.assert(
            "probe driver deferred count",
            self.fixture.bus.probe_driver_deferred_count() == 1,
        );
        assertions.assert(
            "driver list populated",
            self.fixture.bus.driver_count() == 1,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct AddDriverProbeDeviceScenario {
    fixture: PlatformBusActionsFixture,
}

impl AddDriverProbeDeviceScenario {
    fn new() -> Self {
        Self {
            fixture: PlatformBusActionsFixture::new(),
        }
    }
}

impl SmokeScenario for AddDriverProbeDeviceScenario {
    fn name(&self) -> &'static str {
        "platform_bus_actions.add_driver_probe_device"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "probe device before setup",
            self.fixture.bus.probe_device(
                DeviceRef::new(usize::MAX),
                &context_ref().device_tree,
                &mut self.fixture.vmalloc_allocator,
                &mut self.fixture.ioremap,
            ),
        );
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok(
            "add driver",
            self.fixture.bus.add_driver(MOCK_PLATFORM_DRIVER_REF),
        );
        assertions.assert(
            "driver listed",
            self.fixture.bus.contains_driver(MOCK_PLATFORM_DRIVER_REF),
        );
        assertions.assert("driver count", self.fixture.bus.driver_count() == 1);
        assertions.assert_fail(
            "duplicate driver",
            self.fixture.bus.add_driver(MOCK_PLATFORM_DRIVER_REF),
        );

        let Some(device_ref) = self.fixture.add_smoke_device(assertions) else {
            return;
        };
        assertions.assert_ok(
            "probe device deferred",
            self.fixture.bus.probe_device(
                device_ref,
                &context_ref().device_tree,
                &mut self.fixture.vmalloc_allocator,
                &mut self.fixture.ioremap,
            ),
        );
        assertions.assert(
            "probe device scanned drivers",
            self.fixture.bus.probe_device_scanned_drivers(),
        );
        assertions.assert(
            "probe device deferred count",
            self.fixture.bus.probe_device_deferred_count() == 1,
        );
        assertions.assert(
            "device list populated",
            self.fixture.bus.device_count() == 1,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct Ns16550aProbeDeviceScenario {
    fixture: PlatformBusActionsFixture,
}

impl Ns16550aProbeDeviceScenario {
    fn new() -> Self {
        Self {
            fixture: PlatformBusActionsFixture::new(),
        }
    }
}

impl SmokeScenario for Ns16550aProbeDeviceScenario {
    fn name(&self) -> &'static str {
        "platform_bus_actions.ns16550a_probe_device"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "add ns16550a driver",
            self.fixture.bus.add_driver(NS16550A_PLATFORM_DRIVER_REF),
        );

        let Some(serial) = find_ns16550a_node(&ctx.device_tree) else {
            assertions.assert("ns16550a node available", false);
            return;
        };
        let result = self
            .fixture
            .bus
            .add_smoke_platform_device(&ctx.device_tree, serial.id());
        assertions.assert("add ns16550a platform device", result.is_ok());
        let Some(device_ref) = result.ok() else {
            return;
        };

        assertions.assert_ok(
            "probe ns16550a device",
            self.fixture.bus.probe_device(
                device_ref,
                &ctx.device_tree,
                &mut self.fixture.vmalloc_allocator,
                &mut self.fixture.ioremap,
            ),
        );
        assertions.assert(
            "probe device scanned registered drivers",
            self.fixture.bus.probe_device_scanned_drivers(),
        );
        assertions.assert(
            "ns16550a device matched",
            self.fixture.bus.ns16550a_device_matched(),
        );
        assertions.assert(
            "ns16550a probe called",
            self.fixture.bus.ns16550a_probe_called(),
        );
        assertions.assert(
            "ns16550a probe return zero",
            self.fixture.bus.ns16550a_probe_return_zero(),
        );
        assertions.assert(
            "ns16550a device bound",
            self.fixture.bus.ns16550a_bound_device() == Some(device_ref),
        );
        assertions.assert(
            "ns16550a mmio ioremapped",
            self.fixture.bus.ns16550a_probe_ioremaps_uart8250_port()
                && ns16550a::uart8250_port_ioremapped(),
        );
        assertions.assert(
            "ns16550a vmalloc mapping executed",
            self.fixture.vmalloc_allocator.area_count() != 0
                && self.fixture.vmalloc_allocator.mapping_count() != 0,
        );
        if let Some(mapping) = self.fixture.ioremap.mapping_for_device(device_ref) {
            let area = mapping.vmap_area();
            let vmap_mapping = mapping.vmap_mapping();
            assertions.assert(
                "ns16550a ioremap area facts",
                area.busy()
                    && area.is_vm_ioremap()
                    && area.flags().is_vm_ioremap()
                    && area.end() == area.virt_base().saturating_add(area.size())
                    && self.fixture.vmalloc_allocator.area(area.index()) == Some(area),
            );
            assertions.assert(
                "ns16550a vmap mapping facts",
                vmap_mapping.installed()
                    && vmap_mapping.area() == area
                    && vmap_mapping.index() == 0
                    && vmap_mapping.phys_base() == mapping.page_phys_base()
                    && vmap_mapping.size() == mapping.mapped_size()
                    && vmap_mapping.protection().is_io_memory()
                    && self.fixture.vmalloc_allocator.mapping(vmap_mapping.index())
                        == Some(vmap_mapping),
            );
        } else {
            assertions.assert("ns16550a ioremap mapping available", false);
        }
        assertions.assert(
            "ns16550a uart8250 port registered",
            self.fixture.bus.ns16550a_probe_registers_uart8250_port()
                && ns16550a::uart8250_port_registered(),
        );
        assertions.assert(
            "ns16550a stdout-path matched",
            ns16550a::stdout_path_matched()
                && ns16550a::uart8250_port_device_ref() == Some(device_ref),
        );
        assertions.assert(
            "serial8250 console registered",
            self.fixture.bus.ns16550a_probe_registers_serial_console()
                && ns16550a::serial8250_console_registered(),
        );
        assertions.assert(
            "console handoff triggered",
            self.fixture.bus.ns16550a_probe_triggers_console_handoff()
                && ns16550a::handoff_triggered()
                && printk::console_handoff_complete()
                && printk::route() == printk::PrintkRoute::Serial8250,
        );
        assertions.assert(
            "boot console unregistered by default",
            !printk::keep_bootcon()
                && printk::boot_console_registered()
                && !printk::boot_console_online()
                && printk::boot_console_unregistered()
                && printk::boot_console_removed_from_registry(),
        );
        let area_count = self.fixture.vmalloc_allocator.area_count();
        let mapping_count = self.fixture.vmalloc_allocator.mapping_count();
        let registered_again = printk::register_serial8250_console(true);
        assertions.assert("serial console duplicate accepted", registered_again);
        assertions.assert(
            "serial console duplicate idempotent",
            self.fixture.vmalloc_allocator.area_count() == area_count
                && self.fixture.vmalloc_allocator.mapping_count() == mapping_count
                && printk::serial8250_console_registered()
                && printk::preferred_console_from_stdout()
                && printk::serial8250_consdev()
                && printk::serial8250_write_ready()
                && printk::console_handoff_complete()
                && printk::route() == printk::PrintkRoute::Serial8250,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
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

struct DummyConsoleSnapshot {
    registry: printk::ConsoleRegistry,
}

impl DummyConsoleSnapshot {
    fn take() -> Self {
        Self {
            registry: printk::registry_snapshot(),
        }
    }

    fn restore(self) {
        printk::restore_registry(self.registry);
    }
}

struct DummyConsole;

impl DummyConsole {
    fn register(self) -> bool {
        printk::register_serial8250_console(false)
    }
}

struct DummyConsoleNonStdoutScenario {
    snapshot: Option<DummyConsoleSnapshot>,
}

impl DummyConsoleNonStdoutScenario {
    fn new() -> Self {
        Self { snapshot: None }
    }
}

impl SmokeScenario for DummyConsoleNonStdoutScenario {
    fn name(&self) -> &'static str {
        "platform_bus_actions.dummy_console_non_stdout"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {
        self.snapshot = Some(DummyConsoleSnapshot::take());
        printk::reset_registry_for_smoke(false);
        printk::register_boot_console();
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let registered = DummyConsole.register();
        assertions.assert("dummy console rejected", !registered);
        assertions.assert(
            "dummy console not registered",
            !printk::serial8250_console_registered()
                && !printk::preferred_console_from_stdout()
                && !printk::serial8250_consdev()
                && !printk::serial8250_write_ready(),
        );
        assertions.assert(
            "dummy console keeps boot route",
            printk::boot_console_registered()
                && printk::boot_console_online()
                && !printk::boot_console_unregistered()
                && !printk::boot_console_removed_from_registry()
                && !printk::console_handoff_complete()
                && printk::route() == printk::PrintkRoute::BootConsole,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {
        if let Some(snapshot) = self.snapshot.take() {
            snapshot.restore();
        }
    }
}

struct KeepBootconScenario {
    snapshot: Option<ConsoleProbeSnapshot>,
    fixture: PlatformBusActionsFixture,
}

impl KeepBootconScenario {
    fn new() -> Self {
        Self {
            snapshot: None,
            fixture: PlatformBusActionsFixture::new(),
        }
    }
}

impl SmokeScenario for KeepBootconScenario {
    fn name(&self) -> &'static str {
        "platform_bus_actions.keep_bootcon"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.snapshot = Some(ConsoleProbeSnapshot::take());
        printk::reset_registry_for_smoke(true);
        printk::register_boot_console();
        ns16550a::reset_probe_state_for_smoke();
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "add ns16550a driver",
            self.fixture.bus.add_driver(NS16550A_PLATFORM_DRIVER_REF),
        );

        let Some(serial) = find_ns16550a_node(&ctx.device_tree) else {
            assertions.assert("ns16550a node available", false);
            return;
        };
        let result = self
            .fixture
            .bus
            .add_smoke_platform_device(&ctx.device_tree, serial.id());
        assertions.assert("add ns16550a platform device", result.is_ok());
        let Some(device_ref) = result.ok() else {
            return;
        };

        assertions.assert_ok(
            "probe ns16550a device",
            self.fixture.bus.probe_device(
                device_ref,
                &ctx.device_tree,
                &mut self.fixture.vmalloc_allocator,
                &mut self.fixture.ioremap,
            ),
        );
        assertions.assert(
            "keep_bootcon mmio ioremapped",
            self.fixture.bus.ns16550a_probe_ioremaps_uart8250_port()
                && ns16550a::uart8250_port_ioremapped(),
        );
        assertions.assert(
            "keep_bootcon vmalloc mapping executed",
            self.fixture.vmalloc_allocator.area_count() != 0
                && self.fixture.vmalloc_allocator.mapping_count() != 0,
        );
        assertions.assert(
            "keep_bootcon serial console registered",
            self.fixture.bus.ns16550a_probe_registers_serial_console()
                && ns16550a::serial8250_console_registered()
                && printk::serial8250_console_registered()
                && printk::preferred_console_from_stdout(),
        );
        assertions.assert(
            "keep_bootcon retains boot console",
            printk::keep_bootcon()
                && printk::boot_console_registered()
                && printk::boot_console_online()
                && !printk::boot_console_unregistered()
                && !printk::boot_console_removed_from_registry()
                && printk::console_handoff_complete(),
        );
        assertions.assert(
            "keep_bootcon route serial",
            ns16550a::handoff_triggered()
                && printk::serial8250_consdev()
                && printk::serial8250_write_ready()
                && printk::route() == printk::PrintkRoute::Serial8250,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {
        if let Some(snapshot) = self.snapshot.take() {
            snapshot.restore();
        }
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
