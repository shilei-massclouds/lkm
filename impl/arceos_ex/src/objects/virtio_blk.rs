use super::{
    kernel_image::KernelImage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    virtio::{VirtioBus, VirtioDevice, VirtioDeviceRef},
    virtio_mmio::VIRTIO_ID_BLOCK,
    virtio_ring::{VirtQueue, VirtqueueError},
};

const VIRTIO_BLK_QUEUE_SIZE: u16 = 8;
const VIRTIO_BLK_QUEUE_INDEX: u16 = 0;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VirtioBlkError {
    DriverNotReady,
    DeviceNotReady,
    UnsupportedDevice,
    CapacityUnavailable,
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
    single_request_queue: bool,
    capacity: Option<u64>,
    capacity_read: bool,
    capacity_nonzero: bool,
    queue_setup_done: bool,
    driver_ok: bool,
    request_io_deferred: bool,
    block_layer_deferred: bool,
    multi_queue_deferred: bool,
    reset_remove_deferred: bool,
}

#[allow(dead_code)]
impl VirtioBlkDevice {
    fn new(virtio_device: VirtioDevice) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            virtio_device,
            queue: VirtQueue::new(VIRTIO_BLK_QUEUE_SIZE),
            single_request_queue: false,
            capacity: None,
            capacity_read: false,
            capacity_nonzero: false,
            queue_setup_done: false,
            driver_ok: false,
            request_io_deferred: true,
            block_layer_deferred: true,
            multi_queue_deferred: true,
            reset_remove_deferred: true,
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

    pub const fn request_io_deferred(&self) -> bool {
        self.request_io_deferred
    }

    pub const fn block_layer_deferred(&self) -> bool {
        self.block_layer_deferred
    }

    pub const fn multi_queue_deferred(&self) -> bool {
        self.multi_queue_deferred
    }

    pub const fn reset_remove_deferred(&self) -> bool {
        self.reset_remove_deferred
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
        Ok(())
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
    kernel_image: &KernelImage,
) -> EventResult {
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
        .setup_real_transport(kernel_image)
        .map_err(|_| live_setup_error())?;
    runtime.live_device_ref = Some(device.device_ref());
    runtime.device = Some(block);
    runtime.real_probe_succeeded = true;
    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::VirtioBlkReady,
        crate::context::context_ref(),
    );
    Ok(())
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
