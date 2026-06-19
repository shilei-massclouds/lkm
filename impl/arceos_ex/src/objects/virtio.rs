use super::{
    device::DeviceRef,
    device_tree::DeviceNodeId,
    initcall::PlatformBus,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    virtio_mmio::{VirtioMmioTransportDevice, VIRTIO_ID_RNG},
};
use crate::trace::Checkpoint;
use alloc::vec::Vec;

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum VirtioTransportKind {
    Mmio,
}

#[derive(Clone, Copy)]
pub enum VirtioTransportDevice {
    Mmio(VirtioMmioTransportDevice),
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum VirtioDeviceState {
    Discovered,
    Registered,
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub struct VirtioDeviceRef {
    index: usize,
}

#[allow(dead_code)]
impl VirtioDeviceRef {
    pub const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct VirtioDevice {
    device_ref: VirtioDeviceRef,
    device_id: u32,
    vendor_id: u32,
    state: VirtioDeviceState,
    transport: VirtioTransportDevice,
    status_reset: bool,
    status_acknowledged: bool,
    status_driver_seen: bool,
    status_features_ok: bool,
    status_driver_ok: bool,
    features_read: bool,
    driver_features_written: bool,
    feature_negotiation_done: bool,
    config_access_ready: bool,
    config_capacity_read: bool,
    config_capacity: Option<u64>,
    config_read_deferred: bool,
    single_queue_discovered: bool,
    queue_setup_done: bool,
    queue_notify_done: bool,
    multi_queue_deferred: bool,
    reset_remove_deferred: bool,
}

#[allow(dead_code)]
impl VirtioDevice {
    fn from_mmio_transport(
        device_ref: VirtioDeviceRef,
        transport: VirtioMmioTransportDevice,
    ) -> Self {
        Self {
            device_ref,
            device_id: transport.device_id(),
            vendor_id: transport.vendor_id(),
            state: VirtioDeviceState::Registered,
            transport: VirtioTransportDevice::Mmio(transport),
            status_reset: false,
            status_acknowledged: false,
            status_driver_seen: false,
            status_features_ok: false,
            status_driver_ok: false,
            features_read: false,
            driver_features_written: false,
            feature_negotiation_done: false,
            config_access_ready: true,
            config_capacity_read: false,
            config_capacity: None,
            config_read_deferred: true,
            single_queue_discovered: false,
            queue_setup_done: false,
            queue_notify_done: false,
            multi_queue_deferred: true,
            reset_remove_deferred: true,
        }
    }

    pub const fn device_ref(self) -> VirtioDeviceRef {
        self.device_ref
    }

    pub const fn device_id(self) -> u32 {
        self.device_id
    }

    pub const fn vendor_id(self) -> u32 {
        self.vendor_id
    }

    pub const fn state(self) -> VirtioDeviceState {
        self.state
    }

    pub const fn transport_kind(self) -> VirtioTransportKind {
        match self.transport {
            VirtioTransportDevice::Mmio(_) => VirtioTransportKind::Mmio,
        }
    }

    pub const fn transport_is_mmio(self) -> bool {
        matches!(self.transport, VirtioTransportDevice::Mmio(_))
    }

    pub const fn is_rng(self) -> bool {
        self.device_id == VIRTIO_ID_RNG
    }

    pub const fn status_reset(self) -> bool {
        self.status_reset
    }

    pub const fn status_acknowledged(self) -> bool {
        self.status_acknowledged
    }

    pub const fn status_driver_seen(self) -> bool {
        self.status_driver_seen
    }

    pub const fn status_features_ok(self) -> bool {
        self.status_features_ok
    }

    pub const fn status_driver_ok(self) -> bool {
        self.status_driver_ok
    }

    pub const fn features_read(self) -> bool {
        self.features_read
    }

    pub const fn driver_features_written(self) -> bool {
        self.driver_features_written
    }

    pub const fn feature_negotiation_done(self) -> bool {
        self.feature_negotiation_done
    }

    pub const fn config_access_ready(self) -> bool {
        self.config_access_ready
    }

    pub const fn config_capacity_read(self) -> bool {
        self.config_capacity_read
    }

    pub const fn config_capacity(self) -> Option<u64> {
        self.config_capacity
    }

    pub const fn config_read_deferred(self) -> bool {
        self.config_read_deferred
    }

    pub const fn single_queue_discovered(self) -> bool {
        self.single_queue_discovered
    }

    pub const fn queue_setup_done(self) -> bool {
        self.queue_setup_done
    }

    pub const fn queue_notify_done(self) -> bool {
        self.queue_notify_done
    }

    pub const fn multi_queue_deferred(self) -> bool {
        self.multi_queue_deferred
    }

    pub const fn reset_remove_deferred(self) -> bool {
        self.reset_remove_deferred
    }

    pub const fn platform_device_ref(self) -> DeviceRef {
        match self.transport {
            VirtioTransportDevice::Mmio(transport) => transport.device_ref(),
        }
    }

    pub const fn node_id(self) -> DeviceNodeId {
        match self.transport {
            VirtioTransportDevice::Mmio(transport) => transport.node_id(),
        }
    }

    pub const fn mmio_transport(self) -> Option<VirtioMmioTransportDevice> {
        match self.transport {
            VirtioTransportDevice::Mmio(transport) => Some(transport),
        }
    }

    pub fn update_mmio_transport(&mut self, transport: VirtioMmioTransportDevice) -> bool {
        if !self.transport_is_mmio() || self.platform_device_ref() != transport.device_ref() {
            return false;
        }
        self.transport = VirtioTransportDevice::Mmio(transport);
        true
    }

    pub fn reset_status(&mut self) -> bool {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !super::virtio_mmio::reset_status(&mut transport) {
            return false;
        }
        self.status_reset = transport.status_reset_written();
        self.status_acknowledged = false;
        self.status_driver_seen = false;
        self.status_features_ok = false;
        self.status_driver_ok = false;
        self.transport = VirtioTransportDevice::Mmio(transport);
        true
    }

    pub fn setup_driver_status(&mut self) -> bool {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !self.status_reset || !super::virtio_mmio::setup_driver_status(&mut transport) {
            return false;
        }
        self.status_acknowledged = transport.status_acknowledge_written();
        self.status_driver_seen = transport.status_driver_written();
        self.transport = VirtioTransportDevice::Mmio(transport);
        true
    }

    pub fn negotiate_features(&mut self, driver_features: u64) -> bool {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !self.status_driver_seen
            || !super::virtio_mmio::negotiate_features(&mut transport, driver_features)
        {
            return false;
        }
        self.features_read = transport.device_features_read();
        self.driver_features_written = transport.driver_features_written();
        self.status_features_ok =
            transport.version() == 1 || transport.status_features_ok_written();
        self.feature_negotiation_done = transport.feature_negotiation_done();
        self.transport = VirtioTransportDevice::Mmio(transport);
        true
    }

    pub fn read_config_capacity(&mut self) -> Option<u64> {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !self.config_access_ready || !self.feature_negotiation_done {
            return None;
        }
        let capacity = super::virtio_mmio::read_config_capacity(&mut transport)?;
        self.config_capacity_read = transport.config_capacity_read();
        self.config_capacity = transport.config_capacity();
        self.config_read_deferred = false;
        self.transport = VirtioTransportDevice::Mmio(transport);
        Some(capacity)
    }

    pub fn setup_queue(
        &mut self,
        queue: &mut super::virtio_ring::VirtQueue,
        kernel_image: &super::kernel_image::KernelImage,
        queue_index: u16,
    ) -> Result<(), super::virtio_ring::VirtqueueError> {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !self.feature_negotiation_done {
            return Err(super::virtio_ring::VirtqueueError::TransportUnavailable);
        }
        queue.setup_real_mmio(kernel_image, &mut transport, queue_index)?;
        self.single_queue_discovered = queue.queue_num_max_observed();
        self.queue_setup_done = transport.queue_setup_done();
        self.transport = VirtioTransportDevice::Mmio(transport);
        Ok(())
    }

    pub fn set_driver_ok(&mut self) -> bool {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !self.queue_setup_done || !super::virtio_mmio::set_driver_ok(&mut transport) {
            return false;
        }
        self.status_driver_ok = transport.status_driver_ok_written();
        self.transport = VirtioTransportDevice::Mmio(transport);
        true
    }

    pub fn notify_queue(
        &mut self,
        queue: &mut super::virtio_ring::VirtQueue,
    ) -> Result<(), super::virtio_ring::VirtqueueError> {
        let VirtioTransportDevice::Mmio(mut transport) = self.transport;
        if !self.status_driver_ok {
            return Err(super::virtio_ring::VirtqueueError::TransportUnavailable);
        }
        queue.kick_mmio(&mut transport)?;
        self.queue_notify_done = transport.queue_notify_written();
        self.transport = VirtioTransportDevice::Mmio(transport);
        Ok(())
    }
}

pub struct VirtioBus {
    lifecycle: Lifecycle,
    registered: bool,
    devices: Vec<VirtioDevice>,
    mmio_transport_count: usize,
    rng_device_count: usize,
}

#[allow(dead_code)]
impl VirtioBus {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registered: false,
            devices: Vec::new(),
            mmio_transport_count: 0,
            rng_device_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub const fn mmio_transport_count(&self) -> usize {
        self.mmio_transport_count
    }

    pub const fn rng_device_count(&self) -> usize {
        self.rng_device_count
    }

    pub fn device(&self, device_ref: VirtioDeviceRef) -> Option<VirtioDevice> {
        self.devices.get(device_ref.index()).copied()
    }

    pub fn device_at(&self, index: usize) -> Option<VirtioDevice> {
        self.devices.get(index).copied()
    }

    pub fn rng_device(&self) -> Option<VirtioDevice> {
        self.devices.iter().copied().find(|device| device.is_rng())
    }

    pub fn contains_platform_device(&self, platform_device_ref: DeviceRef) -> bool {
        self.devices
            .iter()
            .any(|device| device.platform_device_ref() == platform_device_ref)
    }

    pub fn setup(&mut self, platform_bus: &PlatformBus) -> EventResult {
        if self.lifecycle.state() != State::Base
            || platform_bus.state() != State::Ready
            || !platform_bus.registered()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.registered = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VirtioBusReady,
        )
    }

    pub fn register_mmio_device(
        &mut self,
        transport: VirtioMmioTransportDevice,
    ) -> Option<VirtioDeviceRef> {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !transport.ready_for_virtio_core()
            || self.contains_platform_device(transport.device_ref())
        {
            return None;
        }

        let device_ref = VirtioDeviceRef::new(self.devices.len());
        let device = VirtioDevice::from_mmio_transport(device_ref, transport);
        self.devices.push(device);
        self.mmio_transport_count = self.mmio_transport_count.saturating_add(1);
        if device.is_rng() {
            self.rng_device_count = self.rng_device_count.saturating_add(1);
        }
        crate::trace::checkpoint(Checkpoint::VirtioBusDeviceAdded);
        Some(device_ref)
    }
}
