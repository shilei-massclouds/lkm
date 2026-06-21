use super::{config::Config, kernel_image::KernelImage};

pub const SWAPPER_L1_TABLES: usize = 4;
pub const SWAPPER_VMALLOC_L0_TABLES: usize = 4;
pub const PAGE_TABLE_ENTRIES: usize = 512;
pub const VMALLOC_RUNTIME_L1_TABLE_SLOTS: usize = 32;
pub const VMALLOC_RUNTIME_L0_TABLE_SLOTS: usize =
    VMALLOC_RUNTIME_L1_TABLE_SLOTS * PAGE_TABLE_ENTRIES;
pub const VMALLOC_L0_SLOT_CHUNK_SIZE: usize = 128;
pub const VMALLOC_L0_SLOT_CHUNKS: usize =
    VMALLOC_RUNTIME_L0_TABLE_SLOTS / VMALLOC_L0_SLOT_CHUNK_SIZE;
const PTE_V: usize = 1 << 0;
const PTE_R: usize = 1 << 1;
const PTE_W: usize = 1 << 2;
const PTE_X: usize = 1 << 3;
const PTE_U: usize = 1 << 4;
const PTE_A: usize = 1 << 6;
const PTE_D: usize = 1 << 7;
const PTE_TABLE: usize = PTE_V;
const PTE_LEAF_RW: usize = PTE_V | PTE_R | PTE_W | PTE_A | PTE_D;
const PTE_LEAF_RWX: usize = PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;
const SV39_HIGH_HALF_ROOT_INDEX: usize = 256;

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct PageTablePage {
    entries: [usize; PAGE_TABLE_ENTRIES],
}

impl PageTablePage {
    pub const fn zeroed() -> Self {
        Self {
            entries: [0; PAGE_TABLE_ENTRIES],
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(0);
    }

    pub fn entry(&self, index: usize) -> Option<usize> {
        if index < PAGE_TABLE_ENTRIES {
            Some(self.entries[index])
        } else {
            None
        }
    }

    pub fn set_entry(&mut self, index: usize, value: usize) -> bool {
        if index >= PAGE_TABLE_ENTRIES {
            return false;
        }
        self.entries[index] = value;
        true
    }

    fn set(&mut self, index: usize, value: usize) {
        self.entries[index] = value;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PageTablePageSlot {
    addr: usize,
    phys: usize,
    present: bool,
}

impl PageTablePageSlot {
    pub const fn empty() -> Self {
        Self {
            addr: 0,
            phys: 0,
            present: false,
        }
    }

    pub const fn new(addr: usize, phys: usize) -> Self {
        Self {
            addr,
            phys,
            present: true,
        }
    }

    pub const fn addr(self) -> usize {
        self.addr
    }

    pub const fn phys(self) -> usize {
        self.phys
    }

    pub const fn ready(self) -> bool {
        self.present && self.addr != 0 && self.phys != 0
    }
}

pub struct PageTablePageSlotChunk {
    slots: [PageTablePageSlot; VMALLOC_L0_SLOT_CHUNK_SIZE],
}

impl PageTablePageSlotChunk {
    pub const fn empty() -> Self {
        Self {
            slots: [PageTablePageSlot::empty(); VMALLOC_L0_SLOT_CHUNK_SIZE],
        }
    }

    pub const fn slot(&self, index: usize) -> Option<PageTablePageSlot> {
        if index < VMALLOC_L0_SLOT_CHUNK_SIZE {
            Some(self.slots[index])
        } else {
            None
        }
    }

    pub fn set_slot(&mut self, index: usize, slot: PageTablePageSlot) -> bool {
        if index >= VMALLOC_L0_SLOT_CHUNK_SIZE {
            return false;
        }
        self.slots[index] = slot;
        true
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PageTableInstallRange {
    root_addr: usize,
    l1_table_count: usize,
    l1_slots: [PageTablePageSlot; VMALLOC_RUNTIME_L1_TABLE_SLOTS],
    l0_table_count: usize,
    l0_slot_chunk_addrs: [usize; VMALLOC_L0_SLOT_CHUNKS],
    l0_slot_chunk_count: usize,
    virt_start: usize,
    virt_end: usize,
    page_size: usize,
}

impl PageTableInstallRange {
    pub const fn empty() -> Self {
        Self {
            root_addr: 0,
            l1_table_count: 0,
            l1_slots: [PageTablePageSlot::empty(); VMALLOC_RUNTIME_L1_TABLE_SLOTS],
            l0_table_count: 0,
            l0_slot_chunk_addrs: [0; VMALLOC_L0_SLOT_CHUNKS],
            l0_slot_chunk_count: 0,
            virt_start: 0,
            virt_end: 0,
            page_size: 0,
        }
    }

    pub const fn new(
        root_addr: usize,
        virt_start: usize,
        virt_end: usize,
        page_size: usize,
    ) -> Self {
        Self {
            root_addr,
            l1_table_count: 0,
            l1_slots: [PageTablePageSlot::empty(); VMALLOC_RUNTIME_L1_TABLE_SLOTS],
            l0_table_count: 0,
            l0_slot_chunk_addrs: [0; VMALLOC_L0_SLOT_CHUNKS],
            l0_slot_chunk_count: 0,
            virt_start,
            virt_end,
            page_size,
        }
    }

    pub const fn ready(self) -> bool {
        self.root_addr != 0
            && self.l1_table_count != 0
            && self.l0_table_count != 0
            && self.l1_table_count <= VMALLOC_RUNTIME_L1_TABLE_SLOTS
            && self.l0_table_count <= self.max_l0_table_count()
            && first_l1_slot_ready(self.l1_slots)
            && self.l0_slot_chunk_count != 0
            && self.virt_start != 0
            && self.virt_start < self.virt_end
            && self.page_size != 0
            && self.page_size.is_power_of_two()
    }

    pub const fn window_start(self) -> usize {
        self.virt_start
    }

    pub const fn window_size(self) -> usize {
        self.virt_end.saturating_sub(self.virt_start)
    }

    pub const fn window_end(self) -> usize {
        self.virt_end
    }

    pub const fn l0_table_count(self) -> usize {
        self.l0_table_count
    }

    pub const fn l0_window_size(self) -> usize {
        PAGE_TABLE_ENTRIES.saturating_mul(self.page_size)
    }

    pub const fn l1_window_size(self) -> usize {
        PAGE_TABLE_ENTRIES.saturating_mul(self.l0_window_size())
    }

    pub const fn max_l1_table_count(self) -> usize {
        VMALLOC_RUNTIME_L1_TABLE_SLOTS
    }

    pub const fn max_l0_table_count(self) -> usize {
        VMALLOC_RUNTIME_L0_TABLE_SLOTS
    }

    pub const fn l1_slot(self, index: usize) -> Option<PageTablePageSlot> {
        if index < VMALLOC_RUNTIME_L1_TABLE_SLOTS && self.l1_slots[index].ready() {
            Some(self.l1_slots[index])
        } else {
            None
        }
    }

    pub fn l0_slot(self, index: usize) -> Option<PageTablePageSlot> {
        if index >= self.max_l0_table_count() {
            return None;
        }
        let chunk_index = index / VMALLOC_L0_SLOT_CHUNK_SIZE;
        let slot_index = index % VMALLOC_L0_SLOT_CHUNK_SIZE;
        let chunk_addr = self.l0_slot_chunk_addrs[chunk_index];
        if chunk_addr == 0 {
            return None;
        }
        let chunk = unsafe { &*(chunk_addr as *const PageTablePageSlotChunk) };
        let slot = chunk.slot(slot_index)?;
        slot.ready().then_some(slot)
    }

    pub const fn l0_slot_chunk_addr(self, chunk_index: usize) -> Option<usize> {
        if chunk_index < VMALLOC_L0_SLOT_CHUNKS && self.l0_slot_chunk_addrs[chunk_index] != 0 {
            Some(self.l0_slot_chunk_addrs[chunk_index])
        } else {
            None
        }
    }

    pub fn install_l0_slot_chunk(&mut self, chunk_index: usize, chunk_addr: usize) -> bool {
        if chunk_index >= VMALLOC_L0_SLOT_CHUNKS || chunk_addr == 0 {
            return false;
        }
        if self.l0_slot_chunk_addrs[chunk_index] == 0 {
            self.l0_slot_chunk_count += 1;
        }
        self.l0_slot_chunk_addrs[chunk_index] = chunk_addr;
        true
    }

    pub fn install_l1_table(
        &mut self,
        index: usize,
        addr: usize,
        phys: usize,
        page_size: usize,
    ) -> bool {
        if index >= VMALLOC_RUNTIME_L1_TABLE_SLOTS
            || page_size != self.page_size
            || !page_table_storage_ready(addr, page_size)
            || !phys.is_multiple_of(page_size)
        {
            return false;
        }
        if !self.l1_slots[index].ready() {
            self.l1_table_count += 1;
        }
        self.l1_slots[index] = PageTablePageSlot::new(addr, phys);
        true
    }

    pub fn install_l0_table(
        &mut self,
        index: usize,
        addr: usize,
        phys: usize,
        page_size: usize,
    ) -> bool {
        if index >= self.max_l0_table_count()
            || page_size != self.page_size
            || !page_table_storage_ready(addr, page_size)
            || !phys.is_multiple_of(page_size)
        {
            return false;
        }
        let chunk_index = index / VMALLOC_L0_SLOT_CHUNK_SIZE;
        let slot_index = index % VMALLOC_L0_SLOT_CHUNK_SIZE;
        let Some(chunk_addr) = self.l0_slot_chunk_addr(chunk_index) else {
            return false;
        };
        let chunk = unsafe { &mut *(chunk_addr as *mut PageTablePageSlotChunk) };
        if !chunk.slot(slot_index).is_some_and(PageTablePageSlot::ready) {
            self.l0_table_count += 1;
        }
        if !chunk.set_slot(slot_index, PageTablePageSlot::new(addr, phys)) {
            return false;
        }
        true
    }
}

pub fn page_table_storage_ready(addr: usize, page_size: usize) -> bool {
    page_size != 0
        && page_size.is_power_of_two()
        && addr & (page_size - 1) == 0
        && core::mem::size_of::<PageTablePage>() >= page_size
}

pub fn map_pmd_range(
    _config: &Config,
    kernel_image: &KernelImage,
    root: &mut PageTablePage,
    leaf_table: &mut PageTablePage,
    virt_start: usize,
    phys_start: usize,
    bytes: usize,
    pmd_size: usize,
) -> bool {
    if bytes == 0
        || !aligned(virt_start, pmd_size)
        || !aligned(phys_start, pmd_size)
        || !pmd_size.is_power_of_two()
    {
        return false;
    }

    let Some(covered) = round_up(bytes, pmd_size) else {
        return false;
    };
    let vpn2 = sv39_index(virt_start, 30);
    let mut vpn1 = sv39_index(virt_start, 21);
    let mut phys = phys_start;
    let Some(leaf_table_phys) = page_table_phys_addr(kernel_image, leaf_table) else {
        return false;
    };
    for _ in 0..covered / pmd_size {
        if vpn1 >= PAGE_TABLE_ENTRIES {
            return false;
        }
        leaf_table.set(vpn1, leaf_pte(phys, PTE_LEAF_RWX));
        vpn1 += 1;
        let Some(next_phys) = phys.checked_add(pmd_size) else {
            return false;
        };
        phys = next_phys;
    }
    root.set(vpn2, table_pte(leaf_table_phys));
    true
}

pub fn map_page_range(
    _config: &Config,
    kernel_image: &KernelImage,
    root: &mut PageTablePage,
    l1_table: &mut PageTablePage,
    l0_table: &mut PageTablePage,
    virt_start: usize,
    phys_start: usize,
    bytes: usize,
    page_size: usize,
) -> bool {
    if bytes == 0 || !aligned(virt_start, page_size) || !page_size.is_power_of_two() {
        return false;
    }

    let page_offset = phys_start & (page_size - 1);
    let phys_base = phys_start - page_offset;
    let Some(mapped_bytes) = bytes.checked_add(page_offset) else {
        return false;
    };
    let Some(covered) = round_up(mapped_bytes, page_size) else {
        return false;
    };
    let vpn2 = sv39_index(virt_start, 30);
    let vpn1 = sv39_index(virt_start, 21);
    let mut vpn0 = sv39_index(virt_start, 12);
    let mut phys = phys_base;
    let Some(l1_table_phys) = page_table_phys_addr(kernel_image, l1_table) else {
        return false;
    };
    let Some(l0_table_phys) = page_table_phys_addr(kernel_image, l0_table) else {
        return false;
    };
    for _ in 0..covered / page_size {
        if vpn0 >= PAGE_TABLE_ENTRIES {
            return false;
        }
        l0_table.set(vpn0, leaf_pte(phys, PTE_LEAF_RWX));
        vpn0 += 1;
        let Some(next_phys) = phys.checked_add(page_size) else {
            return false;
        };
        phys = next_phys;
    }
    l1_table.set(vpn1, table_pte(l0_table_phys));
    root.set(vpn2, table_pte(l1_table_phys));
    true
}

pub fn map_page_range_runtime(
    tables: PageTableInstallRange,
    virt_start: usize,
    phys_start: usize,
    bytes: usize,
    page_size: usize,
) -> bool {
    if !install_range_valid(tables)
        || bytes == 0
        || !aligned(virt_start, page_size)
        || page_size != tables.page_size
        || !page_size.is_power_of_two()
    {
        return false;
    }

    let page_offset = phys_start & (page_size - 1);
    let phys_base = phys_start - page_offset;
    let Some(mapped_bytes) = bytes.checked_add(page_offset) else {
        return false;
    };
    let Some(covered) = round_up(mapped_bytes, page_size) else {
        return false;
    };
    if !range_within_install_range(tables, virt_start, covered) {
        return false;
    }
    let root = unsafe { &mut *(tables.root_addr as *mut PageTablePage) };
    let page_count = covered / page_size;
    let mut virt = virt_start;
    let mut phys = phys_base;
    let mut remaining = page_count;
    while remaining != 0 {
        let Some((l1_index, l0_index)) = install_window_indices(tables, virt) else {
            return false;
        };
        let vpn2 = sv39_index(virt, 30);
        let vpn1 = sv39_index(virt, 21);
        if vpn2 != sv39_index(tables.window_start(), 30).saturating_add(l1_index)
            || vpn1 >= PAGE_TABLE_ENTRIES
        {
            return false;
        }
        let mut vpn0 = sv39_index(virt, 12);
        let pages_in_window = PAGE_TABLE_ENTRIES - vpn0;
        let chunk_pages = remaining.min(pages_in_window);
        let Some(l1_slot) = tables.l1_slot(l1_index) else {
            return false;
        };
        let Some(l0_addr) = l0_table_addr(tables, l0_index) else {
            return false;
        };
        let Some(l0_phys) = l0_table_phys(tables, l0_index) else {
            return false;
        };
        let l1_table = unsafe { &mut *(l1_slot.addr() as *mut PageTablePage) };
        let l0_table = unsafe { &mut *(l0_addr as *mut PageTablePage) };
        let mut chunk_remaining = chunk_pages;
        while chunk_remaining != 0 {
            l0_table.set(vpn0, leaf_pte(phys, PTE_LEAF_RW));
            vpn0 += 1;
            let Some(next_phys) = phys.checked_add(page_size) else {
                return false;
            };
            phys = next_phys;
            chunk_remaining -= 1;
        }
        l1_table.set(vpn1, table_pte(l0_phys));
        root.set(vpn2, table_pte(l1_slot.phys()));
        let Some(next_virt) = virt.checked_add(chunk_pages * page_size) else {
            return false;
        };
        virt = next_virt;
        remaining -= chunk_pages;
    }
    true
}

#[allow(dead_code)]
pub fn unmap_page_range_runtime(
    tables: PageTableInstallRange,
    virt_start: usize,
    bytes: usize,
    page_size: usize,
) -> bool {
    if !install_range_valid(tables)
        || bytes == 0
        || !aligned(virt_start, page_size)
        || page_size != tables.page_size
        || !page_size.is_power_of_two()
    {
        return false;
    }

    let Some(covered) = round_up(bytes, page_size) else {
        return false;
    };
    if !range_within_install_range(tables, virt_start, covered) {
        return false;
    }
    let root = unsafe { &mut *(tables.root_addr as *mut PageTablePage) };
    let page_count = covered / page_size;
    let mut virt = virt_start;
    let mut remaining = page_count;
    while remaining != 0 {
        let Some((l1_index, l0_index)) = install_window_indices(tables, virt) else {
            return false;
        };
        let vpn2 = sv39_index(virt, 30);
        let vpn1 = sv39_index(virt, 21);
        if vpn2 != sv39_index(tables.window_start(), 30).saturating_add(l1_index)
            || vpn1 >= PAGE_TABLE_ENTRIES
        {
            return false;
        }
        let mut vpn0 = sv39_index(virt, 12);
        let pages_in_window = PAGE_TABLE_ENTRIES - vpn0;
        let chunk_pages = remaining.min(pages_in_window);
        let Some(l1_slot) = tables.l1_slot(l1_index) else {
            return false;
        };
        let Some(l0_addr) = l0_table_addr(tables, l0_index) else {
            return false;
        };
        let Some(l0_phys) = l0_table_phys(tables, l0_index) else {
            return false;
        };
        let l1_table = unsafe { &mut *(l1_slot.addr() as *mut PageTablePage) };
        let l0_table = unsafe { &mut *(l0_addr as *mut PageTablePage) };
        root.set(vpn2, table_pte(l1_slot.phys()));
        l1_table.set(vpn1, table_pte(l0_phys));
        let mut chunk_remaining = chunk_pages;
        while chunk_remaining != 0 {
            l0_table.set(vpn0, 0);
            vpn0 += 1;
            chunk_remaining -= 1;
        }
        let Some(next_virt) = virt.checked_add(chunk_pages * page_size) else {
            return false;
        };
        virt = next_virt;
        remaining -= chunk_pages;
    }
    true
}

pub fn map_linear_pmd_range(
    config: &Config,
    kernel_image: &KernelImage,
    root: &mut PageTablePage,
    leaf_tables: &mut [PageTablePage; SWAPPER_L1_TABLES],
    virt_start: usize,
    phys_start: usize,
    bytes: usize,
    pmd_size: usize,
) -> bool {
    if bytes == 0
        || !aligned(virt_start, pmd_size)
        || !aligned(phys_start, pmd_size)
        || !pmd_size.is_power_of_two()
    {
        return false;
    }

    let Some(covered) = round_up(bytes, pmd_size) else {
        return false;
    };
    let mut virt = virt_start;
    let mut phys = phys_start;
    for _ in 0..covered / pmd_size {
        let vpn2 = sv39_index(virt, 30);
        let vpn1 = sv39_index(virt, 21);
        let table_index = vpn2.checked_sub(sv39_index(config.linear_map_virt_start(), 30));
        let Some(table_index) = table_index else {
            return false;
        };
        if table_index >= SWAPPER_L1_TABLES || vpn1 >= PAGE_TABLE_ENTRIES {
            return false;
        }
        let table = &mut leaf_tables[table_index];
        let Some(table_phys) = page_table_phys_addr(kernel_image, table) else {
            return false;
        };
        table.set(vpn1, leaf_pte(phys, PTE_LEAF_RWX));
        root.set(vpn2, table_pte(table_phys));
        let Some(next_virt) = virt.checked_add(pmd_size) else {
            return false;
        };
        let Some(next_phys) = phys.checked_add(pmd_size) else {
            return false;
        };
        virt = next_virt;
        phys = next_phys;
    }
    true
}

pub fn aligned(value: usize, align: usize) -> bool {
    align != 0 && align.is_power_of_two() && value & (align - 1) == 0
}

pub fn round_up(value: usize, align: usize) -> Option<usize> {
    let addend = align.checked_sub(1)?;
    value.checked_add(addend).map(|sum| sum & !addend)
}

pub fn sv39_indices(virt: usize) -> (usize, usize, usize) {
    (
        sv39_index(virt, 30),
        sv39_index(virt, 21),
        sv39_index(virt, 12),
    )
}

pub fn table_pte_from_phys(table_phys: usize) -> usize {
    table_pte(table_phys)
}

pub fn user_leaf_pte_from_phys(
    phys: usize,
    readable: bool,
    writable: bool,
    executable: bool,
) -> Option<usize> {
    if phys & 0xfff != 0 || (!readable && writable) || (!readable && !writable && !executable) {
        return None;
    }

    let mut flags = PTE_V | PTE_U | PTE_A | PTE_D;
    if readable {
        flags |= PTE_R;
    }
    if writable {
        flags |= PTE_W;
    }
    if executable {
        flags |= PTE_X;
    }
    Some(leaf_pte(phys, flags))
}

pub fn copy_high_half_root_entries(dst: &mut PageTablePage, src: &PageTablePage) -> usize {
    let mut copied = 0usize;
    let mut index = SV39_HIGH_HALF_ROOT_INDEX;
    while index < PAGE_TABLE_ENTRIES {
        if let Some(entry) = src.entry(index) {
            if entry != 0 && dst.set_entry(index, entry) {
                copied += 1;
            }
        }
        index += 1;
    }
    copied
}

fn sv39_index(virt: usize, shift: usize) -> usize {
    (virt >> shift) & 0x1ff
}

fn install_range_valid(tables: PageTableInstallRange) -> bool {
    let vpn2_start = sv39_index(tables.window_start(), 30);
    tables.ready()
        && sv39_index(tables.window_start(), 12) == 0
        && sv39_index(tables.window_start(), 21) == 0
        && tables
            .l1_window_size()
            .checked_mul(tables.max_l1_table_count())
            .is_some_and(|size| tables.window_size() <= size)
        && vpn2_start
            .checked_add(tables.max_l1_table_count())
            .is_some_and(|vpn2_end| vpn2_end <= PAGE_TABLE_ENTRIES)
}

const fn first_l1_slot_ready(slots: [PageTablePageSlot; VMALLOC_RUNTIME_L1_TABLE_SLOTS]) -> bool {
    slots[0].ready()
}

fn range_within_install_range(
    tables: PageTableInstallRange,
    virt_start: usize,
    bytes: usize,
) -> bool {
    let Some(virt_end) = virt_start.checked_add(bytes) else {
        return false;
    };
    virt_start >= tables.window_start() && virt_end <= tables.window_end()
}

fn install_window_index(tables: PageTableInstallRange, virt: usize) -> Option<usize> {
    let offset = virt.checked_sub(tables.window_start())?;
    let window_size = tables.l0_window_size();
    if window_size == 0 {
        return None;
    }
    let index = offset / window_size;
    if index < tables.max_l0_table_count() {
        Some(index)
    } else {
        None
    }
}

fn install_window_indices(tables: PageTableInstallRange, virt: usize) -> Option<(usize, usize)> {
    let l0_index = install_window_index(tables, virt)?;
    let l1_index = l0_index / PAGE_TABLE_ENTRIES;
    (l1_index < tables.max_l1_table_count()).then_some((l1_index, l0_index))
}

fn l0_table_addr(tables: PageTableInstallRange, index: usize) -> Option<usize> {
    tables.l0_slot(index).map(|slot| slot.addr())
}

fn l0_table_phys(tables: PageTableInstallRange, index: usize) -> Option<usize> {
    tables.l0_slot(index).map(|slot| slot.phys())
}

fn table_pte(table_addr: usize) -> usize {
    ((table_addr >> 12) << 10) | PTE_TABLE
}

fn leaf_pte(phys: usize, flags: usize) -> usize {
    ((phys >> 12) << 10) | flags
}

fn page_table_phys_addr(kernel_image: &KernelImage, table: &PageTablePage) -> Option<usize> {
    kernel_image.runtime_to_phys(table as *const PageTablePage as usize)
}
