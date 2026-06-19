use super::state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State};
use alloc::vec::Vec;

pub const VIRTBLK_MAJOR: u32 = 254;
pub const VIRTBLK_FIRST_MINOR: u32 = 0;
#[allow(dead_code)]
pub const VIRTBLK_MINORS: u32 = 1 << 5;

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum BlockDeviceProviderKind {
    VirtioBlk,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct BlockDeviceRef {
    index: usize,
}

impl BlockDeviceRef {
    pub const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DevT {
    major: u32,
    minor: u32,
}

#[allow(dead_code)]
impl DevT {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub const fn major(self) -> u32 {
        self.major
    }

    pub const fn minor(self) -> u32 {
        self.minor
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum BlockDeviceError {
    CoreNotReady,
    DeviceNotReady,
    DuplicateDev,
    NoDevice,
}

pub struct BlockDevice {
    lifecycle: Lifecycle,
    device_ref: Option<BlockDeviceRef>,
    name: [u8; 16],
    name_len: usize,
    provider_kind: BlockDeviceProviderKind,
    capacity_sectors: u64,
    sector_size: u32,
    devt: Option<DevT>,
    registered: bool,
    default_device: bool,
}

#[allow(dead_code)]
impl BlockDevice {
    pub const fn new_virtio_blk(index: usize) -> Self {
        let mut name = [0u8; 16];
        name[0] = b'v';
        name[1] = b'd';
        name[2] = b'a' + (index as u8);
        Self {
            lifecycle: Lifecycle::new(State::Base),
            device_ref: None,
            name,
            name_len: 3,
            provider_kind: BlockDeviceProviderKind::VirtioBlk,
            capacity_sectors: 0,
            sector_size: 512,
            devt: None,
            registered: false,
            default_device: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn device_ref(&self) -> Option<BlockDeviceRef> {
        self.device_ref
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub const fn name_bound(&self) -> bool {
        self.name_len != 0
    }

    #[allow(dead_code)]
    pub const fn provider_kind(&self) -> BlockDeviceProviderKind {
        self.provider_kind
    }

    pub const fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    pub const fn sector_size(&self) -> u32 {
        self.sector_size
    }

    pub const fn capacity_bound(&self) -> bool {
        self.capacity_sectors != 0 && self.sector_size != 0
    }

    pub const fn devt(&self) -> Option<DevT> {
        self.devt
    }

    pub const fn major_minor_bound(&self) -> bool {
        self.devt.is_some()
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    pub const fn default_device(&self) -> bool {
        self.default_device
    }

    pub fn setup_from_virtio_blk(&mut self, capacity_sectors: u64) -> EventResult {
        if self.lifecycle.state() != State::Base || capacity_sectors == 0 || !self.name_bound() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.capacity_sectors = capacity_sectors;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn mark_registered(&mut self, device_ref: BlockDeviceRef, devt: DevT) {
        self.device_ref = Some(device_ref);
        self.devt = Some(devt);
        self.registered = true;
    }

    fn mark_default(&mut self) {
        self.default_device = true;
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct BlockDeviceRegistryEntry {
    device_ref: BlockDeviceRef,
    name: [u8; 16],
    name_len: usize,
    provider_kind: BlockDeviceProviderKind,
    capacity_sectors: u64,
    sector_size: u32,
    devt: DevT,
    registered: bool,
    default_device: bool,
}

#[allow(dead_code)]
impl BlockDeviceRegistryEntry {
    fn from_device(device_ref: BlockDeviceRef, device: &BlockDevice) -> Option<Self> {
        Some(Self {
            device_ref,
            name: device.name,
            name_len: device.name_len,
            provider_kind: device.provider_kind,
            capacity_sectors: device.capacity_sectors,
            sector_size: device.sector_size,
            devt: device.devt?,
            registered: true,
            default_device: false,
        })
    }

    pub const fn device_ref(&self) -> BlockDeviceRef {
        self.device_ref
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub const fn provider_kind(&self) -> BlockDeviceProviderKind {
        self.provider_kind
    }

    pub const fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    pub const fn sector_size(&self) -> u32 {
        self.sector_size
    }

    pub const fn devt(&self) -> DevT {
        self.devt
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    pub const fn default_device(&self) -> bool {
        self.default_device
    }

    fn mark_default(&mut self) {
        self.default_device = true;
    }
}

pub struct BlockDeviceRegistry {
    lifecycle: Lifecycle,
    registry_ready: bool,
    major_allocator_ready: bool,
    default_device_slot_ready: bool,
    request_queue_deferred: bool,
    bio_page_cache_deferred: bool,
    partition_scan_deferred: bool,
    dev_node_deferred: bool,
    devices: Vec<BlockDeviceRegistryEntry>,
    default_device: Option<BlockDeviceRef>,
    register_blkdev_called: bool,
    register_blkdev_returned_major: bool,
    device_add_disk_called: bool,
    device_add_disk_return_zero: bool,
    major_minor_lookup_ready: bool,
    register_count: usize,
}

#[allow(dead_code)]
impl BlockDeviceRegistry {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registry_ready: false,
            major_allocator_ready: false,
            default_device_slot_ready: false,
            request_queue_deferred: false,
            bio_page_cache_deferred: false,
            partition_scan_deferred: false,
            dev_node_deferred: false,
            devices: Vec::new(),
            default_device: None,
            register_blkdev_called: false,
            register_blkdev_returned_major: false,
            device_add_disk_called: false,
            device_add_disk_return_zero: false,
            major_minor_lookup_ready: false,
            register_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn registry_ready(&self) -> bool {
        self.registry_ready
    }

    pub const fn major_allocator_ready(&self) -> bool {
        self.major_allocator_ready
    }

    pub const fn default_device_slot_ready(&self) -> bool {
        self.default_device_slot_ready
    }

    pub const fn request_queue_deferred(&self) -> bool {
        self.request_queue_deferred
    }

    pub const fn bio_page_cache_deferred(&self) -> bool {
        self.bio_page_cache_deferred
    }

    pub const fn partition_scan_deferred(&self) -> bool {
        self.partition_scan_deferred
    }

    pub const fn dev_node_deferred(&self) -> bool {
        self.dev_node_deferred
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub const fn default_device(&self) -> Option<BlockDeviceRef> {
        self.default_device
    }

    pub fn device(&self, device_ref: BlockDeviceRef) -> Option<&BlockDeviceRegistryEntry> {
        self.devices.get(device_ref.index())
    }

    pub fn default_entry(&self) -> Option<&BlockDeviceRegistryEntry> {
        self.device(self.default_device?)
    }

    pub fn lookup(&self, devt: DevT) -> Option<&BlockDeviceRegistryEntry> {
        self.devices
            .iter()
            .find(|entry| entry.registered() && entry.devt() == devt)
    }

    pub const fn register_blkdev_called(&self) -> bool {
        self.register_blkdev_called
    }

    pub const fn register_blkdev_returned_major(&self) -> bool {
        self.register_blkdev_returned_major
    }

    pub const fn device_add_disk_called(&self) -> bool {
        self.device_add_disk_called
    }

    pub const fn device_add_disk_return_zero(&self) -> bool {
        self.device_add_disk_return_zero
    }

    pub const fn major_minor_lookup_ready(&self) -> bool {
        self.major_minor_lookup_ready
    }

    pub const fn register_count(&self) -> usize {
        self.register_count
    }

    pub fn setup(
        &mut self,
        driver_core_base: &crate::objects::initcall::DriverCoreBase,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base || driver_core_base.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.registry_ready = true;
        self.major_allocator_ready = true;
        self.default_device_slot_ready = true;
        self.request_queue_deferred = true;
        self.bio_page_cache_deferred = true;
        self.partition_scan_deferred = true;
        self.dev_node_deferred = true;
        self.major_minor_lookup_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn register(
        &mut self,
        device: &mut BlockDevice,
    ) -> Result<BlockDeviceRef, BlockDeviceError> {
        if self.lifecycle.state() != State::Ready {
            return Err(BlockDeviceError::CoreNotReady);
        }
        if device.state() != State::Ready || !device.name_bound() || !device.capacity_bound() {
            return Err(BlockDeviceError::DeviceNotReady);
        }

        let devt = DevT::new(VIRTBLK_MAJOR, VIRTBLK_FIRST_MINOR);
        if self.lookup(devt).is_some() {
            return Err(BlockDeviceError::DuplicateDev);
        }

        self.register_blkdev_called = true;
        self.register_blkdev_returned_major = true;
        self.device_add_disk_called = true;
        let device_ref = BlockDeviceRef::new(self.devices.len());
        device.mark_registered(device_ref, devt);
        let Some(mut entry) = BlockDeviceRegistryEntry::from_device(device_ref, device) else {
            return Err(BlockDeviceError::DeviceNotReady);
        };
        self.device_add_disk_return_zero = true;
        if self.default_device.is_none() {
            device.mark_default();
            entry.mark_default();
            self.default_device = Some(device_ref);
        }
        self.devices.push(entry);
        self.register_count = self.register_count.saturating_add(1);
        Ok(device_ref)
    }
}
