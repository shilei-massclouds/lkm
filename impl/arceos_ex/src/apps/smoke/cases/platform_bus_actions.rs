use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        device::DeviceRef,
        driver::{PlatformProbeContext, MOCK_PLATFORM_DRIVER_REF},
        initcall::PlatformBus,
        ioremap::Ioremap,
        irq_time::{IrqHandlerRegistry, PlicIrqDomain},
        mm_core::{PageAllocator, PageTableCaches, VmallocAllocator},
        state::State,
        virtio::VirtioBus,
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut AddDeviceProbeDriverScenario::new());
    suite.scenario(&mut AddDriverProbeDeviceScenario::new());
    suite.result()
}

struct PlatformBusActionsFixture {
    bus: PlatformBus,
}

impl PlatformBusActionsFixture {
    fn new() -> Self {
        Self {
            bus: PlatformBus::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
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

    fn add_platform_device(&mut self, assertions: &mut SmokeAssertions) -> Option<DeviceRef> {
        let ctx = context_ref();
        let Some(root) = ctx.device_tree.root() else {
            assertions.assert("device tree root available", false);
            return None;
        };
        let result = self
            .bus
            .add_platform_device_from_node(&ctx.device_tree, root.id());
        assertions.assert("add platform device from node", result.is_ok());
        result.ok()
    }
}

struct ProbeSupport {
    page_table_caches: PageTableCaches,
    page_allocator: PageAllocator,
    vmalloc_allocator: VmallocAllocator,
    ioremap: Ioremap,
    plic_irq_domain: PlicIrqDomain,
    irq_handler_registry: IrqHandlerRegistry,
    virtio_bus: VirtioBus,
}

impl ProbeSupport {
    const fn new() -> Self {
        Self {
            page_table_caches: PageTableCaches::new(),
            page_allocator: PageAllocator::new(),
            vmalloc_allocator: VmallocAllocator::new(),
            ioremap: Ioremap::new(),
            plic_irq_domain: PlicIrqDomain::new(),
            irq_handler_registry: IrqHandlerRegistry::new(),
            virtio_bus: VirtioBus::new(),
        }
    }

    fn resources<'a>(&'a mut self, ctx: &'a crate::context::Context) -> PlatformProbeContext<'a> {
        PlatformProbeContext::new(
            &ctx.device_tree,
            &mut self.vmalloc_allocator,
            &mut self.page_table_caches,
            &mut self.page_allocator,
            &ctx.page_metadata_map,
            &ctx.config,
            &mut self.ioremap,
            &mut self.plic_irq_domain,
            &mut self.irq_handler_registry,
            &mut self.virtio_bus,
        )
    }
}

static mut PROBE_SUPPORT: ProbeSupport = ProbeSupport::new();

fn probe_support() -> &'static mut ProbeSupport {
    // The probe support objects are larger than the boot stack.  Keep the
    // smoke-only fixture in static storage; the mock driver used here never
    // matches, so the mutable resources are not consumed by a real probe.
    unsafe { &mut *core::ptr::addr_of_mut!(PROBE_SUPPORT) }
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
        let Some(device_ref) = self.fixture.add_platform_device(assertions) else {
            return;
        };
        assertions.assert(
            "device listed",
            self.fixture.bus.contains_device(device_ref),
        );
        assertions.assert(
            "platform device discovered",
            self.fixture.bus.platform_device_discovered(device_ref),
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
        assertions.assert(
            "platform driver registered",
            self.fixture
                .bus
                .platform_driver_registered(MOCK_PLATFORM_DRIVER_REF),
        );
        assertions.assert_ok("probe driver deferred", {
            let ctx = context_ref();
            let support = probe_support();
            let mut resources = support.resources(ctx);
            self.fixture
                .bus
                .probe_driver(MOCK_PLATFORM_DRIVER_REF, &mut resources)
        });
        assertions.assert(
            "probe driver scanned devices",
            self.fixture.bus.probe_driver_scanned_devices(),
        );
        assertions.assert(
            "platform match attempted",
            self.fixture
                .bus
                .platform_match_attempted(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform device not matched",
            !self
                .fixture
                .bus
                .platform_driver_matched_device(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform probe not called",
            !self
                .fixture
                .bus
                .platform_probe_called(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform probe did not return zero",
            !self
                .fixture
                .bus
                .platform_probe_return_zero(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform device not bound",
            !self
                .fixture
                .bus
                .platform_device_bound(MOCK_PLATFORM_DRIVER_REF, device_ref),
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
        assertions.assert_fail("probe device before setup", {
            let ctx = context_ref();
            let support = probe_support();
            let mut resources = support.resources(ctx);
            self.fixture
                .bus
                .probe_device(DeviceRef::new(usize::MAX), &mut resources)
        });
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
        assertions.assert(
            "platform driver registered",
            self.fixture
                .bus
                .platform_driver_registered(MOCK_PLATFORM_DRIVER_REF),
        );
        assertions.assert("driver count", self.fixture.bus.driver_count() == 1);
        assertions.assert_fail(
            "duplicate driver",
            self.fixture.bus.add_driver(MOCK_PLATFORM_DRIVER_REF),
        );

        let Some(device_ref) = self.fixture.add_platform_device(assertions) else {
            return;
        };
        assertions.assert(
            "platform device discovered",
            self.fixture.bus.platform_device_discovered(device_ref),
        );
        assertions.assert_ok("probe device deferred", {
            let ctx = context_ref();
            let support = probe_support();
            let mut resources = support.resources(ctx);
            self.fixture.bus.probe_device(device_ref, &mut resources)
        });
        assertions.assert(
            "probe device scanned drivers",
            self.fixture.bus.probe_device_scanned_drivers(),
        );
        assertions.assert(
            "platform match attempted",
            self.fixture
                .bus
                .platform_match_attempted(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform device not matched",
            !self
                .fixture
                .bus
                .platform_driver_matched_device(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform probe not called",
            !self
                .fixture
                .bus
                .platform_probe_called(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform probe did not return zero",
            !self
                .fixture
                .bus
                .platform_probe_return_zero(MOCK_PLATFORM_DRIVER_REF, device_ref),
        );
        assertions.assert(
            "platform device not bound",
            !self
                .fixture
                .bus
                .platform_device_bound(MOCK_PLATFORM_DRIVER_REF, device_ref),
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
