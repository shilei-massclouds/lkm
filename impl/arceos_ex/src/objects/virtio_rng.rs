use super::{
    completion::Completion,
    hwrng::{HwRngCore, HwRngDevice, HwRngDeviceRef, HwRngError, HwRngProvider},
    irq_time::{IrqHandlerRegistry, Plic, PlicIrqDomain},
    kernel_image::KernelImage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    virtio::{VirtioBus, VirtioDevice, VirtioDeviceRef},
    virtio_mmio::{VIRTIO_ID_RNG, VirtioMmioTransportDevice},
    virtio_ring::{VirtQueue, VirtqueueBufferToken, VirtqueueError},
};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub const VIRTIO_RNG_QUEUE_SIZE: u16 = 4;
const VIRTIO_RNG_QUEUE_INDEX: u16 = 0;
const VIRTIO_RNG_BUFFER_SIZE: usize = 64;

#[repr(C, align(64))]
struct EntropyBuffer {
    bytes: [u8; VIRTIO_RNG_BUFFER_SIZE],
}

static mut VIRTIO_RNG_ENTROPY_BUFFER: EntropyBuffer = EntropyBuffer {
    bytes: [0; VIRTIO_RNG_BUFFER_SIZE],
};
static VIRTIO_RNG_LAST_IRQ_STATUS: AtomicU32 = AtomicU32::new(0);
static VIRTIO_RNG_IRQ_COMPLETION_CALLS: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_RNG_ENTROPY_READY_CHECKPOINTS: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_RNG_LIVE_PTR: AtomicUsize = AtomicUsize::new(0);

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
    TransportUnavailable,
    HwRng(HwRngError),
    NoDataAvailable,
}

impl From<VirtqueueError> for VirtioRngError {
    fn from(error: VirtqueueError) -> Self {
        Self::Queue(error)
    }
}

impl From<HwRngError> for VirtioRngError {
    fn from(error: HwRngError) -> Self {
        Self::HwRng(error)
    }
}

pub struct VirtioRngDriver {
    lifecycle: Lifecycle,
    name_bound: bool,
    id_table_contains_rng: bool,
    probe_called: bool,
    probe_return_zero: bool,
    matched_device: bool,
    scan_callback_bound: bool,
    scan_called: bool,
    scan_registered_hwrng: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl VirtioRngDriver {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            name_bound: false,
            id_table_contains_rng: false,
            probe_called: false,
            probe_return_zero: false,
            matched_device: false,
            scan_callback_bound: false,
            scan_called: false,
            scan_registered_hwrng: false,
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

    pub const fn scan_callback_bound(&self) -> bool {
        self.scan_callback_bound
    }

    pub const fn scan_called(&self) -> bool {
        self.scan_called
    }

    pub const fn scan_registered_hwrng(&self) -> bool {
        self.scan_registered_hwrng
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
        self.scan_callback_bound = true;
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

    pub fn scan(
        &mut self,
        rng: &mut VirtioRngDevice,
        hwrng_core: &mut HwRngCore,
    ) -> Result<HwRngDeviceRef, VirtioRngError> {
        if self.lifecycle.state() != State::Ready || !self.scan_callback_bound {
            return Err(VirtioRngError::DriverNotReady);
        }
        if rng.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }

        let device_ref = rng.register_hwrng(hwrng_core)?;
        self.scan_called = true;
        self.scan_registered_hwrng = true;
        Ok(device_ref)
    }
}

pub struct VirtioRngDevice {
    lifecycle: Lifecycle,
    virtio_device: VirtioDevice,
    queue: VirtQueue,
    hwrng: HwRngDevice,
    have_data: Completion,
    single_input_queue: bool,
    hwrng_embedded: bool,
    hwrng_register_done: bool,
    random_pool_deferred: bool,
    dev_hwrng_plumbing_deferred: bool,
    blocking_wait_deferred: bool,
    real_notify_irq_deferred: bool,
    real_notify_irq_ready: bool,
    probe_common_requested_entropy: bool,
    pending_token: Option<VirtqueueBufferToken>,
    pending_buffer_addr: usize,
    pending_buffer_len: u32,
    request_pending: bool,
    request_submits_inbuf: bool,
    request_kicks_queue: bool,
    repeat_request_rejected: bool,
    zero_len_completion_rejected: bool,
    complete_gets_used_buffer: bool,
    data_avail_updated: bool,
    data_idx_reset: bool,
    have_data_completion_ready: bool,
    have_data_reinitialized: bool,
    have_data_completed: bool,
    read_consumes_available_data: bool,
    read_updates_data_idx: bool,
    read_updates_data_avail: bool,
    read_requeues_when_empty: bool,
    read_requeues_below_request_watermark: bool,
    read_count: usize,
    last_read_len: usize,
    removed_rejects_io: bool,
    data_avail: u32,
    data_idx: u32,
    request_count: usize,
    notify_count: usize,
    irq_count: usize,
    completion_count: usize,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl VirtioRngDevice {
    fn new(virtio_device: VirtioDevice) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            virtio_device,
            queue: VirtQueue::new(VIRTIO_RNG_QUEUE_SIZE),
            hwrng: HwRngDevice::new_virtio_rng(0),
            have_data: Completion::new(),
            single_input_queue: false,
            hwrng_embedded: false,
            hwrng_register_done: false,
            random_pool_deferred: false,
            dev_hwrng_plumbing_deferred: false,
            blocking_wait_deferred: false,
            real_notify_irq_deferred: false,
            real_notify_irq_ready: false,
            probe_common_requested_entropy: false,
            pending_token: None,
            pending_buffer_addr: 0,
            pending_buffer_len: 0,
            request_pending: false,
            request_submits_inbuf: false,
            request_kicks_queue: false,
            repeat_request_rejected: false,
            zero_len_completion_rejected: false,
            complete_gets_used_buffer: false,
            data_avail_updated: false,
            data_idx_reset: false,
            have_data_completion_ready: false,
            have_data_reinitialized: false,
            have_data_completed: false,
            read_consumes_available_data: false,
            read_updates_data_idx: false,
            read_updates_data_avail: false,
            read_requeues_when_empty: false,
            read_requeues_below_request_watermark: false,
            read_count: 0,
            last_read_len: 0,
            removed_rejects_io: false,
            data_avail: 0,
            data_idx: 0,
            request_count: 0,
            notify_count: 0,
            irq_count: 0,
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

    pub const fn hwrng(&self) -> &HwRngDevice {
        &self.hwrng
    }

    pub const fn have_data(&self) -> &Completion {
        &self.have_data
    }

    pub const fn single_input_queue(&self) -> bool {
        self.single_input_queue
    }

    pub const fn hwrng_embedded(&self) -> bool {
        self.hwrng_embedded
    }

    pub const fn hwrng_register_done(&self) -> bool {
        self.hwrng_register_done
    }

    pub const fn random_pool_deferred(&self) -> bool {
        self.random_pool_deferred
    }

    pub const fn dev_hwrng_plumbing_deferred(&self) -> bool {
        self.dev_hwrng_plumbing_deferred
    }

    pub const fn blocking_wait_deferred(&self) -> bool {
        self.blocking_wait_deferred
    }

    pub const fn real_notify_irq_deferred(&self) -> bool {
        self.real_notify_irq_deferred
    }

    #[allow(dead_code)]
    pub const fn real_notify_irq_ready(&self) -> bool {
        self.real_notify_irq_ready
    }

    #[allow(dead_code)]
    pub const fn probe_common_requested_entropy(&self) -> bool {
        self.probe_common_requested_entropy
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

    pub const fn have_data_completion_ready(&self) -> bool {
        self.have_data_completion_ready
    }

    #[allow(dead_code)]
    pub const fn have_data_reinitialized(&self) -> bool {
        self.have_data_reinitialized
    }

    #[allow(dead_code)]
    pub const fn have_data_completed(&self) -> bool {
        self.have_data_completed
    }

    pub const fn read_consumes_available_data(&self) -> bool {
        self.read_consumes_available_data
    }

    pub const fn read_updates_data_idx(&self) -> bool {
        self.read_updates_data_idx
    }

    pub const fn read_updates_data_avail(&self) -> bool {
        self.read_updates_data_avail
    }

    pub const fn read_requeues_when_empty(&self) -> bool {
        self.read_requeues_when_empty
    }

    pub const fn read_requeues_below_request_watermark(&self) -> bool {
        self.read_requeues_below_request_watermark
    }

    pub const fn read_count(&self) -> usize {
        self.read_count
    }

    pub const fn last_read_len(&self) -> usize {
        self.last_read_len
    }

    pub const fn request_count(&self) -> usize {
        self.request_count
    }

    #[allow(dead_code)]
    pub const fn notify_count(&self) -> usize {
        self.notify_count
    }

    #[allow(dead_code)]
    pub const fn irq_count(&self) -> usize {
        self.irq_count
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
        self.hwrng
            .setup()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.have_data
            .setup()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.have_data
            .enable()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.single_input_queue = true;
        self.hwrng_embedded = true;
        self.random_pool_deferred = true;
        self.dev_hwrng_plumbing_deferred = true;
        self.blocking_wait_deferred = true;
        self.real_notify_irq_deferred = true;
        self.have_data_completion_ready = true;
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
        self.have_data
            .reinit()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.have_data_reinitialized = true;
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

    pub fn setup_real_transport(
        &mut self,
        kernel_image: &KernelImage,
        plic: &Plic,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> Result<(), VirtioRngError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }
        let Some(transport) = self.virtio_device.mmio_transport() else {
            return Err(VirtioRngError::TransportUnavailable);
        };
        if !transport.irq_handler_registered()
            || !irq_handler_registry.has_handler_for_logical_irq(transport.logical_irq())
        {
            return Err(VirtioRngError::TransportUnavailable);
        }
        if !self.virtio_device.reset_status()
            || !self.virtio_device.setup_driver_status()
            || !self.virtio_device.negotiate_features(0)
        {
            return Err(VirtioRngError::TransportUnavailable);
        }
        self.virtio_device
            .setup_queue(&mut self.queue, kernel_image, VIRTIO_RNG_QUEUE_INDEX)?;
        if !self.virtio_device.set_driver_ok() {
            return Err(VirtioRngError::TransportUnavailable);
        }
        let Some(mut transport) = self.virtio_device.mmio_transport() else {
            return Err(VirtioRngError::TransportUnavailable);
        };
        if !transport.enable_irq_source_gate(plic, plic_irq_domain) {
            return Err(VirtioRngError::TransportUnavailable);
        }
        self.virtio_device.update_mmio_transport(transport);
        self.real_notify_irq_ready = true;
        self.real_notify_irq_deferred = false;
        let (buffer_addr, buffer_len) = entropy_buffer_request(kernel_image)?;
        self.request_entropy_mmio(buffer_addr, buffer_len)?;
        self.probe_common_requested_entropy = true;
        Ok(())
    }

    pub fn request_entropy_mmio(
        &mut self,
        buffer_addr: usize,
        buffer_len: u32,
    ) -> Result<(), VirtioRngError> {
        if self.lifecycle.state() == State::Destroyed {
            self.removed_rejects_io = true;
            return Err(VirtioRngError::Removed);
        }
        if self.lifecycle.state() != State::Ready || !self.real_notify_irq_ready {
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
        self.virtio_device.notify_queue(&mut self.queue)?;
        self.have_data
            .reinit()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.have_data_reinitialized = true;
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
        self.notify_count = self.notify_count.saturating_add(1);
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
        self.request_pending = false;
        self.complete_gets_used_buffer = true;
        if used.len() == 0 {
            self.zero_len_completion_rejected = true;
            return Err(VirtioRngError::ZeroLengthCompletion);
        }
        self.data_avail = used.len();
        self.data_idx = 0;
        self.data_avail_updated = true;
        self.data_idx_reset = true;
        self.completion_count = self.completion_count.saturating_add(1);
        self.have_data
            .complete()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.have_data_completed = true;
        Ok(used.len())
    }

    pub fn complete_entropy_from_irq(&mut self) -> Result<u32, VirtioRngError> {
        if self.lifecycle.state() == State::Destroyed {
            self.removed_rejects_io = true;
            return Err(VirtioRngError::Removed);
        }
        if self.lifecycle.state() != State::Ready || !self.real_notify_irq_ready {
            return Err(VirtioRngError::DeviceNotReady);
        }
        if !self.request_pending {
            return Err(VirtioRngError::NoRequestPending);
        }

        let used = self.queue.get_buf_from_device()?;
        self.pending_token = None;
        self.request_pending = false;
        self.complete_gets_used_buffer = true;
        if used.len() == 0 {
            self.zero_len_completion_rejected = true;
            return Err(VirtioRngError::ZeroLengthCompletion);
        }
        self.data_avail = used.len();
        self.data_idx = 0;
        self.data_avail_updated = true;
        self.data_idx_reset = true;
        self.irq_count = self.irq_count.saturating_add(1);
        self.completion_count = self.completion_count.saturating_add(1);
        self.have_data
            .complete()
            .map_err(|_| VirtioRngError::DeviceNotReady)?;
        self.have_data_completed = true;
        Ok(used.len())
    }

    pub fn register_hwrng(
        &mut self,
        hwrng_core: &mut HwRngCore,
    ) -> Result<HwRngDeviceRef, VirtioRngError> {
        if self.lifecycle.state() != State::Ready || !self.hwrng_embedded {
            return Err(VirtioRngError::DeviceNotReady);
        }
        if self.hwrng_register_done {
            return self
                .hwrng
                .device_ref()
                .ok_or(VirtioRngError::HwRng(HwRngError::ProviderUnavailable));
        }

        let device_ref = hwrng_core.register(&mut self.hwrng)?;
        self.hwrng_register_done = true;
        Ok(device_ref)
    }

    pub fn read_entropy(&mut self, buffer: &mut [u8], wait: bool) -> Result<usize, VirtioRngError> {
        if self.lifecycle.state() == State::Destroyed {
            self.removed_rejects_io = true;
            return Err(VirtioRngError::Removed);
        }
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioRngError::DeviceNotReady);
        }
        if wait {
            self.blocking_wait_deferred = true;
        }
        if buffer.is_empty() || self.data_avail == 0 {
            return Err(VirtioRngError::NoDataAvailable);
        }

        let available = self.data_avail as usize;
        let copy_len = core::cmp::min(buffer.len(), available);
        let start = self.data_idx as usize;
        let entropy = entropy_buffer_bytes();
        let end = core::cmp::min(start.saturating_add(copy_len), entropy.len());
        if end <= start {
            return Err(VirtioRngError::NoDataAvailable);
        }
        let actual_len = end - start;
        buffer[..actual_len].copy_from_slice(&entropy[start..end]);
        self.data_idx = self.data_idx.saturating_add(actual_len as u32);
        self.data_avail = self.data_avail.saturating_sub(actual_len as u32);
        self.read_consumes_available_data = true;
        self.read_updates_data_idx = true;
        self.read_updates_data_avail = true;
        self.read_count = self.read_count.saturating_add(1);
        self.last_read_len = actual_len;
        self.hwrng.record_read(actual_len);

        let empty = self.data_avail == 0;
        let below_request_watermark =
            actual_len == buffer.len() && (self.data_avail as usize) < buffer.len();
        if empty || below_request_watermark {
            self.read_requeues_when_empty |= empty;
            self.read_requeues_below_request_watermark |= below_request_watermark;
            self.data_avail = 0;
            self.data_idx = 0;
            if self.real_notify_irq_ready && !self.request_pending {
                let (buffer_addr, buffer_len) =
                    entropy_buffer_request(&crate::context::context_ref().kernel_image)?;
                self.request_entropy_mmio(buffer_addr, buffer_len)?;
            } else if self.pending_buffer_addr != 0
                && self.pending_buffer_len != 0
                && !self.request_pending
            {
                self.request_entropy(self.pending_buffer_addr, self.pending_buffer_len)?;
            }
        }

        Ok(actual_len)
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

#[cfg(app_smoke)]
pub(crate) mod smoke_fixture {
    use super::{VirtioRngDevice, VirtioRngError};
    use crate::objects::virtio_ring::smoke_fixture as ring_smoke_fixture;

    pub(crate) fn prepare_completion(
        rng: &mut VirtioRngDevice,
        len: u32,
    ) -> Result<(), VirtioRngError> {
        let token = rng.pending_token.ok_or(VirtioRngError::NoRequestPending)?;
        ring_smoke_fixture::prepare_used_entry(&mut rng.queue, token, len)
            .map_err(VirtioRngError::from)
    }
}

impl HwRngProvider for VirtioRngDevice {
    fn read_hwrng(
        &mut self,
        device_ref: HwRngDeviceRef,
        buffer: &mut [u8],
        wait: bool,
    ) -> Result<usize, HwRngError> {
        if self.hwrng.device_ref() != Some(device_ref) || !self.hwrng_register_done {
            return Err(HwRngError::ProviderUnavailable);
        }
        self.read_entropy(buffer, wait)
            .map_err(|error| match error {
                VirtioRngError::NoDataAvailable => HwRngError::EmptyRead,
                VirtioRngError::HwRng(error) => error,
                _ => HwRngError::ProviderUnavailable,
            })
    }
}

pub struct VirtioRngRuntime {
    driver: VirtioRngDriver,
    device: Option<VirtioRngDevice>,
    live_device_ref: Option<VirtioDeviceRef>,
    real_probe_attempted: bool,
    real_probe_succeeded: bool,
    real_completion_len: u32,
}

impl VirtioRngRuntime {
    pub const fn new() -> Self {
        Self {
            driver: VirtioRngDriver::new(),
            device: None,
            live_device_ref: None,
            real_probe_attempted: false,
            real_probe_succeeded: false,
            real_completion_len: 0,
        }
    }

    #[allow(dead_code)]
    pub const fn driver(&self) -> &VirtioRngDriver {
        &self.driver
    }

    pub const fn device(&self) -> Option<&VirtioRngDevice> {
        self.device.as_ref()
    }

    #[allow(dead_code)]
    pub const fn real_probe_attempted(&self) -> bool {
        self.real_probe_attempted
    }

    #[allow(dead_code)]
    pub const fn real_probe_succeeded(&self) -> bool {
        self.real_probe_succeeded
    }

    #[allow(dead_code)]
    pub const fn real_completion_len(&self) -> u32 {
        self.real_completion_len
    }

    pub fn read_current_hwrng(
        &mut self,
        hwrng_core: &mut HwRngCore,
        buffer: &mut [u8],
        wait: bool,
    ) -> Result<usize, HwRngError> {
        let Some(device) = self.device.as_mut() else {
            return Err(HwRngError::ProviderUnavailable);
        };
        hwrng_core.read_current(device, buffer, wait)
    }
}

pub fn setup_live_driver(
    context_runtime: &mut VirtioRngRuntime,
    virtio_bus: &VirtioBus,
    hwrng_core: &mut HwRngCore,
    kernel_image: &KernelImage,
    plic: &Plic,
    plic_irq_domain: &mut PlicIrqDomain,
    irq_handler_registry: &IrqHandlerRegistry,
) -> EventResult {
    VIRTIO_RNG_LIVE_PTR.store(
        context_runtime as *mut VirtioRngRuntime as usize,
        Ordering::Release,
    );
    let runtime = context_runtime;
    runtime.real_probe_attempted = true;
    if runtime.driver.state() == State::Base {
        runtime.driver.setup(virtio_bus)?;
    }
    let Some(device) = virtio_bus.rng_device() else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    if runtime.live_device_ref == Some(device.device_ref()) && runtime.real_probe_succeeded {
        return Ok(());
    }
    let mut rng = runtime
        .driver
        .probe(device)
        .map_err(|_| live_setup_error())?;
    rng.setup_real_transport(kernel_image, plic, plic_irq_domain, irq_handler_registry)
        .map_err(|_| live_setup_error())?;
    runtime
        .driver
        .scan(&mut rng, hwrng_core)
        .map_err(|_| live_setup_error())?;
    runtime.live_device_ref = Some(device.device_ref());
    runtime.device = Some(rng);
    runtime.real_probe_succeeded = true;
    Ok(())
}

pub fn live_runtime() -> Option<&'static VirtioRngRuntime> {
    let ptr = VIRTIO_RNG_LIVE_PTR.load(Ordering::Acquire);
    if ptr == 0 {
        return None;
    }
    unsafe { (ptr as *const VirtioRngRuntime).as_ref() }
}

fn live_runtime_mut() -> Option<&'static mut VirtioRngRuntime> {
    let ptr = VIRTIO_RNG_LIVE_PTR.load(Ordering::Acquire);
    if ptr == 0 {
        return None;
    }
    unsafe { (ptr as *mut VirtioRngRuntime).as_mut() }
}

pub fn live_mmio_transport() -> Option<VirtioMmioTransportDevice> {
    live_runtime()?.device()?.virtio_device().mmio_transport()
}

pub fn note_mmio_irq(status: u32) {
    VIRTIO_RNG_LAST_IRQ_STATUS.store(status, Ordering::Release);
}

pub fn handle_irq_completion() {
    VIRTIO_RNG_IRQ_COMPLETION_CALLS.fetch_add(1, Ordering::AcqRel);
    let Some(runtime) = live_runtime_mut() else {
        return;
    };
    let Some(device) = runtime.device.as_mut() else {
        return;
    };
    let Ok(len) = device.complete_entropy_from_irq() else {
        return;
    };
    runtime.real_completion_len = len;
    VIRTIO_RNG_ENTROPY_READY_CHECKPOINTS.fetch_add(1, Ordering::AcqRel);
    crate::checkpoint::dispatch(
        crate::checkpoint::Checkpoint::VirtioRngEntropyReady,
        crate::context::context_ref(),
    );
}

#[allow(dead_code)]
pub fn last_irq_status() -> u32 {
    VIRTIO_RNG_LAST_IRQ_STATUS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn irq_completion_calls() -> usize {
    VIRTIO_RNG_IRQ_COMPLETION_CALLS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn entropy_ready_checkpoints() -> usize {
    VIRTIO_RNG_ENTROPY_READY_CHECKPOINTS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn entropy_buffer_nonzero() -> bool {
    let bytes = unsafe {
        &(&raw const VIRTIO_RNG_ENTROPY_BUFFER)
            .as_ref()
            .unwrap()
            .bytes
    };
    bytes.iter().any(|byte| *byte != 0)
}

fn entropy_buffer_request(kernel_image: &KernelImage) -> Result<(usize, u32), VirtioRngError> {
    let virt = unsafe {
        (&raw mut VIRTIO_RNG_ENTROPY_BUFFER)
            .as_mut()
            .ok_or(VirtioRngError::InvalidBuffer)?
            .bytes
            .as_mut_ptr() as usize
    };
    let phys = kernel_image
        .runtime_to_phys(virt)
        .ok_or(VirtioRngError::InvalidBuffer)?;
    Ok((phys, VIRTIO_RNG_BUFFER_SIZE as u32))
}

fn entropy_buffer_bytes() -> &'static [u8; VIRTIO_RNG_BUFFER_SIZE] {
    unsafe {
        &(&raw const VIRTIO_RNG_ENTROPY_BUFFER)
            .as_ref()
            .unwrap()
            .bytes
    }
}

fn live_setup_error() -> super::state::EventError {
    super::state::EventError::failed(
        super::state::EventErrorCode::ConditionFailed,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        State::Ready,
    )
}
