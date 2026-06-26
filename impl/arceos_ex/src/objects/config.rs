use super::{
    fix_map::FixMapSlot,
    state::{Lifecycle, State},
};

const PAGE_SIZE: usize = 4096;
const PMD_SIZE: usize = 2 * 1024 * 1024;
const FDT_SLOT_SIZE: usize = 2 * 1024 * 1024;
const KERNEL_LINK_ADDR: usize = 0xffff_ffff_8000_0000;
const FDT_FIXMAP_VIRT_START: usize = 0xffff_ffc0_0000_0000;
const LINEAR_MAP_VIRT_START: usize = 0xffff_ffd0_0000_0000;

pub struct Config {
    lifecycle: Lifecycle,
    page_size: usize,
    pmd_size: usize,
    kernel_link_addr: usize,
    fixmap: FixMapConfig,
}

impl Config {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Online),
            page_size: PAGE_SIZE,
            pmd_size: PMD_SIZE,
            kernel_link_addr: KERNEL_LINK_ADDR,
            fixmap: FixMapConfig::new(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn page_size(&self) -> usize {
        self.page_size
    }

    pub const fn pmd_size(&self) -> usize {
        self.pmd_size
    }

    pub const fn kernel_link_addr(&self) -> usize {
        self.kernel_link_addr
    }

    pub const fn stack_depot_enabled(&self) -> bool {
        true
    }

    pub const fn stack_depot_always_init(&self) -> bool {
        false
    }

    pub const fn irq_stacks_enabled(&self) -> bool {
        true
    }

    pub const fn vmap_stack_enabled(&self) -> bool {
        true
    }

    pub const fn shadow_call_stack_enabled(&self) -> bool {
        false
    }

    pub const fn rcu_nocb_cpu_enabled(&self) -> bool {
        false
    }

    pub const fn kfence_enabled(&self) -> bool {
        false
    }

    pub const fn debug_lock_alloc_enabled(&self) -> bool {
        false
    }

    pub const fn debug_locking_api_selftests_enabled(&self) -> bool {
        false
    }

    pub const fn blk_dev_initrd_enabled(&self) -> bool {
        false
    }

    pub const fn numa_enabled(&self) -> bool {
        false
    }

    pub const fn acpi_enabled(&self) -> bool {
        false
    }

    pub const fn riscv_late_time_init_hook_set(&self) -> bool {
        false
    }

    pub const fn arch_has_cpu_finalize_init(&self) -> bool {
        false
    }

    pub const fn x86_arch(&self) -> bool {
        false
    }

    pub const fn lockdep_enabled(&self) -> bool {
        false
    }

    pub const fn kgdb_enabled(&self) -> bool {
        false
    }

    pub const fn net_ns_enabled(&self) -> bool {
        true
    }

    pub const fn proc_fs_enabled(&self) -> bool {
        true
    }

    pub const fn pid_ns_enabled(&self) -> bool {
        true
    }

    pub const fn cpusets_enabled(&self) -> bool {
        false
    }

    pub const fn cgroups_enabled(&self) -> bool {
        false
    }

    pub const fn taskstats_enabled(&self) -> bool {
        false
    }

    pub const fn task_delay_acct_enabled(&self) -> bool {
        false
    }

    pub const fn kcsan_enabled(&self) -> bool {
        false
    }

    pub const fn linear_map_virt_start(&self) -> usize {
        LINEAR_MAP_VIRT_START
    }

    pub fn phys_to_linear(&self, addr: usize) -> Option<usize> {
        addr.checked_add(self.linear_map_virt_start())
    }

    pub fn entry_prelude_ready(&self) -> bool {
        self.state() == State::Online
            && self.page_size != 0
            && self.page_size.is_power_of_two()
            && self.pmd_size >= self.page_size
            && self.pmd_size.is_multiple_of(self.page_size)
            && self.kernel_link_addr != 0
            && self.kernel_link_addr.is_multiple_of(self.page_size)
            && self.fixmap.fdt().page_size() == self.page_size
    }

    pub const fn fixmap(&self) -> &FixMapConfig {
        &self.fixmap
    }
}

pub struct FixMapConfig {
    fdt: FixMapSlot,
}

impl FixMapConfig {
    const fn new() -> Self {
        Self {
            fdt: FixMapSlot::new(PAGE_SIZE, FDT_SLOT_SIZE / PAGE_SIZE, FDT_FIXMAP_VIRT_START),
        }
    }

    pub const fn fdt(&self) -> FixMapSlot {
        self.fdt
    }
}
