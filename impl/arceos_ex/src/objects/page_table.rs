use super::{config::Config, kernel_image::KernelImage};

pub const SWAPPER_L1_TABLES: usize = 4;
pub const SWAPPER_VMALLOC_L0_TABLES: usize = 4;
pub const VMALLOC_RUNTIME_L0_TABLE_SLOTS: usize = 16;
pub const PAGE_TABLE_ENTRIES: usize = 512;
const PTE_V: usize = 1 << 0;
const PTE_R: usize = 1 << 1;
const PTE_W: usize = 1 << 2;
const PTE_X: usize = 1 << 3;
const PTE_A: usize = 1 << 6;
const PTE_D: usize = 1 << 7;
const PTE_TABLE: usize = PTE_V;
const PTE_LEAF_RW: usize = PTE_V | PTE_R | PTE_W | PTE_A | PTE_D;
const PTE_LEAF_RWX: usize = PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;

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

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PageTableInstallRange {
    root_addr: usize,
    l1_addr: usize,
    l1_phys: usize,
    l0_table_count: usize,
    l0_slots: [PageTablePageSlot; VMALLOC_RUNTIME_L0_TABLE_SLOTS],
    virt_start: usize,
    page_size: usize,
}

impl PageTableInstallRange {
    pub const fn empty() -> Self {
        Self {
            root_addr: 0,
            l1_addr: 0,
            l1_phys: 0,
            l0_table_count: 0,
            l0_slots: [PageTablePageSlot::empty(); VMALLOC_RUNTIME_L0_TABLE_SLOTS],
            virt_start: 0,
            page_size: 0,
        }
    }

    pub const fn new(
        root_addr: usize,
        l1_addr: usize,
        l1_phys: usize,
        l0_slots: [PageTablePageSlot; VMALLOC_RUNTIME_L0_TABLE_SLOTS],
        virt_start: usize,
        page_size: usize,
    ) -> Self {
        Self {
            root_addr,
            l1_addr,
            l1_phys,
            l0_table_count: count_ready_slots(l0_slots),
            l0_slots,
            virt_start,
            page_size,
        }
    }

    pub const fn ready(self) -> bool {
        self.root_addr != 0
            && self.l1_addr != 0
            && self.l1_phys != 0
            && self.l0_table_count != 0
            && self.l0_table_count <= VMALLOC_RUNTIME_L0_TABLE_SLOTS
            && first_n_l0_slots_ready(self.l0_slots, self.l0_table_count)
            && self.virt_start != 0
            && self.page_size != 0
            && self.page_size.is_power_of_two()
    }

    pub const fn window_start(self) -> usize {
        self.virt_start
    }

    pub const fn window_size(self) -> usize {
        PAGE_TABLE_ENTRIES
            .saturating_mul(self.page_size)
            .saturating_mul(self.l0_table_count)
    }

    pub const fn window_end(self) -> usize {
        self.virt_start.saturating_add(self.window_size())
    }

    pub const fn l0_table_count(self) -> usize {
        self.l0_table_count
    }

    pub const fn l0_window_size(self) -> usize {
        PAGE_TABLE_ENTRIES.saturating_mul(self.page_size)
    }

    pub const fn max_l0_table_count(self) -> usize {
        VMALLOC_RUNTIME_L0_TABLE_SLOTS
    }

    pub const fn l0_slot(self, index: usize) -> Option<PageTablePageSlot> {
        if index < self.l0_table_count {
            Some(self.l0_slots[index])
        } else {
            None
        }
    }

    pub fn push_l0_table(&mut self, addr: usize, phys: usize, page_size: usize) -> bool {
        if self.l0_table_count >= VMALLOC_RUNTIME_L0_TABLE_SLOTS
            || page_size != self.page_size
            || !page_table_storage_ready(addr, page_size)
            || !phys.is_multiple_of(page_size)
        {
            return false;
        }
        self.l0_slots[self.l0_table_count] = PageTablePageSlot::new(addr, phys);
        self.l0_table_count += 1;
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
    let l1_table = unsafe { &mut *(tables.l1_addr as *mut PageTablePage) };
    let page_count = covered / page_size;
    let mut virt = virt_start;
    let mut phys = phys_base;
    let mut remaining = page_count;
    while remaining != 0 {
        let Some(window_index) = install_window_index(tables, virt) else {
            return false;
        };
        let vpn2 = sv39_index(virt, 30);
        let vpn1 = sv39_index(virt, 21);
        if vpn2 != sv39_index(tables.window_start(), 30) || vpn1 >= PAGE_TABLE_ENTRIES {
            return false;
        }
        let mut vpn0 = sv39_index(virt, 12);
        let pages_in_window = PAGE_TABLE_ENTRIES - vpn0;
        let chunk_pages = remaining.min(pages_in_window);
        let Some(l0_addr) = l0_table_addr(tables, window_index) else {
            return false;
        };
        let Some(l0_phys) = l0_table_phys(tables, window_index) else {
            return false;
        };
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
        root.set(vpn2, table_pte(tables.l1_phys));
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
    let l1_table = unsafe { &mut *(tables.l1_addr as *mut PageTablePage) };
    let page_count = covered / page_size;
    let mut virt = virt_start;
    let mut remaining = page_count;
    while remaining != 0 {
        let Some(window_index) = install_window_index(tables, virt) else {
            return false;
        };
        let vpn2 = sv39_index(virt, 30);
        let vpn1 = sv39_index(virt, 21);
        if vpn2 != sv39_index(tables.window_start(), 30) || vpn1 >= PAGE_TABLE_ENTRIES {
            return false;
        }
        let mut vpn0 = sv39_index(virt, 12);
        let pages_in_window = PAGE_TABLE_ENTRIES - vpn0;
        let chunk_pages = remaining.min(pages_in_window);
        let Some(l0_addr) = l0_table_addr(tables, window_index) else {
            return false;
        };
        let Some(l0_phys) = l0_table_phys(tables, window_index) else {
            return false;
        };
        let l0_table = unsafe { &mut *(l0_addr as *mut PageTablePage) };
        root.set(vpn2, table_pte(tables.l1_phys));
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

fn sv39_index(virt: usize, shift: usize) -> usize {
    (virt >> shift) & 0x1ff
}

fn install_range_valid(tables: PageTableInstallRange) -> bool {
    let vpn1_start = sv39_index(tables.window_start(), 21);
    tables.ready()
        && sv39_index(tables.window_start(), 12) == 0
        && vpn1_start
            .checked_add(tables.l0_table_count())
            .is_some_and(|vpn1_end| vpn1_end <= PAGE_TABLE_ENTRIES)
}

const fn count_ready_slots(slots: [PageTablePageSlot; VMALLOC_RUNTIME_L0_TABLE_SLOTS]) -> usize {
    let mut index = 0usize;
    let mut count = 0usize;
    while index < VMALLOC_RUNTIME_L0_TABLE_SLOTS {
        if slots[index].ready() {
            count += 1;
        } else {
            return count;
        }
        index += 1;
    }
    count
}

const fn first_n_l0_slots_ready(
    slots: [PageTablePageSlot; VMALLOC_RUNTIME_L0_TABLE_SLOTS],
    count: usize,
) -> bool {
    if count == 0 || count > VMALLOC_RUNTIME_L0_TABLE_SLOTS {
        return false;
    }
    let mut index = 0usize;
    while index < count {
        if !slots[index].ready() {
            return false;
        }
        index += 1;
    }
    true
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
    if index < tables.l0_table_count {
        Some(index)
    } else {
        None
    }
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
