use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    objects::{
        state::State,
        virtio_ring::{VirtQueue, VirtqueueError},
    },
};

const BUFFER_ADDR: usize = 0x8020_0000;
const BUFFER_LEN: u32 = 64;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SplitRingLifecycleScenario::new());
    suite.scenario(&mut SingleInbufCompletionScenario::new());
    suite.scenario(&mut DescriptorExhaustionScenario::new());
    suite.scenario(&mut EmptyCompletionScenario::new());
    suite.scenario(&mut InvalidCompletionScenario::new());
    suite.result()
}

struct VirtQueueFixture {
    queue: VirtQueue,
}

impl VirtQueueFixture {
    fn new(queue_size: u16) -> Self {
        Self {
            queue: VirtQueue::new(queue_size),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("setup", self.queue.setup());
        assertions.assert("queue ready", self.queue.state() == State::Ready);
        assertions.assert("ring ready", self.queue.ring().state() == State::Ready);
        assertions.assert(
            "desc table ready",
            self.queue.ring().descriptor_table_ready(),
        );
        assertions.assert("avail ready", self.queue.ring().avail_ring_ready());
        assertions.assert("used ready", self.queue.ring().used_ring_ready());
        assertions.assert(
            "free count",
            self.queue.ring().free_count() == self.queue.ring().queue_size(),
        );
        assertions.assert("free list ready", self.queue.ring().free_list_ready());
        assertions.assert("single queue", self.queue.ring().single_queue());
        assertions.assert("direct desc", self.queue.ring().direct_descriptors_only());
        assertions.assert(
            "indirect deferred",
            self.queue.ring().indirect_descriptors_deferred(),
        );
        assertions.assert("event idx deferred", self.queue.ring().event_idx_deferred());
        assertions.assert("dma deferred", self.queue.ring().dma_cache_deferred());
    }
}

struct SplitRingLifecycleScenario {
    fixture: VirtQueueFixture,
}

impl SplitRingLifecycleScenario {
    fn new() -> Self {
        Self {
            fixture: VirtQueueFixture::new(4),
        }
    }
}

impl SmokeScenario for SplitRingLifecycleScenario {
    fn name(&self) -> &'static str {
        "virtio_ring.split_ring_lifecycle"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("avail idx zero", self.fixture.queue.ring().avail_idx() == 0);
        assertions.assert("used idx zero", self.fixture.queue.ring().used_idx() == 0);
        assertions.assert(
            "last used idx zero",
            self.fixture.queue.ring().last_used_idx() == 0,
        );
        assertions.assert_fail("duplicate setup", self.fixture.queue.setup());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct SingleInbufCompletionScenario {
    fixture: VirtQueueFixture,
}

impl SingleInbufCompletionScenario {
    fn new() -> Self {
        Self {
            fixture: VirtQueueFixture::new(4),
        }
    }
}

impl SmokeScenario for SingleInbufCompletionScenario {
    fn name(&self) -> &'static str {
        "virtio_ring.single_inbuf_completion"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let token = match self.fixture.queue.add_inbuf(BUFFER_ADDR, BUFFER_LEN) {
            Ok(token) => token,
            Err(_) => {
                assertions.assert("add inbuf", false);
                return;
            }
        };
        assertions.assert("input added", self.fixture.queue.input_buffer_added());
        assertions.assert(
            "free descriptor consumed",
            self.fixture.queue.free_descriptor_consumed(),
        );
        assertions.assert("avail advanced", self.fixture.queue.avail_index_advanced());
        assertions.assert("submitted", self.fixture.queue.submitted_count() == 1);
        assertions.assert(
            "free count dec",
            self.fixture.queue.ring().free_count() == 3,
        );
        assertions.assert("avail idx", self.fixture.queue.ring().avail_idx() == 1);
        assertions.assert(
            "avail entry",
            self.fixture.queue.ring().avail_entry(0) == Some(token.head()),
        );
        let Some(descriptor) = self.fixture.queue.ring().descriptor(token) else {
            assertions.assert("descriptor exists", false);
            return;
        };
        assertions.assert("descriptor addr", descriptor.addr() == BUFFER_ADDR);
        assertions.assert("descriptor len", descriptor.len() == BUFFER_LEN);
        assertions.assert("descriptor active", descriptor.active());
        assertions.assert("descriptor not completed", !descriptor.completed());
        assertions.assert("descriptor writable", descriptor.writable());
        assertions.assert("descriptor direct", descriptor.direct());

        assertions.assert_ok("kick", self.fixture.queue.kick());
        assertions.assert("kick recorded", self.fixture.queue.kick_recorded());
        assertions.assert("kick count", self.fixture.queue.kick_count() == 1);

        assertions.assert_ok(
            "fake complete",
            self.fixture.queue.fake_complete_used(token, 32),
        );
        assertions.assert(
            "fake completion",
            self.fixture.queue.fake_completion_recorded(),
        );
        assertions.assert("used advanced", self.fixture.queue.used_index_advanced());
        assertions.assert(
            "completion count",
            self.fixture.queue.completion_count() == 1,
        );
        assertions.assert("used idx", self.fixture.queue.ring().used_idx() == 1);
        assertions.assert(
            "used entry id",
            self.fixture
                .queue
                .ring()
                .used_entry(0)
                .is_some_and(|entry| entry.id() == token.head() && entry.len() == 32),
        );

        let used = match self.fixture.queue.get_buf() {
            Ok(used) => used,
            Err(_) => {
                assertions.assert("get buf", false);
                return;
            }
        };
        assertions.assert("used token", used.token() == token);
        assertions.assert("used addr", used.addr() == BUFFER_ADDR);
        assertions.assert("used len", used.len() == 32);
        assertions.assert(
            "descriptor len carried",
            used.descriptor_len() == BUFFER_LEN,
        );
        assertions.assert(
            "get buf returns len",
            self.fixture.queue.get_buf_returns_len(),
        );
        assertions.assert(
            "ownership released",
            self.fixture.queue.buffer_ownership_released()
                && self.fixture.queue.ring().buffer_ownership_released(),
        );
        assertions.assert("get count", self.fixture.queue.get_buf_count() == 1);
        assertions.assert("free restored", self.fixture.queue.ring().free_count() == 4);
        assertions.assert(
            "last used idx",
            self.fixture.queue.ring().last_used_idx() == 1,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct DescriptorExhaustionScenario {
    fixture: VirtQueueFixture,
}

impl DescriptorExhaustionScenario {
    fn new() -> Self {
        Self {
            fixture: VirtQueueFixture::new(2),
        }
    }
}

impl SmokeScenario for DescriptorExhaustionScenario {
    fn name(&self) -> &'static str {
        "virtio_ring.descriptor_exhaustion"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("add first", self.fixture.queue.add_inbuf(BUFFER_ADDR, 16));
        assertions.assert_ok(
            "add second",
            self.fixture.queue.add_inbuf(BUFFER_ADDR + 64, 16),
        );
        assertions.assert("free empty", self.fixture.queue.ring().free_count() == 0);
        assertions.assert(
            "exhausted",
            self.fixture.queue.add_inbuf(BUFFER_ADDR + 128, 16)
                == Err(VirtqueueError::NoFreeDescriptor),
        );
        assertions.assert(
            "exhaustion recorded",
            self.fixture.queue.ring().descriptor_exhaustion_rejected(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct EmptyCompletionScenario {
    fixture: VirtQueueFixture,
}

impl EmptyCompletionScenario {
    fn new() -> Self {
        Self {
            fixture: VirtQueueFixture::new(4),
        }
    }
}

impl SmokeScenario for EmptyCompletionScenario {
    fn name(&self) -> &'static str {
        "virtio_ring.empty_completion"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "empty get",
            matches!(
                self.fixture.queue.get_buf(),
                Err(VirtqueueError::NoUsedBuffer)
            ),
        );
        assertions.assert(
            "empty recorded",
            self.fixture.queue.get_buf_empty_rejected(),
        );
        assertions.assert(
            "empty ring recorded",
            self.fixture.queue.ring().empty_get_rejected(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct InvalidCompletionScenario {
    fixture: VirtQueueFixture,
}

impl InvalidCompletionScenario {
    fn new() -> Self {
        Self {
            fixture: VirtQueueFixture::new(4),
        }
    }
}

impl SmokeScenario for InvalidCompletionScenario {
    fn name(&self) -> &'static str {
        "virtio_ring.invalid_completion"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "zero addr",
            self.fixture.queue.add_inbuf(0, BUFFER_LEN) == Err(VirtqueueError::InvalidBuffer),
        );
        assertions.assert(
            "zero len",
            self.fixture.queue.add_inbuf(BUFFER_ADDR, 0) == Err(VirtqueueError::InvalidBuffer),
        );
        let token = match self.fixture.queue.add_inbuf(BUFFER_ADDR, BUFFER_LEN) {
            Ok(token) => token,
            Err(_) => {
                assertions.assert("add inbuf", false);
                return;
            }
        };
        assertions.assert(
            "too large completion",
            self.fixture.queue.fake_complete_used(token, BUFFER_LEN + 1)
                == Err(VirtqueueError::UsedLengthTooLarge),
        );
        assertions.assert_ok(
            "valid completion",
            self.fixture.queue.fake_complete_used(token, BUFFER_LEN),
        );
        assertions.assert(
            "duplicate completion",
            self.fixture.queue.fake_complete_used(token, BUFFER_LEN)
                == Err(VirtqueueError::AlreadyCompleted),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
