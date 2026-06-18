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
