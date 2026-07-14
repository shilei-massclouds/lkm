use alloc::{boxed::Box, vec::Vec};
use core::pin::Pin;

use super::device_tree::{DeviceNodeId, DeviceTree};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DeviceRef {
    index: usize,
}

impl DeviceRef {
    pub const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

pub struct Device {
    node_id: DeviceNodeId,
    bus_bound: bool,
    registered: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Device {
    pub const fn new(node_id: DeviceNodeId) -> Self {
        Self {
            node_id,
            bus_bound: false,
            registered: false,
        }
    }

    pub const fn node_id(&self) -> DeviceNodeId {
        self.node_id
    }

    pub const fn bus_bound(&self) -> bool {
        self.bus_bound
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    fn bind_platform_bus(&mut self) {
        self.bus_bound = true;
    }

    fn register(&mut self) {
        self.registered = true;
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct PlatformDevice {
    dev: Device,
    id_bound: bool,
    resources_bound: bool,
    added: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl PlatformDevice {
    pub fn from_node_id(device_tree: &DeviceTree, node_id: DeviceNodeId) -> Option<Self> {
        device_tree.node(node_id)?;
        let mut dev = Device::new(node_id);
        dev.bind_platform_bus();
        dev.register();

        Some(Self {
            dev,
            id_bound: true,
            resources_bound: true,
            added: true,
        })
    }

    pub const fn dev(&self) -> &Device {
        &self.dev
    }

    pub const fn id_bound(&self) -> bool {
        self.id_bound
    }

    pub const fn resources_bound(&self) -> bool {
        self.resources_bound
    }

    pub const fn added(&self) -> bool {
        self.added
    }
}

pub struct PlatformDeviceStorage {
    devices: Vec<Pin<Box<PlatformDevice>>>,
}

impl PlatformDeviceStorage {
    pub const fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn push(&mut self, device: PlatformDevice) -> DeviceRef {
        let index = self.devices.len();
        self.devices.push(Box::pin(device));
        DeviceRef::new(index)
    }

    pub fn get(&self, device_ref: DeviceRef) -> Option<&PlatformDevice> {
        self.devices
            .get(device_ref.index())
            .map(|device| device.as_ref().get_ref())
    }

    pub fn contains(&self, device_ref: DeviceRef) -> bool {
        self.get(device_ref).is_some()
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }
}
