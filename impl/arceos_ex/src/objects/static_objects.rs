use super::{
    config::Config,
    fix_map::FixMap,
    raw_dtb::RawDtb,
    state::{Lifecycle, State},
};

const PAGE_TABLE_ENTRIES: usize = 512;
const PTE_V: usize = 1 << 0;
const PTE_R: usize = 1 << 1;
const PTE_W: usize = 1 << 2;
const PTE_X: usize = 1 << 3;
const PTE_A: usize = 1 << 6;
const PTE_D: usize = 1 << 7;
const PTE_TABLE: usize = PTE_V;
const PTE_LEAF_RWX: usize = PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;

#[repr(align(4096))]
pub struct PageTablePage {
    entries: [usize; PAGE_TABLE_ENTRIES],
}

impl PageTablePage {
    const fn zeroed() -> Self {
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

pub struct StaticObjects {
    lifecycle: Lifecycle,
}

impl StaticObjects {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Online),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn storage_ready(&self, config: &Config) -> bool {
        page_table_storage_ready(trampoline_pg_dir_addr(), config.page_size())
            && page_table_storage_ready(early_pg_dir_addr(), config.page_size())
            && page_table_storage_ready(early_kernel_pg_table_addr(), config.page_size())
            && page_table_storage_ready(early_fixmap_l1_table_addr(), config.page_size())
            && page_table_storage_ready(early_fixmap_l0_table_addr(), config.page_size())
            && page_table_storage_ready(swapper_pg_dir_addr(), config.page_size())
    }

    pub fn build_early_pg_dir(
        &mut self,
        config: &Config,
        kernel_start: usize,
        kernel_end: usize,
        raw_dtb: &RawDtb,
        fix_map: &FixMap,
    ) -> bool {
        if !aligned(kernel_start, config.pmd_size())
            || kernel_end <= kernel_start
            || config.kernel_link_addr() == 0
        {
            return false;
        }

        let root = early_pg_dir_mut();
        let kernel_table = early_kernel_pg_table_mut();
        let fixmap_l1_table = early_fixmap_l1_table_mut();
        let fixmap_l0_table = early_fixmap_l0_table_mut();
        root.clear();
        kernel_table.clear();
        fixmap_l1_table.clear();
        fixmap_l0_table.clear();

        let Some(kernel_size) = kernel_end.checked_sub(kernel_start) else {
            return false;
        };
        if !map_pmd_range(
            root,
            kernel_table,
            config.kernel_link_addr(),
            kernel_start,
            kernel_size,
            config.pmd_size(),
        ) {
            return false;
        }

        let fdt_slot = fix_map.fdt_slot();
        map_page_range(
            root,
            fixmap_l1_table,
            fixmap_l0_table,
            fdt_slot.virt_start(),
            raw_dtb.range().start(),
            raw_dtb.range().size(),
            config.page_size(),
        )
    }
}

fn page_table_storage_ready(addr: usize, page_size: usize) -> bool {
    page_size != 0
        && page_size.is_power_of_two()
        && addr & (page_size - 1) == 0
        && core::mem::size_of::<PageTablePage>() >= page_size
}

fn map_pmd_range(
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
    root.set(vpn2, table_pte(page_table_addr(leaf_table)));
    true
}

fn map_page_range(
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
    l1_table.set(vpn1, table_pte(page_table_addr(l0_table)));
    root.set(vpn2, table_pte(page_table_addr(l1_table)));
    true
}

fn aligned(value: usize, align: usize) -> bool {
    align != 0 && align.is_power_of_two() && value & (align - 1) == 0
}

fn round_up(value: usize, align: usize) -> Option<usize> {
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

fn page_table_addr(table: &PageTablePage) -> usize {
    table as *const PageTablePage as usize
}

fn trampoline_pg_dir_addr() -> usize {
    core::ptr::addr_of!(TRAMPOLINE_PG_DIR) as usize
}

fn early_pg_dir_addr() -> usize {
    core::ptr::addr_of!(EARLY_PG_DIR) as usize
}

fn early_kernel_pg_table_addr() -> usize {
    core::ptr::addr_of!(EARLY_KERNEL_PG_TABLE) as usize
}

fn early_fixmap_l1_table_addr() -> usize {
    core::ptr::addr_of!(EARLY_FIXMAP_L1_TABLE) as usize
}

fn early_fixmap_l0_table_addr() -> usize {
    core::ptr::addr_of!(EARLY_FIXMAP_L0_TABLE) as usize
}

fn swapper_pg_dir_addr() -> usize {
    core::ptr::addr_of!(SWAPPER_PG_DIR) as usize
}

fn early_pg_dir_mut() -> &'static mut PageTablePage {
    // Early boot is still single-threaded here; this function is the object
    // boundary that owns initialization of StaticObjects.early_pg_dir.
    unsafe { &mut *core::ptr::addr_of_mut!(EARLY_PG_DIR) }
}

fn early_kernel_pg_table_mut() -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *core::ptr::addr_of_mut!(EARLY_KERNEL_PG_TABLE) }
}

fn early_fixmap_l1_table_mut() -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *core::ptr::addr_of_mut!(EARLY_FIXMAP_L1_TABLE) }
}

fn early_fixmap_l0_table_mut() -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *core::ptr::addr_of_mut!(EARLY_FIXMAP_L0_TABLE) }
}

static mut TRAMPOLINE_PG_DIR: PageTablePage = PageTablePage::zeroed();
static mut EARLY_PG_DIR: PageTablePage = PageTablePage::zeroed();
static mut EARLY_KERNEL_PG_TABLE: PageTablePage = PageTablePage::zeroed();
static mut EARLY_FIXMAP_L1_TABLE: PageTablePage = PageTablePage::zeroed();
static mut EARLY_FIXMAP_L0_TABLE: PageTablePage = PageTablePage::zeroed();
static mut SWAPPER_PG_DIR: PageTablePage = PageTablePage::zeroed();
