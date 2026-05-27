use crate::arch::riscv64::csr;

use super::{
    config::Config,
    kernel_image::KernelImage,
    page_table::{aligned, page_table_storage_ready, PageTablePage, SWAPPER_L1_TABLES},
};

pub(super) fn storage_ready(config: &Config) -> bool {
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

pub(super) fn trampoline_satp(kernel_image: &KernelImage) -> Option<usize> {
    satp_from_root(kernel_image, trampoline_pg_dir_addr(kernel_image))
}

pub(super) fn early_satp(kernel_image: &KernelImage) -> Option<usize> {
    satp_from_root(kernel_image, early_pg_dir_addr(kernel_image))
}

pub(super) fn swapper_satp(kernel_image: &KernelImage) -> Option<usize> {
    satp_from_root(kernel_image, swapper_pg_dir_addr(kernel_image))
}

pub(super) fn kernel_runtime_range(
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

pub(super) fn early_pg_dir_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns initialization of StaticObjects.early_pg_dir.
    unsafe { &mut *(early_pg_dir_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn trampoline_pg_dir_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // TrampolineVm.Setup owns initialization of StaticObjects.trampoline_pg_dir.
    unsafe { &mut *(trampoline_pg_dir_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn trampoline_kernel_pg_table_mut(
    kernel_image: &KernelImage,
) -> &'static mut PageTablePage {
    // TrampolineVm.Setup owns this subordinate page table while constructing trampoline_pg_dir.
    unsafe { &mut *(trampoline_kernel_pg_table_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn early_kernel_pg_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *(early_kernel_pg_table_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn early_fixmap_l1_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *(early_fixmap_l1_table_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn early_fixmap_l0_table_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    // EarlyVm.Setup owns this subordinate page table while constructing early_pg_dir.
    unsafe { &mut *(early_fixmap_l0_table_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn swapper_pg_dir_mut(kernel_image: &KernelImage) -> &'static mut PageTablePage {
    unsafe { &mut *(swapper_pg_dir_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn swapper_kernel_pg_table_mut(
    kernel_image: &KernelImage,
) -> &'static mut PageTablePage {
    unsafe { &mut *(swapper_kernel_pg_table_addr(kernel_image) as *mut PageTablePage) }
}

pub(super) fn swapper_linear_pg_tables_mut(
    kernel_image: &KernelImage,
) -> &'static mut [PageTablePage; SWAPPER_L1_TABLES] {
    unsafe {
        &mut *(swapper_linear_pg_tables_addr(kernel_image)
            as *mut [PageTablePage; SWAPPER_L1_TABLES])
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
        .map(|root_phys| csr::SATP_MODE_SV39 | (root_phys >> 12))
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
    if csr::read_satp() == 0 {
        kernel_image.link_to_phys(link_addr).unwrap_or(link_addr)
    } else {
        link_addr
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
