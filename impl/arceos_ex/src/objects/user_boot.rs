use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(app_user_boot)]
use core::sync::atomic::AtomicUsize;

use crate::arch::riscv64::csr;

#[cfg(app_user_boot)]
use super::config::Config;
#[cfg(app_user_boot)]
use super::files::STDIN_READY_FIXTURE;
#[cfg(app_user_boot)]
use super::mm_core::{PageProtection, PageTableCaches, VmallocAllocator, VmapAreaFlags};
pub use super::user_stack::UserStack;
use super::user_stack::UserStackGrowReject;
use super::{elf_object::MAX_LOAD_SEGMENTS, exec_sync_boundaries::ExecSyncBoundaries};

#[allow(unused_imports)]
pub use super::elf_object::{
    ELF_HEADER_LEN, ElfError, ElfLoadSegment, ElfObject, ElfObjectRole, USER_INIT_EXPECTED_MESSAGE,
    USER_INTERPRETER_LOAD_BIAS, USER_MAIN_PIE_LOAD_BIAS, contains_bytes,
};
use super::{
    exception_type::{ExceptionType, SyscallTable},
    files::{FilesStruct, FilesStructSnapshot},
    kernel_image::KernelImage,
    mm_core::{GfpFlags, KernelGlobalAllocator, PageAllocator, PageMetadataMap, PageRef},
    next_generation,
    page_table::{
        PageTablePage, copy_high_half_root_entries, page_table_storage_ready, sv39_indices,
        table_pte_from_phys, user_leaf_pte_from_phys,
    },
    rest_init::KernelInitTask,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_page_tables,
    swapper_vm::SwapperVm,
    task::{Task, TaskEntry, TaskKind, TaskRef, USER_TASK_SLOT_COUNT},
    task_flow::{TaskFlow, TaskFlowRef},
    trap_type::TrapFrame,
    vfs::{FsStruct, FsStructSnapshot},
};

pub const USER_INIT_PATH: &[u8] = b"/sbin/init";
// Alternate candidates are consumed only by the user-boot init-selection path.
#[allow(dead_code)]
pub const USER_ETC_INIT_PATH: &[u8] = b"/etc/init";
#[allow(dead_code)]
pub const USER_BIN_INIT_PATH: &[u8] = b"/bin/init";
#[allow(dead_code)]
pub const USER_BIN_SH_PATH: &[u8] = b"/bin/sh";
#[allow(dead_code)]
const USER_SMOKE_STDIN_MARKER: &[u8] = b"user-smoke: begin";
pub const USER_SIGNAL_COUNT: usize = 64;
pub const USER_CHILD_PID: usize = 3;
pub const USER_CLONE_SIGCHLD: usize = 17;
pub const USER_SIGCHLD_MASK: usize = 1usize << (USER_CLONE_SIGCHLD - 1);
pub const USER_WAIT4_ALL_CHILDREN: usize = usize::MAX;
pub const USER_WAIT4_WUNTRACED: usize = 2;
pub const USER_SIGNAL_WAIT_REASON_NONE: usize = 0;
pub const USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE: usize = 1;

#[cfg_attr(not(app_user_boot), allow(dead_code))]
pub const USER_BOOT_READ_MAX: usize = super::ext2::EXT2_SINGLE_INDIRECT_READ_MAX;
pub const USER_PAGE_SIZE: usize = super::user_stack::USER_PAGE_SIZE;
pub const USER_STACK_SIZE: usize = super::user_stack::USER_STACK_SIZE;
#[allow(dead_code)]
pub const USER_STACK_TOP: usize = super::user_stack::USER_STACK_TOP;
pub const USER_HEAP_BASE: usize = 0x3000_0000;
pub const USER_HEAP_SIZE: usize = 2 * 1024 * 1024;
pub const USER_PARENT_WAIT_STACK_PROBE_LEN: usize = 512;
pub const USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX: usize = 768;
pub const USER_COMPLETED_CHILD_RECORD_CAPACITY: usize = 8;
pub const USER_SUPPLEMENTARY_GROUP_MAX: usize = 1;
#[cfg(app_user_boot)]
pub const USER_KERNEL_TRAP_STACK_ORDER: usize = 2;
#[cfg(app_user_boot)]
pub const USER_KERNEL_TRAP_STACK_SIZE: usize = USER_PAGE_SIZE << USER_KERNEL_TRAP_STACK_ORDER;
#[cfg(app_user_boot)]
pub const USER_KERNEL_TRAP_STACK_ALIGN: usize = USER_KERNEL_TRAP_STACK_SIZE * 2;
#[cfg(app_user_boot)]
pub const USER_KERNEL_TRAP_GUARD_SIZE: usize = USER_PAGE_SIZE;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE: usize =
    super::trap_type::KERNEL_TRAP_OVERFLOW_STACK_SIZE;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_FRAME_SIZE: usize = super::trap_type::TRAP_FRAME_SIZE;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_ENTRY_CONTEXT_SIZE: usize =
    super::trap_type::USER_TRAP_ENTRY_CONTEXT_SIZE;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_THREAD_SHIFT: usize = super::trap_type::KERNEL_TRAP_THREAD_SHIFT;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_IRQ_STACK_SIZE: usize = USER_KERNEL_TRAP_STACK_SIZE;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_THREAD_INFO_IN_TASK: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_VMAP_STACK: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_IRQ_STACKS: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_GUARD_PAGE_READY: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_OVERFLOW_STACK_READY: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_EARLY_OVERFLOW_CHECK_READY: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_EARLY_CHECK_REGISTER_PRESERVING: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_USER_PATH_BIT_TEST_BYPASSED: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_CURRENT_TASK_TP_READY: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_OVERFLOW_FRAME_COMPLETE: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_OVERFLOW_TERMINAL_PANIC: bool = true;
#[cfg(app_user_boot)]
#[allow(dead_code)]
pub const USER_KERNEL_TRAP_IRQ_STACK_SWITCH_DEFERRED: bool = true;

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
pub enum UserRtSigtimedwaitResult {
    ReturnSignal(usize),
    Sleep,
    Unsupported,
}

#[derive(Clone, Copy)]
struct UserSignalWaitQueue {
    waiter_enqueued: bool,
    waiter_finished: bool,
    wake_sigchld_committed: bool,
}

impl UserSignalWaitQueue {
    const fn new() -> Self {
        Self {
            waiter_enqueued: false,
            waiter_finished: false,
            wake_sigchld_committed: false,
        }
    }

    const fn waiter_enqueued(&self) -> bool {
        self.waiter_enqueued
    }

    const fn waiter_finished(&self) -> bool {
        self.waiter_finished
    }

    const fn wake_sigchld_committed(&self) -> bool {
        self.wake_sigchld_committed
    }

    fn prepare_wait(&mut self) {
        self.waiter_enqueued = true;
        self.waiter_finished = false;
        self.wake_sigchld_committed = false;
    }

    fn wake_sigchld(&mut self) {
        self.wake_sigchld_committed = true;
    }

    fn finish_wait(&mut self) {
        self.waiter_finished = true;
        self.waiter_enqueued = false;
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
    UpdatedPendingChild(usize),
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

pub const fn sigchld_mask_matches(mask: usize) -> bool {
    mask & USER_SIGCHLD_MASK != 0
}

#[cfg(app_user_boot)]
const USER_INIT_CANDIDATES: [UserInitPathRef; 4] = [
    UserInitPathRef::DefaultInit,
    UserInitPathRef::EtcInit,
    UserInitPathRef::BinInit,
    UserInitPathRef::BinSh,
];
const USER_SELECTED_PATH_MAX: usize = 128;
const MAX_USER_MAPPINGS: usize = MAX_LOAD_SEGMENTS * 2 + 16;
const MAX_MAPPING_BACKING_PAGES: usize = 512;
const MAX_USER_L0_TABLES: usize = MAX_USER_MAPPINGS + 2;
const USER_CLONE_CSIGNAL_MASK: usize = 0xff;
const USER_CLONE_VM: usize = 0x0000_0100;
const USER_CLONE_PIDFD: usize = 0x0000_1000;
const USER_CLONE_VFORK: usize = 0x0000_4000;
const USER_CLONE_SETTLS: usize = 0x0008_0000;

#[derive(Clone, Copy)]
struct UserCompletedChildRecord {
    occupied: bool,
    task_ref: TaskRef,
    pid: usize,
    exit_status: usize,
    wait_status: usize,
    pidfd_fd: usize,
    reaped: bool,
}

impl UserCompletedChildRecord {
    const fn empty() -> Self {
        Self {
            occupied: false,
            task_ref: TaskRef::NONE,
            pid: 0,
            exit_status: 0,
            wait_status: 0,
            pidfd_fd: usize::MAX,
            reaped: false,
        }
    }

    const fn archived(task_ref: TaskRef, pid: usize, exit_status: usize, pidfd_fd: usize) -> Self {
        Self {
            occupied: true,
            task_ref,
            pid,
            exit_status,
            wait_status: (exit_status & 0xff) << 8,
            pidfd_fd,
            reaped: false,
        }
    }
}

struct UserTaskStorageSlot {
    generation: u32,
    flow_generation: u32,
    occupied: bool,
    pid: usize,
    task: Task,
    runtime: UserAppRuntime,
}

impl UserTaskStorageSlot {
    const fn new(_slot: usize) -> Self {
        Self {
            generation: 0,
            flow_generation: 0,
            occupied: false,
            pid: 0,
            task: Task::new(),
            runtime: UserAppRuntime::new(),
        }
    }

    const fn task_ref(&self) -> TaskRef {
        self.task.task_ref()
    }

    const fn state(&self) -> State {
        self.task.state()
    }
}

fn task_event_or_terminate(result: EventResult) {
    if result.is_err() {
        panic!("declared Task/TaskFlow invariant failed");
    }
}

pub struct UserCloneDeferredBoundaries {
    lifecycle: Lifecycle,
    linux_6_12_legacy_clone_bound: bool,
    riscv_abi_argument_order_bound: bool,
    observed_plain_fork_args_bound: bool,
    observed_vfork_vm_args_bound: bool,
    observed_vfork_pidfd_args_bound: bool,
    plain_fork_first_slice_bound: bool,
    vfork_vm_first_slice_bound: bool,
    vfork_pidfd_first_slice_bound: bool,
    csignal_split_bound: bool,
    sigchld_exit_signal_bound: bool,
    newsp_zero_inherits_parent_sp: bool,
    newsp_sets_child_sp: bool,
    legacy_pidfd_uses_parent_tidptr: bool,
    tls_ignored_without_clone_settls: bool,
    thread_group_deferred: bool,
    clone_vm_vfork_deferred: bool,
    cow_mm_deferred: bool,
    full_pidfd_file_ops_deferred: bool,
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
            observed_vfork_vm_args_bound: false,
            observed_vfork_pidfd_args_bound: false,
            plain_fork_first_slice_bound: false,
            vfork_vm_first_slice_bound: false,
            vfork_pidfd_first_slice_bound: false,
            csignal_split_bound: false,
            sigchld_exit_signal_bound: false,
            newsp_zero_inherits_parent_sp: false,
            newsp_sets_child_sp: false,
            legacy_pidfd_uses_parent_tidptr: false,
            tls_ignored_without_clone_settls: false,
            thread_group_deferred: true,
            clone_vm_vfork_deferred: true,
            cow_mm_deferred: true,
            full_pidfd_file_ops_deferred: true,
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

    pub const fn vfork_pidfd_first_slice_bound(&self) -> bool {
        self.vfork_pidfd_first_slice_bound
    }

    pub const fn vfork_vm_first_slice_bound(&self) -> bool {
        self.vfork_vm_first_slice_bound
    }

    pub const fn wait_exit_reap_deferred(&self) -> bool {
        self.wait_exit_reap_deferred
    }

    pub fn setup(&mut self, exec_sync: &ExecSyncBoundaries) -> EventResult {
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
        self.observed_vfork_vm_args_bound = true;
        self.observed_vfork_pidfd_args_bound = true;
        self.plain_fork_first_slice_bound = true;
        self.vfork_vm_first_slice_bound = true;
        self.vfork_pidfd_first_slice_bound = true;
        self.csignal_split_bound = true;
        self.sigchld_exit_signal_bound = true;
        self.newsp_zero_inherits_parent_sp = true;
        self.newsp_sets_child_sp = true;
        self.legacy_pidfd_uses_parent_tidptr = true;
        self.tls_ignored_without_clone_settls = true;
        self.thread_group_deferred = true;
        self.clone_vm_vfork_deferred = true;
        self.cow_mm_deferred = true;
        self.full_pidfd_file_ops_deferred = true;
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

    pub fn accepts_vfork_pidfd_first_slice(&self, clone_flags: usize) -> bool {
        self.lifecycle.state() == State::Ready
            && self.linux_6_12_legacy_clone_bound
            && self.riscv_abi_argument_order_bound
            && self.vfork_pidfd_first_slice_bound
            && self.legacy_pidfd_uses_parent_tidptr
            && self.clone_flags_without_csignal(clone_flags)
                == (USER_CLONE_PIDFD | USER_CLONE_VFORK)
            && self.exit_signal(clone_flags) == USER_CLONE_SIGCHLD
            && self.tls_inherited_without_clone_settls(clone_flags)
    }

    pub fn accepts_vfork_vm_first_slice(&self, clone_flags: usize) -> bool {
        self.lifecycle.state() == State::Ready
            && self.linux_6_12_legacy_clone_bound
            && self.riscv_abi_argument_order_bound
            && self.vfork_vm_first_slice_bound
            && self.clone_flags_without_csignal(clone_flags) == (USER_CLONE_VM | USER_CLONE_VFORK)
            && self.exit_signal(clone_flags) == USER_CLONE_SIGCHLD
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
    // Stable indices and paths are emitted by optional user-boot diagnostics.
    #[allow(dead_code)]
    pub const fn index(self) -> usize {
        match self {
            Self::DefaultInit => 0,
            Self::EtcInit => 1,
            Self::BinInit => 2,
            Self::BinSh => 3,
            Self::RequestedInit => 4,
        }
    }

    #[allow(dead_code)]
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

// Non-None stages are constructed only by the user-boot init-selection path.
#[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

// Non-None reasons are constructed only by the user-boot init-selection path.
#[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

// The complete failure record is consumed by optional user-boot diagnostics.
#[allow(dead_code)]
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

#[allow(dead_code)]
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

static USER_INIT_RUNTIME_ENTERED: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserMappingKind {
    Empty,
    ElfSegment,
    Stack,
    Heap,
    AnonymousPrivate,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserFaultAccess {
    Instruction,
    Load,
    Store,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserFaultClass {
    NotPresent,
    #[allow(dead_code)]
    CowWriteProtect,
    Protection,
    Unmapped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserFaultResult {
    RetrySameInstruction,
    SegvMaperr,
    SegvAccerr,
    TaskOom,
    InvalidContext,
}

#[derive(Clone, Copy)]
pub struct UserFaultRequest {
    task_ref: TaskRef,
    mm_satp: usize,
    address: usize,
    sepc: usize,
    access: UserFaultAccess,
}

impl UserFaultRequest {
    pub const fn new(
        task_ref: TaskRef,
        mm_satp: usize,
        address: usize,
        sepc: usize,
        access: UserFaultAccess,
    ) -> Self {
        Self {
            task_ref,
            mm_satp,
            address,
            sepc,
            access,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserFaultResolution {
    class: UserFaultClass,
    result: UserFaultResult,
    sepc: usize,
}

impl UserFaultResolution {
    const fn new(class: UserFaultClass, result: UserFaultResult, sepc: usize) -> Self {
        Self {
            class,
            result,
            sepc,
        }
    }

    #[allow(dead_code)]
    pub const fn class(self) -> UserFaultClass {
        self.class
    }

    #[allow(dead_code)]
    pub const fn result(self) -> UserFaultResult {
        self.result
    }

    pub const fn sepc(self) -> usize {
        self.sepc
    }

    pub const fn retry_same_instruction(self) -> bool {
        matches!(self.result, UserFaultResult::RetrySameInstruction)
    }
}

#[derive(Clone, Copy)]
pub struct UserFaultDiagnostic {
    task_ref: TaskRef,
    mm_satp: usize,
    address: usize,
    sepc: usize,
    access: UserFaultAccess,
    class: UserFaultClass,
    result: UserFaultResult,
    cow_shared_count: usize,
    cow_copied_count: usize,
}

impl UserFaultDiagnostic {
    const fn empty() -> Self {
        Self {
            task_ref: TaskRef::NONE,
            mm_satp: 0,
            address: 0,
            sepc: 0,
            access: UserFaultAccess::Unknown,
            class: UserFaultClass::Unmapped,
            result: UserFaultResult::InvalidContext,
            cow_shared_count: 0,
            cow_copied_count: 0,
        }
    }

    const fn from_request(
        request: UserFaultRequest,
        class: UserFaultClass,
        result: UserFaultResult,
    ) -> Self {
        Self {
            task_ref: request.task_ref,
            mm_satp: request.mm_satp,
            address: request.address,
            sepc: request.sepc,
            access: request.access,
            class,
            result,
            cow_shared_count: 0,
            cow_copied_count: 0,
        }
    }

    pub const fn class(self) -> UserFaultClass {
        self.class
    }

    pub const fn task_ref(self) -> TaskRef {
        self.task_ref
    }

    pub const fn mm_satp(self) -> usize {
        self.mm_satp
    }

    pub const fn address(self) -> usize {
        self.address
    }

    pub const fn sepc(self) -> usize {
        self.sepc
    }

    pub const fn access(self) -> UserFaultAccess {
        self.access
    }

    pub const fn result(self) -> UserFaultResult {
        self.result
    }

    pub const fn cow_shared_count(self) -> usize {
        self.cow_shared_count
    }

    pub const fn cow_copied_count(self) -> usize {
        self.cow_copied_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserStackFaultResolution {
    fault_page: usize,
    old_base: usize,
    new_base: usize,
    expanded: bool,
}

#[allow(dead_code)]
impl UserStackFaultResolution {
    pub const fn fault_page(self) -> usize {
        self.fault_page
    }

    pub const fn old_base(self) -> usize {
        self.old_base
    }

    pub const fn new_base(self) -> usize {
        self.new_base
    }

    pub const fn expanded(self) -> bool {
        self.expanded
    }
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
    backing_vaddrs: [usize; MAX_MAPPING_BACKING_PAGES],
    backing_page_count: usize,
    file_bytes_copied: usize,
    bss_bytes_zeroed: usize,
    page_table_entry_bound: bool,
    stack_ownership_token: usize,
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
            backing_vaddrs: [0; MAX_MAPPING_BACKING_PAGES],
            backing_page_count: 0,
            file_bytes_copied: 0,
            bss_bytes_zeroed: 0,
            page_table_entry_bound: false,
            stack_ownership_token: 0,
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
        self.stack_ownership_token = 0;
    }

    fn clear_backing_pages(&mut self) {
        let mut index = 0usize;
        while index < MAX_MAPPING_BACKING_PAGES {
            self.backing_pages[index] = None;
            self.backing_vaddrs[index] = 0;
            index += 1;
        }
    }

    fn init_segment(&mut self, segment: ElfLoadSegment) {
        self.reset_empty();
        self.kind = UserMappingKind::ElfSegment;
        self.vaddr = segment.vaddr();
        self.memsz = segment.memsz();
        self.page_offset = segment.vaddr() % USER_PAGE_SIZE;
        self.file_offset = segment.offset();
        self.filesz = segment.filesz();
        self.readable = segment.readable();
        self.writable = segment.writable();
        self.executable = segment.executable();
        self.user_accessible = true;
        self.bss_zero_bytes = segment.memsz() - segment.filesz();
    }

    fn init_stack(&mut self, stack: &UserStack) {
        self.reset_empty();
        self.kind = UserMappingKind::Stack;
        self.vaddr = stack.base();
        self.memsz = stack.size();
        self.readable = true;
        self.writable = true;
        self.user_accessible = true;
        self.bss_zero_bytes = stack.size();
        self.bss_bytes_zeroed = stack.size();
        self.page_table_entry_bound = stack.backing_pages_allocated();
        self.stack_ownership_token = stack.top();
    }

    fn init_heap(&mut self) {
        self.reset_empty();
        self.kind = UserMappingKind::Heap;
        self.vaddr = USER_HEAP_BASE;
        self.memsz = 0;
        self.readable = true;
        self.writable = true;
        self.user_accessible = true;
        self.page_table_entry_bound = true;
    }

    fn init_anonymous_private(
        &mut self,
        vaddr: usize,
        memsz: usize,
        readable: bool,
        writable: bool,
        executable: bool,
    ) {
        self.reset_empty();
        self.kind = UserMappingKind::AnonymousPrivate;
        self.vaddr = vaddr;
        self.memsz = memsz;
        self.readable = readable;
        self.writable = writable;
        self.executable = executable;
        self.user_accessible = true;
        self.bss_zero_bytes = memsz;
        self.bss_bytes_zeroed = memsz;
        self.page_table_entry_bound = true;
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

    pub const fn backing_page_vaddr(&self, index: usize) -> Option<usize> {
        if index < self.backing_page_count {
            Some(self.backing_vaddrs[index])
        } else {
            None
        }
    }

    fn backing_page_index(&self, vaddr: usize) -> Option<usize> {
        let mut index = 0usize;
        while index < self.backing_page_count {
            if self.backing_vaddrs[index] == vaddr {
                return Some(index);
            }
            index += 1;
        }
        None
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

    pub const fn stack_ownership_token(&self) -> usize {
        self.stack_ownership_token
    }

    pub const fn end_vaddr(&self) -> usize {
        self.vaddr + self.memsz
    }

    const fn contains_vaddr(&self, addr: usize) -> bool {
        self.vaddr <= addr && addr < self.end_vaddr()
    }

    fn mapped_start(&self) -> Option<usize> {
        self.vaddr.checked_sub(self.page_offset)
    }

    fn mapped_end(&self) -> Option<usize> {
        let start = self.mapped_start()?;
        let bytes = align_up_checked(self.page_offset.checked_add(self.memsz)?, USER_PAGE_SIZE)?;
        start.checked_add(bytes)
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
    last_user_fault: UserFaultDiagnostic,
    #[cfg(app_smoke)]
    fail_next_stack_pte_install: bool,
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
            last_user_fault: UserFaultDiagnostic::empty(),
            #[cfg(app_smoke)]
            fail_next_stack_pte_install: false,
        }
    }

    fn reset_staging_metadata(&mut self) {
        self.lifecycle = Lifecycle::new(State::Base);
        self.allocated = false;
        self.first_instance = false;
        self.bound_to_kernel_init_task = false;
        self.low_half_private = false;
        self.high_half_shares_swapper = false;
        self.kernel_pages_u_disabled = false;
        self.user_pages_u_enabled = false;
        self.elf_load_plan_consumed = false;
        self.segment_mappings_bound = false;
        self.entry_mapping_executable = false;
        self.bss_zero_plan_consumed = false;
        self.backing_pages_allocated = false;
        self.elf_file_bytes_copied = false;
        self.bss_bytes_zeroed = false;
        self.page_table_view_ready = false;
        self.elf_segments_mapped = false;
        self.stack_mapped = false;
        self.heap_mapped = false;
        self.elf_mapped = false;
        self.elf_bss_zeroed = false;
        self.runtime_ready = false;
        self.real_page_table_allocated = false;
        self.user_leaf_ptes_installed = false;
        self.high_half_root_entries_shared = false;
        self.satp_token_ready = false;
        self.prepared_but_not_current = false;
        self.satp_token = 0;
        self.user_leaf_pte_count = 0;
        self.page_table_root = None;
        self.page_table_l1 = None;
        let mut index = 0usize;
        while index < MAX_USER_L0_TABLES {
            self.page_table_l0s[index] = UserL0TableSlot::empty();
            index += 1;
        }
        self.page_table_l0_count = 0;
        self.reset_mappings();
        self.mapping_count = 0;
        self.segment_mapping_count = 0;
        self.stack_mapping_index = 0;
        self.heap_mapping_index = 0;
        self.heap_base = 0;
        self.heap_size = 0;
        self.heap_brk = 0;
        self.mmap_next = 0;
        self.last_user_fault = UserFaultDiagnostic::empty();
        #[cfg(app_smoke)]
        {
            self.fail_next_stack_pte_install = false;
        }
    }

    pub fn reset_staging_after_exec_commit(&mut self) {
        self.reset_staging_metadata();
    }

    pub fn discard_staging_after_exec_failure(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) {
        release_mappings(
            &mut self.mappings,
            self.mapping_count,
            page_allocator,
            page_metadata_map,
        );
        self.release_page_table_pages(page_allocator, page_metadata_map);
        self.reset_staging_metadata();
    }

    pub fn release_current_user_backing_for_exec_replacement(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> usize {
        let released_pages = total_mapping_page_count(&self.mappings, self.mapping_count);
        release_mappings(
            &mut self.mappings,
            self.mapping_count,
            page_allocator,
            page_metadata_map,
        );
        self.mapping_count = 0;
        self.segment_mapping_count = 0;
        self.stack_mapping_index = 0;
        self.heap_mapping_index = 0;
        self.backing_pages_allocated = false;
        self.elf_file_bytes_copied = false;
        self.bss_bytes_zeroed = false;
        self.page_table_view_ready = false;
        self.elf_segments_mapped = false;
        self.stack_mapped = false;
        self.heap_mapped = false;
        self.elf_mapped = false;
        self.elf_bss_zeroed = false;
        self.heap_base = 0;
        self.heap_size = 0;
        self.heap_brk = 0;
        self.mmap_next = 0;
        released_pages
    }

    pub fn release_retired_exec_image(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> usize {
        let released_pages = total_mapping_page_count(&self.mappings, self.mapping_count)
            + usize::from(self.page_table_root.is_some())
            + usize::from(self.page_table_l1.is_some())
            + self.page_table_l0_count;
        release_mappings(
            &mut self.mappings,
            self.mapping_count,
            page_allocator,
            page_metadata_map,
        );
        self.release_page_table_pages(page_allocator, page_metadata_map);
        self.reset_staging_metadata();
        released_pages
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

    pub const fn last_user_fault(&self) -> UserFaultDiagnostic {
        self.last_user_fault
    }

    fn vma_overlaps(&self, start: usize, end: usize) -> bool {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = &self.mappings[index];
            let Some(mapping_start) = mapping.mapped_start() else {
                index += 1;
                continue;
            };
            let Some(mapping_end) = mapping.mapped_end() else {
                index += 1;
                continue;
            };
            if mapping.kind() != UserMappingKind::Empty
                && mapping.memsz() != 0
                && start < mapping_end
                && end > mapping_start
            {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn preset(
        &mut self,
        swapper_vm: &SwapperVm,
        page_allocator: &PageAllocator,
        global_allocator: &KernelGlobalAllocator,
        kernel_init_task: &KernelInitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || swapper_vm.state() != State::Ready
            || page_allocator.state() != State::Ready
            || global_allocator.state() != State::Ready
            || !kernel_init_task.task().online()
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

    // Address-space setup keeps ELF, interpreter, stack and allocator inputs explicit.
    #[allow(clippy::too_many_arguments)]
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
        let mapping = &mut self.mappings[self.heap_mapping_index];
        mapping.memsz = requested - self.heap_base;
        mapping.bss_zero_bytes = mapping.memsz;
        mapping.bss_bytes_zeroed = mapping.memsz;
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
        const PROT_READ: usize = 0x1;
        const PROT_WRITE: usize = 0x2;
        const PROT_EXEC: usize = 0x4;
        const SUPPORTED_PROT: usize = PROT_READ | PROT_WRITE | PROT_EXEC;
        const MAP_PRIVATE: usize = 0x02;
        const MAP_FIXED: usize = 0x10;
        const MAP_ANONYMOUS: usize = 0x20;
        const SUPPORTED_FLAGS: usize = MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS;
        if !offset.is_multiple_of(USER_PAGE_SIZE) {
            return Err(UserMmapError::Invalid);
        }
        if prot & !SUPPORTED_PROT != 0 {
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
                && addr.is_multiple_of(USER_PAGE_SIZE)
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
            self.first_available_mmap_range(mmap_start, heap_end, len)
                .ok_or(UserMmapError::NoMemory)?
        };
        let end = base.checked_add(len).ok_or(UserMmapError::NoMemory)?;
        if base < mmap_start || end > heap_end {
            return Err(UserMmapError::NoMemory);
        }
        if self.vma_overlaps(base, end) {
            return Err(UserMmapError::NoMemory);
        }
        let mapping_index = self
            .first_empty_mapping_slot()
            .or_else(|| (self.mapping_count < MAX_USER_MAPPINGS).then_some(self.mapping_count))
            .ok_or(UserMmapError::NoMemory)?;
        self.mappings[mapping_index].init_anonymous_private(
            base,
            len,
            prot & PROT_READ != 0,
            prot & PROT_WRITE != 0,
            prot & PROT_EXEC != 0,
        );
        if mapping_index == self.mapping_count {
            self.mapping_count += 1;
        }
        if addr == 0 {
            self.mmap_next = end;
        }
        Ok(base)
    }

    pub fn user_mprotect(&self, addr: usize, len: usize) -> bool {
        self.user_range_mapped(addr, len)
    }

    pub fn user_munmap(
        &mut self,
        addr: usize,
        len: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if self.lifecycle.state() != State::Online
            || len == 0
            || !addr.is_multiple_of(USER_PAGE_SIZE)
        {
            return false;
        }
        let Some(len) = align_up_checked(len, USER_PAGE_SIZE) else {
            return false;
        };
        let Some(end) = addr.checked_add(len) else {
            return false;
        };
        let mut mapping_index = 0usize;
        while mapping_index < self.mapping_count {
            let mapping = &self.mappings[mapping_index];
            if mapping.kind() == UserMappingKind::AnonymousPrivate
                && mapping.mapped_start() == Some(addr)
                && mapping.mapped_end() == Some(end)
            {
                break;
            }
            mapping_index += 1;
        }
        if mapping_index == self.mapping_count {
            return false;
        }

        let backing_page_count = self.mappings[mapping_index].backing_page_count();
        let mut page_index = 0usize;
        while page_index < backing_page_count {
            let Some(page) = self.mappings[mapping_index].backing_page(page_index) else {
                return false;
            };
            let Some(virt) = self.mappings[mapping_index].backing_page_vaddr(page_index) else {
                return false;
            };
            if !self.user_leaf_pte_maps_page(virt, page.phys().value(), page_metadata_map) {
                return false;
            }
            page_index += 1;
        }

        page_index = 0;
        while page_index < backing_page_count {
            let page = self.mappings[mapping_index]
                .backing_page(page_index)
                .expect("prevalidated anonymous backing page must remain present");
            let virt = self.mappings[mapping_index]
                .backing_page_vaddr(page_index)
                .expect("prevalidated anonymous backing VA must remain present");
            if !self.clear_user_leaf_pte(virt, page.phys().value(), page_metadata_map) {
                return false;
            }
            crate::arch::riscv64::csr::sfence_vma_addr(virt);
            page_index += 1;
        }
        release_mapping_pages(
            &mut self.mappings[mapping_index],
            page_allocator,
            page_metadata_map,
        );
        self.mappings[mapping_index].reset_empty();
        while self.mapping_count > 0
            && self.mappings[self.mapping_count - 1].kind() == UserMappingKind::Empty
        {
            self.mapping_count -= 1;
        }
        self.mmap_next = self.mmap_next.min(addr);
        true
    }

    fn first_empty_mapping_slot(&self) -> Option<usize> {
        let mut index = 0usize;
        while index < self.mapping_count {
            if self.mappings[index].kind() == UserMappingKind::Empty {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn first_available_mmap_range(
        &self,
        mmap_start: usize,
        heap_end: usize,
        len: usize,
    ) -> Option<usize> {
        let mut base = mmap_start;
        loop {
            let end = base.checked_add(len).filter(|end| *end <= heap_end)?;
            let mut next_base = 0usize;
            let mut index = 0usize;
            while index < self.mapping_count {
                let mapping = &self.mappings[index];
                if let (Some(mapping_start), Some(mapping_end)) =
                    (mapping.mapped_start(), mapping.mapped_end())
                    && base < mapping_end
                    && end > mapping_start
                {
                    next_base = next_base.max(mapping_end);
                }
                index += 1;
            }
            if next_base == 0 {
                return Some(base);
            }
            base = align_up_checked(next_base, USER_PAGE_SIZE)?;
        }
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
            let Some(mapping_start) = mapping.mapped_start() else {
                index += 1;
                continue;
            };
            let Some(mapping_end) = mapping.mapped_end() else {
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
            let Some(mapping_start) = mapping.mapped_start() else {
                index += 1;
                continue;
            };
            let Some(mapping_end) = mapping.mapped_end() else {
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

    fn mapping_index_for_address(&self, addr: usize) -> Option<usize> {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = &self.mappings[index];
            let start = mapping.mapped_start()?;
            let end = mapping.mapped_end()?;
            if mapping.kind() != UserMappingKind::Empty && addr >= start && addr < end {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn record_user_fault(
        &mut self,
        request: UserFaultRequest,
        class: UserFaultClass,
        result: UserFaultResult,
    ) -> UserFaultResolution {
        self.last_user_fault = UserFaultDiagnostic::from_request(request, class, result);
        UserFaultResolution::new(class, result, request.sepc)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_user_fault(
        &mut self,
        stack: &mut UserStack,
        request: UserFaultRequest,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> UserFaultResolution {
        if self.lifecycle.state() != State::Online
            || stack.state() != State::Ready
            || request.task_ref == TaskRef::NONE
            || request.mm_satp == 0
            || request.mm_satp != self.satp_token
            || matches!(request.access, UserFaultAccess::Unknown)
        {
            return self.record_user_fault(
                request,
                UserFaultClass::Protection,
                UserFaultResult::InvalidContext,
            );
        }

        let stack_growth_candidate = matches!(
            request.access,
            UserFaultAccess::Load | UserFaultAccess::Store
        ) && request.address >= stack.rlimit_base()
            && request.address < stack.top();
        let mapping_index = self.mapping_index_for_address(request.address);
        if mapping_index.is_none() && !stack_growth_candidate {
            return self.record_user_fault(
                request,
                UserFaultClass::Unmapped,
                UserFaultResult::SegvMaperr,
            );
        }

        if stack_growth_candidate
            && mapping_index.is_none_or(|index| index == self.stack_mapping_index)
        {
            let result = self.resolve_user_stack_fault(
                stack,
                request.address,
                request.access,
                request.mm_satp,
                page_allocator,
                page_metadata_map,
            );
            let outcome = match result {
                Ok(_) => UserFaultResult::RetrySameInstruction,
                Err(
                    UserStackGrowReject::BackingAllocation
                    | UserStackGrowReject::PageTableAllocation
                    | UserStackGrowReject::PteInstall,
                ) => UserFaultResult::TaskOom,
                Err(UserStackGrowReject::Rlimit | UserStackGrowReject::MappingCollision) => {
                    UserFaultResult::SegvMaperr
                }
                Err(_) => UserFaultResult::SegvAccerr,
            };
            return self.record_user_fault(request, UserFaultClass::NotPresent, outcome);
        }

        let Some(mapping_index) = mapping_index else {
            return self.record_user_fault(
                request,
                UserFaultClass::Unmapped,
                UserFaultResult::SegvMaperr,
            );
        };
        let mapping = &self.mappings[mapping_index];
        let permitted = mapping.user_accessible()
            && match request.access {
                UserFaultAccess::Instruction => mapping.executable(),
                UserFaultAccess::Load => mapping.readable(),
                UserFaultAccess::Store => mapping.writable(),
                UserFaultAccess::Unknown => false,
            };
        if !permitted {
            return self.record_user_fault(
                request,
                UserFaultClass::Protection,
                UserFaultResult::SegvAccerr,
            );
        }

        let fault_page = align_down(request.address, USER_PAGE_SIZE);
        if mapping.kind() == UserMappingKind::Stack
            || mapping.backing_page_index(fault_page).is_some()
        {
            return self.record_user_fault(
                request,
                UserFaultClass::Protection,
                UserFaultResult::SegvAccerr,
            );
        }
        if !matches!(
            mapping.kind(),
            UserMappingKind::Heap | UserMappingKind::AnonymousPrivate
        ) {
            return self.record_user_fault(
                request,
                UserFaultClass::Protection,
                UserFaultResult::SegvAccerr,
            );
        }

        let outcome = if self.fault_in_sparse_mapping_page(
            mapping_index,
            fault_page,
            page_allocator,
            page_metadata_map,
        ) {
            crate::arch::riscv64::csr::sfence_vma_addr(fault_page);
            UserFaultResult::RetrySameInstruction
        } else {
            UserFaultResult::TaskOom
        };
        self.record_user_fault(request, UserFaultClass::NotPresent, outcome)
    }

    fn fault_in_sparse_mapping_page(
        &mut self,
        mapping_index: usize,
        fault_page: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if mapping_index >= self.mapping_count
            || self.mappings[mapping_index].backing_page_count >= MAX_MAPPING_BACKING_PAGES
        {
            return false;
        }
        let Some(page) = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map) else {
            return false;
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            return false;
        };
        unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };

        let slot = self.mappings[mapping_index].backing_page_count;
        self.mappings[mapping_index].backing_pages[slot] = Some(page);
        self.mappings[mapping_index].backing_vaddrs[slot] = fault_page;
        self.mappings[mapping_index].backing_page_count += 1;
        let old_l0_count = self.page_table_l0_count;
        let mapping = &self.mappings[mapping_index];
        if !self.install_user_leaf_pte(
            fault_page,
            page.phys().value(),
            mapping.readable(),
            mapping.writable(),
            mapping.executable(),
            page_allocator,
            page_metadata_map,
        ) {
            self.rollback_l0_tables(old_l0_count, page_allocator, page_metadata_map);
            let mapping = &mut self.mappings[mapping_index];
            mapping.backing_page_count -= 1;
            mapping.backing_pages[slot] = None;
            mapping.backing_vaddrs[slot] = 0;
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            return false;
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_user_fault_range(
        &mut self,
        stack: &mut UserStack,
        task_ref: TaskRef,
        addr: usize,
        len: usize,
        access: UserFaultAccess,
        current_satp: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if len == 0 {
            return true;
        }
        let Some(end) = addr.checked_add(len) else {
            return false;
        };
        let mut page = align_down(addr, USER_PAGE_SIZE);
        while page < end {
            let stack_page_present =
                page >= stack.base() && page < stack.top() && stack.page_for_vaddr(page).is_some();
            let mapping_page_present = self.mapping_index_for_address(page).is_some_and(|index| {
                self.mappings[index].kind() != UserMappingKind::Stack
                    && self.mappings[index].backing_page_index(page).is_some()
            });
            if !stack_page_present && !mapping_page_present {
                let request = UserFaultRequest::new(task_ref, current_satp, page, 0, access);
                if !self
                    .resolve_user_fault(stack, request, page_allocator, page_metadata_map)
                    .retry_same_instruction()
                {
                    return false;
                }
            }
            let Some(next) = page.checked_add(USER_PAGE_SIZE) else {
                return false;
            };
            page = next;
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_user_stack_fault(
        &mut self,
        stack: &mut UserStack,
        fault_addr: usize,
        access: UserFaultAccess,
        current_satp: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<UserStackFaultResolution, UserStackGrowReject> {
        let old_base = stack.base();
        let reject = |stack: &mut UserStack, reason| {
            stack.record_grow_rejected(fault_addr, old_base, reason);
            Err(reason)
        };
        if self.lifecycle.state() != State::Online || stack.state() != State::Ready {
            return reject(stack, UserStackGrowReject::InvalidState);
        }
        if current_satp == 0 || current_satp != self.satp_token {
            return reject(stack, UserStackGrowReject::WrongAddressSpace);
        }
        if !matches!(access, UserFaultAccess::Load | UserFaultAccess::Store) {
            return reject(stack, UserStackGrowReject::UnsupportedAccess);
        }
        if self.stack_mapping_index >= self.mapping_count {
            return reject(stack, UserStackGrowReject::InvalidState);
        }
        let mapping = &self.mappings[self.stack_mapping_index];
        if mapping.kind() != UserMappingKind::Stack
            || mapping.stack_ownership_token() != stack.top()
            || !mapping.user_accessible()
            || (access == UserFaultAccess::Load && !mapping.readable())
            || (access == UserFaultAccess::Store && !mapping.writable())
            || mapping.executable()
        {
            return reject(stack, UserStackGrowReject::Permission);
        }

        let fault_page = align_down(fault_addr, USER_PAGE_SIZE);
        if fault_addr >= stack.top() || fault_page < stack.rlimit_base() {
            return reject(stack, UserStackGrowReject::Rlimit);
        }
        let new_base = if fault_page < old_base {
            let mut index = 0usize;
            while index < self.mapping_count {
                if index != self.stack_mapping_index {
                    let other = &self.mappings[index];
                    if other.kind() != UserMappingKind::Empty {
                        let other_start = other.mapped_start().unwrap_or(other.vaddr());
                        let other_end = other.mapped_end().unwrap_or(other.end_vaddr());
                        if fault_page < other_end && fault_page + USER_PAGE_SIZE > other_start {
                            return reject(stack, UserStackGrowReject::MappingCollision);
                        }
                        if other_end <= old_base {
                            let guard_end = other_end.saturating_add(stack.config().guard_gap());
                            if fault_page < guard_end {
                                return reject(stack, UserStackGrowReject::GuardGap);
                            }
                        }
                    }
                }
                index += 1;
            }
            fault_page
        } else if fault_page < stack.top() {
            old_base
        } else {
            return reject(stack, UserStackGrowReject::Rlimit);
        };

        if stack.page_for_vaddr(fault_page).is_some() {
            return reject(stack, UserStackGrowReject::Permission);
        }
        let page = match stack.allocate_page(fault_page, page_allocator, page_metadata_map) {
            Ok(page) => page,
            Err(reason) => return reject(stack, reason),
        };
        let old_l0_count = self.page_table_l0_count;
        let (_, vpn1, _) = sv39_indices(fault_page);
        let had_l0 = self.page_table_l0s[..old_l0_count]
            .iter()
            .any(|slot| slot.page.is_some() && slot.vpn1 == vpn1);
        #[cfg(app_smoke)]
        let inject_pte_failure = core::mem::take(&mut self.fail_next_stack_pte_install);
        #[cfg(not(app_smoke))]
        let inject_pte_failure = false;
        if inject_pte_failure
            || !self.install_user_leaf_pte(
                fault_page,
                page.phys().value(),
                true,
                true,
                false,
                page_allocator,
                page_metadata_map,
            )
        {
            self.rollback_l0_tables(old_l0_count, page_allocator, page_metadata_map);
            stack.rollback_page(fault_page, page_allocator, page_metadata_map);
            let reason = if had_l0 {
                UserStackGrowReject::PteInstall
            } else {
                UserStackGrowReject::PageTableAllocation
            };
            return reject(stack, reason);
        }

        if new_base != old_base {
            let mapping = &mut self.mappings[self.stack_mapping_index];
            mapping.vaddr = new_base;
            mapping.memsz = stack.top() - new_base;
            mapping.bss_zero_bytes = mapping.memsz;
            mapping.bss_bytes_zeroed = mapping.memsz;
            stack.commit_vma_base(new_base);
        }
        crate::arch::riscv64::csr::sfence_vma_addr(fault_page);
        stack.record_grow_complete(fault_addr, old_base, new_base, true);
        Ok(UserStackFaultResolution {
            fault_page,
            old_base,
            new_base,
            expanded: new_base != old_base,
        })
    }

    #[cfg(app_smoke)]
    pub fn smoke_fail_next_stack_pte_install(&mut self) {
        self.fail_next_stack_pte_install = true;
    }

    #[cfg(app_smoke)]
    pub fn smoke_set_heap_mapping_end(&mut self, end: usize) -> Option<usize> {
        if self.heap_mapping_index >= self.mapping_count {
            return None;
        }
        let mapping = &mut self.mappings[self.heap_mapping_index];
        let bytes = if mapping.memsz() < 2 * USER_PAGE_SIZE {
            2 * USER_PAGE_SIZE
        } else {
            mapping.memsz()
        };
        let old_vaddr = mapping.vaddr;
        mapping.vaddr = end.checked_sub(bytes)?;
        mapping.memsz = bytes;
        Some(old_vaddr)
    }

    #[cfg(app_smoke)]
    pub fn smoke_restore_heap_mapping_vaddr(&mut self, vaddr: usize) {
        if self.heap_mapping_index < self.mapping_count {
            let heap_extent = self.heap_brk.saturating_sub(self.heap_base);
            let mapping = &mut self.mappings[self.heap_mapping_index];
            mapping.vaddr = vaddr;
            mapping.memsz = heap_extent;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_user_stack_range(
        &mut self,
        stack: &mut UserStack,
        addr: usize,
        len: usize,
        access: UserFaultAccess,
        current_satp: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), UserStackGrowReject> {
        if len == 0 {
            return Ok(());
        }
        let end = addr.checked_add(len).ok_or(UserStackGrowReject::Rlimit)?;
        let mut page = align_down(addr, USER_PAGE_SIZE);
        while page < end {
            if stack.page_for_vaddr(page).is_none() {
                self.resolve_user_stack_fault(
                    stack,
                    page,
                    access,
                    current_satp,
                    page_allocator,
                    page_metadata_map,
                )?;
            }
            page = page
                .checked_add(USER_PAGE_SIZE)
                .ok_or(UserStackGrowReject::Rlimit)?;
        }
        Ok(())
    }

    pub fn enable(
        &mut self,
        trap_frame: &UserTrapFrame,
        stack: &UserStack,
        swapper_vm: &SwapperVm,
        kernel_image: &KernelImage,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Ready
            || trap_frame.state() != State::Ready
            || stack.state() != State::Ready
            || swapper_vm.state() != State::Ready
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
            || !self.install_all_user_leaf_ptes(stack, page_allocator, page_metadata_map)
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
        stack: &UserStack,
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
        let mut stack_page_index = 0usize;
        while stack_page_index < stack.backing_page_count() {
            let Some(virt) = stack.backing_page_vaddr(stack_page_index) else {
                return false;
            };
            let Some(page) = stack.backing_page(stack_page_index) else {
                return false;
            };
            if !self.install_user_leaf_pte(
                virt,
                page.phys().value(),
                true,
                true,
                false,
                page_allocator,
                page_metadata_map,
            ) {
                return false;
            }
            stack_page_index += 1;
        }
        self.user_leaf_ptes_installed = self.user_leaf_pte_count != 0
            && self.user_leaf_pte_count
                == total_mapping_page_count(&self.mappings, self.mapping_count)
                    + stack.backing_page_count();
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
        if kind == UserMappingKind::Stack {
            return user_accessible;
        }
        if kind == UserMappingKind::Empty || !user_accessible {
            return false;
        }
        if backing_page_count == 0 {
            return matches!(
                kind,
                UserMappingKind::Heap | UserMappingKind::AnonymousPrivate
            );
        }

        let readable = self.mappings[mapping_index].readable();
        let writable = self.mappings[mapping_index].writable();
        let executable = self.mappings[mapping_index].executable();
        let mut page_index = 0usize;
        while page_index < backing_page_count {
            let Some(page) = self.mappings[mapping_index].backing_page(page_index) else {
                return false;
            };
            let Some(virt) = self.mappings[mapping_index].backing_page_vaddr(page_index) else {
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
            page_index += 1;
        }
        true
    }

    // Leaf installation keeps permission and allocator inputs explicit for rollback accounting.
    #[allow(clippy::too_many_arguments)]
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
        if !virt.is_multiple_of(USER_PAGE_SIZE) || !phys.is_multiple_of(USER_PAGE_SIZE) {
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

    fn user_leaf_pte_maps_page(
        &self,
        virt: usize,
        phys: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        const PTE_VALID: usize = 1 << 0;
        const PTE_LEAF_PERMISSIONS: usize = (1 << 1) | (1 << 2) | (1 << 3);
        const PTE_USER: usize = 1 << 4;
        const SV39_PPN_MASK: usize = (1usize << 44) - 1;

        let (vpn2, vpn1, vpn0) = sv39_indices(virt);
        if vpn2 != 0 || !phys.is_multiple_of(USER_PAGE_SIZE) {
            return false;
        }
        let Some(l0_page) = self.existing_l0_page(vpn1) else {
            return false;
        };
        let Some(l0_table) = page_table_page_mut(l0_page, page_metadata_map) else {
            return false;
        };
        let Some(pte) = l0_table.entry(vpn0) else {
            return false;
        };
        pte & PTE_VALID != 0
            && pte & PTE_LEAF_PERMISSIONS != 0
            && pte & PTE_USER != 0
            && (((pte >> 10) & SV39_PPN_MASK) << 12) == phys
    }

    fn clear_user_leaf_pte(
        &mut self,
        virt: usize,
        expected_phys: usize,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if !self.user_leaf_pte_maps_page(virt, expected_phys, page_metadata_map) {
            return false;
        }
        let (_, vpn1, vpn0) = sv39_indices(virt);
        let Some(l0_page) = self.existing_l0_page(vpn1) else {
            return false;
        };
        let Some(l0_table) = page_table_page_mut(l0_page, page_metadata_map) else {
            return false;
        };
        if self.user_leaf_pte_count == 0 || !l0_table.set_entry(vpn0, 0) {
            return false;
        }
        self.user_leaf_pte_count -= 1;
        true
    }

    fn existing_l0_page(&self, vpn1: usize) -> Option<PageRef> {
        let mut index = 0usize;
        while index < self.page_table_l0_count {
            let slot = self.page_table_l0s[index];
            if slot.vpn1 == vpn1 {
                return slot.page;
            }
            index += 1;
        }
        None
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

    fn rollback_l0_tables(
        &mut self,
        old_count: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) {
        while self.page_table_l0_count > old_count {
            self.page_table_l0_count -= 1;
            let slot = self.page_table_l0s[self.page_table_l0_count];
            if let Some(l1_page) = self.page_table_l1
                && let Some(l1_table) = page_table_page_mut(l1_page, page_metadata_map)
            {
                let _ = l1_table.set_entry(slot.vpn1, 0);
            }
            if let Some(page) = slot.page {
                let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            }
            self.page_table_l0s[self.page_table_l0_count] = UserL0TableSlot::empty();
        }
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
struct UserKernelTrapStackRuntime {
    ready: AtomicUsize,
    vmapped: AtomicUsize,
    guard_base: AtomicUsize,
    guard_size: AtomicUsize,
    guard_unmapped: AtomicUsize,
    stack_base: AtomicUsize,
    stack_top: AtomicUsize,
    stack_size: AtomicUsize,
    stack_align: AtomicUsize,
    backing_phys: AtomicUsize,
    backing_order: AtomicUsize,
}

#[cfg(app_user_boot)]
impl UserKernelTrapStackRuntime {
    const fn new() -> Self {
        Self {
            ready: AtomicUsize::new(0),
            vmapped: AtomicUsize::new(0),
            guard_base: AtomicUsize::new(0),
            guard_size: AtomicUsize::new(0),
            guard_unmapped: AtomicUsize::new(0),
            stack_base: AtomicUsize::new(0),
            stack_top: AtomicUsize::new(0),
            stack_size: AtomicUsize::new(0),
            stack_align: AtomicUsize::new(0),
            backing_phys: AtomicUsize::new(0),
            backing_order: AtomicUsize::new(0),
        }
    }
}

#[cfg(app_user_boot)]
static USER_KERNEL_TRAP_STACK_RUNTIME: UserKernelTrapStackRuntime =
    UserKernelTrapStackRuntime::new();

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

/// Stable application runtime owned by KernelInitFlow. Exec replaces only the
/// internal application instance; it never replaces the owning TaskFlow.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct UserAppRuntime {
    lifecycle: Lifecycle,
    declared: bool,
    declaration_occurrence: usize,
    owner_pid: usize,
    owner_task_ref: TaskRef,
    owner_flow_ref: TaskFlowRef,
    owner_bound: bool,
    entry_source_pid1_exec: bool,
    instance_fresh: bool,
    execution_context_ready: bool,
    active_binding_committed: bool,
    application_entered: bool,
    application_generation: u32,
    released: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl UserAppRuntime {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            declared: false,
            declaration_occurrence: 0,
            owner_pid: 0,
            owner_task_ref: TaskRef::NONE,
            owner_flow_ref: TaskFlowRef::NONE,
            owner_bound: false,
            entry_source_pid1_exec: false,
            instance_fresh: false,
            execution_context_ready: false,
            active_binding_committed: false,
            application_entered: false,
            application_generation: 0,
            released: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn declared(&self) -> bool {
        self.declared
    }

    pub const fn declaration_occurrence(&self) -> usize {
        self.declaration_occurrence
    }

    pub const fn owner_pid(&self) -> usize {
        self.owner_pid
    }

    pub const fn active_binding_committed(&self) -> bool {
        self.active_binding_committed
    }

    pub const fn application_entered(&self) -> bool {
        self.application_entered
    }

    pub const fn released(&self) -> bool {
        self.released
    }

    /// Materialize the one lifetime-stable runtime in Base.
    pub fn declare(&mut self) -> EventResult {
        if self.declared || self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Base,
            );
        }
        self.declared = true;
        self.declaration_occurrence = self.declaration_occurrence.wrapping_add(1);
        self.owner_bound = false;
        self.owner_task_ref = TaskRef::NONE;
        self.owner_flow_ref = TaskFlowRef::NONE;
        self.execution_context_ready = false;
        self.active_binding_committed = false;
        self.application_entered = false;
        self.released = false;
        Ok(())
    }

    pub fn preset(&mut self, owner: &mut KernelInitTask) -> EventResult {
        if !self.declared
            || self.declaration_occurrence == 0
            || self.lifecycle.state() != State::Base
            || owner.state() != State::OnCpu
            || owner.pid() != super::rest_init::KERNEL_INIT_PID
            || owner.task().flow() != TaskFlowRef::KERNEL_INIT
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.owner_pid = owner.pid();
        self.owner_task_ref = owner.task_ref();
        self.owner_flow_ref = owner.task().flow();
        self.owner_bound = true;
        self.entry_source_pid1_exec = true;
        self.instance_fresh = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            crate::checkpoint::Checkpoint::UserAppRuntimePrepared,
        )
    }

    pub fn setup(
        &mut self,
        owner: &KernelInitTask,
        owner_state: &KernelInitTaskUserState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.owner_bound
            || !self.entry_source_pid1_exec
            || !self.instance_fresh
            || owner.task().flow() != TaskFlowRef::KERNEL_INIT
            || !owner_state.resources_bound()
            || !owner_state.trap_frame_bound()
            || !owner_state.address_space_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.execution_context_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            crate::checkpoint::Checkpoint::UserAppRuntimeReady,
        )
    }

    pub fn commit_active_binding(&mut self, owner: &KernelInitTask) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.execution_context_ready
            || owner.state() != State::OnCpu
            || owner.pid() != self.owner_pid
            || owner.task().flow() != TaskFlowRef::KERNEL_INIT
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.active_binding_committed = true;
        Ok(())
    }

    pub fn enable(
        &mut self,
        owner: &KernelInitTask,
        owner_state: &KernelInitTaskUserState,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.execution_context_ready
            || !self.active_binding_committed
            || owner.task().flow() != TaskFlowRef::KERNEL_INIT
            || !owner_state.resources_bound()
            || !owner_state.syscall_context_bound()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.application_entered = true;
        self.application_generation = next_generation(self.application_generation);
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            crate::checkpoint::Checkpoint::UserAppRuntimeOnline,
        )
    }

    #[cfg_attr(app_smoke, allow(dead_code))]
    pub const fn flow_ref(&self) -> super::task_flow::TaskFlowRef {
        self.owner_flow_ref
    }

    pub const fn task_ref_owner(&self) -> super::task::TaskRef {
        self.owner_task_ref
    }

    pub const fn flow_generation(&self) -> u32 {
        self.owner_flow_ref.generation()
    }

    pub const fn flow_ref_valid(&self, flow_ref: super::task_flow::TaskFlowRef) -> bool {
        self.declared && flow_ref.same_identity(self.owner_flow_ref)
    }

    pub fn replace_kernel_application(&mut self, owner: &mut KernelInitTask) -> EventResult {
        if !self.declared
            || self.lifecycle.state() != State::Online
            || owner.state() != State::OnCpu
            || owner.task().flow() != TaskFlowRef::KERNEL_INIT
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.state(),
                State::Online,
                State::Online,
            );
        }

        self.active_binding_committed = true;
        self.application_entered = true;
        self.application_generation = next_generation(self.application_generation);
        owner.mark_application_committed();
        Ok(())
    }

    pub fn begin_kernel_shutdown(&mut self, owner: &mut KernelInitTask) -> EventResult {
        if !self.declared
            || owner.state() != State::OnCpu
            || owner.task().flow() != TaskFlowRef::KERNEL_INIT
            || self.lifecycle.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.state(),
                State::Online,
                State::Destroyed,
            );
        }

        self.active_binding_committed = false;
        self.application_entered = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Disable, State::Online, State::Offline)?;
        Ok(())
    }

    pub fn finish_kernel_shutdown(&mut self, owner: &KernelInitTask) -> EventResult {
        if self.lifecycle.state() != State::Offline || owner.state() != State::Destroyed {
            return failed_condition(
                LifecycleEvent::Cleanup,
                owner.state(),
                State::Online,
                State::Destroyed,
            );
        }
        self.released = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Cleanup, State::Offline, State::Destroyed)
    }

    fn bind_dynamic(&mut self, owner: &Task, flow: &TaskFlow, pid: usize) -> EventResult {
        self.declare()?;
        if pid == 0
            || owner.state() != State::Online
            || flow.state() != State::Online
            || owner.flow() != flow.flow_ref()
            || flow.owner() != owner.task_ref()
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.state(),
                State::Base,
                State::Prepared,
            );
        }
        self.owner_pid = pid;
        self.owner_task_ref = owner.task_ref();
        self.owner_flow_ref = flow.flow_ref();
        self.owner_bound = true;
        self.instance_fresh = true;
        self.execution_context_ready = true;
        self.active_binding_committed = true;
        self.application_entered = true;
        self.application_generation = next_generation(self.application_generation);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    fn replace_dynamic_application(&mut self, owner: &Task, flow: &TaskFlow) -> bool {
        if self.lifecycle.state() != State::Online
            || owner.state() != State::OnCpu
            || !owner.task_ref().same_identity(self.owner_task_ref)
            || !flow.flow_ref().same_identity(self.owner_flow_ref)
            || !super::task_flow::task_flow_execution_guard_satisfied(flow, owner)
        {
            return false;
        }
        self.application_generation = next_generation(self.application_generation);
        true
    }

    fn cleanup_dynamic(&mut self, owner: &Task, flow: &TaskFlow) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !owner.task_ref().same_identity(self.owner_task_ref)
            || !flow.flow_ref().same_identity(self.owner_flow_ref)
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.state(),
                State::Online,
                State::Destroyed,
            );
        }
        self.active_binding_committed = false;
        self.application_entered = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Disable, State::Online, State::Offline)?;
        self.released = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Cleanup, State::Offline, State::Destroyed)
    }
}

/// PID 1 resources and execution context owned by the stable
/// [`KernelInitTask`].  This is not a task or flow lifecycle carrier.
pub struct KernelInitTaskUserState {
    resources_bound: bool,
    active_user_flow_online: bool,
    preserves_kernel_init_task: bool,
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
    supplementary_group_count: usize,
    supplementary_groups: [usize; USER_SUPPLEMENTARY_GROUP_MAX],
    supplementary_groups_read_observed: bool,
    setgroups_observed: bool,
    pid_read_observed: bool,
    ppid_zero_first_slice: bool,
    ppid_read_observed: bool,
    session_leader_first_slice: bool,
    session_id: usize,
    session_id_read_observed: bool,
    process_group_leader_first_slice: bool,
    process_group: usize,
    process_group_read_observed: bool,
    process_group_set_observed: bool,
    setsid_eperm_observed: bool,
    child_process_pid: usize,
    child_process_group_visible: bool,
    child_process_group: usize,
    child_process_session_id: usize,
    pending_plain_fork_child_visible: bool,
    pending_plain_fork_child_parent_pid: usize,
    pending_plain_fork_child_pid: usize,
    pending_plain_fork_child_process_group: usize,
    pending_plain_fork_child_session_id: usize,
    pending_plain_fork_child_process_group_set_observed: bool,
    observed_child_parent_identity_saved: bool,
    observed_child_parent_process_group: usize,
    observed_child_parent_session_id: usize,
    observed_child_parent_session_leader_first_slice: bool,
    observed_child_parent_controlling_tty_bound: bool,
    observed_child_parent_controlling_tty_cleared_on_setsid: bool,
    child_session_leader_first_slice: bool,
    child_process_group_set_observed: bool,
    child_setsid_success_observed: bool,
    child_controlling_tty_bound: bool,
    child_controlling_tty_cleared_on_setsid: bool,
    controlling_tty_bound: bool,
    foreground_pgrp_bound: bool,
    foreground_pgrp: usize,
    foreground_pgrp_read_observed: bool,
    foreground_pgrp_set_observed: bool,
    tty_session_id_read_observed: bool,
    tiocsctty_observed: bool,
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
    pending_signal_set_empty_first_slice: bool,
    blocked_signal_mask: usize,
    signal_actions: [UserSignalAction; USER_SIGNAL_COUNT],
    rt_sigprocmask_observed: bool,
    rt_sigaction_observed: bool,
    rt_sigtimedwait_observed: bool,
    rt_sigtimedwait_mask: usize,
    rt_sigtimedwait_uinfo_null: bool,
    rt_sigtimedwait_uts_null: bool,
    rt_sigtimedwait_pending_match: bool,
    rt_sigtimedwait_infinite_wait: bool,
    pending_sigchld: bool,
    rt_sigtimedwait_sleeping: bool,
    rt_sigtimedwait_wait_queue: UserSignalWaitQueue,
    rt_sigtimedwait_saved_frame: Option<TrapFrame>,
    rt_sigtimedwait_sleep_reason: usize,
    rt_sigtimedwait_wake_signal: usize,
    rt_sigtimedwait_dequeued_signal: usize,
    rt_sigtimedwait_return_signal: usize,
    clear_child_tid_bound: bool,
    clear_child_tid: usize,
    root_cwd_first_slice: bool,
    getcwd_observed: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum BuiltinGrandchildParentResume {
    None,
    Wait4,
    PipeRead,
}

struct BuiltinGrandchildExecStorageBinding {
    address_space_ptr: usize,
    stack_ptr: usize,
    parent_satp: usize,
    parent_stack_top: usize,
    bind_count: usize,
    restore_count: usize,
}

impl BuiltinGrandchildExecStorageBinding {
    const fn new() -> Self {
        Self {
            address_space_ptr: 0,
            stack_ptr: 0,
            parent_satp: 0,
            parent_stack_top: 0,
            bind_count: 0,
            restore_count: 0,
        }
    }
}

struct BuiltinGrandchildContinuation {
    bound: bool,
    child_trap_frame: Option<TrapFrame>,
    child_stack_snapshot: [u8; USER_STACK_SIZE],
    child_stack_snapshot_len: usize,
    child_stack_snapshot_copied: bool,
    child_stack_snapshot_restored: bool,
    child_fd_snapshot: FilesStructSnapshot,
    child_fd_snapshot_saved: bool,
    child_fd_snapshot_restored: bool,
    parent_wait_frame: Option<TrapFrame>,
    parent_wait_status_ptr: usize,
    parent_resume: BuiltinGrandchildParentResume,
    exited_waitable: bool,
    parent_fs_snapshot: Option<FsStructSnapshot>,
    parent_fs_snapshot_restored: bool,
    parent_fd_snapshot: FilesStructSnapshot,
    parent_fd_snapshot_saved: bool,
    parent_fd_snapshot_restored: bool,
    parent_stack_snapshot: [u8; USER_STACK_SIZE],
    parent_stack_snapshot_len: usize,
    parent_stack_snapshot_copied: bool,
    parent_stack_snapshot_restored: bool,
    parent_writable_page_snapshot_pages: [Option<PageRef>; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    parent_writable_page_checksums: [usize; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    parent_writable_page_count: usize,
    parent_writable_page_snapshot_saved: bool,
    parent_writable_page_snapshot_truncated: bool,
    parent_writable_page_snapshot_restored: bool,
    parent_exec_storage: BuiltinGrandchildExecStorageBinding,
    parent_exec_address_space_snapshot_saved: bool,
    parent_exec_stack_snapshot_saved: bool,
    parent_exec_snapshot_ever_saved: bool,
    parent_exec_snapshot_restored: bool,
    parent_exec_retained_satp: usize,
    exec_commit_count: usize,
}

impl BuiltinGrandchildContinuation {
    const fn new() -> Self {
        Self {
            bound: false,
            child_trap_frame: None,
            child_stack_snapshot: [0; USER_STACK_SIZE],
            child_stack_snapshot_len: 0,
            child_stack_snapshot_copied: false,
            child_stack_snapshot_restored: false,
            child_fd_snapshot: FilesStructSnapshot::empty(),
            child_fd_snapshot_saved: false,
            child_fd_snapshot_restored: false,
            parent_wait_frame: None,
            parent_wait_status_ptr: 0,
            parent_resume: BuiltinGrandchildParentResume::None,
            exited_waitable: false,
            parent_fs_snapshot: None,
            parent_fs_snapshot_restored: false,
            parent_fd_snapshot: FilesStructSnapshot::empty(),
            parent_fd_snapshot_saved: false,
            parent_fd_snapshot_restored: false,
            parent_stack_snapshot: [0; USER_STACK_SIZE],
            parent_stack_snapshot_len: 0,
            parent_stack_snapshot_copied: false,
            parent_stack_snapshot_restored: false,
            parent_writable_page_snapshot_pages: [None; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
            parent_writable_page_checksums: [0; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
            parent_writable_page_count: 0,
            parent_writable_page_snapshot_saved: false,
            parent_writable_page_snapshot_truncated: false,
            parent_writable_page_snapshot_restored: false,
            parent_exec_storage: BuiltinGrandchildExecStorageBinding::new(),
            parent_exec_address_space_snapshot_saved: false,
            parent_exec_stack_snapshot_saved: false,
            parent_exec_snapshot_ever_saved: false,
            parent_exec_snapshot_restored: false,
            parent_exec_retained_satp: 0,
            exec_commit_count: 0,
        }
    }

    fn reset_empty(&mut self) {
        self.bound = false;
        self.child_trap_frame = None;
        self.child_stack_snapshot.fill(0);
        self.child_stack_snapshot_len = 0;
        self.child_stack_snapshot_copied = false;
        self.child_stack_snapshot_restored = false;
        self.child_fd_snapshot = FilesStructSnapshot::empty();
        self.child_fd_snapshot_saved = false;
        self.child_fd_snapshot_restored = false;
        self.parent_wait_frame = None;
        self.parent_wait_status_ptr = 0;
        self.parent_resume = BuiltinGrandchildParentResume::None;
        self.exited_waitable = false;
        self.parent_fs_snapshot = None;
        self.parent_fs_snapshot_restored = false;
        self.parent_fd_snapshot = FilesStructSnapshot::empty();
        self.parent_fd_snapshot_saved = false;
        self.parent_fd_snapshot_restored = false;
        self.parent_stack_snapshot.fill(0);
        self.parent_stack_snapshot_len = 0;
        self.parent_stack_snapshot_copied = false;
        self.parent_stack_snapshot_restored = false;
        self.parent_writable_page_snapshot_pages.fill(None);
        self.parent_writable_page_checksums.fill(0);
        self.parent_writable_page_count = 0;
        self.parent_writable_page_snapshot_saved = false;
        self.parent_writable_page_snapshot_truncated = false;
        self.parent_writable_page_snapshot_restored = false;
        self.parent_exec_storage = BuiltinGrandchildExecStorageBinding::new();
        self.parent_exec_address_space_snapshot_saved = false;
        self.parent_exec_stack_snapshot_saved = false;
        self.parent_exec_snapshot_ever_saved = false;
        self.parent_exec_snapshot_restored = false;
        self.parent_exec_retained_satp = 0;
        self.exec_commit_count = 0;
    }
}

pub struct UserTaskSet {
    set_lifecycle: Lifecycle,
    task_slots: [UserTaskStorageSlot; USER_TASK_SLOT_COUNT],
    active_task_ref: TaskRef,
    parent_task_ref: TaskRef,
    pending_task_ref: TaskRef,
    last_exited_task_ref: TaskRef,
    carrier_stack_base: usize,
    carrier_stack_top: usize,
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
    parent_fd_snapshot: FilesStructSnapshot,
    parent_fd_snapshot_saved: bool,
    parent_fd_snapshot_restored: bool,
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
    parent_wait_frame: Option<TrapFrame>,
    parent_wait_status_ptr: usize,
    parent_address_space_snapshot: UserAddressSpace,
    parent_address_space_snapshot_saved: bool,
    parent_exec_stack_snapshot: UserStack,
    parent_exec_stack_snapshot_saved: bool,
    parent_wait_stack_snapshot: [u8; USER_STACK_SIZE],
    parent_wait_stack_snapshot_len: usize,
    parent_wait_stack_snapshot_copied: bool,
    parent_wait_stack_snapshot_restored: bool,
    parent_wait_stack_window: [u8; USER_PARENT_WAIT_STACK_PROBE_LEN],
    parent_wait_stack_window_start: usize,
    parent_wait_stack_window_len: usize,
    parent_wait_stack_window_saved: bool,
    parent_wait_stack_window_compared: bool,
    parent_wait_stack_window_diff_count: usize,
    parent_wait_stack_window_first_diff_addr: usize,
    parent_wait_stack_window_before_byte: u8,
    parent_wait_stack_window_after_byte: u8,
    parent_wait_writable_page_snapshot_pages:
        [Option<PageRef>; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    parent_wait_writable_page_checksums: [usize; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    parent_wait_writable_page_count: usize,
    parent_wait_writable_page_snapshot_saved: bool,
    parent_wait_writable_page_snapshot_truncated: bool,
    parent_wait_writable_page_snapshot_restored: bool,
    parent_wait_writable_page_compared: bool,
    parent_wait_writable_page_dirty_count: usize,
    parent_wait_writable_page_stack_dirty_count: usize,
    parent_wait_writable_page_non_stack_dirty_count: usize,
    parent_wait_first_non_stack_dirty_kind: usize,
    parent_wait_first_non_stack_dirty_mapping_index: usize,
    parent_wait_first_non_stack_dirty_page_index: usize,
    parent_wait_first_non_stack_dirty_addr: usize,
    parent_wait_first_non_stack_dirty_before_checksum: usize,
    parent_wait_first_non_stack_dirty_after_checksum: usize,
    child_exit_status: usize,
    child_exit_status_observed: bool,
    wait4_status_copied: bool,
    parent_wait_resumed: bool,
    vfork_vm_clone: bool,
    vfork_pidfd_clone: bool,
    vfork_parent_frame: Option<TrapFrame>,
    vfork_parent_frame_saved: bool,
    vfork_child_handoff: bool,
    vfork_parent_resumed: bool,
    vfork_child_sp: usize,
    nested_vfork_clone: bool,
    nested_vfork_parent_pid: usize,
    vfork_parent_resume_on_exit: bool,
    pidfd_fd: usize,
    pidfd_copyout: bool,
    parent_clone_return: usize,
    enqueued: bool,
    wait4_parent_wait_observed: bool,
    child_continuation_taken: bool,
    observed_plain_fork_clone: bool,
    observed_plain_fork_parent_pid: usize,
    observed_plain_fork_parent_parent_pid: usize,
    observed_plain_fork_parent_tgid: usize,
    observed_plain_fork_child_pid: usize,
    observed_plain_fork_child_active: bool,
    observed_plain_fork_parent_restored: bool,
    builtin_grandchild: BuiltinGrandchildContinuation,
    #[cfg(app_smoke)]
    fail_next_builtin_grandchild_wait_capture: bool,
    next_child_pid: usize,
    completed_child_records: [UserCompletedChildRecord; USER_COMPLETED_CHILD_RECORD_CAPACITY],
    completed_child_record_count: usize,
    completed_child_record_archived: bool,
    completed_child_record_reaped: bool,
    completed_child_record_released: bool,
    completed_child_record_total_archived: usize,
    completed_child_record_reaped_count: usize,
    completed_child_record_released_count: usize,
    last_archived_child_pid: usize,
    last_archived_child_exit_status: usize,
    last_archived_child_wait_status: usize,
    last_reaped_child_pid: usize,
    last_reaped_child_wait_status: usize,
    last_released_child_pid: usize,
    last_released_child_wait_status: usize,
    active_task_record_available: bool,
    task_allocation_count: usize,
    vfork_next_child_accepted: bool,
}

#[allow(dead_code)]
impl UserTaskSet {
    pub const fn new() -> Self {
        Self {
            set_lifecycle: Lifecycle::new(State::Base),
            task_slots: [
                UserTaskStorageSlot::new(0),
                UserTaskStorageSlot::new(1),
                UserTaskStorageSlot::new(2),
                UserTaskStorageSlot::new(3),
                UserTaskStorageSlot::new(4),
                UserTaskStorageSlot::new(5),
                UserTaskStorageSlot::new(6),
                UserTaskStorageSlot::new(7),
            ],
            active_task_ref: TaskRef::NONE,
            parent_task_ref: TaskRef::NONE,
            pending_task_ref: TaskRef::NONE,
            last_exited_task_ref: TaskRef::NONE,
            carrier_stack_base: 0,
            carrier_stack_top: 0,
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
            parent_fd_snapshot: FilesStructSnapshot::empty(),
            parent_fd_snapshot_saved: false,
            parent_fd_snapshot_restored: false,
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
            parent_wait_frame: None,
            parent_wait_status_ptr: 0,
            parent_address_space_snapshot: UserAddressSpace::new(),
            parent_address_space_snapshot_saved: false,
            parent_exec_stack_snapshot: UserStack::new(),
            parent_exec_stack_snapshot_saved: false,
            parent_wait_stack_snapshot: [0; USER_STACK_SIZE],
            parent_wait_stack_snapshot_len: 0,
            parent_wait_stack_snapshot_copied: false,
            parent_wait_stack_snapshot_restored: false,
            parent_wait_stack_window: [0; USER_PARENT_WAIT_STACK_PROBE_LEN],
            parent_wait_stack_window_start: 0,
            parent_wait_stack_window_len: 0,
            parent_wait_stack_window_saved: false,
            parent_wait_stack_window_compared: false,
            parent_wait_stack_window_diff_count: 0,
            parent_wait_stack_window_first_diff_addr: 0,
            parent_wait_stack_window_before_byte: 0,
            parent_wait_stack_window_after_byte: 0,
            parent_wait_writable_page_snapshot_pages: [None; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
            parent_wait_writable_page_checksums: [0; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
            parent_wait_writable_page_count: 0,
            parent_wait_writable_page_snapshot_saved: false,
            parent_wait_writable_page_snapshot_truncated: false,
            parent_wait_writable_page_snapshot_restored: false,
            parent_wait_writable_page_compared: false,
            parent_wait_writable_page_dirty_count: 0,
            parent_wait_writable_page_stack_dirty_count: 0,
            parent_wait_writable_page_non_stack_dirty_count: 0,
            parent_wait_first_non_stack_dirty_kind: 0,
            parent_wait_first_non_stack_dirty_mapping_index: 0,
            parent_wait_first_non_stack_dirty_page_index: 0,
            parent_wait_first_non_stack_dirty_addr: 0,
            parent_wait_first_non_stack_dirty_before_checksum: 0,
            parent_wait_first_non_stack_dirty_after_checksum: 0,
            child_exit_status: 0,
            child_exit_status_observed: false,
            wait4_status_copied: false,
            parent_wait_resumed: false,
            vfork_vm_clone: false,
            vfork_pidfd_clone: false,
            vfork_parent_frame: None,
            vfork_parent_frame_saved: false,
            vfork_child_handoff: false,
            vfork_parent_resumed: false,
            vfork_child_sp: 0,
            nested_vfork_clone: false,
            nested_vfork_parent_pid: 0,
            vfork_parent_resume_on_exit: true,
            pidfd_fd: usize::MAX,
            pidfd_copyout: false,
            parent_clone_return: 0,
            enqueued: false,
            wait4_parent_wait_observed: false,
            child_continuation_taken: false,
            observed_plain_fork_clone: false,
            observed_plain_fork_parent_pid: 0,
            observed_plain_fork_parent_parent_pid: 0,
            observed_plain_fork_parent_tgid: 0,
            observed_plain_fork_child_pid: 0,
            observed_plain_fork_child_active: false,
            observed_plain_fork_parent_restored: false,
            builtin_grandchild: BuiltinGrandchildContinuation::new(),
            #[cfg(app_smoke)]
            fail_next_builtin_grandchild_wait_capture: false,
            next_child_pid: USER_CHILD_PID,
            completed_child_records: [UserCompletedChildRecord::empty();
                USER_COMPLETED_CHILD_RECORD_CAPACITY],
            completed_child_record_count: 0,
            completed_child_record_archived: false,
            completed_child_record_reaped: false,
            completed_child_record_released: false,
            completed_child_record_total_archived: 0,
            completed_child_record_reaped_count: 0,
            completed_child_record_released_count: 0,
            last_archived_child_pid: 0,
            last_archived_child_exit_status: 0,
            last_archived_child_wait_status: 0,
            last_reaped_child_pid: 0,
            last_reaped_child_wait_status: 0,
            last_released_child_pid: 0,
            last_released_child_wait_status: 0,
            active_task_record_available: false,
            task_allocation_count: 0,
            vfork_next_child_accepted: false,
        }
    }

    pub const fn state(&self) -> State {
        self.set_lifecycle.state()
    }

    pub const fn active_task_state(&self) -> State {
        if let Some(slot) = self.slot_for_ref(self.active_task_ref) {
            slot.state()
        } else if self.active_task_record_available {
            State::Prepared
        } else if let Some(slot) = self.slot_for_ref(self.last_exited_task_ref) {
            slot.state()
        } else {
            State::Base
        }
    }

    pub const fn active_task_ref(&self) -> TaskRef {
        self.active_task_ref
    }

    pub const fn last_exited_task_ref(&self) -> TaskRef {
        self.last_exited_task_ref
    }

    pub const fn last_exited_flow_ref(&self) -> TaskFlowRef {
        if let Some(slot) = self.slot_for_ref(self.last_exited_task_ref) {
            slot.task.flow_ref()
        } else {
            TaskFlowRef::NONE
        }
    }

    pub const fn parent_task_ref(&self) -> TaskRef {
        self.parent_task_ref
    }

    pub const fn pending_task_ref(&self) -> TaskRef {
        self.pending_task_ref
    }

    pub const fn active_task_generation(&self) -> u32 {
        self.active_task_ref.generation()
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        if let Some(slot) = self.slot_for_ref(self.active_task_ref) {
            slot.task.flow()
        } else {
            TaskFlowRef::NONE
        }
    }

    pub(crate) fn carrier_stack_matches(&self, base: usize, top: usize) -> bool {
        self.set_lifecycle.state() == State::Ready
            && base != 0
            && top > base
            && self.carrier_stack_base == base
            && self.carrier_stack_top == top
    }

    pub(crate) fn carrier_stack_bounds(&self) -> Option<(usize, usize)> {
        if self.set_lifecycle.state() == State::Ready
            && self.carrier_stack_base != 0
            && self.carrier_stack_top > self.carrier_stack_base
        {
            Some((self.carrier_stack_base, self.carrier_stack_top))
        } else {
            None
        }
    }

    pub const fn cpu_ref_for_task(&self, task_ref: TaskRef) -> Option<super::cpu::CpuRef> {
        let Some(slot) = self.slot_for_ref(task_ref) else {
            return None;
        };
        slot.task.flow_cpu_ref()
    }

    pub const fn task_ref_valid(&self, task_ref: TaskRef) -> bool {
        self.slot_for_ref(task_ref).is_some()
    }

    pub const fn flow_ref_valid(&self, task_ref: TaskRef, flow_ref: TaskFlowRef) -> bool {
        let Some(slot) = self.slot_for_ref(task_ref) else {
            return false;
        };
        slot.task.embedded_flow().declared() && slot.task.flow_ref().same_identity(flow_ref)
    }

    pub const fn free_task_slot_count(&self) -> usize {
        let mut count = 0usize;
        let mut index = 0usize;
        while index < USER_TASK_SLOT_COUNT {
            if !self.task_slots[index].occupied {
                count += 1;
            }
            index += 1;
        }
        count
    }

    const fn slot_for_ref(&self, task_ref: TaskRef) -> Option<&UserTaskStorageSlot> {
        let Some(index) = task_ref.user_slot() else {
            return None;
        };
        let slot = &self.task_slots[index];
        if slot.occupied && slot.task_ref().same_identity(task_ref) {
            Some(slot)
        } else {
            None
        }
    }

    pub(crate) fn task_switch_in_ready(&self, task_ref: TaskRef) -> bool {
        self.slot_for_ref(task_ref)
            .map(|slot| slot.task.switch_in_ready())
            .unwrap_or(false)
    }

    pub(crate) fn task_switch_out_ready(&self, task_ref: TaskRef) -> bool {
        self.slot_for_ref(task_ref)
            .map(|slot| slot.task.switch_out_ready() || slot.task.terminal_switch_out_ready())
            .unwrap_or(false)
    }

    pub(crate) fn task_state(&self, task_ref: TaskRef) -> Option<State> {
        self.slot_for_ref(task_ref).map(|slot| slot.task.state())
    }

    pub(crate) fn task_identity_ptr(&self, task_ref: TaskRef) -> Option<usize> {
        self.slot_for_ref(task_ref)
            .map(|slot| &slot.task as *const Task as usize)
    }

    pub(crate) fn current_task_candidate_by_identity(
        &self,
        identity: usize,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        let mut index = 0usize;
        while index < USER_TASK_SLOT_COUNT {
            let slot = &self.task_slots[index];
            if slot.occupied && (&slot.task as *const Task as usize) == identity {
                return Some(super::current_task::CurrentTaskCandidate {
                    task: &slot.task,
                    flow: slot.task.embedded_flow(),
                });
            }
            index += 1;
        }
        None
    }

    pub(crate) fn current_task_candidate_by_ref(
        &self,
        task_ref: TaskRef,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        let slot = self.slot_for_ref(task_ref)?;
        Some(super::current_task::CurrentTaskCandidate {
            task: &slot.task,
            flow: slot.task.embedded_flow(),
        })
    }

    pub(crate) fn task_mut_by_ref(&mut self, task_ref: TaskRef) -> Option<&mut Task> {
        let index = task_ref.user_slot()?;
        let slot = &mut self.task_slots[index];
        if slot.occupied && slot.task_ref().same_identity(task_ref) {
            Some(&mut slot.task)
        } else {
            None
        }
    }

    fn allocate_user_task(&mut self, pid: usize, parent_ref: TaskRef) -> Option<TaskRef> {
        if pid == 0 {
            return None;
        }
        let mut index = 0usize;
        while index < USER_TASK_SLOT_COUNT && self.task_slots[index].occupied {
            index += 1;
        }
        if index == USER_TASK_SLOT_COUNT {
            return None;
        }

        let slot = &mut self.task_slots[index];
        slot.generation = next_generation(slot.generation);
        let task_ref = TaskRef::user(index, slot.generation);
        slot.task = Task::new_user(task_ref, slot.flow_generation);
        slot.pid = pid;
        slot.runtime = UserAppRuntime::new();
        slot.occupied = true;

        task_event_or_terminate(slot.task.set_identity_metadata(
            pid,
            TaskEntry::UserChild,
            TaskKind::UserModeThread,
        ));
        task_event_or_terminate(slot.task.adopt_preset());
        task_event_or_terminate(slot.task.declare_and_bind_embedded_flow());
        slot.task.init_dummy_switch_context();
        if !slot
            .task
            .set_kernel_stack_bounds(self.carrier_stack_base, self.carrier_stack_top)
        {
            panic!("UserTask shared kernel stack boundary invariant failed");
        }
        task_event_or_terminate(slot.task.adopt_setup());
        slot.flow_generation = slot.task.flow_ref().generation();

        self.parent_task_ref = parent_ref;
        self.active_task_ref = task_ref;
        self.pending_task_ref = TaskRef::NONE;
        self.last_exited_task_ref = TaskRef::NONE;

        let slot = &mut self.task_slots[index];
        let UserTaskStorageSlot { task, runtime, .. } = slot;
        if !task.bind_flow_cpu_ref(super::cpu::CpuRef::new(0)) {
            panic!("declared UserTaskFlow CPU binding invariant failed");
        }
        task_event_or_terminate(task.publish_embedded_flow());
        task_event_or_terminate(task.adopt_enable());
        task_event_or_terminate(runtime.bind_dynamic(task, task.embedded_flow(), pid));
        Some(task_ref)
    }

    #[cfg(app_smoke)]
    pub fn dispatch_task_for_test(
        &mut self,
        task_ref: TaskRef,
        cpu_ref: super::cpu::CpuRef,
    ) -> EventResult {
        let Some(index) = task_ref.user_slot() else {
            return failed_condition(
                LifecycleEvent::Dispatch,
                State::Destroyed,
                State::Online,
                State::OnCpu,
            );
        };
        let slot = &mut self.task_slots[index];
        if !slot.occupied || !slot.task_ref().same_identity(task_ref) {
            return failed_condition(
                LifecycleEvent::Dispatch,
                slot.state(),
                State::Online,
                State::OnCpu,
            );
        }
        if slot.task.flow_cpu_ref() != Some(cpu_ref) {
            return failed_condition(
                LifecycleEvent::Dispatch,
                slot.task.flow_state(),
                State::Online,
                State::Online,
            );
        }
        slot.task.dispatch_and_enter_for_test()?;
        if slot.task.flow_state() != State::Online
            || !super::task_flow::task_flow_execution_guard_satisfied(
                slot.task.embedded_flow(),
                &slot.task,
            )
        {
            return failed_condition(
                LifecycleEvent::Dispatch,
                slot.task.flow_state(),
                State::Online,
                State::Online,
            );
        }
        Ok(())
    }

    pub(crate) fn suspend_task(&mut self, task_ref: TaskRef) -> EventResult {
        let Some(index) = task_ref.user_slot() else {
            return failed_condition(
                LifecycleEvent::Suspend,
                State::Destroyed,
                State::OnCpu,
                State::Online,
            );
        };
        let slot = &mut self.task_slots[index];
        if !slot.occupied || !slot.task_ref().same_identity(task_ref) {
            return failed_condition(
                LifecycleEvent::Suspend,
                slot.state(),
                State::OnCpu,
                State::Online,
            );
        }
        if self.last_exited_task_ref.same_identity(task_ref) {
            slot.task.disable()?;
            slot.task.cleanup()?;
        } else {
            slot.task.suspend_from_cpu()?;
        }
        Ok(())
    }

    fn destroy_active_user_task(&mut self) -> bool {
        let Some(index) = self.active_task_ref.user_slot() else {
            return false;
        };
        if !self.task_slots[index].occupied
            || !self.task_slots[index]
                .task_ref()
                .same_identity(self.active_task_ref)
        {
            return false;
        }

        let slot = &mut self.task_slots[index];
        let UserTaskStorageSlot { task, runtime, .. } = slot;
        if runtime.cleanup_dynamic(task, task.embedded_flow()).is_err()
            || task.cleanup_embedded_flow().is_err()
        {
            return false;
        }

        self.last_exited_task_ref = self.active_task_ref;
        self.active_task_ref = if self.task_ref_valid(self.parent_task_ref) {
            self.parent_task_ref
        } else {
            TaskRef::NONE
        };
        self.parent_task_ref = TaskRef::NONE;
        true
    }

    fn release_destroyed_task_ref(&mut self, task_ref: TaskRef) -> bool {
        let Some(index) = task_ref.user_slot() else {
            return false;
        };
        let slot = &mut self.task_slots[index];
        if !slot.occupied
            || !slot.task_ref().same_identity(task_ref)
            || slot.state() != State::Destroyed
        {
            return false;
        }
        slot.occupied = false;
        slot.pid = 0;
        if self.last_exited_task_ref.same_identity(task_ref) {
            self.last_exited_task_ref = TaskRef::NONE;
        }
        true
    }

    pub fn commit_active_application_replacement(&mut self) -> bool {
        let Some(index) = self.active_task_ref.user_slot() else {
            return false;
        };
        let slot = &mut self.task_slots[index];
        if !slot.occupied || !slot.task_ref().same_identity(self.active_task_ref) {
            return false;
        }
        if slot.task.state() != State::OnCpu
            || slot.task.flow_state() != State::Online
            || slot.task.flow() != slot.task.embedded_flow().flow_ref()
        {
            return false;
        }
        slot.runtime
            .replace_dynamic_application(&slot.task, slot.task.embedded_flow())
    }

    pub fn prepare_active_task_for_shutdown(&mut self) -> Option<TaskRef> {
        if !self.destroy_active_user_task() {
            return None;
        }
        Some(self.last_exited_task_ref)
    }

    #[cfg(app_smoke)]
    pub fn smoke_task_slot_generation_contract(&mut self) -> Result<(), &'static str> {
        if self.free_task_slot_count() != USER_TASK_SLOT_COUNT
            || self.active_task_ref.is_valid()
            || !self.active_task_record_available
        {
            return Err("slot contract precondition");
        }

        let next_pid_before = self.next_child_pid;
        let mut task_refs = [TaskRef::NONE; USER_TASK_SLOT_COUNT];
        let mut flow_refs = [TaskFlowRef::NONE; USER_TASK_SLOT_COUNT];
        let mut index = 0usize;
        while index < USER_TASK_SLOT_COUNT {
            let Some(task_ref) = self.allocate_user_task(10_000 + index, TaskRef::NONE) else {
                return Err("allocate all user task slots");
            };
            task_refs[index] = task_ref;
            flow_refs[index] = self.task_slots[index].task.flow();
            index += 1;
        }

        if self.free_task_slot_count() != 0
            || self
                .allocate_user_task(10_000 + USER_TASK_SLOT_COUNT, TaskRef::NONE)
                .is_some()
            || self.next_child_pid != next_pid_before
            || !self.task_slots[0].task.cleanup().is_err()
            || self.task_slots[1].task.owns_flow(flow_refs[0])
            || self.flow_ref_valid(task_refs[1], flow_refs[0])
        {
            return Err("full slot set invariants");
        }

        index = 0;
        while index < USER_TASK_SLOT_COUNT {
            let mut other = index + 1;
            while other < USER_TASK_SLOT_COUNT {
                if task_refs[index].same_identity(task_refs[other])
                    || flow_refs[index].same_identity(flow_refs[other])
                {
                    return Err("task and initial-flow identities unique");
                }
                other += 1;
            }

            let task = &mut self.task_slots[index].task;
            if task.dispatch_and_enter_for_test().is_err()
                || task.cleanup_embedded_flow().is_err()
                || task.disable().is_err()
                || task.cleanup().is_err()
            {
                return Err("dispatched task and flow cleanup");
            }
            index += 1;
        }

        index = 0;
        while index < USER_TASK_SLOT_COUNT {
            if !self.release_destroyed_task_ref(task_refs[index]) {
                return Err("release destroyed task slots");
            }
            index += 1;
        }
        self.active_task_ref = TaskRef::NONE;
        self.parent_task_ref = TaskRef::NONE;
        self.pending_task_ref = TaskRef::NONE;
        self.last_exited_task_ref = TaskRef::NONE;

        if self.task_ref_valid(task_refs[0]) || self.flow_ref_valid(task_refs[0], flow_refs[0]) {
            return Err("stale task and flow refs invalidated");
        }

        let Some(recycled_ref) = self.allocate_user_task(20_000, TaskRef::NONE) else {
            return Err("allocate recycled task slot");
        };
        let recycled_flow_ref = self
            .slot_for_ref(recycled_ref)
            .map(|slot| slot.task.flow())
            .unwrap_or(TaskFlowRef::NONE);
        if self
            .dispatch_task_for_test(recycled_ref, super::cpu::CpuRef::new(0))
            .is_err()
        {
            return Err("start recycled initial flow");
        }
        if recycled_ref.slot() != task_refs[0].slot()
            || recycled_ref.generation() == task_refs[0].generation()
            || recycled_flow_ref.slot() != flow_refs[0].slot()
            || recycled_flow_ref.generation() == flow_refs[0].generation()
            || self.task_ref_valid(task_refs[0])
            || self.flow_ref_valid(recycled_ref, flow_refs[0])
            || !self.task_ref_valid(recycled_ref)
            || !self.flow_ref_valid(recycled_ref, recycled_flow_ref)
            || !self.destroy_active_user_task()
            || self.suspend_task(self.last_exited_task_ref).is_err()
        {
            return Err("recycled identities and cleanup");
        }
        let destroyed_ref = self.last_exited_task_ref;
        if !self.release_destroyed_task_ref(destroyed_ref)
            || self.free_task_slot_count() != USER_TASK_SLOT_COUNT
            || self.next_child_pid != next_pid_before
        {
            return Err("recycled slot release and pid stability");
        }
        Ok(())
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

    pub const fn task_struct_allocated(&self) -> bool {
        self.task_struct_allocated
    }

    pub const fn pid_allocated(&self) -> bool {
        self.pid_allocated
    }

    pub const fn thread_context_ready(&self) -> bool {
        self.thread_context_ready
    }

    pub const fn sched_entity_ready(&self) -> bool {
        self.sched_entity_ready
    }

    pub const fn task_state_new(&self) -> bool {
        self.task_state_new
    }

    pub const fn files_struct_copied(&self) -> bool {
        self.files_struct_copied
    }

    pub const fn fs_struct_copied(&self) -> bool {
        self.fs_struct_copied
    }

    pub const fn credentials_copied(&self) -> bool {
        self.credentials_copied
    }

    pub const fn signal_state_copied(&self) -> bool {
        self.signal_state_copied
    }

    pub const fn user_address_space_snapshot(&self) -> bool {
        self.user_address_space_snapshot
    }

    pub const fn trap_frame_copied(&self) -> bool {
        self.trap_frame_copied
    }

    pub fn child_trap_frame_reg(&self, index: usize) -> Option<usize> {
        if self.builtin_grandchild.bound {
            self.builtin_grandchild
                .child_trap_frame
                .map(|frame| frame.reg(index))
        } else {
            self.child_trap_frame.map(|frame| frame.reg(index))
        }
    }

    pub fn child_trap_frame_sepc(&self) -> Option<usize> {
        self.child_trap_frame.map(|frame| frame.sepc)
    }

    pub const fn trap_frame_child_return_zero(&self) -> bool {
        self.trap_frame_child_return_zero
    }

    pub const fn tls_inherited(&self) -> bool {
        self.tls_inherited
    }

    pub const fn parent_fd_snapshot_saved(&self) -> bool {
        self.parent_fd_snapshot_saved
    }

    pub const fn parent_fd_snapshot_restored(&self) -> bool {
        self.parent_fd_snapshot_restored
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

    pub const fn observed_plain_fork_clone(&self) -> bool {
        self.observed_plain_fork_clone
    }

    pub const fn observed_plain_fork_parent_pid(&self) -> usize {
        self.observed_plain_fork_parent_pid
    }

    pub const fn observed_plain_fork_child_pid(&self) -> usize {
        self.observed_plain_fork_child_pid
    }

    pub const fn observed_plain_fork_child_active(&self) -> bool {
        self.observed_plain_fork_child_active
    }

    pub const fn observed_plain_fork_parent_restored(&self) -> bool {
        self.observed_plain_fork_parent_restored
    }

    pub const fn builtin_grandchild_bound(&self) -> bool {
        self.builtin_grandchild.bound
    }

    pub const fn builtin_grandchild_active(&self) -> bool {
        self.builtin_grandchild.bound && self.observed_plain_fork_child_active
    }

    pub const fn builtin_grandchild_parent_resume_is_pipe_read(&self) -> bool {
        matches!(
            self.builtin_grandchild.parent_resume,
            BuiltinGrandchildParentResume::PipeRead
        )
    }

    pub const fn builtin_grandchild_exit_waitable(&self) -> bool {
        self.builtin_grandchild.bound
            && self.builtin_grandchild.exited_waitable
            && self.observed_plain_fork_clone
            && !self.observed_plain_fork_child_active
            && self.observed_plain_fork_parent_restored
            && self.child_exit_status_observed
    }

    pub const fn builtin_grandchild_parent_wait_frame_saved(&self) -> bool {
        self.builtin_grandchild.parent_wait_frame.is_some()
    }

    pub const fn builtin_grandchild_parent_fd_snapshot_saved(&self) -> bool {
        self.builtin_grandchild.parent_fd_snapshot_saved
    }

    pub const fn builtin_grandchild_parent_fd_snapshot_restored(&self) -> bool {
        self.builtin_grandchild.parent_fd_snapshot_restored
    }

    pub const fn builtin_grandchild_parent_stack_snapshot_copied(&self) -> bool {
        self.builtin_grandchild.parent_stack_snapshot_copied
            && self.builtin_grandchild.parent_stack_snapshot_len != 0
    }

    pub const fn builtin_grandchild_parent_writable_snapshot_saved(&self) -> bool {
        self.builtin_grandchild.parent_writable_page_snapshot_saved
    }

    pub const fn builtin_grandchild_parent_writable_snapshot_restored(&self) -> bool {
        self.builtin_grandchild
            .parent_writable_page_snapshot_restored
    }

    pub const fn builtin_grandchild_exec_commit_count(&self) -> usize {
        self.builtin_grandchild.exec_commit_count
    }

    pub const fn builtin_grandchild_parent_exec_snapshot_saved(&self) -> bool {
        self.builtin_grandchild
            .parent_exec_address_space_snapshot_saved
            && self.builtin_grandchild.parent_exec_stack_snapshot_saved
    }

    pub const fn builtin_grandchild_parent_exec_snapshot_ever_saved(&self) -> bool {
        self.builtin_grandchild.parent_exec_snapshot_ever_saved
    }

    pub const fn builtin_grandchild_parent_exec_snapshot_restored(&self) -> bool {
        self.builtin_grandchild.parent_exec_snapshot_restored
    }

    pub const fn builtin_grandchild_parent_exec_satp(&self) -> usize {
        self.builtin_grandchild.parent_exec_retained_satp
    }

    pub fn builtin_grandchild_first_exec_retention_required(&self) -> bool {
        self.builtin_grandchild_active()
            && self.builtin_grandchild.exec_commit_count == 0
            && !self.builtin_grandchild.parent_exec_snapshot_ever_saved
            && !self
                .builtin_grandchild
                .parent_exec_address_space_snapshot_saved
            && !self.builtin_grandchild.parent_exec_stack_snapshot_saved
            && !self.builtin_grandchild.parent_exec_snapshot_restored
    }

    pub fn builtin_grandchild_exec_parent_snapshot_live(&self) -> bool {
        self.builtin_grandchild_active()
            && self.builtin_grandchild.exec_commit_count != 0
            && self.builtin_grandchild.parent_exec_snapshot_ever_saved
            && self
                .builtin_grandchild
                .parent_exec_address_space_snapshot_saved
            && self.builtin_grandchild.parent_exec_stack_snapshot_saved
            && !self.builtin_grandchild.parent_exec_snapshot_restored
            && self.builtin_grandchild.parent_exec_retained_satp != 0
            && self
                .builtin_grandchild
                .parent_exec_storage
                .address_space_ptr
                != 0
            && self.builtin_grandchild.parent_exec_storage.stack_ptr != 0
            && self.builtin_grandchild.parent_exec_storage.bind_count == 1
            && self.builtin_grandchild.parent_exec_storage.restore_count == 0
    }

    pub fn can_retain_builtin_grandchild_exec_objects(
        &self,
        address_space: &UserAddressSpace,
        stack: &UserStack,
    ) -> bool {
        self.builtin_grandchild_first_exec_retention_required()
            && address_space.state() == State::Online
            && address_space.satp_token() != 0
            && stack.state() == State::Ready
            && address_space
                .stack_mapping()
                .is_some_and(|mapping| mapping.stack_ownership_token() == stack.top())
    }

    pub fn retain_builtin_grandchild_exec_objects(
        &mut self,
        address_space: &UserAddressSpace,
        stack: &mut UserStack,
    ) -> bool {
        if !self.can_retain_builtin_grandchild_exec_objects(address_space, stack) {
            return false;
        }
        self.builtin_grandchild
            .parent_exec_address_space_snapshot_saved = true;
        self.builtin_grandchild.parent_exec_stack_snapshot_saved = true;
        self.builtin_grandchild.parent_exec_snapshot_ever_saved = true;
        self.builtin_grandchild.parent_exec_snapshot_restored = false;
        self.builtin_grandchild.parent_exec_retained_satp = address_space.satp_token();
        self.builtin_grandchild
            .parent_exec_storage
            .address_space_ptr = address_space as *const UserAddressSpace as usize;
        self.builtin_grandchild.parent_exec_storage.stack_ptr = stack as *const UserStack as usize;
        self.builtin_grandchild.parent_exec_storage.parent_satp = address_space.satp_token();
        self.builtin_grandchild.parent_exec_storage.parent_stack_top = stack.top();
        self.builtin_grandchild.parent_exec_storage.bind_count = 1;
        self.builtin_grandchild.parent_exec_storage.restore_count = 0;
        self.builtin_grandchild.exec_commit_count = 1;
        true
    }

    pub fn mark_builtin_grandchild_subsequent_exec_committed(&mut self) -> bool {
        if !self.builtin_grandchild_exec_parent_snapshot_live() {
            return false;
        }
        self.builtin_grandchild.exec_commit_count =
            self.builtin_grandchild.exec_commit_count.saturating_add(1);
        true
    }

    #[cfg(app_smoke)]
    pub fn smoke_fail_next_builtin_grandchild_wait_capture(&mut self) {
        self.fail_next_builtin_grandchild_wait_capture = true;
    }

    pub fn pid1_plain_fork_child_continuation(&self) -> bool {
        self.active_task_state() == State::OnCpu
            && self.current_child_continuation()
            && !self.vfork_clone()
            && self.pid != 0
            && self.parent_pid == super::rest_init::KERNEL_INIT_PID
            && self.parent_wait_frame.is_some()
            && self.parent_address_space_snapshot_saved
            && self.parent_fd_snapshot_saved
            && !self.parent_fd_snapshot_restored
            && self.parent_wait_stack_snapshot_copied
            && !self.parent_wait_stack_snapshot_restored
            && self.parent_wait_writable_page_snapshot_saved
            && !self.parent_wait_writable_page_snapshot_restored
    }

    pub fn observed_plain_fork_child_pending_wait(&self) -> bool {
        self.active_task_state() == State::OnCpu
            && self.observed_plain_fork_clone
            && !self.observed_plain_fork_child_active
            && !self.observed_plain_fork_parent_restored
            && self.observed_plain_fork_parent_pid != 0
            && self.observed_plain_fork_child_pid != 0
            && self.pid == self.observed_plain_fork_parent_pid
            && self.current_child_continuation()
    }

    pub const fn current_child_continuation(&self) -> bool {
        self.child_continuation_taken && !self.parent_wait_resumed && !self.vfork_parent_resumed
    }

    pub fn active_task_record_available(&self) -> bool {
        self.active_task_record_available
            && self.set_lifecycle.state() == State::Ready
            && self.active_task_state() == State::Prepared
            && self.prepared
            && self.task_entry == TaskEntry::UserChild
    }

    pub const fn task_allocation_count(&self) -> usize {
        self.task_allocation_count
    }

    pub const fn next_child_pid(&self) -> usize {
        self.next_child_pid
    }

    pub const fn completed_child_record_capacity(&self) -> usize {
        USER_COMPLETED_CHILD_RECORD_CAPACITY
    }

    pub const fn completed_child_record_count(&self) -> usize {
        self.completed_child_record_count
    }

    pub const fn completed_child_record_occupied_count(&self) -> usize {
        self.completed_child_record_count
    }

    pub const fn completed_child_record_free_count(&self) -> usize {
        USER_COMPLETED_CHILD_RECORD_CAPACITY.saturating_sub(self.completed_child_record_count)
    }

    pub const fn completed_child_records_full(&self) -> bool {
        self.completed_child_record_count >= USER_COMPLETED_CHILD_RECORD_CAPACITY
    }

    pub const fn completed_child_record_archived(&self) -> bool {
        self.completed_child_record_archived
    }

    pub const fn completed_child_record_reaped(&self) -> bool {
        self.completed_child_record_reaped
    }

    pub const fn completed_child_record_released(&self) -> bool {
        self.completed_child_record_released
    }

    pub const fn completed_child_record_total_archived(&self) -> usize {
        self.completed_child_record_total_archived
    }

    pub const fn completed_child_record_reaped_count(&self) -> usize {
        self.completed_child_record_reaped_count
    }

    pub const fn completed_child_record_released_count(&self) -> usize {
        self.completed_child_record_released_count
    }

    pub const fn last_archived_child_pid(&self) -> usize {
        self.last_archived_child_pid
    }

    pub const fn last_archived_child_exit_status(&self) -> usize {
        self.last_archived_child_exit_status
    }

    pub const fn last_archived_child_wait_status(&self) -> usize {
        self.last_archived_child_wait_status
    }

    pub const fn last_reaped_child_pid(&self) -> usize {
        self.last_reaped_child_pid
    }

    pub const fn last_reaped_child_wait_status(&self) -> usize {
        self.last_reaped_child_wait_status
    }

    pub const fn last_released_child_pid(&self) -> usize {
        self.last_released_child_pid
    }

    pub const fn last_released_child_wait_status(&self) -> usize {
        self.last_released_child_wait_status
    }

    pub const fn vfork_next_child_accepted(&self) -> bool {
        self.vfork_next_child_accepted
    }

    pub fn first_unreaped_completed_child(&self) -> Option<(usize, usize, usize, usize)> {
        let mut index = 0usize;
        while index < USER_COMPLETED_CHILD_RECORD_CAPACITY {
            let record = self.completed_child_records[index];
            if record.occupied && !record.reaped {
                return Some((
                    record.pid,
                    record.exit_status,
                    record.wait_status,
                    record.pidfd_fd,
                ));
            }
            index += 1;
        }
        None
    }

    pub const fn vfork_pidfd_clone(&self) -> bool {
        self.vfork_pidfd_clone
    }

    pub const fn vfork_vm_clone(&self) -> bool {
        self.vfork_vm_clone
    }

    pub const fn vfork_clone(&self) -> bool {
        self.vfork_vm_clone || self.vfork_pidfd_clone
    }

    pub const fn vfork_parent_frame_saved(&self) -> bool {
        self.vfork_parent_frame_saved
    }

    pub const fn vfork_child_handoff(&self) -> bool {
        self.vfork_child_handoff
    }

    pub const fn vfork_parent_resumed(&self) -> bool {
        self.vfork_parent_resumed
    }

    pub const fn vfork_child_sp(&self) -> usize {
        self.vfork_child_sp
    }

    pub const fn nested_vfork_clone(&self) -> bool {
        self.nested_vfork_clone
    }

    pub const fn nested_vfork_parent_pid(&self) -> usize {
        self.nested_vfork_parent_pid
    }

    pub const fn vfork_parent_resume_on_exit(&self) -> bool {
        self.vfork_parent_resume_on_exit
    }

    pub fn nested_vfork_copy_ready(&self) -> bool {
        self.active_task_state() == State::OnCpu
            && self.current_child_continuation()
            && self.vfork_clone()
            && !self.nested_vfork_clone
            && !self.active_task_record_available
            && self.pid != 0
            && self.task_entry == TaskEntry::UserChild
    }

    pub const fn pidfd_fd(&self) -> usize {
        self.pidfd_fd
    }

    pub const fn pidfd_copyout_observed(&self) -> bool {
        self.pidfd_copyout
    }

    pub const fn parent_clone_return(&self) -> usize {
        self.parent_clone_return
    }

    pub const fn user_stack_snapshot_copied(&self) -> bool {
        self.user_stack_snapshot_copied
    }

    pub const fn user_stack_snapshot_restored(&self) -> bool {
        self.user_stack_snapshot_restored
    }

    pub const fn parent_address_space_snapshot_saved(&self) -> bool {
        self.parent_address_space_snapshot_saved
    }

    pub fn runtime_exec_parent_snapshot_live(&self, address_space: &UserAddressSpace) -> bool {
        self.parent_address_space_snapshot_saved
            && self.current_child_continuation()
            && address_space.satp_token() != 0
            && self.parent_address_space_snapshot.satp_token() == address_space.satp_token()
    }

    pub fn can_retain_parent_exec_objects(
        &self,
        address_space: &UserAddressSpace,
        stack: &UserStack,
    ) -> bool {
        self.runtime_exec_parent_snapshot_live(address_space)
            && stack.state() == State::Ready
            && address_space
                .stack_mapping()
                .is_some_and(|mapping| mapping.stack_ownership_token() == stack.top())
    }

    pub fn retain_parent_exec_objects(
        &mut self,
        address_space: &UserAddressSpace,
        stack: &mut UserStack,
    ) -> bool {
        if !self.can_retain_parent_exec_objects(address_space, stack) {
            return false;
        }
        core::mem::swap(stack, &mut self.parent_exec_stack_snapshot);
        self.parent_exec_stack_snapshot_saved = true;
        true
    }

    pub const fn parent_wait_frame_saved(&self) -> bool {
        self.parent_wait_frame.is_some()
    }

    pub const fn parent_wait_register_checkpoint_bound(&self) -> bool {
        self.parent_wait_frame.is_some() && self.parent_address_space_snapshot_saved
    }

    pub const fn parent_wait_stack_window_checkpoint_bound(&self) -> bool {
        self.parent_wait_stack_window_saved && self.parent_wait_stack_window_len != 0
    }

    pub const fn parent_wait_stack_snapshot_copied(&self) -> bool {
        self.parent_wait_stack_snapshot_copied && self.parent_wait_stack_snapshot_len != 0
    }

    pub const fn parent_wait_stack_snapshot_restored(&self) -> bool {
        self.parent_wait_stack_snapshot_restored
    }

    pub const fn parent_wait_stack_window_compared(&self) -> bool {
        self.parent_wait_stack_window_compared
    }

    pub const fn parent_wait_stack_window_start(&self) -> usize {
        self.parent_wait_stack_window_start
    }

    pub const fn parent_wait_stack_window_len(&self) -> usize {
        self.parent_wait_stack_window_len
    }

    pub const fn parent_wait_stack_window_diff_count(&self) -> usize {
        self.parent_wait_stack_window_diff_count
    }

    pub const fn parent_wait_stack_window_first_diff_addr(&self) -> usize {
        self.parent_wait_stack_window_first_diff_addr
    }

    pub const fn parent_wait_stack_window_before_byte(&self) -> u8 {
        self.parent_wait_stack_window_before_byte
    }

    pub const fn parent_wait_stack_window_after_byte(&self) -> u8 {
        self.parent_wait_stack_window_after_byte
    }

    pub const fn parent_wait_writable_page_snapshot_saved(&self) -> bool {
        self.parent_wait_writable_page_snapshot_saved
    }

    pub const fn parent_wait_writable_page_snapshot_copied(&self) -> bool {
        self.parent_wait_writable_page_snapshot_saved
            && self.parent_wait_writable_page_count != 0
            && !self.parent_wait_writable_page_snapshot_truncated
    }

    pub const fn parent_wait_writable_page_count(&self) -> usize {
        self.parent_wait_writable_page_count
    }

    pub const fn parent_wait_writable_page_snapshot_truncated(&self) -> bool {
        self.parent_wait_writable_page_snapshot_truncated
    }

    pub const fn parent_wait_writable_page_snapshot_restored(&self) -> bool {
        self.parent_wait_writable_page_snapshot_restored
    }

    pub const fn parent_wait_writable_page_compared(&self) -> bool {
        self.parent_wait_writable_page_compared
    }

    pub const fn parent_wait_writable_page_dirty_count(&self) -> usize {
        self.parent_wait_writable_page_dirty_count
    }

    pub const fn parent_wait_writable_page_stack_dirty_count(&self) -> usize {
        self.parent_wait_writable_page_stack_dirty_count
    }

    pub const fn parent_wait_writable_page_non_stack_dirty_count(&self) -> usize {
        self.parent_wait_writable_page_non_stack_dirty_count
    }

    pub const fn parent_wait_first_non_stack_dirty_kind(&self) -> usize {
        self.parent_wait_first_non_stack_dirty_kind
    }

    pub const fn parent_wait_first_non_stack_dirty_mapping_index(&self) -> usize {
        self.parent_wait_first_non_stack_dirty_mapping_index
    }

    pub const fn parent_wait_first_non_stack_dirty_page_index(&self) -> usize {
        self.parent_wait_first_non_stack_dirty_page_index
    }

    pub const fn parent_wait_first_non_stack_dirty_addr(&self) -> usize {
        self.parent_wait_first_non_stack_dirty_addr
    }

    pub const fn parent_wait_first_non_stack_dirty_before_checksum(&self) -> usize {
        self.parent_wait_first_non_stack_dirty_before_checksum
    }

    pub const fn parent_wait_first_non_stack_dirty_after_checksum(&self) -> usize {
        self.parent_wait_first_non_stack_dirty_after_checksum
    }

    pub const fn child_exit_status(&self) -> usize {
        self.child_exit_status
    }

    pub const fn child_exit_status_observed(&self) -> bool {
        self.child_exit_status_observed
    }

    pub const fn wait4_status_copied(&self) -> bool {
        self.wait4_status_copied
    }

    pub const fn parent_wait_resumed(&self) -> bool {
        self.parent_wait_resumed
    }

    pub const fn parent_wait_resume_checkpoint_bound(&self) -> bool {
        self.parent_wait_resumed && self.child_exit_status_observed
    }

    pub fn setup(&mut self, carrier: &KernelInitTask) -> EventResult {
        self.setup_with_carrier_stack(
            carrier,
            carrier.task().kernel_stack_base(),
            carrier.task().kernel_stack_top(),
        )
    }

    pub fn setup_with_carrier_stack(
        &mut self,
        carrier: &KernelInitTask,
        carrier_stack_base: usize,
        carrier_stack_top: usize,
    ) -> EventResult {
        if self.set_lifecycle.state() != State::Base
            || self.active_task_state() != State::Base
            || carrier.state() != State::OnCpu
            || !carrier.current_stack_pointer_in_range()
            || carrier_stack_base == 0
            || carrier_stack_top <= carrier_stack_base
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.set_lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.prepared = true;
        self.carrier_stack_base = carrier_stack_base;
        self.carrier_stack_top = carrier_stack_top;
        self.task_entry = TaskEntry::UserChild;
        self.task_entry_bound = true;
        self.active_task_record_available = true;
        self.set_lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    // Plain-fork copy validates the complete specified parent and child task snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_plain_fork_from_parent(
        &mut self,
        parent: &KernelInitTaskUserState,
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
        if self.active_task_state() != State::Prepared
            || !self.prepared
            || !self.active_task_record_available
            || self.free_task_slot_count() == 0
            || self.task_entry != TaskEntry::UserChild
            || !parent.active_user_flow_online()
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
        let parent_fd_snapshot = files_struct.save_parent_fd_snapshot().ok()?;

        let child_pid = self.next_child_pid;
        self.allocate_user_task(child_pid, TaskRef::NONE)?;
        self.pid = child_pid;
        self.parent_pid = super::rest_init::KERNEL_INIT_PID;
        self.tgid = child_pid;
        self.exit_signal = boundaries.exit_signal(clone_flags);
        self.task_struct_allocated = task_struct_allocated;
        self.pid_allocated = true;
        self.thread_context_ready = thread_context_ready;
        self.sched_entity_ready = sched_entity_ready;
        self.task_state_new = task_state_new;
        self.files_struct_copied = true;
        self.parent_fd_snapshot = parent_fd_snapshot;
        self.parent_fd_snapshot_saved = true;
        self.parent_fd_snapshot_restored = false;
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
        self.parent_wait_frame = None;
        self.parent_wait_status_ptr = 0;
        self.parent_address_space_snapshot = UserAddressSpace::new();
        self.parent_address_space_snapshot_saved = false;
        self.parent_exec_stack_snapshot = UserStack::new();
        self.parent_exec_stack_snapshot_saved = false;
        self.parent_wait_stack_snapshot_len = 0;
        self.parent_wait_stack_snapshot_copied = false;
        self.parent_wait_stack_snapshot_restored = false;
        self.parent_wait_stack_window_start = 0;
        self.parent_wait_stack_window_len = 0;
        self.parent_wait_stack_window_saved = false;
        self.parent_wait_stack_window_compared = false;
        self.parent_wait_stack_window_diff_count = 0;
        self.parent_wait_stack_window_first_diff_addr = 0;
        self.parent_wait_stack_window_before_byte = 0;
        self.parent_wait_stack_window_after_byte = 0;
        self.parent_wait_writable_page_snapshot_pages =
            [None; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX];
        self.parent_wait_writable_page_checksums = [0; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX];
        self.parent_wait_writable_page_count = 0;
        self.parent_wait_writable_page_snapshot_saved = false;
        self.parent_wait_writable_page_snapshot_truncated = false;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;
        self.child_exit_status = 0;
        self.child_exit_status_observed = false;
        self.wait4_status_copied = false;
        self.parent_wait_resumed = false;
        self.vfork_vm_clone = false;
        self.vfork_pidfd_clone = false;
        self.vfork_parent_frame = None;
        self.vfork_parent_frame_saved = false;
        self.vfork_child_handoff = false;
        self.vfork_parent_resumed = false;
        self.vfork_child_sp = 0;
        self.nested_vfork_clone = false;
        self.nested_vfork_parent_pid = 0;
        self.vfork_parent_resume_on_exit = true;
        self.pidfd_fd = usize::MAX;
        self.pidfd_copyout = false;
        self.parent_clone_return = 0;
        self.clear_observed_plain_fork_state();
        self.active_task_record_available = false;
        self.vfork_next_child_accepted = false;
        self.next_child_pid = child_pid.saturating_add(1);

        self.task_allocation_count = self.task_allocation_count.wrapping_add(1);
        Some(self.pid)
    }

    // Vfork copy keeps the shared-VM, pidfd and saved-frame handoff inputs explicit.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_vfork_from_parent(
        &mut self,
        parent: &KernelInitTaskUserState,
        boundaries: &UserCloneDeferredBoundaries,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        fs_struct: &FsStruct,
        files_struct: &FilesStruct,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        current_frame: &TrapFrame,
        clone_flags: usize,
        newsp: usize,
        pidfd_fd: usize,
        pidfd_copyout: bool,
        task_struct_allocated: bool,
        thread_context_ready: bool,
        sched_entity_ready: bool,
        task_state_new: bool,
    ) -> Option<TrapFrame> {
        if self.active_task_state() != State::Prepared
            || !self.prepared
            || !self.active_task_record_available
            || self.free_task_slot_count() == 0
            || self.task_entry != TaskEntry::UserChild
            || !parent.active_user_flow_online()
            || !parent.pid1_preserved()
            || boundaries.state() != State::Ready
            || !(boundaries.accepts_vfork_pidfd_first_slice(clone_flags)
                || boundaries.accepts_vfork_vm_first_slice(clone_flags))
            || newsp == 0
            || (boundaries.accepts_vfork_pidfd_first_slice(clone_flags)
                && (!pidfd_copyout || pidfd_fd == usize::MAX))
            || (boundaries.accepts_vfork_vm_first_slice(clone_flags)
                && (pidfd_copyout || pidfd_fd != usize::MAX))
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
        child_frame.set_reg(2, newsp);
        let stack_snapshot_len = copy_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &mut self.user_stack_snapshot,
        )?;
        let parent_fd_snapshot = files_struct.save_parent_fd_snapshot().ok()?;

        unsafe {
            core::ptr::copy_nonoverlapping(
                address_space as *const UserAddressSpace,
                &mut self.parent_address_space_snapshot as *mut UserAddressSpace,
                1,
            );
        }
        let (writable_page_count, writable_page_truncated) = capture_writable_page_snapshot(
            address_space,
            page_allocator,
            page_metadata_map,
            &mut self.parent_wait_writable_page_snapshot_pages,
            &mut self.parent_wait_writable_page_checksums,
        )?;

        let child_pid = self.next_child_pid;
        self.allocate_user_task(child_pid, TaskRef::NONE)?;
        let next_child_accepted =
            self.completed_child_record_total_archived != 0 || self.task_allocation_count != 0;

        self.pid = child_pid;
        self.parent_pid = super::rest_init::KERNEL_INIT_PID;
        self.tgid = child_pid;
        self.exit_signal = boundaries.exit_signal(clone_flags);
        self.task_struct_allocated = task_struct_allocated;
        self.pid_allocated = true;
        self.thread_context_ready = thread_context_ready;
        self.sched_entity_ready = sched_entity_ready;
        self.task_state_new = task_state_new;
        self.files_struct_copied = true;
        self.parent_fd_snapshot = parent_fd_snapshot;
        self.parent_fd_snapshot_saved = true;
        self.parent_fd_snapshot_restored = false;
        self.fs_struct_copied = true;
        self.credentials_copied = parent.credentials_inherited();
        self.signal_state_copied = parent.signal_state_inherited();
        self.user_address_space_snapshot = true;
        self.user_stack_snapshot_len = stack_snapshot_len;
        self.user_stack_snapshot_copied = true;
        self.user_stack_snapshot_restored = true;
        self.trap_frame_copied = true;
        self.trap_frame_child_return_zero = child_frame.reg(10) == 0;
        self.tls_inherited = boundaries.tls_inherited_without_clone_settls(clone_flags);
        self.child_trap_frame = Some(child_frame);
        self.parent_wait_frame = None;
        self.parent_wait_status_ptr = 0;
        self.parent_address_space_snapshot_saved = true;
        self.parent_wait_stack_snapshot_len = 0;
        self.parent_wait_stack_snapshot_copied = false;
        self.parent_wait_stack_snapshot_restored = false;
        self.parent_wait_stack_window_start = 0;
        self.parent_wait_stack_window_len = 0;
        self.parent_wait_stack_window_saved = false;
        self.parent_wait_stack_window_compared = false;
        self.parent_wait_stack_window_diff_count = 0;
        self.parent_wait_stack_window_first_diff_addr = 0;
        self.parent_wait_stack_window_before_byte = 0;
        self.parent_wait_stack_window_after_byte = 0;
        self.parent_wait_writable_page_count = writable_page_count;
        self.parent_wait_writable_page_snapshot_saved = true;
        self.parent_wait_writable_page_snapshot_truncated = writable_page_truncated;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;
        self.child_exit_status = 0;
        self.child_exit_status_observed = false;
        self.wait4_status_copied = false;
        self.parent_wait_resumed = false;
        self.vfork_vm_clone = boundaries.accepts_vfork_vm_first_slice(clone_flags);
        self.vfork_pidfd_clone = boundaries.accepts_vfork_pidfd_first_slice(clone_flags);
        self.vfork_parent_frame = Some(*current_frame);
        self.vfork_parent_frame_saved = true;
        self.vfork_child_handoff = true;
        self.vfork_parent_resumed = false;
        self.vfork_child_sp = newsp;
        self.nested_vfork_clone = false;
        self.nested_vfork_parent_pid = 0;
        self.vfork_parent_resume_on_exit = true;
        self.pidfd_fd = pidfd_fd;
        self.pidfd_copyout = pidfd_copyout;
        self.parent_clone_return = 0;
        self.enqueued = false;
        self.wait4_parent_wait_observed = false;
        self.child_continuation_taken = true;
        self.clear_observed_plain_fork_state();
        self.active_task_record_available = false;
        self.vfork_next_child_accepted = next_child_accepted;
        self.next_child_pid = child_pid.saturating_add(1);

        self.task_allocation_count = self.task_allocation_count.wrapping_add(1);
        Some(child_frame)
    }

    // Nested vfork copy retains the full current-child and parent resume boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_nested_vfork_from_current_child(
        &mut self,
        parent: &KernelInitTaskUserState,
        boundaries: &UserCloneDeferredBoundaries,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        fs_struct: &FsStruct,
        files_struct: &FilesStruct,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        current_frame: &TrapFrame,
        clone_flags: usize,
        newsp: usize,
        task_struct_allocated: bool,
        thread_context_ready: bool,
        sched_entity_ready: bool,
        task_state_new: bool,
    ) -> Option<(TrapFrame, usize)> {
        if !self.nested_vfork_copy_ready()
            || self.free_task_slot_count() == 0
            || !parent.active_user_flow_online()
            || !parent.pid1_preserved()
            || boundaries.state() != State::Ready
            || !boundaries.accepts_vfork_vm_first_slice(clone_flags)
            || boundaries.accepts_vfork_pidfd_first_slice(clone_flags)
            || newsp == 0
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

        let parent_pid = self.pid;
        let parent_task_ref = self.active_task_ref;
        let child_pid = self.next_child_pid;
        let mut child_frame = *current_frame;
        child_frame.set_reg(10, 0);
        child_frame.sepc = child_frame.sepc.wrapping_add(4);
        child_frame.set_reg(2, newsp);
        let stack_snapshot_len = copy_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &mut self.user_stack_snapshot,
        )?;
        let parent_fd_snapshot = files_struct.save_parent_fd_snapshot().ok()?;

        if self.parent_wait_writable_page_snapshot_saved
            && !self.parent_wait_writable_page_snapshot_restored
            && self.parent_wait_writable_page_count != 0
        {
            release_writable_page_snapshot_pages(
                &mut self.parent_wait_writable_page_snapshot_pages,
                self.parent_wait_writable_page_count,
                page_allocator,
                page_metadata_map,
            );
            self.parent_wait_writable_page_count = 0;
            self.parent_wait_writable_page_snapshot_saved = false;
            self.parent_wait_writable_page_snapshot_truncated = false;
            self.parent_wait_writable_page_snapshot_restored = false;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                address_space as *const UserAddressSpace,
                &mut self.parent_address_space_snapshot as *mut UserAddressSpace,
                1,
            );
        }
        let (writable_page_count, writable_page_truncated) = capture_writable_page_snapshot(
            address_space,
            page_allocator,
            page_metadata_map,
            &mut self.parent_wait_writable_page_snapshot_pages,
            &mut self.parent_wait_writable_page_checksums,
        )?;

        self.allocate_user_task(child_pid, parent_task_ref)?;

        self.pid = child_pid;
        self.parent_pid = parent_pid;
        self.tgid = child_pid;
        self.exit_signal = boundaries.exit_signal(clone_flags);
        self.task_struct_allocated = task_struct_allocated;
        self.pid_allocated = true;
        self.thread_context_ready = thread_context_ready;
        self.sched_entity_ready = sched_entity_ready;
        self.task_state_new = task_state_new;
        self.files_struct_copied = true;
        self.parent_fd_snapshot = parent_fd_snapshot;
        self.parent_fd_snapshot_saved = true;
        self.parent_fd_snapshot_restored = false;
        self.fs_struct_copied = true;
        self.credentials_copied = parent.credentials_inherited();
        self.signal_state_copied = parent.signal_state_inherited();
        self.user_address_space_snapshot = true;
        self.user_stack_snapshot_len = stack_snapshot_len;
        self.user_stack_snapshot_copied = true;
        self.user_stack_snapshot_restored = true;
        self.trap_frame_copied = true;
        self.trap_frame_child_return_zero = child_frame.reg(10) == 0;
        self.tls_inherited = boundaries.tls_inherited_without_clone_settls(clone_flags);
        self.child_trap_frame = Some(child_frame);
        self.parent_wait_frame = None;
        self.parent_wait_status_ptr = 0;
        self.parent_address_space_snapshot_saved = true;
        self.parent_wait_stack_snapshot_len = 0;
        self.parent_wait_stack_snapshot_copied = false;
        self.parent_wait_stack_snapshot_restored = false;
        self.parent_wait_stack_window_start = 0;
        self.parent_wait_stack_window_len = 0;
        self.parent_wait_stack_window_saved = false;
        self.parent_wait_stack_window_compared = false;
        self.parent_wait_stack_window_diff_count = 0;
        self.parent_wait_stack_window_first_diff_addr = 0;
        self.parent_wait_stack_window_before_byte = 0;
        self.parent_wait_stack_window_after_byte = 0;
        self.parent_wait_writable_page_count = writable_page_count;
        self.parent_wait_writable_page_snapshot_saved = true;
        self.parent_wait_writable_page_snapshot_truncated = writable_page_truncated;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;
        self.child_exit_status = 0;
        self.child_exit_status_observed = false;
        self.wait4_status_copied = false;
        self.parent_wait_resumed = false;
        self.vfork_vm_clone = true;
        self.vfork_pidfd_clone = false;
        self.vfork_parent_frame = Some(*current_frame);
        self.vfork_parent_frame_saved = true;
        self.vfork_child_handoff = true;
        self.vfork_parent_resumed = false;
        self.vfork_child_sp = newsp;
        self.nested_vfork_clone = true;
        self.nested_vfork_parent_pid = parent_pid;
        self.vfork_parent_resume_on_exit = false;
        self.pidfd_fd = usize::MAX;
        self.pidfd_copyout = false;
        self.parent_clone_return = 0;
        self.enqueued = true;
        self.wait4_parent_wait_observed = false;
        self.child_continuation_taken = true;
        self.clear_observed_plain_fork_state();
        self.active_task_record_available = false;
        self.vfork_next_child_accepted = true;
        self.next_child_pid = child_pid.saturating_add(1);

        self.task_allocation_count = self.task_allocation_count.wrapping_add(1);
        Some((child_frame, parent_pid))
    }

    // Child-originated plain fork validates the full inherited task and address-space state.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_plain_fork_from_current_child(
        &mut self,
        parent: &KernelInitTaskUserState,
        boundaries: &UserCloneDeferredBoundaries,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        fs_struct: &FsStruct,
        files_struct: &FilesStruct,
        page_metadata_map: &PageMetadataMap,
        current_frame: &TrapFrame,
        clone_flags: usize,
        newsp: usize,
    ) -> Option<usize> {
        let builtin_grandchild_source = self.pid1_plain_fork_child_continuation();
        if self.active_task_state() != State::OnCpu
            || !self.current_child_continuation()
            || (!self.vfork_clone() && !builtin_grandchild_source)
            || self.observed_plain_fork_child_active
            || (self.observed_plain_fork_clone && !self.observed_plain_fork_parent_restored)
            || self.pid == 0
            || !parent.active_user_flow_online()
            || !parent.pid1_preserved()
            || boundaries.state() != State::Ready
            || !boundaries.accepts_plain_fork_first_slice(clone_flags, newsp)
            || address_space.state() != State::Online
            || trap_frame.state() != State::Ready
            || fs_struct.state() != State::Ready
            || files_struct.state() != State::Ready
            || !self.task_struct_allocated
            || !self.pid_allocated
            || !self.thread_context_ready
            || !self.sched_entity_ready
            || !self.task_state_new
            || !self.files_struct_copied
            || !self.fs_struct_copied
            || !self.credentials_copied
            || !self.signal_state_copied
            || self.free_task_slot_count() == 0
        {
            return None;
        }

        let parent_pid = self.pid;
        let parent_task_ref = self.active_task_ref;
        let parent_parent_pid = self.parent_pid;
        let parent_tgid = self.tgid;
        let child_pid = self.next_child_pid;
        let mut child_frame = *current_frame;
        child_frame.set_reg(10, 0);
        child_frame.sepc = child_frame.sepc.wrapping_add(4);
        if builtin_grandchild_source {
            self.builtin_grandchild.reset_empty();
            let stack_snapshot_len = copy_user_stack_snapshot(
                address_space,
                page_metadata_map,
                &mut self.builtin_grandchild.child_stack_snapshot,
            )?;
            let child_fd_snapshot = files_struct.save_parent_fd_snapshot().ok()?;
            self.builtin_grandchild.bound = true;
            self.builtin_grandchild.child_trap_frame = Some(child_frame);
            self.builtin_grandchild.child_stack_snapshot_len = stack_snapshot_len;
            self.builtin_grandchild.child_stack_snapshot_copied = true;
            self.builtin_grandchild.child_stack_snapshot_restored = false;
            self.builtin_grandchild.child_fd_snapshot = child_fd_snapshot;
            self.builtin_grandchild.child_fd_snapshot_saved = true;
            self.builtin_grandchild.child_fd_snapshot_restored = false;
        } else {
            let stack_snapshot_len = copy_user_stack_snapshot(
                address_space,
                page_metadata_map,
                &mut self.user_stack_snapshot,
            )?;
            let parent_fd_snapshot = files_struct.save_parent_fd_snapshot().ok()?;

            self.exit_signal = boundaries.exit_signal(clone_flags);
            self.parent_fd_snapshot = parent_fd_snapshot;
            self.parent_fd_snapshot_saved = true;
            self.parent_fd_snapshot_restored = false;
            self.user_address_space_snapshot = true;
            self.user_stack_snapshot_len = stack_snapshot_len;
            self.user_stack_snapshot_copied = true;
            self.user_stack_snapshot_restored = false;
            self.trap_frame_copied = true;
            self.trap_frame_child_return_zero = child_frame.reg(10) == 0;
            self.tls_inherited = boundaries.tls_inherited_without_clone_settls(clone_flags);
            self.child_trap_frame = Some(child_frame);
            self.parent_wait_frame = None;
            self.parent_wait_status_ptr = 0;
            self.parent_wait_stack_snapshot_len = 0;
            self.parent_wait_stack_snapshot_copied = false;
            self.parent_wait_stack_snapshot_restored = false;
            self.parent_wait_stack_window_start = 0;
            self.parent_wait_stack_window_len = 0;
            self.parent_wait_stack_window_saved = false;
            self.parent_wait_stack_window_compared = false;
            self.parent_wait_stack_window_diff_count = 0;
            self.parent_wait_stack_window_first_diff_addr = 0;
            self.parent_wait_stack_window_before_byte = 0;
            self.parent_wait_stack_window_after_byte = 0;
            self.parent_clone_return = child_pid;
        }
        self.allocate_user_task(child_pid, parent_task_ref)?;
        self.pending_task_ref = self.active_task_ref;
        self.active_task_ref = parent_task_ref;
        self.child_exit_status = 0;
        self.child_exit_status_observed = false;
        self.wait4_status_copied = false;
        self.observed_plain_fork_clone = true;
        self.observed_plain_fork_parent_pid = parent_pid;
        self.observed_plain_fork_parent_parent_pid = parent_parent_pid;
        self.observed_plain_fork_parent_tgid = parent_tgid;
        self.observed_plain_fork_child_pid = child_pid;
        self.observed_plain_fork_child_active = false;
        self.observed_plain_fork_parent_restored = false;
        self.next_child_pid = child_pid.saturating_add(1);
        self.task_allocation_count = self.task_allocation_count.wrapping_add(1);
        Some(child_pid)
    }

    pub fn mark_enqueued(&mut self) -> bool {
        if self.active_task_state() != State::Online
            || self.pid == 0
            || self.active_task_record_available
        {
            return false;
        }
        self.enqueued = true;
        true
    }

    // This handoff records the complete specified parent wait and child continuation boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn wait4_yield_to_child_continuation(
        &mut self,
        parent: &KernelInitTaskUserState,
        address_space: &UserAddressSpace,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        parent_wait_frame: &TrapFrame,
        stat_addr: usize,
        upid: usize,
        options: usize,
        rusage: usize,
    ) -> Option<TrapFrame> {
        if self.active_task_state() != State::Online
            || !self.enqueued
            || self.child_continuation_taken
            || self.pid == 0
            || self.parent_pid != super::rest_init::KERNEL_INIT_PID
            || self.tgid != self.pid
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
            || self.parent_wait_resumed
            || self.pid == 0
            || !parent.active_user_flow_online()
            || address_space.state() != State::Online
            || !parent.pid1_preserved()
            || upid != USER_WAIT4_ALL_CHILDREN
            || (options != 0 && options != USER_WAIT4_WUNTRACED)
            || rusage != 0
        {
            return None;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                address_space as *const UserAddressSpace,
                &mut self.parent_address_space_snapshot as *mut UserAddressSpace,
                1,
            );
        }
        self.parent_wait_frame = Some(*parent_wait_frame);
        self.parent_wait_status_ptr = stat_addr;
        self.parent_address_space_snapshot_saved = true;

        if !self.capture_parent_wait_stack_window(
            address_space,
            page_metadata_map,
            parent_wait_frame.reg(2),
        ) {
            return None;
        }
        let parent_stack_snapshot_len = copy_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &mut self.parent_wait_stack_snapshot,
        )?;
        self.parent_wait_stack_snapshot_len = parent_stack_snapshot_len;
        self.parent_wait_stack_snapshot_copied = true;
        self.parent_wait_stack_snapshot_restored = false;

        let (writable_page_count, writable_page_truncated) = capture_writable_page_snapshot(
            address_space,
            page_allocator,
            page_metadata_map,
            &mut self.parent_wait_writable_page_snapshot_pages,
            &mut self.parent_wait_writable_page_checksums,
        )?;
        self.parent_wait_writable_page_count = writable_page_count;
        self.parent_wait_writable_page_snapshot_saved = true;
        self.parent_wait_writable_page_snapshot_truncated = writable_page_truncated;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;

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

    // Observed-fork handoff retains all parent wait and snapshot inputs explicitly.
    #[allow(clippy::too_many_arguments)]
    pub fn wait4_yield_to_observed_child_continuation(
        &mut self,
        parent: &KernelInitTaskUserState,
        address_space: &UserAddressSpace,
        fs_struct: &mut FsStruct,
        files_struct: &mut FilesStruct,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        parent_wait_frame: &TrapFrame,
        stat_addr: usize,
        upid: usize,
        rusage: usize,
    ) -> Option<(TrapFrame, usize, usize)> {
        if self.builtin_grandchild.bound {
            return self.wait4_yield_to_builtin_grandchild_continuation(
                parent,
                address_space,
                fs_struct,
                files_struct,
                page_allocator,
                page_metadata_map,
                parent_wait_frame,
                stat_addr,
                upid,
                rusage,
            );
        }
        if !self.observed_plain_fork_child_pending_wait()
            || !parent.active_user_flow_online()
            || address_space.state() != State::Online
            || !parent.pid1_preserved()
            || upid != USER_WAIT4_ALL_CHILDREN
            || rusage != 0
            || !self.user_stack_snapshot_copied
            || self.user_stack_snapshot_len == 0
            || !self.trap_frame_copied
            || !self.trap_frame_child_return_zero
            || self.child_trap_frame.is_none()
        {
            return None;
        }

        if self.parent_wait_writable_page_snapshot_saved
            && !self.parent_wait_writable_page_snapshot_restored
            && self.parent_wait_writable_page_count != 0
        {
            release_writable_page_snapshot_pages(
                &mut self.parent_wait_writable_page_snapshot_pages,
                self.parent_wait_writable_page_count,
                page_allocator,
                page_metadata_map,
            );
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                address_space as *const UserAddressSpace,
                &mut self.parent_address_space_snapshot as *mut UserAddressSpace,
                1,
            );
        }
        self.parent_wait_frame = Some(*parent_wait_frame);
        self.parent_wait_status_ptr = stat_addr;
        self.parent_address_space_snapshot_saved = true;

        if !self.capture_parent_wait_stack_window(
            address_space,
            page_metadata_map,
            parent_wait_frame.reg(2),
        ) {
            return None;
        }
        let parent_stack_snapshot_len = copy_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &mut self.parent_wait_stack_snapshot,
        )?;
        self.parent_wait_stack_snapshot_len = parent_stack_snapshot_len;
        self.parent_wait_stack_snapshot_copied = true;
        self.parent_wait_stack_snapshot_restored = false;

        let (writable_page_count, writable_page_truncated) = capture_writable_page_snapshot(
            address_space,
            page_allocator,
            page_metadata_map,
            &mut self.parent_wait_writable_page_snapshot_pages,
            &mut self.parent_wait_writable_page_checksums,
        )?;
        self.parent_wait_writable_page_count = writable_page_count;
        self.parent_wait_writable_page_snapshot_saved = true;
        self.parent_wait_writable_page_snapshot_truncated = writable_page_truncated;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;

        if !restore_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &self.user_stack_snapshot,
            self.user_stack_snapshot_len,
        ) {
            return None;
        }

        let child_frame = self.child_trap_frame?;
        let parent_pid = self.observed_plain_fork_parent_pid;
        let child_pid = self.observed_plain_fork_child_pid;
        self.wait4_parent_wait_observed = true;
        self.user_stack_snapshot_restored = true;
        self.observed_plain_fork_child_active = true;
        self.active_task_ref = self.pending_task_ref;
        self.pending_task_ref = TaskRef::NONE;
        self.pid = child_pid;
        self.parent_pid = parent_pid;
        self.tgid = child_pid;
        Some((child_frame, parent_pid, child_pid))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn pipe_read_yield_to_builtin_grandchild_continuation(
        &mut self,
        parent: &KernelInitTaskUserState,
        address_space: &UserAddressSpace,
        fs_struct: &mut FsStruct,
        files_struct: &mut FilesStruct,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        parent_read_frame: &TrapFrame,
    ) -> Option<(TrapFrame, usize, usize)> {
        let handoff = self.wait4_yield_to_builtin_grandchild_continuation(
            parent,
            address_space,
            fs_struct,
            files_struct,
            page_allocator,
            page_metadata_map,
            parent_read_frame,
            0,
            USER_WAIT4_ALL_CHILDREN,
            0,
        )?;
        self.builtin_grandchild.parent_resume = BuiltinGrandchildParentResume::PipeRead;
        Some(handoff)
    }

    #[allow(clippy::too_many_arguments)]
    fn wait4_yield_to_builtin_grandchild_continuation(
        &mut self,
        parent: &KernelInitTaskUserState,
        address_space: &UserAddressSpace,
        fs_struct: &mut FsStruct,
        files_struct: &mut FilesStruct,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        parent_wait_frame: &TrapFrame,
        stat_addr: usize,
        upid: usize,
        rusage: usize,
    ) -> Option<(TrapFrame, usize, usize)> {
        if !self.observed_plain_fork_child_pending_wait()
            || !self.pid1_plain_fork_child_continuation()
            || !parent.active_user_flow_online()
            || address_space.state() != State::Online
            || fs_struct.state() != State::Ready
            || files_struct.state() != State::Ready
            || !parent.pid1_preserved()
            || upid != USER_WAIT4_ALL_CHILDREN
            || rusage != 0
            || !self.builtin_grandchild.child_stack_snapshot_copied
            || self.builtin_grandchild.child_stack_snapshot_len == 0
            || !self.builtin_grandchild.child_fd_snapshot_saved
            || self.builtin_grandchild.child_fd_snapshot_restored
            || self.builtin_grandchild.child_trap_frame.is_none()
            || self.builtin_grandchild.parent_wait_frame.is_some()
            || self.builtin_grandchild.parent_fd_snapshot_saved
            || self.builtin_grandchild.parent_writable_page_snapshot_saved
        {
            return None;
        }

        let parent_stack_snapshot_len = copy_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &mut self.builtin_grandchild.parent_stack_snapshot,
        )?;
        let (writable_page_count, writable_page_truncated) = capture_writable_page_snapshot(
            address_space,
            page_allocator,
            page_metadata_map,
            &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
            &mut self.builtin_grandchild.parent_writable_page_checksums,
        )?;
        #[cfg(app_smoke)]
        let inject_capture_failure =
            core::mem::take(&mut self.fail_next_builtin_grandchild_wait_capture);
        #[cfg(not(app_smoke))]
        let inject_capture_failure = false;
        if inject_capture_failure || writable_page_truncated || writable_page_count == 0 {
            release_writable_page_snapshot_pages(
                &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
                writable_page_count,
                page_allocator,
                page_metadata_map,
            );
            return None;
        }

        let parent_fs_snapshot = match fs_struct.save_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                release_writable_page_snapshot_pages(
                    &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
                    writable_page_count,
                    page_allocator,
                    page_metadata_map,
                );
                return None;
            }
        };
        let parent_fd_snapshot = match files_struct.save_parent_fd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                release_writable_page_snapshot_pages(
                    &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
                    writable_page_count,
                    page_allocator,
                    page_metadata_map,
                );
                return None;
            }
        };
        if !restore_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &self.builtin_grandchild.child_stack_snapshot,
            self.builtin_grandchild.child_stack_snapshot_len,
        ) {
            let _ = restore_user_stack_snapshot(
                address_space,
                page_metadata_map,
                &self.builtin_grandchild.parent_stack_snapshot,
                parent_stack_snapshot_len,
            );
            files_struct.discard_parent_fd_snapshot(&parent_fd_snapshot);
            release_writable_page_snapshot_pages(
                &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
                writable_page_count,
                page_allocator,
                page_metadata_map,
            );
            return None;
        }
        if files_struct
            .restore_parent_fd_snapshot(&self.builtin_grandchild.child_fd_snapshot)
            .is_err()
        {
            let _ = restore_user_stack_snapshot(
                address_space,
                page_metadata_map,
                &self.builtin_grandchild.parent_stack_snapshot,
                parent_stack_snapshot_len,
            );
            files_struct.discard_parent_fd_snapshot(&parent_fd_snapshot);
            release_writable_page_snapshot_pages(
                &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
                writable_page_count,
                page_allocator,
                page_metadata_map,
            );
            return None;
        }

        self.builtin_grandchild.parent_wait_frame = Some(*parent_wait_frame);
        self.builtin_grandchild.parent_wait_status_ptr = stat_addr;
        self.builtin_grandchild.parent_resume = BuiltinGrandchildParentResume::Wait4;
        self.builtin_grandchild.parent_fs_snapshot = Some(parent_fs_snapshot);
        self.builtin_grandchild.parent_fs_snapshot_restored = false;
        self.builtin_grandchild.parent_fd_snapshot = parent_fd_snapshot;
        self.builtin_grandchild.parent_fd_snapshot_saved = true;
        self.builtin_grandchild.parent_fd_snapshot_restored = false;
        self.builtin_grandchild.parent_stack_snapshot_len = parent_stack_snapshot_len;
        self.builtin_grandchild.parent_stack_snapshot_copied = true;
        self.builtin_grandchild.parent_stack_snapshot_restored = false;
        self.builtin_grandchild.parent_writable_page_count = writable_page_count;
        self.builtin_grandchild.parent_writable_page_snapshot_saved = true;
        self.builtin_grandchild
            .parent_writable_page_snapshot_truncated = false;
        self.builtin_grandchild
            .parent_writable_page_snapshot_restored = false;
        self.builtin_grandchild.child_stack_snapshot_restored = true;
        self.builtin_grandchild.child_fd_snapshot_restored = true;

        let child_frame = self.builtin_grandchild.child_trap_frame?;
        let parent_pid = self.observed_plain_fork_parent_pid;
        let child_pid = self.observed_plain_fork_child_pid;
        self.observed_plain_fork_child_active = true;
        self.active_task_ref = self.pending_task_ref;
        self.pending_task_ref = TaskRef::NONE;
        self.pid = child_pid;
        self.parent_pid = parent_pid;
        self.tgid = child_pid;
        Some((child_frame, parent_pid, child_pid))
    }

    pub fn child_exit_to_parent_wait(
        &mut self,
        address_space: &mut UserAddressSpace,
        stack: &mut UserStack,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        exit_status: usize,
    ) -> Option<(TrapFrame, usize, usize)> {
        if self.active_task_state() != State::OnCpu
            || !self.child_continuation_taken
            || self.parent_wait_resumed
            || !self.parent_address_space_snapshot_saved
            || self.pid == 0
            || self.parent_pid != super::rest_init::KERNEL_INIT_PID
        {
            return None;
        }

        let parent_frame = self.parent_wait_frame?;
        if !self.restore_parent_exec_objects(
            address_space,
            stack,
            page_allocator,
            page_metadata_map,
        ) {
            return None;
        }
        self.child_exit_status = exit_status;
        self.child_exit_status_observed = true;
        if !self.destroy_active_user_task() {
            return None;
        }
        Some((parent_frame, self.parent_wait_status_ptr, self.pid))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn child_exit_to_observed_child_parent_wait(
        &mut self,
        address_space: &mut UserAddressSpace,
        stack: &mut UserStack,
        retained_exec_objects: (&mut UserAddressSpace, &mut UserStack),
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        exit_status: usize,
    ) -> Option<(TrapFrame, usize, usize, usize)> {
        let (retained_address_space, retained_stack) = retained_exec_objects;
        if self.builtin_grandchild.bound {
            if self.active_task_state() != State::OnCpu
                || !self.observed_plain_fork_clone
                || !self.observed_plain_fork_child_active
                || self.observed_plain_fork_parent_restored
                || self.pid == 0
                || self.pid != self.observed_plain_fork_child_pid
                || self.parent_pid != self.observed_plain_fork_parent_pid
                || !self.builtin_grandchild.child_stack_snapshot_restored
                || !self.builtin_grandchild.child_fd_snapshot_restored
                || !self.builtin_grandchild.parent_stack_snapshot_copied
                || !self.builtin_grandchild.parent_writable_page_snapshot_saved
                || !self.builtin_grandchild.parent_fd_snapshot_saved
            {
                return None;
            }
            let parent_frame = self.builtin_grandchild.parent_wait_frame?;
            let child_pid = self.observed_plain_fork_child_pid;
            let parent_pid = self.observed_plain_fork_parent_pid;
            if !self.restore_builtin_grandchild_exec_objects(
                address_space,
                stack,
                retained_address_space,
                retained_stack,
                page_allocator,
                page_metadata_map,
            ) {
                return None;
            }
            self.child_exit_status = exit_status;
            self.child_exit_status_observed = true;
            if !self.destroy_active_user_task() {
                return None;
            }
            return Some((
                parent_frame,
                self.builtin_grandchild.parent_wait_status_ptr,
                child_pid,
                parent_pid,
            ));
        }
        if self.active_task_state() != State::OnCpu
            || !self.observed_plain_fork_clone
            || !self.observed_plain_fork_child_active
            || self.observed_plain_fork_parent_restored
            || !self.parent_address_space_snapshot_saved
            || self.pid == 0
            || self.pid != self.observed_plain_fork_child_pid
            || self.parent_pid != self.observed_plain_fork_parent_pid
        {
            return None;
        }

        let parent_frame = self.parent_wait_frame?;
        let child_pid = self.observed_plain_fork_child_pid;
        let parent_pid = self.observed_plain_fork_parent_pid;
        if !self.restore_parent_exec_objects(
            address_space,
            stack,
            page_allocator,
            page_metadata_map,
        ) {
            return None;
        }
        self.child_exit_status = exit_status;
        self.child_exit_status_observed = true;
        if !self.destroy_active_user_task() {
            return None;
        }
        Some((
            parent_frame,
            self.parent_wait_status_ptr,
            child_pid,
            parent_pid,
        ))
    }

    pub fn child_exit_to_vfork_parent(
        &mut self,
        address_space: &mut UserAddressSpace,
        stack: &mut UserStack,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        exit_status: usize,
    ) -> Option<(TrapFrame, usize)> {
        if self.active_task_state() != State::OnCpu
            || !self.vfork_clone()
            || !self.vfork_parent_resume_on_exit
            || !self.child_continuation_taken
            || !self.vfork_parent_frame_saved
            || self.vfork_parent_resumed
            || !self.parent_address_space_snapshot_saved
            || self.pid == 0
            || self.parent_pid != super::rest_init::KERNEL_INIT_PID
        {
            return None;
        }

        let parent_frame = self.vfork_parent_frame?;
        if !self.restore_parent_exec_objects(
            address_space,
            stack,
            page_allocator,
            page_metadata_map,
        ) {
            return None;
        }
        self.child_exit_status = exit_status;
        self.child_exit_status_observed = true;
        if !self.destroy_active_user_task() {
            return None;
        }
        Some((parent_frame, self.pid))
    }

    fn restore_parent_exec_objects(
        &mut self,
        address_space: &mut UserAddressSpace,
        stack: &mut UserStack,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if address_space.satp_token() == self.parent_address_space_snapshot.satp_token() {
            return true;
        }
        if address_space.satp_token() == 0
            || self.parent_address_space_snapshot.satp_token() == 0
            || !self.parent_exec_stack_snapshot_saved
        {
            return false;
        }

        address_space.release_retired_exec_image(page_allocator, page_metadata_map);
        stack.release_exec_backing(page_allocator, page_metadata_map);
        unsafe {
            core::ptr::copy_nonoverlapping(
                &self.parent_address_space_snapshot as *const UserAddressSpace,
                address_space as *mut UserAddressSpace,
                1,
            );
        }
        core::mem::swap(stack, &mut self.parent_exec_stack_snapshot);
        self.parent_exec_stack_snapshot_saved = false;
        true
    }

    fn restore_builtin_grandchild_exec_objects(
        &mut self,
        address_space: &mut UserAddressSpace,
        stack: &mut UserStack,
        retained_address_space: &mut UserAddressSpace,
        retained_stack: &mut UserStack,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if !self.builtin_grandchild_active()
            || self.builtin_grandchild.parent_exec_snapshot_restored
        {
            return false;
        }
        if self.builtin_grandchild.exec_commit_count == 0 {
            if self.builtin_grandchild.parent_exec_snapshot_ever_saved
                || self
                    .builtin_grandchild
                    .parent_exec_address_space_snapshot_saved
                || self.builtin_grandchild.parent_exec_stack_snapshot_saved
            {
                return false;
            }
            self.builtin_grandchild.parent_exec_snapshot_restored = true;
            return true;
        }
        if !self.builtin_grandchild_exec_parent_snapshot_live()
            || address_space.satp_token() == 0
            || retained_address_space.satp_token()
                != self.builtin_grandchild.parent_exec_retained_satp
            || retained_stack.state() != State::Ready
            || self
                .builtin_grandchild
                .parent_exec_storage
                .address_space_ptr
                != retained_address_space as *mut UserAddressSpace as usize
            || self.builtin_grandchild.parent_exec_storage.stack_ptr
                != retained_stack as *mut UserStack as usize
            || self.builtin_grandchild.parent_exec_storage.parent_satp
                != retained_address_space.satp_token()
            || self.builtin_grandchild.parent_exec_storage.parent_stack_top != retained_stack.top()
        {
            return false;
        }

        address_space.release_retired_exec_image(page_allocator, page_metadata_map);
        stack.release_exec_backing(page_allocator, page_metadata_map);
        unsafe {
            core::ptr::copy_nonoverlapping(
                retained_address_space as *const UserAddressSpace,
                address_space as *mut UserAddressSpace,
                1,
            );
        }
        retained_address_space.reset_staging_after_exec_commit();
        core::mem::swap(stack, retained_stack);
        retained_stack.reset_staging_after_exec_commit();
        self.builtin_grandchild
            .parent_exec_address_space_snapshot_saved = false;
        self.builtin_grandchild.parent_exec_stack_snapshot_saved = false;
        self.builtin_grandchild.parent_exec_snapshot_restored = true;
        self.builtin_grandchild.parent_exec_storage.restore_count = 1;
        true
    }

    pub fn compare_parent_wait_stack_window(
        &mut self,
        address_space: &UserAddressSpace,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if self.builtin_grandchild_active() {
            return false;
        }
        if !self.parent_wait_stack_window_saved
            || self.parent_wait_stack_window_len == 0
            || self.parent_wait_stack_window_len > USER_PARENT_WAIT_STACK_PROBE_LEN
        {
            return false;
        }
        let Some(mapping) = address_space.stack_mapping() else {
            return false;
        };
        if mapping.kind() != UserMappingKind::Stack {
            return false;
        }
        let Some(window_end) = self
            .parent_wait_stack_window_start
            .checked_add(self.parent_wait_stack_window_len)
        else {
            return false;
        };
        if self.parent_wait_stack_window_start < mapping.vaddr() || window_end > mapping.end_vaddr()
        {
            return false;
        }

        let mut diff_count = 0usize;
        let mut first_diff_addr = 0usize;
        let mut first_before = 0u8;
        let mut first_after = 0u8;
        let mut index = 0usize;
        let stack = &crate::context::context_ref().user_stack;
        while index < self.parent_wait_stack_window_len {
            let addr = self.parent_wait_stack_window_start + index;
            let Some(after) = stack.read_byte(page_metadata_map, addr) else {
                return false;
            };
            let before = self.parent_wait_stack_window[index];
            if before != after {
                diff_count = diff_count.wrapping_add(1);
                if first_diff_addr == 0 {
                    first_diff_addr = addr;
                    first_before = before;
                    first_after = after;
                }
            }
            index += 1;
        }

        self.parent_wait_stack_window_compared = true;
        self.parent_wait_stack_window_diff_count = diff_count;
        self.parent_wait_stack_window_first_diff_addr = first_diff_addr;
        self.parent_wait_stack_window_before_byte = first_before;
        self.parent_wait_stack_window_after_byte = first_after;
        true
    }

    pub fn compare_parent_wait_writable_pages(
        &mut self,
        address_space: &UserAddressSpace,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if self.builtin_grandchild_active() {
            if !self.builtin_grandchild.parent_writable_page_snapshot_saved
                || self.builtin_grandchild.parent_writable_page_count == 0
            {
                return false;
            }
            let Some(diff) = compare_writable_page_checksums(
                address_space,
                page_metadata_map,
                &self.builtin_grandchild.parent_writable_page_checksums,
                self.builtin_grandchild.parent_writable_page_count,
            ) else {
                return false;
            };
            self.parent_wait_writable_page_compared = true;
            self.parent_wait_writable_page_dirty_count = diff.dirty_count;
            self.parent_wait_writable_page_stack_dirty_count = diff.stack_dirty_count;
            self.parent_wait_writable_page_non_stack_dirty_count = diff.non_stack_dirty_count;
            self.parent_wait_first_non_stack_dirty_kind = diff.first_non_stack_kind;
            self.parent_wait_first_non_stack_dirty_mapping_index =
                diff.first_non_stack_mapping_index;
            self.parent_wait_first_non_stack_dirty_page_index = diff.first_non_stack_page_index;
            self.parent_wait_first_non_stack_dirty_addr = diff.first_non_stack_addr;
            self.parent_wait_first_non_stack_dirty_before_checksum = diff.first_non_stack_before;
            self.parent_wait_first_non_stack_dirty_after_checksum = diff.first_non_stack_after;
            return true;
        }
        if !self.parent_wait_writable_page_snapshot_saved
            || self.parent_wait_writable_page_count == 0
            || self.parent_wait_writable_page_count > USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX
        {
            return false;
        }
        let Some(diff) = compare_writable_page_checksums(
            address_space,
            page_metadata_map,
            &self.parent_wait_writable_page_checksums,
            self.parent_wait_writable_page_count,
        ) else {
            return false;
        };

        self.parent_wait_writable_page_compared = true;
        self.parent_wait_writable_page_dirty_count = diff.dirty_count;
        self.parent_wait_writable_page_stack_dirty_count = diff.stack_dirty_count;
        self.parent_wait_writable_page_non_stack_dirty_count = diff.non_stack_dirty_count;
        self.parent_wait_first_non_stack_dirty_kind = diff.first_non_stack_kind;
        self.parent_wait_first_non_stack_dirty_mapping_index = diff.first_non_stack_mapping_index;
        self.parent_wait_first_non_stack_dirty_page_index = diff.first_non_stack_page_index;
        self.parent_wait_first_non_stack_dirty_addr = diff.first_non_stack_addr;
        self.parent_wait_first_non_stack_dirty_before_checksum = diff.first_non_stack_before;
        self.parent_wait_first_non_stack_dirty_after_checksum = diff.first_non_stack_after;
        true
    }

    pub fn restore_parent_wait_stack_snapshot(
        &mut self,
        address_space: &UserAddressSpace,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if self.builtin_grandchild_active() {
            if !self.builtin_grandchild.parent_stack_snapshot_copied
                || self.builtin_grandchild.parent_stack_snapshot_len == 0
                || self.builtin_grandchild.parent_stack_snapshot_restored
            {
                return false;
            }
            if !restore_user_stack_snapshot(
                address_space,
                page_metadata_map,
                &self.builtin_grandchild.parent_stack_snapshot,
                self.builtin_grandchild.parent_stack_snapshot_len,
            ) {
                return false;
            }
            self.builtin_grandchild.parent_stack_snapshot_restored = true;
            return true;
        }
        if !self.parent_wait_stack_snapshot_copied
            || self.parent_wait_stack_snapshot_len == 0
            || self.parent_wait_stack_snapshot_restored
        {
            return false;
        }
        if !restore_user_stack_snapshot(
            address_space,
            page_metadata_map,
            &self.parent_wait_stack_snapshot,
            self.parent_wait_stack_snapshot_len,
        ) {
            return false;
        }
        self.parent_wait_stack_snapshot_restored = true;
        true
    }

    pub fn restore_parent_wait_writable_page_snapshot(
        &mut self,
        address_space: &UserAddressSpace,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if self.builtin_grandchild_active() {
            if !self.builtin_grandchild.parent_writable_page_snapshot_saved
                || self.builtin_grandchild.parent_writable_page_count == 0
                || self
                    .builtin_grandchild
                    .parent_writable_page_snapshot_truncated
                || self
                    .builtin_grandchild
                    .parent_writable_page_snapshot_restored
            {
                return false;
            }
            if !restore_writable_page_snapshot(
                address_space,
                page_metadata_map,
                &self.builtin_grandchild.parent_writable_page_snapshot_pages,
                self.builtin_grandchild.parent_writable_page_count,
            ) {
                return false;
            }
            release_writable_page_snapshot_pages(
                &mut self.builtin_grandchild.parent_writable_page_snapshot_pages,
                self.builtin_grandchild.parent_writable_page_count,
                page_allocator,
                page_metadata_map,
            );
            self.builtin_grandchild
                .parent_writable_page_snapshot_restored = true;
            return true;
        }
        if !self.parent_wait_writable_page_snapshot_copied()
            || self.parent_wait_writable_page_snapshot_restored
        {
            return false;
        }
        if !restore_writable_page_snapshot(
            address_space,
            page_metadata_map,
            &self.parent_wait_writable_page_snapshot_pages,
            self.parent_wait_writable_page_count,
        ) {
            return false;
        }
        release_writable_page_snapshot_pages(
            &mut self.parent_wait_writable_page_snapshot_pages,
            self.parent_wait_writable_page_count,
            page_allocator,
            page_metadata_map,
        );
        self.parent_wait_writable_page_snapshot_restored = true;
        true
    }

    pub fn restore_parent_fd_snapshot(&mut self, files_struct: &mut FilesStruct) -> bool {
        if self.builtin_grandchild_active() {
            if !self.builtin_grandchild.parent_fd_snapshot_saved
                || self.builtin_grandchild.parent_fd_snapshot_restored
                || files_struct
                    .restore_parent_fd_snapshot(&self.builtin_grandchild.parent_fd_snapshot)
                    .is_err()
            {
                return false;
            }
            self.builtin_grandchild.parent_fd_snapshot_restored = true;
            return true;
        }
        if !self.parent_fd_snapshot_saved || self.parent_fd_snapshot_restored {
            return false;
        }
        if files_struct
            .restore_parent_fd_snapshot(&self.parent_fd_snapshot)
            .is_err()
        {
            return false;
        }
        self.parent_fd_snapshot_restored = true;
        true
    }

    pub fn restore_observed_child_parent_fs_snapshot(&mut self, fs_struct: &mut FsStruct) -> bool {
        if !self.builtin_grandchild_active() {
            return true;
        }
        if self.builtin_grandchild.parent_fs_snapshot_restored {
            return false;
        }
        let Some(snapshot) = self.builtin_grandchild.parent_fs_snapshot.as_ref() else {
            return false;
        };
        if fs_struct.restore_snapshot(snapshot).is_err() {
            return false;
        }
        self.builtin_grandchild.parent_fs_snapshot_restored = true;
        true
    }

    pub fn mark_parent_wait_resumed(&mut self, status_copied: bool) -> bool {
        if !self.child_exit_status_observed
            || !self.parent_wait_stack_snapshot_restored
            || !self.parent_wait_writable_page_snapshot_restored
            || !self.parent_fd_snapshot_restored
            || self.parent_wait_resumed
        {
            return false;
        }
        self.wait4_status_copied = status_copied;
        self.parent_wait_resumed = true;
        true
    }

    pub fn mark_observed_child_parent_wait_resumed(
        &mut self,
        status_copied: bool,
        wait_status: usize,
    ) -> bool {
        let continuation_restored = if self.builtin_grandchild.bound {
            self.builtin_grandchild.parent_exec_snapshot_restored
                && self.builtin_grandchild.parent_stack_snapshot_restored
                && self
                    .builtin_grandchild
                    .parent_writable_page_snapshot_restored
                && self.builtin_grandchild.parent_fd_snapshot_restored
                && self.builtin_grandchild.parent_fs_snapshot_restored
        } else {
            self.parent_wait_stack_snapshot_restored
                && self.parent_wait_writable_page_snapshot_restored
                && self.parent_fd_snapshot_restored
        };
        if !self.observed_plain_fork_clone
            || !self.observed_plain_fork_child_active
            || self.observed_plain_fork_parent_restored
            || !self.child_exit_status_observed
            || !continuation_restored
            || (self.builtin_grandchild.bound
                && self.builtin_grandchild.parent_resume != BuiltinGrandchildParentResume::Wait4)
        {
            return false;
        }

        let child_pid = self.observed_plain_fork_child_pid;
        self.wait4_status_copied = status_copied;
        self.observed_plain_fork_child_active = false;
        self.observed_plain_fork_parent_restored = true;
        self.last_reaped_child_pid = child_pid;
        self.last_reaped_child_wait_status = wait_status;
        self.last_released_child_pid = child_pid;
        self.last_released_child_wait_status = wait_status;
        self.pid = self.observed_plain_fork_parent_pid;
        self.parent_pid = self.observed_plain_fork_parent_parent_pid;
        self.tgid = self.observed_plain_fork_parent_tgid;
        true
    }

    pub fn mark_builtin_grandchild_exited_to_parent_read(&mut self) -> bool {
        if !self.builtin_grandchild.bound
            || self.builtin_grandchild.parent_resume != BuiltinGrandchildParentResume::PipeRead
            || !self.observed_plain_fork_clone
            || !self.observed_plain_fork_child_active
            || self.observed_plain_fork_parent_restored
            || !self.child_exit_status_observed
            || !self.builtin_grandchild.parent_exec_snapshot_restored
            || !self.builtin_grandchild.parent_stack_snapshot_restored
            || !self
                .builtin_grandchild
                .parent_writable_page_snapshot_restored
            || !self.builtin_grandchild.parent_fd_snapshot_restored
            || !self.builtin_grandchild.parent_fs_snapshot_restored
        {
            return false;
        }

        self.observed_plain_fork_child_active = false;
        self.observed_plain_fork_parent_restored = true;
        self.builtin_grandchild.exited_waitable = true;
        self.pid = self.observed_plain_fork_parent_pid;
        self.parent_pid = self.observed_plain_fork_parent_parent_pid;
        self.tgid = self.observed_plain_fork_parent_tgid;
        true
    }

    pub fn mark_builtin_grandchild_reaped(&mut self, wait_status: usize) -> bool {
        if !self.builtin_grandchild_exit_waitable() {
            return false;
        }
        let child_pid = self.observed_plain_fork_child_pid;
        if !self.release_destroyed_task_ref(self.last_exited_task_ref) {
            return false;
        }
        self.wait4_status_copied = true;
        self.last_reaped_child_pid = child_pid;
        self.last_reaped_child_wait_status = wait_status;
        self.last_released_child_pid = child_pid;
        self.last_released_child_wait_status = wait_status;
        self.builtin_grandchild.exited_waitable = false;
        true
    }

    pub fn finish_observed_child_parent_restore(&mut self) -> bool {
        if self.builtin_grandchild.bound {
            if self.active_task_state() != State::OnCpu
                || !self.observed_plain_fork_clone
                || self.observed_plain_fork_child_active
                || !self.observed_plain_fork_parent_restored
                || self.pid != self.observed_plain_fork_parent_pid
                || !self.current_child_continuation()
                || !self.enqueued
                || !self.builtin_grandchild.parent_stack_snapshot_restored
                || !self.builtin_grandchild.parent_exec_snapshot_restored
                || !self
                    .builtin_grandchild
                    .parent_writable_page_snapshot_restored
                || !self.builtin_grandchild.parent_fd_snapshot_restored
                || !self.builtin_grandchild.parent_fs_snapshot_restored
                || !self.parent_fd_snapshot_saved
                || self.parent_fd_snapshot_restored
                || self.parent_wait_frame.is_none()
                || !self.parent_address_space_snapshot_saved
                || !self.parent_wait_stack_snapshot_copied
                || self.parent_wait_stack_snapshot_restored
                || !self.parent_wait_writable_page_snapshot_saved
                || self.parent_wait_writable_page_snapshot_restored
                || self
                    .builtin_grandchild
                    .parent_exec_address_space_snapshot_saved
                || self.builtin_grandchild.parent_exec_stack_snapshot_saved
            {
                return false;
            }

            if self.last_exited_task_ref.is_valid()
                && !self.release_destroyed_task_ref(self.last_exited_task_ref)
            {
                return false;
            }

            self.builtin_grandchild.reset_empty();
            self.child_exit_status = 0;
            self.child_exit_status_observed = false;
            self.wait4_status_copied = false;
            self.parent_wait_writable_page_compared = false;
            self.parent_wait_writable_page_dirty_count = 0;
            self.parent_wait_writable_page_stack_dirty_count = 0;
            self.parent_wait_writable_page_non_stack_dirty_count = 0;
            self.parent_wait_first_non_stack_dirty_kind = 0;
            self.parent_wait_first_non_stack_dirty_mapping_index = 0;
            self.parent_wait_first_non_stack_dirty_page_index = 0;
            self.parent_wait_first_non_stack_dirty_addr = 0;
            self.parent_wait_first_non_stack_dirty_before_checksum = 0;
            self.parent_wait_first_non_stack_dirty_after_checksum = 0;
            self.clear_observed_plain_fork_state();
            return true;
        }
        if self.active_task_state() != State::OnCpu
            || !self.observed_plain_fork_clone
            || self.observed_plain_fork_child_active
            || !self.observed_plain_fork_parent_restored
            || self.pid != self.observed_plain_fork_parent_pid
            || !self.current_child_continuation()
            || !self.enqueued
            || !self.parent_wait_stack_snapshot_restored
            || !self.parent_wait_writable_page_snapshot_restored
            || !self.parent_fd_snapshot_restored
        {
            return false;
        }

        if !self.release_destroyed_task_ref(self.last_exited_task_ref) {
            return false;
        }

        self.parent_fd_snapshot = FilesStructSnapshot::empty();
        self.parent_fd_snapshot_saved = false;
        self.parent_fd_snapshot_restored = false;
        self.user_stack_snapshot_len = 0;
        self.user_stack_snapshot_copied = false;
        self.user_stack_snapshot_restored = false;
        self.trap_frame_copied = false;
        self.trap_frame_child_return_zero = false;
        self.tls_inherited = false;
        self.child_trap_frame = None;
        self.parent_wait_frame = None;
        self.parent_wait_status_ptr = 0;
        self.parent_address_space_snapshot = UserAddressSpace::new();
        self.parent_address_space_snapshot_saved = false;
        self.parent_exec_stack_snapshot = UserStack::new();
        self.parent_exec_stack_snapshot_saved = false;
        self.parent_wait_stack_snapshot_len = 0;
        self.parent_wait_stack_snapshot_copied = false;
        self.parent_wait_stack_snapshot_restored = false;
        self.parent_wait_stack_window_start = 0;
        self.parent_wait_stack_window_len = 0;
        self.parent_wait_stack_window_saved = false;
        self.parent_wait_stack_window_compared = false;
        self.parent_wait_stack_window_diff_count = 0;
        self.parent_wait_stack_window_first_diff_addr = 0;
        self.parent_wait_stack_window_before_byte = 0;
        self.parent_wait_stack_window_after_byte = 0;
        self.parent_wait_writable_page_snapshot_pages =
            [None; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX];
        self.parent_wait_writable_page_checksums = [0; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX];
        self.parent_wait_writable_page_count = 0;
        self.parent_wait_writable_page_snapshot_saved = false;
        self.parent_wait_writable_page_snapshot_truncated = false;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;
        self.child_exit_status = 0;
        self.child_exit_status_observed = false;
        self.wait4_status_copied = false;
        self.parent_clone_return = 0;
        self.wait4_parent_wait_observed = false;
        self.clear_observed_plain_fork_state();
        true
    }

    pub fn mark_vfork_parent_resumed(&mut self) -> bool {
        if !self.vfork_clone()
            || !self.child_exit_status_observed
            || !self.parent_wait_writable_page_snapshot_restored
            || !self.parent_fd_snapshot_restored
            || self.vfork_parent_resumed
        {
            return false;
        }

        self.vfork_parent_resumed = true;
        self.parent_clone_return = self.pid;
        true
    }

    pub fn archive_completed_child_record(&mut self) -> bool {
        if !self.vfork_parent_resumed
            || !self.child_exit_status_observed
            || self.pid == 0
            || self.completed_child_records_full()
        {
            return false;
        }

        let Some(exited_slot) = self.slot_for_ref(self.last_exited_task_ref) else {
            return false;
        };
        if exited_slot.state() != State::Destroyed || exited_slot.pid != self.pid {
            return false;
        }
        let record = UserCompletedChildRecord::archived(
            self.last_exited_task_ref,
            self.pid,
            self.child_exit_status,
            self.pidfd_fd,
        );
        let mut index = 0usize;
        while index < USER_COMPLETED_CHILD_RECORD_CAPACITY {
            if !self.completed_child_records[index].occupied {
                self.completed_child_records[index] = record;
                self.completed_child_record_count += 1;
                self.completed_child_record_archived = true;
                self.completed_child_record_total_archived += 1;
                self.last_archived_child_pid = record.pid;
                self.last_archived_child_exit_status = record.exit_status;
                self.last_archived_child_wait_status = record.wait_status;
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn mark_completed_child_reaped(&mut self, pid: usize) -> bool {
        if pid == 0 {
            return false;
        }

        let mut index = 0usize;
        while index < USER_COMPLETED_CHILD_RECORD_CAPACITY {
            let record = self.completed_child_records[index];
            if record.occupied && record.pid == pid && !record.reaped {
                if self.completed_child_record_count == 0 {
                    return false;
                }
                if !self.release_destroyed_task_ref(record.task_ref) {
                    return false;
                }
                self.completed_child_record_reaped = true;
                self.completed_child_record_released = true;
                self.completed_child_record_reaped_count += 1;
                self.completed_child_record_released_count += 1;
                self.last_reaped_child_pid = record.pid;
                self.last_reaped_child_wait_status = record.wait_status;
                self.last_released_child_pid = record.pid;
                self.last_released_child_wait_status = record.wait_status;
                self.completed_child_records[index] = UserCompletedChildRecord::empty();
                self.completed_child_record_count -= 1;
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn release_active_task_record(&mut self) -> bool {
        if !self.vfork_parent_resumed
            || !self.completed_child_record_archived
            || !self.parent_wait_writable_page_snapshot_restored
        {
            return false;
        }

        self.prepare_fresh_task_record()
    }

    pub fn release_reaped_active_task_record(&mut self) -> bool {
        if self.active_task_state() != State::Destroyed
            || self.vfork_clone()
            || self.observed_plain_fork_clone
            || !self.parent_wait_resumed
            || !self.child_exit_status_observed
            || !self.wait4_status_copied
            || !self.parent_wait_stack_snapshot_restored
            || !self.parent_wait_writable_page_snapshot_restored
            || !self.parent_fd_snapshot_restored
            || self.pid == 0
            || self.parent_pid != super::rest_init::KERNEL_INIT_PID
            || !self.enqueued
        {
            return false;
        }

        if !self.release_destroyed_task_ref(self.last_exited_task_ref) {
            return false;
        }
        self.prepare_fresh_task_record()
    }

    fn prepare_fresh_task_record(&mut self) -> bool {
        self.prepared = true;
        self.task_entry = TaskEntry::UserChild;
        self.task_entry_bound = true;
        self.pid = 0;
        self.parent_pid = 0;
        self.tgid = 0;
        self.exit_signal = 0;
        self.task_struct_allocated = false;
        self.pid_allocated = false;
        self.thread_context_ready = false;
        self.sched_entity_ready = false;
        self.task_state_new = false;
        self.files_struct_copied = false;
        self.parent_fd_snapshot = FilesStructSnapshot::empty();
        self.parent_fd_snapshot_saved = false;
        self.parent_fd_snapshot_restored = false;
        self.fs_struct_copied = false;
        self.credentials_copied = false;
        self.signal_state_copied = false;
        self.user_address_space_snapshot = false;
        self.user_stack_snapshot_len = 0;
        self.user_stack_snapshot_copied = false;
        self.user_stack_snapshot_restored = false;
        self.trap_frame_copied = false;
        self.trap_frame_child_return_zero = false;
        self.tls_inherited = false;
        self.child_trap_frame = None;
        self.parent_wait_frame = None;
        self.parent_wait_status_ptr = 0;
        self.parent_address_space_snapshot = UserAddressSpace::new();
        self.parent_address_space_snapshot_saved = false;
        self.parent_exec_stack_snapshot = UserStack::new();
        self.parent_exec_stack_snapshot_saved = false;
        self.parent_wait_stack_snapshot_len = 0;
        self.parent_wait_stack_snapshot_copied = false;
        self.parent_wait_stack_snapshot_restored = false;
        self.parent_wait_stack_window_start = 0;
        self.parent_wait_stack_window_len = 0;
        self.parent_wait_stack_window_saved = false;
        self.parent_wait_stack_window_compared = false;
        self.parent_wait_stack_window_diff_count = 0;
        self.parent_wait_stack_window_first_diff_addr = 0;
        self.parent_wait_stack_window_before_byte = 0;
        self.parent_wait_stack_window_after_byte = 0;
        self.parent_wait_writable_page_snapshot_pages =
            [None; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX];
        self.parent_wait_writable_page_checksums = [0; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX];
        self.parent_wait_writable_page_count = 0;
        self.parent_wait_writable_page_snapshot_saved = false;
        self.parent_wait_writable_page_snapshot_truncated = false;
        self.parent_wait_writable_page_snapshot_restored = false;
        self.parent_wait_writable_page_compared = false;
        self.parent_wait_writable_page_dirty_count = 0;
        self.parent_wait_writable_page_stack_dirty_count = 0;
        self.parent_wait_writable_page_non_stack_dirty_count = 0;
        self.parent_wait_first_non_stack_dirty_kind = 0;
        self.parent_wait_first_non_stack_dirty_mapping_index = 0;
        self.parent_wait_first_non_stack_dirty_page_index = 0;
        self.parent_wait_first_non_stack_dirty_addr = 0;
        self.parent_wait_first_non_stack_dirty_before_checksum = 0;
        self.parent_wait_first_non_stack_dirty_after_checksum = 0;
        self.child_exit_status = 0;
        self.child_exit_status_observed = false;
        self.wait4_status_copied = false;
        self.parent_wait_resumed = false;
        self.vfork_vm_clone = false;
        self.vfork_pidfd_clone = false;
        self.vfork_parent_frame = None;
        self.vfork_parent_frame_saved = false;
        self.vfork_child_handoff = false;
        self.vfork_parent_resumed = false;
        self.vfork_child_sp = 0;
        self.nested_vfork_clone = false;
        self.nested_vfork_parent_pid = 0;
        self.vfork_parent_resume_on_exit = true;
        self.pidfd_fd = usize::MAX;
        self.pidfd_copyout = false;
        self.parent_clone_return = 0;
        self.enqueued = false;
        self.wait4_parent_wait_observed = false;
        self.child_continuation_taken = false;
        self.vfork_next_child_accepted = false;
        self.clear_observed_plain_fork_state();
        self.builtin_grandchild.reset_empty();
        self.active_task_record_available = true;
        true
    }

    fn clear_observed_plain_fork_state(&mut self) {
        self.observed_plain_fork_clone = false;
        self.observed_plain_fork_parent_pid = 0;
        self.observed_plain_fork_parent_parent_pid = 0;
        self.observed_plain_fork_parent_tgid = 0;
        self.observed_plain_fork_child_pid = 0;
        self.observed_plain_fork_child_active = false;
        self.observed_plain_fork_parent_restored = false;
    }

    fn capture_parent_wait_stack_window(
        &mut self,
        address_space: &UserAddressSpace,
        page_metadata_map: &PageMetadataMap,
        start: usize,
    ) -> bool {
        self.parent_wait_stack_window_start = 0;
        self.parent_wait_stack_window_len = 0;
        self.parent_wait_stack_window_saved = false;
        self.parent_wait_stack_window_compared = false;
        self.parent_wait_stack_window_diff_count = 0;
        self.parent_wait_stack_window_first_diff_addr = 0;
        self.parent_wait_stack_window_before_byte = 0;
        self.parent_wait_stack_window_after_byte = 0;

        let Some(len) = copy_user_stack_window_to_buffer(
            address_space,
            page_metadata_map,
            start,
            &mut self.parent_wait_stack_window,
        ) else {
            return false;
        };
        if len == 0 {
            return false;
        }
        self.parent_wait_stack_window_start = start;
        self.parent_wait_stack_window_len = len;
        self.parent_wait_stack_window_saved = true;
        true
    }
}

struct WritablePageChecksumDiff {
    dirty_count: usize,
    stack_dirty_count: usize,
    non_stack_dirty_count: usize,
    first_non_stack_kind: usize,
    first_non_stack_mapping_index: usize,
    first_non_stack_page_index: usize,
    first_non_stack_addr: usize,
    first_non_stack_before: usize,
    first_non_stack_after: usize,
}

fn capture_writable_page_snapshot(
    address_space: &UserAddressSpace,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
    snapshot_pages: &mut [Option<PageRef>; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    output: &mut [usize; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
) -> Option<(usize, bool)> {
    let mut reset_index = 0usize;
    while reset_index < snapshot_pages.len() {
        snapshot_pages[reset_index] = None;
        output[reset_index] = 0;
        reset_index += 1;
    }

    let mut count = 0usize;
    let mut mapping_index = 0usize;
    while mapping_index < address_space.mapping_count {
        let mapping = &address_space.mappings[mapping_index];
        if mapping.kind() != UserMappingKind::Empty
            && mapping.user_accessible()
            && mapping.writable()
        {
            let mut page_index = 0usize;
            while page_index < mapping.backing_page_count() {
                if count == output.len() {
                    return Some((count, true));
                }
                let Some(checksum) = checksum_mapping_page(mapping, page_metadata_map, page_index)
                else {
                    release_writable_page_snapshot_pages(
                        snapshot_pages,
                        count,
                        page_allocator,
                        page_metadata_map,
                    );
                    return None;
                };
                output[count] = checksum;
                let Some(source_page) = mapping.backing_page(page_index) else {
                    release_writable_page_snapshot_pages(
                        snapshot_pages,
                        count,
                        page_allocator,
                        page_metadata_map,
                    );
                    return None;
                };
                let Some(source_linear) = page_metadata_map.page_address(source_page) else {
                    release_writable_page_snapshot_pages(
                        snapshot_pages,
                        count,
                        page_allocator,
                        page_metadata_map,
                    );
                    return None;
                };
                let Some(snapshot_page) =
                    page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map)
                else {
                    release_writable_page_snapshot_pages(
                        snapshot_pages,
                        count,
                        page_allocator,
                        page_metadata_map,
                    );
                    return None;
                };
                let Some(snapshot_linear) = page_metadata_map.page_address(snapshot_page) else {
                    let _ = page_allocator.free_pages(snapshot_page, 0, page_metadata_map);
                    release_writable_page_snapshot_pages(
                        snapshot_pages,
                        count,
                        page_allocator,
                        page_metadata_map,
                    );
                    return None;
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        source_linear as *const u8,
                        snapshot_linear as *mut u8,
                        USER_PAGE_SIZE,
                    );
                }
                snapshot_pages[count] = Some(snapshot_page);
                count += 1;
                page_index += 1;
            }
        }
        mapping_index += 1;
    }
    Some((count, false))
}

fn restore_writable_page_snapshot(
    address_space: &UserAddressSpace,
    page_metadata_map: &PageMetadataMap,
    snapshot_pages: &[Option<PageRef>; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    expected_count: usize,
) -> bool {
    if expected_count == 0 || expected_count > snapshot_pages.len() {
        return false;
    }

    let mut restored = 0usize;
    let mut mapping_index = 0usize;
    while mapping_index < address_space.mapping_count && restored < expected_count {
        let mapping = &address_space.mappings[mapping_index];
        if mapping.kind() != UserMappingKind::Empty
            && mapping.user_accessible()
            && mapping.writable()
        {
            let mut page_index = 0usize;
            while page_index < mapping.backing_page_count() && restored < expected_count {
                let Some(snapshot_page) = snapshot_pages[restored] else {
                    return false;
                };
                let Some(snapshot_linear) = page_metadata_map.page_address(snapshot_page) else {
                    return false;
                };
                let Some(target_page) = mapping.backing_page(page_index) else {
                    return false;
                };
                let Some(target_linear) = page_metadata_map.page_address(target_page) else {
                    return false;
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        snapshot_linear as *const u8,
                        target_linear as *mut u8,
                        USER_PAGE_SIZE,
                    );
                }
                restored += 1;
                page_index += 1;
            }
        }
        mapping_index += 1;
    }

    restored == expected_count
}

fn release_writable_page_snapshot_pages(
    snapshot_pages: &mut [Option<PageRef>; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    count: usize,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) {
    let mut index = 0usize;
    while index < count && index < snapshot_pages.len() {
        if let Some(page) = snapshot_pages[index].take() {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        }
        index += 1;
    }
}

fn compare_writable_page_checksums(
    address_space: &UserAddressSpace,
    page_metadata_map: &PageMetadataMap,
    expected: &[usize; USER_PARENT_WAIT_DIRTY_PAGE_PROBE_MAX],
    expected_count: usize,
) -> Option<WritablePageChecksumDiff> {
    if expected_count == 0 || expected_count > expected.len() {
        return None;
    }

    let mut checked = 0usize;
    let mut dirty_count = 0usize;
    let mut stack_dirty_count = 0usize;
    let mut non_stack_dirty_count = 0usize;
    let mut first_non_stack_kind = 0usize;
    let mut first_non_stack_mapping_index = 0usize;
    let mut first_non_stack_page_index = 0usize;
    let mut first_non_stack_addr = 0usize;
    let mut first_non_stack_before = 0usize;
    let mut first_non_stack_after = 0usize;

    let mut mapping_index = 0usize;
    while mapping_index < address_space.mapping_count && checked < expected_count {
        let mapping = &address_space.mappings[mapping_index];
        if mapping.kind() != UserMappingKind::Empty
            && mapping.user_accessible()
            && mapping.writable()
        {
            let mut page_index = 0usize;
            while page_index < mapping.backing_page_count() && checked < expected_count {
                let before = expected[checked];
                let after = checksum_mapping_page(mapping, page_metadata_map, page_index)?;
                if before != after {
                    dirty_count += 1;
                    if mapping.kind() == UserMappingKind::Stack {
                        stack_dirty_count += 1;
                    } else {
                        non_stack_dirty_count += 1;
                        if first_non_stack_addr == 0 {
                            first_non_stack_kind = user_mapping_kind_index(mapping.kind());
                            first_non_stack_mapping_index = mapping_index;
                            first_non_stack_page_index = page_index;
                            first_non_stack_addr = mapping_page_user_start(mapping, page_index)?;
                            first_non_stack_before = before;
                            first_non_stack_after = after;
                        }
                    }
                }
                checked += 1;
                page_index += 1;
            }
        }
        mapping_index += 1;
    }

    if checked != expected_count {
        return None;
    }
    Some(WritablePageChecksumDiff {
        dirty_count,
        stack_dirty_count,
        non_stack_dirty_count,
        first_non_stack_kind,
        first_non_stack_mapping_index,
        first_non_stack_page_index,
        first_non_stack_addr,
        first_non_stack_before,
        first_non_stack_after,
    })
}

fn checksum_mapping_page(
    mapping: &UserMapping,
    page_metadata_map: &PageMetadataMap,
    page_index: usize,
) -> Option<usize> {
    let page = mapping.backing_page(page_index)?;
    let linear = page_metadata_map.page_address(page)?;
    if matches!(
        mapping.kind(),
        UserMappingKind::Heap | UserMappingKind::AnonymousPrivate
    ) {
        let mut hash = 0xcbf29ce484222325usize;
        let mut offset = 0usize;
        while offset < USER_PAGE_SIZE {
            let byte = unsafe { *((linear + offset) as *const u8) };
            hash ^= byte as usize;
            hash = hash.wrapping_mul(0x100000001b3usize);
            offset += 1;
        }
        return Some(hash);
    }
    let page_base_offset = page_index.checked_mul(USER_PAGE_SIZE)?;
    let mapping_end_offset = mapping.page_offset().checked_add(mapping.memsz())?;
    if page_base_offset >= mapping_end_offset {
        return None;
    }
    let page_end_offset = page_base_offset.checked_add(USER_PAGE_SIZE)?;
    let start = if page_base_offset < mapping.page_offset() {
        mapping.page_offset() - page_base_offset
    } else {
        0
    };
    let end = if mapping_end_offset < page_end_offset {
        mapping_end_offset - page_base_offset
    } else {
        USER_PAGE_SIZE
    };
    if start >= end {
        return None;
    }

    let mut hash = 0xcbf29ce484222325usize;
    let mut offset = start;
    while offset < end {
        let byte = unsafe { *((linear + offset) as *const u8) };
        hash ^= byte as usize;
        hash = hash.wrapping_mul(0x100000001b3usize);
        offset += 1;
    }
    Some(hash)
}

fn mapping_page_user_start(mapping: &UserMapping, page_index: usize) -> Option<usize> {
    mapping.backing_page_vaddr(page_index)
}

fn user_mapping_kind_index(kind: UserMappingKind) -> usize {
    match kind {
        UserMappingKind::Empty => 0,
        UserMappingKind::ElfSegment => 1,
        UserMappingKind::Stack => 2,
        UserMappingKind::Heap => 3,
        UserMappingKind::AnonymousPrivate => 4,
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
    let stack = &crate::context::context_ref().user_stack;
    let snapshot_len = min_usize(stack.size(), output.len());
    if snapshot_len == 0 || snapshot_len > output.len() {
        return None;
    }
    let start = stack.top().checked_sub(snapshot_len)?;
    let mut index = 0usize;
    while index < snapshot_len {
        output[index] = stack
            .read_byte(page_metadata_map, start + index)
            .unwrap_or(0);
        index += 1;
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
    let stack = &crate::context::context_ref().user_stack;
    if len > stack.size() {
        return false;
    }
    let Some(start) = stack.top().checked_sub(len) else {
        return false;
    };
    let mut index = 0usize;
    while index < len {
        if !stack.write_existing_byte(page_metadata_map, start + index, input[index]) {
            return false;
        }
        index += 1;
    }
    true
}

fn copy_user_stack_window_to_buffer(
    address_space: &UserAddressSpace,
    page_metadata_map: &PageMetadataMap,
    start: usize,
    output: &mut [u8; USER_PARENT_WAIT_STACK_PROBE_LEN],
) -> Option<usize> {
    let mapping = address_space.stack_mapping()?;
    let stack = &crate::context::context_ref().user_stack;
    if mapping.kind() != UserMappingKind::Stack || start < stack.base() {
        return None;
    }
    let remaining = stack.top().checked_sub(start)?;
    let len = min_usize(output.len(), remaining);
    if len == 0 {
        return None;
    }
    let mut index = 0usize;
    while index < len {
        output[index] = stack.read_byte(page_metadata_map, start + index)?;
        index += 1;
    }
    Some(len)
}

#[allow(dead_code)]
impl KernelInitTaskUserState {
    pub const fn new() -> Self {
        Self {
            resources_bound: false,
            active_user_flow_online: false,
            preserves_kernel_init_task: false,
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
            supplementary_group_count: 0,
            supplementary_groups: [0; USER_SUPPLEMENTARY_GROUP_MAX],
            supplementary_groups_read_observed: false,
            setgroups_observed: false,
            pid_read_observed: false,
            ppid_zero_first_slice: false,
            ppid_read_observed: false,
            session_leader_first_slice: false,
            session_id: 0,
            session_id_read_observed: false,
            process_group_leader_first_slice: false,
            process_group: 0,
            process_group_read_observed: false,
            process_group_set_observed: false,
            setsid_eperm_observed: false,
            child_process_pid: 0,
            child_process_group_visible: false,
            child_process_group: 0,
            child_process_session_id: 0,
            pending_plain_fork_child_visible: false,
            pending_plain_fork_child_parent_pid: 0,
            pending_plain_fork_child_pid: 0,
            pending_plain_fork_child_process_group: 0,
            pending_plain_fork_child_session_id: 0,
            pending_plain_fork_child_process_group_set_observed: false,
            observed_child_parent_identity_saved: false,
            observed_child_parent_process_group: 0,
            observed_child_parent_session_id: 0,
            observed_child_parent_session_leader_first_slice: false,
            observed_child_parent_controlling_tty_bound: false,
            observed_child_parent_controlling_tty_cleared_on_setsid: false,
            child_session_leader_first_slice: false,
            child_process_group_set_observed: false,
            child_setsid_success_observed: false,
            child_controlling_tty_bound: false,
            child_controlling_tty_cleared_on_setsid: false,
            controlling_tty_bound: false,
            foreground_pgrp_bound: false,
            foreground_pgrp: 0,
            foreground_pgrp_read_observed: false,
            foreground_pgrp_set_observed: false,
            tty_session_id_read_observed: false,
            tiocsctty_observed: false,
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
            pending_signal_set_empty_first_slice: false,
            blocked_signal_mask: 0,
            signal_actions: [UserSignalAction::default(); USER_SIGNAL_COUNT],
            rt_sigprocmask_observed: false,
            rt_sigaction_observed: false,
            rt_sigtimedwait_observed: false,
            rt_sigtimedwait_mask: 0,
            rt_sigtimedwait_uinfo_null: false,
            rt_sigtimedwait_uts_null: false,
            rt_sigtimedwait_pending_match: false,
            rt_sigtimedwait_infinite_wait: false,
            pending_sigchld: false,
            rt_sigtimedwait_sleeping: false,
            rt_sigtimedwait_wait_queue: UserSignalWaitQueue::new(),
            rt_sigtimedwait_saved_frame: None,
            rt_sigtimedwait_sleep_reason: USER_SIGNAL_WAIT_REASON_NONE,
            rt_sigtimedwait_wake_signal: 0,
            rt_sigtimedwait_dequeued_signal: 0,
            rt_sigtimedwait_return_signal: 0,
            clear_child_tid_bound: false,
            clear_child_tid: 0,
            root_cwd_first_slice: false,
            getcwd_observed: false,
        }
    }

    pub const fn resources_bound(&self) -> bool {
        self.resources_bound
    }

    pub const fn active_user_flow_online(&self) -> bool {
        self.active_user_flow_online
    }

    pub const fn preserves_kernel_init_task(&self) -> bool {
        self.preserves_kernel_init_task
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

    pub const fn supplementary_group_count(&self) -> usize {
        self.supplementary_group_count
    }

    pub const fn supplementary_group(&self, index: usize) -> Option<usize> {
        if index < self.supplementary_group_count && index < USER_SUPPLEMENTARY_GROUP_MAX {
            Some(self.supplementary_groups[index])
        } else {
            None
        }
    }

    pub const fn supplementary_groups_read_observed(&self) -> bool {
        self.supplementary_groups_read_observed
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

    pub const fn session_id(&self) -> usize {
        self.session_id
    }

    pub const fn session_id_read_observed(&self) -> bool {
        self.session_id_read_observed
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

    pub const fn setsid_eperm_observed(&self) -> bool {
        self.setsid_eperm_observed
    }

    pub const fn child_process_group_visible(&self) -> bool {
        self.child_process_group_visible
    }

    pub const fn child_process_pid(&self) -> usize {
        self.child_process_pid
    }

    pub const fn child_process_group(&self) -> usize {
        self.child_process_group
    }

    pub const fn child_process_session_id(&self) -> usize {
        self.child_process_session_id
    }

    pub const fn pending_plain_fork_child_visible(&self) -> bool {
        self.pending_plain_fork_child_visible
    }

    pub const fn pending_plain_fork_child_parent_pid(&self) -> usize {
        self.pending_plain_fork_child_parent_pid
    }

    pub const fn pending_plain_fork_child_pid(&self) -> usize {
        self.pending_plain_fork_child_pid
    }

    pub const fn pending_plain_fork_child_process_group(&self) -> usize {
        self.pending_plain_fork_child_process_group
    }

    pub const fn pending_plain_fork_child_session_id(&self) -> usize {
        self.pending_plain_fork_child_session_id
    }

    pub const fn pending_plain_fork_child_process_group_set_observed(&self) -> bool {
        self.pending_plain_fork_child_process_group_set_observed
    }

    pub const fn child_session_leader_first_slice(&self) -> bool {
        self.child_session_leader_first_slice
    }

    pub const fn child_process_group_set_observed(&self) -> bool {
        self.child_process_group_set_observed
    }

    pub const fn child_setsid_success_observed(&self) -> bool {
        self.child_setsid_success_observed
    }

    pub const fn child_controlling_tty_bound(&self) -> bool {
        self.child_controlling_tty_bound
    }

    pub const fn child_controlling_tty_cleared_on_setsid(&self) -> bool {
        self.child_controlling_tty_cleared_on_setsid
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

    pub const fn tty_session_id_read_observed(&self) -> bool {
        self.tty_session_id_read_observed
    }

    pub const fn tiocsctty_observed(&self) -> bool {
        self.tiocsctty_observed
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

    pub const fn setgroups_observed(&self) -> bool {
        self.setgroups_observed
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

    pub const fn pending_signal_set_empty_first_slice(&self) -> bool {
        self.pending_signal_set_empty_first_slice
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

    pub const fn rt_sigtimedwait_observed(&self) -> bool {
        self.rt_sigtimedwait_observed
    }

    pub const fn rt_sigtimedwait_mask(&self) -> usize {
        self.rt_sigtimedwait_mask
    }

    pub const fn rt_sigtimedwait_uinfo_null(&self) -> bool {
        self.rt_sigtimedwait_uinfo_null
    }

    pub const fn rt_sigtimedwait_uts_null(&self) -> bool {
        self.rt_sigtimedwait_uts_null
    }

    pub const fn rt_sigtimedwait_pending_match(&self) -> bool {
        self.rt_sigtimedwait_pending_match
    }

    pub const fn rt_sigtimedwait_infinite_wait(&self) -> bool {
        self.rt_sigtimedwait_infinite_wait
    }

    pub const fn pending_sigchld(&self) -> bool {
        self.pending_sigchld
    }

    pub const fn rt_sigtimedwait_sleeping(&self) -> bool {
        self.rt_sigtimedwait_sleeping
    }

    pub const fn rt_sigtimedwait_waiter_enqueued(&self) -> bool {
        self.rt_sigtimedwait_wait_queue.waiter_enqueued()
    }

    pub const fn rt_sigtimedwait_waiter_finished(&self) -> bool {
        self.rt_sigtimedwait_wait_queue.waiter_finished()
    }

    pub const fn rt_sigtimedwait_sleep_reason(&self) -> usize {
        self.rt_sigtimedwait_sleep_reason
    }

    pub const fn rt_sigtimedwait_wake_signal(&self) -> usize {
        self.rt_sigtimedwait_wake_signal
    }

    pub const fn rt_sigtimedwait_dequeued_signal(&self) -> usize {
        self.rt_sigtimedwait_dequeued_signal
    }

    pub const fn rt_sigtimedwait_return_signal(&self) -> usize {
        self.rt_sigtimedwait_return_signal
    }

    pub const fn rt_sigtimedwait_wake_sigchld_committed(&self) -> bool {
        self.rt_sigtimedwait_wait_queue.wake_sigchld_committed()
    }

    pub const fn rt_sigtimedwait_saved_frame_bound(&self) -> bool {
        self.rt_sigtimedwait_saved_frame.is_some()
    }

    pub const fn rt_sigtimedwait_sigchld_mask_match(&self) -> bool {
        sigchld_mask_matches(self.rt_sigtimedwait_mask)
    }

    pub fn credentials_syscall_ready(&self) -> bool {
        self.active_user_flow_online && self.credentials_inherited && self.root_credentials_bound
    }

    pub fn signal_mask_syscall_ready(&self) -> bool {
        self.active_user_flow_online
            && self.signal_state_inherited
            && self.signal_runtime_bound
            && self.thread_signal_state_bound
            && self.blocked_signal_mask_bound
    }

    pub fn signal_action_syscall_ready(&self) -> bool {
        self.active_user_flow_online
            && self.signal_state_inherited
            && self.signal_runtime_bound
            && self.signal_action_table_bound
            && self.signal_action_table_layout_bound
    }

    pub fn signal_wait_syscall_ready(&self) -> bool {
        self.active_user_flow_online
            && self.signal_state_inherited
            && self.signal_runtime_bound
            && self.thread_signal_state_bound
            && self.blocked_signal_mask_bound
            && self.pending_signal_set_empty_first_slice
            && self.signal_delivery_deferred
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

    // User-init setup binds the selected image, trap, task and file-system handoff facts.
    #[allow(clippy::too_many_arguments)]
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
        if self.resources_bound
            || kernel_init_task.state() != State::OnCpu
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
                State::Ready,
                State::Base,
                State::Ready,
            );
        }

        self.preserves_kernel_init_task = true;
        self.pid1_preserved = true;
        self.exec_identity_handoff = true;
        self.no_new_task_struct = true;
        self.kernel_init_not_destroyed = kernel_init_task.task().online();
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
        self.supplementary_group_count = 0;
        self.supplementary_groups = [0; USER_SUPPLEMENTARY_GROUP_MAX];
        self.supplementary_groups_read_observed = false;
        self.setgroups_observed = false;
        self.session_leader_first_slice = true;
        self.session_id = super::rest_init::KERNEL_INIT_PID;
        self.session_id_read_observed = false;
        self.process_group_leader_first_slice = true;
        self.process_group = super::rest_init::KERNEL_INIT_PID;
        self.child_process_pid = 0;
        self.child_process_group_visible = false;
        self.child_process_group = 0;
        self.child_process_session_id = 0;
        self.clear_pending_plain_fork_child_identity();
        self.observed_child_parent_identity_saved = false;
        self.observed_child_parent_process_group = 0;
        self.observed_child_parent_session_id = 0;
        self.observed_child_parent_session_leader_first_slice = false;
        self.observed_child_parent_controlling_tty_bound = false;
        self.observed_child_parent_controlling_tty_cleared_on_setsid = false;
        self.child_session_leader_first_slice = false;
        self.child_process_group_set_observed = false;
        self.child_setsid_success_observed = false;
        self.child_controlling_tty_bound = false;
        self.child_controlling_tty_cleared_on_setsid = false;
        self.controlling_tty_bound = true;
        self.foreground_pgrp_bound = true;
        self.foreground_pgrp = super::rest_init::KERNEL_INIT_PID;
        self.tty_session_id_read_observed = false;
        self.tiocsctty_observed = false;
        self.signal_state_inherited = true;
        self.signal_runtime_bound = true;
        self.thread_signal_state_bound = true;
        self.process_signal_state_deferred = true;
        self.signal_action_table_bound = true;
        self.signal_action_table_layout_bound = true;
        self.blocked_signal_mask_bound = true;
        self.signal_delivery_deferred = true;
        self.pending_signal_set_empty_first_slice = true;
        self.blocked_signal_mask = 0;
        self.signal_actions = [UserSignalAction::default(); USER_SIGNAL_COUNT];
        self.pending_sigchld = false;
        self.rt_sigtimedwait_sleeping = false;
        self.rt_sigtimedwait_wait_queue = UserSignalWaitQueue::new();
        self.rt_sigtimedwait_saved_frame = None;
        self.rt_sigtimedwait_sleep_reason = USER_SIGNAL_WAIT_REASON_NONE;
        self.rt_sigtimedwait_wake_signal = 0;
        self.rt_sigtimedwait_dequeued_signal = 0;
        self.rt_sigtimedwait_return_signal = 0;
        self.resources_bound = true;
        Ok(())
    }

    pub fn bind_syscall_context(
        &mut self,
        trap_frame: &UserTrapFrame,
        exception_type: &ExceptionType,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if !self.resources_bound
            || trap_frame.state() != State::Ready
            || exception_type.syscall_state() != State::Online
            || syscall_table.state() != State::Ready
            || !syscall_table.bound_to_exception()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                State::Base,
                State::Ready,
                State::Online,
            );
        }

        self.syscall_context_bound = true;
        self.trap_return_bound = true;
        self.syscall_dispatch_bound = true;
        self.syscall_arguments_extracted = true;
        Ok(())
    }

    pub fn activate_user_flow(&mut self, flow: &UserAppRuntime) -> EventResult {
        if !self.resources_bound
            || self.active_user_flow_online
            || flow.state() != State::Online
            || !flow.active_binding_committed()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Ready,
                State::Online,
            );
        }

        self.active_user_flow_online = true;
        Ok(())
    }

    pub fn enter_user_mode(
        &mut self,
        trap_frame: &UserTrapFrame,
        flow: &UserAppRuntime,
    ) -> EventResult {
        if !self.active_user_flow_online
            || flow.state() != State::Online
            || trap_frame.state() != State::Ready
            || !self.trap_return_bound
        {
            return failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
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
        if !self.active_user_flow_online {
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

    pub fn read_pid(&mut self, current_child_continuation: bool) -> Option<usize> {
        if !self.active_user_flow_online || !self.pid1_preserved {
            return None;
        }
        let pid = if current_child_continuation {
            if self.child_process_pid == 0 || !self.child_process_group_visible {
                return None;
            }
            self.child_process_pid
        } else {
            super::rest_init::KERNEL_INIT_PID
        };
        self.pid_read_observed = true;
        Some(pid)
    }

    pub fn read_ppid(&mut self) -> Option<usize> {
        if !self.active_user_flow_online || !self.pid1_preserved {
            return None;
        }
        self.ppid_zero_first_slice = true;
        self.ppid_read_observed = true;
        Some(0)
    }

    pub fn read_process_group(
        &mut self,
        pid_arg: usize,
        current_child_continuation: bool,
    ) -> UserProcessGroupLookup {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || !self.process_group_leader_first_slice
            || self.process_group == 0
        {
            return UserProcessGroupLookup::NotReady;
        }

        let pid = pid_t_arg(pid_arg);
        if pid < 0 {
            return UserProcessGroupLookup::NoSuchProcess;
        }
        let normalized_pid = if pid == 0 {
            if current_child_continuation {
                if self.child_process_pid == 0 || !self.child_process_group_visible {
                    return UserProcessGroupLookup::NotReady;
                }
                self.child_process_pid
            } else {
                super::rest_init::KERNEL_INIT_PID
            }
        } else {
            pid as usize
        };

        if normalized_pid == self.child_process_pid && self.child_process_group_visible {
            self.process_group_read_observed = true;
            return UserProcessGroupLookup::Found(self.child_process_group);
        }

        if normalized_pid != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupLookup::NoSuchProcess;
        }

        self.process_group_read_observed = true;
        UserProcessGroupLookup::Found(self.process_group)
    }

    pub fn read_session_id(
        &mut self,
        pid_arg: usize,
        current_child_continuation: bool,
    ) -> UserProcessGroupLookup {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || self.session_id == 0
        {
            return UserProcessGroupLookup::NotReady;
        }

        let pid = pid_t_arg(pid_arg);
        if pid < 0 {
            return UserProcessGroupLookup::NoSuchProcess;
        }
        let normalized_pid = if pid == 0 {
            if current_child_continuation {
                if self.child_process_pid == 0
                    || !self.child_process_group_visible
                    || self.child_process_session_id == 0
                {
                    return UserProcessGroupLookup::NotReady;
                }
                self.child_process_pid
            } else {
                super::rest_init::KERNEL_INIT_PID
            }
        } else {
            pid as usize
        };

        if normalized_pid == self.child_process_pid && self.child_process_group_visible {
            if self.child_process_session_id == 0 {
                return UserProcessGroupLookup::NotReady;
            }
            self.session_id_read_observed = true;
            return UserProcessGroupLookup::Found(self.child_process_session_id);
        }

        if normalized_pid != super::rest_init::KERNEL_INIT_PID {
            return UserProcessGroupLookup::NoSuchProcess;
        }

        self.session_id_read_observed = true;
        UserProcessGroupLookup::Found(self.session_id)
    }

    pub const fn can_observe_child_process_group_visible(&self, child_pid: usize) -> bool {
        self.active_user_flow_online
            && self.pid1_preserved
            && child_pid != 0
            && self.process_group != 0
            && self.session_id != 0
    }

    pub fn observe_child_process_group_visible(&mut self, child_pid: usize) -> bool {
        if !self.can_observe_child_process_group_visible(child_pid) {
            return false;
        }

        self.child_process_pid = child_pid;
        self.child_process_group_visible = true;
        self.child_process_group = self.process_group;
        self.child_process_session_id = self.session_id;
        self.clear_pending_plain_fork_child_identity();
        self.observed_child_parent_identity_saved = false;
        self.observed_child_parent_process_group = 0;
        self.observed_child_parent_session_id = 0;
        self.observed_child_parent_session_leader_first_slice = false;
        self.observed_child_parent_controlling_tty_bound = false;
        self.observed_child_parent_controlling_tty_cleared_on_setsid = false;
        self.child_session_leader_first_slice = false;
        self.child_setsid_success_observed = false;
        self.child_controlling_tty_bound = self.controlling_tty_bound;
        self.child_controlling_tty_cleared_on_setsid = false;
        self.tiocsctty_observed = false;
        true
    }

    pub const fn can_observe_pending_plain_fork_child_process_group_visible(
        &self,
        parent_pid: usize,
        child_pid: usize,
    ) -> bool {
        self.active_user_flow_online
            && self.pid1_preserved
            && parent_pid != 0
            && child_pid != 0
            && parent_pid != child_pid
            && self.child_process_pid == parent_pid
            && self.child_process_group_visible
            && self.child_process_group != 0
            && self.child_process_session_id != 0
            && !self.pending_plain_fork_child_visible
    }

    pub fn observe_pending_plain_fork_child_process_group_visible(
        &mut self,
        parent_pid: usize,
        child_pid: usize,
    ) -> bool {
        if !self.can_observe_pending_plain_fork_child_process_group_visible(parent_pid, child_pid) {
            return false;
        }

        self.pending_plain_fork_child_visible = true;
        self.pending_plain_fork_child_parent_pid = parent_pid;
        self.pending_plain_fork_child_pid = child_pid;
        self.pending_plain_fork_child_process_group = self.child_process_group;
        self.pending_plain_fork_child_session_id = self.child_process_session_id;
        self.pending_plain_fork_child_process_group_set_observed = false;
        true
    }

    pub const fn can_observe_nested_child_process_group_visible(
        &self,
        parent_pid: usize,
        child_pid: usize,
    ) -> bool {
        self.active_user_flow_online
            && self.pid1_preserved
            && parent_pid != 0
            && child_pid != 0
            && self.child_process_pid == parent_pid
            && self.child_process_group_visible
            && self.child_process_group != 0
            && self.child_process_session_id != 0
    }

    pub fn observe_nested_child_process_group_visible(
        &mut self,
        parent_pid: usize,
        child_pid: usize,
    ) -> bool {
        if !self.can_observe_nested_child_process_group_visible(parent_pid, child_pid) {
            return false;
        }

        self.child_process_pid = child_pid;
        self.observed_child_parent_identity_saved = false;
        self.observed_child_parent_process_group = 0;
        self.observed_child_parent_session_id = 0;
        self.observed_child_parent_session_leader_first_slice = false;
        self.observed_child_parent_controlling_tty_bound = false;
        self.observed_child_parent_controlling_tty_cleared_on_setsid = false;
        self.child_session_leader_first_slice = false;
        self.child_setsid_success_observed = false;
        true
    }

    pub fn switch_observed_child_process_visible(
        &mut self,
        parent_pid: usize,
        child_pid: usize,
    ) -> bool {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || parent_pid == 0
            || child_pid == 0
            || self.child_process_pid != parent_pid
            || !self.child_process_group_visible
            || self.child_process_group == 0
            || self.child_process_session_id == 0
            || !self.pending_plain_fork_child_visible
            || self.pending_plain_fork_child_parent_pid != parent_pid
            || self.pending_plain_fork_child_pid != child_pid
            || self.pending_plain_fork_child_process_group == 0
            || self.pending_plain_fork_child_session_id == 0
            || self.pending_plain_fork_child_session_id != self.child_process_session_id
        {
            return false;
        }

        self.observed_child_parent_identity_saved = true;
        self.observed_child_parent_process_group = self.child_process_group;
        self.observed_child_parent_session_id = self.child_process_session_id;
        self.observed_child_parent_session_leader_first_slice =
            self.child_session_leader_first_slice;
        self.observed_child_parent_controlling_tty_bound = self.child_controlling_tty_bound;
        self.observed_child_parent_controlling_tty_cleared_on_setsid =
            self.child_controlling_tty_cleared_on_setsid;
        self.child_process_pid = child_pid;
        self.child_process_group = self.pending_plain_fork_child_process_group;
        self.child_process_session_id = self.pending_plain_fork_child_session_id;
        self.child_session_leader_first_slice = false;
        self.child_process_group_set_observed =
            self.pending_plain_fork_child_process_group_set_observed;
        self.child_setsid_success_observed = false;
        self.clear_pending_plain_fork_child_identity();
        true
    }

    pub fn restore_observed_child_parent_process_visible(
        &mut self,
        parent_pid: usize,
        child_pid: usize,
    ) -> bool {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || parent_pid == 0
            || child_pid == 0
            || self.child_process_pid != child_pid
            || !self.child_process_group_visible
            || self.child_process_group == 0
            || self.child_process_session_id == 0
        {
            return false;
        }

        self.child_process_pid = parent_pid;
        if self.observed_child_parent_identity_saved {
            self.child_process_group = self.observed_child_parent_process_group;
            self.child_process_session_id = self.observed_child_parent_session_id;
            self.child_session_leader_first_slice =
                self.observed_child_parent_session_leader_first_slice;
            self.child_controlling_tty_bound = self.observed_child_parent_controlling_tty_bound;
            self.child_controlling_tty_cleared_on_setsid =
                self.observed_child_parent_controlling_tty_cleared_on_setsid;
            self.observed_child_parent_identity_saved = false;
            self.observed_child_parent_process_group = 0;
            self.observed_child_parent_session_id = 0;
            self.observed_child_parent_session_leader_first_slice = false;
            self.observed_child_parent_controlling_tty_bound = false;
            self.observed_child_parent_controlling_tty_cleared_on_setsid = false;
        }
        self.clear_pending_plain_fork_child_identity();
        true
    }

    fn clear_pending_plain_fork_child_identity(&mut self) {
        self.pending_plain_fork_child_visible = false;
        self.pending_plain_fork_child_parent_pid = 0;
        self.pending_plain_fork_child_pid = 0;
        self.pending_plain_fork_child_process_group = 0;
        self.pending_plain_fork_child_session_id = 0;
        self.pending_plain_fork_child_process_group_set_observed = false;
    }

    pub fn set_process_group_first_slice(
        &mut self,
        pid_arg: usize,
        pgid_arg: usize,
        current_child_continuation: bool,
    ) -> UserProcessGroupUpdate {
        if !self.active_user_flow_online
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
        let current_pid = if current_child_continuation {
            self.child_process_pid
        } else {
            super::rest_init::KERNEL_INIT_PID
        };
        let normalized_pid = if pid == 0 {
            current_pid
        } else if pid < 0 {
            return UserProcessGroupUpdate::NoSuchProcess;
        } else {
            pid as usize
        };

        if current_child_continuation
            && self.pending_plain_fork_child_visible
            && self.child_process_pid == self.pending_plain_fork_child_parent_pid
            && normalized_pid == self.pending_plain_fork_child_pid
        {
            let normalized_pgid = if pgid == 0 {
                normalized_pid
            } else {
                pgid as usize
            };
            if self.pending_plain_fork_child_session_id != self.child_process_session_id
                || normalized_pgid != self.pending_plain_fork_child_pid
            {
                return UserProcessGroupUpdate::PermissionDenied;
            }
            self.pending_plain_fork_child_process_group = normalized_pgid;
            self.pending_plain_fork_child_process_group_set_observed = true;
            self.process_group_set_observed = true;
            return UserProcessGroupUpdate::UpdatedPendingChild(normalized_pgid);
        }

        if normalized_pid == self.child_process_pid && self.child_process_group_visible {
            if !self.child_process_group_visible {
                return UserProcessGroupUpdate::NoSuchProcess;
            }
            let normalized_pgid = if pgid == 0 {
                normalized_pid
            } else {
                pgid as usize
            };
            let joins_inherited_pgrp = self.child_process_session_id != 0
                && ((self.child_process_group != 0 && normalized_pgid == self.child_process_group)
                    || normalized_pgid == self.child_process_session_id);
            if normalized_pgid != self.child_process_pid && !joins_inherited_pgrp {
                return UserProcessGroupUpdate::PermissionDenied;
            }
            self.child_process_group = normalized_pgid;
            self.process_group_set_observed = true;
            self.child_process_group_set_observed = true;
            return UserProcessGroupUpdate::Updated(self.child_process_group);
        }

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

    pub fn set_session_id_first_slice(
        &mut self,
        current_child_continuation: bool,
    ) -> UserProcessGroupUpdate {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || self.session_id == 0
        {
            return UserProcessGroupUpdate::NotReady;
        }

        if current_child_continuation {
            let child_pid = self.child_process_pid;
            if child_pid == 0
                || !self.child_process_group_visible
                || self.child_process_session_id == 0
            {
                return UserProcessGroupUpdate::NotReady;
            }
            if self.child_session_leader_first_slice
                || self.process_group == child_pid
                || self.child_process_group == child_pid
            {
                return UserProcessGroupUpdate::PermissionDenied;
            }

            self.child_process_session_id = child_pid;
            self.child_process_group = child_pid;
            self.child_session_leader_first_slice = true;
            self.child_setsid_success_observed = true;
            self.child_controlling_tty_bound = false;
            self.child_controlling_tty_cleared_on_setsid = true;
            return UserProcessGroupUpdate::Updated(child_pid);
        }

        if !self.process_group_leader_first_slice
            || self.process_group != super::rest_init::KERNEL_INIT_PID
        {
            return UserProcessGroupUpdate::NotReady;
        }

        self.setsid_eperm_observed = true;
        UserProcessGroupUpdate::PermissionDenied
    }

    pub fn read_foreground_pgrp(&mut self) -> Option<usize> {
        if !self.active_user_flow_online
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
        if !self.active_user_flow_online
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
            let pgrp_value = pgrp as usize;
            let child_pgrp_matches =
                self.child_process_group_visible && self.child_process_group == pgrp_value;
            let saved_shell_pgrp_matches = self.observed_child_parent_identity_saved
                && self.observed_child_parent_process_group == pgrp_value
                && self.observed_child_parent_session_id != 0;
            let inherited_session_pgrp_matches =
                self.child_process_session_id != 0 && self.child_process_session_id == pgrp_value;
            if child_pgrp_matches || saved_shell_pgrp_matches || inherited_session_pgrp_matches {
                self.foreground_pgrp = pgrp as usize;
                self.foreground_pgrp_bound = true;
                self.foreground_pgrp_set_observed = true;
                return UserProcessGroupUpdate::Updated(self.foreground_pgrp);
            }
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

    pub fn read_tty_session_id_first_slice(
        &mut self,
        current_child_continuation: bool,
    ) -> UserProcessGroupLookup {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || self.session_id == 0
        {
            return UserProcessGroupLookup::NotReady;
        }

        if current_child_continuation {
            if self.child_process_pid == 0
                || !self.child_process_group_visible
                || self.child_process_session_id == 0
            {
                return UserProcessGroupLookup::NotReady;
            }
            if !self.child_controlling_tty_bound {
                return UserProcessGroupLookup::NotReady;
            }
            self.tty_session_id_read_observed = true;
            return UserProcessGroupLookup::Found(self.child_process_session_id);
        }

        if !self.controlling_tty_bound {
            return UserProcessGroupLookup::NotReady;
        }
        self.tty_session_id_read_observed = true;
        UserProcessGroupLookup::Found(self.session_id)
    }

    pub fn bind_controlling_tty_first_slice(
        &mut self,
        current_child_continuation: bool,
        _arg: usize,
    ) -> UserProcessGroupUpdate {
        if !self.active_user_flow_online
            || !self.pid1_preserved
            || !self.session_leader_first_slice
            || self.session_id == 0
        {
            return UserProcessGroupUpdate::NotReady;
        }

        if !current_child_continuation {
            return UserProcessGroupUpdate::PermissionDenied;
        }
        if self.child_process_pid == 0
            || !self.child_process_group_visible
            || self.child_process_session_id == 0
        {
            return UserProcessGroupUpdate::NotReady;
        }
        if !self.child_session_leader_first_slice
            || !self.child_controlling_tty_cleared_on_setsid
            || self.child_controlling_tty_bound
            || self.child_process_session_id != self.child_process_pid
            || self.child_process_group != self.child_process_pid
        {
            return UserProcessGroupUpdate::PermissionDenied;
        }

        self.child_controlling_tty_bound = true;
        self.foreground_pgrp = self.child_process_group;
        self.foreground_pgrp_bound = true;
        self.tiocsctty_observed = true;
        UserProcessGroupUpdate::Updated(self.child_process_session_id)
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

    pub fn read_supplementary_groups(
        &mut self,
    ) -> Option<(usize, [usize; USER_SUPPLEMENTARY_GROUP_MAX])> {
        if !self.credentials_syscall_ready() {
            return None;
        }
        self.supplementary_groups_read_observed = true;
        Some((self.supplementary_group_count, self.supplementary_groups))
    }

    pub fn set_uid_root_slice(&mut self, uid: usize) -> bool {
        if !self.credentials_syscall_ready() || self.euid != 0 || uid > u32::MAX as usize {
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
        if !self.credentials_syscall_ready() || self.euid != 0 || gid > u32::MAX as usize {
            return false;
        }
        self.gid = gid;
        self.egid = gid;
        self.sgid = gid;
        self.fsgid = gid;
        self.gid_set_observed = true;
        true
    }

    pub fn set_supplementary_groups_root_slice(
        &mut self,
        size: usize,
        first_gid: Option<usize>,
    ) -> bool {
        if !self.credentials_syscall_ready()
            || self.euid != 0
            || size > USER_SUPPLEMENTARY_GROUP_MAX
        {
            return false;
        }

        let gid = if size == 1 {
            let Some(gid) = first_gid else {
                return false;
            };
            gid
        } else {
            0
        };

        self.supplementary_groups = [0; USER_SUPPLEMENTARY_GROUP_MAX];
        self.supplementary_group_count = size;
        if size == 1 {
            self.supplementary_groups[0] = gid;
        }
        self.setgroups_observed = true;
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

    fn observe_rt_sigtimedwait(
        &mut self,
        mask: usize,
        uinfo_null: bool,
        uts_null: bool,
        pending_match: bool,
        infinite_wait: bool,
    ) -> bool {
        if !self.signal_wait_syscall_ready() {
            return false;
        }
        self.rt_sigtimedwait_mask = mask;
        self.rt_sigtimedwait_uinfo_null = uinfo_null;
        self.rt_sigtimedwait_uts_null = uts_null;
        self.rt_sigtimedwait_pending_match = pending_match;
        self.rt_sigtimedwait_infinite_wait = infinite_wait;
        self.rt_sigtimedwait_observed = true;
        true
    }

    pub fn begin_rt_sigtimedwait(
        &mut self,
        mask: usize,
        uinfo_null: bool,
        uts_null: bool,
        frame: &TrapFrame,
    ) -> UserRtSigtimedwaitResult {
        if !self.signal_wait_syscall_ready() {
            return UserRtSigtimedwaitResult::Unsupported;
        }

        let pending_match = self.pending_sigchld && sigchld_mask_matches(mask);
        if !self.observe_rt_sigtimedwait(mask, uinfo_null, uts_null, pending_match, uts_null) {
            return UserRtSigtimedwaitResult::Unsupported;
        }

        self.rt_sigtimedwait_sleep_reason = USER_SIGNAL_WAIT_REASON_NONE;
        self.rt_sigtimedwait_wake_signal = 0;
        self.rt_sigtimedwait_dequeued_signal = 0;
        self.rt_sigtimedwait_return_signal = 0;

        if pending_match {
            self.pending_sigchld = false;
            self.rt_sigtimedwait_sleeping = false;
            self.rt_sigtimedwait_saved_frame = None;
            self.rt_sigtimedwait_dequeued_signal = USER_CLONE_SIGCHLD;
            self.rt_sigtimedwait_return_signal = USER_CLONE_SIGCHLD;
            return UserRtSigtimedwaitResult::ReturnSignal(USER_CLONE_SIGCHLD);
        }

        self.rt_sigtimedwait_sleeping = true;
        self.rt_sigtimedwait_saved_frame = Some(*frame);
        self.rt_sigtimedwait_sleep_reason =
            USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE;
        self.rt_sigtimedwait_wait_queue.prepare_wait();
        UserRtSigtimedwaitResult::Sleep
    }

    pub fn record_child_exit_sigchld(&mut self) -> Option<bool> {
        if !self.active_user_flow_online || !self.pid1_preserved {
            return None;
        }

        self.pending_sigchld = true;
        if self.rt_sigtimedwait_sleeping && sigchld_mask_matches(self.rt_sigtimedwait_mask) {
            self.pending_sigchld = false;
            self.rt_sigtimedwait_pending_match = true;
            self.rt_sigtimedwait_sleeping = false;
            self.rt_sigtimedwait_wake_signal = USER_CLONE_SIGCHLD;
            self.rt_sigtimedwait_dequeued_signal = USER_CLONE_SIGCHLD;
            self.rt_sigtimedwait_return_signal = USER_CLONE_SIGCHLD;
            self.rt_sigtimedwait_wait_queue.wake_sigchld();
            self.rt_sigtimedwait_wait_queue.finish_wait();
            return Some(true);
        }
        Some(false)
    }

    pub fn complete_rt_sigtimedwait_wake(&mut self, frame: &mut TrapFrame) -> Option<usize> {
        if self.rt_sigtimedwait_sleeping
            || self.rt_sigtimedwait_return_signal != USER_CLONE_SIGCHLD
            || self.rt_sigtimedwait_dequeued_signal != USER_CLONE_SIGCHLD
        {
            return None;
        }

        let saved_frame = self.rt_sigtimedwait_saved_frame.take()?;
        *frame = saved_frame;
        Some(USER_CLONE_SIGCHLD)
    }

    pub fn observe_getcwd_root_slice(&mut self) -> bool {
        if !self.active_user_flow_online || !self.fs_struct_inherited {
            return false;
        }
        self.root_cwd_first_slice = true;
        self.getcwd_observed = true;
        true
    }
}

pub struct UserBootPayload {
    payload_lifecycle: Lifecycle,
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
            payload_lifecycle: Lifecycle::new(State::Base),
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
        self.payload_lifecycle.state()
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
        exec_sync: &ExecSyncBoundaries,
    ) -> EventResult {
        if self.payload_lifecycle.state() != State::Base
            || kernel_init_task.state() != State::OnCpu
            || exec_sync.state() != State::Ready
            || !exec_sync.kernel_execve_linux_window_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.payload_lifecycle.state(),
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
        self.payload_lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn try_candidate(
        &mut self,
        path: UserInitPathRef,
        actual_path: &[u8],
        elf: &ElfObject,
    ) -> EventResult {
        if self.payload_lifecycle.state() != State::Ready
            || !matches!(elf.state(), State::Ready | State::Online)
            || !valid_selected_path(actual_path)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.payload_lifecycle.state(),
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
        if self.payload_lifecycle.state() != State::Ready
            || failure.stage() == UserInitAttemptStage::None
            || failure.reason() == UserInitAttemptReason::None
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.payload_lifecycle.state(),
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
    #[allow(clippy::too_many_arguments)]
    pub fn enable_for_user_entry(
        &mut self,
        elf: &ElfObject,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        user_runtime: &UserAppRuntime,
        kernel_init_user_state: &KernelInitTaskUserState,
        exception_type: &ExceptionType,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if self.payload_lifecycle.state() != State::Ready
            || elf.state() != State::Online
            || address_space.state() != State::Online
            || trap_frame.state() != State::Ready
            || user_runtime.state() != State::Online
            || !user_runtime.active_binding_committed()
            || !kernel_init_user_state.active_user_flow_online()
            || !kernel_init_user_state.user_entry_ready()
            || !kernel_init_user_state.runtime_entered()
            || syscall_table.state() != State::Ready
            || exception_type.syscall_state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.payload_lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.enters_user_mode = true;
        self.no_return_handoff = true;
        self.payload_lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}
#[cfg(app_user_boot)]
pub fn prepare_first_user_init_handoff(ctx: &mut crate::context::Context) -> EventResult {
    if ctx.user_boot_payload.state() != State::Ready
        || !ctx.user_boot_payload.driven_by_kernel_init_task()
        || ctx.exec_sync_boundaries.state() != State::Ready
        || ctx.exec_transaction.state() != State::Ready
        || ctx.binary_format_registry.state() != State::Ready
    {
        user_boot_panic("user payload setup failed\n");
    }
    if !prepare_user_kernel_trap_stack(
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_table_caches,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &ctx.config,
    ) {
        user_boot_panic("user kernel trap stack setup failed\n");
    }
    if ctx
        .user_task_set
        .setup_with_carrier_stack(
            &ctx.kernel_init_task,
            user_kernel_trap_stack_base(),
            user_kernel_trap_stack_top(),
        )
        .is_err()
    {
        user_boot_panic("user child process preset failed\n");
    }

    let success = select_and_exec_user_init(ctx);
    let selected_path = ctx.user_boot_payload.selected_path();

    if ctx
        .cpu_group
        .boot_cpu_exception_mut()
        .expect("boot CPU exception resource must exist after CPU discovery")
        .syscall_setup(&mut ctx.syscall_table)
        .is_err()
    {
        user_boot_panic("user syscall setup failed\n");
    }
    if ctx
        .cpu_group
        .boot_cpu_exception_mut()
        .expect("boot CPU exception resource must exist after CPU discovery")
        .syscall_enable(&ctx.syscall_table)
        .is_err()
    {
        user_boot_panic("user syscall enable failed\n");
    }
    let trap = ctx
        .cpu_group
        .boot_cpu_trap_mut()
        .expect("boot CPU trap resource must exist after CPU discovery");
    if trap.exception_mut().enable().is_err()
        || trap.enable().is_err()
        || !trap.exception().service_online()
        || !trap.service_online()
    {
        user_boot_panic("user trap service enable failed\n");
    }
    if ctx
        .cpu_group
        .enable_secondary_trap_services(&ctx.syscall_table)
        .is_err()
    {
        user_boot_panic("secondary CPU trap service enable failed\n");
    }
    if ctx.files_struct.setup(&ctx.kernel_init_task).is_err() {
        user_boot_panic("user files struct setup failed\n");
    }
    if ctx.files_struct.clear_stdin_ready_data().is_err() {
        user_boot_panic("user stdin ready data clear failed\n");
    }
    if success.image_contains_stdin_fixture {
        if ctx
            .files_struct
            .prepare_default_stdin_ready_data(STDIN_READY_FIXTURE)
            .is_err()
        {
            user_boot_panic("user stdin ready data setup failed\n");
        }
    } else if ctx.files_struct.enable_stdin_blocking_wait().is_err() {
        user_boot_panic("user stdin blocking wait setup failed\n");
    }
    if ctx
        .kernel_init_user_state
        .setup(
            &ctx.kernel_init_task,
            &ctx.user_address_space,
            &ctx.elf_object,
            &ctx.user_trap_frame,
            &ctx.fs_struct,
            &ctx.files_struct,
            selected_path,
        )
        .is_err()
    {
        user_boot_panic("user init process setup failed\n");
    }
    let boot_cpu_exception = ctx
        .cpu_group
        .boot_cpu_exception()
        .expect("boot CPU exception resource must exist after CPU discovery");
    if ctx
        .kernel_init_user_state
        .bind_syscall_context(&ctx.user_trap_frame, boot_cpu_exception, &ctx.syscall_table)
        .is_err()
    {
        user_boot_panic("PID 1 syscall context bind failed\n");
    }
    if ctx.kernel_init_user_runtime.declare().is_err()
        || ctx
            .kernel_init_user_runtime
            .preset(&mut ctx.kernel_init_task)
            .is_err()
        || ctx
            .kernel_init_user_runtime
            .setup(&ctx.kernel_init_task, &ctx.kernel_init_user_state)
            .is_err()
    {
        user_boot_panic("PID 1 runtime preparation failed\n");
    }

    Ok(())
}

#[cfg(app_user_boot)]
pub fn commit_first_user_init_handoff(ctx: &mut crate::context::Context) -> EventResult {
    if ctx.kernel_init_user_runtime.state() != State::Ready
        || ctx.kernel_init_task.flow_state() != State::Online
        || ctx.kernel_init_task.task().flow() != TaskFlowRef::KERNEL_INIT
        || ctx.selected_payload_handoff.state() != State::Online
    {
        user_boot_panic("PID 1 runtime precommit failed\n");
    }

    if ctx
        .kernel_init_user_runtime
        .commit_active_binding(&ctx.kernel_init_task)
        .is_err()
        || ctx
            .kernel_init_user_runtime
            .enable(&ctx.kernel_init_task, &ctx.kernel_init_user_state)
            .is_err()
        || ctx
            .kernel_init_user_state
            .activate_user_flow(&ctx.kernel_init_user_runtime)
            .is_err()
    {
        user_boot_panic("PID 1 runtime commit failed\n");
    }
    if ctx
        .kernel_init_user_state
        .enter_user_mode(&ctx.user_trap_frame, &ctx.kernel_init_user_runtime)
        .is_err()
    {
        user_boot_panic("user init process enter failed\n");
    }
    let boot_cpu_exception = ctx
        .cpu_group
        .boot_cpu_exception()
        .expect("boot CPU exception resource must exist after CPU discovery");
    if ctx
        .user_boot_payload
        .enable_for_user_entry(
            &ctx.elf_object,
            &ctx.user_address_space,
            &ctx.user_trap_frame,
            &ctx.kernel_init_user_runtime,
            &ctx.kernel_init_user_state,
            boot_cpu_exception,
            &ctx.syscall_table,
        )
        .is_err()
    {
        user_boot_panic("user payload enable failed\n");
    }

    Ok(())
}

#[cfg(app_user_boot)]
pub fn enter_first_user_init(
    payload: &UserBootPayload,
    address_space: &UserAddressSpace,
    trap_frame: &UserTrapFrame,
    user_runtime: &UserAppRuntime,
    kernel_init_user_state: &KernelInitTaskUserState,
    trap_entry_context: usize,
) -> ! {
    if payload.state() != State::Online
        || !payload.enters_user_mode()
        || !payload.no_return_handoff()
        || address_space.state() != State::Online
        || trap_frame.state() != State::Ready
        || user_runtime.state() != State::Online
        || !user_runtime.application_entered()
        || !kernel_init_user_state.active_user_flow_online()
        || !kernel_init_user_state.user_entry_ready()
        || !kernel_init_user_state.runtime_entered()
    {
        user_boot_panic("user init process entry invariant failed\n");
    }
    crate::checkpoint::dispatch(
        crate::checkpoint::Checkpoint::UserAppRuntimeEnterUserMode,
        crate::context::context_ref(),
    );
    unsafe {
        crate::arch::riscv64::csr::enter_user_mode(
            address_space.satp_token(),
            trap_frame.entry(),
            trap_frame.sp(),
            trap_frame.sstatus(),
            trap_entry_context,
        )
    }
}

fn valid_selected_path(path: &[u8]) -> bool {
    !path.is_empty() && path.len() <= USER_SELECTED_PATH_MAX && path[0] == b'/'
}

#[cfg(app_user_boot)]
fn select_and_exec_user_init(
    ctx: &mut crate::context::Context,
) -> super::exec_transaction::ExecSuccess {
    let mut requested_path = [0u8; USER_SELECTED_PATH_MAX];
    let requested_len = ctx
        .boot_param
        .init_value(ctx.static_command_line.as_bytes())
        .map(|path| {
            if path.len() > requested_path.len() {
                return requested_path.len() + 1;
            }
            requested_path[..path.len()].copy_from_slice(path);
            path.len()
        });

    if let Some(len) = requested_len {
        let path = &requested_path[..core::cmp::min(len, requested_path.len())];
        match try_boot_exec_candidate(ctx, UserInitPathRef::RequestedInit, path) {
            Ok(success) => return success,
            Err(error) => {
                emit_exec_attempt_failure(
                    ctx,
                    UserInitPathRef::RequestedInit,
                    path,
                    error,
                    true,
                    false,
                );
                user_boot_panic("requested init failed\n");
            }
        }
    }

    let mut index = 0usize;
    while index < USER_INIT_CANDIDATES.len() {
        let path_ref = USER_INIT_CANDIDATES[index];
        match try_boot_exec_candidate(ctx, path_ref, path_ref.path()) {
            Ok(success) => return success,
            Err(super::exec_transaction::ExecError::EntropyUnavailable) => {
                user_boot_panic("init AT_RANDOM entropy unavailable\n")
            }
            Err(error) => {
                emit_exec_attempt_failure(ctx, path_ref, path_ref.path(), error, false, true)
            }
        }
        index += 1;
    }
    user_boot_panic("no working init found\n")
}

#[cfg(app_user_boot)]
fn try_boot_exec_candidate(
    ctx: &mut crate::context::Context,
    path_ref: UserInitPathRef,
    path: &[u8],
) -> Result<super::exec_transaction::ExecSuccess, super::exec_transaction::ExecError> {
    let success = super::exec_transaction::execute_slices(
        ctx,
        path,
        &[path],
        &[b"HOME=/", b"TERM=linux"],
        super::exec_transaction::ExecOwner::Boot,
        None,
    )?;
    if ctx
        .user_boot_payload
        .try_candidate(path_ref, path, &ctx.elf_object)
        .is_err()
    {
        print_post_commit_candidate_rejection(ctx, path);
        return Err(super::exec_transaction::ExecError::InvalidState);
    }
    Ok(success)
}

#[cfg(app_user_boot)]
fn print_post_commit_candidate_rejection(ctx: &crate::context::Context, path: &[u8]) {
    crate::arch::riscv64::sbi::putstr("user boot post-commit candidate rejected payload_state=");
    crate::arch::riscv64::sbi::putchar(ctx.user_boot_payload.state().code());
    crate::arch::riscv64::sbi::putstr(" elf_state=");
    crate::arch::riscv64::sbi::putchar(ctx.elf_object.state().code());
    crate::arch::riscv64::sbi::putstr(" path_valid=");
    crate::arch::riscv64::sbi::putchar(if valid_selected_path(path) {
        b'1'
    } else {
        b'0'
    });
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(app_user_boot)]
fn emit_exec_attempt_failure(
    ctx: &mut crate::context::Context,
    path_ref: UserInitPathRef,
    path: &[u8],
    error: super::exec_transaction::ExecError,
    requested_terminal: bool,
    default_nonfatal: bool,
) {
    let (stage, reason, elf_error) = classify_exec_attempt_failure(error);
    emit_init_attempt_failure(
        &mut ctx.user_boot_payload,
        UserInitAttemptFailure::new(
            path_ref,
            path,
            stage,
            reason,
            requested_terminal,
            default_nonfatal,
            elf_error,
        ),
    );
}

#[cfg(app_user_boot)]
fn classify_exec_attempt_failure(
    error: super::exec_transaction::ExecError,
) -> (
    UserInitAttemptStage,
    UserInitAttemptReason,
    Option<ElfError>,
) {
    use super::exec_transaction::ExecError;
    match error {
        ExecError::InvalidPath | ExecError::ArgumentsTooBig => (
            UserInitAttemptStage::ValidatePath,
            UserInitAttemptReason::InvalidPath,
            None,
        ),
        ExecError::NotFound => (
            UserInitAttemptStage::ReadImage,
            UserInitAttemptReason::ReadFailed,
            None,
        ),
        ExecError::NoMemory => (
            UserInitAttemptStage::SetupElf,
            UserInitAttemptReason::ElfSetupFailed,
            None,
        ),
        ExecError::EntropyUnavailable => (
            UserInitAttemptStage::SetupElf,
            UserInitAttemptReason::ElfSetupFailed,
            None,
        ),
        ExecError::NoExecutableFormat(Some(error @ ElfError::EntryOutsideExecutableSegment)) => (
            UserInitAttemptStage::UnsupportedCandidate,
            UserInitAttemptReason::CandidateUnsupported,
            Some(error),
        ),
        ExecError::NoExecutableFormat(Some(error)) => {
            let preset = matches!(
                error,
                ElfError::ShortInput
                    | ElfError::BadMagic
                    | ElfError::UnsupportedClass
                    | ElfError::UnsupportedEndian
                    | ElfError::UnsupportedVersion
                    | ElfError::UnsupportedType
                    | ElfError::UnsupportedMachine
            );
            if preset {
                (
                    UserInitAttemptStage::PresetElf,
                    UserInitAttemptReason::ElfPresetFailed,
                    Some(error),
                )
            } else {
                (
                    UserInitAttemptStage::SetupElf,
                    UserInitAttemptReason::ElfSetupFailed,
                    Some(error),
                )
            }
        }
        ExecError::NoExecutableFormat(None) => (
            UserInitAttemptStage::PresetElf,
            UserInitAttemptReason::ElfPresetFailed,
            None,
        ),
        ExecError::Busy | ExecError::InvalidState => (
            UserInitAttemptStage::UnsupportedCandidate,
            UserInitAttemptReason::CandidateUnsupported,
            None,
        ),
    }
}

#[cfg(app_user_boot)]
fn emit_init_attempt_failure(payload: &mut UserBootPayload, failure: UserInitAttemptFailure) {
    if payload.record_init_attempt_failure(failure).is_err() {
        user_boot_panic("user init failure record failed\n");
    }
    print_init_attempt_failure(failure);
    crate::checkpoint::dispatch(
        crate::checkpoint::Checkpoint::UserBootInitAttemptFailed,
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
#[allow(dead_code)]
pub fn user_kernel_trap_stack_base() -> usize {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .stack_base
        .load(Ordering::Acquire)
}

#[cfg(app_user_boot)]
pub fn user_kernel_trap_stack_top() -> usize {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .stack_top
        .load(Ordering::Acquire)
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_entry_context() -> usize {
    crate::context::context_ref()
        .cpu_group
        .boot_cpu_trap()
        .map(super::trap_type::TrapType::entry_context_address)
        .unwrap_or(0)
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_base_aligned() -> bool {
    user_kernel_trap_stack_base().is_multiple_of(USER_KERNEL_TRAP_STACK_ALIGN)
}

#[cfg(app_user_boot)]
pub fn user_kernel_trap_stack_ready() -> bool {
    USER_KERNEL_TRAP_STACK_RUNTIME.ready.load(Ordering::Acquire) != 0
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_vmapped() -> bool {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .vmapped
        .load(Ordering::Acquire)
        != 0
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_guard_base() -> usize {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .guard_base
        .load(Ordering::Acquire)
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_guard_size() -> usize {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .guard_size
        .load(Ordering::Acquire)
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_guard_unmapped() -> bool {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .guard_unmapped
        .load(Ordering::Acquire)
        != 0
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_backing_phys() -> usize {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .backing_phys
        .load(Ordering::Acquire)
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_stack_backing_order() -> usize {
    USER_KERNEL_TRAP_STACK_RUNTIME
        .backing_order
        .load(Ordering::Acquire)
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_overflow_stack_base() -> usize {
    super::trap_type::kernel_trap_overflow_stack_base()
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_overflow_stack_top() -> usize {
    super::trap_type::kernel_trap_overflow_stack_top()
}

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn user_kernel_trap_overflow_stack_ready() -> bool {
    user_kernel_trap_overflow_stack_base() != 0
        && user_kernel_trap_overflow_stack_top()
            == user_kernel_trap_overflow_stack_base() + USER_KERNEL_TRAP_OVERFLOW_STACK_SIZE
        && user_kernel_trap_overflow_stack_base().is_multiple_of(16)
}

#[cfg(app_user_boot)]
fn prepare_user_kernel_trap_stack(
    vmalloc_allocator: &mut VmallocAllocator,
    page_table_caches: &mut PageTableCaches,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
    config: &Config,
) -> bool {
    if user_kernel_trap_stack_ready() {
        return true;
    }
    if config.page_size() != USER_PAGE_SIZE {
        return false;
    }
    let Some(area) = vmalloc_allocator.get_vm_area_aligned_with_guard(
        USER_KERNEL_TRAP_STACK_SIZE,
        USER_KERNEL_TRAP_STACK_ALIGN,
        USER_KERNEL_TRAP_GUARD_SIZE,
        VmapAreaFlags::VmStack,
    ) else {
        return false;
    };
    if !area.is_vm_stack()
        || area.size() != USER_KERNEL_TRAP_STACK_SIZE
        || area.virt_base() % USER_KERNEL_TRAP_STACK_ALIGN != 0
    {
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    }
    let Some(guard_base) = area.virt_base().checked_sub(USER_KERNEL_TRAP_GUARD_SIZE) else {
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    };
    let Some(page) = page_allocator.alloc_pages(
        USER_KERNEL_TRAP_STACK_ORDER,
        GfpFlags::kernel(),
        page_metadata_map,
    ) else {
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    };
    let Some(phys) = page_metadata_map
        .page_to_phys(page)
        .map(|addr| addr.value())
    else {
        let _ = page_allocator.free_pages(page, USER_KERNEL_TRAP_STACK_ORDER, page_metadata_map);
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    };
    let Some(linear) = page_metadata_map.page_address(page) else {
        let _ = page_allocator.free_pages(page, USER_KERNEL_TRAP_STACK_ORDER, page_metadata_map);
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    };
    unsafe {
        core::ptr::write_bytes(linear as *mut u8, 0, USER_KERNEL_TRAP_STACK_SIZE);
    }
    let Some(mapping) = vmalloc_allocator.map_page_range(
        page_table_caches,
        page_allocator,
        page_metadata_map,
        config,
        area,
        phys,
        USER_KERNEL_TRAP_STACK_SIZE,
        PageProtection::KernelData,
    ) else {
        let _ = page_allocator.free_pages(page, USER_KERNEL_TRAP_STACK_ORDER, page_metadata_map);
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    };
    if !mapping.uses_kernel_data_protection() {
        let _ = vmalloc_allocator.unmap_page_range(mapping);
        let _ = page_allocator.free_pages(page, USER_KERNEL_TRAP_STACK_ORDER, page_metadata_map);
        let _ = vmalloc_allocator.free_vm_area(area);
        return false;
    }

    USER_KERNEL_TRAP_STACK_RUNTIME
        .guard_base
        .store(guard_base, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .guard_size
        .store(USER_KERNEL_TRAP_GUARD_SIZE, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .guard_unmapped
        .store(1, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .stack_base
        .store(area.virt_base(), Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .stack_top
        .store(area.end(), Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .stack_size
        .store(area.size(), Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .stack_align
        .store(USER_KERNEL_TRAP_STACK_ALIGN, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .backing_phys
        .store(phys, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .backing_order
        .store(USER_KERNEL_TRAP_STACK_ORDER, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .vmapped
        .store(1, Ordering::Release);
    USER_KERNEL_TRAP_STACK_RUNTIME
        .ready
        .store(1, Ordering::Release);
    true
}

#[cfg(app_user_boot)]
fn user_boot_panic(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
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
        mapping.backing_vaddrs[index] = mapping
            .vaddr()
            .saturating_sub(mapping.page_offset())
            .saturating_add(index * USER_PAGE_SIZE);
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
            mapping.backing_vaddrs[mapping.backing_page_count] = 0;
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
    Ok(end.div_ceil(USER_PAGE_SIZE))
}

fn mappings_have_backing_pages(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if !matches!(
            mappings[index].kind(),
            UserMappingKind::Stack | UserMappingKind::Heap | UserMappingKind::AnonymousPrivate
        ) && mappings[index].backing_page_count() == 0
        {
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
    if a < b { a } else { b }
}
