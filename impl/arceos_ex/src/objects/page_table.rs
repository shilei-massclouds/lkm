use super::{config::Config, kernel_image::KernelImage};

pub const SWAPPER_L1_TABLES: usize = 4;

const PAGE_TABLE_ENTRIES: usize = 512;
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
pub struct PageTableInstallRange {
    root_addr: usize,
    l1_addr: usize,
    l0_addr: usize,
    l1_phys: usize,
    l0_phys: usize,
}

impl PageTableInstallRange {
    pub const fn empty() -> Self {
        Self {
            root_addr: 0,
            l1_addr: 0,
            l0_addr: 0,
            l1_phys: 0,
            l0_phys: 0,
        }
    }

    pub const fn new(
        root_addr: usize,
        l1_addr: usize,
        l0_addr: usize,
        l1_phys: usize,
        l0_phys: usize,
    ) -> Self {
        Self {
            root_addr,
            l1_addr,
            l0_addr,
            l1_phys,
            l0_phys,
        }
    }

    pub const fn ready(self) -> bool {
        self.root_addr != 0
            && self.l1_addr != 0
            && self.l0_addr != 0
            && self.l1_phys != 0
            && self.l0_phys != 0
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
    if !tables.ready()
        || bytes == 0
        || !aligned(virt_start, page_size)
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
    let root = unsafe { &mut *(tables.root_addr as *mut PageTablePage) };
    let l1_table = unsafe { &mut *(tables.l1_addr as *mut PageTablePage) };
    let l0_table = unsafe { &mut *(tables.l0_addr as *mut PageTablePage) };
    let vpn2 = sv39_index(virt_start, 30);
    let vpn1 = sv39_index(virt_start, 21);
    let mut vpn0 = sv39_index(virt_start, 12);
    let page_count = covered / page_size;
    let Some(vpn0_end) = vpn0.checked_add(page_count) else {
        return false;
    };
    if vpn0_end > PAGE_TABLE_ENTRIES {
        return false;
    }
    let mut phys = phys_base;
    for _ in 0..page_count {
        l0_table.set(vpn0, leaf_pte(phys, PTE_LEAF_RW));
        vpn0 += 1;
        let Some(next_phys) = phys.checked_add(page_size) else {
            return false;
        };
        phys = next_phys;
    }
    l1_table.set(vpn1, table_pte(tables.l0_phys));
    root.set(vpn2, table_pte(tables.l1_phys));
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

fn table_pte(table_addr: usize) -> usize {
    ((table_addr >> 12) << 10) | PTE_TABLE
}

fn leaf_pte(phys: usize, flags: usize) -> usize {
    ((phys >> 12) << 10) | flags
}

fn page_table_phys_addr(kernel_image: &KernelImage, table: &PageTablePage) -> Option<usize> {
    kernel_image.runtime_to_phys(table as *const PageTablePage as usize)
}
