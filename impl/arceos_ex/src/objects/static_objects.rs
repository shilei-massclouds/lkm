use super::{
    config::Config,
    fix_map::FixMap,
    kernel_image::KernelImage,
    memblock::MemBlock,
    page_table::{
        aligned, map_linear_pmd_range, map_page_range, map_pmd_range, page_table_storage_ready,
        PageTablePage, SWAPPER_L1_TABLES,
    },
    raw_dtb::RawDtb,
    state::{Lifecycle, State},
};

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
