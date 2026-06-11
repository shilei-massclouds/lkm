use super::{
    config::Config,
    device::DeviceRef,
    fix_map::FixMap,
    mm_core::{
        PageAllocator, PageMetadataMap, PageProtection, PageTableCaches, VmallocAllocator,
        VmapArea, VmapAreaFlags, VmapMapping,
    },
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

const MAX_IOREMAP_MAPPINGS: usize = 4;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum MmioOwner {
    PlatformDevice(DeviceRef),
    SystemIrqChip(&'static str),
}

impl MmioOwner {
    const fn platform_device(device: DeviceRef) -> Self {
        Self::PlatformDevice(device)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum MmioMappingKind {
    PlainDevice,
    NonCached,
    WriteCombine,
    NormalMemory,
}

impl MmioMappingKind {
    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn is_plain_device(self) -> bool {
        matches!(self, Self::PlainDevice)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ArchMmioPageAttr {
    RiscvPageIoremap,
}

impl ArchMmioPageAttr {
    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn is_riscv_page_ioremap(self) -> bool {
        matches!(self, Self::RiscvPageIoremap)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct IoMemoryMapping {
    owner: MmioOwner,
    phys_base: usize,
    size: usize,
    page_phys_base: usize,
    page_size: usize,
    mapped_size: usize,
    virt_base: usize,
    membase: usize,
    vmap_area: VmapArea,
    vmap_mapping: VmapMapping,
    kind: MmioMappingKind,
    arch_attr: ArchMmioPageAttr,
    vm_ioremap: bool,
    io_page_protection: bool,
    active: bool,
    unmapped: bool,
}

impl IoMemoryMapping {
    const fn empty() -> Self {
        Self {
            owner: MmioOwner::PlatformDevice(DeviceRef::new(usize::MAX)),
            phys_base: 0,
            size: 0,
            page_phys_base: 0,
            page_size: 0,
            mapped_size: 0,
            virt_base: 0,
            membase: 0,
            vmap_area: VmapArea::empty(),
            vmap_mapping: VmapMapping::empty(),
            kind: MmioMappingKind::PlainDevice,
            arch_attr: ArchMmioPageAttr::RiscvPageIoremap,
            vm_ioremap: false,
            io_page_protection: false,
            active: false,
            unmapped: false,
        }
    }

    pub const fn owner(self) -> MmioOwner {
        self.owner
    }

    #[allow(dead_code)]
    pub const fn device(self) -> DeviceRef {
        match self.owner {
            MmioOwner::PlatformDevice(device) => device,
            MmioOwner::SystemIrqChip(_) => DeviceRef::new(usize::MAX),
        }
    }

    pub const fn owner_is_system_irqchip(self) -> bool {
        matches!(self.owner, MmioOwner::SystemIrqChip(_))
    }

    pub const fn phys_base(self) -> usize {
        self.phys_base
    }

    pub const fn size(self) -> usize {
        self.size
    }

    pub const fn page_phys_base(self) -> usize {
        self.page_phys_base
    }

    pub const fn mapped_size(self) -> usize {
        self.mapped_size
    }

    pub const fn virt_base(self) -> usize {
        self.virt_base
    }

    pub const fn membase(self) -> usize {
        self.membase
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn vmap_area(self) -> VmapArea {
        self.vmap_area
    }

    #[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_vmalloc_mapping))]
    pub const fn vmap_mapping(self) -> VmapMapping {
        self.vmap_mapping
    }

    pub const fn active(self) -> bool {
        self.active
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn unmapped(self) -> bool {
        self.unmapped
    }

    pub const fn uses_vm_ioremap(self) -> bool {
        self.vm_ioremap && self.vmap_area.is_vm_ioremap()
    }

    pub const fn uses_io_page_protection(self) -> bool {
        self.io_page_protection && self.vmap_mapping.uses_io_memory_protection()
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn kind(self) -> MmioMappingKind {
        self.kind
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn arch_attr(self) -> ArchMmioPageAttr {
        self.arch_attr
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn uses_plain_device_attribute(self) -> bool {
        self.kind.is_plain_device() && self.arch_attr.is_riscv_page_ioremap()
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn claims_noncached(self) -> bool {
        matches!(self.kind, MmioMappingKind::NonCached)
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn claims_writecombine(self) -> bool {
        matches!(self.kind, MmioMappingKind::WriteCombine)
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn claims_normal_memory(self) -> bool {
        matches!(self.kind, MmioMappingKind::NormalMemory)
    }

    pub const fn page_aligned(self) -> bool {
        self.page_size != 0
            && self.page_phys_base.is_multiple_of(self.page_size)
            && self.virt_base.is_multiple_of(self.page_size)
            && self.mapped_size.is_multiple_of(self.page_size)
    }

    pub const fn membase_cookie_ready(self) -> bool {
        self.membase != 0 && self.membase >= self.virt_base && self.membase < self.mapping_end()
    }

    pub const fn not_linear_direct_map(self) -> bool {
        self.membase != self.phys_base
    }

    const fn mapping_end(self) -> usize {
        self.virt_base.saturating_add(self.mapped_size)
    }

    #[allow(dead_code)]
    const fn retire(mut self) -> Self {
        self.active = false;
        self.unmapped = true;
        self
    }
}

pub struct Ioremap {
    lifecycle: Lifecycle,
    page_size: usize,
    mapping_count: usize,
    mappings: [IoMemoryMapping; MAX_IOREMAP_MAPPINGS],
    uses_vmalloc_area_management: bool,
    uses_vmalloc_mapping_execution: bool,
    uses_vmap_address_space: bool,
    distinct_from_vmalloc_allocation: bool,
    does_not_use_fixmap: bool,
    physical_resource_policy_ready: bool,
    vm_ioremap_flags_ready: bool,
    io_page_protection_ready: bool,
    mmio_attribute_policy_ready: bool,
    plain_device_attribute_supported: bool,
    noncached_attribute_deferred: bool,
    writecombine_attribute_deferred: bool,
    normal_memory_attribute_deferred: bool,
}

impl Ioremap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            page_size: 0,
            mapping_count: 0,
            mappings: [IoMemoryMapping::empty(); MAX_IOREMAP_MAPPINGS],
            uses_vmalloc_area_management: false,
            uses_vmalloc_mapping_execution: false,
            uses_vmap_address_space: false,
            distinct_from_vmalloc_allocation: false,
            does_not_use_fixmap: false,
            physical_resource_policy_ready: false,
            vm_ioremap_flags_ready: false,
            io_page_protection_ready: false,
            mmio_attribute_policy_ready: false,
            plain_device_attribute_supported: false,
            noncached_attribute_deferred: false,
            writecombine_attribute_deferred: false,
            normal_memory_attribute_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn mapping_count(&self) -> usize {
        self.mapping_count
    }

    pub const fn uses_vmalloc_area_management(&self) -> bool {
        self.uses_vmalloc_area_management
    }

    pub const fn uses_vmalloc_mapping_execution(&self) -> bool {
        self.uses_vmalloc_mapping_execution
    }

    pub const fn uses_vmap_address_space(&self) -> bool {
        self.uses_vmap_address_space
    }

    pub const fn distinct_from_vmalloc_allocation(&self) -> bool {
        self.distinct_from_vmalloc_allocation
    }

    pub const fn does_not_use_fixmap(&self) -> bool {
        self.does_not_use_fixmap
    }

    pub const fn physical_resource_policy_ready(&self) -> bool {
        self.physical_resource_policy_ready
    }

    pub const fn vm_ioremap_flags_ready(&self) -> bool {
        self.vm_ioremap_flags_ready
    }

    pub const fn io_page_protection_ready(&self) -> bool {
        self.io_page_protection_ready
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn mmio_attribute_policy_ready(&self) -> bool {
        self.mmio_attribute_policy_ready
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn plain_device_attribute_supported(&self) -> bool {
        self.plain_device_attribute_supported
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn noncached_attribute_deferred(&self) -> bool {
        self.noncached_attribute_deferred
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn writecombine_attribute_deferred(&self) -> bool {
        self.writecombine_attribute_deferred
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub const fn normal_memory_attribute_deferred(&self) -> bool {
        self.normal_memory_attribute_deferred
    }

    pub fn runtime_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready && self.foundation_ready()
    }

    fn foundation_ready(&self) -> bool {
        self.page_size != 0
            && self.page_size.is_power_of_two()
            && self.uses_vmalloc_area_management
            && self.uses_vmalloc_mapping_execution
            && self.uses_vmap_address_space
            && self.distinct_from_vmalloc_allocation
            && self.does_not_use_fixmap
            && self.physical_resource_policy_ready
            && self.vm_ioremap_flags_ready
            && self.io_page_protection_ready
            && self.mmio_attribute_policy_ready
            && self.plain_device_attribute_supported
            && self.noncached_attribute_deferred
            && self.writecombine_attribute_deferred
            && self.normal_memory_attribute_deferred
    }

    pub fn setup(
        &mut self,
        vm: &Vm,
        vmalloc_allocator: &VmallocAllocator,
        page_table_caches: &PageTableCaches,
        fix_map: &FixMap,
        config: &Config,
    ) -> EventResult {
        let vmap_space = vmalloc_allocator.address_space();
        if self.lifecycle.state() != State::Base
            || !vm.entry_successor_ready()
            || vmalloc_allocator.state() != State::Ready
            || vmap_space.state() != State::Ready
            || page_table_caches.state() != State::Ready
            || fix_map.state() != State::Ready
            || config.state() != State::Online
            || config.page_size() == 0
            || !config.page_size().is_power_of_two()
            || vmap_space.start() == 0
            || vmap_space.start() >= vmap_space.end()
        {
            return self.failed_setup();
        }

        self.page_size = config.page_size();
        self.uses_vmalloc_area_management =
            vmalloc_allocator.initialized() && vmalloc_allocator.vmap_area_api_ready();
        self.uses_vmalloc_mapping_execution = vmalloc_allocator.page_range_mapping_api_ready();
        self.uses_vmap_address_space = vmap_space.free_space_ready();
        self.distinct_from_vmalloc_allocation = true;
        self.does_not_use_fixmap = true;
        self.physical_resource_policy_ready = true;
        self.vm_ioremap_flags_ready = true;
        self.io_page_protection_ready = true;
        self.mmio_attribute_policy_ready = true;
        self.plain_device_attribute_supported = true;
        self.noncached_attribute_deferred = true;
        self.writecombine_attribute_deferred = true;
        self.normal_memory_attribute_deferred = true;

        if !self.foundation_ready() {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IoremapReady,
        )
    }

    pub fn map_device_mmio(
        &mut self,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        device: DeviceRef,
        phys_base: usize,
        size: usize,
    ) -> Option<IoMemoryMapping> {
        self.map_mmio_for_owner(
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            MmioOwner::platform_device(device),
            phys_base,
            size,
            MmioMappingKind::PlainDevice,
        )
    }

    pub fn map_system_irqchip_mmio(
        &mut self,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        name: &'static str,
        phys_base: usize,
        size: usize,
    ) -> Option<IoMemoryMapping> {
        self.map_mmio_for_owner(
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            MmioOwner::SystemIrqChip(name),
            phys_base,
            size,
            MmioMappingKind::PlainDevice,
        )
    }

    #[allow(dead_code)]
    pub fn map_device_mmio_with_kind(
        &mut self,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        device: DeviceRef,
        phys_base: usize,
        size: usize,
        kind: MmioMappingKind,
    ) -> Option<IoMemoryMapping> {
        self.map_mmio_for_owner(
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            MmioOwner::platform_device(device),
            phys_base,
            size,
            kind,
        )
    }

    fn map_mmio_for_owner(
        &mut self,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        owner: MmioOwner,
        phys_base: usize,
        size: usize,
        kind: MmioMappingKind,
    ) -> Option<IoMemoryMapping> {
        let page_size = self.page_size;
        if !self.runtime_ready() || phys_base == 0 || size == 0 || !page_size.is_power_of_two() {
            return None;
        }
        let attr = self.resolve_arch_attr(kind)?;
        if let Some(mapping) = self.mapping_for_owner(owner) {
            return Some(mapping);
        }
        if self.mapping_count >= MAX_IOREMAP_MAPPINGS {
            return None;
        }

        let offset = phys_base & (page_size - 1);
        let page_phys_base = phys_base.checked_sub(offset)?;
        let covered_size = size.checked_add(offset)?;
        let mapped_size = align_up(covered_size, page_size)?;
        let vmap_area = vmalloc_allocator.get_vm_area(mapped_size, VmapAreaFlags::VmIoremap)?;
        let vmap_mapping = vmalloc_allocator.map_page_range(
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            vmap_area,
            page_phys_base,
            mapped_size,
            PageProtection::IoMemory,
        )?;
        let virt_base = vmap_area.virt_base();
        let membase = virt_base.checked_add(offset)?;
        let mapping = IoMemoryMapping {
            owner,
            phys_base,
            size,
            page_phys_base,
            page_size,
            mapped_size,
            virt_base,
            membase,
            vmap_area,
            vmap_mapping,
            kind,
            arch_attr: attr,
            vm_ioremap: true,
            io_page_protection: true,
            active: true,
            unmapped: false,
        };
        if !mapping.page_aligned()
            || !mapping.membase_cookie_ready()
            || !mapping.not_linear_direct_map()
        {
            return None;
        }

        self.mappings[self.mapping_count] = mapping;
        self.mapping_count += 1;
        Some(mapping)
    }

    fn resolve_arch_attr(&self, kind: MmioMappingKind) -> Option<ArchMmioPageAttr> {
        if !self.mmio_attribute_policy_ready {
            return None;
        }
        match kind {
            MmioMappingKind::PlainDevice if self.plain_device_attribute_supported => {
                Some(ArchMmioPageAttr::RiscvPageIoremap)
            }
            MmioMappingKind::NonCached if self.noncached_attribute_deferred => None,
            MmioMappingKind::WriteCombine if self.writecombine_attribute_deferred => None,
            MmioMappingKind::NormalMemory if self.normal_memory_attribute_deferred => None,
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn iounmap(&mut self, vmalloc_allocator: &mut VmallocAllocator, membase: usize) -> bool {
        let page_size = self.page_size;
        if !self.runtime_ready() || membase == 0 || !page_size.is_power_of_two() {
            return false;
        }

        let page_base = membase & !(page_size - 1);
        let Some(index) = self.mapping_index_for_page_base(page_base) else {
            return false;
        };
        let mapping = self.mappings[index];
        if !mapping.active
            || mapping.unmapped
            || !vmalloc_allocator.unmap_page_range(mapping.vmap_mapping)
            || !vmalloc_allocator.free_vm_area(mapping.vmap_area)
        {
            return false;
        }

        self.mappings[index] = mapping.retire();
        true
    }

    pub fn mapping_for_device(&self, device: DeviceRef) -> Option<IoMemoryMapping> {
        self.mapping_for_owner(MmioOwner::platform_device(device))
    }

    pub fn mapping_for_system_irqchip(&self, name: &'static str) -> Option<IoMemoryMapping> {
        self.mapping_for_owner(MmioOwner::SystemIrqChip(name))
    }

    fn mapping_for_owner(&self, owner: MmioOwner) -> Option<IoMemoryMapping> {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = self.mappings[index];
            if mapping.active() && mapping.owner() == owner {
                return Some(mapping);
            }
            index += 1;
        }
        None
    }

    #[cfg(checkpoint_handler_vmalloc_mapping)]
    pub fn mapping(&self, index: usize) -> Option<IoMemoryMapping> {
        if index < self.mapping_count {
            Some(self.mappings[index])
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn mapping_index_for_page_base(&self, page_base: usize) -> Option<usize> {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = self.mappings[index];
            if mapping.active && !mapping.unmapped && mapping.virt_base == page_base {
                return Some(index);
            }
            index += 1;
        }
        None
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

fn align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    Some(value.checked_add(mask)? & !mask)
}
