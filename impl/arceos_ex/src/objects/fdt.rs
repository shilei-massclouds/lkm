use super::{
    fix_map::FixMap,
    raw_dtb::{PhysRange, RawDtb},
};

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

const FDT_OFF_DT_STRUCT: usize = 8;
const FDT_OFF_DT_STRINGS: usize = 12;
const FDT_OFF_MEM_RSVMAP: usize = 16;
const FDT_SIZE_DT_STRINGS: usize = 32;
const FDT_SIZE_DT_STRUCT: usize = 36;
const MAX_DEPTH: usize = 32;
pub const MAX_HARTS: usize = 16;
pub const MAX_PHYS_RANGES: usize = 16;
pub const MAX_CMDLINE: usize = 256;

#[derive(Clone, Copy)]
pub struct HartSet {
    harts: [usize; MAX_HARTS],
    count: usize,
}

impl HartSet {
    pub const fn empty() -> Self {
        Self {
            harts: [0; MAX_HARTS],
            count: 0,
        }
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub fn contains(&self, hartid: usize) -> bool {
        let mut index = 0;
        while index < self.count {
            if self.harts[index] == hartid {
                return true;
            }
            index += 1;
        }
        false
    }

    fn push(&mut self, hartid: usize) -> bool {
        if self.contains(hartid) {
            return true;
        }
        if self.count >= MAX_HARTS {
            return false;
        }
        self.harts[self.count] = hartid;
        self.count += 1;
        true
    }
}

#[derive(Clone, Copy)]
pub struct PhysRangeSet {
    ranges: [PhysRange; MAX_PHYS_RANGES],
    count: usize,
}

impl PhysRangeSet {
    pub const fn empty() -> Self {
        Self {
            ranges: [PhysRange::empty(); MAX_PHYS_RANGES],
            count: 0,
        }
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub fn get(&self, index: usize) -> Option<PhysRange> {
        if index < self.count {
            Some(self.ranges[index])
        } else {
            None
        }
    }

    pub fn contains_range(&self, range: PhysRange) -> bool {
        let mut index = 0;
        while index < self.count {
            let item = self.ranges[index];
            if item.start() <= range.start() && item.end() >= range.end() {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn push(&mut self, range: PhysRange) -> bool {
        if range.start() >= range.end() {
            return false;
        }
        if self.count >= MAX_PHYS_RANGES {
            return false;
        }
        self.ranges[self.count] = range;
        self.count += 1;
        true
    }
}

#[derive(Clone, Copy)]
pub struct BootCommandLine {
    bytes: [u8; MAX_CMDLINE],
    len: usize,
}

impl BootCommandLine {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; MAX_CMDLINE],
            len: 0,
        }
    }

    pub fn contains(&self, needle: &[u8]) -> bool {
        if needle.is_empty() || needle.len() > self.len {
            return false;
        }

        let mut index = 0;
        while index + needle.len() <= self.len {
            let mut matched = true;
            let mut sub = 0;
            while sub < needle.len() {
                if self.bytes[index + sub] != needle[sub] {
                    matched = false;
                    break;
                }
                sub += 1;
            }
            if matched {
                return true;
            }
            index += 1;
        }
        false
    }

    fn copy_from(&mut self, addr: usize, len: usize, limit: usize) -> Option<()> {
        let mut copied = 0;
        while copied < len && copied + 1 < MAX_CMDLINE {
            let byte = read_u8(addr.checked_add(copied)?, limit)?;
            if byte == 0 {
                break;
            }
            self.bytes[copied] = byte;
            copied += 1;
        }
        self.len = copied;
        Some(())
    }
}

#[derive(Clone, Copy)]
pub struct FdtFacts {
    pub harts: HartSet,
    pub memory: PhysRangeSet,
    pub reserved: PhysRangeSet,
    pub cmdline: BootCommandLine,
}

impl FdtFacts {
    const fn empty() -> Self {
        Self {
            harts: HartSet::empty(),
            memory: PhysRangeSet::empty(),
            reserved: PhysRangeSet::empty(),
            cmdline: BootCommandLine::empty(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum NodeKind {
    Root,
    Cpus,
    Cpu,
    Memory,
    Chosen,
    ReservedMemory,
    ReservedMemoryChild,
    Other,
}

#[derive(Clone, Copy)]
struct FdtHeader {
    end: usize,
    struct_start: usize,
    struct_end: usize,
    strings_start: usize,
    strings_end: usize,
    mem_rsvmap_start: usize,
}

struct Parser {
    header: FdtHeader,
    facts: FdtFacts,
    stack: [NodeKind; MAX_DEPTH],
    depth: usize,
    root_addr_cells: usize,
    root_size_cells: usize,
    cpu_addr_cells: usize,
    reserved_addr_cells: usize,
    reserved_size_cells: usize,
}

impl Parser {
    fn new(raw_dtb: &RawDtb, fix_map: &FixMap) -> Option<Self> {
        let range = raw_dtb.range();
        let base = fix_map.fdt_slot().virt_for_phys(range.start())?;
        let end = base.checked_add(range.size())?;

        if read_be_u32(base, end)? != FDT_MAGIC {
            return None;
        }

        let off_dt_struct = read_be_u32(base.checked_add(FDT_OFF_DT_STRUCT)?, end)? as usize;
        let off_dt_strings = read_be_u32(base.checked_add(FDT_OFF_DT_STRINGS)?, end)? as usize;
        let off_mem_rsvmap = read_be_u32(base.checked_add(FDT_OFF_MEM_RSVMAP)?, end)? as usize;
        let size_dt_strings = read_be_u32(base.checked_add(FDT_SIZE_DT_STRINGS)?, end)? as usize;
        let size_dt_struct = read_be_u32(base.checked_add(FDT_SIZE_DT_STRUCT)?, end)? as usize;

        let struct_start = base.checked_add(off_dt_struct)?;
        let struct_end = struct_start.checked_add(size_dt_struct)?;
        let strings_start = base.checked_add(off_dt_strings)?;
        let strings_end = strings_start.checked_add(size_dt_strings)?;
        let mem_rsvmap_start = base.checked_add(off_mem_rsvmap)?;
        if struct_start >= end
            || struct_end > end
            || strings_start >= end
            || strings_end > end
            || mem_rsvmap_start >= end
        {
            return None;
        }

        Some(Self {
            header: FdtHeader {
                end,
                struct_start,
                struct_end,
                strings_start,
                strings_end,
                mem_rsvmap_start,
            },
            facts: FdtFacts::empty(),
            stack: [NodeKind::Other; MAX_DEPTH],
            depth: 0,
            root_addr_cells: 2,
            root_size_cells: 1,
            cpu_addr_cells: 1,
            reserved_addr_cells: 2,
            reserved_size_cells: 1,
        })
    }

    fn parse(mut self) -> Option<FdtFacts> {
        self.parse_mem_rsvmap()?;
        self.parse_struct_block()?;
        Some(self.facts)
    }

    fn parse_mem_rsvmap(&mut self) -> Option<()> {
        let mut cursor = self.header.mem_rsvmap_start;
        loop {
            let address = read_be_u64(cursor, self.header.end)?;
            let size = read_be_u64(cursor.checked_add(8)?, self.header.end)?;
            cursor = cursor.checked_add(16)?;
            if address == 0 && size == 0 {
                return Some(());
            }
            if size != 0 {
                let start = address as usize;
                let end = start.checked_add(size as usize)?;
                if !self.facts.reserved.push(PhysRange::new(start, end)) {
                    return None;
                }
            }
        }
    }

    fn parse_struct_block(&mut self) -> Option<()> {
        let mut cursor = self.header.struct_start;
        while cursor < self.header.struct_end {
            let token = read_be_u32(cursor, self.header.struct_end)?;
            cursor = cursor.checked_add(4)?;
            match token {
                FDT_BEGIN_NODE => {
                    let name_start = cursor;
                    let name_len = cstr_len(name_start, self.header.struct_end)?;
                    let kind = self.classify_node(name_start, name_len)?;
                    if self.depth >= MAX_DEPTH {
                        return None;
                    }
                    self.stack[self.depth] = kind;
                    self.depth += 1;
                    cursor = align4(name_start.checked_add(name_len)?.checked_add(1)?)?;
                }
                FDT_END_NODE => {
                    if self.depth == 0 {
                        return None;
                    }
                    self.depth -= 1;
                }
                FDT_PROP => {
                    let len = read_be_u32(cursor, self.header.struct_end)? as usize;
                    let nameoff =
                        read_be_u32(cursor.checked_add(4)?, self.header.struct_end)? as usize;
                    let value = cursor.checked_add(8)?;
                    let value_end = value.checked_add(len)?;
                    if value_end > self.header.struct_end {
                        return None;
                    }
                    self.parse_property(nameoff, value, len)?;
                    cursor = align4(value_end)?;
                }
                FDT_NOP => {}
                FDT_END => return Some(()),
                _ => return None,
            }
        }
        None
    }

    fn classify_node(&self, name: usize, name_len: usize) -> Option<NodeKind> {
        if self.depth == 0 {
            return Some(NodeKind::Root);
        }

        let parent = self.stack[self.depth - 1];
        if parent == NodeKind::Root && cstr_eq(name, name_len, b"cpus", self.header.struct_end)? {
            return Some(NodeKind::Cpus);
        }
        if parent == NodeKind::Cpus {
            return Some(NodeKind::Cpu);
        }
        if parent == NodeKind::Root
            && cstr_starts_with(name, name_len, b"memory", self.header.struct_end)?
        {
            return Some(NodeKind::Memory);
        }
        if parent == NodeKind::Root && cstr_eq(name, name_len, b"chosen", self.header.struct_end)? {
            return Some(NodeKind::Chosen);
        }
        if parent == NodeKind::Root
            && cstr_eq(name, name_len, b"reserved-memory", self.header.struct_end)?
        {
            return Some(NodeKind::ReservedMemory);
        }
        if parent == NodeKind::ReservedMemory {
            return Some(NodeKind::ReservedMemoryChild);
        }
        Some(NodeKind::Other)
    }

    fn parse_property(&mut self, nameoff: usize, value: usize, len: usize) -> Option<()> {
        let kind = self.current_kind()?;
        if kind == NodeKind::Root {
            if self.prop_name_eq(nameoff, b"#address-cells")? && len >= 4 {
                self.root_addr_cells = read_be_u32(value, self.header.struct_end)? as usize;
                self.reserved_addr_cells = self.root_addr_cells;
            } else if self.prop_name_eq(nameoff, b"#size-cells")? && len >= 4 {
                self.root_size_cells = read_be_u32(value, self.header.struct_end)? as usize;
                self.reserved_size_cells = self.root_size_cells;
            }
        } else if kind == NodeKind::Cpus {
            if self.prop_name_eq(nameoff, b"#address-cells")? && len >= 4 {
                self.cpu_addr_cells = read_be_u32(value, self.header.struct_end)? as usize;
            }
        } else if kind == NodeKind::Cpu {
            if self.prop_name_eq(nameoff, b"reg")? {
                self.parse_cpu_reg(value, len)?;
            }
        } else if kind == NodeKind::Memory {
            if self.prop_name_eq(nameoff, b"reg")? {
                self.parse_ranges(value, len, self.root_addr_cells, self.root_size_cells, true)?;
            }
        } else if kind == NodeKind::Chosen {
            if self.prop_name_eq(nameoff, b"bootargs")? {
                self.facts
                    .cmdline
                    .copy_from(value, len, self.header.struct_end)?;
            }
        } else if kind == NodeKind::ReservedMemory {
            if self.prop_name_eq(nameoff, b"#address-cells")? && len >= 4 {
                self.reserved_addr_cells = read_be_u32(value, self.header.struct_end)? as usize;
            } else if self.prop_name_eq(nameoff, b"#size-cells")? && len >= 4 {
                self.reserved_size_cells = read_be_u32(value, self.header.struct_end)? as usize;
            }
        } else if kind == NodeKind::ReservedMemoryChild && self.prop_name_eq(nameoff, b"reg")? {
            self.parse_ranges(
                value,
                len,
                self.reserved_addr_cells,
                self.reserved_size_cells,
                false,
            )?;
        }

        Some(())
    }

    fn parse_cpu_reg(&mut self, value: usize, len: usize) -> Option<()> {
        let Some((hartid, _)) = read_cells(value, len, self.cpu_addr_cells) else {
            return Some(());
        };
        if !self.facts.harts.push(hartid as usize) {
            return None;
        }
        Some(())
    }

    fn parse_ranges(
        &mut self,
        value: usize,
        len: usize,
        addr_cells: usize,
        size_cells: usize,
        memory: bool,
    ) -> Option<()> {
        if addr_cells == 0 || size_cells == 0 || addr_cells > 2 || size_cells > 2 {
            return None;
        }

        let entry_cells = addr_cells.checked_add(size_cells)?;
        let entry_size = entry_cells.checked_mul(4)?;
        if entry_size == 0 || len % entry_size != 0 {
            return None;
        }

        let mut offset = 0;
        while offset < len {
            let (start, next) = read_cells(value.checked_add(offset)?, len - offset, addr_cells)?;
            let (size, _) = read_cells(
                value.checked_add(offset)?.checked_add(next)?,
                len - offset - next,
                size_cells,
            )?;
            if size != 0 {
                let start = start as usize;
                let end = start.checked_add(size as usize)?;
                let pushed = if memory {
                    self.facts.memory.push(PhysRange::new(start, end))
                } else {
                    self.facts.reserved.push(PhysRange::new(start, end))
                };
                if !pushed {
                    return None;
                }
            }
            offset = offset.checked_add(entry_size)?;
        }
        Some(())
    }

    fn current_kind(&self) -> Option<NodeKind> {
        if self.depth == 0 {
            None
        } else {
            Some(self.stack[self.depth - 1])
        }
    }

    fn prop_name_eq(&self, nameoff: usize, expected: &[u8]) -> Option<bool> {
        let start = self.header.strings_start.checked_add(nameoff)?;
        let len = cstr_len(start, self.header.strings_end)?;
        cstr_eq(start, len, expected, self.header.strings_end)
    }
}

pub fn parse(raw_dtb: &RawDtb, fix_map: &FixMap) -> Option<FdtFacts> {
    Parser::new(raw_dtb, fix_map)?.parse()
}

fn read_cells(addr: usize, len: usize, cells: usize) -> Option<(u64, usize)> {
    if cells == 0 || cells > 2 {
        return None;
    }
    let bytes = cells.checked_mul(4)?;
    if bytes > len {
        return None;
    }

    let value = if cells == 1 {
        read_be_u32(addr, addr.checked_add(len)?)? as u64
    } else {
        read_be_u64(addr, addr.checked_add(len)?)?
    };
    Some((value, bytes))
}

fn read_be_u64(addr: usize, limit: usize) -> Option<u64> {
    let high = read_be_u32(addr, limit)? as u64;
    let low = read_be_u32(addr.checked_add(4)?, limit)? as u64;
    Some((high << 32) | low)
}

fn read_be_u32(addr: usize, limit: usize) -> Option<u32> {
    addr.checked_add(4).filter(|end| *end <= limit)?;
    let bytes = [
        read_u8(addr, limit)?,
        read_u8(addr.checked_add(1)?, limit)?,
        read_u8(addr.checked_add(2)?, limit)?,
        read_u8(addr.checked_add(3)?, limit)?,
    ];
    Some(u32::from_be_bytes(bytes))
}

fn read_u8(addr: usize, limit: usize) -> Option<u8> {
    if addr >= limit {
        return None;
    }
    Some(unsafe { core::ptr::read_volatile(addr as *const u8) })
}

fn cstr_len(start: usize, limit: usize) -> Option<usize> {
    let mut cursor = start;
    while cursor < limit {
        if read_u8(cursor, limit)? == 0 {
            return cursor.checked_sub(start);
        }
        cursor = cursor.checked_add(1)?;
    }
    None
}

fn cstr_eq(start: usize, len: usize, expected: &[u8], limit: usize) -> Option<bool> {
    if len != expected.len() {
        return Some(false);
    }
    let mut index = 0;
    while index < expected.len() {
        if read_u8(start.checked_add(index)?, limit)? != expected[index] {
            return Some(false);
        }
        index += 1;
    }
    Some(true)
}

fn cstr_starts_with(start: usize, len: usize, expected: &[u8], limit: usize) -> Option<bool> {
    if len < expected.len() {
        return Some(false);
    }
    let mut index = 0;
    while index < expected.len() {
        if read_u8(start.checked_add(index)?, limit)? != expected[index] {
            return Some(false);
        }
        index += 1;
    }
    Some(true)
}

fn align4(value: usize) -> Option<usize> {
    value.checked_add(3).map(|value| value & !3)
}
