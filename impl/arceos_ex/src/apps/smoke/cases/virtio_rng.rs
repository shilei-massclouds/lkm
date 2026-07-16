use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        hwrng::HwRngCore,
        state::State,
        virtio::VirtioDevice,
        virtio_rng::{
            VirtioRngDevice, VirtioRngDriver, VirtioRngError, smoke_fixture as rng_smoke_fixture,
        },
    },
};

const ENTROPY_BUFFER_ADDR: usize = 0x8021_0000;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut ProbeScenario::new());
    suite.scenario(&mut RequestCompletionScenario::new(
        "virtio_rng.request_completion_32",
        32,
        16,
    ));
    suite.scenario(&mut RequestCompletionScenario::new(
        "virtio_rng.request_completion_64",
        64,
        48,
    ));
    suite.scenario(&mut RepeatRequestScenario::new());
    suite.scenario(&mut InvalidCompletionScenario::new());
    suite.scenario(&mut RemovedRejectsIoScenario::new());
    suite.scenario(&mut HwrngReadScenario::new());
    suite.result()
}

fn rng_virtio_device() -> Option<VirtioDevice> {
    context().virtio_bus.rng_device()
}

struct VirtioRngFixture {
    driver: VirtioRngDriver,
    device: Option<VirtioRngDevice>,
}

impl VirtioRngFixture {
    const fn new() -> Self {
        Self {
            driver: VirtioRngDriver::new(),
            device: None,
        }
    }

    fn setup_driver(&mut self, assertions: &mut SmokeAssertions) -> Option<VirtioDevice> {
        let ctx = context();
        assertions.assert_ok("driver setup", self.driver.setup(&ctx.virtio_bus));
        assertions.assert("driver ready", self.driver.state() == State::Ready);
        assertions.assert("driver name", self.driver.name_bound());
        assertions.assert("driver id table", self.driver.id_table_contains_rng());
        assertions.assert("driver scan callback", self.driver.scan_callback_bound());

        let Some(device) = rng_virtio_device() else {
            assertions.assert("rng virtio device", false);
            return None;
        };
        assertions.assert("matches rng", self.driver.matches(device));
        Some(device)
    }

    fn setup_probed(&mut self, assertions: &mut SmokeAssertions) {
        let Some(device) = self.setup_driver(assertions) else {
            return;
        };
        match self.driver.probe(device) {
            Ok(device) => {
                self.device = Some(device);
            }
            Err(_) => {
                assertions.assert("probe", false);
                return;
            }
        }
        let Some(rng) = self.device.as_ref() else {
            assertions.assert("probed device", false);
            return;
        };
        assertions.assert("probe called", self.driver.probe_called());
        assertions.assert("probe returned", self.driver.probe_return_zero());
        assertions.assert("matched device", self.driver.matched_device());
        assertions.assert("rng ready", rng.state() == State::Ready);
        assertions.assert(
            "bound device ref",
            rng.virtio_device().device_ref() == device.device_ref(),
        );
        assertions.assert(
            "bound device id",
            rng.virtio_device().device_id() == device.device_id(),
        );
        assertions.assert("single queue", rng.single_input_queue());
        assertions.assert("queue ready", rng.queue().state() == State::Ready);
        assertions.assert("hwrng embedded", rng.hwrng_embedded());
        assertions.assert("hwrng ready", rng.hwrng().state() == State::Ready);
        assertions.assert("hwrng name", rng.hwrng().name_bound());
        assertions.assert("hwrng read callback", rng.hwrng().read_callback_bound());
        assertions.assert(
            "hwrng cleanup callback",
            rng.hwrng().cleanup_callback_bound(),
        );
        assertions.assert("hwrng priv", rng.hwrng().priv_points_to_provider());
        assertions.assert("have data ready", rng.have_data_completion_ready());
        assertions.assert("have data online", rng.have_data().state() == State::Online);
        assertions.assert("random pool deferred", rng.random_pool_deferred());
        assertions.assert(
            "dev hwrng plumbing deferred",
            rng.dev_hwrng_plumbing_deferred(),
        );
        assertions.assert("blocking wait deferred", rng.blocking_wait_deferred());
        assertions.assert("notify irq deferred", rng.real_notify_irq_deferred());
        assertions.assert("config access", rng.virtio_device().config_access_ready());
        assertions.assert(
            "config deferred",
            rng.virtio_device().config_read_deferred(),
        );
        assertions.assert(
            "multi queue deferred",
            rng.virtio_device().multi_queue_deferred(),
        );
        assertions.assert(
            "reset/remove deferred",
            rng.virtio_device().reset_remove_deferred(),
        );
    }

    fn rng_mut(&mut self, assertions: &mut SmokeAssertions) -> Option<&mut VirtioRngDevice> {
        let Some(rng) = self.device.as_mut() else {
            assertions.assert("rng device present", false);
            return None;
        };
        Some(rng)
    }

    fn prepare_completion(rng: &mut VirtioRngDevice, len: u32) -> Result<(), VirtioRngError> {
        rng_smoke_fixture::prepare_completion(rng, len)
    }
}

struct ProbeScenario {
    fixture: VirtioRngFixture,
}

impl ProbeScenario {
    const fn new() -> Self {
        Self {
            fixture: VirtioRngFixture::new(),
        }
    }
}

impl SmokeScenario for ProbeScenario {
    fn name(&self) -> &'static str {
        "virtio_rng.probe"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_probed(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let Some(rng) = self.fixture.device.as_ref() else {
            assertions.assert("rng device", false);
            return;
        };
        assertions.assert("no pending request", !rng.request_pending());
        assertions.assert("data empty", rng.data_avail() == 0 && rng.data_idx() == 0);
        assertions.assert("requests zero", rng.request_count() == 0);
        assertions.assert("completions zero", rng.completion_count() == 0);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RequestCompletionScenario {
    name: &'static str,
    fixture: VirtioRngFixture,
    request_len: u32,
    completion_len: u32,
}

impl RequestCompletionScenario {
    const fn new(name: &'static str, request_len: u32, completion_len: u32) -> Self {
        Self {
            name,
            fixture: VirtioRngFixture::new(),
            request_len,
            completion_len,
        }
    }
}

impl SmokeScenario for RequestCompletionScenario {
    fn name(&self) -> &'static str {
        self.name
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_probed(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let Some(rng) = self.fixture.rng_mut(assertions) else {
            return;
        };
        assertions.assert_ok(
            "request entropy",
            rng.request_entropy(ENTROPY_BUFFER_ADDR, self.request_len),
        );
        assertions.assert("request pending", rng.request_pending());
        assertions.assert("request inbuf", rng.request_submits_inbuf());
        assertions.assert("request kick", rng.request_kicks_queue());
        assertions.assert("request count", rng.request_count() == 1);
        assertions.assert("queue submitted", rng.queue().submitted_count() == 1);
        assertions.assert("queue kicked", rng.queue().kick_count() == 1);

        assertions.assert_ok(
            "fixture completion",
            VirtioRngFixture::prepare_completion(rng, self.completion_len),
        );

        let len = match rng.complete_entropy() {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("complete entropy", false);
                return;
            }
        };
        assertions.assert("len returned", len == self.completion_len);
        assertions.assert("request cleared", !rng.request_pending());
        assertions.assert("get used", rng.complete_gets_used_buffer());
        assertions.assert("data avail", rng.data_avail() == self.completion_len);
        assertions.assert("data avail updated", rng.data_avail_updated());
        assertions.assert(
            "data idx reset",
            rng.data_idx() == 0 && rng.data_idx_reset(),
        );
        assertions.assert("completion count", rng.completion_count() == 1);
        assertions.assert("queue get", rng.queue().get_buf_count() == 1);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RepeatRequestScenario {
    fixture: VirtioRngFixture,
}

impl RepeatRequestScenario {
    const fn new() -> Self {
        Self {
            fixture: VirtioRngFixture::new(),
        }
    }
}

impl SmokeScenario for RepeatRequestScenario {
    fn name(&self) -> &'static str {
        "virtio_rng.repeat_request"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_probed(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let Some(rng) = self.fixture.rng_mut(assertions) else {
            return;
        };
        assertions.assert_ok(
            "first request",
            rng.request_entropy(ENTROPY_BUFFER_ADDR, 32),
        );
        assertions.assert(
            "repeat rejected",
            rng.request_entropy(ENTROPY_BUFFER_ADDR + 64, 32)
                == Err(VirtioRngError::RequestPending),
        );
        assertions.assert("repeat recorded", rng.repeat_request_rejected());
        assertions.assert("request still pending", rng.request_pending());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct InvalidCompletionScenario {
    fixture: VirtioRngFixture,
}

impl InvalidCompletionScenario {
    const fn new() -> Self {
        Self {
            fixture: VirtioRngFixture::new(),
        }
    }
}

impl SmokeScenario for InvalidCompletionScenario {
    fn name(&self) -> &'static str {
        "virtio_rng.invalid_completion"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_probed(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let Some(rng) = self.fixture.rng_mut(assertions) else {
            return;
        };
        assertions.assert(
            "complete without request",
            rng.complete_entropy() == Err(VirtioRngError::NoRequestPending),
        );
        assertions.assert(
            "fixture without request",
            VirtioRngFixture::prepare_completion(rng, 1) == Err(VirtioRngError::NoRequestPending),
        );
        assertions.assert(
            "zero buffer",
            rng.request_entropy(0, 32) == Err(VirtioRngError::InvalidBuffer),
        );
        assertions.assert(
            "zero len request",
            rng.request_entropy(ENTROPY_BUFFER_ADDR, 0) == Err(VirtioRngError::InvalidBuffer),
        );
        assertions.assert_ok("request", rng.request_entropy(ENTROPY_BUFFER_ADDR, 32));
        assertions.assert_ok(
            "fixture zero completion",
            VirtioRngFixture::prepare_completion(rng, 0),
        );
        assertions.assert(
            "zero completion",
            rng.complete_entropy() == Err(VirtioRngError::ZeroLengthCompletion),
        );
        assertions.assert("zero recorded", rng.zero_len_completion_rejected());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct RemovedRejectsIoScenario {
    fixture: VirtioRngFixture,
}

impl RemovedRejectsIoScenario {
    const fn new() -> Self {
        Self {
            fixture: VirtioRngFixture::new(),
        }
    }
}

impl SmokeScenario for RemovedRejectsIoScenario {
    fn name(&self) -> &'static str {
        "virtio_rng.removed_rejects_io"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_probed(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let Some(rng) = self.fixture.rng_mut(assertions) else {
            return;
        };
        assertions.assert_ok("cleanup", rng.cleanup());
        assertions.assert("destroyed", rng.state() == State::Destroyed);
        assertions.assert("removed recorded", rng.removed_rejects_io());
        assertions.assert(
            "request rejected",
            rng.request_entropy(ENTROPY_BUFFER_ADDR, 32) == Err(VirtioRngError::Removed),
        );
        assertions.assert(
            "complete rejected",
            rng.complete_entropy() == Err(VirtioRngError::Removed),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct HwrngReadScenario {
    fixture: VirtioRngFixture,
    hwrng_core: HwRngCore,
}

impl HwrngReadScenario {
    const fn new() -> Self {
        Self {
            fixture: VirtioRngFixture::new(),
            hwrng_core: HwRngCore::new(),
        }
    }
}

impl SmokeScenario for HwrngReadScenario {
    fn name(&self) -> &'static str {
        "virtio_rng.hwrng_read_current"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_probed(assertions);
        let ctx = context();
        assertions.assert_ok(
            "hwrng core setup",
            self.hwrng_core.setup(&ctx.driver_core_base),
        );
        assertions.assert("hwrng core ready", self.hwrng_core.state() == State::Ready);
        assertions.assert("hwrng registry ready", self.hwrng_core.registry_ready());
        assertions.assert("hwrng current slot", self.hwrng_core.current_slot_ready());
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let VirtioRngFixture { driver, device } = &mut self.fixture;
        let Some(rng) = device.as_mut() else {
            assertions.assert("rng device present", false);
            return;
        };
        assertions.assert_ok(
            "request entropy",
            rng.request_entropy(ENTROPY_BUFFER_ADDR, 64),
        );
        assertions.assert_ok(
            "fixture completion",
            VirtioRngFixture::prepare_completion(rng, 40),
        );
        let len = match rng.complete_entropy() {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("complete entropy", false);
                return;
            }
        };
        assertions.assert("completion len", len == 40);

        match driver.scan(rng, &mut self.hwrng_core) {
            Ok(device_ref) => {
                assertions.assert("scan called", driver.scan_called());
                assertions.assert("scan registered", driver.scan_registered_hwrng());
                assertions.assert("hwrng register done", rng.hwrng_register_done());
                assertions.assert("hwrng registered", rng.hwrng().registered());
                assertions.assert("hwrng current", rng.hwrng().current());
                assertions.assert(
                    "current ref",
                    self.hwrng_core.current_device() == Some(device_ref),
                );
            }
            Err(_) => {
                assertions.assert("scan hwrng", false);
                return;
            }
        }

        let mut out = [0u8; 32];
        let read_len = match self.hwrng_core.read_current(rng, &mut out, false) {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("hwrng read current", false);
                return;
            }
        };
        assertions.assert("read len", read_len == out.len());
        assertions.assert("hwrng core read", self.hwrng_core.read_current_count() == 1);
        assertions.assert(
            "hwrng ref acquired",
            self.hwrng_core.current_rng_ref_acquired(),
        );
        assertions.assert(
            "hwrng read invoked",
            self.hwrng_core.current_rng_read_invoked(),
        );
        assertions.assert("hwrng copied", self.hwrng_core.read_copies_from_current());
        assertions.assert("hwrng nonblocking", self.hwrng_core.read_nonblocking());
        assertions.assert(
            "hwrng last len",
            self.hwrng_core.last_read_len() == out.len(),
        );
        assertions.assert("rng read count", rng.read_count() == 1);
        assertions.assert("rng last read", rng.last_read_len() == out.len());
        assertions.assert("rng consumed", rng.read_consumes_available_data());
        assertions.assert("rng idx update", rng.read_updates_data_idx());
        assertions.assert("rng avail update", rng.read_updates_data_avail());
        assertions.assert("remaining data", rng.data_avail() == 0);
        assertions.assert("data idx reset for next request", rng.data_idx() == 0);
        assertions.assert(
            "not empty before low-watermark refill",
            !rng.read_requeues_when_empty(),
        );
        assertions.assert(
            "requeued below request watermark",
            rng.read_requeues_below_request_watermark(),
        );
        assertions.assert("next request pending", rng.request_pending());
        assertions.assert("request count", rng.request_count() == 2);
        assertions.assert("nonzero bytes", out.iter().any(|byte| *byte != 0));
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
