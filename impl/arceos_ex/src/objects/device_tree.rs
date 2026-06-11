use super::{
    config::Config,
    fdt_reader::{align4, cstr_len, read_be_u32, read_u8},
    memblock::MemBlock,
    raw_dtb::{PhysRange, RawDtb},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};
use crate::trace::Checkpoint;

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

const FDT_OFF_DT_STRUCT: usize = 8;
const FDT_OFF_DT_STRINGS: usize = 12;
const FDT_SIZE_DT_STRINGS: usize = 32;
const FDT_SIZE_DT_STRUCT: usize = 36;
const MAX_DEPTH: usize = 64;
const NO_INDEX: usize = usize::MAX;
const ROOT_NODE_NAME: [u8; 1] = [b'/'];
const STDOUT_PATH_PROPERTY: &[u8] = b"stdout-path";
const LINUX_STDOUT_PATH_PROPERTY: &[u8] = b"linux,stdout-path";

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DeviceNodeId(usize);

impl DeviceNodeId {
    pub const fn index(self) -> usize {
        self.0
    }

    pub const fn invalid() -> Self {
        Self(usize::MAX)
    }
}

pub struct DeviceTree {
    lifecycle: Lifecycle,
    storage: PhysRange,
    storage_virt: usize,
    storage_size: usize,
    node_count: usize,
    property_count: usize,
    max_depth: usize,
    stdout_path_node: Option<DeviceNodeId>,
    stdout_path_options: RawSlice,
}

impl DeviceTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            storage: PhysRange::empty(),
            storage_virt: 0,
            storage_size: 0,
            node_count: 0,
            property_count: 0,
            max_depth: 0,
            stdout_path_node: None,
            stdout_path_options: RawSlice::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn storage_range(&self) -> PhysRange {
        self.storage
    }

    #[allow(dead_code)]
    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    #[allow(dead_code)]
    pub const fn property_count(&self) -> usize {
        self.property_count
    }

    pub fn setup(
        &mut self,
        raw_dtb: &RawDtb,
        vm: &Vm,
        memblock: &mut MemBlock,
        config: &Config,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || raw_dtb.state() != State::Ready
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || memblock.state() != State::Online
            || config.state() != State::Online
        {
            return self.failed_setup();
        }

        let Some(view) = FdtView::new(raw_dtb, config) else {
            return self.failed_setup();
        };
        let Some(plan) = DeviceTreePlan::scan(&view) else {
            return self.failed_setup();
        };
        let Some(storage) = memblock.alloc_phys(plan.storage_size, plan.storage_align) else {
            return self.failed_setup();
        };
        let Some(storage_virt) = config.phys_to_linear(storage.start()) else {
            return self.failed_setup();
        };
        if storage_virt.checked_add(plan.storage_size).is_none() {
            return self.failed_setup();
        }

        let Some(records) =
            UnflattenStorage::new(storage_virt, plan.node_count, plan.property_count)
        else {
            return self.failed_setup();
        };
        records.zero(plan.storage_size);

        if !populate_device_tree(&view, &records, &plan) {
            return self.failed_setup();
        }

        let Some(stdout_path) = resolve_stdout_path(&records) else {
            return self.failed_setup();
        };

        self.storage = storage;
        self.storage_virt = storage_virt;
        self.storage_size = plan.storage_size;
        self.node_count = plan.node_count;
        self.property_count = plan.property_count;
        self.max_depth = plan.max_depth;
        self.stdout_path_node = Some(DeviceNodeId(stdout_path.node_index));
        self.stdout_path_options = stdout_path.options;

        if !self.ready_facts_hold(&view) {
            return self.failed_setup();
        }
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DeviceTreeReady,
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

    fn records(&self) -> Option<UnflattenStorage> {
        UnflattenStorage::new(self.storage_virt, self.node_count, self.property_count)
    }

    fn ready_facts_hold(&self, view: &FdtView) -> bool {
        if self.node_count == 0
            || self.storage.start() >= self.storage.end()
            || self.storage.size() != self.storage_size
            || self.max_depth == 0
        {
            return false;
        }

        let Some(records) = self.records() else {
            return false;
        };
        let Some(root) = records.node(0) else {
            return false;
        };
        if root.parent != NO_INDEX
            || root.name.len == 0
            || self.find_path_in_records(b"/").is_none()
        {
            return false;
        }

        self.tree_links_valid(&records, view)
            && self.properties_queryable(&records, view)
            && self.stdout_path_ready()
    }

    fn tree_links_valid(&self, records: &UnflattenStorage, _view: &FdtView) -> bool {
        let mut index = 0;
        while index < self.node_count {
            let Some(node) = records.node(index) else {
                return false;
            };
            if node.name.len == 0 || (index == 0 && node.parent != NO_INDEX) {
                return false;
            }
            if index != 0 && node.parent >= self.node_count {
                return false;
            }
            if index != 0 && !self.reaches_root(records, index) {
                return false;
            }

            let mut child_count = 0;
            let mut child = node.first_child;
            while child != NO_INDEX {
                if child >= self.node_count || child_count >= self.node_count {
                    return false;
                }
                let Some(child_node) = records.node(child) else {
                    return false;
                };
                if child_node.parent != index {
                    return false;
                }
                child = child_node.next_sibling;
                child_count += 1;
            }

            index += 1;
        }
        true
    }

    fn reaches_root(&self, records: &UnflattenStorage, node_index: usize) -> bool {
        let mut current = node_index;
        let mut depth = 0;
        while depth < self.node_count {
            if current == 0 {
                return true;
            }
            let Some(node) = records.node(current) else {
                return false;
            };
            if node.parent == NO_INDEX || node.parent >= self.node_count {
                return false;
            }
            current = node.parent;
            depth += 1;
        }
        false
    }

    fn properties_queryable(&self, records: &UnflattenStorage, view: &FdtView) -> bool {
        let mut total = 0;
        let mut node_index = 0;
        while node_index < self.node_count {
            let Some(node) = records.node(node_index) else {
                return false;
            };
            let mut property_count = 0;
            let mut property = node.first_property;
            while property != NO_INDEX {
                if property >= self.property_count || property_count >= self.property_count {
                    return false;
                }
                let Some(record) = records.property(property) else {
                    return false;
                };
                if record.name.len == 0 {
                    return false;
                }
                if record.value_len != 0
                    && record
                        .value_addr
                        .checked_add(record.value_len)
                        .filter(|end| *end <= view.struct_end)
                        .is_none()
                {
                    return false;
                }
                property = record.next;
                property_count += 1;
                total += 1;
            }
            if property_count != node.property_count {
                return false;
            }
            node_index += 1;
        }
        total == self.property_count
    }

    #[allow(dead_code)]
    pub fn find_path(&self, path: &[u8]) -> Option<usize> {
        if self.lifecycle.state() != State::Ready {
            return None;
        }
        self.find_path_in_records(path)
    }

    pub fn root(&self) -> Option<DeviceNodeRef<'_>> {
        if self.lifecycle.state() != State::Ready || self.record_node(0).is_none() {
            return None;
        }
        Some(DeviceNodeRef {
            tree: self,
            index: 0,
        })
    }

    pub fn find_node(&self, path: &[u8]) -> Option<DeviceNodeRef<'_>> {
        let index = self.find_path(path)?;
        self.record_node(index)?;
        Some(DeviceNodeRef { tree: self, index })
    }

    pub fn node(&self, id: DeviceNodeId) -> Option<DeviceNodeRef<'_>> {
        if self.lifecycle.state() != State::Ready {
            return None;
        }
        self.record_node(id.index())?;
        Some(DeviceNodeRef {
            tree: self,
            index: id.index(),
        })
    }

    pub fn stdout_path_node(&self) -> Option<DeviceNodeRef<'_>> {
        self.node(self.stdout_path_node?)
    }

    pub const fn stdout_path_node_id(&self) -> Option<DeviceNodeId> {
        self.stdout_path_node
    }

    pub fn stdout_path_options(&self) -> &[u8] {
        raw_bytes_slice(self.stdout_path_options.addr, self.stdout_path_options.len).unwrap_or(&[])
    }

    pub fn stdout_path_selects(&self, node_id: DeviceNodeId) -> bool {
        self.stdout_path_node == Some(node_id)
    }

    fn stdout_path_ready(&self) -> bool {
        let Some(node_id) = self.stdout_path_node else {
            return false;
        };
        self.record_node(node_id.index()).is_some()
            && raw_bytes_slice(self.stdout_path_options.addr, self.stdout_path_options.len)
                .is_some()
    }

    fn record_node(&self, index: usize) -> Option<DeviceNodeRecord> {
        self.records()?.node(index)
    }

    fn record_property(&self, index: usize) -> Option<DevicePropertyRecord> {
        self.records()?.property(index)
    }

    fn find_path_in_records(&self, path: &[u8]) -> Option<usize> {
        if path == b"/" {
            return Some(0);
        }
        if path.is_empty() || path[0] != b'/' {
            return None;
        }

        let records = self.records()?;
        let mut current = 0;
        let mut cursor = 1;
        while cursor < path.len() {
            let segment_start = cursor;
            while cursor < path.len() && path[cursor] != b'/' {
                cursor += 1;
            }
            if cursor == segment_start {
                return None;
            }
            current = records.find_child_by_name(current, &path[segment_start..cursor])?;
            if cursor < path.len() && path[cursor] == b'/' {
                cursor += 1;
            }
        }
        Some(current)
    }
}

#[derive(Clone, Copy)]
pub struct DeviceNodeRef<'dt> {
    tree: &'dt DeviceTree,
    index: usize,
}

impl<'dt> DeviceNodeRef<'dt> {
    pub const fn id(&self) -> DeviceNodeId {
        DeviceNodeId(self.index)
    }

    pub fn name(&self) -> &'dt [u8] {
        self.record()
            .and_then(|record| raw_string_slice(record.name))
            .unwrap_or(&[])
    }

    pub fn parent(&self) -> Option<DeviceNodeRef<'dt>> {
        let parent = self.record()?.parent;
        if parent == NO_INDEX {
            return None;
        }
        self.tree.record_node(parent)?;
        Some(DeviceNodeRef {
            tree: self.tree,
            index: parent,
        })
    }

    pub fn children(&self) -> ChildIter<'dt> {
        let next = self
            .record()
            .map(|record| record.first_child)
            .unwrap_or(NO_INDEX);
        ChildIter {
            tree: self.tree,
            next,
            remaining: self.tree.node_count,
        }
    }

    pub fn properties(&self) -> PropertyIter<'dt> {
        let next = self
            .record()
            .map(|record| record.first_property)
            .unwrap_or(NO_INDEX);
        PropertyIter {
            tree: self.tree,
            next,
            remaining: self.tree.property_count,
        }
    }

    pub fn property(&self, name: &[u8]) -> Option<DevicePropertyRef<'dt>> {
        self.properties().find(|property| property.name() == name)
    }

    pub fn has_compatible(&self, expected: &[u8]) -> bool {
        self.property(b"compatible")
            .is_some_and(|property| compatible_list_contains(property.raw_value(), expected))
    }

    pub fn compatibles(&self) -> CompatibleIter<'dt> {
        CompatibleIter {
            value: self
                .property(b"compatible")
                .map(|property| property.raw_value())
                .unwrap_or(&[]),
        }
    }

    fn record(&self) -> Option<DeviceNodeRecord> {
        self.tree.record_node(self.index)
    }
}

pub struct CompatibleIter<'dt> {
    value: &'dt [u8],
}

impl<'dt> Iterator for CompatibleIter<'dt> {
    type Item = &'dt [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        let item_len = cstr_slice_len(self.value);
        let item = &self.value[..item_len];
        if item_len == self.value.len() {
            self.value = &[];
        } else {
            self.value = &self.value[item_len + 1..];
        }

        if item.is_empty() {
            self.next()
        } else {
            Some(item)
        }
    }
}

fn compatible_list_contains(mut value: &[u8], expected: &[u8]) -> bool {
    while !value.is_empty() {
        let item_len = cstr_slice_len(value);
        if item_len == expected.len() && &value[..item_len] == expected {
            return true;
        }
        if item_len == value.len() {
            return false;
        }
        value = &value[item_len + 1..];
    }
    false
}

fn cstr_slice_len(value: &[u8]) -> usize {
    let mut len = 0usize;
    while len < value.len() && value[len] != 0 {
        len += 1;
    }
    len
}

#[derive(Clone, Copy)]
pub struct DevicePropertyRef<'dt> {
    tree: &'dt DeviceTree,
    index: usize,
}

impl<'dt> DevicePropertyRef<'dt> {
    pub fn name(&self) -> &'dt [u8] {
        self.record()
            .and_then(|record| raw_string_slice(record.name))
            .unwrap_or(&[])
    }

    pub fn raw_value(&self) -> &'dt [u8] {
        self.record()
            .and_then(|record| raw_bytes_slice(record.value_addr, record.value_len))
            .unwrap_or(&[])
    }

    fn record(&self) -> Option<DevicePropertyRecord> {
        self.tree.record_property(self.index)
    }
}

pub struct ChildIter<'dt> {
    tree: &'dt DeviceTree,
    next: usize,
    remaining: usize,
}

impl<'dt> Iterator for ChildIter<'dt> {
    type Item = DeviceNodeRef<'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == NO_INDEX || self.remaining == 0 {
            return None;
        }

        let index = self.next;
        let Some(record) = self.tree.record_node(index) else {
            self.next = NO_INDEX;
            return None;
        };
        self.next = record.next_sibling;
        self.remaining -= 1;
        Some(DeviceNodeRef {
            tree: self.tree,
            index,
        })
    }
}

pub struct PropertyIter<'dt> {
    tree: &'dt DeviceTree,
    next: usize,
    remaining: usize,
}

impl<'dt> Iterator for PropertyIter<'dt> {
    type Item = DevicePropertyRef<'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == NO_INDEX || self.remaining == 0 {
            return None;
        }

        let index = self.next;
        let Some(record) = self.tree.record_property(index) else {
            self.next = NO_INDEX;
            return None;
        };
        self.next = record.next;
        self.remaining -= 1;
        Some(DevicePropertyRef {
            tree: self.tree,
            index,
        })
    }
}

#[derive(Clone, Copy)]
struct FdtView {
    struct_start: usize,
    struct_end: usize,
    strings_start: usize,
    strings_end: usize,
}

impl FdtView {
    fn new(raw_dtb: &RawDtb, config: &Config) -> Option<Self> {
        let range = raw_dtb.range();
        let base = config.phys_to_linear(range.start())?;
        let end = base.checked_add(range.size())?;

        if read_be_u32(base, end)? != FDT_MAGIC {
            return None;
        }

        let off_dt_struct = read_be_u32(base.checked_add(FDT_OFF_DT_STRUCT)?, end)? as usize;
        let off_dt_strings = read_be_u32(base.checked_add(FDT_OFF_DT_STRINGS)?, end)? as usize;
        let size_dt_strings = read_be_u32(base.checked_add(FDT_SIZE_DT_STRINGS)?, end)? as usize;
        let size_dt_struct = read_be_u32(base.checked_add(FDT_SIZE_DT_STRUCT)?, end)? as usize;

        let struct_start = base.checked_add(off_dt_struct)?;
        let struct_end = struct_start.checked_add(size_dt_struct)?;
        let strings_start = base.checked_add(off_dt_strings)?;
        let strings_end = strings_start.checked_add(size_dt_strings)?;
        if struct_start >= end || struct_end > end || strings_start >= end || strings_end > end {
            return None;
        }

        Some(Self {
            struct_start,
            struct_end,
            strings_start,
            strings_end,
        })
    }

    fn property_name(&self, nameoff: usize) -> Option<RawString> {
        let addr = self.strings_start.checked_add(nameoff)?;
        let len = cstr_len(addr, self.strings_end)?;
        if len == 0 {
            return None;
        }
        Some(RawString { addr, len })
    }

    fn string_eq(&self, left: RawString, right: RawString) -> Option<bool> {
        if left.len != right.len {
            return Some(false);
        }
        let mut index = 0;
        while index < left.len {
            if read_u8(left.addr.checked_add(index)?, self.strings_end)?
                != read_u8(right.addr.checked_add(index)?, self.strings_end)?
            {
                return Some(false);
            }
            index += 1;
        }
        Some(true)
    }
}

#[derive(Clone, Copy)]
struct DeviceTreePlan {
    node_count: usize,
    property_count: usize,
    max_depth: usize,
    storage_size: usize,
    storage_align: usize,
}

impl DeviceTreePlan {
    fn scan(view: &FdtView) -> Option<Self> {
        let mut cursor = view.struct_start;
        let mut depth = 0usize;
        let mut max_depth = 0usize;
        let mut node_count = 0usize;
        let mut property_count = 0usize;

        while cursor < view.struct_end {
            let token = read_be_u32(cursor, view.struct_end)?;
            cursor = cursor.checked_add(4)?;
            match token {
                FDT_BEGIN_NODE => {
                    if depth == 0 && node_count != 0 {
                        return None;
                    }
                    let name_start = cursor;
                    let name_len = cstr_len(name_start, view.struct_end)?;
                    if node_count != 0 && name_len == 0 {
                        return None;
                    }
                    if depth >= MAX_DEPTH {
                        return None;
                    }
                    node_count = node_count.checked_add(1)?;
                    depth = depth.checked_add(1)?;
                    max_depth = max_depth.max(depth);
                    cursor = align4(name_start.checked_add(name_len)?.checked_add(1)?)?;
                }
                FDT_END_NODE => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                }
                FDT_PROP => {
                    if depth == 0 {
                        return None;
                    }
                    let len = read_be_u32(cursor, view.struct_end)? as usize;
                    let nameoff = read_be_u32(cursor.checked_add(4)?, view.struct_end)? as usize;
                    let value = cursor.checked_add(8)?;
                    let value_end = value.checked_add(len)?;
                    if value_end > view.struct_end || view.property_name(nameoff).is_none() {
                        return None;
                    }
                    property_count = property_count.checked_add(1)?;
                    cursor = align4(value_end)?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth == 0 && node_count != 0 {
                        let storage_align = core::mem::align_of::<DeviceNodeRecord>()
                            .max(core::mem::align_of::<DevicePropertyRecord>());
                        let node_bytes =
                            node_count.checked_mul(core::mem::size_of::<DeviceNodeRecord>())?;
                        let node_bytes = round_up(node_bytes, storage_align)?;
                        let property_bytes = property_count
                            .checked_mul(core::mem::size_of::<DevicePropertyRecord>())?;
                        let storage_size = node_bytes.checked_add(property_bytes)?;
                        return Some(Self {
                            node_count,
                            property_count,
                            max_depth,
                            storage_size,
                            storage_align,
                        });
                    }
                    return None;
                }
                _ => return None,
            }
        }
        None
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawString {
    addr: usize,
    len: usize,
}

#[derive(Clone, Copy)]
struct RawSlice {
    addr: usize,
    len: usize,
}

impl RawSlice {
    const fn empty() -> Self {
        Self { addr: 1, len: 0 }
    }
}

#[derive(Clone, Copy)]
struct StdoutPath {
    node_index: usize,
    options: RawSlice,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DeviceNodeRecord {
    name: RawString,
    parent: usize,
    first_child: usize,
    last_child: usize,
    next_sibling: usize,
    first_property: usize,
    last_property: usize,
    property_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DevicePropertyRecord {
    name: RawString,
    value_addr: usize,
    value_len: usize,
    next: usize,
}

#[derive(Clone, Copy)]
struct UnflattenStorage {
    nodes: *mut DeviceNodeRecord,
    properties: *mut DevicePropertyRecord,
    node_count: usize,
    property_count: usize,
}

impl UnflattenStorage {
    fn new(base: usize, node_count: usize, property_count: usize) -> Option<Self> {
        let align = core::mem::align_of::<DeviceNodeRecord>()
            .max(core::mem::align_of::<DevicePropertyRecord>());
        if base == 0 || base & (align - 1) != 0 || node_count == 0 {
            return None;
        }
        let node_bytes = node_count.checked_mul(core::mem::size_of::<DeviceNodeRecord>())?;
        let node_bytes = round_up(node_bytes, align)?;
        let property_base = base.checked_add(node_bytes)?;
        Some(Self {
            nodes: base as *mut DeviceNodeRecord,
            properties: property_base as *mut DevicePropertyRecord,
            node_count,
            property_count,
        })
    }

    fn zero(&self, storage_size: usize) {
        unsafe { core::ptr::write_bytes(self.nodes as *mut u8, 0, storage_size) };
    }

    fn node(&self, index: usize) -> Option<DeviceNodeRecord> {
        if index >= self.node_count {
            return None;
        }
        Some(unsafe { core::ptr::read(self.nodes.add(index)) })
    }

    fn write_node(&self, index: usize, node: DeviceNodeRecord) -> Option<()> {
        if index >= self.node_count {
            return None;
        }
        unsafe { core::ptr::write(self.nodes.add(index), node) };
        Some(())
    }

    fn property(&self, index: usize) -> Option<DevicePropertyRecord> {
        if index >= self.property_count {
            return None;
        }
        Some(unsafe { core::ptr::read(self.properties.add(index)) })
    }

    fn write_property(&self, index: usize, property: DevicePropertyRecord) -> Option<()> {
        if index >= self.property_count {
            return None;
        }
        unsafe { core::ptr::write(self.properties.add(index), property) };
        Some(())
    }

    fn init_node(&self, index: usize, name: RawString, parent: usize) -> Option<()> {
        self.write_node(
            index,
            DeviceNodeRecord {
                name,
                parent,
                first_child: NO_INDEX,
                last_child: NO_INDEX,
                next_sibling: NO_INDEX,
                first_property: NO_INDEX,
                last_property: NO_INDEX,
                property_count: 0,
            },
        )
    }

    fn attach_child(&self, parent_index: usize, child_index: usize) -> Option<()> {
        let mut parent = self.node(parent_index)?;
        if parent.last_child == NO_INDEX {
            parent.first_child = child_index;
        } else {
            let mut last = self.node(parent.last_child)?;
            last.next_sibling = child_index;
            self.write_node(parent.last_child, last)?;
        }
        parent.last_child = child_index;
        self.write_node(parent_index, parent)
    }

    fn add_property(
        &self,
        view: &FdtView,
        node_index: usize,
        property_index: usize,
        name: RawString,
        value_addr: usize,
        value_len: usize,
    ) -> Option<()> {
        if self.node_has_property_name(view, node_index, name)? {
            return None;
        }
        self.write_property(
            property_index,
            DevicePropertyRecord {
                name,
                value_addr,
                value_len,
                next: NO_INDEX,
            },
        )?;

        let mut node = self.node(node_index)?;
        if node.last_property == NO_INDEX {
            node.first_property = property_index;
        } else {
            let mut last = self.property(node.last_property)?;
            last.next = property_index;
            self.write_property(node.last_property, last)?;
        }
        node.last_property = property_index;
        node.property_count = node.property_count.checked_add(1)?;
        self.write_node(node_index, node)
    }

    fn node_has_property_name(
        &self,
        view: &FdtView,
        node_index: usize,
        name: RawString,
    ) -> Option<bool> {
        let node = self.node(node_index)?;
        let mut property = node.first_property;
        let mut count = 0;
        while property != NO_INDEX {
            if count >= self.property_count {
                return None;
            }
            let record = self.property(property)?;
            if view.string_eq(record.name, name)? {
                return Some(true);
            }
            property = record.next;
            count += 1;
        }
        Some(false)
    }

    fn find_child_by_name(&self, parent_index: usize, name: &[u8]) -> Option<usize> {
        let parent = self.node(parent_index)?;
        let mut child = parent.first_child;
        let mut count = 0;
        while child != NO_INDEX {
            if count >= self.node_count {
                return None;
            }
            let node = self.node(child)?;
            if raw_string_matches_slice(node.name, name)? {
                return Some(child);
            }
            child = node.next_sibling;
            count += 1;
        }
        None
    }

    fn property_value<'a>(&self, node_index: usize, name: &[u8]) -> Option<&'a [u8]> {
        let node = self.node(node_index)?;
        let mut property = node.first_property;
        let mut count = 0usize;
        while property != NO_INDEX {
            if count >= self.property_count {
                return None;
            }
            let record = self.property(property)?;
            if raw_string_matches_slice(record.name, name)? {
                let len = cstr_slice_len(raw_bytes_slice(record.value_addr, record.value_len)?);
                return raw_bytes_slice(record.value_addr, len);
            }
            property = record.next;
            count += 1;
        }
        None
    }

    fn find_path(&self, path: &[u8]) -> Option<usize> {
        if path == b"/" {
            return Some(0);
        }
        if path.is_empty() || path[0] != b'/' {
            return None;
        }

        let mut current = 0usize;
        let mut cursor = 1usize;
        while cursor < path.len() {
            let segment_start = cursor;
            while cursor < path.len() && path[cursor] != b'/' {
                cursor += 1;
            }
            if cursor == segment_start {
                return None;
            }
            current = self.find_child_by_name(current, &path[segment_start..cursor])?;
            if cursor < path.len() && path[cursor] == b'/' {
                cursor += 1;
            }
        }
        Some(current)
    }

    fn resolve_path_or_alias(&self, path: &[u8]) -> Option<usize> {
        if path.first() == Some(&b'/') {
            return self.find_path(path);
        }

        let aliases = self.find_child_by_name(0, b"aliases")?;
        let alias_value = self.property_value(aliases, path)?;
        self.find_path(alias_value)
    }
}

fn populate_device_tree(view: &FdtView, records: &UnflattenStorage, plan: &DeviceTreePlan) -> bool {
    let mut cursor = view.struct_start;
    let mut stack = [NO_INDEX; MAX_DEPTH];
    let mut depth = 0usize;
    let mut next_node = 0usize;
    let mut next_property = 0usize;

    while cursor < view.struct_end {
        let Some(token) = read_be_u32(cursor, view.struct_end) else {
            return false;
        };
        let Some(next_cursor) = cursor.checked_add(4) else {
            return false;
        };
        cursor = next_cursor;

        match token {
            FDT_BEGIN_NODE => {
                if next_node >= plan.node_count || depth >= MAX_DEPTH {
                    return false;
                }
                let name_start = cursor;
                let Some(name_len) = cstr_len(name_start, view.struct_end) else {
                    return false;
                };
                let name = if next_node == 0 && name_len == 0 {
                    RawString {
                        addr: ROOT_NODE_NAME.as_ptr() as usize,
                        len: ROOT_NODE_NAME.len(),
                    }
                } else {
                    RawString {
                        addr: name_start,
                        len: name_len,
                    }
                };
                let parent = if depth == 0 {
                    NO_INDEX
                } else {
                    stack[depth - 1]
                };
                if records.init_node(next_node, name, parent).is_none() {
                    return false;
                }
                if parent != NO_INDEX && records.attach_child(parent, next_node).is_none() {
                    return false;
                }
                stack[depth] = next_node;
                next_node += 1;
                depth += 1;
                let Some(name_end) = name_start
                    .checked_add(name_len)
                    .and_then(|v| v.checked_add(1))
                else {
                    return false;
                };
                let Some(aligned) = align4(name_end) else {
                    return false;
                };
                cursor = aligned;
            }
            FDT_END_NODE => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            FDT_PROP => {
                if depth == 0 || next_property >= plan.property_count {
                    return false;
                }
                let Some(len) = read_be_u32(cursor, view.struct_end).map(|v| v as usize) else {
                    return false;
                };
                let Some(nameoff_addr) = cursor.checked_add(4) else {
                    return false;
                };
                let Some(nameoff) = read_be_u32(nameoff_addr, view.struct_end).map(|v| v as usize)
                else {
                    return false;
                };
                let Some(value) = cursor.checked_add(8) else {
                    return false;
                };
                let Some(value_end) = value.checked_add(len) else {
                    return false;
                };
                if value_end > view.struct_end {
                    return false;
                }
                let Some(name) = view.property_name(nameoff) else {
                    return false;
                };
                let node_index = stack[depth - 1];
                if records
                    .add_property(view, node_index, next_property, name, value, len)
                    .is_none()
                {
                    return false;
                }
                next_property += 1;
                let Some(aligned) = align4(value_end) else {
                    return false;
                };
                cursor = aligned;
            }
            FDT_NOP => {}
            FDT_END => {
                return depth == 0
                    && next_node == plan.node_count
                    && next_property == plan.property_count;
            }
            _ => return false,
        }
    }
    false
}

fn resolve_stdout_path(records: &UnflattenStorage) -> Option<StdoutPath> {
    let chosen = records.find_child_by_name(0, b"chosen")?;
    let value = records
        .property_value(chosen, STDOUT_PATH_PROPERTY)
        .or_else(|| records.property_value(chosen, LINUX_STDOUT_PATH_PROPERTY))?;
    let path_len = stdout_path_node_part_len(value);
    if path_len == 0 {
        return None;
    }
    let node_index = records.resolve_path_or_alias(&value[..path_len])?;
    let options = if path_len < value.len() && value[path_len] == b':' {
        RawSlice {
            addr: value.as_ptr() as usize + path_len + 1,
            len: value.len() - path_len - 1,
        }
    } else {
        RawSlice::empty()
    };
    Some(StdoutPath {
        node_index,
        options,
    })
}

fn stdout_path_node_part_len(value: &[u8]) -> usize {
    let mut len = 0usize;
    while len < value.len() && value[len] != 0 && value[len] != b':' {
        len += 1;
    }
    len
}

fn raw_string_matches_slice(string: RawString, expected: &[u8]) -> Option<bool> {
    if string.len != expected.len() {
        return Some(false);
    }
    let mut index = 0;
    while index < expected.len() {
        let addr = string.addr.checked_add(index)?;
        if unsafe { core::ptr::read_volatile(addr as *const u8) } != expected[index] {
            return Some(false);
        }
        index += 1;
    }
    Some(true)
}

fn raw_string_slice<'a>(string: RawString) -> Option<&'a [u8]> {
    raw_bytes_slice(string.addr, string.len)
}

fn raw_bytes_slice<'a>(addr: usize, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    addr.checked_add(len)?;
    Some(unsafe { core::slice::from_raw_parts(addr as *const u8, len) })
}

fn round_up(value: usize, align: usize) -> Option<usize> {
    Some(value.checked_add(align.checked_sub(1)?)? & !(align - 1))
}
