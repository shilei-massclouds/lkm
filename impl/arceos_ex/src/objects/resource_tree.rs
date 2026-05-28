use super::{
    kernel_image::KernelImage,
    lds::Lds,
    memblock::MemBlock,
    raw_dtb::PhysRange,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

const MAX_RESOURCES: usize = 64;
const NO_RESOURCE: usize = usize::MAX;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ResourceKind {
    Root,
    SystemRam,
    Reserved,
    KernelImage,
    KernelCode,
    KernelRodata,
    KernelData,
    KernelBss,
}

impl ResourceKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Root => "I/O memory",
            Self::SystemRam => "System RAM",
            Self::Reserved => "Reserved",
            Self::KernelImage => "Kernel image",
            Self::KernelCode => "Kernel code",
            Self::KernelRodata => "Kernel rodata",
            Self::KernelData => "Kernel data",
            Self::KernelBss => "Kernel bss",
        }
    }
}

pub struct ResourceTree {
    lifecycle: Lifecycle,
    records: [ResourceRecord; MAX_RESOURCES],
    count: usize,
}

impl ResourceTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            records: [ResourceRecord::empty(); MAX_RESOURCES],
            count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn resource_count(&self) -> usize {
        self.count
    }

    pub fn root(&self) -> Option<ResourceRef<'_>> {
        self.resource_at(0)
            .filter(|resource| resource.kind() == ResourceKind::Root)
    }

    pub fn resources(&self, kind: ResourceKind) -> ResourceIter<'_> {
        ResourceIter {
            tree: self,
            kind,
            next: 0,
        }
    }

    pub fn find_first(&self, kind: ResourceKind) -> Option<ResourceRef<'_>> {
        self.resources(kind).next()
    }

    pub fn setup(
        &mut self,
        memblock: &MemBlock,
        kernel_image: &KernelImage,
        lds: &Lds,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || kernel_image.state() != State::Online
            || lds.state() != State::Online
            || lds.kernel_end() <= lds.kernel_start()
        {
            return self.failed_setup();
        }

        let root_range = PhysRange::new(0, usize::MAX);
        self.clear();
        if self
            .add_record(ResourceKind::Root, root_range, NO_RESOURCE)
            .is_none()
            || !self.add_system_ram_resources(memblock)
            || !self.add_reserved_resources(memblock)
            || !self.add_kernel_resources(kernel_image, lds)
            || !self.resources_ready()
        {
            self.clear();
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::ResourceTreeReady,
        )
    }

    fn clear(&mut self) {
        self.records = [ResourceRecord::empty(); MAX_RESOURCES];
        self.count = 0;
    }

    fn add_system_ram_resources(&mut self, memblock: &MemBlock) -> bool {
        let mut index = 0;
        while index < memblock.usable_ranges().count() {
            let Some(range) = memblock.usable_ranges().get(index) else {
                return false;
            };
            if self.add_record(ResourceKind::SystemRam, range, 0).is_none() {
                return false;
            }
            index += 1;
        }
        true
    }

    fn add_reserved_resources(&mut self, memblock: &MemBlock) -> bool {
        let mut index = 0;
        while index < memblock.reserved_ranges().count() {
            let Some(range) = memblock.reserved_ranges().get(index) else {
                return false;
            };
            let parent = self
                .containing_resource(ResourceKind::SystemRam, range)
                .unwrap_or(0);
            if self
                .add_record(ResourceKind::Reserved, range, parent)
                .is_none()
            {
                return false;
            }
            index += 1;
        }
        true
    }

    fn add_kernel_resources(&mut self, kernel_image: &KernelImage, lds: &Lds) -> bool {
        let Some(kernel_range) =
            link_range_to_phys(kernel_image, lds.kernel_start(), lds.kernel_end())
        else {
            return false;
        };
        let Some(system_ram) = self.containing_resource(ResourceKind::SystemRam, kernel_range)
        else {
            return false;
        };
        let Some(kernel) = self.add_record(ResourceKind::KernelImage, kernel_range, system_ram)
        else {
            return false;
        };

        let (text_start, text_end) = lds.text_range();
        let (rodata_start, rodata_end) = lds.rodata_range();
        let (data_start, data_end) = lds.data_range();
        let (bss_start, bss_end) = lds.bss_range();

        self.add_kernel_segment(
            kernel_image,
            kernel,
            ResourceKind::KernelCode,
            text_start,
            text_end,
        ) && self.add_kernel_segment(
            kernel_image,
            kernel,
            ResourceKind::KernelRodata,
            rodata_start,
            rodata_end,
        ) && self.add_kernel_segment(
            kernel_image,
            kernel,
            ResourceKind::KernelData,
            data_start,
            data_end,
        ) && self.add_kernel_segment(
            kernel_image,
            kernel,
            ResourceKind::KernelBss,
            bss_start,
            bss_end,
        )
    }

    fn add_kernel_segment(
        &mut self,
        kernel_image: &KernelImage,
        parent: usize,
        kind: ResourceKind,
        start: usize,
        end: usize,
    ) -> bool {
        let Some(range) = link_range_to_phys(kernel_image, start, end) else {
            return false;
        };
        self.add_record(kind, range, parent).is_some()
    }

    fn add_record(&mut self, kind: ResourceKind, range: PhysRange, parent: usize) -> Option<usize> {
        if range.start() >= range.end() || self.count >= MAX_RESOURCES {
            return None;
        }
        if parent != NO_RESOURCE
            && (parent >= self.count || !range_contains(self.records[parent].range, range))
        {
            return None;
        }

        let index = self.count;
        self.records[index] = ResourceRecord {
            kind,
            range,
            parent,
            first_child: NO_RESOURCE,
            last_child: NO_RESOURCE,
            next_sibling: NO_RESOURCE,
        };
        self.count += 1;

        if parent != NO_RESOURCE {
            if self.records[parent].first_child == NO_RESOURCE {
                self.records[parent].first_child = index;
            } else {
                let last_child = self.records[parent].last_child;
                self.records[last_child].next_sibling = index;
            }
            self.records[parent].last_child = index;
        }

        Some(index)
    }

    fn containing_resource(&self, kind: ResourceKind, range: PhysRange) -> Option<usize> {
        let mut index = 0;
        while index < self.count {
            let record = self.records[index];
            if record.kind == kind && range_contains(record.range, range) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn resources_ready(&self) -> bool {
        if self.count == 0
            || self.records[0].kind != ResourceKind::Root
            || self.find_first(ResourceKind::SystemRam).is_none()
            || self.find_first(ResourceKind::KernelImage).is_none()
            || self.find_first(ResourceKind::KernelCode).is_none()
            || self.find_first(ResourceKind::KernelRodata).is_none()
            || self.find_first(ResourceKind::KernelData).is_none()
            || self.find_first(ResourceKind::KernelBss).is_none()
        {
            return false;
        }

        let mut index = 1;
        while index < self.count {
            let record = self.records[index];
            if record.parent >= self.count
                || !range_contains(self.records[record.parent].range, record.range)
            {
                return false;
            }
            index += 1;
        }

        true
    }

    fn resource_at(&self, index: usize) -> Option<ResourceRef<'_>> {
        if index < self.count {
            Some(ResourceRef { tree: self, index })
        } else {
            None
        }
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
struct ResourceRecord {
    kind: ResourceKind,
    range: PhysRange,
    parent: usize,
    first_child: usize,
    last_child: usize,
    next_sibling: usize,
}

impl ResourceRecord {
    const fn empty() -> Self {
        Self {
            kind: ResourceKind::Root,
            range: PhysRange::empty(),
            parent: NO_RESOURCE,
            first_child: NO_RESOURCE,
            last_child: NO_RESOURCE,
            next_sibling: NO_RESOURCE,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ResourceRef<'tree> {
    tree: &'tree ResourceTree,
    index: usize,
}

impl<'tree> ResourceRef<'tree> {
    pub fn kind(self) -> ResourceKind {
        self.tree.records[self.index].kind
    }

    pub fn range(self) -> PhysRange {
        self.tree.records[self.index].range
    }

    pub fn parent(self) -> Option<ResourceRef<'tree>> {
        let parent = self.tree.records[self.index].parent;
        self.tree.resource_at(parent)
    }

    pub fn children(self) -> ResourceChildIter<'tree> {
        ResourceChildIter {
            tree: self.tree,
            next: self.tree.records[self.index].first_child,
        }
    }
}

pub struct ResourceIter<'tree> {
    tree: &'tree ResourceTree,
    kind: ResourceKind,
    next: usize,
}

impl<'tree> Iterator for ResourceIter<'tree> {
    type Item = ResourceRef<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.tree.count {
            let index = self.next;
            self.next += 1;
            if self.tree.records[index].kind == self.kind {
                return Some(ResourceRef {
                    tree: self.tree,
                    index,
                });
            }
        }
        None
    }
}

pub struct ResourceChildIter<'tree> {
    tree: &'tree ResourceTree,
    next: usize,
}

impl<'tree> Iterator for ResourceChildIter<'tree> {
    type Item = ResourceRef<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == NO_RESOURCE || self.next >= self.tree.count {
            return None;
        }

        let index = self.next;
        self.next = self.tree.records[index].next_sibling;
        Some(ResourceRef {
            tree: self.tree,
            index,
        })
    }
}

fn link_range_to_phys(kernel_image: &KernelImage, start: usize, end: usize) -> Option<PhysRange> {
    if start >= end {
        return None;
    }
    let start = kernel_image.runtime_to_phys(start)?;
    let end = kernel_image.runtime_to_phys(end)?;
    if start >= end {
        return None;
    }
    Some(PhysRange::new(start, end))
}

fn range_contains(parent: PhysRange, child: PhysRange) -> bool {
    parent.start() <= child.start() && parent.end() >= child.end()
}
