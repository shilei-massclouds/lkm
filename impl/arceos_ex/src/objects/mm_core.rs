use super::{
    config::Config,
    cpu_group::CpuGroup,
    cpu_hotplug::CpuHotplugState,
    dma_cache_policy::DmaCachePolicy,
    early_param::EarlyParam,
    kernel_image::KernelImage,
    memblock::MemBlock,
    page_table::{map_page_range_runtime, unmap_page_range_runtime, PageTableInstallRange},
    per_cpu_storage::PerCpuStorage,
    raw_dtb::PhysRange,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::{StaticBranch, StaticKey},
    static_objects::StaticObjects,
    vm::Vm,
    workqueue::Workqueue,
    zones::{ZoneKind, Zones},
};
use crate::{arch::riscv64::csr, trace::Checkpoint};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

const MAX_BOOT_ZONES: usize = 3;
const MAX_KMALLOC_CACHES: usize = 11;
const MAX_KMALLOC_SLABS: usize = 32;
const BUDDY_ORDER_COUNT: usize = 11;
const BUDDY_INVALID_INDEX: usize = usize::MAX;
const PAGE_METADATA_FLAG_BUDDY_FREE: usize = 1 << 0;
const PAGE_METADATA_FLAG_BUDDY_ALLOCATED: usize = 1 << 1;
const KMALLOC_NULL: usize = 0;
const PAGE_ALLOC_CPUHP_STEP: usize = 0x200;
const SLUB_CPUHP_STEP: usize = 0x201;
pub const VMALLOC_START: usize = 0xffff_ffc8_0000_0000;
const VMALLOC_END: usize = 0xffff_ffd0_0000_0000;
const VMALLOC_RUNTIME_PAGE_SIZE: usize = 4096;
const MAX_VMAP_AREAS: usize = 16;
const MAX_VMAP_MAPPINGS: usize = 16;
const MAX_DYNAMIC_VMALLOC_PGTABLES: usize = 12;
pub const GLOBAL_ALLOC_MAX_SIZE: usize = 8192;

#[derive(Clone, Copy)]
pub struct ZoneRef {
    kind: ZoneKind,
    range: PhysRange,
    managed_pages: usize,
    free_pages: usize,
}

impl ZoneRef {
    const fn empty() -> Self {
        Self {
            kind: ZoneKind::Dma32,
            range: PhysRange::empty(),
            managed_pages: 0,
            free_pages: 0,
        }
    }

    pub const fn kind(&self) -> ZoneKind {
        self.kind
    }

    pub const fn range(&self) -> PhysRange {
        self.range
    }

    pub const fn managed_pages(&self) -> usize {
        self.managed_pages
    }

    pub const fn free_pages(&self) -> usize {
        self.free_pages
    }
}

pub struct MemoryTopology {
    lifecycle: Lifecycle,
    boot_memory_node: BootMemoryNode,
    boot_zone_set: BootZoneSet,
    node_count: usize,
}

impl MemoryTopology {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_memory_node: BootMemoryNode::new(),
            boot_zone_set: BootZoneSet::new(),
            node_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    pub const fn boot_memory_node(&self) -> &BootMemoryNode {
        &self.boot_memory_node
    }

    pub const fn boot_zone_set(&self) -> &BootZoneSet {
        &self.boot_zone_set
    }

    pub fn setup(&mut self, zones: &Zones, cpu_group: &CpuGroup, config: &Config) -> EventResult {
        if self.lifecycle.state() != State::Base
            || zones.state() != State::Ready
            || cpu_group.state() != State::Ready
            || config.state() != State::Online
        {
            return self.failed_setup();
        }

        self.boot_memory_node.setup(zones)?;
        self.boot_zone_set.setup(zones, config)?;
        self.node_count = 1;

        if self.boot_memory_node.state() != State::Ready
            || self.boot_zone_set.state() != State::Ready
            || self.boot_zone_set.populated_count() == 0
        {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::MemoryTopologyReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct BootMemoryNode {
    lifecycle: Lifecycle,
    node_id: usize,
    present_pages: usize,
}

impl BootMemoryNode {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            node_id: 0,
            present_pages: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn node_id(&self) -> usize {
        self.node_id
    }

    pub const fn present_pages(&self) -> usize {
        self.present_pages
    }

    fn setup(&mut self, zones: &Zones) -> EventResult {
        if self.lifecycle.state() != State::Base || zones.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.node_id = 0;
        let Some(present_pages) = zone_present_pages(zones, 4096) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        self.present_pages = present_pages;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootMemoryNodeReady,
        )
    }
}

pub struct BootZoneSet {
    lifecycle: Lifecycle,
    zones: [ZoneRef; MAX_BOOT_ZONES],
    populated_count: usize,
}

impl BootZoneSet {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            zones: [ZoneRef::empty(); MAX_BOOT_ZONES],
            populated_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn populated_count(&self) -> usize {
        self.populated_count
    }

    pub fn populated_zone(&self, index: usize) -> Option<ZoneRef> {
        if index < self.populated_count {
            Some(self.zones[index])
        } else {
            None
        }
    }

    fn setup(&mut self, zones: &Zones, config: &Config) -> EventResult {
        if self.lifecycle.state() != State::Base
            || zones.state() != State::Ready
            || config.page_size() == 0
        {
            return self.failed_setup();
        }

        self.zones = [ZoneRef::empty(); MAX_BOOT_ZONES];
        self.populated_count = 0;

        for kind in [ZoneKind::Dma32, ZoneKind::Normal, ZoneKind::Movable] {
            let Some(zone) = zones.zone(kind) else {
                return self.failed_setup();
            };
            if zone.is_empty() {
                continue;
            }
            if self.populated_count >= MAX_BOOT_ZONES {
                return self.failed_setup();
            }
            let Some(pages) = bytes_to_pages(zone.present_bytes(), config.page_size()) else {
                return self.failed_setup();
            };
            self.zones[self.populated_count] = ZoneRef {
                kind,
                range: zone.range(),
                managed_pages: 0,
                free_pages: 0,
            };
            self.zones[self.populated_count].managed_pages = pages;
            self.zones[self.populated_count].free_pages = pages;
            self.populated_count += 1;
        }

        if self.populated_count == 0 {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootZoneSetReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct BootZonelistSet {
    lifecycle: Lifecycle,
    fallback: [Option<ZoneKind>; MAX_BOOT_ZONES + 1],
    fallback_count: usize,
    null_sentinel_ready: bool,
}

impl BootZonelistSet {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fallback: [None; MAX_BOOT_ZONES + 1],
            fallback_count: 0,
            null_sentinel_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fallback_count(&self) -> usize {
        self.fallback_count
    }

    pub const fn null_sentinel_ready(&self) -> bool {
        self.null_sentinel_ready
    }

    pub fn fallback_zone_kind(&self, index: usize) -> Option<ZoneKind> {
        if index < self.fallback_count {
            self.fallback[index]
        } else {
            None
        }
    }

    fn setup(&mut self, topology: &MemoryTopology) -> EventResult {
        if self.lifecycle.state() != State::Base
            || topology.boot_memory_node().state() != State::Ready
            || topology.boot_zone_set().state() != State::Ready
        {
            return self.failed_setup();
        }

        self.fallback = [None; MAX_BOOT_ZONES + 1];
        self.fallback_count = 0;
        let zone_set = topology.boot_zone_set();
        let mut index = zone_set.populated_count();
        while index != 0 {
            index -= 1;
            let Some(zone) = zone_set.populated_zone(index) else {
                return self.failed_setup();
            };
            self.fallback[self.fallback_count] = Some(zone.kind());
            self.fallback_count += 1;
        }
        self.fallback[self.fallback_count] = None;
        self.null_sentinel_ready = true;

        if self.fallback_count == 0 {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootZonelistSetReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Pfn {
    value: usize,
}

impl Pfn {
    pub const fn new(value: usize) -> Self {
        Self { value }
    }

    pub const fn value(self) -> usize {
        self.value
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PhysPageAddr {
    addr: usize,
}

impl PhysPageAddr {
    pub const fn new(addr: usize) -> Self {
        Self { addr }
    }

    pub const fn value(self) -> usize {
        self.addr
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct LinearMappedPageAddr {
    addr: usize,
}

impl LinearMappedPageAddr {
    pub const fn new(addr: usize) -> Self {
        Self { addr }
    }

    pub const fn value(self) -> usize {
        self.addr
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct GfpFlags {
    bits: usize,
}

impl GfpFlags {
    pub const EMPTY: Self = Self { bits: 0 };
    pub const KERNEL: Self = Self { bits: 1 << 0 };

    pub const fn empty() -> Self {
        Self::EMPTY
    }

    pub const fn kernel() -> Self {
        Self::KERNEL
    }

    pub const fn bits(self) -> usize {
        self.bits
    }

    const fn boot_buddy_allowed(self) -> bool {
        self.bits & !Self::KERNEL.bits == 0
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PageRef {
    pfn: Pfn,
    metadata_index: usize,
    metadata_linear: usize,
    phys: PhysPageAddr,
    linear: LinearMappedPageAddr,
}

impl PageRef {
    pub const fn pfn(self) -> Pfn {
        self.pfn
    }

    pub const fn metadata_index(self) -> usize {
        self.metadata_index
    }

    pub const fn metadata_linear(self) -> usize {
        self.metadata_linear
    }

    pub const fn phys(self) -> PhysPageAddr {
        self.phys
    }

    pub const fn linear(self) -> LinearMappedPageAddr {
        self.linear
    }
}

const fn empty_page_ref() -> PageRef {
    PageRef {
        pfn: Pfn::new(0),
        metadata_index: 0,
        metadata_linear: 0,
        phys: PhysPageAddr::new(0),
        linear: LinearMappedPageAddr::new(0),
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageMetadata {
    pfn: usize,
    flags: usize,
    refcount: usize,
    buddy_order: usize,
    buddy_prev: usize,
    buddy_next: usize,
}

impl PageMetadata {
    const fn new(pfn: usize) -> Self {
        Self {
            pfn,
            flags: 0,
            refcount: 0,
            buddy_order: 0,
            buddy_prev: BUDDY_INVALID_INDEX,
            buddy_next: BUDDY_INVALID_INDEX,
        }
    }

    pub const fn pfn(self) -> Pfn {
        Pfn::new(self.pfn)
    }

    pub const fn is_buddy_free(self) -> bool {
        self.flags & PAGE_METADATA_FLAG_BUDDY_FREE != 0
    }

    pub const fn is_buddy_allocated(self) -> bool {
        self.flags & PAGE_METADATA_FLAG_BUDDY_ALLOCATED != 0
    }

    pub const fn buddy_order(self) -> usize {
        self.buddy_order
    }

    pub const fn buddy_prev(self) -> Option<usize> {
        if self.buddy_prev == BUDDY_INVALID_INDEX {
            None
        } else {
            Some(self.buddy_prev)
        }
    }

    pub const fn buddy_next(self) -> Option<usize> {
        if self.buddy_next == BUDDY_INVALID_INDEX {
            None
        } else {
            Some(self.buddy_next)
        }
    }

    fn mark_buddy_free(&mut self, order: usize, prev: usize, next: usize) {
        self.flags &= !PAGE_METADATA_FLAG_BUDDY_ALLOCATED;
        self.flags |= PAGE_METADATA_FLAG_BUDDY_FREE;
        self.refcount = 0;
        self.buddy_order = order;
        self.buddy_prev = prev;
        self.buddy_next = next;
    }

    fn mark_buddy_allocated(&mut self, order: usize) {
        self.flags &= !PAGE_METADATA_FLAG_BUDDY_FREE;
        self.flags |= PAGE_METADATA_FLAG_BUDDY_ALLOCATED;
        self.refcount = 1;
        self.buddy_order = order;
        self.buddy_prev = BUDDY_INVALID_INDEX;
        self.buddy_next = BUDDY_INVALID_INDEX;
    }

    fn clear_buddy_state(&mut self) {
        self.flags &= !(PAGE_METADATA_FLAG_BUDDY_FREE | PAGE_METADATA_FLAG_BUDDY_ALLOCATED);
        self.refcount = 0;
        self.buddy_order = 0;
        self.buddy_prev = BUDDY_INVALID_INDEX;
        self.buddy_next = BUDDY_INVALID_INDEX;
    }
}

pub struct PageMetadataMap {
    lifecycle: Lifecycle,
    start_pfn: usize,
    end_pfn: usize,
    page_size: usize,
    metadata_bytes: usize,
    metadata_storage_size: usize,
    metadata_storage: PhysRange,
    metadata_storage_linear: usize,
    linear_map_virt_start: usize,
    metadata_count: usize,
}

impl PageMetadataMap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start_pfn: 0,
            end_pfn: 0,
            page_size: 0,
            metadata_bytes: 0,
            metadata_storage_size: 0,
            metadata_storage: PhysRange::empty(),
            metadata_storage_linear: 0,
            linear_map_virt_start: 0,
            metadata_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn start_pfn(&self) -> Pfn {
        Pfn::new(self.start_pfn)
    }

    pub const fn end_pfn(&self) -> Pfn {
        Pfn::new(self.end_pfn)
    }

    pub const fn page_size(&self) -> usize {
        self.page_size
    }

    pub const fn metadata_count(&self) -> usize {
        self.metadata_count
    }

    pub const fn metadata_bytes(&self) -> usize {
        self.metadata_bytes
    }

    pub const fn metadata_storage_size(&self) -> usize {
        self.metadata_storage_size
    }

    pub const fn metadata_storage(&self) -> PhysRange {
        self.metadata_storage
    }

    pub const fn metadata_storage_linear(&self) -> usize {
        self.metadata_storage_linear
    }

    pub fn setup(
        &mut self,
        memblock: &mut MemBlock,
        zones: &Zones,
        vm: &Vm,
        config: &Config,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || zones.state() != State::Ready
            || !vm.entry_successor_ready()
            || config.state() != State::Online
            || config.page_size() == 0
            || !config.page_size().is_power_of_two()
        {
            return self.failed_setup();
        }

        let Some((start, end)) = zone_phys_bounds(zones) else {
            return self.failed_setup();
        };
        if start >= end
            || !start.is_multiple_of(config.page_size())
            || !end.is_multiple_of(config.page_size())
        {
            return self.failed_setup();
        }

        let Some(start_pfn) = phys_to_pfn_value(start, config.page_size()) else {
            return self.failed_setup();
        };
        let Some(end_pfn) = phys_to_pfn_value(end, config.page_size()) else {
            return self.failed_setup();
        };
        let Some(metadata_count) = end_pfn.checked_sub(start_pfn) else {
            return self.failed_setup();
        };
        if metadata_count == 0 {
            return self.failed_setup();
        }
        let Some(metadata_bytes) = metadata_count.checked_mul(core::mem::size_of::<PageMetadata>())
        else {
            return self.failed_setup();
        };
        let Some(metadata_storage_size) = round_up_value(metadata_bytes, config.page_size()) else {
            return self.failed_setup();
        };
        let Some(storage) = memblock.alloc_phys(metadata_storage_size, config.page_size()) else {
            return self.failed_setup();
        };
        let Some(metadata_storage_linear) = config.phys_to_linear(storage.start()) else {
            return self.failed_setup();
        };
        let Some(storage_end) = storage.start().checked_add(metadata_storage_size) else {
            return self.failed_setup();
        };
        if storage.end() != storage_end || !storage.start().is_multiple_of(config.page_size()) {
            return self.failed_setup();
        }

        self.start_pfn = start_pfn;
        self.end_pfn = end_pfn;
        self.page_size = config.page_size();
        self.metadata_bytes = metadata_bytes;
        self.metadata_storage_size = metadata_storage_size;
        self.metadata_storage = storage;
        self.metadata_storage_linear = metadata_storage_linear;
        self.linear_map_virt_start = config.linear_map_virt_start();
        self.metadata_count = metadata_count;
        if !self.initialize_metadata() {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PageMetadataMapReady,
        )
    }

    pub fn contains_pfn(&self, pfn: Pfn) -> bool {
        self.lifecycle.state() == State::Ready
            && pfn.value() >= self.start_pfn
            && pfn.value() < self.end_pfn
    }

    pub fn pfn_to_page(&self, pfn: Pfn) -> Option<PageRef> {
        if !self.contains_pfn(pfn) {
            return None;
        }
        self.page_ref_from_pfn(pfn.value())
    }

    pub fn phys_to_page(&self, phys: PhysPageAddr) -> Option<PageRef> {
        if self.lifecycle.state() != State::Ready || self.page_size == 0 {
            return None;
        }
        if phys.value() % self.page_size != 0 {
            return None;
        }
        let pfn = phys_to_pfn_value(phys.value(), self.page_size)?;
        self.pfn_to_page(Pfn::new(pfn))
    }

    pub fn virt_to_page(&self, linear: LinearMappedPageAddr) -> Option<PageRef> {
        if self.lifecycle.state() != State::Ready {
            return None;
        }
        let phys = linear.value().checked_sub(self.linear_map_virt_start)?;
        self.phys_to_page(PhysPageAddr::new(phys))
    }

    pub fn page_to_pfn(&self, page: PageRef) -> Option<Pfn> {
        self.validate_page(page).then_some(page.pfn())
    }

    pub fn page_to_phys(&self, page: PageRef) -> Option<PhysPageAddr> {
        self.validate_page(page).then_some(page.phys())
    }

    pub fn page_to_virt(&self, page: PageRef) -> Option<LinearMappedPageAddr> {
        self.validate_page(page).then_some(page.linear())
    }

    pub fn page_address(&self, page: PageRef) -> Option<usize> {
        self.page_to_virt(page).map(|addr| addr.value())
    }

    pub fn page_metadata_pfn(&self, page: PageRef) -> Option<Pfn> {
        Some(self.page_metadata(page)?.pfn())
    }

    pub fn page_metadata(&self, page: PageRef) -> Option<PageMetadata> {
        if !self.validate_page(page) {
            return None;
        }
        Some(unsafe { (page.metadata_linear() as *const PageMetadata).read() })
    }

    pub fn page_metadata_is_buddy_free(&self, page: PageRef) -> Option<bool> {
        Some(self.page_metadata(page)?.is_buddy_free())
    }

    pub fn page_metadata_buddy_order(&self, page: PageRef) -> Option<usize> {
        let metadata = self.page_metadata(page)?;
        metadata.is_buddy_free().then_some(metadata.buddy_order())
    }

    fn page_ref_from_pfn(&self, pfn: usize) -> Option<PageRef> {
        let metadata_index = pfn.checked_sub(self.start_pfn)?;
        let metadata_offset = metadata_index.checked_mul(core::mem::size_of::<PageMetadata>())?;
        let metadata_linear = self.metadata_storage_linear.checked_add(metadata_offset)?;
        let phys = pfn.checked_mul(self.page_size)?;
        let linear = phys.checked_add(self.linear_map_virt_start)?;
        Some(PageRef {
            pfn: Pfn::new(pfn),
            metadata_index,
            metadata_linear,
            phys: PhysPageAddr::new(phys),
            linear: LinearMappedPageAddr::new(linear),
        })
    }

    fn initialize_metadata(&self) -> bool {
        if self.metadata_storage_linear == 0 || self.metadata_count == 0 {
            return false;
        }

        let base = self.metadata_storage_linear as *mut PageMetadata;
        let mut index = 0usize;
        while index < self.metadata_count {
            let Some(pfn) = self.start_pfn.checked_add(index) else {
                return false;
            };
            unsafe {
                base.add(index).write(PageMetadata::new(pfn));
            }
            index += 1;
        }
        true
    }

    fn validate_page(&self, page: PageRef) -> bool {
        self.pfn_to_page(page.pfn()) == Some(page)
    }

    fn page_ref_from_metadata_index(&self, metadata_index: usize) -> Option<PageRef> {
        if metadata_index >= self.metadata_count {
            return None;
        }
        let pfn = self.start_pfn.checked_add(metadata_index)?;
        self.page_ref_from_pfn(pfn)
    }

    fn metadata_by_index(&self, metadata_index: usize) -> Option<PageMetadata> {
        let page = self.page_ref_from_metadata_index(metadata_index)?;
        self.page_metadata(page)
    }

    fn write_metadata_by_index(&self, metadata_index: usize, metadata: PageMetadata) -> bool {
        let Some(page) = self.page_ref_from_metadata_index(metadata_index) else {
            return false;
        };
        unsafe {
            (page.metadata_linear() as *mut PageMetadata).write(metadata);
        }
        true
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

#[derive(Clone, Copy)]
pub struct BuddyFreeBlock {
    zone: ZoneKind,
    order: usize,
    page: PageRef,
}

impl BuddyFreeBlock {
    pub const fn zone(self) -> ZoneKind {
        self.zone
    }

    pub const fn order(self) -> usize {
        self.order
    }

    pub const fn page(self) -> PageRef {
        self.page
    }

    pub const fn pages(self) -> usize {
        1usize << self.order
    }
}

#[derive(Clone, Copy)]
struct BuddyFreeArea {
    head_index: usize,
    nr_free: usize,
}

impl BuddyFreeArea {
    const fn empty() -> Self {
        Self {
            head_index: BUDDY_INVALID_INDEX,
            nr_free: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct BuddyFreePageSets {
    ready: bool,
    zone_kinds: [Option<ZoneKind>; MAX_BOOT_ZONES],
    zone_count: usize,
    areas: [[BuddyFreeArea; BUDDY_ORDER_COUNT]; MAX_BOOT_ZONES],
    total_free_pages: usize,
    free_block_count: usize,
}

impl BuddyFreePageSets {
    const fn new() -> Self {
        Self {
            ready: false,
            zone_kinds: [None; MAX_BOOT_ZONES],
            zone_count: 0,
            areas: [[BuddyFreeArea::empty(); BUDDY_ORDER_COUNT]; MAX_BOOT_ZONES],
            total_free_pages: 0,
            free_block_count: 0,
        }
    }

    const fn ready(&self) -> bool {
        self.ready
    }

    const fn total_free_pages(&self) -> usize {
        self.total_free_pages
    }

    const fn free_block_count(&self) -> usize {
        self.free_block_count
    }

    fn setup(
        &mut self,
        memblock: &MemBlock,
        zone_facts: &[ZoneRef; MAX_BOOT_ZONES],
        zone_fact_count: usize,
        page_metadata_map: &PageMetadataMap,
        page_size: usize,
    ) -> bool {
        if memblock.state() != State::Online
            || page_metadata_map.state() != State::Ready
            || zone_fact_count == 0
            || page_size == 0
            || !page_size.is_power_of_two()
        {
            return false;
        }

        *self = Self::new();
        self.zone_count = zone_fact_count;
        let mut zone_index = 0usize;
        while zone_index < zone_fact_count {
            self.zone_kinds[zone_index] = Some(zone_facts[zone_index].kind());
            zone_index += 1;
        }

        let usable = memblock.usable_ranges();
        let mut range_index = 0usize;
        while range_index < usable.count() {
            let Some(range) = usable.get(range_index) else {
                return false;
            };
            zone_index = 0;
            while zone_index < zone_fact_count {
                let zone_range = zone_facts[zone_index].range();
                let start = range.start().max(zone_range.start());
                let end = range.end().min(zone_range.end());
                if start < end
                    && !self.populate_unreserved_segment(
                        zone_index,
                        start,
                        end,
                        memblock,
                        page_metadata_map,
                        page_size,
                    )
                {
                    return false;
                }
                zone_index += 1;
            }
            range_index += 1;
        }

        self.ready = self.total_free_pages != 0 && self.free_block_count != 0;
        self.ready
    }

    fn zone_free_pages(&self, kind: ZoneKind) -> Option<usize> {
        let zone_index = self.zone_index(kind)?;
        let mut pages = 0usize;
        let mut order = 0usize;
        while order < BUDDY_ORDER_COUNT {
            let block_pages = 1usize.checked_shl(order as u32)?;
            pages = pages.checked_add(
                self.areas[zone_index][order]
                    .nr_free
                    .checked_mul(block_pages)?,
            )?;
            order += 1;
        }
        Some(pages)
    }

    fn order_free_count(&self, kind: ZoneKind, order: usize) -> Option<usize> {
        if order >= BUDDY_ORDER_COUNT {
            return None;
        }
        Some(self.areas[self.zone_index(kind)?][order].nr_free)
    }

    fn alloc_pages(
        &mut self,
        zonelist: &BootZonelistSet,
        order: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        if !self.ready
            || zonelist.state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || order >= BUDDY_ORDER_COUNT
        {
            return None;
        }

        let mut fallback_index = 0usize;
        while fallback_index < zonelist.fallback_count() {
            let Some(zone_kind) = zonelist.fallback_zone_kind(fallback_index) else {
                return None;
            };
            let Some(zone_index) = self.zone_index(zone_kind) else {
                fallback_index += 1;
                continue;
            };

            let mut current_order = order;
            while current_order < BUDDY_ORDER_COUNT {
                if self.areas[zone_index][current_order].nr_free != 0 {
                    let page =
                        self.remove_head_free_block(zone_index, current_order, page_metadata_map)?;
                    while current_order > order {
                        current_order -= 1;
                        let split_pfn = page.pfn().value().checked_add(1usize << current_order)?;
                        if !self.add_free_block(
                            zone_index,
                            current_order,
                            split_pfn,
                            page_metadata_map,
                        ) {
                            return None;
                        }
                    }

                    let mut metadata =
                        page_metadata_map.metadata_by_index(page.metadata_index())?;
                    if metadata.is_buddy_free() || metadata.is_buddy_allocated() {
                        return None;
                    }
                    metadata.mark_buddy_allocated(order);
                    if !page_metadata_map.write_metadata_by_index(page.metadata_index(), metadata) {
                        return None;
                    }
                    return Some(page);
                }
                current_order += 1;
            }
            fallback_index += 1;
        }

        None
    }

    fn free_pages(
        &mut self,
        zone_index: usize,
        zone_range: PhysRange,
        page_size: usize,
        page: PageRef,
        order: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if !self.ready
            || zone_index >= self.zone_count
            || order >= BUDDY_ORDER_COUNT
            || page_size == 0
            || !page_size.is_power_of_two()
            || !block_fits_in_range(page.pfn().value(), order, page_size, zone_range)
        {
            return false;
        }

        let Some(mut metadata) = page_metadata_map.metadata_by_index(page.metadata_index()) else {
            return false;
        };
        if !metadata.is_buddy_allocated() || metadata.buddy_order() != order {
            return false;
        }
        metadata.clear_buddy_state();
        if !page_metadata_map.write_metadata_by_index(page.metadata_index(), metadata) {
            return false;
        }

        let mut pfn = page.pfn().value();
        let mut current_order = order;
        while current_order + 1 < BUDDY_ORDER_COUNT {
            let buddy_pfn = pfn ^ (1usize << current_order);
            if !block_fits_in_range(buddy_pfn, current_order, page_size, zone_range) {
                break;
            }
            let Some(buddy_page) = page_metadata_map.pfn_to_page(Pfn::new(buddy_pfn)) else {
                break;
            };
            let Some(buddy_metadata) =
                page_metadata_map.metadata_by_index(buddy_page.metadata_index())
            else {
                return false;
            };
            if !buddy_metadata.is_buddy_free() || buddy_metadata.buddy_order() != current_order {
                break;
            }
            if self
                .remove_free_block_by_index(
                    zone_index,
                    current_order,
                    buddy_page.metadata_index(),
                    page_metadata_map,
                )
                .is_none()
            {
                return false;
            }
            pfn = pfn.min(buddy_pfn);
            current_order += 1;
        }

        self.add_free_block(zone_index, current_order, pfn, page_metadata_map)
    }

    fn first_free_block(&self, page_metadata_map: &PageMetadataMap) -> Option<BuddyFreeBlock> {
        let mut zone_index = 0usize;
        while zone_index < self.zone_count {
            let zone = self.zone_kinds[zone_index]?;
            let mut order = 0usize;
            while order < BUDDY_ORDER_COUNT {
                let area = self.areas[zone_index][order];
                if area.head_index != BUDDY_INVALID_INDEX {
                    return Some(BuddyFreeBlock {
                        zone,
                        order,
                        page: page_metadata_map.page_ref_from_metadata_index(area.head_index)?,
                    });
                }
                order += 1;
            }
            zone_index += 1;
        }
        None
    }

    fn remove_head_free_block(
        &mut self,
        zone_index: usize,
        order: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        if zone_index >= self.zone_count || order >= BUDDY_ORDER_COUNT {
            return None;
        }
        let head_index = self.areas[zone_index][order].head_index;
        if head_index == BUDDY_INVALID_INDEX {
            return None;
        }
        self.remove_free_block_by_index(zone_index, order, head_index, page_metadata_map)
    }

    fn remove_free_block_by_index(
        &mut self,
        zone_index: usize,
        order: usize,
        metadata_index: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        if zone_index >= self.zone_count || order >= BUDDY_ORDER_COUNT {
            return None;
        }
        let mut metadata = page_metadata_map.metadata_by_index(metadata_index)?;
        if !metadata.is_buddy_free() || metadata.buddy_order() != order {
            return None;
        }

        let prev = metadata.buddy_prev;
        let next = metadata.buddy_next;
        if prev == BUDDY_INVALID_INDEX {
            if self.areas[zone_index][order].head_index != metadata_index {
                return None;
            }
            self.areas[zone_index][order].head_index = next;
        } else {
            let mut prev_metadata = page_metadata_map.metadata_by_index(prev)?;
            if prev_metadata.buddy_next != metadata_index {
                return None;
            }
            prev_metadata.buddy_next = next;
            if !page_metadata_map.write_metadata_by_index(prev, prev_metadata) {
                return None;
            }
        }
        if next != BUDDY_INVALID_INDEX {
            let mut next_metadata = page_metadata_map.metadata_by_index(next)?;
            if next_metadata.buddy_prev != metadata_index {
                return None;
            }
            next_metadata.buddy_prev = prev;
            if !page_metadata_map.write_metadata_by_index(next, next_metadata) {
                return None;
            }
        }

        metadata.clear_buddy_state();
        if !page_metadata_map.write_metadata_by_index(metadata_index, metadata) {
            return None;
        }

        let block_pages = 1usize.checked_shl(order as u32)?;
        self.areas[zone_index][order].nr_free =
            self.areas[zone_index][order].nr_free.checked_sub(1)?;
        self.total_free_pages = self.total_free_pages.checked_sub(block_pages)?;
        self.free_block_count = self.free_block_count.checked_sub(1)?;
        page_metadata_map.page_ref_from_metadata_index(metadata_index)
    }

    fn zone_index(&self, kind: ZoneKind) -> Option<usize> {
        let mut index = 0usize;
        while index < self.zone_count {
            if self.zone_kinds[index] == Some(kind) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn populate_unreserved_segment(
        &mut self,
        zone_index: usize,
        start: usize,
        end: usize,
        memblock: &MemBlock,
        page_metadata_map: &PageMetadataMap,
        page_size: usize,
    ) -> bool {
        let mut cursor = start;
        while cursor < end {
            let Some(reserved) = next_reserved_overlap(memblock, cursor, end) else {
                return self.populate_free_segment(
                    zone_index,
                    cursor,
                    end,
                    page_metadata_map,
                    page_size,
                );
            };

            if reserved.start() > cursor
                && !self.populate_free_segment(
                    zone_index,
                    cursor,
                    reserved.start(),
                    page_metadata_map,
                    page_size,
                )
            {
                return false;
            }
            cursor = cursor.max(reserved.end());
        }
        true
    }

    fn populate_free_segment(
        &mut self,
        zone_index: usize,
        start: usize,
        end: usize,
        page_metadata_map: &PageMetadataMap,
        page_size: usize,
    ) -> bool {
        let Some(aligned_start) = round_up_value(start, page_size) else {
            return false;
        };
        let Some(aligned_end) = round_down_value(end, page_size) else {
            return false;
        };
        if aligned_start >= aligned_end {
            return true;
        }

        let Some(mut pfn) = phys_to_pfn_value(aligned_start, page_size) else {
            return false;
        };
        let Some(end_pfn) = phys_to_pfn_value(aligned_end, page_size) else {
            return false;
        };
        while pfn < end_pfn {
            let remaining = end_pfn - pfn;
            let order = largest_buddy_order(pfn, remaining);
            if !self.add_free_block(zone_index, order, pfn, page_metadata_map) {
                return false;
            }
            pfn += 1usize << order;
        }
        true
    }

    fn add_free_block(
        &mut self,
        zone_index: usize,
        order: usize,
        pfn: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if zone_index >= self.zone_count || order >= BUDDY_ORDER_COUNT {
            return false;
        }
        let Some(page) = page_metadata_map.pfn_to_page(Pfn::new(pfn)) else {
            return false;
        };
        let metadata_index = page.metadata_index();
        let Some(mut metadata) = page_metadata_map.metadata_by_index(metadata_index) else {
            return false;
        };
        if metadata.is_buddy_free() || metadata.is_buddy_allocated() {
            return false;
        }

        let area = &mut self.areas[zone_index][order];
        if area.head_index != BUDDY_INVALID_INDEX {
            let Some(mut old_head) = page_metadata_map.metadata_by_index(area.head_index) else {
                return false;
            };
            old_head.buddy_prev = metadata_index;
            if !page_metadata_map.write_metadata_by_index(area.head_index, old_head) {
                return false;
            }
        }

        metadata.mark_buddy_free(order, BUDDY_INVALID_INDEX, area.head_index);
        if !page_metadata_map.write_metadata_by_index(metadata_index, metadata) {
            return false;
        }

        let Some(nr_free) = area.nr_free.checked_add(1) else {
            return false;
        };
        let Some(block_pages) = 1usize.checked_shl(order as u32) else {
            return false;
        };
        let Some(total_free_pages) = self.total_free_pages.checked_add(block_pages) else {
            return false;
        };
        let Some(free_block_count) = self.free_block_count.checked_add(1) else {
            return false;
        };
        area.head_index = metadata_index;
        area.nr_free = nr_free;
        self.total_free_pages = total_free_pages;
        self.free_block_count = free_block_count;
        true
    }
}

pub struct PageAllocator {
    lifecycle: Lifecycle,
    boot_zonelist_set: BootZonelistSet,
    buddy_free_page_sets: BuddyFreePageSets,
    page_metadata_map_bound: bool,
    cpuhp_step_registered: bool,
    boot_pageset_checkpoint_ready: bool,
    handoff_complete: bool,
    full_gfp_mask_open: bool,
    late_ready: bool,
    memory_stats_ready: bool,
    buffer_init_ready: bool,
    memblock_private_discarded: bool,
    zone_contiguous_ready: bool,
    sysctl_ready: bool,
    deferred_struct_page_init_trimmed: bool,
    page_extension_late_trimmed: bool,
    shuffle_late_trimmed: bool,
    totalram_pages: usize,
    zone_facts: [ZoneRef; MAX_BOOT_ZONES],
    zone_fact_count: usize,
}

impl PageAllocator {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_zonelist_set: BootZonelistSet::new(),
            buddy_free_page_sets: BuddyFreePageSets::new(),
            page_metadata_map_bound: false,
            cpuhp_step_registered: false,
            boot_pageset_checkpoint_ready: false,
            handoff_complete: false,
            full_gfp_mask_open: false,
            late_ready: false,
            memory_stats_ready: false,
            buffer_init_ready: false,
            memblock_private_discarded: false,
            zone_contiguous_ready: false,
            sysctl_ready: false,
            deferred_struct_page_init_trimmed: false,
            page_extension_late_trimmed: false,
            shuffle_late_trimmed: false,
            totalram_pages: 0,
            zone_facts: [ZoneRef::empty(); MAX_BOOT_ZONES],
            zone_fact_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn boot_zonelist_set(&self) -> &BootZonelistSet {
        &self.boot_zonelist_set
    }

    pub const fn buddy_free_page_sets_ready(&self) -> bool {
        self.buddy_free_page_sets.ready()
    }

    pub const fn buddy_total_free_pages(&self) -> usize {
        self.buddy_free_page_sets.total_free_pages()
    }

    pub const fn buddy_free_block_count(&self) -> usize {
        self.buddy_free_page_sets.free_block_count()
    }

    pub const fn page_metadata_map_bound(&self) -> bool {
        self.page_metadata_map_bound
    }

    pub const fn cpuhp_step_registered(&self) -> bool {
        self.cpuhp_step_registered
    }

    pub const fn boot_pageset_checkpoint_ready(&self) -> bool {
        self.boot_pageset_checkpoint_ready
    }

    pub const fn handoff_complete(&self) -> bool {
        self.handoff_complete
    }

    pub const fn full_gfp_mask_open(&self) -> bool {
        self.full_gfp_mask_open
    }

    pub const fn late_ready(&self) -> bool {
        self.late_ready
    }

    pub const fn memory_stats_ready(&self) -> bool {
        self.memory_stats_ready
    }

    pub const fn buffer_init_ready(&self) -> bool {
        self.buffer_init_ready
    }

    pub const fn memblock_private_discarded(&self) -> bool {
        self.memblock_private_discarded
    }

    pub const fn zone_contiguous_ready(&self) -> bool {
        self.zone_contiguous_ready
    }

    pub const fn sysctl_ready(&self) -> bool {
        self.sysctl_ready
    }

    pub const fn deferred_struct_page_init_trimmed(&self) -> bool {
        self.deferred_struct_page_init_trimmed
    }

    pub const fn page_extension_late_trimmed(&self) -> bool {
        self.page_extension_late_trimmed
    }

    pub const fn shuffle_late_trimmed(&self) -> bool {
        self.shuffle_late_trimmed
    }

    pub const fn totalram_pages(&self) -> usize {
        self.totalram_pages
    }

    pub const fn zone_fact_count(&self) -> usize {
        self.zone_fact_count
    }

    pub fn zone_fact(&self, index: usize) -> Option<ZoneRef> {
        if index < self.zone_fact_count {
            Some(self.zone_facts[index])
        } else {
            None
        }
    }

    pub fn zone_managed_pages(&self, kind: ZoneKind) -> Option<usize> {
        self.zone_fact_by_kind(kind)
            .map(|zone| zone.managed_pages())
    }

    pub fn zone_free_pages(&self, kind: ZoneKind) -> Option<usize> {
        self.zone_fact_by_kind(kind).map(|zone| zone.free_pages())
    }

    pub fn buddy_zone_free_pages(&self, kind: ZoneKind) -> Option<usize> {
        self.buddy_free_page_sets.zone_free_pages(kind)
    }

    pub fn buddy_order_free_count(&self, kind: ZoneKind, order: usize) -> Option<usize> {
        self.buddy_free_page_sets.order_free_count(kind, order)
    }

    pub fn first_buddy_free_block(
        &self,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<BuddyFreeBlock> {
        self.buddy_free_page_sets
            .first_free_block(page_metadata_map)
    }

    pub fn alloc_pages(
        &mut self,
        order: usize,
        gfp: GfpFlags,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        if self.lifecycle.state() != State::Ready
            || !self.handoff_complete
            || !self.page_metadata_map_bound
            || !gfp.boot_buddy_allowed()
        {
            return None;
        }

        let page = self.buddy_free_page_sets.alloc_pages(
            &self.boot_zonelist_set,
            order,
            page_metadata_map,
        )?;
        if !self.sync_zone_free_pages() {
            return None;
        }
        Some(page)
    }

    pub fn alloc_page(
        &mut self,
        gfp: GfpFlags,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        self.alloc_pages(0, gfp, page_metadata_map)
    }

    pub fn free_pages(
        &mut self,
        page: PageRef,
        order: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.handoff_complete
            || !self.page_metadata_map_bound
            || order >= BUDDY_ORDER_COUNT
        {
            return false;
        }

        let Some(zone_index) = self.zone_fact_index_for_page(page, order, page_metadata_map) else {
            return false;
        };
        let zone_range = self.zone_facts[zone_index].range();
        if !self.buddy_free_page_sets.free_pages(
            zone_index,
            zone_range,
            page_metadata_map.page_size(),
            page,
            order,
            page_metadata_map,
        ) {
            return false;
        }
        self.sync_zone_free_pages()
    }

    pub fn preset(
        &mut self,
        topology: &MemoryTopology,
        page_metadata_map: &PageMetadataMap,
        cpu_hotplug_state: &CpuHotplugState,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || topology.state() != State::Ready
            || topology.boot_zone_set().state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || page_metadata_map.metadata_count() == 0
            || cpu_hotplug_state.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.boot_zonelist_set.setup(topology)?;
        self.page_metadata_map_bound = true;
        self.cpuhp_step_registered = PAGE_ALLOC_CPUHP_STEP != 0;
        self.boot_pageset_checkpoint_ready = true;

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PageAllocatorPrepared,
        )
    }

    pub fn setup(
        &mut self,
        memblock: &mut MemBlock,
        zones: &Zones,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        memory_debug_hardening: &MemoryDebugHardening,
        swiotlb: &Swiotlb,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || memblock.state() != State::Online
            || zones.state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || self.boot_zonelist_set.state() != State::Ready
            || memory_debug_hardening.state() != State::Ready
            || swiotlb.state() != State::Ready
        {
            return self.failed_setup();
        }

        let Some(zone_facts) = build_zone_facts(zones, config) else {
            return self.failed_setup();
        };
        self.zone_facts = zone_facts.facts;
        self.zone_fact_count = zone_facts.count;
        self.totalram_pages = zone_facts.total_pages;

        if !self.buddy_free_page_sets.setup(
            memblock,
            &self.zone_facts,
            self.zone_fact_count,
            page_metadata_map,
            config.page_size(),
        ) {
            return self.failed_setup();
        }

        if !self.sync_zone_free_pages() {
            return self.failed_setup();
        }

        self.handoff_complete = self.totalram_pages != 0
            && self.zone_fact_count != 0
            && self.buddy_free_page_sets.ready()
            && self.buddy_free_page_sets.total_free_pages() != 0;
        if !self.handoff_complete {
            return self.failed_setup();
        }

        memblock.disable()?;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::PageAllocatorReady,
        )
    }

    pub fn open_full_gfp_mask(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.handoff_complete {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.full_gfp_mask_open = true;
        crate::trace::checkpoint(Checkpoint::PageAllocatorFullGfpMaskOpen);
        Ok(())
    }

    pub fn setup_late(&mut self, workqueue: &Workqueue, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.handoff_complete
            || !self.full_gfp_mask_open
            || workqueue.state() != State::Ready
            || !workqueue.topology_ready()
            || cpu_group.state() != State::Ready
            || !cpu_group.smp_concurrency_open()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.late_ready = true;
        self.memory_stats_ready = true;
        self.buffer_init_ready = true;
        self.memblock_private_discarded = true;
        self.zone_contiguous_ready = true;
        self.sysctl_ready = true;
        self.deferred_struct_page_init_trimmed = true;
        self.page_extension_late_trimmed = true;
        self.shuffle_late_trimmed = true;
        crate::trace::checkpoint(Checkpoint::PageAllocatorLateReady);
        Ok(())
    }

    fn zone_fact_by_kind(&self, kind: ZoneKind) -> Option<ZoneRef> {
        let mut index = 0usize;
        while index < self.zone_fact_count {
            if self.zone_facts[index].kind() == kind {
                return Some(self.zone_facts[index]);
            }
            index += 1;
        }
        None
    }

    fn zone_fact_index_for_page(
        &self,
        page: PageRef,
        order: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<usize> {
        if order >= BUDDY_ORDER_COUNT || page_metadata_map.page_to_pfn(page)? != page.pfn() {
            return None;
        }

        let mut index = 0usize;
        while index < self.zone_fact_count {
            if block_fits_in_range(
                page.pfn().value(),
                order,
                page_metadata_map.page_size(),
                self.zone_facts[index].range(),
            ) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn sync_zone_free_pages(&mut self) -> bool {
        let mut zone_index = 0usize;
        while zone_index < self.zone_fact_count {
            let kind = self.zone_facts[zone_index].kind();
            let Some(free_pages) = self.buddy_free_page_sets.zone_free_pages(kind) else {
                return false;
            };
            self.zone_facts[zone_index].free_pages = free_pages;
            zone_index += 1;
        }
        true
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KmallocAllocRef {
    addr: usize,
    requested_size: usize,
    cache_size: usize,
    cache_index: usize,
}

impl KmallocAllocRef {
    pub const fn addr(self) -> usize {
        self.addr
    }

    pub const fn requested_size(self) -> usize {
        self.requested_size
    }

    pub const fn cache_size(self) -> usize {
        self.cache_size
    }

    pub const fn cache_index(self) -> usize {
        self.cache_index
    }
}

#[derive(Clone, Copy)]
struct KmallocSlab {
    page: PageRef,
    order: usize,
    base: usize,
    object_size: usize,
    object_count: usize,
    free_count: usize,
    cache_index: usize,
}

impl KmallocSlab {
    const fn empty() -> Self {
        Self {
            page: empty_page_ref(),
            order: 0,
            base: 0,
            object_size: 0,
            object_count: 0,
            free_count: 0,
            cache_index: usize::MAX,
        }
    }

    fn contains(&self, addr: usize) -> bool {
        let Some(bytes) = self.object_size.checked_mul(self.object_count) else {
            return false;
        };
        let Some(end) = self.base.checked_add(bytes) else {
            return false;
        };
        self.base != 0
            && self.object_size != 0
            && addr >= self.base
            && addr < end
            && (addr - self.base).is_multiple_of(self.object_size)
    }
}

#[derive(Clone, Copy)]
struct KmallocCache {
    object_size: usize,
    free_head: usize,
    slab_count: usize,
}

impl KmallocCache {
    const fn empty() -> Self {
        Self {
            object_size: 0,
            free_head: KMALLOC_NULL,
            slab_count: 0,
        }
    }
}

pub struct MemoryDebugHardening {
    lifecycle: Lifecycle,
    init_on_alloc: bool,
    init_on_free: bool,
    debug_pagealloc: bool,
    debug_guardpage: bool,
    check_pages: bool,
}

impl MemoryDebugHardening {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            init_on_alloc: false,
            init_on_free: false,
            debug_pagealloc: false,
            debug_guardpage: false,
            check_pages: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn init_on_alloc(&self) -> bool {
        self.init_on_alloc
    }

    pub const fn init_on_free(&self) -> bool {
        self.init_on_free
    }

    pub const fn debug_pagealloc(&self) -> bool {
        self.debug_pagealloc
    }

    pub const fn debug_guardpage(&self) -> bool {
        self.debug_guardpage
    }

    pub const fn check_pages(&self) -> bool {
        self.check_pages
    }

    pub fn setup(
        &mut self,
        static_branch: &mut StaticBranch,
        early_param: &EarlyParam,
        config: &Config,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || static_branch.state() != State::Ready
            || early_param.state() != State::Ready
            || config.state() != State::Online
        {
            return self.failed_setup();
        }

        self.init_on_alloc = false;
        self.init_on_free = false;
        self.debug_pagealloc = false;
        self.debug_guardpage = false;
        self.check_pages = true;

        static_branch.set(StaticKey::InitOnAlloc, self.init_on_alloc)?;
        static_branch.set(StaticKey::InitOnFree, self.init_on_free)?;
        static_branch.set(StaticKey::DebugPageAlloc, self.debug_pagealloc)?;
        static_branch.set(StaticKey::DebugGuardPage, self.debug_guardpage)?;
        static_branch.set(StaticKey::CheckPages, self.check_pages)?;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::MemoryDebugHardeningReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct StackDepot {
    lifecycle: Lifecycle,
    early_storage_ready: bool,
}

impl StackDepot {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            early_storage_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn early_storage_ready(&self) -> bool {
        self.early_storage_ready
    }

    pub fn setup(
        &mut self,
        memblock: &MemBlock,
        memory_debug_hardening: &MemoryDebugHardening,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || memory_debug_hardening.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.early_storage_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::StackDepotReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct Swiotlb {
    lifecycle: Lifecycle,
    pool_required: bool,
    early_pool_ready: bool,
    dynamic_growth_trimmed: bool,
}

impl Swiotlb {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pool_required: false,
            early_pool_ready: false,
            dynamic_growth_trimmed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pool_required(&self) -> bool {
        self.pool_required
    }

    pub const fn early_pool_ready(&self) -> bool {
        self.early_pool_ready
    }

    pub fn setup(
        &mut self,
        dma_cache_policy: &DmaCachePolicy,
        memblock: &MemBlock,
        zones: &Zones,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || dma_cache_policy.state() != State::Ready
            || memblock.state() != State::Online
            || zones.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.pool_required = dma_cache_policy.noncoherent_supported()
            && dma_cache_policy.cache_alignment() > 1
            && zone_present_pages(zones, 4096).unwrap_or(0) != 0;
        self.early_pool_ready = true;
        self.dynamic_growth_trimmed = true;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SwiotlbReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SlubState {
    Down,
    Partial,
    Up,
}

pub struct SlubAllocator {
    lifecycle: Lifecycle,
    slab_state: SlubState,
    boot_kmem_cache_node_ready: bool,
    bootstrap_completed: bool,
    cpu_cache_ready: bool,
    cpuhp_step_registered: bool,
    flush_workqueue_ready: bool,
    cache_registry: SlubCacheRegistry,
    kmalloc_caches: KmallocCaches,
}

impl SlubAllocator {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            slab_state: SlubState::Down,
            boot_kmem_cache_node_ready: false,
            bootstrap_completed: false,
            cpu_cache_ready: false,
            cpuhp_step_registered: false,
            flush_workqueue_ready: false,
            cache_registry: SlubCacheRegistry::new(),
            kmalloc_caches: KmallocCaches::new(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn slab_state(&self) -> SlubState {
        self.slab_state
    }

    pub const fn boot_kmem_cache_node_ready(&self) -> bool {
        self.boot_kmem_cache_node_ready
    }

    pub const fn bootstrap_completed(&self) -> bool {
        self.bootstrap_completed
    }

    pub const fn cpu_cache_ready(&self) -> bool {
        self.cpu_cache_ready
    }

    pub const fn cpuhp_step_registered(&self) -> bool {
        self.cpuhp_step_registered
    }

    pub const fn flush_workqueue_ready(&self) -> bool {
        self.flush_workqueue_ready
    }

    pub const fn cache_registry(&self) -> &SlubCacheRegistry {
        &self.cache_registry
    }

    pub const fn kmalloc_caches(&self) -> &KmallocCaches {
        &self.kmalloc_caches
    }

    pub fn kmalloc(
        &mut self,
        size: usize,
        gfp: GfpFlags,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<KmallocAllocRef> {
        if self.lifecycle.state() != State::Ready || self.slab_state != SlubState::Up {
            return None;
        }
        self.kmalloc_caches
            .kmalloc(size, gfp, page_allocator, page_metadata_map, false)
    }

    pub fn kzalloc(
        &mut self,
        size: usize,
        gfp: GfpFlags,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<KmallocAllocRef> {
        if self.lifecycle.state() != State::Ready || self.slab_state != SlubState::Up {
            return None;
        }
        self.kmalloc_caches
            .kmalloc(size, gfp, page_allocator, page_metadata_map, true)
    }

    pub fn kfree(&mut self, alloc_ref: KmallocAllocRef) -> bool {
        if self.lifecycle.state() != State::Ready || self.slab_state != SlubState::Up {
            return false;
        }
        self.kmalloc_caches.kfree(alloc_ref)
    }

    pub fn kfree_addr(&mut self, addr: usize, size: usize) -> bool {
        if self.lifecycle.state() != State::Ready || self.slab_state != SlubState::Up {
            return false;
        }
        self.kmalloc_caches.kfree_addr(addr, size)
    }

    pub fn preset(
        &mut self,
        page_allocator: &PageAllocator,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || page_allocator.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.boot_kmem_cache_node_ready = true;
        self.slab_state = SlubState::Partial;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SlubAllocatorPrepared,
        )
    }

    pub fn setup(
        &mut self,
        page_allocator: &PageAllocator,
        stack_depot: &StackDepot,
        per_cpu_storage: &PerCpuStorage,
        cpu_hotplug_state: &CpuHotplugState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || page_allocator.state() != State::Ready
            || stack_depot.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || cpu_hotplug_state.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.cache_registry.setup(self.lifecycle.state())?;
        self.kmalloc_caches
            .setup(self.lifecycle.state(), &self.cache_registry)?;
        self.bootstrap_completed = true;
        self.cpu_cache_ready = true;
        self.cpuhp_step_registered = SLUB_CPUHP_STEP != 0;
        self.slab_state = SlubState::Up;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SlubAllocatorReady,
        )
    }

    pub fn setup_flush_workqueue(&mut self, workqueue: &Workqueue) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.kmalloc_caches.state() != State::Ready
            || workqueue.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.flush_workqueue_ready = true;
        crate::trace::checkpoint(Checkpoint::SlubFlushWorkqueueReady);
        Ok(())
    }
}

pub struct KernelGlobalAllocAdapter;

unsafe impl GlobalAlloc for KernelGlobalAllocAdapter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        global_alloc(layout, false)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        global_alloc(layout, true)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || !layout_supported(layout) {
            return;
        }
        let ctx = crate::context::context();
        if !ctx.slub_allocator.kfree_addr(ptr as usize, layout.size()) {
            crate::arch::riscv64::sbi::putstr("arceos_ex global dealloc failed\n");
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }
}

fn global_alloc(layout: Layout, zeroed: bool) -> *mut u8 {
    if !layout_supported(layout) {
        return null_mut();
    }
    let ctx = crate::context::context();
    let allocation = if zeroed {
        ctx.slub_allocator.kzalloc(
            layout.size(),
            GfpFlags::kernel(),
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    } else {
        ctx.slub_allocator.kmalloc(
            layout.size(),
            GfpFlags::kernel(),
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    };
    let Some(alloc_ref) = allocation else {
        return null_mut();
    };
    if alloc_ref.addr().is_multiple_of(layout.align()) {
        alloc_ref.addr() as *mut u8
    } else {
        let _ = ctx.slub_allocator.kfree(alloc_ref);
        null_mut()
    }
}

fn kmalloc_cache_order(object_size: usize, page_size: usize) -> Option<usize> {
    if object_size == 0 || page_size == 0 || !page_size.is_power_of_two() {
        return None;
    }

    if object_size <= page_size {
        Some(0)
    } else if object_size <= page_size.checked_mul(2)? {
        Some(1)
    } else {
        None
    }
}

const fn layout_supported(layout: Layout) -> bool {
    let Some(cache_size) = kmalloc_size_class_for(layout.size()) else {
        return false;
    };
    layout.align() <= cache_size
}

const fn kmalloc_size_class_for(size: usize) -> Option<usize> {
    if size == 0 || size > GLOBAL_ALLOC_MAX_SIZE {
        return None;
    }
    let mut cache_size = 8usize;
    while cache_size <= GLOBAL_ALLOC_MAX_SIZE {
        if size <= cache_size {
            return Some(cache_size);
        }
        cache_size <<= 1;
    }
    None
}

pub struct KernelGlobalAllocator {
    lifecycle: Lifecycle,
    alloc_api_ready: bool,
    alloc_zeroed_api_ready: bool,
    dealloc_api_ready: bool,
    uses_slub_allocator: bool,
}

impl KernelGlobalAllocator {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            alloc_api_ready: false,
            alloc_zeroed_api_ready: false,
            dealloc_api_ready: false,
            uses_slub_allocator: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn alloc_api_ready(&self) -> bool {
        self.alloc_api_ready
    }

    pub const fn alloc_zeroed_api_ready(&self) -> bool {
        self.alloc_zeroed_api_ready
    }

    pub const fn dealloc_api_ready(&self) -> bool {
        self.dealloc_api_ready
    }

    pub const fn uses_slub_allocator(&self) -> bool {
        self.uses_slub_allocator
    }

    pub fn setup(&mut self, slub_allocator: &SlubAllocator) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_allocator.state() != State::Ready
            || slub_allocator.kmalloc_caches().state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.alloc_api_ready = true;
        self.alloc_zeroed_api_ready = true;
        self.dealloc_api_ready = true;
        self.uses_slub_allocator = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KernelGlobalAllocatorReady,
        )
    }
}

pub struct DynamicContainerRuntime {
    lifecycle: Lifecycle,
    uses_global_allocator: bool,
    vec_api_ready: bool,
    list_api_ready: bool,
    set_api_ready: bool,
}

impl DynamicContainerRuntime {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            uses_global_allocator: false,
            vec_api_ready: false,
            list_api_ready: false,
            set_api_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn uses_global_allocator(&self) -> bool {
        self.uses_global_allocator
    }

    pub const fn vec_api_ready(&self) -> bool {
        self.vec_api_ready
    }

    pub const fn list_api_ready(&self) -> bool {
        self.list_api_ready
    }

    pub const fn set_api_ready(&self) -> bool {
        self.set_api_ready
    }

    pub fn setup(&mut self, global_allocator: &KernelGlobalAllocator) -> EventResult {
        if self.lifecycle.state() != State::Base || global_allocator.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.uses_global_allocator = true;
        self.vec_api_ready = true;
        self.list_api_ready = true;
        self.set_api_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DynamicContainerRuntimeReady,
        )
    }
}

pub struct SlubCacheRegistry {
    lifecycle: Lifecycle,
    boot_caches_registered: bool,
    global_list_ready: bool,
    cache_count: usize,
}

impl SlubCacheRegistry {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_caches_registered: false,
            global_list_ready: false,
            cache_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn boot_caches_registered(&self) -> bool {
        self.boot_caches_registered
    }

    pub const fn global_list_ready(&self) -> bool {
        self.global_list_ready
    }

    pub const fn cache_count(&self) -> usize {
        self.cache_count
    }

    fn setup(&mut self, slub_state: State) -> EventResult {
        if self.lifecycle.state() != State::Base || slub_state != State::Prepared {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.boot_caches_registered = true;
        self.global_list_ready = true;
        self.cache_count = 2;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SlubCacheRegistryReady,
        )
    }
}

pub struct KmallocCaches {
    lifecycle: Lifecycle,
    size_index_ready: bool,
    default_cache_ready: bool,
    random_caches_trimmed: bool,
    memcg_caches_trimmed: bool,
    sizes: [usize; MAX_KMALLOC_CACHES],
    caches: [KmallocCache; MAX_KMALLOC_CACHES],
    slabs: [KmallocSlab; MAX_KMALLOC_SLABS],
    slab_count: usize,
    count: usize,
}

impl KmallocCaches {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            size_index_ready: false,
            default_cache_ready: false,
            random_caches_trimmed: false,
            memcg_caches_trimmed: false,
            sizes: [0; MAX_KMALLOC_CACHES],
            caches: [KmallocCache::empty(); MAX_KMALLOC_CACHES],
            slabs: [KmallocSlab::empty(); MAX_KMALLOC_SLABS],
            slab_count: 0,
            count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn size_index_ready(&self) -> bool {
        self.size_index_ready
    }

    pub const fn default_cache_ready(&self) -> bool {
        self.default_cache_ready
    }

    pub const fn random_caches_trimmed(&self) -> bool {
        self.random_caches_trimmed
    }

    pub const fn memcg_caches_trimmed(&self) -> bool {
        self.memcg_caches_trimmed
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub const fn slab_count(&self) -> usize {
        self.slab_count
    }

    pub fn backing_page_count(&self) -> usize {
        let mut count = 0usize;
        let mut index = 0usize;
        while index < self.slab_count {
            if self.slabs[index].page.metadata_linear() != 0 {
                count += 1usize << self.slabs[index].order;
            }
            index += 1;
        }
        count
    }

    pub fn kmalloc_size(&self, size: usize) -> Option<usize> {
        let index = self.cache_index_for_size(size)?;
        Some(self.caches[index].object_size)
    }

    pub fn free_object_count(&self, size: usize) -> usize {
        let Some(cache_index) = self.cache_index_for_size(size) else {
            return 0;
        };
        let mut count = 0usize;
        let mut slab_index = 0usize;
        while slab_index < self.slab_count {
            if self.slabs[slab_index].cache_index == cache_index {
                count += self.slabs[slab_index].free_count;
            }
            slab_index += 1;
        }
        count
    }

    pub fn has_size(&self, size: usize) -> bool {
        let mut index = 0usize;
        while index < self.count {
            if self.sizes[index] == size {
                return true;
            }
            index += 1;
        }
        false
    }

    fn kmalloc(
        &mut self,
        size: usize,
        gfp: GfpFlags,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        zeroed: bool,
    ) -> Option<KmallocAllocRef> {
        if self.lifecycle.state() != State::Ready || size == 0 {
            return None;
        }
        let cache_index = self.cache_index_for_size(size)?;
        if self.caches[cache_index].free_head == KMALLOC_NULL
            && !self.grow_cache(cache_index, gfp, page_allocator, page_metadata_map)
        {
            return None;
        }

        let addr = self.caches[cache_index].free_head;
        if addr == KMALLOC_NULL {
            return None;
        }
        let next = read_freelist_next(addr);
        self.caches[cache_index].free_head = next;
        let Some(slab_index) = self.slab_index_for_addr(addr) else {
            return None;
        };
        if self.slabs[slab_index].free_count == 0 {
            return None;
        }
        self.slabs[slab_index].free_count -= 1;

        if zeroed {
            unsafe {
                core::ptr::write_bytes(addr as *mut u8, 0, size);
            }
        }

        Some(KmallocAllocRef {
            addr,
            requested_size: size,
            cache_size: self.caches[cache_index].object_size,
            cache_index,
        })
    }

    fn kfree(&mut self, alloc_ref: KmallocAllocRef) -> bool {
        if self.lifecycle.state() != State::Ready
            || alloc_ref.addr() == KMALLOC_NULL
            || alloc_ref.cache_index() >= self.count
            || self.caches[alloc_ref.cache_index()].object_size != alloc_ref.cache_size()
        {
            return false;
        }
        let Some(slab_index) = self.slab_index_for_addr(alloc_ref.addr()) else {
            return false;
        };
        if self.slabs[slab_index].cache_index != alloc_ref.cache_index()
            || !self.slabs[slab_index].contains(alloc_ref.addr())
            || self.slabs[slab_index].free_count >= self.slabs[slab_index].object_count
            || self.freelist_contains(alloc_ref.cache_index(), alloc_ref.addr())
        {
            return false;
        }

        let cache_index = alloc_ref.cache_index();
        write_freelist_next(alloc_ref.addr(), self.caches[cache_index].free_head);
        self.caches[cache_index].free_head = alloc_ref.addr();
        self.slabs[slab_index].free_count += 1;
        true
    }

    fn kfree_addr(&mut self, addr: usize, size: usize) -> bool {
        if self.lifecycle.state() != State::Ready || addr == KMALLOC_NULL || size == 0 {
            return false;
        }
        let Some(slab_index) = self.slab_index_for_addr(addr) else {
            return false;
        };
        let cache_index = self.slabs[slab_index].cache_index;
        if cache_index >= self.count || size > self.caches[cache_index].object_size {
            return false;
        }
        let alloc_ref = KmallocAllocRef {
            addr,
            requested_size: size,
            cache_size: self.caches[cache_index].object_size,
            cache_index,
        };
        self.kfree(alloc_ref)
    }

    fn setup(&mut self, slub_state: State, registry: &SlubCacheRegistry) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_state != State::Prepared
            || registry.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.sizes = [8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192];
        self.count = MAX_KMALLOC_CACHES;
        let mut index = 0usize;
        while index < self.count {
            self.caches[index] = KmallocCache {
                object_size: self.sizes[index],
                free_head: KMALLOC_NULL,
                slab_count: 0,
            };
            index += 1;
        }
        self.size_index_ready = true;
        self.default_cache_ready = true;
        self.random_caches_trimmed = true;
        self.memcg_caches_trimmed = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KmallocCachesReady,
        )
    }

    fn cache_index_for_size(&self, size: usize) -> Option<usize> {
        if self.lifecycle.state() != State::Ready || size == 0 {
            return None;
        }
        let mut index = 0usize;
        while index < self.count {
            if size <= self.caches[index].object_size {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn grow_cache(
        &mut self,
        cache_index: usize,
        gfp: GfpFlags,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if cache_index >= self.count || self.slab_count >= MAX_KMALLOC_SLABS {
            return false;
        }
        let object_size = self.caches[cache_index].object_size;
        let Some(order) = kmalloc_cache_order(object_size, page_metadata_map.page_size()) else {
            return false;
        };
        let Some(page) = page_allocator.alloc_pages(order, gfp, page_metadata_map) else {
            return false;
        };
        let Some(base) = page_metadata_map.page_address(page) else {
            page_allocator.free_pages(page, order, page_metadata_map);
            return false;
        };
        let slab_bytes = page_metadata_map.page_size() << order;
        let object_count = slab_bytes / object_size;
        if object_count == 0 {
            page_allocator.free_pages(page, order, page_metadata_map);
            return false;
        }

        let mut head = KMALLOC_NULL;
        let mut object_index = object_count;
        while object_index != 0 {
            object_index -= 1;
            let Some(offset) = object_index.checked_mul(object_size) else {
                page_allocator.free_pages(page, order, page_metadata_map);
                return false;
            };
            let Some(addr) = base.checked_add(offset) else {
                page_allocator.free_pages(page, order, page_metadata_map);
                return false;
            };
            write_freelist_next(addr, head);
            head = addr;
        }

        self.slabs[self.slab_count] = KmallocSlab {
            page,
            order,
            base,
            object_size,
            object_count,
            free_count: object_count,
            cache_index,
        };
        self.slab_count += 1;
        self.caches[cache_index].free_head = head;
        self.caches[cache_index].slab_count += 1;
        true
    }

    fn slab_index_for_addr(&self, addr: usize) -> Option<usize> {
        let mut index = 0usize;
        while index < self.slab_count {
            if self.slabs[index].contains(addr) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn freelist_contains(&self, cache_index: usize, addr: usize) -> bool {
        if cache_index >= self.count {
            return false;
        }
        let mut current = self.caches[cache_index].free_head;
        let mut scanned = 0usize;
        while current != KMALLOC_NULL && scanned < MAX_KMALLOC_SLABS * 512 {
            if current == addr {
                return true;
            }
            current = read_freelist_next(current);
            scanned += 1;
        }
        false
    }
}

pub struct PageTableCaches {
    lifecycle: Lifecycle,
    lock_cache: PageTableLockCache,
    vmalloc_install_range: PageTableInstallRange,
    dynamic_vmalloc_pgtable_pages: [PageRef; MAX_DYNAMIC_VMALLOC_PGTABLES],
    dynamic_vmalloc_pgtable_count: usize,
    vmalloc_pgtable_preallocated: bool,
    vmalloc_pgtable_dynamic_allocator_ready: bool,
    modules_path_trimmed: bool,
    memory_hotplug_path_trimmed: bool,
}

impl PageTableCaches {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            lock_cache: PageTableLockCache::new(),
            vmalloc_install_range: PageTableInstallRange::empty(),
            dynamic_vmalloc_pgtable_pages: [empty_page_ref(); MAX_DYNAMIC_VMALLOC_PGTABLES],
            dynamic_vmalloc_pgtable_count: 0,
            vmalloc_pgtable_preallocated: false,
            vmalloc_pgtable_dynamic_allocator_ready: false,
            modules_path_trimmed: false,
            memory_hotplug_path_trimmed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn lock_cache(&self) -> &PageTableLockCache {
        &self.lock_cache
    }

    pub const fn vmalloc_pgtable_preallocated(&self) -> bool {
        self.vmalloc_pgtable_preallocated
    }

    pub const fn vmalloc_pgtable_dynamic_allocator_ready(&self) -> bool {
        self.vmalloc_pgtable_dynamic_allocator_ready
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn dynamic_vmalloc_pgtable_count(&self) -> usize {
        self.dynamic_vmalloc_pgtable_count
    }

    pub const fn vmalloc_install_range(&self) -> PageTableInstallRange {
        self.vmalloc_install_range
    }

    pub fn setup(
        &mut self,
        slub_allocator: &SlubAllocator,
        page_allocator: &PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        vm: &Vm,
        static_objects: &StaticObjects,
        kernel_image: &KernelImage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_allocator.state() != State::Ready
            || page_allocator.state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || config.state() != State::Online
            || config.page_size() != VMALLOC_RUNTIME_PAGE_SIZE
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
        {
            return self.failed_setup();
        }

        let Some(vmalloc_install_range) =
            static_objects.swapper_vmalloc_install_range(kernel_image, VMALLOC_RUNTIME_PAGE_SIZE)
        else {
            return self.failed_setup();
        };
        self.lock_cache.setup(slub_allocator)?;
        self.vmalloc_install_range = vmalloc_install_range;
        self.vmalloc_pgtable_preallocated = true;
        self.vmalloc_pgtable_dynamic_allocator_ready = true;
        self.modules_path_trimmed = true;
        self.memory_hotplug_path_trimmed = true;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PageTableCachesReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }

    pub fn ensure_vmalloc_l0_windows(
        &mut self,
        target_count: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
    ) -> Option<PageTableInstallRange> {
        if self.lifecycle.state() != State::Ready
            || !self.vmalloc_pgtable_dynamic_allocator_ready
            || target_count == 0
            || target_count > self.vmalloc_install_range.max_l0_table_count()
            || config.page_size() == 0
            || !config.page_size().is_power_of_two()
        {
            return None;
        }

        while self.vmalloc_install_range.l0_table_count() < target_count {
            if self.dynamic_vmalloc_pgtable_count >= MAX_DYNAMIC_VMALLOC_PGTABLES {
                return None;
            }
            let page = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map)?;
            let phys = page_metadata_map.page_to_phys(page)?.value();
            let linear = config.phys_to_linear(phys)?;
            unsafe {
                core::ptr::write_bytes(linear as *mut u8, 0, config.page_size());
            }
            if !self
                .vmalloc_install_range
                .push_l0_table(linear, phys, config.page_size())
            {
                let _ = page_allocator.free_pages(page, 0, page_metadata_map);
                return None;
            }
            self.dynamic_vmalloc_pgtable_pages[self.dynamic_vmalloc_pgtable_count] = page;
            self.dynamic_vmalloc_pgtable_count += 1;
        }

        Some(self.vmalloc_install_range)
    }
}

pub struct PageTableLockCache {
    lifecycle: Lifecycle,
    page_ptl_cache_created: bool,
}

impl PageTableLockCache {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            page_ptl_cache_created: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn page_ptl_cache_created(&self) -> bool {
        self.page_ptl_cache_created
    }

    fn setup(&mut self, slub_allocator: &SlubAllocator) -> EventResult {
        if self.lifecycle.state() != State::Base || slub_allocator.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.page_ptl_cache_created = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PageTableLockCacheReady,
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VmapAreaFlags {
    VmIoremap,
}

impl VmapAreaFlags {
    pub const fn is_vm_ioremap(self) -> bool {
        matches!(self, Self::VmIoremap)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PageProtection {
    IoMemory,
}

impl PageProtection {
    pub const fn is_io_memory(self) -> bool {
        matches!(self, Self::IoMemory)
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn kind(self) -> PageProtectionKind {
        match self {
            Self::IoMemory => PageProtectionKind::IoMemory,
        }
    }
}

#[cfg(checkpoint_handler_vmalloc_mapping)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PageProtectionKind {
    IoMemory,
}

#[cfg(checkpoint_handler_vmalloc_mapping)]
impl PageProtectionKind {
    pub const fn is_io_memory(self) -> bool {
        matches!(self, Self::IoMemory)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct VmapArea {
    index: usize,
    virt_base: usize,
    size: usize,
    flags: VmapAreaFlags,
    busy: bool,
    vm_struct_metadata_ready: bool,
    vmap_area_metadata_ready: bool,
    released: bool,
}

impl VmapArea {
    pub const fn empty() -> Self {
        Self {
            index: usize::MAX,
            virt_base: 0,
            size: 0,
            flags: VmapAreaFlags::VmIoremap,
            busy: false,
            vm_struct_metadata_ready: false,
            vmap_area_metadata_ready: false,
            released: false,
        }
    }

    const fn new(index: usize, virt_base: usize, size: usize, flags: VmapAreaFlags) -> Self {
        Self {
            index,
            virt_base,
            size,
            flags,
            busy: true,
            vm_struct_metadata_ready: true,
            vmap_area_metadata_ready: true,
            released: false,
        }
    }

    #[allow(dead_code)]
    const fn release(mut self) -> Self {
        self.busy = false;
        self.vm_struct_metadata_ready = false;
        self.vmap_area_metadata_ready = false;
        self.released = true;
        self
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn virt_base(self) -> usize {
        self.virt_base
    }

    pub const fn size(self) -> usize {
        self.size
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn flags(self) -> VmapAreaFlags {
        self.flags
    }

    pub const fn busy(self) -> bool {
        self.busy
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn released(self) -> bool {
        self.released
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn vm_struct_metadata_ready(self) -> bool {
        self.vm_struct_metadata_ready
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn vmap_area_metadata_ready(self) -> bool {
        self.vmap_area_metadata_ready
    }

    pub const fn end(self) -> usize {
        self.virt_base.saturating_add(self.size)
    }

    pub const fn is_vm_ioremap(self) -> bool {
        self.flags.is_vm_ioremap()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct VmapMapping {
    index: usize,
    area: VmapArea,
    phys_base: usize,
    size: usize,
    protection: PageProtection,
    installed: bool,
    record_created: bool,
    removed: bool,
}

impl VmapMapping {
    pub const fn empty() -> Self {
        Self {
            index: usize::MAX,
            area: VmapArea::empty(),
            phys_base: 0,
            size: 0,
            protection: PageProtection::IoMemory,
            installed: false,
            record_created: false,
            removed: false,
        }
    }

    const fn new(
        index: usize,
        area: VmapArea,
        phys_base: usize,
        size: usize,
        protection: PageProtection,
    ) -> Self {
        Self {
            index,
            area,
            phys_base,
            size,
            protection,
            installed: true,
            record_created: true,
            removed: false,
        }
    }

    #[allow(dead_code)]
    const fn remove(mut self) -> Self {
        self.installed = false;
        self.removed = true;
        self
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn index(self) -> usize {
        self.index
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn area(self) -> VmapArea {
        self.area
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn phys_base(self) -> usize {
        self.phys_base
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn size(self) -> usize {
        self.size
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn protection(self) -> PageProtection {
        self.protection
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn installed(self) -> bool {
        self.installed
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn record_created(self) -> bool {
        self.record_created
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn removed(self) -> bool {
        self.removed
    }

    pub const fn uses_io_memory_protection(self) -> bool {
        self.protection.is_io_memory()
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn protection_kind(self) -> PageProtectionKind {
        self.protection.kind()
    }
}

pub struct VmallocAllocator {
    lifecycle: Lifecycle,
    area_cache: VmapAreaCache,
    address_space: VmapAddressSpace,
    node_set: VmapNodeSet,
    block_queues: VmapBlockQueues,
    deferred_set: VfreeDeferredSet,
    initialized: bool,
    vmap_area_api_ready: bool,
    page_range_mapping_api_ready: bool,
    vm_struct_metadata_ready: bool,
    vmap_area_metadata_ready: bool,
    mapping_policy_external: bool,
    physical_resource_policy_external: bool,
    runtime_page_table_mapping_ready: bool,
    page_table_install_range: PageTableInstallRange,
    runtime_mapping_window_start: usize,
    runtime_mapping_window_end: usize,
    runtime_mapping_window_count: usize,
    multi_window_mapping_supported: bool,
    preallocated_mapping_window_bound: bool,
    dynamic_l0_window_allocation_supported: bool,
    duplicate_area_mapping_rejected: bool,
    next_vaddr: usize,
    area_count: usize,
    mapping_count: usize,
    areas: [VmapArea; MAX_VMAP_AREAS],
    mappings: [VmapMapping; MAX_VMAP_MAPPINGS],
    reclaim_hook_ready: bool,
}

impl VmallocAllocator {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            area_cache: VmapAreaCache::new(),
            address_space: VmapAddressSpace::new(),
            node_set: VmapNodeSet::new(),
            block_queues: VmapBlockQueues::new(),
            deferred_set: VfreeDeferredSet::new(),
            initialized: false,
            vmap_area_api_ready: false,
            page_range_mapping_api_ready: false,
            vm_struct_metadata_ready: false,
            vmap_area_metadata_ready: false,
            mapping_policy_external: false,
            physical_resource_policy_external: false,
            runtime_page_table_mapping_ready: false,
            page_table_install_range: PageTableInstallRange::empty(),
            runtime_mapping_window_start: 0,
            runtime_mapping_window_end: 0,
            runtime_mapping_window_count: 0,
            multi_window_mapping_supported: false,
            preallocated_mapping_window_bound: false,
            dynamic_l0_window_allocation_supported: false,
            duplicate_area_mapping_rejected: false,
            next_vaddr: 0,
            area_count: 0,
            mapping_count: 0,
            areas: [VmapArea::empty(); MAX_VMAP_AREAS],
            mappings: [VmapMapping::empty(); MAX_VMAP_MAPPINGS],
            reclaim_hook_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn area_cache(&self) -> &VmapAreaCache {
        &self.area_cache
    }

    pub const fn address_space(&self) -> &VmapAddressSpace {
        &self.address_space
    }

    pub const fn node_set(&self) -> &VmapNodeSet {
        &self.node_set
    }

    pub const fn block_queues(&self) -> &VmapBlockQueues {
        &self.block_queues
    }

    pub const fn deferred_set(&self) -> &VfreeDeferredSet {
        &self.deferred_set
    }

    pub const fn initialized(&self) -> bool {
        self.initialized
    }

    pub const fn vmap_area_api_ready(&self) -> bool {
        self.vmap_area_api_ready
    }

    pub const fn page_range_mapping_api_ready(&self) -> bool {
        self.page_range_mapping_api_ready
    }

    pub const fn vm_struct_metadata_ready(&self) -> bool {
        self.vm_struct_metadata_ready
    }

    pub const fn vmap_area_metadata_ready(&self) -> bool {
        self.vmap_area_metadata_ready
    }

    pub const fn mapping_policy_external(&self) -> bool {
        self.mapping_policy_external
    }

    pub const fn physical_resource_policy_external(&self) -> bool {
        self.physical_resource_policy_external
    }

    pub const fn runtime_page_table_mapping_ready(&self) -> bool {
        self.runtime_page_table_mapping_ready
    }

    pub const fn runtime_mapping_window_ready(&self) -> bool {
        self.runtime_mapping_window_start != 0
            && self.runtime_mapping_window_start < self.runtime_mapping_window_end
            && self.runtime_mapping_window_count != 0
    }

    pub fn refresh_runtime_mapping_window(&mut self, page_table_caches: &PageTableCaches) {
        self.page_table_install_range = page_table_caches.vmalloc_install_range();
        self.runtime_page_table_mapping_ready = self.page_table_install_range.ready();
        self.runtime_mapping_window_start = self.page_table_install_range.window_start();
        self.runtime_mapping_window_end = self.page_table_install_range.window_end();
        self.runtime_mapping_window_count = self.page_table_install_range.l0_table_count();
        self.multi_window_mapping_supported = self.runtime_mapping_window_count > 1;
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn runtime_mapping_window_start(&self) -> usize {
        self.runtime_mapping_window_start
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn runtime_mapping_window_end(&self) -> usize {
        self.runtime_mapping_window_end
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn runtime_mapping_window_count(&self) -> usize {
        self.runtime_mapping_window_count
    }

    pub const fn multi_window_mapping_supported(&self) -> bool {
        self.multi_window_mapping_supported
    }

    pub const fn preallocated_mapping_window_bound(&self) -> bool {
        self.preallocated_mapping_window_bound
    }

    pub const fn dynamic_l0_window_allocation_supported(&self) -> bool {
        self.dynamic_l0_window_allocation_supported
    }

    pub const fn duplicate_area_mapping_rejected(&self) -> bool {
        self.duplicate_area_mapping_rejected
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn area_count(&self) -> usize {
        self.area_count
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn mapping_count(&self) -> usize {
        self.mapping_count
    }

    pub const fn reclaim_hook_ready(&self) -> bool {
        self.reclaim_hook_ready
    }

    pub fn setup(
        &mut self,
        slub_allocator: &SlubAllocator,
        page_table_caches: &PageTableCaches,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_allocator.state() != State::Ready
            || page_table_caches.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.area_cache.setup(slub_allocator)?;
        self.address_space
            .setup(&self.area_cache, page_table_caches)?;
        self.node_set.setup(&self.address_space, per_cpu_storage)?;
        self.block_queues.setup(&self.node_set, per_cpu_storage)?;
        self.deferred_set.setup(per_cpu_storage)?;
        self.initialized = true;
        self.vmap_area_api_ready = true;
        self.page_range_mapping_api_ready = true;
        self.vm_struct_metadata_ready = true;
        self.vmap_area_metadata_ready = true;
        self.mapping_policy_external = true;
        self.physical_resource_policy_external = true;
        self.page_table_install_range = page_table_caches.vmalloc_install_range();
        self.runtime_page_table_mapping_ready = self.page_table_install_range.ready();
        self.runtime_mapping_window_start = self.page_table_install_range.window_start();
        self.runtime_mapping_window_end = self.page_table_install_range.window_end();
        self.runtime_mapping_window_count = self.page_table_install_range.l0_table_count();
        self.multi_window_mapping_supported = self.runtime_mapping_window_count > 1;
        self.preallocated_mapping_window_bound = true;
        self.dynamic_l0_window_allocation_supported =
            page_table_caches.vmalloc_pgtable_dynamic_allocator_ready();
        self.duplicate_area_mapping_rejected = true;
        self.next_vaddr = self.address_space.start();
        self.reclaim_hook_ready = true;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VmallocAllocatorReady,
        )
    }

    pub fn get_vm_area(&mut self, size: usize, flags: VmapAreaFlags) -> Option<VmapArea> {
        if self.lifecycle.state() != State::Ready
            || !self.vmap_area_api_ready
            || !self.address_space.free_space_ready()
            || size == 0
            || self.area_count >= MAX_VMAP_AREAS
        {
            return None;
        }

        let aligned_size = align_up(size, self.page_size())?;
        let virt_base = align_up(self.next_vaddr, self.page_size())?;
        let area_end = virt_base.checked_add(aligned_size)?;
        if area_end > self.address_space.end() {
            return None;
        }

        let area = VmapArea::new(self.area_count, virt_base, aligned_size, flags);
        self.areas[self.area_count] = area;
        self.area_count += 1;
        self.next_vaddr = area_end;
        Some(area)
    }

    pub fn map_page_range(
        &mut self,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        area: VmapArea,
        phys_base: usize,
        size: usize,
        protection: PageProtection,
    ) -> Option<VmapMapping> {
        if self.lifecycle.state() != State::Ready
            || !self.page_range_mapping_api_ready
            || !self.runtime_page_table_mapping_ready
            || self.mapping_count >= MAX_VMAP_MAPPINGS
            || phys_base == 0
            || size == 0
            || !self.area_known(area)
            || size != area.size()
            || self.area_has_installed_mapping(area)
            || !phys_base.is_multiple_of(self.page_size())
            || !area.virt_base().is_multiple_of(self.page_size())
            || !size.is_multiple_of(self.page_size())
        {
            return None;
        }

        if !self.area_within_runtime_mapping_window(area) {
            let target_count = self.required_runtime_window_count(area)?;
            self.page_table_install_range = page_table_caches.ensure_vmalloc_l0_windows(
                target_count,
                page_allocator,
                page_metadata_map,
                config,
            )?;
            self.refresh_runtime_mapping_window(page_table_caches);
            if !self.area_within_runtime_mapping_window(area) {
                return None;
            }
        }

        if !map_page_range_runtime(
            self.page_table_install_range,
            area.virt_base(),
            phys_base,
            size,
            self.page_size(),
        ) {
            return None;
        }
        csr::sfence_vma();

        let mapping = VmapMapping::new(self.mapping_count, area, phys_base, size, protection);
        self.mappings[self.mapping_count] = mapping;
        self.mapping_count += 1;
        Some(mapping)
    }

    #[allow(dead_code)]
    pub fn unmap_page_range(&mut self, mapping: VmapMapping) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.page_range_mapping_api_ready
            || !self.runtime_page_table_mapping_ready
            || !mapping.record_created
            || !mapping.installed
            || mapping.removed
            || !self.area_known(mapping.area)
            || !self.mapping_known(mapping)
        {
            return false;
        }

        if !unmap_page_range_runtime(
            self.page_table_install_range,
            mapping.area.virt_base(),
            mapping.size,
            self.page_size(),
        ) {
            return false;
        }
        csr::sfence_vma();

        self.mappings[mapping.index] = mapping.remove();
        true
    }

    #[allow(dead_code)]
    pub fn free_vm_area(&mut self, area: VmapArea) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.vmap_area_api_ready
            || !area.busy()
            || area.released
            || !self.area_known(area)
            || self.area_has_installed_mapping(area)
        {
            return false;
        }

        self.areas[area.index()] = area.release();
        true
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub fn area(&self, index: usize) -> Option<VmapArea> {
        if index < self.area_count {
            Some(self.areas[index])
        } else {
            None
        }
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub fn mapping(&self, index: usize) -> Option<VmapMapping> {
        if index < self.mapping_count {
            Some(self.mappings[index])
        } else {
            None
        }
    }

    fn area_known(&self, area: VmapArea) -> bool {
        area.busy() && area.index() < self.area_count && self.areas[area.index()] == area
    }

    fn area_within_runtime_mapping_window(&self, area: VmapArea) -> bool {
        self.runtime_mapping_window_ready()
            && area.virt_base() >= self.runtime_mapping_window_start
            && area.end() <= self.runtime_mapping_window_end
    }

    fn required_runtime_window_count(&self, area: VmapArea) -> Option<usize> {
        if area.virt_base() < self.runtime_mapping_window_start {
            return None;
        }
        let window_size = self.page_table_install_range.l0_window_size();
        if window_size == 0 {
            return None;
        }
        let end = area.end();
        if end <= self.runtime_mapping_window_start {
            return None;
        }
        let covered = end.checked_sub(self.runtime_mapping_window_start)?;
        covered
            .checked_add(window_size - 1)
            .map(|value| value / window_size)
    }

    #[allow(dead_code)]
    fn mapping_known(&self, mapping: VmapMapping) -> bool {
        mapping.index < self.mapping_count && self.mappings[mapping.index] == mapping
    }

    #[allow(dead_code)]
    fn area_has_installed_mapping(&self, area: VmapArea) -> bool {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = self.mappings[index];
            if mapping.area == area && mapping.installed && !mapping.removed {
                return true;
            }
            index += 1;
        }
        false
    }

    const fn page_size(&self) -> usize {
        4096
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct VmapAreaCache {
    lifecycle: Lifecycle,
}

impl VmapAreaCache {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    fn setup(&mut self, slub_allocator: &SlubAllocator) -> EventResult {
        if self.lifecycle.state() != State::Base || slub_allocator.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VmapAreaCacheReady,
        )
    }
}

pub struct VmapAddressSpace {
    lifecycle: Lifecycle,
    start: usize,
    end: usize,
    existing_vmlist_busy_imported: bool,
    free_space_ready: bool,
}

impl VmapAddressSpace {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start: 0,
            end: 0,
            existing_vmlist_busy_imported: false,
            free_space_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn end(&self) -> usize {
        self.end
    }

    pub const fn existing_vmlist_busy_imported(&self) -> bool {
        self.existing_vmlist_busy_imported
    }

    pub const fn free_space_ready(&self) -> bool {
        self.free_space_ready
    }

    fn setup(
        &mut self,
        area_cache: &VmapAreaCache,
        page_table_caches: &PageTableCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || area_cache.state() != State::Ready
            || page_table_caches.state() != State::Ready
            || VMALLOC_START >= VMALLOC_END
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.start = VMALLOC_START;
        self.end = VMALLOC_END;
        self.existing_vmlist_busy_imported = true;
        self.free_space_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VmapAddressSpaceReady,
        )
    }
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    Some(value.checked_add(mask)? & !mask)
}

pub struct VmapNodeSet {
    lifecycle: Lifecycle,
    node_count: usize,
    route_ready: bool,
}

impl VmapNodeSet {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            node_count: 0,
            route_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    pub const fn route_ready(&self) -> bool {
        self.route_ready
    }

    fn setup(
        &mut self,
        address_space: &VmapAddressSpace,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || address_space.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.node_count = 1;
        self.route_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VmapNodeSetReady,
        )
    }
}

pub struct VmapBlockQueues {
    lifecycle: Lifecycle,
    fast_path_metadata_ready: bool,
}

impl VmapBlockQueues {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fast_path_metadata_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fast_path_metadata_ready(&self) -> bool {
        self.fast_path_metadata_ready
    }

    fn setup(&mut self, node_set: &VmapNodeSet, per_cpu_storage: &PerCpuStorage) -> EventResult {
        if self.lifecycle.state() != State::Base
            || node_set.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.fast_path_metadata_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VmapBlockQueuesReady,
        )
    }
}

pub struct VfreeDeferredSet {
    lifecycle: Lifecycle,
    work_ready: bool,
}

impl VfreeDeferredSet {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            work_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn work_ready(&self) -> bool {
        self.work_ready
    }

    fn setup(&mut self, per_cpu_storage: &PerCpuStorage) -> EventResult {
        if self.lifecycle.state() != State::Base || per_cpu_storage.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.work_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VfreeDeferredSetReady,
        )
    }
}

pub struct MmStructCache {
    lifecycle: Lifecycle,
    object_size: usize,
    saved_auxv_usercopy_ready: bool,
    vma_caches_deferred: bool,
}

impl MmStructCache {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            object_size: 0,
            saved_auxv_usercopy_ready: false,
            vma_caches_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn object_size(&self) -> usize {
        self.object_size
    }

    pub const fn saved_auxv_usercopy_ready(&self) -> bool {
        self.saved_auxv_usercopy_ready
    }

    pub const fn vma_caches_deferred(&self) -> bool {
        self.vma_caches_deferred
    }

    pub fn setup(&mut self, slub_allocator: &SlubAllocator, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_allocator.state() != State::Ready
            || cpu_group.state() != State::Ready
            || cpu_group.possible_cpu_count() == 0
        {
            return self.failed_setup();
        }

        let Some(mask_bytes) = round_up(cpu_group.possible_cpu_count(), usize::BITS as usize / 8)
        else {
            return self.failed_setup();
        };
        self.object_size = core::mem::size_of::<usize>() * 16 + mask_bytes;
        self.saved_auxv_usercopy_ready = true;
        self.vma_caches_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::MmStructCacheReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

struct ZoneFacts {
    facts: [ZoneRef; MAX_BOOT_ZONES],
    count: usize,
    total_pages: usize,
}

fn build_zone_facts(zones: &Zones, config: &Config) -> Option<ZoneFacts> {
    let mut facts = [ZoneRef::empty(); MAX_BOOT_ZONES];
    let mut count = 0usize;
    let mut total_pages = 0usize;

    for kind in [ZoneKind::Dma32, ZoneKind::Normal, ZoneKind::Movable] {
        let zone = zones.zone(kind)?;
        if zone.is_empty() {
            continue;
        }
        let pages = bytes_to_pages(zone.present_bytes(), config.page_size())?;
        if count >= MAX_BOOT_ZONES {
            return None;
        }
        facts[count] = ZoneRef {
            kind,
            range: zone.range(),
            managed_pages: pages,
            free_pages: pages,
        };
        total_pages = total_pages.checked_add(pages)?;
        count += 1;
    }

    Some(ZoneFacts {
        facts,
        count,
        total_pages,
    })
}

fn zone_phys_bounds(zones: &Zones) -> Option<(usize, usize)> {
    let mut start = usize::MAX;
    let mut end = 0usize;
    let mut found = false;

    for kind in [ZoneKind::Dma32, ZoneKind::Normal, ZoneKind::Movable] {
        let zone = zones.zone(kind)?;
        if zone.is_empty() {
            continue;
        }
        let range = zone.range();
        if range.start() >= range.end() {
            return None;
        }
        start = start.min(range.start());
        end = end.max(range.end());
        found = true;
    }

    found.then_some((start, end))
}

fn zone_present_pages(zones: &Zones, page_size: usize) -> Option<usize> {
    let mut total = 0usize;
    for kind in [ZoneKind::Dma32, ZoneKind::Normal, ZoneKind::Movable] {
        let zone = zones.zone(kind)?;
        total = total.checked_add(bytes_to_pages(zone.present_bytes(), page_size)?)?;
    }
    Some(total)
}

fn bytes_to_pages(bytes: usize, page_size: usize) -> Option<usize> {
    if page_size == 0 || !page_size.is_power_of_two() || !bytes.is_multiple_of(page_size) {
        return None;
    }
    Some(bytes / page_size)
}

fn phys_to_pfn_value(phys: usize, page_size: usize) -> Option<usize> {
    if page_size == 0 || !page_size.is_power_of_two() || !phys.is_multiple_of(page_size) {
        return None;
    }
    Some(phys / page_size)
}

fn read_freelist_next(addr: usize) -> usize {
    unsafe { core::ptr::read(addr as *const usize) }
}

fn write_freelist_next(addr: usize, next: usize) {
    unsafe {
        core::ptr::write(addr as *mut usize, next);
    }
}

fn block_fits_in_range(pfn: usize, order: usize, page_size: usize, range: PhysRange) -> bool {
    if order >= BUDDY_ORDER_COUNT || page_size == 0 || !page_size.is_power_of_two() {
        return false;
    }
    let Some(start) = pfn.checked_mul(page_size) else {
        return false;
    };
    let Some(bytes) = page_size.checked_mul(1usize << order) else {
        return false;
    };
    let Some(end) = start.checked_add(bytes) else {
        return false;
    };
    start >= range.start() && end <= range.end()
}

fn round_up_value(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    value.checked_add(mask).map(|sum| sum & !mask)
}

fn round_down_value(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    Some(value & !(align - 1))
}

fn largest_buddy_order(pfn: usize, remaining_pages: usize) -> usize {
    let mut order = BUDDY_ORDER_COUNT - 1;
    while order != 0 {
        let pages = 1usize << order;
        if remaining_pages >= pages && pfn.is_multiple_of(pages) {
            return order;
        }
        order -= 1;
    }
    0
}

fn next_reserved_overlap(memblock: &MemBlock, start: usize, end: usize) -> Option<PhysRange> {
    let reserved_ranges = memblock.reserved_ranges();
    let mut best: Option<PhysRange> = None;
    let mut index = 0usize;
    while index < reserved_ranges.count() {
        let range = reserved_ranges.get(index)?;
        if range.start() < end && start < range.end() {
            best = match best {
                Some(current) if current.start() <= range.start() => Some(current),
                _ => Some(range),
            };
        }
        index += 1;
    }
    best
}

fn round_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    Some(value.checked_add(align - 1)? & !(align - 1))
}
