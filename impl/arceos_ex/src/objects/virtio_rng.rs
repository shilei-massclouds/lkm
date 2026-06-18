use super::{
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    virtio::{VirtioBus, VirtioDevice},
    virtio_mmio::VIRTIO_ID_RNG,
    virtio_ring::{VirtQueue, VirtqueueBufferToken, VirtqueueError},
};

pub const VIRTIO_RNG_QUEUE_SIZE: u16 = 4;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VirtioRngError {
    DriverNotReady,
    DeviceNotReady,
    UnsupportedDevice,
    InvalidBuffer,
    RequestPending,
    NoRequestPending,
    ZeroLengthCompletion,
    Removed,
    Queue(VirtqueueError),
}

impl From<VirtqueueError> for VirtioRngError {
    fn from(error: VirtqueueError) -> Self {
        Self::Queue(error)
    }
}

pub struct VirtioRngDriver {
    lifecycle: Lifecycle,
    name_bound: bool,
    id_table_contains_rng: bool,
    probe_called: bool,
    probe_return_zero: bool,
    matched_device: bool,
}

impl VirtioRngDriver {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            name_bound: false,
            id_table_contains_rng: false,
            probe_called: false,
            probe_return_zero: false,
            matched_device: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn name_bound(&self) -> bool {
        self.name_bound
    }

    pub const fn id_table_contains_rng(&self) -> bool {
        self.id_table_contains_rng
    }

    pub const fn probe_called(&self) -> bool {
        self.probe_called
    }

    pub const fn probe_return_zero(&self) -> bool {
        self.probe_return_zero
    }

    pub const fn matched_device(&self) -> bool {
        self.matched_device
    }

    pub fn setup(&mut self, virtio_bus: &VirtioBus) -> EventResult {
        if self.lifecycle.state() != State::Base
            || virtio_bus.state() != State::Ready
            || !virtio_bus.registered()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.name_bound = true;
        self.id_table_contains_rng = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn matches(&self, device: VirtioDevice) -> bool {
        self.lifecycle.state() == State::Ready
            && self.id_table_contains_rng
            && device.device_id() == VIRTIO_ID_RNG
    }

    pub fn probe(&mut self, device: VirtioDevice) -> Result<VirtioRngDevice, VirtioRngError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DriverNotReady);
        }
        if !self.matches(device) {
            return Err(VirtioRngError::UnsupportedDevice);
        }

        self.probe_called = true;
        self.matched_device = true;

        let mut rng = VirtioRngDevice::new(device);
        rng.setup()?;
        self.probe_return_zero = true;
        Ok(rng)
    }
}

pub struct VirtioRngDevice {
    lifecycle: Lifecycle,
    virtio_device: VirtioDevice,
    queue: VirtQueue,
    single_input_queue: bool,
    hwrng_registration_deferred: bool,
    random_pool_deferred: bool,
    user_api_deferred: bool,
    real_notify_irq_deferred: bool,
    pending_token: Option<VirtqueueBufferToken>,
    pending_buffer_addr: usize,
    pending_buffer_len: u32,
    request_pending: bool,
    request_submits_inbuf: bool,
    request_kicks_queue: bool,
    repeat_request_rejected: bool,
    fake_transport_completion_recorded: bool,
    zero_len_completion_rejected: bool,
    complete_gets_used_buffer: bool,
    data_avail_updated: bool,
    data_idx_reset: bool,
    removed_rejects_io: bool,
    data_avail: u32,
    data_idx: u32,
    request_count: usize,
    fake_completion_count: usize,
    completion_count: usize,
}

impl VirtioRngDevice {
    fn new(virtio_device: VirtioDevice) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            virtio_device,
            queue: VirtQueue::new(VIRTIO_RNG_QUEUE_SIZE),
            single_input_queue: false,
            hwrng_registration_deferred: false,
            random_pool_deferred: false,
            user_api_deferred: false,
            real_notify_irq_deferred: false,
            pending_token: None,
            pending_buffer_addr: 0,
            pending_buffer_len: 0,
            request_pending: false,
            request_submits_inbuf: false,
            request_kicks_queue: false,
            repeat_request_rejected: false,
            fake_transport_completion_recorded: false,
            zero_len_completion_rejected: false,
            complete_gets_used_buffer: false,
            data_avail_updated: false,
            data_idx_reset: false,
            removed_rejects_io: false,
            data_avail: 0,
            data_idx: 0,
            request_count: 0,
            fake_completion_count: 0,
            completion_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn virtio_device(&self) -> VirtioDevice {
        self.virtio_device
    }

    pub const fn queue(&self) -> &VirtQueue {
        &self.queue
    }

    pub const fn single_input_queue(&self) -> bool {
        self.single_input_queue
    }

    pub const fn hwrng_registration_deferred(&self) -> bool {
        self.hwrng_registration_deferred
    }

    pub const fn random_pool_deferred(&self) -> bool {
        self.random_pool_deferred
    }

    pub const fn user_api_deferred(&self) -> bool {
        self.user_api_deferred
    }

    pub const fn real_notify_irq_deferred(&self) -> bool {
        self.real_notify_irq_deferred
    }

    pub const fn request_pending(&self) -> bool {
        self.request_pending
    }

    pub const fn request_submits_inbuf(&self) -> bool {
        self.request_submits_inbuf
    }

    pub const fn request_kicks_queue(&self) -> bool {
        self.request_kicks_queue
    }

    pub const fn repeat_request_rejected(&self) -> bool {
        self.repeat_request_rejected
    }

    pub const fn fake_transport_completion_recorded(&self) -> bool {
        self.fake_transport_completion_recorded
    }

    pub const fn zero_len_completion_rejected(&self) -> bool {
        self.zero_len_completion_rejected
    }

    pub const fn complete_gets_used_buffer(&self) -> bool {
        self.complete_gets_used_buffer
    }

    pub const fn data_avail_updated(&self) -> bool {
        self.data_avail_updated
    }

    pub const fn data_idx_reset(&self) -> bool {
        self.data_idx_reset
    }

    pub const fn removed_rejects_io(&self) -> bool {
        self.removed_rejects_io
    }

    pub const fn data_avail(&self) -> u32 {
        self.data_avail
    }

    pub const fn data_idx(&self) -> u32 {
        self.data_idx
    }

    pub const fn request_count(&self) -> usize {
        self.request_count
    }

    pub const fn fake_completion_count(&self) -> usize {
        self.fake_completion_count
    }

    pub const fn completion_count(&self) -> usize {
        self.completion_count
    }

    fn setup(&mut self) -> Result<(), VirtioRngError> {
        if self.lifecycle.state() != State::Base || !self.virtio_device.is_rng() {
            return Err(VirtioRngError::UnsupportedDevice);
        }

        self.queue
            .setup()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.single_input_queue = true;
        self.hwrng_registration_deferred = true;
        self.random_pool_deferred = true;
        self.user_api_deferred = true;
        self.real_notify_irq_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
            .map_err(|_| VirtioRngError::DeviceNotReady)
    }

    pub fn request_entropy(
        &mut self,
        buffer_addr: usize,
        buffer_len: u32,
    ) -> Result<(), VirtioRngError> {
        if self.lifecycle.state() == State::Destroyed {
            self.removed_rejects_io = true;
            return Err(VirtioRngError::Removed);
        }
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }
        if buffer_addr == 0 || buffer_len == 0 {
            return Err(VirtioRngError::InvalidBuffer);
        }
        if self.request_pending {
            self.repeat_request_rejected = true;
            return Err(VirtioRngError::RequestPending);
        }

        let token = self.queue.add_inbuf(buffer_addr, buffer_len)?;
        self.queue.kick()?;
        self.pending_token = Some(token);
        self.pending_buffer_addr = buffer_addr;
        self.pending_buffer_len = buffer_len;
        self.request_pending = true;
        self.request_submits_inbuf = true;
        self.request_kicks_queue = true;
        self.data_avail = 0;
        self.data_idx = 0;
        self.data_idx_reset = true;
        self.request_count = self.request_count.saturating_add(1);
        Ok(())
    }

    pub fn fake_transport_complete(&mut self, len: u32) -> Result<(), VirtioRngError> {
        if self.lifecycle.state() == State::Destroyed {
            self.removed_rejects_io = true;
            return Err(VirtioRngError::Removed);
        }
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }
        let Some(token) = self.pending_token else {
            return Err(VirtioRngError::NoRequestPending);
        };
        if len == 0 {
            self.zero_len_completion_rejected = true;
            return Err(VirtioRngError::ZeroLengthCompletion);
        }

        self.queue.fake_complete_used(token, len)?;
        self.fake_transport_completion_recorded = true;
        self.fake_completion_count = self.fake_completion_count.saturating_add(1);
        Ok(())
    }

    pub fn complete_entropy(&mut self) -> Result<u32, VirtioRngError> {
        if self.lifecycle.state() == State::Destroyed {
            self.removed_rejects_io = true;
            return Err(VirtioRngError::Removed);
        }
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }
        if !self.request_pending {
            return Err(VirtioRngError::NoRequestPending);
        }

        let used = self.queue.get_buf()?;
        self.pending_token = None;
        self.pending_buffer_addr = 0;
        self.pending_buffer_len = 0;
        self.request_pending = false;
        self.complete_gets_used_buffer = true;
        self.data_avail = used.len();
        self.data_idx = 0;
        self.data_avail_updated = true;
        self.data_idx_reset = true;
        self.completion_count = self.completion_count.saturating_add(1);
        Ok(used.len())
    }

    pub fn cleanup(&mut self) -> Result<(), VirtioRngError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }

        self.pending_token = None;
        self.pending_buffer_addr = 0;
        self.pending_buffer_len = 0;
        self.request_pending = false;
        self.data_avail = 0;
        self.data_idx = 0;
        self.removed_rejects_io = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Cleanup, State::Ready, State::Destroyed)
            .map_err(|_| VirtioRngError::DeviceNotReady)
    }
}
