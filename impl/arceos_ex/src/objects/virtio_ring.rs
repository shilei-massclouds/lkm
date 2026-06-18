use super::state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State};
use alloc::vec::Vec;
use core::{mem::size_of, ptr};

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQUEUE_DESC_NONE: u16 = u16::MAX;
const VRING_USED_F_NO_NOTIFY: u16 = 1;
const STATIC_REAL_QUEUE_SIZE: u16 = 8;
const STATIC_REAL_QUEUE_BYTES: usize = 8192;

#[repr(C, align(4096))]
struct StaticVirtqueueBacking {
    bytes: [u8; STATIC_REAL_QUEUE_BYTES],
}

static mut STATIC_REAL_QUEUE_BACKING: StaticVirtqueueBacking = StaticVirtqueueBacking {
    bytes: [0; STATIC_REAL_QUEUE_BYTES],
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VirtqueueError {
    NotReady,
    InvalidBuffer,
    NoFreeDescriptor,
    InvalidToken,
    AlreadyCompleted,
    UsedRingFull,
    UsedLengthTooLarge,
    NoUsedBuffer,
    RingBackingUnavailable,
    QueueSizeUnsupported,
    TransportUnavailable,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct VirtqueueBufferToken {
    head: u16,
}

impl VirtqueueBufferToken {
    pub const fn head(self) -> u16 {
        self.head
    }
}

#[derive(Clone, Copy)]
pub struct VirtqDescriptor {
    addr: usize,
    len: u32,
    flags: u16,
    next: u16,
    next_free: u16,
    active: bool,
    completed: bool,
}

impl VirtqDescriptor {
    const fn empty() -> Self {
        Self {
            addr: 0,
            len: 0,
            flags: 0,
            next: VIRTQUEUE_DESC_NONE,
            next_free: VIRTQUEUE_DESC_NONE,
            active: false,
            completed: false,
        }
    }

    pub const fn addr(self) -> usize {
        self.addr
    }

    pub const fn len(self) -> u32 {
        self.len
    }

    pub const fn active(self) -> bool {
        self.active
    }

    pub const fn completed(self) -> bool {
        self.completed
    }

    pub const fn writable(self) -> bool {
        self.flags & VIRTQ_DESC_F_WRITE != 0
    }

    pub const fn direct(self) -> bool {
        self.flags & VIRTQ_DESC_F_NEXT == 0 && self.next == VIRTQUEUE_DESC_NONE
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawVirtqDescriptor {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawVirtqUsedElem {
    id: u32,
    len: u32,
}

#[derive(Clone, Copy)]
pub struct VirtioSplitRingLayout {
    queue_size: u16,
    desc_virt: usize,
    avail_virt: usize,
    used_virt: usize,
    desc_phys: usize,
    avail_phys: usize,
    used_phys: usize,
}

impl VirtioSplitRingLayout {
    const fn empty() -> Self {
        Self {
            queue_size: 0,
            desc_virt: 0,
            avail_virt: 0,
            used_virt: 0,
            desc_phys: 0,
            avail_phys: 0,
            used_phys: 0,
        }
    }

    pub const fn queue_size(self) -> u16 {
        self.queue_size
    }

    pub const fn desc_phys(self) -> usize {
        self.desc_phys
    }

    pub const fn avail_phys(self) -> usize {
        self.avail_phys
    }

    pub const fn used_phys(self) -> usize {
        self.used_phys
    }
}

#[derive(Clone, Copy)]
pub struct VirtqUsedElem {
    id: u16,
    len: u32,
}

impl VirtqUsedElem {
    const fn empty() -> Self {
        Self {
            id: VIRTQUEUE_DESC_NONE,
            len: 0,
        }
    }

    pub const fn id(self) -> u16 {
        self.id
    }

    pub const fn len(self) -> u32 {
        self.len
    }
}

#[derive(Clone, Copy)]
pub struct VirtqueueUsedBuffer {
    token: VirtqueueBufferToken,
    addr: usize,
    len: u32,
    descriptor_len: u32,
}

impl VirtqueueUsedBuffer {
    pub const fn token(self) -> VirtqueueBufferToken {
        self.token
    }

    pub const fn addr(self) -> usize {
        self.addr
    }

    pub const fn len(self) -> u32 {
        self.len
    }

    pub const fn descriptor_len(self) -> u32 {
        self.descriptor_len
    }
}

pub struct VirtioSplitRing {
    lifecycle: Lifecycle,
    queue_size: u16,
    descriptors: Vec<VirtqDescriptor>,
    avail_ring: Vec<u16>,
    used_ring: Vec<VirtqUsedElem>,
    free_head: Option<u16>,
    free_count: u16,
    avail_idx: u16,
    used_idx: u16,
    last_used_idx: u16,
    direct_descriptors_only: bool,
    indirect_descriptors_deferred: bool,
    event_idx_deferred: bool,
    dma_cache_deferred: bool,
    static_coherent_backing_ready: bool,
    desc_avail_used_layout_ready: bool,
    device_visible_phys_addr_ready: bool,
    real_layout: VirtioSplitRingLayout,
    descriptor_exhaustion_rejected: bool,
    empty_get_rejected: bool,
    buffer_ownership_released: bool,
}

impl VirtioSplitRing {
    pub fn new(queue_size: u16) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            queue_size,
            descriptors: Vec::new(),
            avail_ring: Vec::new(),
            used_ring: Vec::new(),
            free_head: None,
            free_count: 0,
            avail_idx: 0,
            used_idx: 0,
            last_used_idx: 0,
            direct_descriptors_only: false,
            indirect_descriptors_deferred: false,
            event_idx_deferred: false,
            dma_cache_deferred: false,
            static_coherent_backing_ready: false,
            desc_avail_used_layout_ready: false,
            device_visible_phys_addr_ready: false,
            real_layout: VirtioSplitRingLayout::empty(),
            descriptor_exhaustion_rejected: false,
            empty_get_rejected: false,
            buffer_ownership_released: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn queue_size(&self) -> u16 {
        self.queue_size
    }

    pub fn descriptor_table_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.descriptors.len() == self.queue_size as usize
            && self.free_head.is_some()
    }

    pub fn avail_ring_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready && self.avail_ring.len() == self.queue_size as usize
    }

    pub fn used_ring_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready && self.used_ring.len() == self.queue_size as usize
    }

    pub fn free_list_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready && self.free_count == self.queue_size
    }

    pub const fn single_queue(&self) -> bool {
        true
    }

    pub const fn direct_descriptors_only(&self) -> bool {
        self.direct_descriptors_only
    }

    pub const fn indirect_descriptors_deferred(&self) -> bool {
        self.indirect_descriptors_deferred
    }

    pub const fn event_idx_deferred(&self) -> bool {
        self.event_idx_deferred
    }

    pub const fn dma_cache_deferred(&self) -> bool {
        self.dma_cache_deferred
    }

    pub const fn free_count(&self) -> u16 {
        self.free_count
    }

    pub const fn avail_idx(&self) -> u16 {
        self.avail_idx
    }

    pub const fn used_idx(&self) -> u16 {
        self.used_idx
    }

    pub const fn last_used_idx(&self) -> u16 {
        self.last_used_idx
    }

    pub const fn descriptor_exhaustion_rejected(&self) -> bool {
        self.descriptor_exhaustion_rejected
    }

    pub const fn empty_get_rejected(&self) -> bool {
        self.empty_get_rejected
    }

    pub const fn buffer_ownership_released(&self) -> bool {
        self.buffer_ownership_released
    }

    pub fn descriptor(&self, token: VirtqueueBufferToken) -> Option<VirtqDescriptor> {
        let index = usize::from(token.head());
        if index >= self.descriptors.len() {
            return None;
        }
        Some(self.descriptors[index])
    }

    pub fn avail_entry(&self, index: u16) -> Option<u16> {
        if self.lifecycle.state() != State::Ready || index >= self.avail_idx {
            return None;
        }
        Some(self.avail_ring[self.ring_index(index)])
    }

    pub fn used_entry(&self, index: u16) -> Option<VirtqUsedElem> {
        if self.lifecycle.state() != State::Ready || index >= self.used_idx {
            return None;
        }
        Some(self.used_ring[self.ring_index(index)])
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.queue_size == 0
            || !self.queue_size.is_power_of_two()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.descriptors.clear();
        self.avail_ring.clear();
        self.used_ring.clear();
        let mut index = 0usize;
        while index < usize::from(self.queue_size) {
            let mut descriptor = VirtqDescriptor::empty();
            descriptor.next_free = if index + 1 == usize::from(self.queue_size) {
                VIRTQUEUE_DESC_NONE
            } else {
                u16::try_from(index + 1).unwrap_or(VIRTQUEUE_DESC_NONE)
            };
            self.descriptors.push(descriptor);
            self.avail_ring.push(VIRTQUEUE_DESC_NONE);
            self.used_ring.push(VirtqUsedElem::empty());
            index += 1;
        }

        self.free_head = Some(0);
        self.free_count = self.queue_size;
        self.avail_idx = 0;
        self.used_idx = 0;
        self.last_used_idx = 0;
        self.direct_descriptors_only = true;
        self.indirect_descriptors_deferred = true;
        self.event_idx_deferred = true;
        self.dma_cache_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn pop_free_descriptor(&mut self) -> Option<u16> {
        let head = self.free_head?;
        let index = usize::from(head);
        if index >= self.descriptors.len() {
            return None;
        }
        let next = self.descriptors[index].next_free;
        self.free_head = if next == VIRTQUEUE_DESC_NONE {
            None
        } else {
            Some(next)
        };
        self.free_count = self.free_count.saturating_sub(1);
        self.descriptors[index].next_free = VIRTQUEUE_DESC_NONE;
        Some(head)
    }

    fn release_descriptor(&mut self, head: u16) {
        let index = usize::from(head);
        if index >= self.descriptors.len() {
            return;
        }
        self.descriptors[index] = VirtqDescriptor::empty();
        self.descriptors[index].next_free = self.free_head.unwrap_or(VIRTQUEUE_DESC_NONE);
        self.free_head = Some(head);
        if self.free_count < self.queue_size {
            self.free_count = self.free_count.saturating_add(1);
        }
    }

    fn ring_index(&self, index: u16) -> usize {
        usize::from(index % self.queue_size)
    }

    fn setup_static_real_backing(
        &mut self,
        kernel_image: &super::kernel_image::KernelImage,
        queue_size: u16,
    ) -> Result<VirtioSplitRingLayout, VirtqueueError> {
        if self.lifecycle.state() != State::Ready
            || queue_size == 0
            || queue_size > STATIC_REAL_QUEUE_SIZE
            || !queue_size.is_power_of_two()
        {
            return Err(VirtqueueError::QueueSizeUnsupported);
        }

        let backing_virt = unsafe {
            let backing = (&raw mut STATIC_REAL_QUEUE_BACKING)
                .as_mut()
                .ok_or(VirtqueueError::RingBackingUnavailable)?;
            backing.bytes.fill(0);
            backing.bytes.as_mut_ptr() as usize
        };
        let desc_virt = backing_virt;
        let desc_bytes = align_up(usize::from(queue_size) * size_of::<RawVirtqDescriptor>(), 2)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let avail_virt = backing_virt
            .checked_add(desc_bytes)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let avail_bytes = 4usize
            .checked_add(
                usize::from(queue_size)
                    .checked_mul(size_of::<u16>())
                    .ok_or(VirtqueueError::RingBackingUnavailable)?,
            )
            .and_then(|size| size.checked_add(size_of::<u16>()))
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let used_offset = align_up(
            desc_bytes
                .checked_add(avail_bytes)
                .ok_or(VirtqueueError::RingBackingUnavailable)?,
            4096,
        )
        .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let used_virt = backing_virt
            .checked_add(used_offset)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let used_bytes = 4usize
            .checked_add(
                usize::from(queue_size)
                    .checked_mul(size_of::<RawVirtqUsedElem>())
                    .ok_or(VirtqueueError::RingBackingUnavailable)?,
            )
            .and_then(|size| size.checked_add(size_of::<u16>()))
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let end = used_virt
            .checked_add(used_bytes)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        if end > backing_virt + STATIC_REAL_QUEUE_BYTES {
            return Err(VirtqueueError::RingBackingUnavailable);
        }

        let desc_phys = kernel_image
            .runtime_to_phys(desc_virt)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let avail_phys = kernel_image
            .runtime_to_phys(avail_virt)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let used_phys = kernel_image
            .runtime_to_phys(used_virt)
            .ok_or(VirtqueueError::RingBackingUnavailable)?;
        let layout = VirtioSplitRingLayout {
            queue_size,
            desc_virt,
            avail_virt,
            used_virt,
            desc_phys,
            avail_phys,
            used_phys,
        };
        self.real_layout = layout;
        self.static_coherent_backing_ready = true;
        self.desc_avail_used_layout_ready = true;
        self.device_visible_phys_addr_ready = desc_phys != 0 && avail_phys != 0 && used_phys != 0;
        if self.device_visible_phys_addr_ready {
            Ok(layout)
        } else {
            Err(VirtqueueError::RingBackingUnavailable)
        }
    }

    fn raw_desc_ptr(&self, index: u16) -> Option<*mut RawVirtqDescriptor> {
        if self.real_layout.desc_virt == 0 || index >= self.real_layout.queue_size {
            return None;
        }
        Some((self.real_layout.desc_virt as *mut RawVirtqDescriptor).wrapping_add(index as usize))
    }

    fn raw_avail_flags_ptr(&self) -> Option<*mut u16> {
        if self.real_layout.avail_virt == 0 {
            return None;
        }
        Some(self.real_layout.avail_virt as *mut u16)
    }

    fn raw_avail_idx_ptr(&self) -> Option<*mut u16> {
        Some(unsafe { self.raw_avail_flags_ptr()?.add(1) })
    }

    fn raw_avail_ring_ptr(&self, index: u16) -> Option<*mut u16> {
        let ring_base = unsafe { self.raw_avail_flags_ptr()?.add(2) };
        Some(unsafe { ring_base.add(self.ring_index(index)) })
    }

    fn raw_used_flags_ptr(&self) -> Option<*const u16> {
        if self.real_layout.used_virt == 0 {
            return None;
        }
        Some(self.real_layout.used_virt as *const u16)
    }

    fn raw_used_idx_ptr(&self) -> Option<*const u16> {
        Some(unsafe { self.raw_used_flags_ptr()?.add(1) })
    }

    fn raw_used_ring_ptr(&self, index: u16) -> Option<*const RawVirtqUsedElem> {
        let ring_base =
            unsafe { (self.real_layout.used_virt as *const u16).add(2) } as *const RawVirtqUsedElem;
        Some(unsafe { ring_base.add(self.ring_index(index)) })
    }
}

pub struct VirtQueue {
    lifecycle: Lifecycle,
    ring: VirtioSplitRing,
    input_buffer_added: bool,
    free_descriptor_consumed: bool,
    avail_index_advanced: bool,
    kick_recorded: bool,
    used_index_advanced: bool,
    get_buf_returns_len: bool,
    get_buf_empty_rejected: bool,
    buffer_ownership_released: bool,
    real_backing_ready: bool,
    queue_index: u16,
    queue_num_max_observed: bool,
    legacy_mmio_queue_pfn_written: bool,
    modern_mmio_queue_addrs_written: bool,
    mmio_queue_ready_written: bool,
    mmio_notify_written: bool,
    real_used_completion_observed: bool,
    last_used_len: u32,
    submitted_count: usize,
    kick_count: usize,
    completion_count: usize,
    get_buf_count: usize,
}

impl VirtQueue {
    pub fn new(queue_size: u16) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ring: VirtioSplitRing::new(queue_size),
            input_buffer_added: false,
            free_descriptor_consumed: false,
            avail_index_advanced: false,
            kick_recorded: false,
            used_index_advanced: false,
            get_buf_returns_len: false,
            get_buf_empty_rejected: false,
            buffer_ownership_released: false,
            real_backing_ready: false,
            queue_index: 0,
            queue_num_max_observed: false,
            legacy_mmio_queue_pfn_written: false,
            modern_mmio_queue_addrs_written: false,
            mmio_queue_ready_written: false,
            mmio_notify_written: false,
            real_used_completion_observed: false,
            last_used_len: 0,
            submitted_count: 0,
            kick_count: 0,
            completion_count: 0,
            get_buf_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ring(&self) -> &VirtioSplitRing {
        &self.ring
    }

    pub const fn input_buffer_added(&self) -> bool {
        self.input_buffer_added
    }

    pub const fn free_descriptor_consumed(&self) -> bool {
        self.free_descriptor_consumed
    }

    pub const fn avail_index_advanced(&self) -> bool {
        self.avail_index_advanced
    }

    pub const fn kick_recorded(&self) -> bool {
        self.kick_recorded
    }

    pub const fn used_index_advanced(&self) -> bool {
        self.used_index_advanced
    }

    pub const fn get_buf_returns_len(&self) -> bool {
        self.get_buf_returns_len
    }

    pub const fn get_buf_empty_rejected(&self) -> bool {
        self.get_buf_empty_rejected
    }

    pub const fn buffer_ownership_released(&self) -> bool {
        self.buffer_ownership_released
    }

    #[allow(dead_code)]
    pub const fn real_backing_ready(&self) -> bool {
        self.real_backing_ready
    }

    #[allow(dead_code)]
    pub const fn queue_index(&self) -> u16 {
        self.queue_index
    }

    #[allow(dead_code)]
    pub const fn queue_num_max_observed(&self) -> bool {
        self.queue_num_max_observed
    }

    #[allow(dead_code)]
    pub const fn legacy_mmio_queue_pfn_written(&self) -> bool {
        self.legacy_mmio_queue_pfn_written
    }

    #[allow(dead_code)]
    pub const fn modern_mmio_queue_addrs_written(&self) -> bool {
        self.modern_mmio_queue_addrs_written
    }

    #[allow(dead_code)]
    pub const fn mmio_queue_ready_written(&self) -> bool {
        self.mmio_queue_ready_written
    }

    #[allow(dead_code)]
    pub const fn mmio_notify_written(&self) -> bool {
        self.mmio_notify_written
    }

    #[allow(dead_code)]
    pub const fn real_used_completion_observed(&self) -> bool {
        self.real_used_completion_observed
    }

    #[allow(dead_code)]
    pub const fn last_used_len(&self) -> u32 {
        self.last_used_len
    }

    pub const fn submitted_count(&self) -> usize {
        self.submitted_count
    }

    pub const fn kick_count(&self) -> usize {
        self.kick_count
    }

    pub const fn completion_count(&self) -> usize {
        self.completion_count
    }

    pub const fn get_buf_count(&self) -> usize {
        self.get_buf_count
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.ring.setup()?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn setup_real_mmio(
        &mut self,
        kernel_image: &super::kernel_image::KernelImage,
        transport: super::virtio_mmio::VirtioMmioTransportDevice,
        queue_index: u16,
    ) -> Result<(), VirtqueueError> {
        if self.lifecycle.state() == State::Base {
            self.setup().map_err(|_| VirtqueueError::NotReady)?;
        }
        if self.lifecycle.state() != State::Ready {
            return Err(VirtqueueError::NotReady);
        }
        let layout = self
            .ring
            .setup_static_real_backing(kernel_image, self.ring.queue_size())?;
        let setup = super::virtio_mmio::setup_queue(
            transport,
            super::virtio_mmio::VirtioMmioQueueConfig {
                queue_index,
                queue_size: layout.queue_size(),
                desc_phys: layout.desc_phys(),
                avail_phys: layout.avail_phys(),
                used_phys: layout.used_phys(),
            },
        )
        .ok_or(VirtqueueError::TransportUnavailable)?;
        self.queue_index = queue_index;
        self.queue_num_max_observed = setup.num_max >= setup.queue_size;
        self.legacy_mmio_queue_pfn_written = setup.legacy_pfn_written;
        self.modern_mmio_queue_addrs_written = setup.modern_addrs_written;
        self.mmio_queue_ready_written = setup.queue_ready_written;
        self.real_backing_ready = true;
        Ok(())
    }

    pub fn add_inbuf(
        &mut self,
        addr: usize,
        len: u32,
    ) -> Result<VirtqueueBufferToken, VirtqueueError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtqueueError::NotReady);
        }
        if addr == 0 || len == 0 {
            return Err(VirtqueueError::InvalidBuffer);
        }
        let Some(head) = self.ring.pop_free_descriptor() else {
            self.ring.descriptor_exhaustion_rejected = true;
            return Err(VirtqueueError::NoFreeDescriptor);
        };
        let desc_index = usize::from(head);
        self.ring.descriptors[desc_index] = VirtqDescriptor {
            addr,
            len,
            flags: VIRTQ_DESC_F_WRITE,
            next: VIRTQUEUE_DESC_NONE,
            next_free: VIRTQUEUE_DESC_NONE,
            active: true,
            completed: false,
        };
        if self.real_backing_ready {
            let Some(raw_desc) = self.ring.raw_desc_ptr(head) else {
                return Err(VirtqueueError::RingBackingUnavailable);
            };
            unsafe {
                ptr::write_volatile(
                    raw_desc,
                    RawVirtqDescriptor {
                        addr: addr as u64,
                        len,
                        flags: VIRTQ_DESC_F_WRITE,
                        next: 0,
                    },
                );
            }
        }

        let avail_slot = self.ring.ring_index(self.ring.avail_idx);
        self.ring.avail_ring[avail_slot] = head;
        if self.real_backing_ready {
            let Some(raw_avail_entry) = self.ring.raw_avail_ring_ptr(self.ring.avail_idx) else {
                return Err(VirtqueueError::RingBackingUnavailable);
            };
            unsafe {
                ptr::write_volatile(raw_avail_entry, head);
                if let Some(flags) = self.ring.raw_avail_flags_ptr() {
                    ptr::write_volatile(flags, 0);
                }
            }
        }
        self.ring.avail_idx = self.ring.avail_idx.wrapping_add(1);
        if self.real_backing_ready {
            let Some(raw_avail_idx) = self.ring.raw_avail_idx_ptr() else {
                return Err(VirtqueueError::RingBackingUnavailable);
            };
            unsafe {
                ptr::write_volatile(raw_avail_idx, self.ring.avail_idx);
            }
        }
        self.input_buffer_added = true;
        self.free_descriptor_consumed = true;
        self.avail_index_advanced = true;
        self.submitted_count = self.submitted_count.saturating_add(1);
        Ok(VirtqueueBufferToken { head })
    }

    pub fn kick(&mut self) -> Result<(), VirtqueueError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtqueueError::NotReady);
        }
        if !self.input_buffer_added {
            return Err(VirtqueueError::InvalidToken);
        }
        self.kick_recorded = true;
        self.kick_count = self.kick_count.saturating_add(1);
        Ok(())
    }

    pub fn kick_mmio(
        &mut self,
        transport: super::virtio_mmio::VirtioMmioTransportDevice,
    ) -> Result<(), VirtqueueError> {
        self.kick()?;
        if !super::virtio_mmio::notify_queue(transport, self.queue_index) {
            return Err(VirtqueueError::TransportUnavailable);
        }
        self.mmio_notify_written = true;
        Ok(())
    }

    pub fn get_buf(&mut self) -> Result<VirtqueueUsedBuffer, VirtqueueError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtqueueError::NotReady);
        }
        if self.ring.last_used_idx == self.ring.used_idx {
            self.ring.empty_get_rejected = true;
            self.get_buf_empty_rejected = true;
            return Err(VirtqueueError::NoUsedBuffer);
        }

        let used_slot = self.ring.ring_index(self.ring.last_used_idx);
        let used = self.ring.used_ring[used_slot];
        let desc_index = usize::from(used.id());
        if desc_index >= self.ring.descriptors.len()
            || !self.ring.descriptors[desc_index].active
            || !self.ring.descriptors[desc_index].completed
        {
            return Err(VirtqueueError::InvalidToken);
        }
        let descriptor = self.ring.descriptors[desc_index];
        self.ring.last_used_idx = self.ring.last_used_idx.wrapping_add(1);
        self.ring.release_descriptor(used.id());
        self.ring.buffer_ownership_released = true;
        self.get_buf_returns_len = true;
        self.buffer_ownership_released = true;
        self.get_buf_count = self.get_buf_count.saturating_add(1);
        Ok(VirtqueueUsedBuffer {
            token: VirtqueueBufferToken { head: used.id() },
            addr: descriptor.addr(),
            len: used.len(),
            descriptor_len: descriptor.len(),
        })
    }

    pub fn get_buf_from_device(&mut self) -> Result<VirtqueueUsedBuffer, VirtqueueError> {
        if self.lifecycle.state() != State::Ready || !self.real_backing_ready {
            return Err(VirtqueueError::NotReady);
        }
        let used_idx = unsafe {
            let Some(ptr) = self.ring.raw_used_idx_ptr() else {
                return Err(VirtqueueError::RingBackingUnavailable);
            };
            ptr::read_volatile(ptr)
        };
        self.ring.used_idx = used_idx;
        if self.ring.last_used_idx == used_idx {
            self.ring.empty_get_rejected = true;
            self.get_buf_empty_rejected = true;
            return Err(VirtqueueError::NoUsedBuffer);
        }

        let used = unsafe {
            let Some(ptr) = self.ring.raw_used_ring_ptr(self.ring.last_used_idx) else {
                return Err(VirtqueueError::RingBackingUnavailable);
            };
            ptr::read_volatile(ptr)
        };
        let id = u16::try_from(used.id).map_err(|_| VirtqueueError::InvalidToken)?;
        let desc_index = usize::from(id);
        if desc_index >= self.ring.descriptors.len() || !self.ring.descriptors[desc_index].active {
            return Err(VirtqueueError::InvalidToken);
        }
        if used.len > self.ring.descriptors[desc_index].len {
            return Err(VirtqueueError::UsedLengthTooLarge);
        }
        let used_slot = self.ring.ring_index(self.ring.last_used_idx);
        self.ring.used_ring[used_slot] = VirtqUsedElem { id, len: used.len };
        self.ring.descriptors[desc_index].completed = true;
        self.real_used_completion_observed = true;
        self.used_index_advanced = true;
        self.completion_count = self.completion_count.saturating_add(1);
        self.last_used_len = used.len;
        let buffer = self.get_buf()?;
        if let Some(flags) = self.ring.raw_used_flags_ptr() {
            let _ = unsafe { ptr::read_volatile(flags) & VRING_USED_F_NO_NOTIFY };
        }
        Ok(buffer)
    }
}

#[cfg(app_smoke)]
pub(crate) mod smoke_fixture {
    use super::{State, VirtQueue, VirtqUsedElem, VirtqueueBufferToken, VirtqueueError};

    pub(crate) fn prepare_used_entry(
        queue: &mut VirtQueue,
        token: VirtqueueBufferToken,
        len: u32,
    ) -> Result<(), VirtqueueError> {
        if queue.lifecycle.state() != State::Ready {
            return Err(VirtqueueError::NotReady);
        }
        let index = usize::from(token.head());
        if index >= queue.ring.descriptors.len() || !queue.ring.descriptors[index].active {
            return Err(VirtqueueError::InvalidToken);
        }
        if queue.ring.descriptors[index].completed {
            return Err(VirtqueueError::AlreadyCompleted);
        }
        if len > queue.ring.descriptors[index].len {
            return Err(VirtqueueError::UsedLengthTooLarge);
        }
        let pending_used = queue.ring.used_idx.wrapping_sub(queue.ring.last_used_idx);
        if pending_used >= queue.ring.queue_size {
            return Err(VirtqueueError::UsedRingFull);
        }

        let used_slot = queue.ring.ring_index(queue.ring.used_idx);
        queue.ring.used_ring[used_slot] = VirtqUsedElem {
            id: token.head(),
            len,
        };
        queue.ring.used_idx = queue.ring.used_idx.wrapping_add(1);
        queue.ring.descriptors[index].completed = true;
        queue.used_index_advanced = true;
        queue.completion_count = queue.completion_count.saturating_add(1);
        Ok(())
    }
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}
