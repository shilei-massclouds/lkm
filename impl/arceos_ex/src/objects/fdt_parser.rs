use super::{
    fdt_facts::FdtFacts,
    fdt_reader::{
        align4, cstr_eq, cstr_len, cstr_starts_with, read_be_u32, read_be_u64, read_cells, read_u8,
    },
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
                    .copy_from(value, len, self.header.struct_end, read_u8)?;
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
