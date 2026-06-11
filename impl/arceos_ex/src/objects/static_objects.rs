use super::{
    config::Config,
    fix_map::FixMap,
    kernel_image::KernelImage,
    memblock::MemBlock,
    page_table::{
        aligned, map_linear_pmd_range, map_page_range, map_pmd_range, PageTableInstallRange,
        PageTablePageSlot, SWAPPER_VMALLOC_L0_TABLES, VMALLOC_RUNTIME_L0_TABLE_SLOTS,
    },
    raw_dtb::RawDtb,
    state::{Lifecycle, State},
    static_page_tables,
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
        static_page_tables::storage_ready(config)
    }

    pub fn trampoline_satp(&self, kernel_image: &KernelImage) -> Option<usize> {
        static_page_tables::trampoline_satp(kernel_image)
    }

    pub fn early_satp(&self, kernel_image: &KernelImage) -> Option<usize> {
        static_page_tables::early_satp(kernel_image)
    }

    pub fn swapper_satp(&self, kernel_image: &KernelImage) -> Option<usize> {
        static_page_tables::swapper_satp(kernel_image)
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
            static_page_tables::kernel_runtime_range(
                config,
                kernel_image,
                kernel_start,
                kernel_end,
            )
        else {
            return false;
        };

        let root = static_page_tables::early_pg_dir_mut(kernel_image);
        let kernel_table = static_page_tables::early_kernel_pg_table_mut(kernel_image);
        let fixmap_l1_table = static_page_tables::early_fixmap_l1_table_mut(kernel_image);
        let fixmap_l0_table = static_page_tables::early_fixmap_l0_table_mut(kernel_image);
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

        let root = static_page_tables::trampoline_pg_dir_mut(kernel_image);
        let kernel_table = static_page_tables::trampoline_kernel_pg_table_mut(kernel_image);
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
            static_page_tables::kernel_runtime_range(
                config,
                kernel_image,
                kernel_start,
                kernel_end,
            )
        else {
            return false;
        };

        let root = static_page_tables::swapper_pg_dir_mut(kernel_image);
        let kernel_table = static_page_tables::swapper_kernel_pg_table_mut(kernel_image);
        let vmalloc_l1_table = static_page_tables::swapper_vmalloc_l1_table_mut(kernel_image);
        let vmalloc_l0_tables = static_page_tables::swapper_vmalloc_l0_tables_mut(kernel_image);
        let linear_tables = static_page_tables::swapper_linear_pg_tables_mut(kernel_image);
        root.clear();
        kernel_table.clear();
        vmalloc_l1_table.clear();
        for table in vmalloc_l0_tables.iter_mut() {
            table.clear();
        }
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

    pub fn swapper_vmalloc_install_range(
        &self,
        kernel_image: &KernelImage,
        page_size: usize,
    ) -> Option<PageTableInstallRange> {
        let root = static_page_tables::swapper_pg_dir_mut(kernel_image);
        let vmalloc_l1_table = static_page_tables::swapper_vmalloc_l1_table_mut(kernel_image);
        let vmalloc_l0_tables = static_page_tables::swapper_vmalloc_l0_tables_mut(kernel_image);
        let root_addr = root as *mut _ as usize;
        let l1_addr = vmalloc_l1_table as *mut _ as usize;
        let l1_phys = kernel_image.runtime_to_phys(l1_addr)?;
        let mut slots = [PageTablePageSlot::empty(); VMALLOC_RUNTIME_L0_TABLE_SLOTS];
        let mut index = 0usize;
        while index < SWAPPER_VMALLOC_L0_TABLES {
            let l0_addr = &mut vmalloc_l0_tables[index] as *mut _ as usize;
            let l0_phys = kernel_image.runtime_to_phys(l0_addr)?;
            slots[index] = PageTablePageSlot::new(l0_addr, l0_phys);
            index += 1;
        }
        Some(PageTableInstallRange::new(
            root_addr,
            l1_addr,
            l1_phys,
            slots,
            super::mm_core::VMALLOC_START,
            page_size,
        ))
    }
}
