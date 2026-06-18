use super::state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State};
use alloc::vec::Vec;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQUEUE_DESC_NONE: u16 = u16::MAX;

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
}

pub struct VirtQueue {
    lifecycle: Lifecycle,
    ring: VirtioSplitRing,
    input_buffer_added: bool,
    free_descriptor_consumed: bool,
    avail_index_advanced: bool,
    kick_recorded: bool,
    fake_completion_recorded: bool,
    used_index_advanced: bool,
    get_buf_returns_len: bool,
    get_buf_empty_rejected: bool,
    buffer_ownership_released: bool,
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
            fake_completion_recorded: false,
            used_index_advanced: false,
            get_buf_returns_len: false,
            get_buf_empty_rejected: false,
            buffer_ownership_released: false,
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

    pub const fn fake_completion_recorded(&self) -> bool {
        self.fake_completion_recorded
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

        let avail_slot = self.ring.ring_index(self.ring.avail_idx);
        self.ring.avail_ring[avail_slot] = head;
        self.ring.avail_idx = self.ring.avail_idx.wrapping_add(1);
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

    pub fn fake_complete_used(
        &mut self,
        token: VirtqueueBufferToken,
        len: u32,
    ) -> Result<(), VirtqueueError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VirtqueueError::NotReady);
        }
        let index = usize::from(token.head());
        if index >= self.ring.descriptors.len() || !self.ring.descriptors[index].active {
            return Err(VirtqueueError::InvalidToken);
        }
        if self.ring.descriptors[index].completed {
            return Err(VirtqueueError::AlreadyCompleted);
        }
        if len > self.ring.descriptors[index].len {
            return Err(VirtqueueError::UsedLengthTooLarge);
        }
        let pending_used = self.ring.used_idx.wrapping_sub(self.ring.last_used_idx);
        if pending_used >= self.ring.queue_size {
            return Err(VirtqueueError::UsedRingFull);
        }

        let used_slot = self.ring.ring_index(self.ring.used_idx);
        self.ring.used_ring[used_slot] = VirtqUsedElem {
            id: token.head(),
            len,
        };
        self.ring.used_idx = self.ring.used_idx.wrapping_add(1);
        self.ring.descriptors[index].completed = true;
        self.fake_completion_recorded = true;
        self.used_index_advanced = true;
        self.completion_count = self.completion_count.saturating_add(1);
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
}
