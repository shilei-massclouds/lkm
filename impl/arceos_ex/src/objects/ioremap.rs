use super::{
    config::Config,
    device::DeviceRef,
    fix_map::FixMap,
    mm_core::{PageTableCaches, VmallocAllocator},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

const MAX_IOREMAP_MAPPINGS: usize = 4;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct IoMemoryMapping {
    device: DeviceRef,
    phys_base: usize,
    size: usize,
    page_phys_base: usize,
    page_size: usize,
    mapped_size: usize,
    virt_base: usize,
    membase: usize,
    vm_ioremap: bool,
    io_page_protection: bool,
}

impl IoMemoryMapping {
    const fn empty() -> Self {
        Self {
            device: DeviceRef::new(usize::MAX),
            phys_base: 0,
            size: 0,
            page_phys_base: 0,
            page_size: 0,
            mapped_size: 0,
            virt_base: 0,
            membase: 0,
            vm_ioremap: false,
            io_page_protection: false,
        }
    }

    pub const fn device(self) -> DeviceRef {
        self.device
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

    pub const fn uses_vm_ioremap(self) -> bool {
        self.vm_ioremap
    }

    pub const fn uses_io_page_protection(self) -> bool {
        self.io_page_protection
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
}

pub struct Ioremap {
    lifecycle: Lifecycle,
    vmap_start: usize,
    vmap_end: usize,
    next_vaddr: usize,
    page_size: usize,
    mapping_count: usize,
    mappings: [IoMemoryMapping; MAX_IOREMAP_MAPPINGS],
    uses_vmalloc_area_management: bool,
    uses_vmap_address_space: bool,
    distinct_from_vmalloc_allocation: bool,
    does_not_use_fixmap: bool,
    io_page_protection_ready: bool,
}

impl Ioremap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            vmap_start: 0,
            vmap_end: 0,
            next_vaddr: 0,
            page_size: 0,
            mapping_count: 0,
            mappings: [IoMemoryMapping::empty(); MAX_IOREMAP_MAPPINGS],
            uses_vmalloc_area_management: false,
            uses_vmap_address_space: false,
            distinct_from_vmalloc_allocation: false,
            does_not_use_fixmap: false,
            io_page_protection_ready: false,
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

    pub const fn uses_vmap_address_space(&self) -> bool {
        self.uses_vmap_address_space
    }

    pub const fn distinct_from_vmalloc_allocation(&self) -> bool {
        self.distinct_from_vmalloc_allocation
    }

    pub const fn does_not_use_fixmap(&self) -> bool {
        self.does_not_use_fixmap
    }

    pub const fn io_page_protection_ready(&self) -> bool {
        self.io_page_protection_ready
    }

    pub fn runtime_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready && self.foundation_ready()
    }

    fn foundation_ready(&self) -> bool {
        self.vmap_start != 0
            && self.vmap_start < self.vmap_end
            && self.page_size != 0
            && self.page_size.is_power_of_two()
            && self.uses_vmalloc_area_management
            && self.uses_vmap_address_space
            && self.distinct_from_vmalloc_allocation
            && self.does_not_use_fixmap
            && self.io_page_protection_ready
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

        self.vmap_start = vmap_space.start();
        self.vmap_end = vmap_space.end();
        self.next_vaddr = self.vmap_start;
        self.page_size = config.page_size();
        self.uses_vmalloc_area_management = vmalloc_allocator.initialized();
        self.uses_vmap_address_space = vmap_space.free_space_ready();
        self.distinct_from_vmalloc_allocation = true;
        self.does_not_use_fixmap = true;
        self.io_page_protection_ready = true;

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
        device: DeviceRef,
        phys_base: usize,
        size: usize,
    ) -> Option<IoMemoryMapping> {
        let page_size = self.page_size;
        if !self.runtime_ready() || phys_base == 0 || size == 0 || !page_size.is_power_of_two() {
            return None;
        }
        if let Some(mapping) = self.mapping_for_device(device) {
            return Some(mapping);
        }
        if self.mapping_count >= MAX_IOREMAP_MAPPINGS {
            return None;
        }

        let offset = phys_base & (page_size - 1);
        let page_phys_base = phys_base.checked_sub(offset)?;
        let covered_size = size.checked_add(offset)?;
        let mapped_size = align_up(covered_size, page_size)?;
        let virt_base = align_up(self.next_vaddr, page_size)?;
        let area_end = virt_base.checked_add(mapped_size)?;
        if area_end > self.vmap_end {
            return None;
        }
        let membase = virt_base.checked_add(offset)?;
        let mapping = IoMemoryMapping {
            device,
            phys_base,
            size,
            page_phys_base,
            page_size,
            mapped_size,
            virt_base,
            membase,
            vm_ioremap: true,
            io_page_protection: true,
        };
        if !mapping.page_aligned()
            || !mapping.membase_cookie_ready()
            || !mapping.not_linear_direct_map()
        {
            return None;
        }

        self.mappings[self.mapping_count] = mapping;
        self.mapping_count += 1;
        self.next_vaddr = area_end;
        Some(mapping)
    }

    pub fn mapping_for_device(&self, device: DeviceRef) -> Option<IoMemoryMapping> {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = self.mappings[index];
            if mapping.device() == device {
                return Some(mapping);
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
