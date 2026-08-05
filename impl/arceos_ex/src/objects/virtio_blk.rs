use super::{
    block_device::{
        BlockDevice, BlockDeviceError, BlockDeviceProvider, BlockDeviceRef, BlockDeviceRegistry,
    },
    irq_spinlock::IrqSpinLock,
    irq_time::{Plic, PlicIrqDomain},
    kernel_image::KernelImage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    virtio::{VirtioBus, VirtioDevice, VirtioDeviceRef},
    virtio_mmio::{VIRTIO_ID_BLOCK, VIRTIO_MMIO_INT_VRING, VirtioMmioTransportDevice},
    virtio_ring::{VirtQueue, VirtqueueBufferToken, VirtqueueDescriptorSpec, VirtqueueError},
};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const VIRTIO_BLK_QUEUE_SIZE: u16 = 8;
const VIRTIO_BLK_QUEUE_INDEX: u16 = 0;
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_EXT2_SUPERBLOCK_SECTOR: u64 = 2;
const EXT2_SUPER_MAGIC_OFFSET_IN_SECTOR: usize = 56;
const EXT2_SUPER_MAGIC: u16 = 0xef53;
const VIRTIO_BLK_READ_BUFFER_SIZE: usize = 4096;
const VIRTIO_BLK_READ_WAIT_SPINS: usize = 1_000_000;

#[repr(C)]
struct VirtioBlkOutHdr {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(C, align(64))]
struct VirtioBlkReadRequest {
    header: VirtioBlkOutHdr,
    data: [u8; VIRTIO_BLK_READ_BUFFER_SIZE],
    status: u8,
}

static mut VIRTIO_BLK_READ_REQUEST: VirtioBlkReadRequest = VirtioBlkReadRequest {
    header: VirtioBlkOutHdr {
        request_type: 0,
        reserved: 0,
        sector: 0,
    },
    data: [0; VIRTIO_BLK_READ_BUFFER_SIZE],
    status: 0xff,
};
static VIRTIO_BLK_LAST_IRQ_STATUS: AtomicU32 = AtomicU32::new(0);
static VIRTIO_BLK_IRQ_COMPLETION_CALLS: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_BLK_READ_READY_CHECKPOINTS: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_BLK_LIVE_PTR: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_BLK_LIVE_READ_SUBMITTED_CHECKPOINTS: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_BLK_LIVE_READ_COMPLETED_CHECKPOINTS: AtomicUsize = AtomicUsize::new(0);
static VIRTIO_BLK_SYNC_OWNER: IrqSpinLock<()> = IrqSpinLock::new(());

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VirtioBlkError {
    DriverNotReady,
    DeviceNotReady,
    UnsupportedDevice,
    CapacityUnavailable,
    InvalidBuffer,
    RequestPending,
    NoRequestPending,
    BadStatus,
    DataMismatch,
    BlockRegistry,
    Queue(VirtqueueError),
    TransportUnavailable,
}

impl From<VirtqueueError> for VirtioBlkError {
    fn from(error: VirtqueueError) -> Self {
        Self::Queue(error)
    }
}

pub struct VirtioBlkDriver {
    lifecycle: Lifecycle,
    name_bound: bool,
    id_table_contains_block: bool,
    probe_called: bool,
    probe_return_zero: bool,
    matched_device: bool,
}

#[allow(dead_code)]
impl VirtioBlkDriver {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            name_bound: false,
            id_table_contains_block: false,
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

    pub const fn id_table_contains_block(&self) -> bool {
        self.id_table_contains_block
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
        self.id_table_contains_block = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn matches(&self, device: VirtioDevice) -> bool {
        self.lifecycle.state() == State::Ready
            && self.id_table_contains_block
            && device.device_id() == VIRTIO_ID_BLOCK
    }

    pub fn probe(&mut self, device: VirtioDevice) -> Result<VirtioBlkDevice, VirtioBlkError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioBlkError::DriverNotReady);
        }
        if !self.matches(device) {
            return Err(VirtioBlkError::UnsupportedDevice);
        }

        self.probe_called = true;
        self.matched_device = true;

        let mut block = VirtioBlkDevice::new(device);
        block.setup()?;
        self.probe_return_zero = true;
        Ok(block)
    }
}

pub struct VirtioBlkDevice {
    lifecycle: Lifecycle,
    virtio_device: VirtioDevice,
    queue: VirtQueue,
    block_device: BlockDevice,
    single_request_queue: bool,
    capacity: Option<u64>,
    capacity_read: bool,
    capacity_nonzero: bool,
    queue_setup_done: bool,
    driver_ok: bool,
    pending_token: Option<VirtqueueBufferToken>,
    pending_sector: u64,
    pending_data_len: u32,
    read_header_prepared: bool,
    read_data_buffer_prepared: bool,
    read_status_buffer_prepared: bool,
    read_request_pending: bool,
    read_request_submitted: bool,
    read_request_notified: bool,
    mmio_irq_acknowledged: bool,
    irq_callback_invoked: bool,
    completion_observed_by_irq: bool,
    completion_observed_by_sync_poll: bool,
    complete_gets_used_buffer: bool,
    complete_status_ok: bool,
    complete_data_nonzero: bool,
    complete_ext2_magic_observed: bool,
    read_request_done: bool,
    request_count: usize,
    notify_count: usize,
    irq_count: usize,
    completion_count: usize,
    last_status: u8,
    last_used_len: u32,
    last_sector: u64,
    request_queue_deferred: bool,
    filesystem_parse_deferred: bool,
    multi_queue_deferred: bool,
    reset_remove_deferred: bool,
    block_read_served: bool,
    block_read_copies_to_caller: bool,
}

#[allow(dead_code)]
impl VirtioBlkDevice {
    fn new(virtio_device: VirtioDevice) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            virtio_device,
            queue: VirtQueue::new(VIRTIO_BLK_QUEUE_SIZE),
            block_device: BlockDevice::new_virtio_blk(0),
            single_request_queue: false,
            capacity: None,
            capacity_read: false,
            capacity_nonzero: false,
            queue_setup_done: false,
            driver_ok: false,
            pending_token: None,
            pending_sector: 0,
            pending_data_len: 0,
            read_header_prepared: false,
            read_data_buffer_prepared: false,
            read_status_buffer_prepared: false,
            read_request_pending: false,
            read_request_submitted: false,
            read_request_notified: false,
            mmio_irq_acknowledged: false,
            irq_callback_invoked: false,
            completion_observed_by_irq: false,
            completion_observed_by_sync_poll: false,
            complete_gets_used_buffer: false,
            complete_status_ok: false,
            complete_data_nonzero: false,
            complete_ext2_magic_observed: false,
            read_request_done: false,
            request_count: 0,
            notify_count: 0,
            irq_count: 0,
            completion_count: 0,
            last_status: 0xff,
            last_used_len: 0,
            last_sector: 0,
            request_queue_deferred: true,
            filesystem_parse_deferred: true,
            multi_queue_deferred: true,
            reset_remove_deferred: true,
            block_read_served: false,
            block_read_copies_to_caller: false,
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

    pub const fn block_device(&self) -> &BlockDevice {
        &self.block_device
    }

    pub const fn single_request_queue(&self) -> bool {
        self.single_request_queue
    }

    pub const fn capacity(&self) -> Option<u64> {
        self.capacity
    }

    pub const fn capacity_read(&self) -> bool {
        self.capacity_read
    }

    pub const fn capacity_nonzero(&self) -> bool {
        self.capacity_nonzero
    }

    pub const fn queue_setup_done(&self) -> bool {
        self.queue_setup_done
    }

    pub const fn driver_ok(&self) -> bool {
        self.driver_ok
    }

    pub const fn read_header_prepared(&self) -> bool {
        self.read_header_prepared
    }

    pub const fn read_data_buffer_prepared(&self) -> bool {
        self.read_data_buffer_prepared
    }

    pub const fn read_status_buffer_prepared(&self) -> bool {
        self.read_status_buffer_prepared
    }

    pub const fn read_request_pending(&self) -> bool {
        self.read_request_pending
    }

    pub const fn read_request_submitted(&self) -> bool {
        self.read_request_submitted
    }

    pub const fn read_request_notified(&self) -> bool {
        self.read_request_notified
    }

    pub const fn mmio_irq_acknowledged(&self) -> bool {
        self.mmio_irq_acknowledged
    }

    pub const fn irq_callback_invoked(&self) -> bool {
        self.irq_callback_invoked
    }

    pub const fn completion_observed_by_irq(&self) -> bool {
        self.completion_observed_by_irq
    }

    pub const fn completion_observed_by_sync_poll(&self) -> bool {
        self.completion_observed_by_sync_poll
    }

    pub const fn complete_gets_used_buffer(&self) -> bool {
        self.complete_gets_used_buffer
    }

    pub const fn complete_status_ok(&self) -> bool {
        self.complete_status_ok
    }

    pub const fn complete_data_nonzero(&self) -> bool {
        self.complete_data_nonzero
    }

    pub const fn complete_ext2_magic_observed(&self) -> bool {
        self.complete_ext2_magic_observed
    }

    pub const fn read_request_done(&self) -> bool {
        self.read_request_done
    }

    pub const fn request_count(&self) -> usize {
        self.request_count
    }

    pub const fn notify_count(&self) -> usize {
        self.notify_count
    }

    pub const fn irq_count(&self) -> usize {
        self.irq_count
    }

    pub const fn completion_count(&self) -> usize {
        self.completion_count
    }

    pub const fn last_status(&self) -> u8 {
        self.last_status
    }

    pub const fn last_used_len(&self) -> u32 {
        self.last_used_len
    }

    pub const fn last_sector(&self) -> u64 {
        self.last_sector
    }

    pub const fn pending_sector(&self) -> u64 {
        self.pending_sector
    }

    pub const fn pending_data_len(&self) -> u32 {
        self.pending_data_len
    }

    pub const fn request_queue_deferred(&self) -> bool {
        self.request_queue_deferred
    }

    pub const fn filesystem_parse_deferred(&self) -> bool {
        self.filesystem_parse_deferred
    }

    pub const fn multi_queue_deferred(&self) -> bool {
        self.multi_queue_deferred
    }

    pub const fn reset_remove_deferred(&self) -> bool {
        self.reset_remove_deferred
    }

    pub const fn block_read_served(&self) -> bool {
        self.block_read_served
    }

    pub const fn block_read_copies_to_caller(&self) -> bool {
        self.block_read_copies_to_caller
    }

    fn setup(&mut self) -> Result<(), VirtioBlkError> {
        if self.lifecycle.state() != State::Base {
            return Err(VirtioBlkError::DeviceNotReady);
        }
        self.queue
            .setup()
            .map_err(|_| VirtioBlkError::DeviceNotReady)?;
        self.single_request_queue = self.queue.state() == State::Ready;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
            .map_err(|_| VirtioBlkError::DeviceNotReady)
    }

    pub fn setup_real_transport(
        &mut self,
        kernel_image: &KernelImage,
        plic: &Plic,
        plic_irq_domain: &mut PlicIrqDomain,
    ) -> Result<(), VirtioBlkError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtioBlkError::DeviceNotReady);
        }
        let Some(transport) = self.virtio_device.mmio_transport() else {
            return Err(VirtioBlkError::TransportUnavailable);
        };
        if !transport.irq_handler_registered() {
            return Err(VirtioBlkError::TransportUnavailable);
        }
        if !self.virtio_device.reset_status()
            || !self.virtio_device.setup_driver_status()
            || !self.virtio_device.negotiate_features(0)
        {
            return Err(VirtioBlkError::TransportUnavailable);
        }
        let capacity = self
            .virtio_device
            .read_config_capacity()
            .ok_or(VirtioBlkError::CapacityUnavailable)?;
        if capacity == 0 {
            return Err(VirtioBlkError::CapacityUnavailable);
        }
        self.capacity = Some(capacity);
        self.capacity_read = self.virtio_device.config_capacity_read();
        self.capacity_nonzero = true;
        self.virtio_device
            .setup_queue(&mut self.queue, kernel_image, VIRTIO_BLK_QUEUE_INDEX)?;
        self.queue_setup_done = self.virtio_device.queue_setup_done();
        if !self.virtio_device.set_driver_ok() {
            return Err(VirtioBlkError::TransportUnavailable);
        }
        self.driver_ok = self.virtio_device.status_driver_ok();
        let Some(mut transport) = self.virtio_device.mmio_transport() else {
            return Err(VirtioBlkError::TransportUnavailable);
        };
        if !transport.enable_irq_source_gate(plic, plic_irq_domain) {
            return Err(VirtioBlkError::TransportUnavailable);
        }
        self.virtio_device.update_mmio_transport(transport);
        self.block_device
            .setup_from_virtio_blk(capacity)
            .map_err(|_| VirtioBlkError::DeviceNotReady)?;
        Ok(())
    }

    pub fn register_block_device(
        &mut self,
        registry: &mut BlockDeviceRegistry,
    ) -> Result<BlockDeviceRef, VirtioBlkError> {
        registry
            .register(&mut self.block_device)
            .map_err(|_| VirtioBlkError::BlockRegistry)
    }

    fn submit_ext2_superblock_read(
        &mut self,
        kernel_image: &KernelImage,
    ) -> Result<(), VirtioBlkError> {
        self.submit_read_sector(kernel_image, VIRTIO_BLK_EXT2_SUPERBLOCK_SECTOR)
    }

    pub fn probe_ext2_superblock_read(
        &mut self,
        kernel_image: &KernelImage,
    ) -> Result<(), VirtioBlkError> {
        let _owner = VIRTIO_BLK_SYNC_OWNER.lock();
        self.converge_pending_read()?;
        let start_completion_count = self.completion_count();
        self.submit_ext2_superblock_read(kernel_image)?;
        self.wait_for_current_read_completion(start_completion_count)
    }

    fn submit_read_sector(
        &mut self,
        kernel_image: &KernelImage,
        sector: u64,
    ) -> Result<(), VirtioBlkError> {
        if self.lifecycle.state() != State::Ready || !self.driver_ok {
            return Err(VirtioBlkError::DeviceNotReady);
        }
        if self.read_request_pending {
            return Err(VirtioBlkError::RequestPending);
        }

        let (header_phys, data_phys, status_phys) = prepare_read_request(kernel_image, sector)?;
        self.submit_prepared_read_chain(
            header_phys,
            data_phys,
            status_phys,
            sector,
            VIRTIO_BLK_READ_BUFFER_SIZE,
        )
    }

    pub fn note_mmio_irq(&mut self, status: u32) {
        self.mmio_irq_acknowledged = status & VIRTIO_MMIO_INT_VRING != 0;
    }

    pub fn complete_read_from_irq(&mut self) -> Result<(), VirtioBlkError> {
        if self.lifecycle.state() != State::Ready || !self.driver_ok {
            return Err(VirtioBlkError::DeviceNotReady);
        }
        self.irq_callback_invoked = true;
        self.completion_observed_by_irq = true;
        self.complete_read_request()?;
        self.irq_count = self.irq_count.saturating_add(1);
        Ok(())
    }

    pub fn poll_read_completion(&mut self) -> Result<(), VirtioBlkError> {
        if self.lifecycle.state() != State::Ready || !self.driver_ok {
            return Err(VirtioBlkError::DeviceNotReady);
        }
        self.completion_observed_by_sync_poll = true;
        self.complete_read_request()
    }

    fn converge_pending_read(&mut self) -> Result<(), VirtioBlkError> {
        if !self.read_request_pending {
            return Ok(());
        }
        let start_completion_count = self.completion_count();
        self.wait_for_current_read_completion(start_completion_count)
    }

    fn wait_for_current_read_completion(
        &mut self,
        start_completion_count: usize,
    ) -> Result<(), VirtioBlkError> {
        let mut remaining = VIRTIO_BLK_READ_WAIT_SPINS;
        while remaining != 0 {
            if !self.read_request_pending && self.completion_count() > start_completion_count {
                return Ok(());
            }
            match self.poll_read_completion() {
                Ok(()) => return Ok(()),
                Err(VirtioBlkError::Queue(VirtqueueError::NoUsedBuffer)) => {}
                Err(VirtioBlkError::NoRequestPending) => {
                    if self.completion_count() > start_completion_count {
                        return Ok(());
                    }
                }
                Err(error) => return Err(error),
            }
            core::hint::spin_loop();
            remaining -= 1;
        }
        Err(VirtioBlkError::TransportUnavailable)
    }

    fn complete_read_request(&mut self) -> Result<(), VirtioBlkError> {
        if !self.read_request_pending {
            return Err(VirtioBlkError::NoRequestPending);
        }

        let used = self.queue.get_buf_from_device()?;
        self.complete_gets_used_buffer = true;
        self.last_used_len = used.len();
        let Some(pending_token) = self.pending_token else {
            return Err(VirtioBlkError::NoRequestPending);
        };
        if used.token().head() != pending_token.head()
            || used.descriptor_count() != 3
            || used.len() > self.pending_data_len.saturating_add(1)
            || self.pending_sector != self.last_sector
        {
            return Err(VirtioBlkError::DataMismatch);
        }
        let status = read_request_status();
        self.last_status = status;
        if status != VIRTIO_BLK_S_OK {
            return Err(VirtioBlkError::BadStatus);
        }
        self.complete_status_ok = true;
        self.complete_data_nonzero = read_request_data_nonzero();
        self.complete_ext2_magic_observed = read_request_ext2_magic_observed();
        let required_superblock_probe =
            self.pending_sector == VIRTIO_BLK_EXT2_SUPERBLOCK_SECTOR && self.request_count == 1;
        if required_superblock_probe
            && (!self.complete_data_nonzero || !self.complete_ext2_magic_observed)
        {
            return Err(VirtioBlkError::DataMismatch);
        }
        self.pending_token = None;
        self.read_request_pending = false;
        self.read_request_done = true;
        self.completion_count = self.completion_count.saturating_add(1);
        Ok(())
    }
}

pub struct VirtioBlkLiveProvider {
    kernel_virt_start: usize,
    kernel_virt_offset: usize,
}

impl BlockDeviceProvider for VirtioBlkLiveProvider {
    fn read_block(
        &mut self,
        device_ref: BlockDeviceRef,
        sector: u64,
        buffer: &mut [u8],
    ) -> Result<usize, BlockDeviceError> {
        read_live_block(self, device_ref, sector, buffer)
    }
}

pub fn live_provider(kernel_image: &KernelImage) -> VirtioBlkLiveProvider {
    VirtioBlkLiveProvider {
        kernel_virt_start: kernel_image.virt_start(),
        kernel_virt_offset: kernel_image.virt_offset(),
    }
}

pub struct VirtioBlkRuntime {
    driver: VirtioBlkDriver,
    device: Option<VirtioBlkDevice>,
    live_device_ref: Option<VirtioDeviceRef>,
    real_probe_attempted: bool,
    real_probe_succeeded: bool,
}

#[allow(dead_code)]
impl VirtioBlkRuntime {
    pub const fn new() -> Self {
        Self {
            driver: VirtioBlkDriver::new(),
            device: None,
            live_device_ref: None,
            real_probe_attempted: false,
            real_probe_succeeded: false,
        }
    }

    pub const fn driver(&self) -> &VirtioBlkDriver {
        &self.driver
    }

    pub const fn device(&self) -> Option<&VirtioBlkDevice> {
        self.device.as_ref()
    }

    pub const fn real_probe_attempted(&self) -> bool {
        self.real_probe_attempted
    }

    pub const fn real_probe_succeeded(&self) -> bool {
        self.real_probe_succeeded
    }
}

pub fn setup_live_driver(
    runtime: &mut VirtioBlkRuntime,
    virtio_bus: &VirtioBus,
    block_registry: &mut BlockDeviceRegistry,
    kernel_image: &KernelImage,
    plic: &Plic,
    plic_irq_domain: &mut PlicIrqDomain,
) -> EventResult {
    VIRTIO_BLK_LIVE_PTR.store(runtime as *mut VirtioBlkRuntime as usize, Ordering::Release);
    runtime.real_probe_attempted = true;
    if runtime.driver.state() == State::Base {
        runtime.driver.setup(virtio_bus)?;
    }
    let Some(device) = virtio_bus.block_device() else {
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
    let mut block = runtime
        .driver
        .probe(device)
        .map_err(|_| live_setup_error())?;
    block
        .setup_real_transport(kernel_image, plic, plic_irq_domain)
        .map_err(|_| live_setup_error())?;
    runtime.live_device_ref = Some(device.device_ref());
    runtime.device = Some(block);
    let Some(block) = runtime.device.as_mut() else {
        return Err(live_setup_error());
    };
    block
        .register_block_device(block_registry)
        .map_err(|_| live_setup_error())?;
    block
        .probe_ext2_superblock_read(kernel_image)
        .map_err(|_| live_setup_error())?;
    runtime.real_probe_succeeded = true;
    crate::checkpoint::dispatch(
        crate::checkpoint::Checkpoint::VirtioBlkReady,
        crate::context::context_ref(),
    );
    dispatch_first_read_ready();
    Ok(())
}

pub fn live_runtime() -> Option<&'static VirtioBlkRuntime> {
    let ptr = VIRTIO_BLK_LIVE_PTR.load(Ordering::Acquire);
    if ptr == 0 {
        return None;
    }
    unsafe { (ptr as *const VirtioBlkRuntime).as_ref() }
}

fn live_runtime_mut() -> Option<&'static mut VirtioBlkRuntime> {
    let ptr = VIRTIO_BLK_LIVE_PTR.load(Ordering::Acquire);
    if ptr == 0 {
        return None;
    }
    unsafe { (ptr as *mut VirtioBlkRuntime).as_mut() }
}

pub fn live_mmio_transport() -> Option<VirtioMmioTransportDevice> {
    live_runtime()?.device()?.virtio_device().mmio_transport()
}

pub fn note_mmio_irq(status: u32) {
    VIRTIO_BLK_LAST_IRQ_STATUS.store(status, Ordering::Release);
    if VIRTIO_BLK_SYNC_OWNER.is_locked() {
        return;
    }
    let Some(runtime) = live_runtime_mut() else {
        return;
    };
    let Some(device) = runtime.device.as_mut() else {
        return;
    };
    device.note_mmio_irq(status);
}

pub fn handle_irq_completion() {
    VIRTIO_BLK_IRQ_COMPLETION_CALLS.fetch_add(1, Ordering::AcqRel);
    let Some(_owner) = VIRTIO_BLK_SYNC_OWNER.try_lock() else {
        return;
    };
    let Some(runtime) = live_runtime_mut() else {
        return;
    };
    let Some(device) = runtime.device.as_mut() else {
        return;
    };
    if device.complete_read_from_irq().is_err() {
        return;
    }
    dispatch_first_read_ready();
}

#[allow(dead_code)]
pub fn last_irq_status() -> u32 {
    VIRTIO_BLK_LAST_IRQ_STATUS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn irq_completion_calls() -> usize {
    VIRTIO_BLK_IRQ_COMPLETION_CALLS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn read_ready_checkpoints() -> usize {
    VIRTIO_BLK_READ_READY_CHECKPOINTS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn live_read_submitted_checkpoints() -> usize {
    VIRTIO_BLK_LIVE_READ_SUBMITTED_CHECKPOINTS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn live_read_completed_checkpoints() -> usize {
    VIRTIO_BLK_LIVE_READ_COMPLETED_CHECKPOINTS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn read_buffer_nonzero() -> bool {
    read_request_data_nonzero()
}

#[allow(dead_code)]
pub fn read_buffer_ext2_magic_observed() -> bool {
    read_request_ext2_magic_observed()
}

fn prepare_read_request(
    kernel_image: &KernelImage,
    sector: u64,
) -> Result<(usize, usize, usize), VirtioBlkError> {
    let request = unsafe {
        (&raw mut VIRTIO_BLK_READ_REQUEST)
            .as_mut()
            .ok_or(VirtioBlkError::InvalidBuffer)?
    };
    request.header = VirtioBlkOutHdr {
        request_type: VIRTIO_BLK_T_IN,
        reserved: 0,
        sector,
    };
    request.data.fill(0);
    request.status = 0xff;
    let header_virt = core::ptr::addr_of!(request.header) as usize;
    let data_virt = request.data.as_ptr() as usize;
    let status_virt = core::ptr::addr_of!(request.status) as usize;
    let header_phys = kernel_image
        .runtime_to_phys(header_virt)
        .ok_or(VirtioBlkError::InvalidBuffer)?;
    let data_phys = kernel_image
        .runtime_to_phys(data_virt)
        .ok_or(VirtioBlkError::InvalidBuffer)?;
    let status_phys = kernel_image
        .runtime_to_phys(status_virt)
        .ok_or(VirtioBlkError::InvalidBuffer)?;
    Ok((header_phys, data_phys, status_phys))
}

fn prepare_read_request_with_mapping(
    provider: &VirtioBlkLiveProvider,
    sector: u64,
) -> Result<(usize, usize, usize), VirtioBlkError> {
    let request = unsafe {
        (&raw mut VIRTIO_BLK_READ_REQUEST)
            .as_mut()
            .ok_or(VirtioBlkError::InvalidBuffer)?
    };
    request.header = VirtioBlkOutHdr {
        request_type: VIRTIO_BLK_T_IN,
        reserved: 0,
        sector,
    };
    request.data.fill(0);
    request.status = 0xff;
    let header_virt = core::ptr::addr_of!(request.header) as usize;
    let data_virt = request.data.as_ptr() as usize;
    let status_virt = core::ptr::addr_of!(request.status) as usize;
    let header_phys = provider
        .runtime_to_phys(header_virt)
        .ok_or(VirtioBlkError::InvalidBuffer)?;
    let data_phys = provider
        .runtime_to_phys(data_virt)
        .ok_or(VirtioBlkError::InvalidBuffer)?;
    let status_phys = provider
        .runtime_to_phys(status_virt)
        .ok_or(VirtioBlkError::InvalidBuffer)?;
    Ok((header_phys, data_phys, status_phys))
}

fn read_live_block(
    provider: &VirtioBlkLiveProvider,
    device_ref: BlockDeviceRef,
    sector: u64,
    buffer: &mut [u8],
) -> Result<usize, BlockDeviceError> {
    if buffer.is_empty() {
        return Err(BlockDeviceError::DeviceNotReady);
    }

    let _owner = VIRTIO_BLK_SYNC_OWNER.lock();
    let start_completion_count = {
        let runtime = live_runtime_mut().ok_or(BlockDeviceError::ProviderUnavailable)?;
        let device = runtime
            .device
            .as_mut()
            .ok_or(BlockDeviceError::ProviderUnavailable)?;
        if device.block_device.device_ref() != Some(device_ref) || !device.block_device.registered()
        {
            return Err(BlockDeviceError::ProviderUnavailable);
        }
        device
            .converge_pending_read()
            .map_err(block_error_from_virtio)?;
        let start_completion_count = device.completion_count();
        let request_len = core::cmp::min(buffer.len(), VIRTIO_BLK_READ_BUFFER_SIZE);
        device
            .submit_read_sector_with_mapping(provider, sector, request_len)
            .map_err(block_error_from_virtio)?;
        start_completion_count
    };
    VIRTIO_BLK_LIVE_READ_SUBMITTED_CHECKPOINTS.fetch_add(1, Ordering::AcqRel);
    crate::checkpoint::dispatch(
        crate::checkpoint::Checkpoint::VirtioBlkLiveReadSubmitted,
        crate::context::context_ref(),
    );
    {
        let runtime = live_runtime_mut().ok_or(BlockDeviceError::ProviderUnavailable)?;
        let device = runtime
            .device
            .as_mut()
            .ok_or(BlockDeviceError::ProviderUnavailable)?;
        device
            .wait_for_current_read_completion(start_completion_count)
            .map_err(block_error_from_virtio)?;
    }
    VIRTIO_BLK_LIVE_READ_COMPLETED_CHECKPOINTS.fetch_add(1, Ordering::AcqRel);
    crate::checkpoint::dispatch(
        crate::checkpoint::Checkpoint::VirtioBlkLiveReadCompleted,
        crate::context::context_ref(),
    );

    let data_len = last_read_data_len().ok_or(BlockDeviceError::ProviderUnavailable)?;
    let len = copy_read_request_data(buffer, data_len);
    if len == 0 {
        return Err(BlockDeviceError::EmptyRead);
    }
    let nonzero = buffer[..len].iter().any(|byte| *byte != 0);

    let runtime = live_runtime_mut().ok_or(BlockDeviceError::ProviderUnavailable)?;
    let device = runtime
        .device
        .as_mut()
        .ok_or(BlockDeviceError::ProviderUnavailable)?;
    if device.block_device.device_ref() != Some(device_ref) {
        return Err(BlockDeviceError::ProviderUnavailable);
    }
    device.block_read_served = true;
    device.block_read_copies_to_caller = true;
    device.block_device.record_read(sector, len, nonzero);
    Ok(len)
}

impl VirtioBlkDevice {
    fn submit_read_sector_with_mapping(
        &mut self,
        provider: &VirtioBlkLiveProvider,
        sector: u64,
        data_len: usize,
    ) -> Result<(), VirtioBlkError> {
        if self.lifecycle.state() != State::Ready || !self.driver_ok {
            return Err(VirtioBlkError::DeviceNotReady);
        }
        if self.read_request_pending {
            return Err(VirtioBlkError::RequestPending);
        }
        if data_len == 0 || data_len > VIRTIO_BLK_READ_BUFFER_SIZE {
            return Err(VirtioBlkError::InvalidBuffer);
        }

        let (header_phys, data_phys, status_phys) =
            prepare_read_request_with_mapping(provider, sector)?;
        self.submit_prepared_read_chain(header_phys, data_phys, status_phys, sector, data_len)
    }

    fn submit_prepared_read_chain(
        &mut self,
        header_phys: usize,
        data_phys: usize,
        status_phys: usize,
        sector: u64,
        data_len: usize,
    ) -> Result<(), VirtioBlkError> {
        let header_len = u32::try_from(core::mem::size_of::<VirtioBlkOutHdr>())
            .map_err(|_| VirtioBlkError::InvalidBuffer)?;
        let data_len = u32::try_from(data_len).map_err(|_| VirtioBlkError::InvalidBuffer)?;
        let token = self.queue.add_chain(&[
            VirtqueueDescriptorSpec::out(header_phys, header_len),
            VirtqueueDescriptorSpec::inbuf(data_phys, data_len),
            VirtqueueDescriptorSpec::inbuf(status_phys, 1),
        ])?;
        self.pending_token = Some(token);
        self.pending_sector = sector;
        self.pending_data_len = data_len;
        self.read_header_prepared = true;
        self.read_data_buffer_prepared = true;
        self.read_status_buffer_prepared = true;
        self.read_request_pending = true;
        self.read_request_submitted = true;
        self.last_sector = sector;
        self.request_count = self.request_count.saturating_add(1);
        if self.virtio_device.notify_queue(&mut self.queue).is_err() {
            self.pending_token = None;
            self.pending_sector = 0;
            self.pending_data_len = 0;
            self.read_request_pending = false;
            return Err(VirtioBlkError::TransportUnavailable);
        }
        self.read_request_notified = true;
        self.notify_count = self.notify_count.saturating_add(1);
        Ok(())
    }
}

impl VirtioBlkLiveProvider {
    fn runtime_to_phys(&self, addr: usize) -> Option<usize> {
        if addr >= self.kernel_virt_start {
            addr.checked_sub(self.kernel_virt_offset)
        } else {
            Some(addr)
        }
    }
}

fn last_read_data_len() -> Option<usize> {
    let runtime = live_runtime()?;
    let device = runtime.device()?;
    let len = usize::try_from(device.last_used_len()).ok()?;
    len.checked_sub(1)
}

fn copy_read_request_data(buffer: &mut [u8], data_len: usize) -> usize {
    unsafe {
        (&raw const VIRTIO_BLK_READ_REQUEST)
            .as_ref()
            .map_or(0, |request| {
                let len =
                    core::cmp::min(buffer.len(), core::cmp::min(data_len, request.data.len()));
                buffer[..len].copy_from_slice(&request.data[..len]);
                len
            })
    }
}

fn block_error_from_virtio(error: VirtioBlkError) -> BlockDeviceError {
    match error {
        VirtioBlkError::DeviceNotReady
        | VirtioBlkError::InvalidBuffer
        | VirtioBlkError::RequestPending => BlockDeviceError::DeviceNotReady,
        VirtioBlkError::NoRequestPending
        | VirtioBlkError::BadStatus
        | VirtioBlkError::DataMismatch => BlockDeviceError::ProviderUnavailable,
        _ => BlockDeviceError::ProviderUnavailable,
    }
}

fn read_request_status() -> u8 {
    unsafe {
        (&raw const VIRTIO_BLK_READ_REQUEST)
            .as_ref()
            .map_or(0xff, |request| request.status)
    }
}

fn read_request_data_nonzero() -> bool {
    unsafe {
        (&raw const VIRTIO_BLK_READ_REQUEST)
            .as_ref()
            .is_some_and(|request| request.data.iter().any(|byte| *byte != 0))
    }
}

fn read_request_ext2_magic_observed() -> bool {
    unsafe {
        (&raw const VIRTIO_BLK_READ_REQUEST)
            .as_ref()
            .is_some_and(|request| {
                let offset = EXT2_SUPER_MAGIC_OFFSET_IN_SECTOR;
                offset + 1 < request.data.len()
                    && u16::from_le_bytes([request.data[offset], request.data[offset + 1]])
                        == EXT2_SUPER_MAGIC
            })
    }
}

fn dispatch_first_read_ready() {
    if VIRTIO_BLK_READ_READY_CHECKPOINTS
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        crate::checkpoint::dispatch(
            crate::checkpoint::Checkpoint::VirtioBlkReadReady,
            crate::context::context_ref(),
        );
    }
}

fn live_setup_error() -> super::state::EventError {
    failed_condition(
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        State::Ready,
    )
    .unwrap_err()
}
