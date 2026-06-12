use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        device::DeviceRef,
        driver::MOCK_PLATFORM_DRIVER_REF,
        initcall::PlatformBus,
        ioremap::Ioremap,
        irq_time::{IrqHandlerRegistry, PlicIrqDomain},
        mm_core::{PageAllocator, PageTableCaches, VmallocAllocator},
        state::State,
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
    page_table_caches: PageTableCaches,
    page_allocator: PageAllocator,
    vmalloc_allocator: VmallocAllocator,
    ioremap: Ioremap,
    plic_irq_domain: PlicIrqDomain,
    irq_handler_registry: IrqHandlerRegistry,
}

impl PlatformBusActionsFixture {
    fn new() -> Self {
        Self {
            bus: PlatformBus::new(),
            page_table_caches: PageTableCaches::new(),
            page_allocator: PageAllocator::new(),
            vmalloc_allocator: VmallocAllocator::new(),
            ioremap: Ioremap::new(),
            plic_irq_domain: PlicIrqDomain::new(),
            irq_handler_registry: IrqHandlerRegistry::new(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        assertions.assert_ok(
            "setup page table caches",
            self.page_table_caches.setup(
                &ctx.slub_allocator,
                &ctx.page_allocator,
                &ctx.page_metadata_map,
                &ctx.config,
                &ctx.vm,
                &ctx.static_objects,
                &ctx.kernel_image,
            ),
        );
        assertions.assert_ok(
            "setup vmalloc allocator",
            self.vmalloc_allocator.setup(
                &ctx.slub_allocator,
                &self.page_table_caches,
                &ctx.per_cpu_storage,
            ),
        );
        assertions.assert_ok(
            "setup ioremap",
            self.ioremap.setup(
                &ctx.vm,
                &self.vmalloc_allocator,
                &self.page_table_caches,
                &ctx.fix_map,
                &ctx.config,
            ),
        );
        assertions.assert_ok(
            "setup platform bus",
            self.bus
                .setup(&ctx.driver_core_base, &ctx.platform_bus_root_device),
        );
        assertions.assert_ok(
            "preset plic irq domain",
            self.plic_irq_domain.preset(&ctx.plic, &ctx.irq_controller),
        );
        assertions.assert_ok(
            "setup plic irq domain",
            self.plic_irq_domain.setup(&ctx.plic, &ctx.irq_controller),
        );
        assertions.assert_ok(
            "setup irq handler registry",
            self.irq_handler_registry
                .setup(&ctx.irq_controller, &self.plic_irq_domain),
        );
        assertions.assert("bus ready", self.bus.state() == State::Ready);
        assertions.assert(
            "plic irq domain ready",
            self.plic_irq_domain.state() == State::Ready,
        );
        assertions.assert(
            "irq handler registry ready",
            self.irq_handler_registry.state() == State::Ready,
        );
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
                &mut self.fixture.page_table_caches,
                &mut self.fixture.page_allocator,
                &context_ref().page_metadata_map,
                &context_ref().config,
                &mut self.fixture.ioremap,
                &mut self.fixture.plic_irq_domain,
                &mut self.fixture.irq_handler_registry,
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
                &mut self.fixture.page_table_caches,
                &mut self.fixture.page_allocator,
                &context_ref().page_metadata_map,
                &context_ref().config,
                &mut self.fixture.ioremap,
                &mut self.fixture.plic_irq_domain,
                &mut self.fixture.irq_handler_registry,
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
                &mut self.fixture.page_table_caches,
                &mut self.fixture.page_allocator,
                &context_ref().page_metadata_map,
                &context_ref().config,
                &mut self.fixture.ioremap,
                &mut self.fixture.plic_irq_domain,
                &mut self.fixture.irq_handler_registry,
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
