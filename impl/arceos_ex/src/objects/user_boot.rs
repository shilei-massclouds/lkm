use core::sync::atomic::{AtomicU8, Ordering};

use crate::arch::riscv64::csr;

#[cfg(app_user_boot)]
use super::files::STDIN_READY_FIXTURE;
#[cfg(app_user_boot)]
use super::{
    block_device::BlockDeviceRegistry, boot_param::BootParam, command_line::StaticCommandLine,
    ext2::Ext2FileSystem, vfs::VfsCore, virtio_blk,
};
use super::{
    event_stream::TrapFrame,
    exception_stream::{ExceptionStream, SyscallTable},
    files::FilesStruct,
    kernel_image::KernelImage,
    mm_core::{GfpFlags, KernelGlobalAllocator, PageAllocator, PageMetadataMap, PageRef},
    page_table::{
        copy_high_half_root_entries, page_table_storage_ready, sv39_indices, table_pte_from_phys,
        user_leaf_pte_from_phys, PageTablePage,
    },
    rest_init::{KernelInitTask, SystemState, SystemStateValue},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_page_tables,
    swapper_vm::SwapperVm,
    task::TaskEntry,
    vfs::FsStruct,
};

pub const USER_INIT_PATH: &[u8] = b"/sbin/init";
pub const USER_ETC_INIT_PATH: &[u8] = b"/etc/init";
pub const USER_BIN_INIT_PATH: &[u8] = b"/bin/init";
pub const USER_BIN_SH_PATH: &[u8] = b"/bin/sh";
pub const USER_INIT_EXPECTED_MESSAGE: &[u8] = b"user hello\n";
const USER_SMOKE_STDIN_MARKER: &[u8] = b"user-smoke: begin";
pub const USER_SIGNAL_COUNT: usize = 64;
pub const USER_CHILD_PID: usize = 3;
pub const USER_CLONE_SIGCHLD: usize = 17;
pub const USER_WAIT4_ALL_CHILDREN: usize = usize::MAX;
pub const USER_WAIT4_WUNTRACED: usize = 2;

pub const ELF_HEADER_LEN: usize = 64;
pub const USER_BOOT_READ_MAX: usize = super::ext2::EXT2_SINGLE_INDIRECT_READ_MAX;
pub const USER_STACK_SIZE: usize = 16 * 1024;
pub const USER_STACK_TOP: usize = 0x4000_0000;
pub const USER_HEAP_BASE: usize = 0x3000_0000;
pub const USER_HEAP_SIZE: usize = 2 * 1024 * 1024;
pub const USER_PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy)]
pub struct UserSignalAction {
    handler: usize,
    flags: usize,
    mask: usize,
}

impl UserSignalAction {
    pub const fn new(handler: usize, flags: usize, mask: usize) -> Self {
        Self {
            handler,
            flags,
            mask,
        }
    }

    pub const fn default() -> Self {
        Self::new(0, 0, 0)
    }

    pub const fn handler(&self) -> usize {
        self.handler
    }

    pub const fn flags(&self) -> usize {
        self.flags
    }

    pub const fn mask(&self) -> usize {
        self.mask
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessGroupLookup {
    Found(usize),
    NoSuchProcess,
    NotReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessGroupUpdate {
    Updated(usize),
    Invalid,
    NoSuchProcess,
    PermissionDenied,
    NotReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserMmapError {
    Invalid,
    NoMemory,
}

fn pid_t_arg(value: usize) -> i32 {
    value as u32 as i32
}

#[cfg(app_user_boot)]
pub const USER_KERNEL_TRAP_STACK_SIZE: usize = 4096;
pub const USER_MAIN_PIE_LOAD_BIAS: usize = 0x1000_0000;
pub const USER_INTERPRETER_LOAD_BIAS: usize = 0x2000_0000;
const USER_INITIAL_STACK_WORDS: usize = 18;
#[cfg(app_user_boot)]
const USER_INIT_CANDIDATES: [UserInitPathRef; 4] = [
    UserInitPathRef::DefaultInit,
    UserInitPathRef::EtcInit,
    UserInitPathRef::BinInit,
    UserInitPathRef::BinSh,
];
const AT_NULL: usize = 0;
const AT_PHDR: usize = 3;
const AT_PHENT: usize = 4;
const AT_PHNUM: usize = 5;
const AT_PAGESZ: usize = 6;
const AT_BASE: usize = 7;
const AT_ENTRY: usize = 9;
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_TYPE_DYN: u16 = 3;
const ELF_MACHINE_RISCV: u16 = 243;
const ELF_PHDR_TYPE_LOAD: u32 = 1;
const ELF_PHDR_TYPE_INTERP: u32 = 3;
const ELF_PHDR_TYPE_PHDR: u32 = 6;
const ELF_PF_X: u32 = 1;
const ELF_PF_W: u32 = 2;
const ELF_PF_R: u32 = 4;
const ELF64_PHDR_SIZE: usize = 56;
const ELF_INTERP_PATH_MAX: usize = 128;
const USER_SELECTED_PATH_MAX: usize = 128;
const MAX_LOAD_SEGMENTS: usize = 8;
const MAX_STACK_PAGES: usize = USER_STACK_SIZE / USER_PAGE_SIZE;
const MAX_USER_MAPPINGS: usize = MAX_LOAD_SEGMENTS * 2 + 2;
const MAX_MAPPING_BACKING_PAGES: usize = 512;
const MAX_USER_L0_TABLES: usize = MAX_USER_MAPPINGS + 2;
const USER_CLONE_CSIGNAL_MASK: usize = 0xff;
const USER_CLONE_SETTLS: usize = 0x0008_0000;

pub struct PayloadExecSyncBoundaries {
    lifecycle: Lifecycle,
    kernel_execve_linux_window_bound: bool,
    binfmt_lock_deferred: bool,
    cred_guard_mutex_deferred: bool,
    exec_update_lock_deferred: bool,
    exec_mmap_local_irq_deferred: bool,
    exec_task_siglock_deferred: bool,
    exec_tasklist_lock_deferred: bool,
    exec_fs_lock_rcu_deferred: bool,
    exec_mmap_lock_deferred: bool,
    exec_membarrier_deferred: bool,
    bprm_mm_init_task_lock_deferred: bool,
    exec_mmap_task_lock_deferred: bool,
    exec_sched_mm_cid_deferred: bool,
    exec_files_unshare_cloexec_deferred: bool,
    exec_io_uring_cancel_deferred: bool,
    exec_posix_timer_siglock_deferred: bool,
    exec_namespace_switch_deferred: bool,
    exec_success_accounting_hooks_deferred: bool,
    exec_full_binfmt_deferred: bool,
    binfmt_module_retry_trimmed_noop: bool,
    binfmt_module_retry_trimmed_because_modules_disabled: bool,
    ramdisk_init_branch_trimmed_noop: bool,
    ramdisk_init_trimmed_because_config_initrd_disabled: bool,
    default_init_branch_trimmed_noop: bool,
    default_init_trimmed_because_config_default_init_empty: bool,
    binfmt_script_deferred: bool,
    exec_panic_terminal_bound: bool,
}

#[allow(dead_code)]
impl PayloadExecSyncBoundaries {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kernel_execve_linux_window_bound: false,
            binfmt_lock_deferred: false,
            cred_guard_mutex_deferred: false,
            exec_update_lock_deferred: false,
            exec_mmap_local_irq_deferred: false,
            exec_task_siglock_deferred: false,
            exec_tasklist_lock_deferred: false,
            exec_fs_lock_rcu_deferred: false,
            exec_mmap_lock_deferred: false,
            exec_membarrier_deferred: false,
            bprm_mm_init_task_lock_deferred: false,
            exec_mmap_task_lock_deferred: false,
            exec_sched_mm_cid_deferred: false,
            exec_files_unshare_cloexec_deferred: false,
            exec_io_uring_cancel_deferred: false,
            exec_posix_timer_siglock_deferred: false,
            exec_namespace_switch_deferred: false,
            exec_success_accounting_hooks_deferred: false,
            exec_full_binfmt_deferred: false,
            binfmt_module_retry_trimmed_noop: false,
            binfmt_module_retry_trimmed_because_modules_disabled: false,
            ramdisk_init_branch_trimmed_noop: false,
            ramdisk_init_trimmed_because_config_initrd_disabled: false,
            default_init_branch_trimmed_noop: false,
            default_init_trimmed_because_config_default_init_empty: false,
            binfmt_script_deferred: false,
            exec_panic_terminal_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kernel_execve_linux_window_bound(&self) -> bool {
        self.kernel_execve_linux_window_bound
    }

    pub const fn binfmt_lock_deferred(&self) -> bool {
        self.binfmt_lock_deferred
    }

    pub const fn cred_guard_mutex_deferred(&self) -> bool {
        self.cred_guard_mutex_deferred
    }

    pub const fn exec_update_lock_deferred(&self) -> bool {
        self.exec_update_lock_deferred
    }

    pub const fn exec_mmap_local_irq_deferred(&self) -> bool {
        self.exec_mmap_local_irq_deferred
    }

    pub const fn exec_task_siglock_deferred(&self) -> bool {
        self.exec_task_siglock_deferred
    }

    pub const fn exec_tasklist_lock_deferred(&self) -> bool {
        self.exec_tasklist_lock_deferred
    }

    pub const fn exec_fs_lock_rcu_deferred(&self) -> bool {
        self.exec_fs_lock_rcu_deferred
    }

    pub const fn exec_mmap_lock_deferred(&self) -> bool {
        self.exec_mmap_lock_deferred
    }

    pub const fn exec_membarrier_deferred(&self) -> bool {
        self.exec_membarrier_deferred
    }

    pub const fn bprm_mm_init_task_lock_deferred(&self) -> bool {
        self.bprm_mm_init_task_lock_deferred
    }

    pub const fn exec_mmap_task_lock_deferred(&self) -> bool {
        self.exec_mmap_task_lock_deferred
    }

    pub const fn exec_sched_mm_cid_deferred(&self) -> bool {
        self.exec_sched_mm_cid_deferred
    }

    pub const fn exec_files_unshare_cloexec_deferred(&self) -> bool {
        self.exec_files_unshare_cloexec_deferred
    }

    pub const fn exec_io_uring_cancel_deferred(&self) -> bool {
        self.exec_io_uring_cancel_deferred
    }

    pub const fn exec_posix_timer_siglock_deferred(&self) -> bool {
        self.exec_posix_timer_siglock_deferred
    }

    pub const fn exec_namespace_switch_deferred(&self) -> bool {
        self.exec_namespace_switch_deferred
    }

    pub const fn exec_success_accounting_hooks_deferred(&self) -> bool {
        self.exec_success_accounting_hooks_deferred
    }

    pub const fn exec_full_binfmt_deferred(&self) -> bool {
        self.exec_full_binfmt_deferred
    }

    pub const fn binfmt_module_retry_trimmed_noop(&self) -> bool {
        self.binfmt_module_retry_trimmed_noop
    }

    pub const fn binfmt_module_retry_trimmed_because_modules_disabled(&self) -> bool {
        self.binfmt_module_retry_trimmed_because_modules_disabled
    }

    pub const fn ramdisk_init_branch_trimmed_noop(&self) -> bool {
        self.ramdisk_init_branch_trimmed_noop
    }

    pub const fn ramdisk_init_trimmed_because_config_initrd_disabled(&self) -> bool {
        self.ramdisk_init_trimmed_because_config_initrd_disabled
    }

    pub const fn default_init_branch_trimmed_noop(&self) -> bool {
        self.default_init_branch_trimmed_noop
    }

    pub const fn default_init_trimmed_because_config_default_init_empty(&self) -> bool {
        self.default_init_trimmed_because_config_default_init_empty
    }

    pub const fn binfmt_script_deferred(&self) -> bool {
        self.binfmt_script_deferred
    }

    pub const fn exec_panic_terminal_bound(&self) -> bool {
        self.exec_panic_terminal_bound
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        system_state: &SystemState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || system_state.state() != State::Online
            || system_state.value() != SystemStateValue::Running
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.kernel_execve_linux_window_bound = true;
        self.binfmt_lock_deferred = true;
        self.cred_guard_mutex_deferred = true;
        self.exec_update_lock_deferred = true;
        self.exec_mmap_local_irq_deferred = true;
        self.exec_task_siglock_deferred = true;
        self.exec_tasklist_lock_deferred = true;
        self.exec_fs_lock_rcu_deferred = true;
        self.exec_mmap_lock_deferred = true;
        self.exec_membarrier_deferred = true;
        self.bprm_mm_init_task_lock_deferred = true;
        self.exec_mmap_task_lock_deferred = true;
        self.exec_sched_mm_cid_deferred = true;
        self.exec_files_unshare_cloexec_deferred = true;
        self.exec_io_uring_cancel_deferred = true;
        self.exec_posix_timer_siglock_deferred = true;
        self.exec_namespace_switch_deferred = true;
        self.exec_success_accounting_hooks_deferred = true;
        self.exec_full_binfmt_deferred = true;
        self.binfmt_module_retry_trimmed_noop = true;
        self.binfmt_module_retry_trimmed_because_modules_disabled = true;
        self.ramdisk_init_branch_trimmed_noop = true;
        self.ramdisk_init_trimmed_because_config_initrd_disabled = true;
        self.default_init_branch_trimmed_noop = true;
        self.default_init_trimmed_because_config_default_init_empty = true;
        self.binfmt_script_deferred = true;
        self.exec_panic_terminal_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct UserCloneDeferredBoundaries {
    lifecycle: Lifecycle,
    linux_6_12_legacy_clone_bound: bool,
    riscv_abi_argument_order_bound: bool,
    observed_plain_fork_args_bound: bool,
    plain_fork_first_slice_bound: bool,
    csignal_split_bound: bool,
    sigchld_exit_signal_bound: bool,
    newsp_zero_inherits_parent_sp: bool,
    tls_ignored_without_clone_settls: bool,
    thread_group_deferred: bool,
    clone_vm_vfork_deferred: bool,
    cow_mm_deferred: bool,
    pidfd_deferred: bool,
    ptrace_seccomp_cgroup_audit_deferred: bool,
    namespace_deferred: bool,
    robust_futex_deferred: bool,
    clear_child_futex_deferred: bool,
    wait_exit_reap_deferred: bool,
    unsupported_flags_first_slice: bool,
}

#[allow(dead_code)]
impl UserCloneDeferredBoundaries {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            linux_6_12_legacy_clone_bound: false,
            riscv_abi_argument_order_bound: false,
            observed_plain_fork_args_bound: false,
            plain_fork_first_slice_bound: false,
            csignal_split_bound: false,
            sigchld_exit_signal_bound: false,
            newsp_zero_inherits_parent_sp: false,
            tls_ignored_without_clone_settls: false,
            thread_group_deferred: true,
            clone_vm_vfork_deferred: true,
            cow_mm_deferred: true,
            pidfd_deferred: true,
            ptrace_seccomp_cgroup_audit_deferred: true,
            namespace_deferred: true,
            robust_futex_deferred: true,
            clear_child_futex_deferred: true,
            wait_exit_reap_deferred: true,
            unsupported_flags_first_slice: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn plain_fork_first_slice_bound(&self) -> bool {
        self.plain_fork_first_slice_bound
    }

    pub const fn wait_exit_reap_deferred(&self) -> bool {
        self.wait_exit_reap_deferred
    }

    pub fn setup(&mut self, exec_sync: &PayloadExecSyncBoundaries) -> EventResult {
        if self.lifecycle.state() != State::Base || exec_sync.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.linux_6_12_legacy_clone_bound = true;
        self.riscv_abi_argument_order_bound = true;
        self.observed_plain_fork_args_bound = true;
        self.plain_fork_first_slice_bound = true;
        self.csignal_split_bound = true;
        self.sigchld_exit_signal_bound = true;
        self.newsp_zero_inherits_parent_sp = true;
        self.tls_ignored_without_clone_settls = true;
        self.thread_group_deferred = true;
        self.clone_vm_vfork_deferred = true;
        self.cow_mm_deferred = true;
        self.pidfd_deferred = true;
        self.ptrace_seccomp_cgroup_audit_deferred = true;
        self.namespace_deferred = true;
        self.robust_futex_deferred = true;
        self.clear_child_futex_deferred = true;
        self.wait_exit_reap_deferred = true;
        self.unsupported_flags_first_slice = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn accepts_plain_fork_first_slice(&self, clone_flags: usize, newsp: usize) -> bool {
        self.lifecycle.state() == State::Ready
            && self.linux_6_12_legacy_clone_bound
            && self.riscv_abi_argument_order_bound
            && self.plain_fork_first_slice_bound
            && self.clone_flags_without_csignal(clone_flags) == 0
            && self.exit_signal(clone_flags) == USER_CLONE_SIGCHLD
            && newsp == 0
            && self.tls_inherited_without_clone_settls(clone_flags)
    }

    pub const fn exit_signal(&self, clone_flags: usize) -> usize {
        clone_flags & USER_CLONE_CSIGNAL_MASK
    }

    pub const fn clone_flags_without_csignal(&self, clone_flags: usize) -> usize {
        clone_flags & !USER_CLONE_CSIGNAL_MASK
    }

    pub const fn tls_inherited_without_clone_settls(&self, clone_flags: usize) -> bool {
        clone_flags & USER_CLONE_SETTLS == 0
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum UserInitPathRef {
    DefaultInit,
    EtcInit,
    BinInit,
    BinSh,
    RequestedInit,
}

impl UserInitPathRef {
    pub const fn index(self) -> usize {
        match self {
            Self::DefaultInit => 0,
            Self::EtcInit => 1,
            Self::BinInit => 2,
            Self::BinSh => 3,
            Self::RequestedInit => 4,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        match self {
            Self::DefaultInit => USER_INIT_PATH,
            Self::EtcInit => USER_ETC_INIT_PATH,
            Self::BinInit => USER_BIN_INIT_PATH,
            Self::BinSh => USER_BIN_SH_PATH,
            Self::RequestedInit => b"",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserInitAttemptStage {
    None,
    ValidatePath,
    ReadImage,
    PresetElf,
    SetupElf,
    UnsupportedCandidate,
}

impl UserInitAttemptStage {
    pub const fn index(self) -> usize {
        match self {
            Self::None => 0,
            Self::ValidatePath => 1,
            Self::ReadImage => 2,
            Self::PresetElf => 3,
            Self::SetupElf => 4,
            Self::UnsupportedCandidate => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ValidatePath => "validate_path",
            Self::ReadImage => "read_image",
            Self::PresetElf => "preset_elf",
            Self::SetupElf => "setup_elf",
            Self::UnsupportedCandidate => "unsupported_candidate",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserInitAttemptReason {
    None,
    InvalidPath,
    ReadFailed,
    ElfPresetFailed,
    ElfSetupFailed,
    CandidateUnsupported,
}

impl UserInitAttemptReason {
    pub const fn index(self) -> usize {
        match self {
            Self::None => 0,
            Self::InvalidPath => 1,
            Self::ReadFailed => 2,
            Self::ElfPresetFailed => 3,
            Self::ElfSetupFailed => 4,
            Self::CandidateUnsupported => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::InvalidPath => "invalid_path",
            Self::ReadFailed => "read_failed",
            Self::ElfPresetFailed => "elf_preset_failed",
            Self::ElfSetupFailed => "elf_setup_failed",
            Self::CandidateUnsupported => "candidate_unsupported",
        }
    }
}

#[derive(Clone, Copy)]
pub struct UserInitAttemptFailure {
    path: UserInitPathRef,
    path_bytes: [u8; USER_SELECTED_PATH_MAX],
    path_len: usize,
    path_actual_len: usize,
    path_truncated: bool,
    stage: UserInitAttemptStage,
    reason: UserInitAttemptReason,
    requested_terminal: bool,
    default_nonfatal: bool,
    elf_error: Option<ElfError>,
}

impl UserInitAttemptFailure {
    pub const fn empty() -> Self {
        Self {
            path: UserInitPathRef::DefaultInit,
            path_bytes: [0; USER_SELECTED_PATH_MAX],
            path_len: 0,
            path_actual_len: 0,
            path_truncated: false,
            stage: UserInitAttemptStage::None,
            reason: UserInitAttemptReason::None,
            requested_terminal: false,
            default_nonfatal: false,
            elf_error: None,
        }
    }

    pub fn new(
        path: UserInitPathRef,
        actual_path: &[u8],
        stage: UserInitAttemptStage,
        reason: UserInitAttemptReason,
        requested_terminal: bool,
        default_nonfatal: bool,
        elf_error: Option<ElfError>,
    ) -> Self {
        let mut path_bytes = [0u8; USER_SELECTED_PATH_MAX];
        let path_len = min_usize(actual_path.len(), USER_SELECTED_PATH_MAX);
        path_bytes[..path_len].copy_from_slice(&actual_path[..path_len]);
        Self {
            path,
            path_bytes,
            path_len,
            path_actual_len: actual_path.len(),
            path_truncated: actual_path.len() > USER_SELECTED_PATH_MAX,
            stage,
            reason,
            requested_terminal,
            default_nonfatal,
            elf_error,
        }
    }

    pub const fn path(&self) -> UserInitPathRef {
        self.path
    }

    pub fn path_bytes(&self) -> &[u8] {
        &self.path_bytes[..self.path_len]
    }

    pub const fn path_len(&self) -> usize {
        self.path_len
    }

    pub const fn path_actual_len(&self) -> usize {
        self.path_actual_len
    }

    pub const fn path_truncated(&self) -> bool {
        self.path_truncated
    }

    pub const fn stage(&self) -> UserInitAttemptStage {
        self.stage
    }

    pub const fn reason(&self) -> UserInitAttemptReason {
        self.reason
    }

    pub const fn requested_terminal(&self) -> bool {
        self.requested_terminal
    }

    pub const fn default_nonfatal(&self) -> bool {
        self.default_nonfatal
    }

    pub const fn elf_error(&self) -> Option<ElfError> {
        self.elf_error
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ElfObjectRole {
    MainExecutable,
    Interpreter,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ElfType {
    Exec,
    Dyn,
}

impl ElfType {
    const fn index(self) -> usize {
        match self {
            Self::Exec => 2,
            Self::Dyn => 3,
        }
    }
}

static USER_INIT_RUNTIME_ENTERED: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ElfError {
    InvalidState,
    ShortInput,
    BadMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedVersion,
    UnsupportedType,
    UnsupportedMachine,
    InvalidHeader,
    InvalidProgramHeader,
    TooManyLoadSegments,
    MissingLoadSegment,
    EntryOutsideExecutableSegment,
    TooManyMappings,
    TooManyMappingPages,
    MissingExecutableEntryMapping,
    InvalidStack,
    BackingAllocationFailed,
    PageTableAllocationFailed,
    PageTableInstallFailed,
    UserCopyOutOfRange,
}

impl ElfError {
    pub const fn index(self) -> usize {
        match self {
            Self::InvalidState => 1,
            Self::ShortInput => 2,
            Self::BadMagic => 3,
            Self::UnsupportedClass => 4,
            Self::UnsupportedEndian => 5,
            Self::UnsupportedVersion => 6,
            Self::UnsupportedType => 7,
            Self::UnsupportedMachine => 8,
            Self::InvalidHeader => 9,
            Self::InvalidProgramHeader => 10,
            Self::TooManyLoadSegments => 11,
            Self::MissingLoadSegment => 12,
            Self::EntryOutsideExecutableSegment => 13,
            Self::TooManyMappings => 14,
            Self::TooManyMappingPages => 15,
            Self::MissingExecutableEntryMapping => 16,
            Self::InvalidStack => 17,
            Self::BackingAllocationFailed => 18,
            Self::PageTableAllocationFailed => 19,
            Self::PageTableInstallFailed => 20,
            Self::UserCopyOutOfRange => 21,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidState => "invalid_state",
            Self::ShortInput => "short_input",
            Self::BadMagic => "bad_magic",
            Self::UnsupportedClass => "unsupported_class",
            Self::UnsupportedEndian => "unsupported_endian",
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnsupportedType => "unsupported_type",
            Self::UnsupportedMachine => "unsupported_machine",
            Self::InvalidHeader => "invalid_header",
            Self::InvalidProgramHeader => "invalid_program_header",
            Self::TooManyLoadSegments => "too_many_load_segments",
            Self::MissingLoadSegment => "missing_load_segment",
            Self::EntryOutsideExecutableSegment => "entry_outside_executable_segment",
            Self::TooManyMappings => "too_many_mappings",
            Self::TooManyMappingPages => "too_many_mapping_pages",
            Self::MissingExecutableEntryMapping => "missing_executable_entry_mapping",
            Self::InvalidStack => "invalid_stack",
            Self::BackingAllocationFailed => "backing_allocation_failed",
            Self::PageTableAllocationFailed => "page_table_allocation_failed",
            Self::PageTableInstallFailed => "page_table_install_failed",
            Self::UserCopyOutOfRange => "user_copy_out_of_range",
        }
    }
}

#[derive(Clone, Copy)]
pub struct ElfLoadSegment {
    offset: usize,
    vaddr: usize,
    filesz: usize,
    memsz: usize,
    flags: u32,
    align: usize,
}

#[allow(dead_code)]
impl ElfLoadSegment {
    const fn empty() -> Self {
        Self {
            offset: 0,
            vaddr: 0,
            filesz: 0,
            memsz: 0,
            flags: 0,
            align: 0,
        }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn vaddr(&self) -> usize {
        self.vaddr
    }

    pub const fn filesz(&self) -> usize {
        self.filesz
    }

    pub const fn memsz(&self) -> usize {
        self.memsz
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn readable(&self) -> bool {
        self.flags & ELF_PF_R != 0
    }

    pub const fn writable(&self) -> bool {
        self.flags & ELF_PF_W != 0
    }

    pub const fn executable(&self) -> bool {
        self.flags & ELF_PF_X != 0
    }

    pub const fn align(&self) -> usize {
        self.align
    }

    pub const fn file_end(&self) -> usize {
        self.offset + self.filesz
    }

    pub const fn vaddr_end(&self) -> usize {
        self.vaddr + self.memsz
    }

    const fn contains_vaddr(&self, addr: usize) -> bool {
        self.vaddr <= addr && addr < self.vaddr_end()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserMappingKind {
    Empty,
    ElfSegment,
    Stack,
    Heap,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserFaultAccess {
    Instruction,
    Load,
    Store,
    Unknown,
}

#[derive(Clone, Copy)]
pub struct UserFaultMappingDiagnostic {
    address: usize,
    access: UserFaultAccess,
    mapped: bool,
    kind: UserMappingKind,
    start: usize,
    end: usize,
    readable: bool,
    writable: bool,
    executable: bool,
    user_accessible: bool,
    permission_satisfied: bool,
}

impl UserFaultMappingDiagnostic {
    const fn unmapped(address: usize, access: UserFaultAccess) -> Self {
        Self {
            address,
            access,
            mapped: false,
            kind: UserMappingKind::Empty,
            start: 0,
            end: 0,
            readable: false,
            writable: false,
            executable: false,
            user_accessible: false,
            permission_satisfied: false,
        }
    }

    const fn covered(
        address: usize,
        access: UserFaultAccess,
        mapping: &UserMapping,
        start: usize,
        end: usize,
        permission_satisfied: bool,
    ) -> Self {
        Self {
            address,
            access,
            mapped: true,
            kind: mapping.kind(),
            start,
            end,
            readable: mapping.readable(),
            writable: mapping.writable(),
            executable: mapping.executable(),
            user_accessible: mapping.user_accessible(),
            permission_satisfied,
        }
    }

    pub const fn address(&self) -> usize {
        self.address
    }

    pub const fn access(&self) -> UserFaultAccess {
        self.access
    }

    pub const fn mapped(&self) -> bool {
        self.mapped
    }

    pub const fn kind(&self) -> UserMappingKind {
        self.kind
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn end(&self) -> usize {
        self.end
    }

    pub const fn readable(&self) -> bool {
        self.readable
    }

    pub const fn writable(&self) -> bool {
        self.writable
    }

    pub const fn executable(&self) -> bool {
        self.executable
    }

    pub const fn user_accessible(&self) -> bool {
        self.user_accessible
    }

    pub const fn permission_satisfied(&self) -> bool {
        self.permission_satisfied
    }
}

pub struct UserMapping {
    kind: UserMappingKind,
    vaddr: usize,
    memsz: usize,
    page_offset: usize,
    file_offset: usize,
    filesz: usize,
    readable: bool,
    writable: bool,
    executable: bool,
    user_accessible: bool,
    bss_zero_bytes: usize,
    backing_pages: [Option<PageRef>; MAX_MAPPING_BACKING_PAGES],
    backing_page_count: usize,
    file_bytes_copied: usize,
    bss_bytes_zeroed: usize,
    page_table_entry_bound: bool,
}

impl UserMapping {
    const fn empty() -> Self {
        Self {
            kind: UserMappingKind::Empty,
            vaddr: 0,
            memsz: 0,
            page_offset: 0,
            file_offset: 0,
            filesz: 0,
            readable: false,
            writable: false,
            executable: false,
            user_accessible: false,
            bss_zero_bytes: 0,
            backing_pages: [None; MAX_MAPPING_BACKING_PAGES],
            backing_page_count: 0,
            file_bytes_copied: 0,
            bss_bytes_zeroed: 0,
            page_table_entry_bound: false,
        }
    }

    fn reset_empty(&mut self) {
        self.kind = UserMappingKind::Empty;
        self.vaddr = 0;
        self.memsz = 0;
        self.page_offset = 0;
        self.file_offset = 0;
        self.filesz = 0;
        self.readable = false;
        self.writable = false;
        self.executable = false;
        self.user_accessible = false;
        self.bss_zero_bytes = 0;
        self.clear_backing_pages();
        self.backing_page_count = 0;
        self.file_bytes_copied = 0;
        self.bss_bytes_zeroed = 0;
        self.page_table_entry_bound = false;
    }

    fn clear_backing_pages(&mut self) {
        let mut index = 0usize;
        while index < MAX_MAPPING_BACKING_PAGES {
            self.backing_pages[index] = None;
            index += 1;
        }
    }

    fn init_segment(&mut self, segment: ElfLoadSegment) {
        self.reset_empty();
        self.kind = UserMappingKind::ElfSegment;
        self.vaddr = segment.vaddr;
        self.memsz = segment.memsz;
        self.page_offset = segment.vaddr % USER_PAGE_SIZE;
        self.file_offset = segment.offset;
        self.filesz = segment.filesz;
        self.readable = segment.readable();
        self.writable = segment.writable();
        self.executable = segment.executable();
        self.user_accessible = true;
        self.bss_zero_bytes = segment.memsz - segment.filesz;
    }

    fn init_stack(&mut self, stack: &UserStack) {
        self.reset_empty();
        self.kind = UserMappingKind::Stack;
        self.vaddr = stack.base;
        self.memsz = stack.size;
        self.readable = true;
        self.writable = true;
        self.user_accessible = true;
        self.bss_zero_bytes = stack.size;
        self.bss_bytes_zeroed = stack.size;
        self.page_table_entry_bound = stack.backing_pages_allocated;
        let mut index = 0usize;
        while index < stack.backing_page_count && index < MAX_MAPPING_BACKING_PAGES {
            self.backing_pages[index] = stack.backing_pages[index];
            index += 1;
        }
        self.backing_page_count = stack.backing_page_count;
    }

    fn init_heap(&mut self) {
        self.reset_empty();
        self.kind = UserMappingKind::Heap;
        self.vaddr = USER_HEAP_BASE;
        self.memsz = USER_HEAP_SIZE;
        self.readable = true;
        self.writable = true;
        self.user_accessible = true;
        self.bss_zero_bytes = USER_HEAP_SIZE;
        self.bss_bytes_zeroed = USER_HEAP_SIZE;
    }

    pub const fn kind(&self) -> UserMappingKind {
        self.kind
    }

    pub const fn vaddr(&self) -> usize {
        self.vaddr
    }

    pub const fn memsz(&self) -> usize {
        self.memsz
    }

    pub const fn page_offset(&self) -> usize {
        self.page_offset
    }

    pub const fn file_offset(&self) -> usize {
        self.file_offset
    }

    pub const fn filesz(&self) -> usize {
        self.filesz
    }

    pub const fn readable(&self) -> bool {
        self.readable
    }

    pub const fn writable(&self) -> bool {
        self.writable
    }

    pub const fn executable(&self) -> bool {
        self.executable
    }

    pub const fn user_accessible(&self) -> bool {
        self.user_accessible
    }

    pub const fn bss_zero_bytes(&self) -> usize {
        self.bss_zero_bytes
    }

    pub const fn backing_page_count(&self) -> usize {
        self.backing_page_count
    }

    pub const fn backing_page(&self, index: usize) -> Option<PageRef> {
        if index < self.backing_page_count {
            self.backing_pages[index]
        } else {
            None
        }
    }

    pub const fn file_bytes_copied(&self) -> usize {
        self.file_bytes_copied
    }

    pub const fn bss_bytes_zeroed(&self) -> usize {
        self.bss_bytes_zeroed
    }

    pub const fn page_table_entry_bound(&self) -> bool {
        self.page_table_entry_bound
    }

    pub const fn end_vaddr(&self) -> usize {
        self.vaddr + self.memsz
    }

    const fn contains_vaddr(&self, addr: usize) -> bool {
        self.vaddr <= addr && addr < self.end_vaddr()
    }
}

pub struct UserStack {
    lifecycle: Lifecycle,
    base: usize,
    top: usize,
    initial_sp: usize,
    arg0_ptr: usize,
    size: usize,
    backing_pages: [Option<PageRef>; MAX_STACK_PAGES],
    backing_page_count: usize,
    allocated: bool,
    fixed_size_bound: bool,
    mapped_into_address_space: bool,
    backing_pages_allocated: bool,
    zeroed: bool,
    initial_sp_bound: bool,
    minimal_arg_env_bound: bool,
}

#[allow(dead_code)]
impl UserStack {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            base: 0,
            top: 0,
            initial_sp: 0,
            arg0_ptr: 0,
            size: 0,
            backing_pages: [None; MAX_STACK_PAGES],
            backing_page_count: 0,
            allocated: false,
            fixed_size_bound: false,
            mapped_into_address_space: false,
            backing_pages_allocated: false,
            zeroed: false,
            initial_sp_bound: false,
            minimal_arg_env_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn base(&self) -> usize {
        self.base
    }

    pub const fn top(&self) -> usize {
        self.top
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn initial_sp(&self) -> usize {
        self.initial_sp
    }

    pub const fn arg0_ptr(&self) -> usize {
        self.arg0_ptr
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn backing_page_count(&self) -> usize {
        self.backing_page_count
    }

    pub const fn backing_page(&self, index: usize) -> Option<PageRef> {
        if index < self.backing_page_count {
            self.backing_pages[index]
        } else {
            None
        }
    }

    pub const fn fixed_size_bound(&self) -> bool {
        self.fixed_size_bound
    }

    pub const fn mapped_into_address_space(&self) -> bool {
        self.mapped_into_address_space
    }

    pub const fn backing_pages_allocated(&self) -> bool {
        self.backing_pages_allocated
    }

    pub const fn zeroed(&self) -> bool {
        self.zeroed
    }

    pub const fn initial_sp_bound(&self) -> bool {
        self.initial_sp_bound
    }

    pub const fn minimal_arg_env_bound(&self) -> bool {
        self.minimal_arg_env_bound
    }

    pub fn setup(
        &mut self,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        interpreter: Option<&ElfObject>,
        selected_path: &[u8],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || address_space.state() != State::Prepared
            || page_allocator.state() != State::Ready
            || elf.state() != State::Ready
            || interpreter.is_some_and(|interp| interp.state() != State::Ready)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.size = USER_STACK_SIZE;
        self.top = USER_STACK_TOP;
        self.base = USER_STACK_TOP - USER_STACK_SIZE;
        self.initial_sp = 0;
        self.arg0_ptr = 0;
        self.backing_pages = [None; MAX_STACK_PAGES];
        self.backing_page_count = 0;
        let mut index = 0usize;
        while index < MAX_STACK_PAGES {
            let Some(page) = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map)
            else {
                while self.backing_page_count > 0 {
                    self.backing_page_count -= 1;
                    if let Some(allocated) = self.backing_pages[self.backing_page_count] {
                        let _ = page_allocator.free_pages(allocated, 0, page_metadata_map);
                    }
                }
                return failed_condition(
                    LifecycleEvent::Setup,
                    self.lifecycle.state(),
                    State::Base,
                    State::Ready,
                );
            };
            let Some(linear) = page_metadata_map.page_address(page) else {
                let _ = page_allocator.free_pages(page, 0, page_metadata_map);
                return failed_condition(
                    LifecycleEvent::Setup,
                    self.lifecycle.state(),
                    State::Base,
                    State::Ready,
                );
            };
            unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };
            self.backing_pages[index] = Some(page);
            self.backing_page_count += 1;
            index += 1;
        }
        let Some(initial_sp) =
            self.write_initial_arg_env(elf, interpreter, selected_path, page_metadata_map)
        else {
            while self.backing_page_count > 0 {
                self.backing_page_count -= 1;
                if let Some(allocated) = self.backing_pages[self.backing_page_count] {
                    let _ = page_allocator.free_pages(allocated, 0, page_metadata_map);
                }
            }
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        self.initial_sp = initial_sp;
        self.allocated = true;
        self.fixed_size_bound = true;
        self.mapped_into_address_space = true;
        self.backing_pages_allocated = true;
        self.zeroed = true;
        self.initial_sp_bound = true;
        self.minimal_arg_env_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn write_initial_arg_env(
        &mut self,
        elf: &ElfObject,
        interpreter: Option<&ElfObject>,
        selected_path: &[u8],
        page_metadata_map: &PageMetadataMap,
    ) -> Option<usize> {
        let arg0 = selected_path;
        let arg0_len = arg0.len().checked_add(1)?;
        let arg0_ptr = align_down(self.top.checked_sub(arg0_len)?, 8);
        write_stack_bytes(self, page_metadata_map, arg0_ptr, arg0)?;
        write_stack_bytes(self, page_metadata_map, arg0_ptr + arg0.len(), &[0])?;

        let words_size = USER_INITIAL_STACK_WORDS.checked_mul(core::mem::size_of::<usize>())?;
        let initial_sp = align_down(arg0_ptr.checked_sub(words_size)?, 16);
        if initial_sp < self.base {
            return None;
        }

        write_stack_usize(self, page_metadata_map, initial_sp, 1)?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + core::mem::size_of::<usize>(),
            arg0_ptr,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 2 * core::mem::size_of::<usize>(),
            0,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 3 * core::mem::size_of::<usize>(),
            0,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 4 * core::mem::size_of::<usize>(),
            AT_PHDR,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 5 * core::mem::size_of::<usize>(),
            elf.phdr_vaddr(),
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 6 * core::mem::size_of::<usize>(),
            AT_PHENT,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 7 * core::mem::size_of::<usize>(),
            elf.phentsize(),
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 8 * core::mem::size_of::<usize>(),
            AT_PHNUM,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 9 * core::mem::size_of::<usize>(),
            elf.program_header_count(),
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 10 * core::mem::size_of::<usize>(),
            AT_ENTRY,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 11 * core::mem::size_of::<usize>(),
            elf.entry(),
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 12 * core::mem::size_of::<usize>(),
            AT_BASE,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 13 * core::mem::size_of::<usize>(),
            interpreter.map_or(0, ElfObject::load_bias),
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 14 * core::mem::size_of::<usize>(),
            AT_PAGESZ,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 15 * core::mem::size_of::<usize>(),
            USER_PAGE_SIZE,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 16 * core::mem::size_of::<usize>(),
            AT_NULL,
        )?;
        write_stack_usize(
            self,
            page_metadata_map,
            initial_sp + 17 * core::mem::size_of::<usize>(),
            0,
        )?;

        self.arg0_ptr = arg0_ptr;
        Some(initial_sp)
    }
}

fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

fn align_up_checked(value: usize, align: usize) -> Option<usize> {
    value
        .checked_add(align - 1)
        .map(|value| value & !(align - 1))
}

fn write_stack_usize(
    stack: &UserStack,
    page_metadata_map: &PageMetadataMap,
    user_addr: usize,
    value: usize,
) -> Option<()> {
    write_stack_bytes(stack, page_metadata_map, user_addr, &value.to_ne_bytes())
}

fn write_stack_bytes(
    stack: &UserStack,
    page_metadata_map: &PageMetadataMap,
    user_addr: usize,
    bytes: &[u8],
) -> Option<()> {
    if user_addr < stack.base
        || user_addr
            .checked_add(bytes.len())
            .filter(|end| *end <= stack.top)
            .is_none()
    {
        return None;
    }

    let mut written = 0usize;
    while written < bytes.len() {
        let stack_offset = user_addr - stack.base + written;
        let page_index = stack_offset / USER_PAGE_SIZE;
        let page_offset = stack_offset % USER_PAGE_SIZE;
        let chunk = min_usize(bytes.len() - written, USER_PAGE_SIZE - page_offset);
        let page = stack.backing_page(page_index)?;
        let linear = page_metadata_map.page_address(page)?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr().add(written),
                (linear + page_offset) as *mut u8,
                chunk,
            );
        }
        written += chunk;
    }
    Some(())
}

pub struct UserAddressSpace {
    lifecycle: Lifecycle,
    allocated: bool,
    first_instance: bool,
    bound_to_kernel_init_task: bool,
    low_half_private: bool,
    high_half_shares_swapper: bool,
    kernel_pages_u_disabled: bool,
    user_pages_u_enabled: bool,
    elf_load_plan_consumed: bool,
    segment_mappings_bound: bool,
    entry_mapping_executable: bool,
    bss_zero_plan_consumed: bool,
    backing_pages_allocated: bool,
    elf_file_bytes_copied: bool,
    bss_bytes_zeroed: bool,
    page_table_view_ready: bool,
    elf_segments_mapped: bool,
    stack_mapped: bool,
    heap_mapped: bool,
    elf_mapped: bool,
    elf_bss_zeroed: bool,
    runtime_ready: bool,
    real_page_table_allocated: bool,
    user_leaf_ptes_installed: bool,
    high_half_root_entries_shared: bool,
    satp_token_ready: bool,
    prepared_but_not_current: bool,
    satp_token: usize,
    user_leaf_pte_count: usize,
    page_table_root: Option<PageRef>,
    page_table_l1: Option<PageRef>,
    page_table_l0s: [UserL0TableSlot; MAX_USER_L0_TABLES],
    page_table_l0_count: usize,
    mappings: [UserMapping; MAX_USER_MAPPINGS],
    mapping_count: usize,
    segment_mapping_count: usize,
    stack_mapping_index: usize,
    heap_mapping_index: usize,
    heap_base: usize,
    heap_size: usize,
    heap_brk: usize,
    mmap_next: usize,
}

#[derive(Clone, Copy)]
struct UserL0TableSlot {
    vpn1: usize,
    page: Option<PageRef>,
}

impl UserL0TableSlot {
    const fn empty() -> Self {
        Self {
            vpn1: 0,
            page: None,
        }
    }
}

#[allow(dead_code)]
impl UserAddressSpace {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            first_instance: false,
            bound_to_kernel_init_task: false,
            low_half_private: false,
            high_half_shares_swapper: false,
            kernel_pages_u_disabled: false,
            user_pages_u_enabled: false,
            elf_load_plan_consumed: false,
            segment_mappings_bound: false,
            entry_mapping_executable: false,
            bss_zero_plan_consumed: false,
            backing_pages_allocated: false,
            elf_file_bytes_copied: false,
            bss_bytes_zeroed: false,
            page_table_view_ready: false,
            elf_segments_mapped: false,
            stack_mapped: false,
            heap_mapped: false,
            elf_mapped: false,
            elf_bss_zeroed: false,
            runtime_ready: false,
            real_page_table_allocated: false,
            user_leaf_ptes_installed: false,
            high_half_root_entries_shared: false,
            satp_token_ready: false,
            prepared_but_not_current: false,
            satp_token: 0,
            user_leaf_pte_count: 0,
            page_table_root: None,
            page_table_l1: None,
            page_table_l0s: [UserL0TableSlot::empty(); MAX_USER_L0_TABLES],
            page_table_l0_count: 0,
            mappings: [const { UserMapping::empty() }; MAX_USER_MAPPINGS],
            mapping_count: 0,
            segment_mapping_count: 0,
            stack_mapping_index: 0,
            heap_mapping_index: 0,
            heap_base: 0,
            heap_size: 0,
            heap_brk: 0,
            mmap_next: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn first_instance(&self) -> bool {
        self.first_instance
    }

    pub const fn bound_to_kernel_init_task(&self) -> bool {
        self.bound_to_kernel_init_task
    }

    pub const fn low_half_private(&self) -> bool {
        self.low_half_private
    }

    pub const fn high_half_shares_swapper(&self) -> bool {
        self.high_half_shares_swapper
    }

    pub const fn kernel_pages_u_disabled(&self) -> bool {
        self.kernel_pages_u_disabled
    }

    pub const fn user_pages_u_enabled(&self) -> bool {
        self.user_pages_u_enabled
    }

    pub const fn elf_load_plan_consumed(&self) -> bool {
        self.elf_load_plan_consumed
    }

    pub const fn segment_mappings_bound(&self) -> bool {
        self.segment_mappings_bound
    }

    pub const fn entry_mapping_executable(&self) -> bool {
        self.entry_mapping_executable
    }

    pub const fn bss_zero_plan_consumed(&self) -> bool {
        self.bss_zero_plan_consumed
    }

    pub const fn backing_pages_allocated(&self) -> bool {
        self.backing_pages_allocated
    }

    pub const fn elf_file_bytes_copied(&self) -> bool {
        self.elf_file_bytes_copied
    }

    pub const fn bss_bytes_zeroed(&self) -> bool {
        self.bss_bytes_zeroed
    }

    pub const fn page_table_view_ready(&self) -> bool {
        self.page_table_view_ready
    }

    pub const fn elf_segments_mapped(&self) -> bool {
        self.elf_segments_mapped
    }

    pub const fn stack_mapped(&self) -> bool {
        self.stack_mapped
    }

    pub const fn heap_mapped(&self) -> bool {
        self.heap_mapped
    }

    pub const fn elf_mapped(&self) -> bool {
        self.elf_mapped
    }

    pub const fn elf_bss_zeroed(&self) -> bool {
        self.elf_bss_zeroed
    }

    pub const fn runtime_ready(&self) -> bool {
        self.runtime_ready
    }

    pub const fn real_page_table_allocated(&self) -> bool {
        self.real_page_table_allocated
    }

    pub const fn user_leaf_ptes_installed(&self) -> bool {
        self.user_leaf_ptes_installed
    }

    pub const fn high_half_root_entries_shared(&self) -> bool {
        self.high_half_root_entries_shared
    }

    pub const fn satp_token_ready(&self) -> bool {
        self.satp_token_ready
    }

    pub const fn prepared_but_not_current(&self) -> bool {
        self.prepared_but_not_current
    }

    pub const fn satp_token(&self) -> usize {
        self.satp_token
    }

    pub const fn user_leaf_pte_count(&self) -> usize {
        self.user_leaf_pte_count
    }

    pub const fn page_table_l0_count(&self) -> usize {
        self.page_table_l0_count
    }

    pub const fn mapping_count(&self) -> usize {
        self.mapping_count
    }

    pub const fn segment_mapping_count(&self) -> usize {
        self.segment_mapping_count
    }

    pub const fn mapping(&self, index: usize) -> Option<&UserMapping> {
        if index < self.mapping_count {
            Some(&self.mappings[index])
        } else {
            None
        }
    }

    pub const fn stack_mapping(&self) -> Option<&UserMapping> {
        if self.stack_mapped && self.stack_mapping_index < self.mapping_count {
            Some(&self.mappings[self.stack_mapping_index])
        } else {
            None
        }
    }

    pub const fn heap_base(&self) -> usize {
        self.heap_base
    }

    pub const fn heap_size(&self) -> usize {
        self.heap_size
    }

    pub const fn heap_brk(&self) -> usize {
        self.heap_brk
    }

    pub fn preset(
        &mut self,
        swapper_vm: &SwapperVm,
        page_allocator: &PageAllocator,
        global_allocator: &KernelGlobalAllocator,
        kernel_init_task: &KernelInitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || swapper_vm.state() != State::Online
            || page_allocator.state() != State::Ready
            || global_allocator.state() != State::Ready
            || kernel_init_task.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.allocated = true;
        self.first_instance = true;
        self.bound_to_kernel_init_task = true;
        self.low_half_private = true;
        self.high_half_shares_swapper = true;
        self.kernel_pages_u_disabled = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(
        &mut self,
        elf: &ElfObject,
        interpreter: Option<&ElfObject>,
        stack: &UserStack,
        image: &[u8],
        interpreter_image: Option<&[u8]>,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Prepared
            || elf.state() != State::Ready
            || stack.state() != State::Ready
            || interpreter.is_some_and(|interp| interp.state() != State::Ready)
        {
            return Err(ElfError::InvalidState);
        }
        if interpreter.is_some() && interpreter_image.is_none() {
            return Err(ElfError::InvalidState);
        }
        if !stack.mapped_into_address_space() || stack.size() == 0 {
            return Err(ElfError::InvalidStack);
        }

        self.reset_mappings();
        self.mapping_count = 0;
        self.segment_mapping_count = 0;

        if let Err(error) = self.map_elf_segments(elf, image, page_allocator, page_metadata_map) {
            release_mappings(
                &mut self.mappings,
                self.mapping_count,
                page_allocator,
                page_metadata_map,
            );
            self.mapping_count = 0;
            self.segment_mapping_count = 0;
            return Err(error);
        }
        if let Some(interpreter) = interpreter {
            let Some(interpreter_image) = interpreter_image else {
                return Err(ElfError::InvalidState);
            };
            if let Err(error) = self.map_elf_segments(
                interpreter,
                interpreter_image,
                page_allocator,
                page_metadata_map,
            ) {
                release_mappings(
                    &mut self.mappings,
                    self.mapping_count,
                    page_allocator,
                    page_metadata_map,
                );
                self.mapping_count = 0;
                self.segment_mapping_count = 0;
                return Err(error);
            }
        }

        if !entry_mapping_is_executable(&self.mappings, self.mapping_count, elf.runtime_entry()) {
            return Err(ElfError::MissingExecutableEntryMapping);
        }

        if self.mapping_count >= MAX_USER_MAPPINGS {
            return Err(ElfError::TooManyMappings);
        }
        self.stack_mapping_index = self.mapping_count;
        self.mappings[self.mapping_count].init_stack(stack);
        self.mapping_count += 1;
        if self.mapping_count >= MAX_USER_MAPPINGS {
            return Err(ElfError::TooManyMappings);
        }
        self.heap_mapping_index = self.mapping_count;
        self.mappings[self.heap_mapping_index].init_heap();
        if let Err(error) = materialize_zero_mapping(
            &mut self.mappings[self.heap_mapping_index],
            page_allocator,
            page_metadata_map,
        ) {
            self.mappings[self.heap_mapping_index].reset_empty();
            release_mappings(
                &mut self.mappings,
                self.mapping_count,
                page_allocator,
                page_metadata_map,
            );
            self.mapping_count = 0;
            self.segment_mapping_count = 0;
            return Err(error);
        }
        self.mapping_count += 1;

        self.user_pages_u_enabled = true;
        self.elf_load_plan_consumed = true;
        self.segment_mappings_bound = self.segment_mapping_count
            == elf.load_segment_count() + interpreter.map_or(0, ElfObject::load_segment_count);
        self.entry_mapping_executable = true;
        self.bss_zero_plan_consumed =
            elf.bss_zero_plan_bound() && interpreter.is_none_or(ElfObject::bss_zero_plan_bound);
        self.backing_pages_allocated =
            mappings_have_backing_pages(&self.mappings, self.mapping_count);
        self.elf_file_bytes_copied =
            mapping_file_bytes_match(&self.mappings, self.segment_mapping_count);
        self.bss_bytes_zeroed = mapping_bss_bytes_match(&self.mappings, self.segment_mapping_count);
        self.page_table_view_ready =
            mappings_have_page_table_entries(&self.mappings, self.mapping_count);
        self.elf_segments_mapped = true;
        self.stack_mapped = true;
        self.heap_mapped = true;
        self.heap_base = USER_HEAP_BASE;
        self.heap_size = USER_HEAP_SIZE;
        self.heap_brk = USER_HEAP_BASE;
        self.mmap_next = align_up(USER_HEAP_BASE + USER_HEAP_SIZE / 2, USER_PAGE_SIZE);
        self.elf_mapped = true;
        self.elf_bss_zeroed = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| ElfError::InvalidState)
    }

    fn reset_mappings(&mut self) {
        let mut index = 0usize;
        while index < MAX_USER_MAPPINGS {
            self.mappings[index].reset_empty();
            index += 1;
        }
    }

    fn map_elf_segments(
        &mut self,
        elf: &ElfObject,
        image: &[u8],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), ElfError> {
        let mut index = 0usize;
        while index < elf.load_segment_count() {
            if self.mapping_count >= MAX_USER_MAPPINGS {
                return Err(ElfError::TooManyMappings);
            }
            let Some(segment) = elf.load_segment(index) else {
                return Err(ElfError::InvalidProgramHeader);
            };
            let mapping_index = self.mapping_count;
            self.mappings[mapping_index].init_segment(segment);
            if self.mappings[mapping_index].memsz() == 0
                || !self.mappings[mapping_index].user_accessible()
                || self.mappings[mapping_index]
                    .file_offset()
                    .checked_add(self.mappings[mapping_index].filesz())
                    .is_none()
                || self.mappings[mapping_index]
                    .vaddr()
                    .checked_add(self.mappings[mapping_index].memsz())
                    .is_none()
            {
                self.mappings[mapping_index].reset_empty();
                return Err(ElfError::InvalidProgramHeader);
            }
            if let Err(error) = materialize_mapping(
                &mut self.mappings[mapping_index],
                image,
                page_allocator,
                page_metadata_map,
            ) {
                self.mappings[mapping_index].reset_empty();
                return Err(error);
            }
            self.mapping_count += 1;
            self.segment_mapping_count += 1;
            index += 1;
        }
        Ok(())
    }

    pub fn user_brk(&mut self, requested: usize) -> usize {
        if self.lifecycle.state() != State::Online || !self.heap_mapped {
            return 0;
        }
        let heap_end = self.heap_base + self.heap_size / 2;
        if requested == 0 {
            return self.heap_brk;
        }
        let requested = align_up(requested, USER_PAGE_SIZE);
        if requested < self.heap_base || requested > heap_end {
            return self.heap_brk;
        }
        self.heap_brk = requested;
        self.heap_brk
    }

    pub fn user_mmap(
        &mut self,
        addr: usize,
        len: usize,
        prot: usize,
        flags: usize,
        _fd: usize,
        offset: usize,
    ) -> Result<usize, UserMmapError> {
        if self.lifecycle.state() != State::Online || !self.heap_mapped || len == 0 {
            return if len == 0 {
                Err(UserMmapError::Invalid)
            } else {
                Err(UserMmapError::NoMemory)
            };
        }
        const PROT_NONE: usize = 0x0;
        const MAP_PRIVATE: usize = 0x02;
        const MAP_FIXED: usize = 0x10;
        const MAP_ANONYMOUS: usize = 0x20;
        const SUPPORTED_FLAGS: usize = MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS;
        if offset % USER_PAGE_SIZE != 0 {
            return Err(UserMmapError::Invalid);
        }
        if flags & !SUPPORTED_FLAGS != 0 {
            return Err(UserMmapError::Invalid);
        }
        if flags & MAP_ANONYMOUS == 0 || flags & MAP_PRIVATE == 0 {
            return Err(UserMmapError::Invalid);
        }
        let len = align_up_checked(len, USER_PAGE_SIZE).ok_or(UserMmapError::NoMemory)?;
        let mmap_start = self.heap_base + self.heap_size / 2;
        let heap_end = self.heap_base + self.heap_size;
        if flags == (MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS) {
            if prot == PROT_NONE
                && addr % USER_PAGE_SIZE == 0
                && len == USER_PAGE_SIZE
                && addr >= self.heap_base
                && addr
                    .checked_add(len)
                    .filter(|end| *end <= heap_end)
                    .is_some()
            {
                return Ok(addr);
            }
            return Err(UserMmapError::NoMemory);
        }
        let base = if addr != 0 {
            align_down(addr, USER_PAGE_SIZE)
        } else {
            self.mmap_next
        };
        let end = base.checked_add(len).ok_or(UserMmapError::NoMemory)?;
        if base < mmap_start || end > heap_end {
            return Err(UserMmapError::NoMemory);
        }
        if addr == 0 {
            self.mmap_next = end;
        }
        Ok(base)
    }

    pub fn user_mprotect(&self, addr: usize, len: usize) -> bool {
        self.user_range_mapped(addr, len)
    }

    pub fn user_munmap(&self, addr: usize, len: usize) -> bool {
        self.user_range_mapped(addr, len)
    }

    pub fn user_range_mapped(&self, addr: usize, len: usize) -> bool {
        if self.lifecycle.state() != State::Online || len == 0 {
            return false;
        }
        let Some(end) = addr.checked_add(len) else {
            return false;
        };
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = &self.mappings[index];
            let Some(mapping_start) = mapping.vaddr().checked_sub(mapping.page_offset()) else {
                index += 1;
                continue;
            };
            let Some(mapping_len) = mapping.backing_page_count().checked_mul(USER_PAGE_SIZE) else {
                index += 1;
                continue;
            };
            let Some(mapping_end) = mapping_start.checked_add(mapping_len) else {
                index += 1;
                continue;
            };
            if mapping.kind() != UserMappingKind::Empty
                && addr >= mapping_start
                && end <= mapping_end
            {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn fault_mapping_diagnostic(
        &self,
        addr: usize,
        access: UserFaultAccess,
    ) -> UserFaultMappingDiagnostic {
        if self.lifecycle.state() != State::Online {
            return UserFaultMappingDiagnostic::unmapped(addr, access);
        }

        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = &self.mappings[index];
            let Some(mapping_start) = mapping.vaddr().checked_sub(mapping.page_offset()) else {
                index += 1;
                continue;
            };
            let Some(mapping_len) = mapping.backing_page_count().checked_mul(USER_PAGE_SIZE) else {
                index += 1;
                continue;
            };
            let Some(mapping_end) = mapping_start.checked_add(mapping_len) else {
                index += 1;
                continue;
            };
            if mapping.kind() != UserMappingKind::Empty
                && addr >= mapping_start
                && addr < mapping_end
            {
                let permission_satisfied = match access {
                    UserFaultAccess::Instruction => {
                        mapping.user_accessible() && mapping.executable()
                    }
                    UserFaultAccess::Load => mapping.user_accessible() && mapping.readable(),
                    UserFaultAccess::Store => mapping.user_accessible() && mapping.writable(),
                    UserFaultAccess::Unknown => mapping.user_accessible(),
                };
                return UserFaultMappingDiagnostic::covered(
                    addr,
                    access,
                    mapping,
                    mapping_start,
                    mapping_end,
                    permission_satisfied,
                );
            }
            index += 1;
        }

        UserFaultMappingDiagnostic::unmapped(addr, access)
    }

    pub fn enable(
        &mut self,
        trap_frame: &UserTrapFrame,
        swapper_vm: &SwapperVm,
        kernel_image: &KernelImage,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Ready
            || trap_frame.state() != State::Ready
            || swapper_vm.state() != State::Online
            || page_allocator.state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || !self.page_table_view_ready()
            || !self.entry_mapping_executable()
        {
            return Err(ElfError::InvalidState);
        }

        self.release_page_table_pages(page_allocator, page_metadata_map);
        if !self.allocate_base_page_tables(page_allocator, page_metadata_map) {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableAllocationFailed);
        }
        if !self.copy_swapper_high_half(kernel_image, page_metadata_map)
            || !self.install_all_user_leaf_ptes(page_allocator, page_metadata_map)
        {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableInstallFailed);
        }

        let Some(root) = self.page_table_root else {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableInstallFailed);
        };
        let root_phys = root.phys().value();
        if root_phys == 0 {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableInstallFailed);
        }

        self.satp_token = csr::SATP_MODE_SV39 | (root_phys >> 12);
        self.satp_token_ready = true;
        self.real_page_table_allocated = true;
        self.runtime_ready = true;
        self.prepared_but_not_current = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
            .map_err(|_| ElfError::InvalidState)
    }

    fn allocate_base_page_tables(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        let Some(root) = allocate_zeroed_page_table_page(page_allocator, page_metadata_map) else {
            return false;
        };
        let Some(l1) = allocate_zeroed_page_table_page(page_allocator, page_metadata_map) else {
            let _ = page_allocator.free_pages(root, 0, page_metadata_map);
            return false;
        };

        self.page_table_root = Some(root);
        self.page_table_l1 = Some(l1);
        self.page_table_l0s = [UserL0TableSlot::empty(); MAX_USER_L0_TABLES];
        self.page_table_l0_count = 0;
        self.user_leaf_pte_count = 0;
        true
    }

    fn copy_swapper_high_half(
        &mut self,
        kernel_image: &KernelImage,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        let Some(root_page) = self.page_table_root else {
            return false;
        };
        let Some(root_table) = page_table_page_mut(root_page, page_metadata_map) else {
            return false;
        };
        let copied = copy_high_half_root_entries(
            root_table,
            static_page_tables::swapper_pg_dir(kernel_image),
        );
        self.high_half_root_entries_shared = copied != 0;
        self.high_half_root_entries_shared
    }

    fn install_all_user_leaf_ptes(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        let mut index = 0usize;
        while index < self.mapping_count {
            if !self.install_mapping_ptes(index, page_allocator, page_metadata_map) {
                return false;
            }
            index += 1;
        }
        self.user_leaf_ptes_installed = self.user_leaf_pte_count != 0
            && self.user_leaf_pte_count
                == total_mapping_page_count(&self.mappings, self.mapping_count);
        self.user_leaf_ptes_installed
    }

    fn install_mapping_ptes(
        &mut self,
        mapping_index: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if mapping_index >= self.mapping_count {
            return false;
        }
        let kind = self.mappings[mapping_index].kind();
        let user_accessible = self.mappings[mapping_index].user_accessible();
        let backing_page_count = self.mappings[mapping_index].backing_page_count();
        let page_offset = self.mappings[mapping_index].page_offset();
        let Some(mut virt) = self.mappings[mapping_index]
            .vaddr()
            .checked_sub(page_offset)
        else {
            return false;
        };
        if kind == UserMappingKind::Empty || !user_accessible || backing_page_count == 0 {
            return false;
        }

        let readable = self.mappings[mapping_index].readable();
        let writable = self.mappings[mapping_index].writable();
        let executable = self.mappings[mapping_index].executable();
        let mut page_index = 0usize;
        while page_index < backing_page_count {
            let Some(page) = self.mappings[mapping_index].backing_page(page_index) else {
                return false;
            };
            if !self.install_user_leaf_pte(
                virt,
                page.phys().value(),
                readable,
                writable,
                executable,
                page_allocator,
                page_metadata_map,
            ) {
                return false;
            }
            let Some(next_virt) = virt.checked_add(USER_PAGE_SIZE) else {
                return false;
            };
            virt = next_virt;
            page_index += 1;
        }
        true
    }

    fn install_user_leaf_pte(
        &mut self,
        virt: usize,
        phys: usize,
        readable: bool,
        writable: bool,
        executable: bool,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if virt % USER_PAGE_SIZE != 0 || phys % USER_PAGE_SIZE != 0 {
            return false;
        }
        let (vpn2, vpn1, vpn0) = sv39_indices(virt);
        if vpn2 != 0 {
            return false;
        }
        let Some(root_page) = self.page_table_root else {
            return false;
        };
        let Some(l1_page) = self.page_table_l1 else {
            return false;
        };
        let Some(l0_page) = self.l0_page_for_vpn1(vpn1, page_allocator, page_metadata_map) else {
            return false;
        };
        let Some(root_table) = page_table_page_mut(root_page, page_metadata_map) else {
            return false;
        };
        if !root_table.set_entry(vpn2, table_pte_from_phys(l1_page.phys().value())) {
            return false;
        }
        let Some(l1_table) = page_table_page_mut(l1_page, page_metadata_map) else {
            return false;
        };
        if !l1_table.set_entry(vpn1, table_pte_from_phys(l0_page.phys().value())) {
            return false;
        }
        let Some(leaf) = user_leaf_pte_from_phys(phys, readable, writable, executable) else {
            return false;
        };
        let Some(l0_table) = page_table_page_mut(l0_page, page_metadata_map) else {
            return false;
        };
        if !l0_table.set_entry(vpn0, leaf) {
            return false;
        }
        self.user_leaf_pte_count += 1;
        true
    }

    fn l0_page_for_vpn1(
        &mut self,
        vpn1: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        let mut index = 0usize;
        while index < self.page_table_l0_count {
            let slot = self.page_table_l0s[index];
            if slot.page.is_some() && slot.vpn1 == vpn1 {
                return slot.page;
            }
            index += 1;
        }

        if self.page_table_l0_count >= MAX_USER_L0_TABLES {
            return None;
        }
        let page = allocate_zeroed_page_table_page(page_allocator, page_metadata_map)?;
        self.page_table_l0s[self.page_table_l0_count] = UserL0TableSlot {
            vpn1,
            page: Some(page),
        };
        self.page_table_l0_count += 1;
        Some(page)
    }

    fn release_page_table_pages(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) {
        let mut index = 0usize;
        while index < self.page_table_l0_count {
            if let Some(page) = self.page_table_l0s[index].page.take() {
                let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            }
            index += 1;
        }
        if let Some(page) = self.page_table_l1.take() {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        }
        if let Some(page) = self.page_table_root.take() {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        }
        self.page_table_l0s = [UserL0TableSlot::empty(); MAX_USER_L0_TABLES];
        self.page_table_l0_count = 0;
        self.user_leaf_pte_count = 0;
        self.real_page_table_allocated = false;
        self.user_leaf_ptes_installed = false;
        self.high_half_root_entries_shared = false;
        self.satp_token_ready = false;
        self.prepared_but_not_current = false;
        self.satp_token = 0;
        self.runtime_ready = false;
    }
}

fn allocate_zeroed_page_table_page(
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) -> Option<PageRef> {
    let page = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map)?;
    let Some(table) = page_table_page_mut(page, page_metadata_map) else {
        let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        return None;
    };
    table.clear();
    Some(page)
}

fn page_table_page_mut(
    page: PageRef,
    page_metadata_map: &PageMetadataMap,
) -> Option<&'static mut PageTablePage> {
    let linear = page_metadata_map.page_address(page)?;
    if !page_table_storage_ready(linear, USER_PAGE_SIZE) {
        return None;
    }
    Some(unsafe { &mut *(linear as *mut PageTablePage) })
}

fn total_mapping_page_count(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> usize {
    let mut pages = 0usize;
    let mut index = 0usize;
    while index < count {
        pages += mappings[index].backing_page_count();
        index += 1;
    }
    pages
}

pub const SSTATUS_SPP_USER_CLEAR: usize = 0;
pub const SSTATUS_SPIE_SET: usize = csr::SSTATUS_SPIE;
pub const SSTATUS_USER_FPU_INITIAL: usize = csr::SSTATUS_FS_INITIAL;
pub const USER_SSTATUS_INITIAL: usize =
    SSTATUS_SPIE_SET | SSTATUS_SPP_USER_CLEAR | SSTATUS_USER_FPU_INITIAL;

#[cfg(app_user_boot)]
#[repr(align(16))]
struct UserKernelTrapStack {
    bytes: [u8; USER_KERNEL_TRAP_STACK_SIZE],
}

#[cfg(app_user_boot)]
static mut USER_KERNEL_TRAP_STACK: UserKernelTrapStack = UserKernelTrapStack {
    bytes: [0; USER_KERNEL_TRAP_STACK_SIZE],
};
#[cfg(app_user_boot)]
static mut USER_BOOT_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];
#[cfg(app_user_boot)]
static mut USER_BOOT_INTERPRETER_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];

pub struct UserTrapFrame {
    lifecycle: Lifecycle,
    entry: usize,
    sp: usize,
    sstatus: usize,
    allocated: bool,
    entry_bound: bool,
    sp_bound: bool,
    sstatus_user_mode: bool,
    user_fpu_initial: bool,
    fpu_context_switch_deferred: bool,
    sret_ready: bool,
    address_space_bound: bool,
    prepared_but_not_entered: bool,
}

#[allow(dead_code)]
impl UserTrapFrame {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            entry: 0,
            sp: 0,
            sstatus: 0,
            allocated: false,
            entry_bound: false,
            sp_bound: false,
            sstatus_user_mode: false,
            user_fpu_initial: false,
            fpu_context_switch_deferred: false,
            sret_ready: false,
            address_space_bound: false,
            prepared_but_not_entered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn entry(&self) -> usize {
        self.entry
    }

    pub const fn sp(&self) -> usize {
        self.sp
    }

    pub const fn sstatus(&self) -> usize {
        self.sstatus
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn entry_bound(&self) -> bool {
        self.entry_bound
    }

    pub const fn sp_bound(&self) -> bool {
        self.sp_bound
    }

    pub const fn sstatus_user_mode(&self) -> bool {
        self.sstatus_user_mode
    }

    pub const fn user_fpu_initial(&self) -> bool {
        self.user_fpu_initial
    }

    pub const fn fpu_context_switch_deferred(&self) -> bool {
        self.fpu_context_switch_deferred
    }

    pub const fn sret_ready(&self) -> bool {
        self.sret_ready
    }

    pub const fn address_space_bound(&self) -> bool {
        self.address_space_bound
    }

    pub const fn prepared_but_not_entered(&self) -> bool {
        self.prepared_but_not_entered
    }

    pub fn setup(
        &mut self,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        stack: &UserStack,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || address_space.state() != State::Ready
            || elf.state() != State::Ready
            || stack.state() != State::Ready
            || !address_space.page_table_view_ready()
            || !address_space.entry_mapping_executable()
            || !address_space.bound_to_kernel_init_task()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.entry = elf.runtime_entry();
        self.sp = stack.initial_sp();
        self.sstatus = USER_SSTATUS_INITIAL;
        self.allocated = true;
        self.entry_bound = true;
        self.sp_bound = true;
        self.sstatus_user_mode = true;
        self.user_fpu_initial = true;
        self.fpu_context_switch_deferred = true;
        self.sret_ready = true;
        self.address_space_bound = true;
        self.prepared_but_not_entered = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct ElfObject {
    lifecycle: Lifecycle,
    role: ElfObjectRole,
    elf_type: ElfType,
    input_len: usize,
    entry: usize,
    runtime_entry: usize,
    load_bias: usize,
    phdr_vaddr: usize,
    phentsize: usize,
    program_header_count: usize,
    load_segment_count: usize,
    load_segments: [ElfLoadSegment; MAX_LOAD_SEGMENTS],
    interpreter_path: [u8; ELF_INTERP_PATH_MAX],
    interpreter_path_len: usize,
    input_bound: bool,
    input_from_vfs: bool,
    magic_valid: bool,
    class_elf64: bool,
    little_endian: bool,
    machine_riscv: bool,
    type_supported: bool,
    static_executable: bool,
    dynamic_executable: bool,
    interpreter_required: bool,
    interpreter_path_bound: bool,
    et_dyn_pie_main_supported: bool,
    main_pie_load_bias_bound: bool,
    et_dyn_interpreter_supported: bool,
    et_dyn_loader_without_interp_deferred: bool,
    runtime_entry_bound: bool,
    auxv_exec_fields_bound: bool,
    no_separate_loader: bool,
    program_headers_parsed: bool,
    pt_load_segments_bound: bool,
    segment_permissions_bound: bool,
    load_plan_bound: bool,
    entry_in_executable_segment: bool,
    init_content_observed: bool,
    bss_zero_plan_bound: bool,
    entry_bound: bool,
    load_merged_into_setup: bool,
    user_entry_ready: bool,
}

#[allow(dead_code)]
impl ElfObject {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            role: ElfObjectRole::MainExecutable,
            elf_type: ElfType::Exec,
            input_len: 0,
            entry: 0,
            runtime_entry: 0,
            load_bias: 0,
            phdr_vaddr: 0,
            phentsize: 0,
            program_header_count: 0,
            load_segment_count: 0,
            load_segments: [ElfLoadSegment::empty(); MAX_LOAD_SEGMENTS],
            interpreter_path: [0; ELF_INTERP_PATH_MAX],
            interpreter_path_len: 0,
            input_bound: false,
            input_from_vfs: false,
            magic_valid: false,
            class_elf64: false,
            little_endian: false,
            machine_riscv: false,
            type_supported: false,
            static_executable: false,
            dynamic_executable: false,
            interpreter_required: false,
            interpreter_path_bound: false,
            et_dyn_pie_main_supported: false,
            main_pie_load_bias_bound: false,
            et_dyn_interpreter_supported: false,
            et_dyn_loader_without_interp_deferred: false,
            runtime_entry_bound: false,
            auxv_exec_fields_bound: false,
            no_separate_loader: false,
            program_headers_parsed: false,
            pt_load_segments_bound: false,
            segment_permissions_bound: false,
            load_plan_bound: false,
            entry_in_executable_segment: false,
            init_content_observed: false,
            bss_zero_plan_bound: false,
            entry_bound: false,
            load_merged_into_setup: false,
            user_entry_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn input_len(&self) -> usize {
        self.input_len
    }

    pub const fn role(&self) -> ElfObjectRole {
        self.role
    }

    pub const fn elf_type_index(&self) -> usize {
        self.elf_type.index()
    }

    pub const fn entry(&self) -> usize {
        self.entry
    }

    pub const fn runtime_entry(&self) -> usize {
        self.runtime_entry
    }

    pub const fn load_bias(&self) -> usize {
        self.load_bias
    }

    pub const fn phdr_vaddr(&self) -> usize {
        self.phdr_vaddr
    }

    pub const fn phentsize(&self) -> usize {
        self.phentsize
    }

    pub const fn program_header_count(&self) -> usize {
        self.program_header_count
    }

    pub const fn load_segment_count(&self) -> usize {
        self.load_segment_count
    }

    pub const fn load_segment(&self, index: usize) -> Option<ElfLoadSegment> {
        if index < self.load_segment_count {
            Some(self.load_segments[index])
        } else {
            None
        }
    }

    pub const fn input_bound(&self) -> bool {
        self.input_bound
    }

    pub const fn input_from_vfs(&self) -> bool {
        self.input_from_vfs
    }

    pub const fn magic_valid(&self) -> bool {
        self.magic_valid
    }

    pub const fn class_elf64(&self) -> bool {
        self.class_elf64
    }

    pub const fn little_endian(&self) -> bool {
        self.little_endian
    }

    pub const fn machine_riscv(&self) -> bool {
        self.machine_riscv
    }

    pub const fn type_supported(&self) -> bool {
        self.type_supported
    }

    pub const fn static_executable(&self) -> bool {
        self.static_executable
    }

    pub const fn dynamic_executable(&self) -> bool {
        self.dynamic_executable
    }

    pub const fn interpreter_required(&self) -> bool {
        self.interpreter_required
    }

    pub fn interpreter_path(&self) -> Option<&[u8]> {
        if self.interpreter_path_bound {
            Some(&self.interpreter_path[..self.interpreter_path_len])
        } else {
            None
        }
    }

    pub const fn interpreter_path_bound(&self) -> bool {
        self.interpreter_path_bound
    }

    pub const fn et_dyn_pie_main_supported(&self) -> bool {
        self.et_dyn_pie_main_supported
    }

    pub const fn main_pie_load_bias_bound(&self) -> bool {
        self.main_pie_load_bias_bound
    }

    pub const fn et_dyn_interpreter_supported(&self) -> bool {
        self.et_dyn_interpreter_supported
    }

    pub const fn et_dyn_loader_without_interp_deferred(&self) -> bool {
        self.et_dyn_loader_without_interp_deferred
    }

    pub const fn runtime_entry_bound(&self) -> bool {
        self.runtime_entry_bound
    }

    pub const fn auxv_exec_fields_bound(&self) -> bool {
        self.auxv_exec_fields_bound
    }

    pub const fn no_separate_loader(&self) -> bool {
        self.no_separate_loader
    }

    pub const fn program_headers_parsed(&self) -> bool {
        self.program_headers_parsed
    }

    pub const fn pt_load_segments_bound(&self) -> bool {
        self.pt_load_segments_bound
    }

    pub const fn segment_permissions_bound(&self) -> bool {
        self.segment_permissions_bound
    }

    pub const fn load_plan_bound(&self) -> bool {
        self.load_plan_bound
    }

    pub const fn entry_in_executable_segment(&self) -> bool {
        self.entry_in_executable_segment
    }

    pub const fn init_content_observed(&self) -> bool {
        self.init_content_observed
    }

    pub const fn bss_zero_plan_bound(&self) -> bool {
        self.bss_zero_plan_bound
    }

    pub const fn entry_bound(&self) -> bool {
        self.entry_bound
    }

    pub const fn load_merged_into_setup(&self) -> bool {
        self.load_merged_into_setup
    }

    pub const fn user_entry_ready(&self) -> bool {
        self.user_entry_ready
    }

    pub fn preset_from_vfs(&mut self, input: &[u8]) -> Result<(), ElfError> {
        let header = parse_header(input)?;
        let load_bias = main_executable_load_bias(&header)?;
        self.preset_with_header(input, header, ElfObjectRole::MainExecutable, load_bias)
    }

    pub fn preset_interpreter_from_vfs(&mut self, input: &[u8]) -> Result<(), ElfError> {
        self.preset_with_role(
            input,
            ElfObjectRole::Interpreter,
            USER_INTERPRETER_LOAD_BIAS,
        )
    }

    fn preset_with_role(
        &mut self,
        input: &[u8],
        role: ElfObjectRole,
        load_bias: usize,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Base {
            return Err(ElfError::InvalidState);
        }
        let header = parse_header(input)?;
        self.preset_with_header(input, header, role, load_bias)
    }

    fn preset_with_header(
        &mut self,
        input: &[u8],
        header: ElfHeader,
        role: ElfObjectRole,
        load_bias: usize,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Base {
            return Err(ElfError::InvalidState);
        }
        if !elf_type_allowed_for_role(header.elf_type, role) {
            return Err(ElfError::UnsupportedType);
        }

        self.role = role;
        self.elf_type = header.elf_type;
        self.load_bias = load_bias;
        self.input_len = input.len();
        self.input_bound = true;
        self.input_from_vfs = true;
        self.magic_valid = true;
        self.class_elf64 = true;
        self.little_endian = true;
        self.machine_riscv = true;
        self.type_supported = true;
        self.static_executable = role == ElfObjectRole::MainExecutable
            && header.elf_type == ElfType::Exec
            && !header.interpreter_required;
        self.dynamic_executable = role == ElfObjectRole::MainExecutable
            && (header.elf_type == ElfType::Exec || header.elf_type == ElfType::Dyn)
            && header.interpreter_required;
        self.interpreter_required =
            role == ElfObjectRole::MainExecutable && header.interpreter_required;
        self.et_dyn_pie_main_supported = role == ElfObjectRole::MainExecutable
            && header.elf_type == ElfType::Dyn
            && header.interpreter_required;
        self.main_pie_load_bias_bound =
            self.et_dyn_pie_main_supported && self.load_bias == USER_MAIN_PIE_LOAD_BIAS;
        self.et_dyn_interpreter_supported =
            role == ElfObjectRole::Interpreter && header.elf_type == ElfType::Dyn;
        self.et_dyn_loader_without_interp_deferred = role == ElfObjectRole::MainExecutable
            && header.elf_type == ElfType::Dyn
            && !header.interpreter_required;
        self.no_separate_loader = !self.interpreter_required;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
            .map_err(|_| ElfError::InvalidState)
    }

    pub fn setup(&mut self, input: &[u8]) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Prepared {
            return Err(ElfError::InvalidState);
        }

        let header = parse_header(input)?;
        if !elf_type_allowed_for_role(header.elf_type, self.role) {
            return Err(ElfError::UnsupportedType);
        }
        if self.role == ElfObjectRole::MainExecutable
            && main_executable_load_bias(&header)? != self.load_bias
        {
            return Err(ElfError::InvalidState);
        }
        let parsed = parse_load_segments(input, &header, self.load_bias)?;
        if parsed.load_segment_count == 0 {
            return Err(ElfError::MissingLoadSegment);
        }
        let entry = self
            .load_bias
            .checked_add(header.entry)
            .ok_or(ElfError::InvalidHeader)?;
        if !entry_in_executable_segment(entry, &parsed.load_segments, parsed.load_segment_count) {
            return Err(ElfError::EntryOutsideExecutableSegment);
        }
        if self.role == ElfObjectRole::MainExecutable && header.interpreter_required {
            self.bind_interpreter_path(input, &header)?;
        }

        self.entry = entry;
        self.runtime_entry = entry;
        self.phdr_vaddr = phdr_vaddr(&header, &parsed, self.load_bias)?;
        self.phentsize = header.phentsize;
        self.program_header_count = header.phnum;
        self.load_segment_count = parsed.load_segment_count;
        self.load_segments = parsed.load_segments;
        self.program_headers_parsed = true;
        self.pt_load_segments_bound = true;
        self.segment_permissions_bound = parsed.segment_permissions_bound;
        self.load_plan_bound = true;
        self.entry_in_executable_segment = true;
        self.init_content_observed = self.role != ElfObjectRole::MainExecutable
            || loadable_content_contains(input, &parsed, USER_INIT_EXPECTED_MESSAGE);
        self.bss_zero_plan_bound = true;
        self.entry_bound = true;
        self.runtime_entry_bound = true;
        self.auxv_exec_fields_bound = self.role == ElfObjectRole::MainExecutable;
        self.load_merged_into_setup = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| ElfError::InvalidState)
    }

    fn bind_interpreter_path(&mut self, input: &[u8], header: &ElfHeader) -> Result<(), ElfError> {
        let Some((offset, len)) = header.interpreter else {
            return Err(ElfError::InvalidProgramHeader);
        };
        if len == 0 || len > ELF_INTERP_PATH_MAX {
            return Err(ElfError::InvalidProgramHeader);
        }
        let path = input
            .get(offset..offset + len)
            .ok_or(ElfError::InvalidProgramHeader)?;
        let path_len = if path[len - 1] == 0 { len - 1 } else { len };
        if path_len == 0 || path_len > ELF_INTERP_PATH_MAX {
            return Err(ElfError::InvalidProgramHeader);
        }
        self.interpreter_path = [0; ELF_INTERP_PATH_MAX];
        self.interpreter_path[..path_len].copy_from_slice(&path[..path_len]);
        self.interpreter_path_len = path_len;
        self.interpreter_path_bound = true;
        Ok(())
    }

    pub fn bind_runtime_interpreter(&mut self, interpreter: &ElfObject) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Ready
            || self.role != ElfObjectRole::MainExecutable
            || !self.interpreter_required
            || interpreter.state() != State::Ready
            || interpreter.role() != ElfObjectRole::Interpreter
        {
            return Err(ElfError::InvalidState);
        }
        self.runtime_entry = interpreter.entry();
        self.runtime_entry_bound = true;
        Ok(())
    }

    pub fn load_segments_fit_direct_read(&self, max_size: usize) -> bool {
        self.input_len <= max_size
    }

    pub fn enable(
        &mut self,
        address_space: &UserAddressSpace,
        stack: &UserStack,
        trap_frame: &UserTrapFrame,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || address_space.state() != State::Ready
            || stack.state() != State::Ready
            || trap_frame.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.user_entry_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}

pub struct UserInitProcess {
    lifecycle: Lifecycle,
    reuses_kernel_init_task: bool,
    pid1_preserved: bool,
    exec_identity_handoff: bool,
    no_new_task_struct: bool,
    kernel_init_not_destroyed: bool,
    path: UserInitPathRef,
    path_bound: bool,
    address_space_bound: bool,
    fs_struct_inherited: bool,
    files_struct_inherited: bool,
    trap_frame_bound: bool,
    syscall_context_bound: bool,
    kernel_init_execve_to_user_init: bool,
    kernel_init_pid1_identity_preserved: bool,
    kernel_init_user_mm_attached: bool,
    kernel_init_user_trap_frame_attached: bool,
    user_entry_ready: bool,
    trap_return_bound: bool,
    trap_return_context_used: bool,
    trap_return_sfence_vma_after_satp: bool,
    trap_return_sret_handoff: bool,
    syscall_dispatch_bound: bool,
    syscall_arguments_extracted: bool,
    runtime_entered: bool,
    credentials_inherited: bool,
    root_credentials_bound: bool,
    credentials_capability_model_deferred: bool,
    uid: usize,
    gid: usize,
    euid: usize,
    egid: usize,
    suid: usize,
    sgid: usize,
    fsuid: usize,
    fsgid: usize,
    pid_read_observed: bool,
    ppid_zero_first_slice: bool,
    ppid_read_observed: bool,
    session_leader_first_slice: bool,
    process_group_leader_first_slice: bool,
    process_group: usize,
    process_group_read_observed: bool,
    process_group_set_observed: bool,
    controlling_tty_bound: bool,
    foreground_pgrp_bound: bool,
    foreground_pgrp: usize,
    foreground_pgrp_read_observed: bool,
    foreground_pgrp_set_observed: bool,
    uid_read_observed: bool,
    euid_read_observed: bool,
    gid_read_observed: bool,
    egid_read_observed: bool,
    resuid_read_observed: bool,
    resgid_read_observed: bool,
    uid_set_observed: bool,
    gid_set_observed: bool,
    signal_state_inherited: bool,
    signal_runtime_bound: bool,
    thread_signal_state_bound: bool,
    process_signal_state_deferred: bool,
    signal_action_table_bound: bool,
    signal_action_table_layout_bound: bool,
    blocked_signal_mask_bound: bool,
    signal_delivery_deferred: bool,
    blocked_signal_mask: usize,
    signal_actions: [UserSignalAction; USER_SIGNAL_COUNT],
    rt_sigprocmask_observed: bool,
    rt_sigaction_observed: bool,
    clear_child_tid_bound: bool,
    clear_child_tid: usize,
    root_cwd_first_slice: bool,
    getcwd_observed: bool,
}

pub struct UserChildProcess {
    lifecycle: Lifecycle,
    prepared: bool,
    task_entry: TaskEntry,
    task_entry_bound: bool,
    pid: usize,
    parent_pid: usize,
    tgid: usize,
    exit_signal: usize,
    task_struct_allocated: bool,
    pid_allocated: bool,
    thread_context_ready: bool,
    sched_entity_ready: bool,
    task_state_new: bool,
    files_struct_copied: bool,
    fs_struct_copied: bool,
    credentials_copied: bool,
    signal_state_copied: bool,
    user_address_space_snapshot: bool,
    user_stack_snapshot: [u8; USER_STACK_SIZE],
    user_stack_snapshot_len: usize,
    user_stack_snapshot_copied: bool,
    user_stack_snapshot_restored: bool,
    trap_frame_copied: bool,
    trap_frame_child_return_zero: bool,
    tls_inherited: bool,
    child_trap_frame: Option<TrapFrame>,
    enqueued: bool,
    wait4_parent_wait_observed: bool,
    child_continuation_taken: bool,
}

#[allow(dead_code)]
impl UserChildProcess {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            prepared: false,
            task_entry: TaskEntry::None,
            task_entry_bound: false,
            pid: 0,
            parent_pid: 0,
            tgid: 0,
            exit_signal: 0,
            task_struct_allocated: false,
            pid_allocated: false,
            thread_context_ready: false,
            sched_entity_ready: false,
            task_state_new: false,
            files_struct_copied: false,
            fs_struct_copied: false,
            credentials_copied: false,
            signal_state_copied: false,
            user_address_space_snapshot: false,
            user_stack_snapshot: [0; USER_STACK_SIZE],
            user_stack_snapshot_len: 0,
            user_stack_snapshot_copied: false,
            user_stack_snapshot_restored: false,
            trap_frame_copied: false,
            trap_frame_child_return_zero: false,
            tls_inherited: false,
            child_trap_frame: None,
            enqueued: false,
            wait4_parent_wait_observed: false,
            child_continuation_taken: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn prepared(&self) -> bool {
        self.prepared
    }

    pub const fn task_entry(&self) -> TaskEntry {
        self.task_entry
    }

    pub const fn pid(&self) -> usize {
        self.pid
    }

    pub const fn parent_pid(&self) -> usize {
        self.parent_pid
    }

    pub const fn tgid(&self) -> usize {
        self.tgid
    }

    pub const fn exit_signal(&self) -> usize {
        self.exit_signal
    }

    pub const fn trap_frame_child_return_zero(&self) -> bool {
        self.trap_frame_child_return_zero
    }

    pub const fn tls_inherited(&self) -> bool {
        self.tls_inherited
    }

    pub const fn enqueued(&self) -> bool {
        self.enqueued
    }

    pub const fn wait4_parent_wait_observed(&self) -> bool {
        self.wait4_parent_wait_observed
    }

    pub const fn child_continuation_taken(&self) -> bool {
        self.child_continuation_taken
    }

    pub const fn user_stack_snapshot_copied(&self) -> bool {
        self.user_stack_snapshot_copied
    }

    pub const fn user_stack_snapshot_restored(&self) -> bool {
        self.user_stack_snapshot_restored
    }

    pub fn preset(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.prepared = true;
        self.task_entry = TaskEntry::UserChild;
        self.task_entry_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_plain_fork_from_parent(
        &mut self,
        parent: &UserInitProcess,
        boundaries: &UserCloneDeferredBoundaries,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        fs_struct: &FsStruct,
        files_struct: &FilesStruct,
        page_metadata_map: &PageMetadataMap,
        current_frame: &TrapFrame,
        clone_flags: usize,
        newsp: usize,
        task_struct_allocated: bool,
        thread_context_ready: bool,
        sched_entity_ready: bool,
        task_state_new: bool,
    ) -> Option<usize> {
        if self.lifecycle.state() != State::Prepared
            || !self.prepared
            || self.task_entry != TaskEntry::UserChild
            || parent.state() != State::Online
            || !parent.pid1_preserved()
            || boundaries.state() != State::Ready
            || !boundaries.accepts_plain_fork_first_slice(clone_flags, newsp)
            || address_space.state() != State::Online
            || trap_frame.state() != State::Ready
            || fs_struct.state() != State::Ready
            || files_struct.state() != State::Ready
            || !task_struct_allocated
            || !thread_context_ready
            || !sched_entity_ready
            || !task_state_new
        {
            return None;
        }

        let mut child_frame = *current_frame;
        child_frame.set_reg(10, 0);
        child_frame.sepc = child_frame.sepc.wrapping_add(4);
        let stack_snapshot_len = copy_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &mut self.user_stack_snapshot,
        )?;

        self.pid = USER_CHILD_PID;
        self.parent_pid = super::rest_init::KERNEL_INIT_PID;
        self.tgid = USER_CHILD_PID;
        self.exit_signal = boundaries.exit_signal(clone_flags);
        self.task_struct_allocated = task_struct_allocated;
        self.pid_allocated = true;
        self.thread_context_ready = thread_context_ready;
        self.sched_entity_ready = sched_entity_ready;
        self.task_state_new = task_state_new;
        self.files_struct_copied = true;
        self.fs_struct_copied = true;
        self.credentials_copied = parent.credentials_inherited();
        self.signal_state_copied = parent.signal_state_inherited();
        self.user_address_space_snapshot = true;
        self.user_stack_snapshot_len = stack_snapshot_len;
        self.user_stack_snapshot_copied = true;
        self.user_stack_snapshot_restored = false;
        self.trap_frame_copied = true;
        self.trap_frame_child_return_zero = child_frame.reg(10) == 0;
        self.tls_inherited = boundaries.tls_inherited_without_clone_settls(clone_flags);
        self.child_trap_frame = Some(child_frame);

        if self
            .lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .is_err()
        {
            return None;
        }
        Some(self.pid)
    }

    pub fn mark_enqueued(&mut self) -> bool {
        if self.lifecycle.state() != State::Ready || self.pid != USER_CHILD_PID {
            return false;
        }
        self.enqueued = true;
        true
    }

    pub fn wait4_yield_to_child_continuation(
        &mut self,
        parent: &UserInitProcess,
        address_space: &UserAddressSpace,
        page_metadata_map: &PageMetadataMap,
        upid: usize,
        options: usize,
        rusage: usize,
    ) -> Option<TrapFrame> {
        if self.lifecycle.state() != State::Ready
            || !self.enqueued
            || self.child_continuation_taken
            || self.pid != USER_CHILD_PID
            || self.parent_pid != super::rest_init::KERNEL_INIT_PID
            || self.tgid != USER_CHILD_PID
            || self.exit_signal != USER_CLONE_SIGCHLD
            || !self.task_struct_allocated
            || !self.pid_allocated
            || !self.thread_context_ready
            || !self.sched_entity_ready
            || !self.task_state_new
            || !self.files_struct_copied
            || !self.fs_struct_copied
            || !self.credentials_copied
            || !self.signal_state_copied
            || !self.user_address_space_snapshot
            || !self.user_stack_snapshot_copied
            || self.user_stack_snapshot_len == 0
            || !self.trap_frame_copied
            || !self.trap_frame_child_return_zero
            || parent.state() != State::Online
            || address_space.state() != State::Online
            || !parent.pid1_preserved()
            || upid != USER_WAIT4_ALL_CHILDREN
            || options != USER_WAIT4_WUNTRACED
            || rusage != 0
        {
            return None;
        }

        if !restore_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &self.user_stack_snapshot,
            self.user_stack_snapshot_len,
        ) {
            return None;
        }

        let child_frame = self.child_trap_frame?;
        self.wait4_parent_wait_observed = true;
        self.user_stack_snapshot_restored = true;
        self.child_continuation_taken = true;
        Some(child_frame)
    }
}

fn copy_user_stack_snapshot(
    address_space: &UserAddressSpace,
    page_metadata_map: &PageMetadataMap,
    output: &mut [u8; USER_STACK_SIZE],
) -> Option<usize> {
    let mapping = address_space.stack_mapping()?;
    if mapping.kind() != UserMappingKind::Stack || mapping.memsz() == 0 {
        return None;
    }
    let page_bytes = mapping
        .backing_page_count()
        .checked_mul(USER_PAGE_SIZE)
        .filter(|bytes| *bytes >= mapping.memsz())?;
    let snapshot_len = min_usize(mapping.memsz(), page_bytes);
    if snapshot_len == 0 || snapshot_len > output.len() {
        return None;
    }
    if !copy_stack_mapping_to_buffer(mapping, page_metadata_map, &mut output[..snapshot_len]) {
        return None;
    }
    Some(snapshot_len)
}

fn restore_user_stack_snapshot(
    address_space: &UserAddressSpace,
    page_metadata_map: &PageMetadataMap,
    input: &[u8; USER_STACK_SIZE],
    len: usize,
) -> bool {
    let Some(mapping) = address_space.stack_mapping() else {
        return false;
    };
    if mapping.kind() != UserMappingKind::Stack || len == 0 || len > input.len() {
        return false;
    }
    let Some(page_bytes) = mapping
        .backing_page_count()
        .checked_mul(USER_PAGE_SIZE)
        .filter(|bytes| *bytes >= len)
    else {
        return false;
    };
    if page_bytes < mapping.memsz() || len > mapping.memsz() {
        return false;
    }
    copy_buffer_to_stack_mapping(mapping, page_metadata_map, &input[..len])
}

fn copy_stack_mapping_to_buffer(
    mapping: &UserMapping,
    page_metadata_map: &PageMetadataMap,
    output: &mut [u8],
) -> bool {
    let mut copied = 0usize;
    let mut page_index = 0usize;
    while copied < output.len() {
        let Some(page) = mapping.backing_page(page_index) else {
            return false;
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            return false;
        };
        let len = min_usize(USER_PAGE_SIZE, output.len() - copied);
        unsafe {
            core::ptr::copy_nonoverlapping(
                linear as *const u8,
                output.as_mut_ptr().add(copied),
                len,
            );
        }
        copied += len;
        page_index += 1;
    }
    true
}

fn copy_buffer_to_stack_mapping(
    mapping: &UserMapping,
    page_metadata_map: &PageMetadataMap,
    input: &[u8],
) -> bool {
    let mut copied = 0usize;
    let mut page_index = 0usize;
    while copied < input.len() {
        let Some(page) = mapping.backing_page(page_index) else {
            return false;
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            return false;
        };
        let len = min_usize(USER_PAGE_SIZE, input.len() - copied);
        unsafe {
            core::ptr::copy_nonoverlapping(input.as_ptr().add(copied), linear as *mut u8, len);
        }
        copied += len;
        page_index += 1;
    }
    true
}

#[allow(dead_code)]
impl UserInitProcess {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            reuses_kernel_init_task: false,
            pid1_preserved: false,
            exec_identity_handoff: false,
            no_new_task_struct: false,
            kernel_init_not_destroyed: false,
            path: UserInitPathRef::DefaultInit,
            path_bound: false,
            address_space_bound: false,
            fs_struct_inherited: false,
            files_struct_inherited: false,
            trap_frame_bound: false,
            syscall_context_bound: false,
            kernel_init_execve_to_user_init: false,
            kernel_init_pid1_identity_preserved: false,
            kernel_init_user_mm_attached: false,
            kernel_init_user_trap_frame_attached: false,
            user_entry_ready: false,
            trap_return_bound: false,
            trap_return_context_used: false,
            trap_return_sfence_vma_after_satp: false,
            trap_return_sret_handoff: false,
            syscall_dispatch_bound: false,
            syscall_arguments_extracted: false,
            runtime_entered: false,
            credentials_inherited: false,
            root_credentials_bound: false,
            credentials_capability_model_deferred: false,
            uid: 0,
            gid: 0,
            euid: 0,
            egid: 0,
            suid: 0,
            sgid: 0,
            fsuid: 0,
            fsgid: 0,
            pid_read_observed: false,
            ppid_zero_first_slice: false,
            ppid_read_observed: false,
            session_leader_first_slice: false,
            process_group_leader_first_slice: false,
            process_group: 0,
            process_group_read_observed: false,
            process_group_set_observed: false,
            controlling_tty_bound: false,
            foreground_pgrp_bound: false,
            foreground_pgrp: 0,
            foreground_pgrp_read_observed: false,
            foreground_pgrp_set_observed: false,
            uid_read_observed: false,
            euid_read_observed: false,
            gid_read_observed: false,
            egid_read_observed: false,
            resuid_read_observed: false,
            resgid_read_observed: false,
            uid_set_observed: false,
            gid_set_observed: false,
            signal_state_inherited: false,
            signal_runtime_bound: false,
            thread_signal_state_bound: false,
            process_signal_state_deferred: false,
            signal_action_table_bound: false,
            signal_action_table_layout_bound: false,
            blocked_signal_mask_bound: false,
            signal_delivery_deferred: false,
            blocked_signal_mask: 0,
            signal_actions: [UserSignalAction::default(); USER_SIGNAL_COUNT],
            rt_sigprocmask_observed: false,
            rt_sigaction_observed: false,
            clear_child_tid_bound: false,
            clear_child_tid: 0,
            root_cwd_first_slice: false,
            getcwd_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn reuses_kernel_init_task(&self) -> bool {
        self.reuses_kernel_init_task
    }

    pub const fn pid1_preserved(&self) -> bool {
        self.pid1_preserved
    }

    pub const fn exec_identity_handoff(&self) -> bool {
        self.exec_identity_handoff
    }

    pub const fn no_new_task_struct(&self) -> bool {
        self.no_new_task_struct
    }

    pub const fn kernel_init_not_destroyed(&self) -> bool {
        self.kernel_init_not_destroyed
    }

    pub const fn path(&self) -> UserInitPathRef {
        self.path
    }

    pub const fn path_bound(&self) -> bool {
        self.path_bound
    }

    pub const fn address_space_bound(&self) -> bool {
        self.address_space_bound
    }

    pub const fn fs_struct_inherited(&self) -> bool {
        self.fs_struct_inherited
    }

    pub const fn files_struct_inherited(&self) -> bool {
        self.files_struct_inherited
    }

    pub const fn trap_frame_bound(&self) -> bool {
        self.trap_frame_bound
    }

    pub const fn syscall_context_bound(&self) -> bool {
        self.syscall_context_bound
    }

    pub const fn kernel_init_execve_to_user_init(&self) -> bool {
        self.kernel_init_execve_to_user_init
    }

    pub const fn kernel_init_pid1_identity_preserved(&self) -> bool {
        self.kernel_init_pid1_identity_preserved
    }

    pub const fn kernel_init_user_mm_attached(&self) -> bool {
        self.kernel_init_user_mm_attached
    }

    pub const fn kernel_init_user_trap_frame_attached(&self) -> bool {
        self.kernel_init_user_trap_frame_attached
    }

    pub const fn user_entry_ready(&self) -> bool {
        self.user_entry_ready
    }

    pub const fn trap_return_bound(&self) -> bool {
        self.trap_return_bound
    }

    pub const fn trap_return_context_used(&self) -> bool {
        self.trap_return_context_used
    }

    pub const fn trap_return_sfence_vma_after_satp(&self) -> bool {
        self.trap_return_sfence_vma_after_satp
    }

    pub const fn trap_return_sret_handoff(&self) -> bool {
        self.trap_return_sret_handoff
    }

    pub const fn syscall_dispatch_bound(&self) -> bool {
        self.syscall_dispatch_bound
    }

    pub const fn syscall_arguments_extracted(&self) -> bool {
        self.syscall_arguments_extracted
    }

    pub const fn runtime_entered(&self) -> bool {
        self.runtime_entered
    }

    pub const fn credentials_inherited(&self) -> bool {
        self.credentials_inherited
    }

    pub const fn root_credentials_bound(&self) -> bool {
        self.root_credentials_bound
    }

    pub const fn credentials_capability_model_deferred(&self) -> bool {
        self.credentials_capability_model_deferred
    }

    pub const fn uid(&self) -> usize {
        self.uid
    }

    pub const fn gid(&self) -> usize {
        self.gid
    }

    pub const fn euid(&self) -> usize {
        self.euid
    }

    pub const fn egid(&self) -> usize {
        self.egid
    }

    pub const fn suid(&self) -> usize {
        self.suid
    }

    pub const fn sgid(&self) -> usize {
        self.sgid
    }

    pub const fn fsuid(&self) -> usize {
        self.fsuid
    }

    pub const fn fsgid(&self) -> usize {
        self.fsgid
    }

    pub const fn pid_read_observed(&self) -> bool {
        self.pid_read_observed
    }

    pub const fn ppid_zero_first_slice(&self) -> bool {
        self.ppid_zero_first_slice
    }

    pub const fn ppid_read_observed(&self) -> bool {
        self.ppid_read_observed
    }

    pub const fn session_leader_first_slice(&self) -> bool {
        self.session_leader_first_slice
    }

    pub const fn process_group_leader_first_slice(&self) -> bool {
        self.process_group_leader_first_slice
    }

    pub const fn process_group(&self) -> usize {
        self.process_group
    }

    pub const fn process_group_read_observed(&self) -> bool {
        self.process_group_read_observed
    }

    pub const fn process_group_set_observed(&self) -> bool {
        self.process_group_set_observed
    }

    pub const fn controlling_tty_bound(&self) -> bool {
        self.controlling_tty_bound
    }

    pub const fn foreground_pgrp_bound(&self) -> bool {
        self.foreground_pgrp_bound
    }

    pub const fn foreground_pgrp(&self) -> usize {
        self.foreground_pgrp
    }

    pub const fn foreground_pgrp_read_observed(&self) -> bool {
        self.foreground_pgrp_read_observed
    }

    pub const fn foreground_pgrp_set_observed(&self) -> bool {
        self.foreground_pgrp_set_observed
    }

    pub const fn uid_read_observed(&self) -> bool {
        self.uid_read_observed
    }

    pub const fn euid_read_observed(&self) -> bool {
        self.euid_read_observed
    }

    pub const fn gid_read_observed(&self) -> bool {
        self.gid_read_observed
    }

    pub const fn egid_read_observed(&self) -> bool {
        self.egid_read_observed
    }

    pub const fn resuid_read_observed(&self) -> bool {
        self.resuid_read_observed
    }

    pub const fn resgid_read_observed(&self) -> bool {
        self.resgid_read_observed
    }

    pub const fn uid_set_observed(&self) -> bool {
        self.uid_set_observed
    }

    pub const fn gid_set_observed(&self) -> bool {
        self.gid_set_observed
    }

    pub const fn signal_state_inherited(&self) -> bool {
        self.signal_state_inherited
    }

    pub const fn signal_runtime_bound(&self) -> bool {
        self.signal_runtime_bound
    }

    pub const fn thread_signal_state_bound(&self) -> bool {
        self.thread_signal_state_bound
    }

    pub const fn process_signal_state_deferred(&self) -> bool {
        self.process_signal_state_deferred
    }

    pub const fn signal_action_table_bound(&self) -> bool {
        self.signal_action_table_bound
    }

    pub const fn signal_action_table_layout_bound(&self) -> bool {
        self.signal_action_table_layout_bound
    }

    pub const fn blocked_signal_mask_bound(&self) -> bool {
        self.blocked_signal_mask_bound
    }

    pub const fn signal_delivery_deferred(&self) -> bool {
        self.signal_delivery_deferred
    }

    pub const fn blocked_signal_mask(&self) -> usize {
        self.blocked_signal_mask
    }

    pub const fn rt_sigprocmask_observed(&self) -> bool {
        self.rt_sigprocmask_observed
    }

    pub const fn rt_sigaction_observed(&self) -> bool {
        self.rt_sigaction_observed
    }

    pub fn credentials_syscall_ready(&self) -> bool {
        self.lifecycle.state() == State::Online
            && self.credentials_inherited
            && self.root_credentials_bound
    }

    pub fn signal_mask_syscall_ready(&self) -> bool {
        self.lifecycle.state() == State::Online
            && self.signal_state_inherited
            && self.signal_runtime_bound
            && self.thread_signal_state_bound
            && self.blocked_signal_mask_bound
    }

    pub fn signal_action_syscall_ready(&self) -> bool {
        self.lifecycle.state() == State::Online
            && self.signal_state_inherited
            && self.signal_runtime_bound
            && self.signal_action_table_bound
            && self.signal_action_table_layout_bound
    }

    pub const fn clear_child_tid_bound(&self) -> bool {
        self.clear_child_tid_bound
    }

    pub const fn clear_child_tid(&self) -> usize {
        self.clear_child_tid
    }

    pub const fn root_cwd_first_slice(&self) -> bool {
        self.root_cwd_first_slice
    }

    pub const fn getcwd_observed(&self) -> bool {
        self.getcwd_observed
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        trap_frame: &UserTrapFrame,
        fs_struct: &FsStruct,
        files_struct: &FilesStruct,
        selected_path: UserInitPathRef,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || kernel_init_task.pid() != super::rest_init::KERNEL_INIT_PID
            || address_space.state() != State::Online
            || elf.state() != State::Online
            || trap_frame.state() != State::Ready
            || fs_struct.state() != State::Ready
            || files_struct.state() != State::Ready
            || !address_space.bound_to_kernel_init_task()
            || !trap_frame.address_space_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.reuses_kernel_init_task = true;
        self.pid1_preserved = true;
        self.exec_identity_handoff = true;
        self.no_new_task_struct = true;
        self.kernel_init_not_destroyed = kernel_init_task.state() == State::Online;
        self.path = selected_path;
        self.path_bound = true;
        self.address_space_bound = true;
        self.fs_struct_inherited = true;
        self.files_struct_inherited = true;
        self.trap_frame_bound = true;
        self.kernel_init_execve_to_user_init = true;
        self.kernel_init_pid1_identity_preserved = true;
        self.kernel_init_user_mm_attached = true;
        self.kernel_init_user_trap_frame_attached = true;
        self.credentials_inherited = true;
        self.root_credentials_bound = true;
        self.credentials_capability_model_deferred = true;
        self.uid = 0;
        self.gid = 0;
        self.euid = 0;
        self.egid = 0;
        self.suid = 0;
        self.sgid = 0;
        self.fsuid = 0;
        self.fsgid = 0;
        self.session_leader_first_slice = true;
        self.process_group_leader_first_slice = true;
        self.process_group = super::rest_init::KERNEL_INIT_PID;
        self.controlling_tty_bound = true;
        self.foreground_pgrp_bound = true;
        self.foreground_pgrp = super::rest_init::KERNEL_INIT_PID;
        self.signal_state_inherited = true;
        self.signal_runtime_bound = true;
        self.thread_signal_state_bound = true;
        self.process_signal_state_deferred = true;
        self.signal_action_table_bound = true;
        self.signal_action_table_layout_bound = true;
        self.blocked_signal_mask_bound = true;
        self.signal_delivery_deferred = true;
        self.blocked_signal_mask = 0;
        self.signal_actions = [UserSignalAction::default(); USER_SIGNAL_COUNT];
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn enable(
        &mut self,
        trap_frame: &UserTrapFrame,
        exception_stream: &ExceptionStream,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || trap_frame.state() != State::Ready
            || exception_stream.syscall_state() != State::Online
            || syscall_table.state() != State::Ready
            || !syscall_table.bound_to_exception()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.syscall_context_bound = true;
        self.trap_return_bound = true;
        self.syscall_dispatch_bound = true;
        self.syscall_arguments_extracted = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    pub fn enter_user_mode(&mut self, trap_frame: &UserTrapFrame) -> EventResult {
        if self.lifecycle.state() != State::Online
            || trap_frame.state() != State::Ready
            || !self.trap_return_bound
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.user_entry_ready = true;
        self.trap_return_context_used = true;
        self.trap_return_sfence_vma_after_satp = true;
        self.trap_return_sret_handoff = true;
        self.runtime_entered = true;
        USER_INIT_RUNTIME_ENTERED.store(1, Ordering::Release);
        Ok(())
    }

    pub fn refresh_runtime_observations(&mut self) {
        self.runtime_entered |= USER_INIT_RUNTIME_ENTERED.load(Ordering::Acquire) != 0;
    }

    pub fn set_clear_child_tid(&mut self, tidptr: usize) -> usize {
        if self.lifecycle.state() != State::Online {
            return 0;
        }
        self.clear_child_tid = tidptr;
        self.clear_child_tid_bound = true;
        super::rest_init::KERNEL_INIT_PID
    }

    pub fn read_uid(&mut self) -> Option<usize> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.uid_read_observed = true;
        Some(self.uid)
    }

    pub fn read_pid(&mut self) -> Option<usize> {
        if self.lifecycle.state() != State::Online || !self.pid1_preserved {
            return None;
        }
        self.pid_read_observed = true;
        Some(super::rest_init::KERNEL_INIT_PID)
    }

    pub fn read_ppid(&mut self) -> Option<usize> {
        if self.lifecycle.state() != State::Online || !self.pid1_preserved {
            return None;
        }
        self.ppid_zero_first_slice = true;
        self.ppid_read_observed = true;
        Some(0)
    }

    pub fn read_process_group(&mut self, pid: usize) -> UserProcessGroupLookup {
        if self.lifecycle.state() != State::Online
            || !self.pid1_preserved
            || !self.process_group_leader_first_slice
            || self.process_group == 0
        {
            return UserProcessGroupLookup::NotReady;
        }
        if pid != 0 && pid != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupLookup::NoSuchProcess;
        }
        self.process_group_read_observed = true;
        UserProcessGroupLookup::Found(self.process_group)
    }

    pub fn set_process_group_first_slice(
        &mut self,
        pid_arg: usize,
        pgid_arg: usize,
    ) -> UserProcessGroupUpdate {
        if self.lifecycle.state() != State::Online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || !self.process_group_leader_first_slice
            || self.process_group == 0
        {
            return UserProcessGroupUpdate::NotReady;
        }

        let pid = pid_t_arg(pid_arg);
        let pgid = pid_t_arg(pgid_arg);
        if pgid < 0 {
            return UserProcessGroupUpdate::Invalid;
        }
        let normalized_pid = if pid == 0 {
            super::rest_init::KERNEL_INIT_PID
        } else if pid < 0 {
            return UserProcessGroupUpdate::NoSuchProcess;
        } else {
            pid as usize
        };
        if normalized_pid != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupUpdate::NoSuchProcess;
        }

        let normalized_pgid = if pgid == 0 {
            normalized_pid
        } else {
            pgid as usize
        };
        if normalized_pgid != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupUpdate::PermissionDenied;
        }

        self.process_group = normalized_pgid;
        self.process_group_set_observed = true;
        UserProcessGroupUpdate::Updated(self.process_group)
    }

    pub fn read_foreground_pgrp(&mut self) -> Option<usize> {
        if self.lifecycle.state() != State::Online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || !self.process_group_leader_first_slice
            || !self.controlling_tty_bound
        {
            return None;
        }
        self.foreground_pgrp_read_observed = true;
        if self.foreground_pgrp_bound {
            Some(self.foreground_pgrp)
        } else {
            Some(0)
        }
    }

    pub fn set_foreground_pgrp_first_slice(&mut self, pgrp_arg: u32) -> UserProcessGroupUpdate {
        if self.lifecycle.state() != State::Online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || !self.process_group_leader_first_slice
            || !self.controlling_tty_bound
        {
            return UserProcessGroupUpdate::NotReady;
        }

        let pgrp = pgrp_arg as i32;
        if pgrp < 0 {
            return UserProcessGroupUpdate::Invalid;
        }
        if pgrp as usize != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupUpdate::NoSuchProcess;
        }
        if self.process_group != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupUpdate::PermissionDenied;
        }

        self.foreground_pgrp = pgrp as usize;
        self.foreground_pgrp_bound = true;
        self.foreground_pgrp_set_observed = true;
        UserProcessGroupUpdate::Updated(self.foreground_pgrp)
    }

    pub fn read_euid(&mut self) -> Option<usize> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.euid_read_observed = true;
        Some(self.euid)
    }

    pub fn read_gid(&mut self) -> Option<usize> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.gid_read_observed = true;
        Some(self.gid)
    }

    pub fn read_egid(&mut self) -> Option<usize> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.egid_read_observed = true;
        Some(self.egid)
    }

    pub fn read_resuid(&mut self) -> Option<(usize, usize, usize)> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.resuid_read_observed = true;
        Some((self.uid, self.euid, self.suid))
    }

    pub fn read_resgid(&mut self) -> Option<(usize, usize, usize)> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.resgid_read_observed = true;
        Some((self.gid, self.egid, self.sgid))
    }

    pub fn set_uid_root_slice(&mut self, uid: usize) -> bool {
        if !self.credentials_syscall_ready() || uid != 0 {
            return false;
        }
        self.uid = uid;
        self.euid = uid;
        self.suid = uid;
        self.fsuid = uid;
        self.uid_set_observed = true;
        true
    }

    pub fn set_gid_root_slice(&mut self, gid: usize) -> bool {
        if !self.credentials_syscall_ready() || gid != 0 {
            return false;
        }
        self.gid = gid;
        self.egid = gid;
        self.sgid = gid;
        self.fsgid = gid;
        self.gid_set_observed = true;
        true
    }

    pub fn set_blocked_signal_mask(&mut self, mask: usize) -> bool {
        if !self.signal_mask_syscall_ready() {
            return false;
        }
        self.blocked_signal_mask = mask;
        true
    }

    pub fn observe_rt_sigprocmask(&mut self) -> bool {
        if !self.signal_mask_syscall_ready() {
            return false;
        }
        self.rt_sigprocmask_observed = true;
        true
    }

    pub fn read_signal_action(&self, signal: usize) -> Option<UserSignalAction> {
        if !self.signal_action_syscall_ready() || signal == 0 || signal > USER_SIGNAL_COUNT {
            return None;
        }
        Some(self.signal_actions[signal - 1])
    }

    pub fn set_signal_action(&mut self, signal: usize, action: UserSignalAction) -> bool {
        if !self.signal_action_syscall_ready() || signal == 0 || signal > USER_SIGNAL_COUNT {
            return false;
        }
        self.signal_actions[signal - 1] = action;
        true
    }

    pub fn observe_rt_sigaction(&mut self) -> bool {
        if !self.signal_action_syscall_ready() {
            return false;
        }
        self.rt_sigaction_observed = true;
        true
    }

    pub fn observe_getcwd_root_slice(&mut self) -> bool {
        if self.lifecycle.state() != State::Online || !self.fs_struct_inherited {
            return false;
        }
        self.root_cwd_first_slice = true;
        self.getcwd_observed = true;
        true
    }
}

pub struct UserBootPayload {
    lifecycle: Lifecycle,
    exec_sync_boundaries_ready: bool,
    selected: bool,
    candidates_bound: bool,
    default_init_path_bound: bool,
    default_init_fallback_order_bound: bool,
    candidate_failure_nonfatal_for_fallback: bool,
    first_successful_candidate_selected: bool,
    success_stops_fallback_chain: bool,
    success_no_return_to_startup_orchestration: bool,
    no_working_init_panic_terminal_bound: bool,
    init_attempt_failure_trace_defined: bool,
    init_attempt_failure_recorded: bool,
    init_attempt_failure_checkpoint_bound: bool,
    init_attempt_failure: UserInitAttemptFailure,
    uses_current_fs_struct: bool,
    no_partition_dependency: bool,
    partition_objects_deferred: bool,
    driven_by_kernel_init_task: bool,
    try_candidate_bound: bool,
    selected_path: UserInitPathRef,
    selected_path_bytes: [u8; USER_SELECTED_PATH_MAX],
    selected_path_len: usize,
    selected_path_bound: bool,
    selected_argv0_path_bound: bool,
    reads_init_from_vfs: bool,
    enters_user_mode: bool,
    no_return_handoff: bool,
}

#[allow(dead_code)]
impl UserBootPayload {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            exec_sync_boundaries_ready: false,
            selected: false,
            candidates_bound: false,
            default_init_path_bound: false,
            default_init_fallback_order_bound: false,
            candidate_failure_nonfatal_for_fallback: false,
            first_successful_candidate_selected: false,
            success_stops_fallback_chain: false,
            success_no_return_to_startup_orchestration: false,
            no_working_init_panic_terminal_bound: false,
            init_attempt_failure_trace_defined: false,
            init_attempt_failure_recorded: false,
            init_attempt_failure_checkpoint_bound: false,
            init_attempt_failure: UserInitAttemptFailure::empty(),
            uses_current_fs_struct: false,
            no_partition_dependency: false,
            partition_objects_deferred: false,
            driven_by_kernel_init_task: false,
            try_candidate_bound: false,
            selected_path: UserInitPathRef::DefaultInit,
            selected_path_bytes: [0; USER_SELECTED_PATH_MAX],
            selected_path_len: 0,
            selected_path_bound: false,
            selected_argv0_path_bound: false,
            reads_init_from_vfs: false,
            enters_user_mode: false,
            no_return_handoff: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn exec_sync_boundaries_ready(&self) -> bool {
        self.exec_sync_boundaries_ready
    }

    pub const fn selected(&self) -> bool {
        self.selected
    }

    pub const fn candidates_bound(&self) -> bool {
        self.candidates_bound
    }

    pub const fn default_init_path_bound(&self) -> bool {
        self.default_init_path_bound
    }

    pub const fn default_init_fallback_order_bound(&self) -> bool {
        self.default_init_fallback_order_bound
    }

    pub const fn candidate_failure_nonfatal_for_fallback(&self) -> bool {
        self.candidate_failure_nonfatal_for_fallback
    }

    pub const fn first_successful_candidate_selected(&self) -> bool {
        self.first_successful_candidate_selected
    }

    pub const fn success_stops_fallback_chain(&self) -> bool {
        self.success_stops_fallback_chain
    }

    pub const fn success_no_return_to_startup_orchestration(&self) -> bool {
        self.success_no_return_to_startup_orchestration
    }

    pub const fn no_working_init_panic_terminal_bound(&self) -> bool {
        self.no_working_init_panic_terminal_bound
    }

    pub const fn init_attempt_failure_trace_defined(&self) -> bool {
        self.init_attempt_failure_trace_defined
    }

    pub const fn init_attempt_failure_recorded(&self) -> bool {
        self.init_attempt_failure_recorded
    }

    pub const fn init_attempt_failure_checkpoint_bound(&self) -> bool {
        self.init_attempt_failure_checkpoint_bound
    }

    pub const fn last_init_attempt_failure(&self) -> UserInitAttemptFailure {
        self.init_attempt_failure
    }

    pub const fn uses_current_fs_struct(&self) -> bool {
        self.uses_current_fs_struct
    }

    pub const fn no_partition_dependency(&self) -> bool {
        self.no_partition_dependency
    }

    pub const fn partition_objects_deferred(&self) -> bool {
        self.partition_objects_deferred
    }

    pub const fn driven_by_kernel_init_task(&self) -> bool {
        self.driven_by_kernel_init_task
    }

    pub const fn try_candidate_bound(&self) -> bool {
        self.try_candidate_bound
    }

    pub const fn selected_path(&self) -> UserInitPathRef {
        self.selected_path
    }

    pub fn selected_path_bytes(&self) -> &[u8] {
        &self.selected_path_bytes[..self.selected_path_len]
    }

    pub const fn selected_path_bound(&self) -> bool {
        self.selected_path_bound
    }

    pub const fn selected_argv0_path_bound(&self) -> bool {
        self.selected_argv0_path_bound
    }

    pub const fn reads_init_from_vfs(&self) -> bool {
        self.reads_init_from_vfs
    }

    pub const fn enters_user_mode(&self) -> bool {
        self.enters_user_mode
    }

    pub const fn no_return_handoff(&self) -> bool {
        self.no_return_handoff
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        exec_sync: &PayloadExecSyncBoundaries,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || exec_sync.state() != State::Ready
            || !exec_sync.kernel_execve_linux_window_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.exec_sync_boundaries_ready = true;
        self.selected = true;
        self.candidates_bound = true;
        self.default_init_path_bound = true;
        self.default_init_fallback_order_bound = true;
        self.candidate_failure_nonfatal_for_fallback = true;
        self.success_stops_fallback_chain = true;
        self.success_no_return_to_startup_orchestration = true;
        self.no_working_init_panic_terminal_bound = true;
        self.init_attempt_failure_trace_defined = true;
        self.uses_current_fs_struct = true;
        self.no_partition_dependency = true;
        self.partition_objects_deferred = true;
        self.driven_by_kernel_init_task = true;
        self.try_candidate_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn try_candidate(
        &mut self,
        path: UserInitPathRef,
        actual_path: &[u8],
        elf: &ElfObject,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || elf.state() != State::Ready
            || !valid_selected_path(actual_path)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.selected_path = path;
        self.selected_path_bytes = [0; USER_SELECTED_PATH_MAX];
        self.selected_path_bytes[..actual_path.len()].copy_from_slice(actual_path);
        self.selected_path_len = actual_path.len();
        self.selected_path_bound = true;
        self.selected_argv0_path_bound = true;
        self.first_successful_candidate_selected = true;
        self.reads_init_from_vfs = true;
        Ok(())
    }

    #[cfg(app_user_boot)]
    pub fn record_init_attempt_failure(&mut self, failure: UserInitAttemptFailure) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || failure.stage() == UserInitAttemptStage::None
            || failure.reason() == UserInitAttemptReason::None
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.init_attempt_failure = failure;
        self.init_attempt_failure_recorded = true;
        self.init_attempt_failure_checkpoint_bound = true;
        Ok(())
    }

    #[cfg(app_user_boot)]
    pub fn enable_for_user_entry(
        &mut self,
        elf: &ElfObject,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        exception_stream: &ExceptionStream,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || elf.state() != State::Online
            || address_space.state() != State::Online
            || trap_frame.state() != State::Ready
            || syscall_table.state() != State::Ready
            || exception_stream.syscall_state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.enters_user_mode = true;
        self.no_return_handoff = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}

#[cfg(app_user_boot)]
#[allow(clippy::too_many_arguments)]
pub fn run_first_user_init(
    payload: &mut UserBootPayload,
    elf: &mut ElfObject,
    interpreter: &mut ElfObject,
    address_space: &mut UserAddressSpace,
    stack: &mut UserStack,
    trap_frame: &mut UserTrapFrame,
    user_init_process: &mut UserInitProcess,
    vfs_core: &mut VfsCore,
    fs_struct: &FsStruct,
    files_struct: &mut FilesStruct,
    ext2_filesystem: &mut Ext2FileSystem,
    block_device_registry: &mut BlockDeviceRegistry,
    kernel_init_task: &KernelInitTask,
    exec_sync: &PayloadExecSyncBoundaries,
    swapper_vm: &SwapperVm,
    kernel_image: &KernelImage,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
    kernel_global_allocator: &KernelGlobalAllocator,
    exception_stream: &mut ExceptionStream,
    syscall_table: &mut SyscallTable,
    boot_param: &BootParam,
    static_command_line: &StaticCommandLine,
) -> ! {
    if payload.setup(kernel_init_task, exec_sync).is_err() {
        user_boot_panic("user payload setup failed\n");
    }

    let selected = select_user_init_candidate(
        payload,
        vfs_core,
        fs_struct,
        ext2_filesystem,
        block_device_registry,
        kernel_image,
        boot_param,
        static_command_line,
    );
    let image = selected.image;
    if elf.preset_from_vfs(image).is_err() || elf.setup(image).is_err() {
        user_boot_panic("user init ELF setup failed\n");
    }
    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::UserBootMainElfReady,
        crate::context::context_ref(),
    );
    let interpreter_image = if let Some(path) = elf.interpreter_path() {
        let image = read_user_path_image(
            vfs_core,
            fs_struct,
            ext2_filesystem,
            block_device_registry,
            kernel_image,
            path,
            true,
        );
        if interpreter.preset_interpreter_from_vfs(image).is_err()
            || interpreter.setup(image).is_err()
            || elf.bind_runtime_interpreter(interpreter).is_err()
        {
            user_boot_panic("user interp ELF setup failed\n");
        }
        crate::checkpoint::dispatch(
            crate::trace::Checkpoint::UserBootInterpreterReady,
            crate::context::context_ref(),
        );
        Some(image)
    } else {
        None
    };
    let interpreter_ref = interpreter_image.map(|_| &*interpreter);
    if payload
        .try_candidate(selected.path, selected.actual_path(), elf)
        .is_err()
    {
        user_boot_panic("user init candidate failed\n");
    }
    if address_space
        .preset(
            swapper_vm,
            page_allocator,
            kernel_global_allocator,
            kernel_init_task,
        )
        .is_err()
    {
        user_boot_panic("user address space preset failed\n");
    }
    if stack
        .setup(
            address_space,
            elf,
            interpreter_ref,
            selected.actual_path(),
            page_allocator,
            page_metadata_map,
        )
        .is_err()
    {
        user_boot_panic("user stack setup failed\n");
    }
    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::UserBootAddressSpaceSetupStart,
        crate::context::context_ref(),
    );
    if address_space
        .setup(
            elf,
            interpreter_ref,
            stack,
            image,
            interpreter_image,
            page_allocator,
            page_metadata_map,
        )
        .is_err()
    {
        user_boot_panic("user address space setup failed\n");
    }
    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::UserAddressSpaceReady,
        crate::context::context_ref(),
    );
    if trap_frame.setup(address_space, elf, stack).is_err() {
        user_boot_panic("user trap frame setup failed\n");
    }
    if elf.enable(address_space, stack, trap_frame).is_err() {
        user_boot_panic("user ELF enable failed\n");
    }
    if address_space
        .enable(
            trap_frame,
            swapper_vm,
            kernel_image,
            page_allocator,
            page_metadata_map,
        )
        .is_err()
    {
        user_boot_panic("user address space enable failed\n");
    }
    if exception_stream.syscall_setup(syscall_table).is_err() {
        user_boot_panic("user syscall setup failed\n");
    }
    if exception_stream.syscall_enable(syscall_table).is_err() {
        user_boot_panic("user syscall enable failed\n");
    }
    if files_struct.setup(kernel_init_task).is_err() {
        user_boot_panic("user files struct setup failed\n");
    }
    if files_struct.clear_stdin_ready_data().is_err() {
        user_boot_panic("user stdin ready data clear failed\n");
    }
    if selected_user_init_needs_stdin_fixture(&selected) {
        if files_struct
            .prepare_default_stdin_ready_data(STDIN_READY_FIXTURE)
            .is_err()
        {
            user_boot_panic("user stdin ready data setup failed\n");
        }
    } else if files_struct.enable_stdin_blocking_wait().is_err() {
        user_boot_panic("user stdin blocking wait setup failed\n");
    }
    if user_init_process
        .setup(
            kernel_init_task,
            address_space,
            elf,
            trap_frame,
            fs_struct,
            files_struct,
            selected.path,
        )
        .is_err()
    {
        user_boot_panic("user init process setup failed\n");
    }
    if user_init_process
        .enable(trap_frame, exception_stream, syscall_table)
        .is_err()
    {
        user_boot_panic("user init process enable failed\n");
    }
    if payload
        .enable_for_user_entry(
            elf,
            address_space,
            trap_frame,
            exception_stream,
            syscall_table,
        )
        .is_err()
    {
        user_boot_panic("user payload enable failed\n");
    }
    if user_init_process.enter_user_mode(trap_frame).is_err() {
        user_boot_panic("user init process enter failed\n");
    }

    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::UserModeEntry,
        crate::context::context_ref(),
    );
    unsafe {
        crate::arch::riscv64::csr::enter_user_mode(
            address_space.satp_token(),
            trap_frame.entry(),
            trap_frame.sp(),
            trap_frame.sstatus(),
            user_kernel_trap_stack_top(),
        )
    }
}

#[cfg(app_user_boot)]
struct SelectedUserInit {
    path: UserInitPathRef,
    actual_path: [u8; USER_SELECTED_PATH_MAX],
    actual_path_len: usize,
    image: &'static [u8],
}

#[cfg(app_user_boot)]
impl SelectedUserInit {
    fn new(path: UserInitPathRef, actual_path: &[u8], image: &'static [u8]) -> Self {
        let mut path_buffer = [0u8; USER_SELECTED_PATH_MAX];
        path_buffer[..actual_path.len()].copy_from_slice(actual_path);
        Self {
            path,
            actual_path: path_buffer,
            actual_path_len: actual_path.len(),
            image,
        }
    }

    fn actual_path(&self) -> &[u8] {
        &self.actual_path[..self.actual_path_len]
    }
}

#[cfg(app_user_boot)]
fn select_user_init_candidate(
    payload: &mut UserBootPayload,
    vfs_core: &mut VfsCore,
    fs_struct: &FsStruct,
    ext2_filesystem: &mut Ext2FileSystem,
    block_device_registry: &mut BlockDeviceRegistry,
    kernel_image: &KernelImage,
    boot_param: &BootParam,
    static_command_line: &StaticCommandLine,
) -> SelectedUserInit {
    if let Some(requested) = boot_param.init_value(static_command_line.as_bytes()) {
        if !valid_selected_path(requested) {
            emit_init_attempt_failure(
                payload,
                UserInitAttemptFailure::new(
                    UserInitPathRef::RequestedInit,
                    requested,
                    UserInitAttemptStage::ValidatePath,
                    UserInitAttemptReason::InvalidPath,
                    true,
                    false,
                    None,
                ),
            );
            user_boot_panic("requested init failed\n");
        }
        match try_read_user_path_image(
            vfs_core,
            fs_struct,
            ext2_filesystem,
            block_device_registry,
            kernel_image,
            requested,
            false,
        ) {
            Ok(image) => match inspect_elf_candidate(image) {
                Ok(()) => {
                    return SelectedUserInit::new(UserInitPathRef::RequestedInit, requested, image);
                }
                Err((stage, reason, elf_error)) => {
                    emit_init_attempt_failure(
                        payload,
                        UserInitAttemptFailure::new(
                            UserInitPathRef::RequestedInit,
                            requested,
                            stage,
                            reason,
                            true,
                            false,
                            elf_error,
                        ),
                    );
                    user_boot_panic("requested init failed\n");
                }
            },
            Err(()) => {
                emit_init_attempt_failure(
                    payload,
                    UserInitAttemptFailure::new(
                        UserInitPathRef::RequestedInit,
                        requested,
                        UserInitAttemptStage::ReadImage,
                        UserInitAttemptReason::ReadFailed,
                        true,
                        false,
                        None,
                    ),
                );
                user_boot_panic("requested init failed\n");
            }
        }
    }

    let mut index = 0usize;
    while index < USER_INIT_CANDIDATES.len() {
        let path = USER_INIT_CANDIDATES[index];
        match try_read_user_path_image(
            vfs_core,
            fs_struct,
            ext2_filesystem,
            block_device_registry,
            kernel_image,
            path.path(),
            false,
        ) {
            Ok(image) => match inspect_elf_candidate(image) {
                Ok(()) => {
                    return SelectedUserInit::new(path, path.path(), image);
                }
                Err((stage, reason, elf_error)) => emit_init_attempt_failure(
                    payload,
                    UserInitAttemptFailure::new(
                        path,
                        path.path(),
                        stage,
                        reason,
                        false,
                        true,
                        elf_error,
                    ),
                ),
            },
            Err(()) => emit_init_attempt_failure(
                payload,
                UserInitAttemptFailure::new(
                    path,
                    path.path(),
                    UserInitAttemptStage::ReadImage,
                    UserInitAttemptReason::ReadFailed,
                    false,
                    true,
                    None,
                ),
            ),
        }
        index += 1;
    }

    user_boot_panic("no working init found\n")
}

fn valid_selected_path(path: &[u8]) -> bool {
    !path.is_empty() && path.len() <= USER_SELECTED_PATH_MAX && path[0] == b'/'
}

#[cfg(app_user_boot)]
fn selected_user_init_needs_stdin_fixture(selected: &SelectedUserInit) -> bool {
    contains_bytes(selected.image, USER_SMOKE_STDIN_MARKER)
}

#[cfg(app_user_boot)]
fn read_user_path_image(
    vfs_core: &mut VfsCore,
    fs_struct: &FsStruct,
    ext2_filesystem: &mut Ext2FileSystem,
    block_device_registry: &mut BlockDeviceRegistry,
    kernel_image: &KernelImage,
    path: &[u8],
    use_interpreter_buffer: bool,
) -> &'static [u8] {
    match try_read_user_path_image(
        vfs_core,
        fs_struct,
        ext2_filesystem,
        block_device_registry,
        kernel_image,
        path,
        use_interpreter_buffer,
    ) {
        Ok(image) => image,
        Err(()) => user_boot_panic("read user ELF failed\n"),
    }
}

#[cfg(app_user_boot)]
fn try_read_user_path_image(
    vfs_core: &mut VfsCore,
    fs_struct: &FsStruct,
    ext2_filesystem: &mut Ext2FileSystem,
    block_device_registry: &mut BlockDeviceRegistry,
    kernel_image: &KernelImage,
    path: &[u8],
    use_interpreter_buffer: bool,
) -> Result<&'static [u8], ()> {
    let mut provider = virtio_blk::live_provider(kernel_image);
    let buffer = unsafe {
        if use_interpreter_buffer {
            let ptr = core::ptr::addr_of_mut!(USER_BOOT_INTERPRETER_READ_BUFFER);
            &mut *ptr
        } else {
            let ptr = core::ptr::addr_of_mut!(USER_BOOT_READ_BUFFER);
            &mut *ptr
        }
    };
    buffer.fill(0);
    let len = vfs_core
        .read_path(
            fs_struct,
            ext2_filesystem,
            block_device_registry,
            &mut provider,
            path,
            buffer,
        )
        .map_err(|_| ())?;
    Ok(&buffer[..len])
}

#[cfg(app_user_boot)]
fn inspect_elf_candidate(
    input: &[u8],
) -> Result<
    (),
    (
        UserInitAttemptStage,
        UserInitAttemptReason,
        Option<ElfError>,
    ),
> {
    let header = parse_header(input).map_err(|error| {
        (
            UserInitAttemptStage::PresetElf,
            UserInitAttemptReason::ElfPresetFailed,
            Some(error),
        )
    })?;
    if !elf_type_allowed_for_role(header.elf_type, ElfObjectRole::MainExecutable) {
        return Err((
            UserInitAttemptStage::PresetElf,
            UserInitAttemptReason::ElfPresetFailed,
            Some(ElfError::UnsupportedType),
        ));
    }
    let load_bias = main_executable_load_bias(&header).map_err(|error| {
        (
            UserInitAttemptStage::PresetElf,
            UserInitAttemptReason::ElfPresetFailed,
            Some(error),
        )
    })?;
    let parsed = parse_load_segments(input, &header, load_bias).map_err(|error| {
        (
            UserInitAttemptStage::SetupElf,
            UserInitAttemptReason::ElfSetupFailed,
            Some(error),
        )
    })?;
    if parsed.load_segment_count == 0 {
        return Err((
            UserInitAttemptStage::SetupElf,
            UserInitAttemptReason::ElfSetupFailed,
            Some(ElfError::MissingLoadSegment),
        ));
    }
    let Some(entry) = header.entry.checked_add(load_bias) else {
        return Err((
            UserInitAttemptStage::SetupElf,
            UserInitAttemptReason::ElfSetupFailed,
            Some(ElfError::InvalidHeader),
        ));
    };
    if !entry_in_executable_segment(entry, &parsed.load_segments, parsed.load_segment_count) {
        return Err((
            UserInitAttemptStage::UnsupportedCandidate,
            UserInitAttemptReason::CandidateUnsupported,
            Some(ElfError::EntryOutsideExecutableSegment),
        ));
    }
    Ok(())
}

#[cfg(app_user_boot)]
fn emit_init_attempt_failure(payload: &mut UserBootPayload, failure: UserInitAttemptFailure) {
    if payload.record_init_attempt_failure(failure).is_err() {
        user_boot_panic("user init failure record failed\n");
    }
    print_init_attempt_failure(failure);
    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::UserBootInitAttemptFailed,
        crate::context::context_ref(),
    );
}

#[cfg(app_user_boot)]
fn print_init_attempt_failure(failure: UserInitAttemptFailure) {
    crate::arch::riscv64::sbi::putstr("user init attempt failed path_index=");
    sbi_put_usize(failure.path().index());
    crate::arch::riscv64::sbi::putstr(" path_len=");
    sbi_put_usize(failure.path_actual_len());
    crate::arch::riscv64::sbi::putstr(" stage=");
    crate::arch::riscv64::sbi::putstr(failure.stage().name());
    crate::arch::riscv64::sbi::putstr(" reason=");
    crate::arch::riscv64::sbi::putstr(failure.reason().name());
    crate::arch::riscv64::sbi::putstr(" requested_terminal=");
    sbi_put_usize(failure.requested_terminal() as usize);
    crate::arch::riscv64::sbi::putstr(" default_nonfatal=");
    sbi_put_usize(failure.default_nonfatal() as usize);
    if let Some(error) = failure.elf_error() {
        crate::arch::riscv64::sbi::putstr(" elf_error=");
        crate::arch::riscv64::sbi::putstr(error.name());
    }
    crate::arch::riscv64::sbi::putstr("\n");
}

#[cfg(app_user_boot)]
fn sbi_put_usize(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;

    if value == 0 {
        crate::arch::riscv64::sbi::putchar(b'0');
        return;
    }

    while value != 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len != 0 {
        len -= 1;
        crate::arch::riscv64::sbi::putchar(digits[len]);
    }
}

#[cfg(app_user_boot)]
fn user_kernel_trap_stack_top() -> usize {
    unsafe {
        let base = core::ptr::addr_of!(USER_KERNEL_TRAP_STACK.bytes) as usize;
        base + USER_KERNEL_TRAP_STACK_SIZE
    }
}

#[cfg(app_user_boot)]
fn user_boot_panic(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
}

#[derive(Clone, Copy)]
struct ElfHeader {
    elf_type: ElfType,
    entry: usize,
    phoff: usize,
    phentsize: usize,
    phnum: usize,
    phdr_vaddr: usize,
    interpreter: Option<(usize, usize)>,
    interpreter_required: bool,
}

struct ParsedLoadSegments {
    load_segments: [ElfLoadSegment; MAX_LOAD_SEGMENTS],
    load_segment_count: usize,
    segment_permissions_bound: bool,
}

fn parse_header(input: &[u8]) -> Result<ElfHeader, ElfError> {
    if input.len() < ELF_HEADER_LEN {
        return Err(ElfError::ShortInput);
    }
    if &input[0..4] != ELF_MAGIC {
        return Err(ElfError::BadMagic);
    }
    if input[4] != ELF_CLASS_64 {
        return Err(ElfError::UnsupportedClass);
    }
    if input[5] != ELF_DATA_LSB {
        return Err(ElfError::UnsupportedEndian);
    }
    if input[6] != ELF_VERSION_CURRENT {
        return Err(ElfError::UnsupportedVersion);
    }
    let elf_type = match read_u16(input, 16)? {
        ELF_TYPE_EXEC => ElfType::Exec,
        ELF_TYPE_DYN => ElfType::Dyn,
        _ => return Err(ElfError::UnsupportedType),
    };
    if read_u16(input, 18)? != ELF_MACHINE_RISCV {
        return Err(ElfError::UnsupportedMachine);
    }
    if read_u32(input, 20)? != ELF_VERSION_CURRENT as u32 {
        return Err(ElfError::UnsupportedVersion);
    }

    let entry = checked_usize(read_u64(input, 24)?)?;
    let phoff = checked_usize(read_u64(input, 32)?)?;
    let ehsize = read_u16(input, 52)? as usize;
    let phentsize = read_u16(input, 54)? as usize;
    let phnum = read_u16(input, 56)? as usize;
    if ehsize != ELF_HEADER_LEN || phentsize != ELF64_PHDR_SIZE || phnum == 0 {
        return Err(ElfError::InvalidHeader);
    }
    if phoff
        .checked_add(
            phentsize
                .checked_mul(phnum)
                .ok_or(ElfError::InvalidHeader)?,
        )
        .filter(|end| *end <= input.len())
        .is_none()
    {
        return Err(ElfError::InvalidHeader);
    }

    let mut phdr_vaddr = 0usize;
    let mut interpreter = None;
    let mut index = 0usize;
    while index < phnum {
        let phdr = phoff + index * phentsize;
        let p_type = read_u32(input, phdr)?;
        if p_type == ELF_PHDR_TYPE_PHDR {
            phdr_vaddr = checked_usize(read_u64(input, phdr + 16)?)?;
        } else if p_type == ELF_PHDR_TYPE_INTERP {
            let offset = checked_usize(read_u64(input, phdr + 8)?)?;
            let filesz = checked_usize(read_u64(input, phdr + 32)?)?;
            if offset
                .checked_add(filesz)
                .filter(|end| *end <= input.len())
                .is_none()
            {
                return Err(ElfError::InvalidProgramHeader);
            }
            interpreter = Some((offset, filesz));
        }
        index += 1;
    }

    Ok(ElfHeader {
        elf_type,
        entry,
        phoff,
        phentsize,
        phnum,
        phdr_vaddr,
        interpreter_required: interpreter.is_some(),
        interpreter,
    })
}

fn elf_type_allowed_for_role(elf_type: ElfType, role: ElfObjectRole) -> bool {
    match role {
        ElfObjectRole::MainExecutable => elf_type == ElfType::Exec || elf_type == ElfType::Dyn,
        ElfObjectRole::Interpreter => elf_type == ElfType::Dyn,
    }
}

fn main_executable_load_bias(header: &ElfHeader) -> Result<usize, ElfError> {
    match header.elf_type {
        ElfType::Exec => Ok(0),
        ElfType::Dyn if header.interpreter_required => Ok(USER_MAIN_PIE_LOAD_BIAS),
        ElfType::Dyn => Err(ElfError::UnsupportedType),
    }
}

fn parse_load_segments(
    input: &[u8],
    header: &ElfHeader,
    load_bias: usize,
) -> Result<ParsedLoadSegments, ElfError> {
    let mut segments = [ElfLoadSegment::empty(); MAX_LOAD_SEGMENTS];
    let mut count = 0usize;
    let mut permissions_bound = true;
    let mut index = 0usize;
    while index < header.phnum {
        let phdr = header.phoff + index * header.phentsize;
        let p_type = read_u32(input, phdr)?;
        if p_type == ELF_PHDR_TYPE_LOAD {
            if count >= MAX_LOAD_SEGMENTS {
                return Err(ElfError::TooManyLoadSegments);
            }
            let flags = read_u32(input, phdr + 4)?;
            let offset = checked_usize(read_u64(input, phdr + 8)?)?;
            let raw_vaddr = checked_usize(read_u64(input, phdr + 16)?)?;
            let vaddr = load_bias
                .checked_add(raw_vaddr)
                .ok_or(ElfError::InvalidProgramHeader)?;
            let filesz = checked_usize(read_u64(input, phdr + 32)?)?;
            let memsz = checked_usize(read_u64(input, phdr + 40)?)?;
            let align = checked_usize(read_u64(input, phdr + 48)?)?;
            if filesz > memsz
                || offset
                    .checked_add(filesz)
                    .filter(|end| *end <= input.len())
                    .is_none()
                || vaddr.checked_add(memsz).is_none()
                || align == 0
            {
                return Err(ElfError::InvalidProgramHeader);
            }
            if flags & (ELF_PF_R | ELF_PF_W | ELF_PF_X) == 0 {
                permissions_bound = false;
            }
            segments[count] = ElfLoadSegment {
                offset,
                vaddr,
                filesz,
                memsz,
                flags,
                align,
            };
            count += 1;
        }
        index += 1;
    }

    Ok(ParsedLoadSegments {
        load_segments: segments,
        load_segment_count: count,
        segment_permissions_bound: permissions_bound,
    })
}

fn phdr_vaddr(
    header: &ElfHeader,
    parsed: &ParsedLoadSegments,
    load_bias: usize,
) -> Result<usize, ElfError> {
    if header.phdr_vaddr != 0 {
        return header
            .phdr_vaddr
            .checked_add(load_bias)
            .ok_or(ElfError::InvalidHeader);
    }
    let phdr_size = header
        .phentsize
        .checked_mul(header.phnum)
        .ok_or(ElfError::InvalidHeader)?;
    let phdr_end = header
        .phoff
        .checked_add(phdr_size)
        .ok_or(ElfError::InvalidHeader)?;
    let mut index = 0usize;
    while index < parsed.load_segment_count {
        let segment = parsed.load_segments[index];
        if header.phoff >= segment.offset() && phdr_end <= segment.file_end() {
            return segment
                .vaddr()
                .checked_add(header.phoff - segment.offset())
                .ok_or(ElfError::InvalidHeader);
        }
        index += 1;
    }
    Err(ElfError::InvalidHeader)
}

fn entry_in_executable_segment(
    entry: usize,
    segments: &[ElfLoadSegment; MAX_LOAD_SEGMENTS],
    count: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        let segment = segments[index];
        if segment.executable() && segment.contains_vaddr(entry) {
            return true;
        }
        index += 1;
    }
    false
}

fn entry_mapping_is_executable(
    mappings: &[UserMapping; MAX_USER_MAPPINGS],
    count: usize,
    entry: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        let mapping = &mappings[index];
        if mapping.kind() == UserMappingKind::ElfSegment
            && mapping.executable()
            && mapping.contains_vaddr(entry)
        {
            return true;
        }
        index += 1;
    }
    false
}

fn materialize_mapping(
    mapping: &mut UserMapping,
    image: &[u8],
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) -> Result<(), ElfError> {
    let page_count = pages_for_range(mapping.page_offset, mapping.memsz)?;
    if page_count > MAX_MAPPING_BACKING_PAGES {
        return Err(ElfError::TooManyMappingPages);
    }
    if mapping
        .file_offset()
        .checked_add(mapping.filesz())
        .filter(|end| *end <= image.len())
        .is_none()
    {
        return Err(ElfError::InvalidProgramHeader);
    }

    let mut index = 0usize;
    while index < page_count {
        let Some(page) = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map) else {
            release_mapping_pages(mapping, page_allocator, page_metadata_map);
            return Err(ElfError::BackingAllocationFailed);
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            release_mapping_pages(mapping, page_allocator, page_metadata_map);
            return Err(ElfError::BackingAllocationFailed);
        };
        unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };
        mapping.backing_pages[index] = Some(page);
        mapping.backing_page_count += 1;
        index += 1;
    }

    if let Err(error) = copy_mapping_bytes(
        mapping,
        image,
        page_metadata_map,
        0,
        mapping.file_offset(),
        mapping.filesz(),
    ) {
        release_mapping_pages(mapping, page_allocator, page_metadata_map);
        return Err(error);
    }
    mapping.file_bytes_copied = mapping.filesz();
    mapping.bss_bytes_zeroed = mapping.bss_zero_bytes();
    mapping.page_table_entry_bound = true;
    Ok(())
}

fn release_mapping_pages(
    mapping: &mut UserMapping,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) {
    if mapping.kind() == UserMappingKind::Stack {
        mapping.clear_backing_pages();
        mapping.backing_page_count = 0;
        mapping.page_table_entry_bound = false;
        return;
    }
    while mapping.backing_page_count > 0 {
        mapping.backing_page_count -= 1;
        if let Some(page) = mapping.backing_pages[mapping.backing_page_count] {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            mapping.backing_pages[mapping.backing_page_count] = None;
        }
    }
}

fn release_mappings(
    mappings: &mut [UserMapping; MAX_USER_MAPPINGS],
    count: usize,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) {
    let mut index = 0usize;
    while index < count {
        release_mapping_pages(&mut mappings[index], page_allocator, page_metadata_map);
        mappings[index].reset_empty();
        index += 1;
    }
}

fn copy_mapping_bytes(
    mapping: &UserMapping,
    image: &[u8],
    page_metadata_map: &PageMetadataMap,
    user_offset: usize,
    file_offset: usize,
    len: usize,
) -> Result<(), ElfError> {
    if file_offset
        .checked_add(len)
        .filter(|end| *end <= image.len())
        .is_none()
        || user_offset
            .checked_add(len)
            .filter(|end| *end <= mapping.memsz())
            .is_none()
    {
        return Err(ElfError::UserCopyOutOfRange);
    }

    let mut remaining = len;
    let mut copied = 0usize;
    while remaining > 0 {
        let absolute = mapping.page_offset() + user_offset + copied;
        let page_index = absolute / USER_PAGE_SIZE;
        let page_offset = absolute % USER_PAGE_SIZE;
        let chunk = min_usize(remaining, USER_PAGE_SIZE - page_offset);
        let Some(page) = mapping.backing_page(page_index) else {
            return Err(ElfError::BackingAllocationFailed);
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            return Err(ElfError::BackingAllocationFailed);
        };
        unsafe {
            core::ptr::copy_nonoverlapping(
                image.as_ptr().add(file_offset + copied),
                (linear + page_offset) as *mut u8,
                chunk,
            );
        }
        copied += chunk;
        remaining -= chunk;
    }
    Ok(())
}

fn materialize_zero_mapping(
    mapping: &mut UserMapping,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) -> Result<(), ElfError> {
    let page_count = pages_for_range(mapping.page_offset, mapping.memsz)?;
    if page_count > MAX_MAPPING_BACKING_PAGES {
        return Err(ElfError::TooManyMappingPages);
    }
    let mut index = 0usize;
    while index < page_count {
        let Some(page) = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map) else {
            release_mapping_pages(mapping, page_allocator, page_metadata_map);
            return Err(ElfError::BackingAllocationFailed);
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            release_mapping_pages(mapping, page_allocator, page_metadata_map);
            return Err(ElfError::BackingAllocationFailed);
        };
        unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };
        mapping.backing_pages[index] = Some(page);
        mapping.backing_page_count += 1;
        index += 1;
    }
    mapping.file_bytes_copied = 0;
    mapping.bss_bytes_zeroed = mapping.bss_zero_bytes();
    mapping.page_table_entry_bound = true;
    Ok(())
}

fn pages_for_range(offset: usize, len: usize) -> Result<usize, ElfError> {
    if offset >= USER_PAGE_SIZE {
        return Err(ElfError::InvalidProgramHeader);
    }
    if len == 0 {
        return Ok(0);
    }
    let end = offset
        .checked_add(len)
        .ok_or(ElfError::InvalidProgramHeader)?;
    Ok((end + USER_PAGE_SIZE - 1) / USER_PAGE_SIZE)
}

fn mappings_have_backing_pages(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if mappings[index].backing_page_count() == 0 {
            return false;
        }
        index += 1;
    }
    true
}

fn mapping_file_bytes_match(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if mappings[index].kind() == UserMappingKind::ElfSegment
            && mappings[index].file_bytes_copied() != mappings[index].filesz()
        {
            return false;
        }
        index += 1;
    }
    true
}

fn mapping_bss_bytes_match(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if mappings[index].kind() == UserMappingKind::ElfSegment
            && mappings[index].bss_bytes_zeroed() != mappings[index].bss_zero_bytes()
        {
            return false;
        }
        index += 1;
    }
    true
}

fn mappings_have_page_table_entries(
    mappings: &[UserMapping; MAX_USER_MAPPINGS],
    count: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        if !mappings[index].page_table_entry_bound() {
            return false;
        }
        index += 1;
    }
    true
}

const fn min_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

fn loadable_content_contains(input: &[u8], parsed: &ParsedLoadSegments, needle: &[u8]) -> bool {
    let mut index = 0usize;
    while index < parsed.load_segment_count {
        let segment = parsed.load_segments[index];
        let haystack = &input[segment.offset..segment.file_end()];
        if contains_bytes(haystack, needle) {
            return true;
        }
        index += 1;
    }
    false
}

pub fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }

    let mut index = 0usize;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            return true;
        }
        index += 1;
    }
    false
}

fn checked_usize(value: u64) -> Result<usize, ElfError> {
    if value > usize::MAX as u64 {
        return Err(ElfError::InvalidHeader);
    }
    Ok(value as usize)
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16, ElfError> {
    let bytes = read_bytes::<2>(input, offset)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32, ElfError> {
    let bytes = read_bytes::<4>(input, offset)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(input: &[u8], offset: usize) -> Result<u64, ElfError> {
    let bytes = read_bytes::<8>(input, offset)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_bytes<const N: usize>(input: &[u8], offset: usize) -> Result<[u8; N], ElfError> {
    let end = offset.checked_add(N).ok_or(ElfError::InvalidHeader)?;
    let slice = input.get(offset..end).ok_or(ElfError::ShortInput)?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    Ok(bytes)
}
