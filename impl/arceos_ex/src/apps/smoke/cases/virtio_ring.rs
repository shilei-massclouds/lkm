use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    objects::{
        state::State,
        virtio_ring::{smoke_fixture, VirtQueue, VirtqueueDescriptorSpec, VirtqueueError},
    },
};

const BUFFER_ADDR: usize = 0x8020_0000;
const BUFFER_LEN: u32 = 64;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SplitRingLifecycleScenario::new());
    suite.scenario(&mut SingleInbufCompletionScenario::new());
    suite.scenario(&mut DescriptorChainScenario::new());
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
        assertions.assert(
            "direct chain",
            self.queue.ring().direct_descriptor_chain_ready(),
        );
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
            self.fixture.queue.free_descriptors_consumed(),
        );
        assertions.assert(
            "chain allocated",
            self.fixture.queue.descriptor_chain_allocated(),
        );
        assertions.assert("chain direct", self.fixture.queue.descriptor_chain_direct());
        assertions.assert("chain in", self.fixture.queue.in_descriptor_added());
        assertions.assert("chain published", self.fixture.queue.chain_head_published());
        assertions.assert(
            "chain count matched",
            self.fixture.queue.chain_free_descriptor_count_matched(),
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
        assertions.assert("descriptor no next", !descriptor.has_next());

        assertions.assert_ok("kick", self.fixture.queue.kick());
        assertions.assert("kick recorded", self.fixture.queue.kick_recorded());
        assertions.assert("kick count", self.fixture.queue.kick_count() == 1);

        assertions.assert_ok(
            "fixture used entry",
            smoke_fixture::prepare_used_entry(&mut self.fixture.queue, token, 32),
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
        assertions.assert("in descriptor len", used.in_descriptor_len() == BUFFER_LEN);
        assertions.assert("descriptor count", used.descriptor_count() == 1);
        assertions.assert(
            "get buf returns len",
            self.fixture.queue.get_buf_returns_len(),
        );
        assertions.assert(
            "chain released",
            self.fixture.queue.descriptor_chain_released(),
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

struct DescriptorChainScenario {
    fixture: VirtQueueFixture,
}

impl DescriptorChainScenario {
    fn new() -> Self {
        Self {
            fixture: VirtQueueFixture::new(4),
        }
    }
}

impl SmokeScenario for DescriptorChainScenario {
    fn name(&self) -> &'static str {
        "virtio_ring.descriptor_chain"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let specs = [
            VirtqueueDescriptorSpec::out(BUFFER_ADDR, 16),
            VirtqueueDescriptorSpec::inbuf(BUFFER_ADDR + 64, 128),
            VirtqueueDescriptorSpec::inbuf(BUFFER_ADDR + 256, 1),
        ];
        let token = match self.fixture.queue.add_chain(&specs) {
            Ok(token) => token,
            Err(_) => {
                assertions.assert("add chain", false);
                return;
            }
        };
        assertions.assert(
            "chain allocated",
            self.fixture.queue.descriptor_chain_allocated(),
        );
        assertions.assert("chain direct", self.fixture.queue.descriptor_chain_direct());
        assertions.assert("chain out", self.fixture.queue.out_descriptor_added());
        assertions.assert("chain in", self.fixture.queue.in_descriptor_added());
        assertions.assert("chain published", self.fixture.queue.chain_head_published());
        assertions.assert(
            "chain count matched",
            self.fixture.queue.chain_free_descriptor_count_matched(),
        );
        assertions.assert(
            "free count dec",
            self.fixture.queue.ring().free_count() == 1,
        );
        assertions.assert("avail idx", self.fixture.queue.ring().avail_idx() == 1);
        assertions.assert(
            "avail head",
            self.fixture.queue.ring().avail_entry(0) == Some(token.head()),
        );

        let Some(first) = self.fixture.queue.ring().descriptor(token) else {
            assertions.assert("first descriptor", false);
            return;
        };
        let second_index = first.next();
        let Some(second) = self.fixture.queue.ring().descriptor_by_index(second_index) else {
            assertions.assert("second descriptor", false);
            return;
        };
        let third_index = second.next();
        let Some(third) = self.fixture.queue.ring().descriptor_by_index(third_index) else {
            assertions.assert("third descriptor", false);
            return;
        };
        assertions.assert("first out", !first.writable());
        assertions.assert("first next", first.has_next());
        assertions.assert("second in", second.writable());
        assertions.assert("second next", second.has_next());
        assertions.assert("third in", third.writable());
        assertions.assert("third last", !third.has_next() && third.direct());

        assertions.assert_ok(
            "fixture used entry",
            smoke_fixture::prepare_used_entry(&mut self.fixture.queue, token, 128),
        );
        let used = match self.fixture.queue.get_buf() {
            Ok(used) => used,
            Err(_) => {
                assertions.assert("get chain", false);
                return;
            }
        };
        assertions.assert("used token", used.token() == token);
        assertions.assert("used len", used.len() == 128);
        assertions.assert("first len", used.descriptor_len() == 16);
        assertions.assert("in len", used.in_descriptor_len() == 129);
        assertions.assert("descriptor count", used.descriptor_count() == 3);
        assertions.assert(
            "chain released",
            self.fixture.queue.descriptor_chain_released(),
        );
        assertions.assert("free restored", self.fixture.queue.ring().free_count() == 4);
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

        let mut chain_queue = VirtQueue::new(2);
        assertions.assert_ok("chain queue setup", chain_queue.setup());
        assertions.assert(
            "oversized chain",
            chain_queue.add_chain(&[
                VirtqueueDescriptorSpec::out(BUFFER_ADDR, 16),
                VirtqueueDescriptorSpec::inbuf(BUFFER_ADDR + 64, 16),
                VirtqueueDescriptorSpec::inbuf(BUFFER_ADDR + 128, 1),
            ]) == Err(VirtqueueError::ChainTooLong),
        );
        assertions.assert("oversized chain free", chain_queue.ring().free_count() == 2);
        assertions.assert_ok("consume one", chain_queue.add_inbuf(BUFFER_ADDR + 192, 16));
        assertions.assert(
            "insufficient chain",
            chain_queue.add_chain(&[
                VirtqueueDescriptorSpec::out(BUFFER_ADDR + 256, 16),
                VirtqueueDescriptorSpec::inbuf(BUFFER_ADDR + 320, 16),
            ]) == Err(VirtqueueError::NoFreeDescriptor),
        );
        assertions.assert("atomic failure", chain_queue.ring().free_count() == 1);
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
        assertions.assert(
            "empty chain",
            self.fixture.queue.add_chain(&[]) == Err(VirtqueueError::InvalidBuffer),
        );
        assertions.assert(
            "chain invalid descriptor",
            self.fixture.queue.add_chain(&[
                VirtqueueDescriptorSpec::out(BUFFER_ADDR, 16),
                VirtqueueDescriptorSpec::inbuf(0, 16),
            ]) == Err(VirtqueueError::InvalidBuffer),
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
            smoke_fixture::prepare_used_entry(&mut self.fixture.queue, token, BUFFER_LEN + 1)
                == Err(VirtqueueError::UsedLengthTooLarge),
        );
        assertions.assert_ok(
            "valid completion",
            smoke_fixture::prepare_used_entry(&mut self.fixture.queue, token, BUFFER_LEN),
        );
        assertions.assert(
            "duplicate completion",
            smoke_fixture::prepare_used_entry(&mut self.fixture.queue, token, BUFFER_LEN)
                == Err(VirtqueueError::AlreadyCompleted),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
