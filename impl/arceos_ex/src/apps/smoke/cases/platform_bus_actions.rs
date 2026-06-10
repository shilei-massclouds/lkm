use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        initcall::{BusDeviceRef, BusDriverRef, PlatformBus},
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
            self.fixture
                .bus
                .add_device(BusDeviceRef::MockPlatformDevice),
        );
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok(
            "add device",
            self.fixture
                .bus
                .add_device(BusDeviceRef::MockPlatformDevice),
        );
        assertions.assert(
            "device listed",
            self.fixture
                .bus
                .contains_device(BusDeviceRef::MockPlatformDevice),
        );
        assertions.assert("device count", self.fixture.bus.device_count() == 1);
        assertions.assert_fail(
            "duplicate device",
            self.fixture
                .bus
                .add_device(BusDeviceRef::MockPlatformDevice),
        );

        assertions.assert_ok(
            "probe driver deferred",
            self.fixture
                .bus
                .probe_driver(BusDriverRef::MockPlatformDriver),
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
            "driver list untouched",
            self.fixture.bus.driver_count() == 0,
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
            self.fixture
                .bus
                .probe_device(BusDeviceRef::MockPlatformDevice),
        );
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok(
            "add driver",
            self.fixture
                .bus
                .add_driver(BusDriverRef::MockPlatformDriver),
        );
        assertions.assert(
            "driver listed",
            self.fixture
                .bus
                .contains_driver(BusDriverRef::MockPlatformDriver),
        );
        assertions.assert("driver count", self.fixture.bus.driver_count() == 1);
        assertions.assert_fail(
            "duplicate driver",
            self.fixture
                .bus
                .add_driver(BusDriverRef::MockPlatformDriver),
        );

        assertions.assert_ok(
            "probe device deferred",
            self.fixture
                .bus
                .probe_device(BusDeviceRef::MockPlatformDevice),
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
            "device list untouched",
            self.fixture.bus.device_count() == 0,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
