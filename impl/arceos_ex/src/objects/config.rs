use super::{
    fix_map::FixMapSlot,
    state::{Lifecycle, State},
};

#[cfg(any(
    all(app_hello, app_smoke),
    all(app_hello, app_user_boot),
    all(app_smoke, app_user_boot)
))]
compile_error!("exactly one selected payload cfg must be enabled");

#[cfg(not(any(app_hello, app_smoke, app_user_boot)))]
compile_error!("one selected payload cfg must be enabled");

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SelectedPayloadKind {
    Hello,
    #[cfg_attr(not(app_smoke), allow(dead_code))]
    Smoke,
    // Constructed only when the user-boot payload cfg is selected.
    #[allow(dead_code)]
    UserBoot,
}

#[cfg(app_hello)]
const SELECTED_PAYLOAD_KIND: SelectedPayloadKind = SelectedPayloadKind::Hello;
#[cfg(app_smoke)]
const SELECTED_PAYLOAD_KIND: SelectedPayloadKind = SelectedPayloadKind::Smoke;
#[cfg(app_user_boot)]
const SELECTED_PAYLOAD_KIND: SelectedPayloadKind = SelectedPayloadKind::UserBoot;

const PAGE_SIZE: usize = 4096;
const PMD_SIZE: usize = 2 * 1024 * 1024;
const FDT_SLOT_SIZE: usize = 2 * 1024 * 1024;
const KERNEL_LINK_ADDR: usize = 0xffff_ffff_8000_0000;
const FDT_FIXMAP_VIRT_START: usize = 0xffff_ffc0_0000_0000;
const LINEAR_MAP_VIRT_START: usize = 0xffff_ffd0_0000_0000;

const LINUX_MAX_ARG_STRINGS: usize = 0x7fff_ffff;
const LINUX_MAX_ARG_STRLEN: usize = PAGE_SIZE * 32;
const LINUX_ARG_MAX: usize = PAGE_SIZE * 32;
const LINUX_STK_LIM: usize = 8 * 1024 * 1024;
const LINUX_INIT_RLIMIT_STACK: usize = LINUX_STK_LIM;
const LINUX_STACK_INITIAL_EXPAND: usize = 128 * 1024;
const LINUX_STACK_GUARD_GAP: usize = 256 * PAGE_SIZE;
const USER_STACK_TOP: usize = 0x4000_0000;
const USER_STACK_RANDOM_BYTES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserStackConfig {
    initial_expand: usize,
    rlimit_stack: usize,
    guard_gap: usize,
    stack_top: usize,
    random_bytes: usize,
}

impl UserStackConfig {
    pub const fn linux_default() -> Self {
        Self {
            initial_expand: LINUX_STACK_INITIAL_EXPAND,
            rlimit_stack: LINUX_INIT_RLIMIT_STACK,
            guard_gap: LINUX_STACK_GUARD_GAP,
            stack_top: USER_STACK_TOP,
            random_bytes: USER_STACK_RANDOM_BYTES,
        }
    }

    pub const fn initial_expand(self) -> usize {
        self.initial_expand
    }

    pub const fn rlimit_stack(self) -> usize {
        self.rlimit_stack
    }

    pub const fn guard_gap(self) -> usize {
        self.guard_gap
    }

    pub const fn stack_top(self) -> usize {
        self.stack_top
    }

    pub const fn random_bytes(self) -> usize {
        self.random_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecArgumentLimits {
    max_arg_strings: usize,
    max_arg_strlen: usize,
    arg_max_floor: usize,
    stack_rlimit: usize,
    stk_lim: usize,
    argument_bytes: usize,
}

#[allow(dead_code)]
impl ExecArgumentLimits {
    pub const fn linux_default() -> Self {
        let stk_fraction = LINUX_STK_LIM / 4 * 3;
        let rlimit_fraction = LINUX_INIT_RLIMIT_STACK / 4;
        let smaller = if stk_fraction < rlimit_fraction {
            stk_fraction
        } else {
            rlimit_fraction
        };
        let argument_bytes = if smaller > LINUX_ARG_MAX {
            smaller
        } else {
            LINUX_ARG_MAX
        };
        Self {
            max_arg_strings: LINUX_MAX_ARG_STRINGS,
            max_arg_strlen: LINUX_MAX_ARG_STRLEN,
            arg_max_floor: LINUX_ARG_MAX,
            stack_rlimit: LINUX_INIT_RLIMIT_STACK,
            stk_lim: LINUX_STK_LIM,
            argument_bytes,
        }
    }

    pub const fn max_arg_strings(self) -> usize {
        self.max_arg_strings
    }

    pub const fn max_arg_strlen(self) -> usize {
        self.max_arg_strlen
    }

    pub const fn arg_max_floor(self) -> usize {
        self.arg_max_floor
    }

    pub const fn stack_rlimit(self) -> usize {
        self.stack_rlimit
    }

    pub const fn stk_lim(self) -> usize {
        self.stk_lim
    }

    pub const fn argument_bytes(self) -> usize {
        self.argument_bytes
    }

    pub fn accepts(self, argc: usize, envc: usize, string_bytes: usize) -> bool {
        if argc > self.max_arg_strings || envc > self.max_arg_strings {
            return false;
        }
        let Some(pointer_count) = core::cmp::max(argc, 1).checked_add(envc) else {
            return false;
        };
        let Some(pointer_bytes) = pointer_count.checked_mul(core::mem::size_of::<usize>()) else {
            return false;
        };
        pointer_bytes < self.argument_bytes && string_bytes <= self.argument_bytes - pointer_bytes
    }
}

pub struct Config {
    lifecycle: Lifecycle,
    page_size: usize,
    pmd_size: usize,
    kernel_link_addr: usize,
    fixmap: FixMapConfig,
    selected_payload_kind: SelectedPayloadKind,
    #[allow(dead_code)]
    exec_argument_limits: ExecArgumentLimits,
    #[cfg_attr(app_hello, allow(dead_code))]
    user_stack: UserStackConfig,
}

impl Config {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Online),
            page_size: PAGE_SIZE,
            pmd_size: PMD_SIZE,
            kernel_link_addr: KERNEL_LINK_ADDR,
            fixmap: FixMapConfig::new(),
            selected_payload_kind: SELECTED_PAYLOAD_KIND,
            exec_argument_limits: ExecArgumentLimits::linux_default(),
            user_stack: UserStackConfig::linux_default(),
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

    pub const fn selected_payload_kind(&self) -> SelectedPayloadKind {
        self.selected_payload_kind
    }

    #[allow(dead_code)]
    pub const fn exec_argument_limits(&self) -> ExecArgumentLimits {
        self.exec_argument_limits
    }

    #[cfg_attr(app_hello, allow(dead_code))]
    pub const fn user_stack(&self) -> UserStackConfig {
        self.user_stack
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

    pub const fn kunit_enabled(&self) -> bool {
        false
    }

    pub const fn md_enabled(&self) -> bool {
        true
    }

    pub const fn root_nfs_enabled(&self) -> bool {
        true
    }

    pub const fn cifs_root_enabled(&self) -> bool {
        false
    }

    pub const fn ext2_fs_enabled(&self) -> bool {
        false
    }

    pub const fn ext4_use_for_ext2_enabled(&self) -> bool {
        true
    }

    pub const fn devtmpfs_enabled(&self) -> bool {
        true
    }

    pub const fn devtmpfs_mount_enabled(&self) -> bool {
        true
    }

    pub const fn integrity_enabled(&self) -> bool {
        true
    }

    pub const fn ima_enabled(&self) -> bool {
        false
    }

    pub const fn evm_enabled(&self) -> bool {
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
