use super::{
    config::Config,
    entry_prelude::KernelImage,
    entry_successor::MemBlock,
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
const SWAPPER_L1_TABLES: usize = 4;

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
        page_table_storage_ready(
            core::ptr::addr_of!(TRAMPOLINE_PG_DIR) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(TRAMPOLINE_KERNEL_PG_TABLE) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(EARLY_PG_DIR) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(EARLY_KERNEL_PG_TABLE) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(EARLY_FIXMAP_L1_TABLE) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(EARLY_FIXMAP_L0_TABLE) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(SWAPPER_PG_DIR) as usize,
            config.page_size(),
        ) && page_table_storage_ready(
            core::ptr::addr_of!(SWAPPER_KERNEL_PG_TABLE) as usize,
            config.page_size(),
        ) && swapper_linear_pg_tables_ready(config)
    }

    pub fn trampoline_satp(&self, kernel_image: &KernelImage) -> Option<usize> {
        satp_from_root(kernel_image, trampoline_pg_dir_addr(kernel_image))
    }

    pub fn early_satp(&self, kernel_image: &KernelImage) -> Option<usize> {
        satp_from_root(kernel_image, early_pg_dir_addr(kernel_image))
    }

    pub fn swapper_satp(&self, kernel_image: &KernelImage) -> Option<usize> {
        satp_from_root(kernel_image, swapper_pg_dir_addr(kernel_image))
    }

    pub fn build_early_pg_dir(
        &mut self,
        config: &Config,
        kernel_image: &KernelImage,
        kernel_start: usize,
        kernel_end: usize,
        raw_dtb: &RawDtb,
        fix_map: &FixMap,
    ) -> bool {
        let Some((kernel_link_start, kernel_phys_start, kernel_size)) =
            kernel_runtime_range(config, kernel_image, kernel_start, kernel_end)
        else {
            return false;
        };

        let root = early_pg_dir_mut(kernel_image);
        let kernel_table = early_kernel_pg_table_mut(kernel_image);
        let fixmap_l1_table = early_fixmap_l1_table_mut(kernel_image);
        let fixmap_l0_table = early_fixmap_l0_table_mut(kernel_image);
        root.clear();
        kernel_table.clear();
        fixmap_l1_table.clear();
        fixmap_l0_table.clear();

        if !map_pmd_range(
            config,
            kernel_image,
            root,
            kernel_table,
            kernel_link_start,
            kernel_phys_start,
            kernel_size,
            config.pmd_size(),
        ) {
            return false;
        }

        let fdt_slot = fix_map.fdt_slot();
        map_page_range(
            config,
            kernel_image,
            root,
            fixmap_l1_table,
            fixmap_l0_table,
            fdt_slot.virt_start(),
            raw_dtb.range().start(),
            raw_dtb.range().size(),
            config.page_size(),
        )
    }

    pub fn build_trampoline_pg_dir(&mut self, config: &Config, kernel_image: &KernelImage) -> bool {
        let kernel_link_start = kernel_image.virt_start();
        let kernel_phys_start = kernel_image.phys_start();
        if kernel_link_start != config.kernel_link_addr()
            || !aligned(kernel_link_start, config.pmd_size())
            || !aligned(kernel_phys_start, config.pmd_size())
        {
            return false;
        }

        let root = trampoline_pg_dir_mut(kernel_image);
        let kernel_table = trampoline_kernel_pg_table_mut(kernel_image);
        root.clear();
        kernel_table.clear();

        map_pmd_range(
            config,
            kernel_image,
            root,
            kernel_table,
            kernel_link_start,
            kernel_phys_start,
            config.pmd_size(),
            config.pmd_size(),
        )
    }

    pub fn build_swapper_pg_dir(
        &mut self,
        config: &Config,
        kernel_image: &KernelImage,
        kernel_start: usize,
        kernel_end: usize,
        memblock: &MemBlock,
    ) -> bool {
        let Some((kernel_link_start, kernel_phys_start, kernel_size)) =
            kernel_runtime_range(config, kernel_image, kernel_start, kernel_end)
        else {
            return false;
        };

        let root = swapper_pg_dir_mut(kernel_image);
        let kernel_table = swapper_kernel_pg_table_mut(kernel_image);
        let linear_tables = swapper_linear_pg_tables_mut(kernel_image);
        root.clear();
        kernel_table.clear();
        for table in linear_tables.iter_mut() {
            table.clear();
        }

        if !map_pmd_range(
            config,
            kernel_image,
            root,
            kernel_table,
            kernel_link_start,
            kernel_phys_start,
            kernel_size,
            config.pmd_size(),
        ) {
            return false;
        }

        let ranges = memblock.usable_ranges();
        let mut range_index = 0;
        while range_index < ranges.count() {
            let Some(range) = ranges.get(range_index) else {
                return false;
            };
            let Some(linear_start) = config.phys_to_linear(range.start()) else {
                return false;
            };
            if !map_linear_pmd_range(
                config,
                kernel_image,
                root,
                linear_tables,
                linear_start,
                range.start(),
                range.size(),
                config.pmd_size(),
            ) {
                return false;
            }
            range_index += 1;
        }

        true
    }
}

fn page_table_storage_ready(addr: usize, page_size: usize) -> bool {
    page_size != 0
        && page_size.is_power_of_two()
        && addr & (page_size - 1) == 0
        && core::mem::size_of::<PageTablePage>() >= page_size
}

fn swapper_linear_pg_tables_ready(config: &Config) -> bool {
    let mut index = 0;
    while index < SWAPPER_L1_TABLES {
        let addr = core::ptr::addr_of!(SWAPPER_LINEAR_PG_TABLES) as usize
            + index * core::mem::size_of::<PageTablePage>();
        if !page_table_storage_ready(addr, config.page_size()) {
            return false;
        }
        index += 1;
    }
    true
}

fn map_pmd_range(
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

fn map_page_range(
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

fn map_linear_pmd_range(
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

fn page_table_phys_addr(kernel_image: &KernelImage, table: &PageTablePage) -> Option<usize> {
    kernel_image.runtime_to_phys(table as *const PageTablePage as usize)
}

fn satp_from_root(kernel_image: &KernelImage, root_addr: usize) -> Option<usize> {
    kernel_image
        .runtime_to_phys(root_addr)
        .map(|root_phys| crate::arch::riscv64::csr::SATP_MODE_SV39 | (root_phys >> 12))
}

fn trampoline_pg_dir_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(TRAMPOLINE_PG_DIR) as usize,
    )
}

fn trampoline_kernel_pg_table_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(TRAMPOLINE_KERNEL_PG_TABLE) as usize,
    )
}

fn early_pg_dir_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(kernel_image, core::ptr::addr_of!(EARLY_PG_DIR) as usize)
}

fn early_kernel_pg_table_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(EARLY_KERNEL_PG_TABLE) as usize,
    )
}

fn early_fixmap_l1_table_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(EARLY_FIXMAP_L1_TABLE) as usize,
    )
}

fn early_fixmap_l0_table_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(EARLY_FIXMAP_L0_TABLE) as usize,
    )
}

fn swapper_pg_dir_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(kernel_image, core::ptr::addr_of!(SWAPPER_PG_DIR) as usize)
}

fn swapper_kernel_pg_table_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(SWAPPER_KERNEL_PG_TABLE) as usize,
    )
}

fn swapper_linear_pg_tables_addr(kernel_image: &KernelImage) -> usize {
    runtime_addr(
        kernel_image,
        core::ptr::addr_of!(SWAPPER_LINEAR_PG_TABLES) as usize,
    )
}

fn runtime_addr(kernel_image: &KernelImage, link_addr: usize) -> usize {
    if crate::arch::riscv64::csr::read_satp() == 0 {
        kernel_image.link_to_phys(link_addr).unwrap_or(link_addr)
    } else {
        link_addr
    }
}

fn kernel_runtime_range(
    config: &Config,
    kernel_image: &KernelImage,
    kernel_start: usize,
    kernel_end: usize,
) -> Option<(usize, usize, usize)> {
    let kernel_link_start = kernel_image.runtime_to_link(kernel_start)?;
    let kernel_link_end = kernel_image.runtime_to_link(kernel_end)?;
    let kernel_phys_start = kernel_image.runtime_to_phys(kernel_start)?;
    let kernel_phys_end = kernel_image.runtime_to_phys(kernel_end)?;
    let kernel_link_size = kernel_link_end.checked_sub(kernel_link_start)?;
    let kernel_phys_size = kernel_phys_end.checked_sub(kernel_phys_start)?;
    if kernel_link_start != config.kernel_link_addr()
        || kernel_phys_start != kernel_image.phys_start()
        || kernel_link_size == 0
        || kernel_link_size != kernel_phys_size
        || !aligned(kernel_link_start, config.pmd_size())
        || !aligned(kernel_phys_start, config.pmd_size())
    {
        return None;
    }

    Some((kernel_link_start, kernel_phys_start, kernel_link_size))
}

fn early_pg_dir_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // Early boot is still single-threaded here; this function is the object
    // boundary that owns initialization of StaticObjects.early_pg_dir.
    unsafe { &mut *(early_pg_dir_addr(kernel_image) as *mut PageTablePage) }
}

fn trampoline_pg_dir_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // TrampolineVm.Setup owns initialization of StaticObjects.trampoline_pg_dir.
    unsafe { &mut *(trampoline_pg_dir_addr(kernel_image) as *mut PageTablePage) }
}

fn trampoline_kernel_pg_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // TrampolineVm.Setup owns this subordinate page table while constructing trampoline_pg_dir.
    unsafe { &mut *(trampoline_kernel_pg_table_addr(kernel_image) as *mut PageTablePage) }
}

fn early_kernel_pg_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *(early_kernel_pg_table_addr(kernel_image) as *mut PageTablePage) }
}

fn early_fixmap_l1_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *(early_fixmap_l1_table_addr(kernel_image) as *mut PageTablePage) }
}

fn early_fixmap_l0_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *(early_fixmap_l0_table_addr(kernel_image) as *mut PageTablePage) }
}

fn swapper_pg_dir_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    unsafe { &mut *(swapper_pg_dir_addr(kernel_image) as *mut PageTablePage) }
}

fn swapper_kernel_pg_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    unsafe { &mut *(swapper_kernel_pg_table_addr(kernel_image) as *mut PageTablePage) }
}

fn swapper_linear_pg_tables_mut(
    kernel_image: &KernelImage,
) -> &'static mut [PageTablePage; SWAPPER_L1_TABLES] {
    unsafe {
        &mut *(swapper_linear_pg_tables_addr(kernel_image)
            as *mut [PageTablePage; SWAPPER_L1_TABLES])
    }
}

static mut TRAMPOLINE_PG_DIR: PageTablePage = PageTablePage::zeroed();
static mut TRAMPOLINE_KERNEL_PG_TABLE: PageTablePage = PageTablePage::zeroed();
static mut EARLY_PG_DIR: PageTablePage = PageTablePage::zeroed();
static mut EARLY_KERNEL_PG_TABLE: PageTablePage = PageTablePage::zeroed();
static mut EARLY_FIXMAP_L1_TABLE: PageTablePage = PageTablePage::zeroed();
static mut EARLY_FIXMAP_L0_TABLE: PageTablePage = PageTablePage::zeroed();
static mut SWAPPER_PG_DIR: PageTablePage = PageTablePage::zeroed();
static mut SWAPPER_KERNEL_PG_TABLE: PageTablePage = PageTablePage::zeroed();
static mut SWAPPER_LINEAR_PG_TABLES: [PageTablePage; SWAPPER_L1_TABLES] =
    [const { PageTablePage::zeroed() }; SWAPPER_L1_TABLES];
