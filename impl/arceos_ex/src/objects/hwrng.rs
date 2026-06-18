use super::state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State};
use alloc::vec::Vec;

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum HwRngProviderKind {
    VirtioRng,
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub struct HwRngDeviceRef {
    index: usize,
}

impl HwRngDeviceRef {
    pub const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum HwRngError {
    CoreNotReady,
    DeviceNotReady,
    DuplicateName,
    NoCurrentDevice,
    ProviderUnavailable,
    EmptyRead,
}

pub trait HwRngProvider {
    fn read_hwrng(
        &mut self,
        device_ref: HwRngDeviceRef,
        buffer: &mut [u8],
        wait: bool,
    ) -> Result<usize, HwRngError>;
}

pub struct HwRngDevice {
    lifecycle: Lifecycle,
    device_ref: Option<HwRngDeviceRef>,
    name: [u8; 25],
    name_len: usize,
    provider_kind: HwRngProviderKind,
    read_callback_bound: bool,
    cleanup_callback_bound: bool,
    priv_points_to_provider: bool,
    quality: u16,
    registered: bool,
    current: bool,
    duplicate_name_rejected: bool,
    read_count: usize,
    last_read_len: usize,
}

impl HwRngDevice {
    pub const fn new_virtio_rng(index: usize) -> Self {
        let mut name = [0u8; 25];
        name[0] = b'v';
        name[1] = b'i';
        name[2] = b'r';
        name[3] = b't';
        name[4] = b'i';
        name[5] = b'o';
        name[6] = b'_';
        name[7] = b'r';
        name[8] = b'n';
        name[9] = b'g';
        name[10] = b'.';
        name[11] = b'0' + (index as u8);
        Self {
            lifecycle: Lifecycle::new(State::Base),
            device_ref: None,
            name,
            name_len: 12,
            provider_kind: HwRngProviderKind::VirtioRng,
            read_callback_bound: false,
            cleanup_callback_bound: false,
            priv_points_to_provider: false,
            quality: 0,
            registered: false,
            current: false,
            duplicate_name_rejected: false,
            read_count: 0,
            last_read_len: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn device_ref(&self) -> Option<HwRngDeviceRef> {
        self.device_ref
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub const fn name_bound(&self) -> bool {
        self.name_len != 0
    }

    #[allow(dead_code)]
    pub const fn provider_kind(&self) -> HwRngProviderKind {
        self.provider_kind
    }

    pub const fn read_callback_bound(&self) -> bool {
        self.read_callback_bound
    }

    pub const fn cleanup_callback_bound(&self) -> bool {
        self.cleanup_callback_bound
    }

    pub const fn priv_points_to_provider(&self) -> bool {
        self.priv_points_to_provider
    }

    #[allow(dead_code)]
    pub const fn quality(&self) -> u16 {
        self.quality
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    pub const fn current(&self) -> bool {
        self.current
    }

    #[allow(dead_code)]
    pub const fn duplicate_name_rejected(&self) -> bool {
        self.duplicate_name_rejected
    }

    #[allow(dead_code)]
    pub const fn read_count(&self) -> usize {
        self.read_count
    }

    #[allow(dead_code)]
    pub const fn last_read_len(&self) -> usize {
        self.last_read_len
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || !self.name_bound() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.read_callback_bound = true;
        self.cleanup_callback_bound = true;
        self.priv_points_to_provider = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn mark_registered(&mut self, device_ref: HwRngDeviceRef, quality: u16) {
        self.device_ref = Some(device_ref);
        self.quality = quality;
        self.registered = true;
    }

    fn mark_current(&mut self) {
        self.current = true;
    }

    fn mark_duplicate_name_rejected(&mut self) {
        self.duplicate_name_rejected = true;
    }

    pub(crate) fn record_read(&mut self, len: usize) {
        self.read_count = self.read_count.saturating_add(1);
        self.last_read_len = len;
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct HwRngRegistryEntry {
    device_ref: HwRngDeviceRef,
    name: [u8; 25],
    name_len: usize,
    provider_kind: HwRngProviderKind,
    quality: u16,
    registered: bool,
    current: bool,
    read_count: usize,
    last_read_len: usize,
}

impl HwRngRegistryEntry {
    fn from_device(device_ref: HwRngDeviceRef, device: &HwRngDevice) -> Self {
        Self {
            device_ref,
            name: device.name,
            name_len: device.name_len,
            provider_kind: device.provider_kind,
            quality: device.quality,
            registered: true,
            current: false,
            read_count: 0,
            last_read_len: 0,
        }
    }

    #[allow(dead_code)]
    pub const fn device_ref(&self) -> HwRngDeviceRef {
        self.device_ref
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    #[allow(dead_code)]
    pub const fn provider_kind(&self) -> HwRngProviderKind {
        self.provider_kind
    }

    #[allow(dead_code)]
    pub const fn quality(&self) -> u16 {
        self.quality
    }

    #[allow(dead_code)]
    pub const fn registered(&self) -> bool {
        self.registered
    }

    #[allow(dead_code)]
    pub const fn current(&self) -> bool {
        self.current
    }

    #[allow(dead_code)]
    pub const fn read_count(&self) -> usize {
        self.read_count
    }

    #[allow(dead_code)]
    pub const fn last_read_len(&self) -> usize {
        self.last_read_len
    }

    fn mark_current(&mut self) {
        self.current = true;
    }

    fn clear_current(&mut self) {
        self.current = false;
    }

    fn record_read(&mut self, len: usize) {
        self.read_count = self.read_count.saturating_add(1);
        self.last_read_len = len;
    }
}

pub struct HwRngCore {
    lifecycle: Lifecycle,
    registry_ready: bool,
    current_slot_ready: bool,
    miscdev_deferred: bool,
    random_pool_deferred: bool,
    user_selection_deferred: bool,
    quality_policy_deferred: bool,
    devices: Vec<HwRngRegistryEntry>,
    current_device: Option<HwRngDeviceRef>,
    register_count: usize,
    read_current_count: usize,
    current_rng_ref_acquired: bool,
    current_rng_read_invoked: bool,
    read_copies_from_current: bool,
    read_nonblocking: bool,
    last_read_len: usize,
}

impl HwRngCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registry_ready: false,
            current_slot_ready: false,
            miscdev_deferred: false,
            random_pool_deferred: false,
            user_selection_deferred: false,
            quality_policy_deferred: false,
            devices: Vec::new(),
            current_device: None,
            register_count: 0,
            read_current_count: 0,
            current_rng_ref_acquired: false,
            current_rng_read_invoked: false,
            read_copies_from_current: false,
            read_nonblocking: false,
            last_read_len: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn registry_ready(&self) -> bool {
        self.registry_ready
    }

    pub const fn current_slot_ready(&self) -> bool {
        self.current_slot_ready
    }

    #[allow(dead_code)]
    pub const fn miscdev_deferred(&self) -> bool {
        self.miscdev_deferred
    }

    #[allow(dead_code)]
    pub const fn random_pool_deferred(&self) -> bool {
        self.random_pool_deferred
    }

    #[allow(dead_code)]
    pub const fn user_selection_deferred(&self) -> bool {
        self.user_selection_deferred
    }

    #[allow(dead_code)]
    pub const fn quality_policy_deferred(&self) -> bool {
        self.quality_policy_deferred
    }

    #[allow(dead_code)]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub const fn current_device(&self) -> Option<HwRngDeviceRef> {
        self.current_device
    }

    pub fn device(&self, device_ref: HwRngDeviceRef) -> Option<&HwRngRegistryEntry> {
        self.devices.get(device_ref.index())
    }

    #[allow(dead_code)]
    pub const fn register_count(&self) -> usize {
        self.register_count
    }

    pub const fn read_current_count(&self) -> usize {
        self.read_current_count
    }

    pub const fn current_rng_ref_acquired(&self) -> bool {
        self.current_rng_ref_acquired
    }

    pub const fn current_rng_read_invoked(&self) -> bool {
        self.current_rng_read_invoked
    }

    pub const fn read_copies_from_current(&self) -> bool {
        self.read_copies_from_current
    }

    pub const fn read_nonblocking(&self) -> bool {
        self.read_nonblocking
    }

    pub const fn last_read_len(&self) -> usize {
        self.last_read_len
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
        self.current_slot_ready = true;
        self.miscdev_deferred = true;
        self.random_pool_deferred = true;
        self.user_selection_deferred = true;
        self.quality_policy_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn register(&mut self, device: &mut HwRngDevice) -> Result<HwRngDeviceRef, HwRngError> {
        if self.lifecycle.state() != State::Ready {
            return Err(HwRngError::CoreNotReady);
        }
        if device.state() != State::Ready || !device.name_bound() || !device.read_callback_bound() {
            return Err(HwRngError::DeviceNotReady);
        }

        if self
            .devices
            .iter()
            .any(|registered| registered.name() == device.name())
        {
            device.mark_duplicate_name_rejected();
            return Err(HwRngError::DuplicateName);
        }

        let device_ref = HwRngDeviceRef::new(self.devices.len());
        device.mark_registered(device_ref, 1024);
        let mut entry = HwRngRegistryEntry::from_device(device_ref, device);
        if self.current_device.is_none() {
            device.mark_current();
            entry.mark_current();
            self.current_device = Some(device_ref);
        }
        self.devices.push(entry);
        self.register_count = self.register_count.saturating_add(1);
        Ok(device_ref)
    }

    pub fn read_current<P: HwRngProvider>(
        &mut self,
        provider: &mut P,
        buffer: &mut [u8],
        wait: bool,
    ) -> Result<usize, HwRngError> {
        if self.lifecycle.state() != State::Ready {
            return Err(HwRngError::CoreNotReady);
        }
        let Some(device_ref) = self.current_device else {
            return Err(HwRngError::NoCurrentDevice);
        };
        if self.device(device_ref).is_none() {
            return Err(HwRngError::NoCurrentDevice);
        }

        self.current_rng_ref_acquired = true;
        self.current_rng_read_invoked = true;
        let len = provider.read_hwrng(device_ref, buffer, wait)?;
        if len == 0 {
            return Err(HwRngError::EmptyRead);
        }
        if let Some(device) = self.devices.get_mut(device_ref.index()) {
            device.record_read(len);
        }
        self.read_current_count = self.read_current_count.saturating_add(1);
        self.read_copies_from_current = true;
        self.read_nonblocking = !wait;
        self.last_read_len = len;
        Ok(len)
    }

    #[allow(dead_code)]
    pub fn unregister(&mut self, device_ref: HwRngDeviceRef) -> Result<(), HwRngError> {
        if self.lifecycle.state() != State::Ready {
            return Err(HwRngError::CoreNotReady);
        }
        let Some(device) = self.devices.get_mut(device_ref.index()) else {
            return Err(HwRngError::NoCurrentDevice);
        };
        device.registered = false;
        if self.current_device == Some(device_ref) {
            device.clear_current();
            self.current_device = None;
        }
        Ok(())
    }
}
