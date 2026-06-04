use super::{
    config::Config,
    cpu_group::CpuGroup,
    cpu_hotplug::CpuHotplugState,
    dma_cache_policy::DmaCachePolicy,
    early_param::EarlyParam,
    memblock::MemBlock,
    per_cpu_storage::PerCpuStorage,
    raw_dtb::PhysRange,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::{StaticBranch, StaticKey},
    vm::Vm,
    workqueue::Workqueue,
    zones::{ZoneKind, Zones},
};
use crate::trace::Checkpoint;

const MAX_BOOT_ZONES: usize = 3;
const MAX_KMALLOC_CACHES: usize = 8;
const PAGE_ALLOC_CPUHP_STEP: usize = 0x200;
const SLUB_CPUHP_STEP: usize = 0x201;
const VMALLOC_START: usize = 0xffff_ffc8_0000_0000;
const VMALLOC_END: usize = 0xffff_ffd0_0000_0000;

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

pub struct PageAllocator {
    lifecycle: Lifecycle,
    boot_zonelist_set: BootZonelistSet,
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

    pub fn preset(
        &mut self,
        topology: &MemoryTopology,
        cpu_hotplug_state: &CpuHotplugState,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || topology.state() != State::Ready
            || topology.boot_zone_set().state() != State::Ready
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
        config: &Config,
        memory_debug_hardening: &MemoryDebugHardening,
        swiotlb: &Swiotlb,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || memblock.state() != State::Online
            || zones.state() != State::Ready
            || self.boot_zonelist_set.state() != State::Ready
            || memory_debug_hardening.state() != State::Ready
            || swiotlb.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        let Some(zone_facts) = build_zone_facts(zones, config) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        };
        self.zone_facts = zone_facts.facts;
        self.zone_fact_count = zone_facts.count;
        self.totalram_pages = zone_facts.total_pages;
        self.handoff_complete = self.totalram_pages != 0 && self.zone_fact_count != 0;
        if !self.handoff_complete {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
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

        self.sizes = [8, 16, 32, 64, 128, 256, 512, 1024];
        self.count = MAX_KMALLOC_CACHES;
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
}

pub struct PageTableCaches {
    lifecycle: Lifecycle,
    lock_cache: PageTableLockCache,
    vmalloc_pgtable_preallocated: bool,
    modules_path_trimmed: bool,
    memory_hotplug_path_trimmed: bool,
}

impl PageTableCaches {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            lock_cache: PageTableLockCache::new(),
            vmalloc_pgtable_preallocated: false,
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

    pub fn setup(&mut self, slub_allocator: &SlubAllocator, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_allocator.state() != State::Ready
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
        {
            return self.failed_setup();
        }

        self.lock_cache.setup(slub_allocator)?;
        self.vmalloc_pgtable_preallocated = true;
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

pub struct VmallocAllocator {
    lifecycle: Lifecycle,
    area_cache: VmapAreaCache,
    address_space: VmapAddressSpace,
    node_set: VmapNodeSet,
    block_queues: VmapBlockQueues,
    deferred_set: VfreeDeferredSet,
    initialized: bool,
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
        self.reclaim_hook_ready = true;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::VmallocAllocatorReady,
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

fn round_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    Some(value.checked_add(align - 1)? & !(align - 1))
}
