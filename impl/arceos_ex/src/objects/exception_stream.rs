#[cfg(app_user_boot)]
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::trace::{self, Checkpoint};

#[cfg(checkpoint_handler_user_syscall_error)]
use super::files::OpenFileDescriptionRef;
#[cfg(app_user_boot)]
use super::user_boot::{
    ElfError, ElfObject, UserAddressSpace, UserStack, UserTrapFrame, USER_EXEC_ARG_MAX,
};
use super::{
    event_stream::{EventStream, TrapFrame},
    files::{is_null_path, is_tty_path, CloseOnExecReport, FileError, FILE_POLLIN, TERMIOS_SIZE},
    hwrng::HwRngError,
    init_stack::InitStack,
    process_prepare::TaskCopyUserProcessInputs,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    task::TaskEntry,
    user_boot::{
        UserFaultAccess, UserFaultMappingDiagnostic, UserMappingKind, UserMmapError,
        UserProcessGroupLookup, UserProcessGroupUpdate, UserRtSigtimedwaitResult, UserSignalAction,
        USER_CHILD_PID, USER_SIGNAL_COUNT,
        USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE, USER_WAIT4_ALL_CHILDREN,
        USER_WAIT4_WUNTRACED,
    },
};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);
const EXC_INSTRUCTION_PAGE_FAULT: usize = 12;
const EXC_LOAD_PAGE_FAULT: usize = 13;
const EXC_STORE_PAGE_FAULT: usize = 15;
const EXC_BREAKPOINT: usize = 3;
const EXC_USER_ECALL: usize = 8;
const EXC_SUPERVISOR_ECALL: usize = 9;
const EXCEPTION_HANDLER_COUNT: usize = 16;

const HANDLER_FALLBACK: u8 = 0;
const HANDLER_PAGE_FAULT: u8 = 1;
const HANDLER_SYSCALL_DISABLED: u8 = 2;
const HANDLER_BREAKPOINT: u8 = 3;
const HANDLER_UNEXPECTED: u8 = 4;
const HANDLER_SYSCALL: u8 = 5;

const PAGE_FAULT_CAUSES: [usize; 3] = [
    EXC_INSTRUCTION_PAGE_FAULT,
    EXC_LOAD_PAGE_FAULT,
    EXC_STORE_PAGE_FAULT,
];
const SYSCALL_CAUSES: [usize; 2] = [EXC_USER_ECALL, EXC_SUPERVISOR_ECALL];
const BREAKPOINT_CAUSES: [usize; 1] = [EXC_BREAKPOINT];

static DISPATCH_READY: AtomicU8 = AtomicU8::new(0);
static EXCEPTION_HANDLER_POLICY: [AtomicU8; EXCEPTION_HANDLER_COUNT] =
    [const { AtomicU8::new(HANDLER_FALLBACK) }; EXCEPTION_HANDLER_COUNT];

#[derive(Clone, Copy)]
struct ExceptionPolicy(u8);

const FALLBACK_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_FALLBACK);
const PAGE_FAULT_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_PAGE_FAULT);
const SYSCALL_DISABLED_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_SYSCALL_DISABLED);
const BREAKPOINT_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_BREAKPOINT);
const UNEXPECTED_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_UNEXPECTED);
#[cfg(app_user_boot)]
const SYSCALL_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_SYSCALL);
#[cfg(not(app_user_boot))]
const SYSCALL_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_SYSCALL_DISABLED);
const SYSCALL_GETCWD: usize = 17;
const SYSCALL_DUP3: usize = 24;
const SYSCALL_FCNTL: usize = 25;
const SYSCALL_IOCTL: usize = 29;
const SYSCALL_FACCESSAT: usize = 48;
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_GETDENTS64: usize = 61;
const SYSCALL_LSEEK: usize = 62;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_WRITEV: usize = 66;
const SYSCALL_PPOLL: usize = 73;
const SYSCALL_READLINKAT: usize = 78;
const SYSCALL_NEWFSTATAT: usize = 79;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_EXIT_GROUP: usize = 94;
const SYSCALL_SET_TID_ADDRESS: usize = 96;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_CLOCK_GETTIME: usize = 113;
const SYSCALL_RT_SIGACTION: usize = 134;
const SYSCALL_RT_SIGPROCMASK: usize = 135;
const SYSCALL_RT_SIGTIMEDWAIT: usize = 137;
const SYSCALL_SETGID: usize = 144;
const SYSCALL_SETUID: usize = 146;
const SYSCALL_GETRESUID: usize = 148;
const SYSCALL_GETRESGID: usize = 150;
const SYSCALL_SETPGID: usize = 154;
const SYSCALL_GETPGID: usize = 155;
const SYSCALL_GETSID: usize = 156;
const SYSCALL_SETSID: usize = 157;
const SYSCALL_UNAME: usize = 160;
const SYSCALL_GETTIMEOFDAY: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_GETUID: usize = 174;
const SYSCALL_GETEUID: usize = 175;
const SYSCALL_GETGID: usize = 176;
const SYSCALL_GETEGID: usize = 177;
const SYSCALL_BRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXECVE: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MPROTECT: usize = 226;
const SYSCALL_WAIT4: usize = 260;
const SYSCALL_GETRANDOM: usize = 278;
const USER_COPY_MAX: usize = 256;
const USER_IOV_MAX: usize = 4;
const USER_PATH_MAX: usize = crate::objects::files::FILE_PATH_MAX;
const USER_EXECVE_VECTOR_DIAG_MAX: usize = 4;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_NONE: usize = 0;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_ARGS_READY: usize = 1;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_MAIN_ELF_READY: usize = 2;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_INTERPRETER_READY: usize = 3;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_ADDRESS_SPACE_READY: usize = 4;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_CONTEXT_REPLACED: usize = 5;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_SATP_READY: usize = 6;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_TRAP_FRAME_READY: usize = 7;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_SATP_SWITCHED: usize = 8;
#[cfg(app_user_boot)]
const EXECVE_OBS_STAGE_RETURN_FRAME_READY: usize = 9;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_NONE: usize = 0;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_CHILD_CONTINUATION: usize = 1;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_PATH_KIND: usize = 2;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_PATH_READ: usize = 3;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_MAIN_PRESET: usize = 4;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_MAIN_SETUP: usize = 5;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_INTERPRETER_READ: usize = 6;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_INTERPRETER_PRESET: usize = 7;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_INTERPRETER_SETUP: usize = 8;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_INTERPRETER_BIND: usize = 9;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_ADDRESS_SPACE_PRESET: usize = 10;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_STACK_SETUP: usize = 11;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_ADDRESS_SPACE_SETUP: usize = 12;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_TRAP_FRAME_SETUP: usize = 13;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_ELF_ENABLE: usize = 14;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_ADDRESS_SPACE_ENABLE: usize = 15;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_CLOSE_ON_EXEC: usize = 16;
#[cfg(app_user_boot)]
const EXECVE_FAIL_STAGE_ARGV_COPY: usize = 17;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_NONE: usize = 0;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_NOT_CHILD_CONTINUATION: usize = 1;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_RELATIVE_PATH: usize = 2;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_VFS_READ: usize = 3;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_UNSUPPORTED_ELF: usize = 4;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_INVALID_STATE: usize = 5;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_INTERPRETER_READ: usize = 6;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_ADDRESS_SPACE: usize = 7;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_STACK: usize = 8;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_TRAP_FRAME: usize = 9;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_CLOSE_ON_EXEC: usize = 10;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_ARGV_COPY: usize = 11;
#[cfg(app_user_boot)]
const EXECVE_FAIL_REASON_ARGV_CAPACITY: usize = 12;
const AT_FDCWD: usize = usize::MAX - 99;
const ACCESS_X_OK: usize = 1;
const ACCESS_W_OK: usize = 2;
const ACCESS_R_OK: usize = 4;
const O_ACCMODE: usize = 0o3;
const O_NONBLOCK: usize = 0o4000;
const O_LARGEFILE: usize = 0o100000;
const O_DIRECTORY: usize = 0o200000;
const O_CLOEXEC: usize = 0o2000000;
const AT_SYMLINK_NOFOLLOW: usize = 0x100;
const F_DUPFD: usize = 0;
const F_GETFD: usize = 1;
const F_SETFD: usize = 2;
const F_GETFL: usize = 3;
const F_SETFL: usize = 4;
const F_LINUX_SPECIFIC_BASE: usize = 1024;
const F_DUPFD_CLOEXEC: usize = F_LINUX_SPECIFIC_BASE + 6;
const FD_CLOEXEC: usize = 1;
const TCGETS: usize = 0x5401;
const TCSETS: usize = 0x5402;
const TIOCSCTTY: usize = 0x540e;
const TIOCGPGRP: usize = 0x540f;
const TIOCSPGRP: usize = 0x5410;
const TIOCGWINSZ: usize = 0x5413;
const TIOCGSID: usize = 0x5429;
const WINSIZE_SIZE: usize = 8;
const STAT_SIZE: usize = 128;
const TIMESPEC_SIZE: usize = 16;
const TIMEVAL_SIZE: usize = 16;
const TIMEZONE_SIZE: usize = 8;
const POLLFD_SIZE: usize = 8;
const USER_PPOLL_MAX: usize = 8;
const POLLNVAL: u16 = 0x0020;
const RT_SIGACTION_SIZE: usize = core::mem::size_of::<usize>() * 3;
const UID_T_SIZE: usize = 4;
const GID_T_SIZE: usize = 4;
const PID_T_SIZE: usize = 4;
const UTS_FIELD_SIZE: usize = 65;
const NEW_UTSNAME_FIELDS: usize = 6;
const NEW_UTSNAME_SIZE: usize = UTS_FIELD_SIZE * NEW_UTSNAME_FIELDS;
const CLOCK_REALTIME: usize = 0;
const CLOCK_MONOTONIC: usize = 1;
const NSEC_PER_SEC: u64 = 1_000_000_000;
const USEC_PER_SEC: u64 = 1_000_000;
const NANOSLEEP_FIRST_SLICE_MAX_NS: u64 = 100_000_000;
const RT_SIGSET_SIZE: usize = core::mem::size_of::<usize>();
const SIG_BLOCK: usize = 0;
const SIG_UNBLOCK: usize = 1;
const SIG_SETMASK: usize = 2;
const SIGKILL: usize = 9;
const SIGSTOP: usize = 19;
const UNBLOCKABLE_SIGNAL_MASK: usize = (1usize << (SIGKILL - 1)) | (1usize << (SIGSTOP - 1));
const UAPI_SA_FLAGS: usize = 0xd800_0807;
const WAIT4_WNOHANG: usize = 0x0000_0001;
const WAIT4_WCONTINUED: usize = 0x0000_0008;
const WAIT4_WNOTHREAD: usize = 0x2000_0000;
const WAIT4_WALL: usize = 0x4000_0000;
const WAIT4_WCLONE: usize = 0x8000_0000;
const WAIT4_LINUX_VALID_OPTIONS: usize = WAIT4_WNOHANG
    | USER_WAIT4_WUNTRACED
    | WAIT4_WCONTINUED
    | WAIT4_WNOTHREAD
    | WAIT4_WALL
    | WAIT4_WCLONE;
const EPERM: usize = 1;
const EACCES: usize = 13;
const ESRCH: usize = 3;
const EFAULT: usize = 14;
const EINVAL: usize = 22;
const EIO: usize = 5;
const ENOSYS: usize = 38;
const ENOMEM: usize = 12;
const EAGAIN: usize = 11;
const ENOENT: usize = 2;
const ERANGE: usize = 34;
const EOVERFLOW: usize = 75;
const EREMOTEIO: usize = 121;
const ESPIPE: usize = 29;
const ENOTTY: usize = 25;
const ELOOP: usize = 40;
const EMFILE: usize = 24;
const ENOTDIR: usize = 20;
const ECHILD: usize = 10;

#[cfg(app_user_boot)]
#[derive(Clone, Copy)]
pub struct ExecveCheckpointObservation {
    pub stage: usize,
    pub failure_stage: usize,
    pub failure_reason: usize,
    pub failure_detail: usize,
    pub failure_free_pages: usize,
    pub failure_totalram_pages: usize,
    pub filename_len: usize,
    pub argv0_len: usize,
    pub argv_argc: usize,
    pub argv_total_bytes: usize,
    pub argv_capacity_exceeded: usize,
    pub main_elf_type: usize,
    pub main_input_len: usize,
    pub main_load_bias: usize,
    pub main_entry: usize,
    pub main_runtime_entry: usize,
    pub main_segments: usize,
    pub main_interpreter_required: usize,
    pub interpreter_path_len: usize,
    pub interpreter_input_len: usize,
    pub interpreter_load_bias: usize,
    pub interpreter_entry: usize,
    pub interpreter_segments: usize,
    pub address_space_state: usize,
    pub mapping_count: usize,
    pub segment_mapping_count: usize,
    pub satp_token: usize,
    pub trap_entry: usize,
    pub trap_sp: usize,
    pub trap_sstatus: usize,
    pub old_satp: usize,
    pub new_satp: usize,
    pub current_satp: usize,
    pub frame_before_sepc: usize,
    pub frame_before_sp: usize,
    pub frame_before_ra: usize,
    pub frame_before_sstatus: usize,
    pub frame_after_sepc: usize,
    pub frame_after_sp: usize,
    pub frame_after_ra: usize,
    pub frame_after_sstatus: usize,
    pub kernel_sp: usize,
}

#[cfg(app_user_boot)]
struct ExecveCheckpointObservationAtomics {
    stage: AtomicUsize,
    failure_stage: AtomicUsize,
    failure_reason: AtomicUsize,
    failure_detail: AtomicUsize,
    failure_free_pages: AtomicUsize,
    failure_totalram_pages: AtomicUsize,
    filename_len: AtomicUsize,
    argv0_len: AtomicUsize,
    argv_argc: AtomicUsize,
    argv_total_bytes: AtomicUsize,
    argv_capacity_exceeded: AtomicUsize,
    main_elf_type: AtomicUsize,
    main_input_len: AtomicUsize,
    main_load_bias: AtomicUsize,
    main_entry: AtomicUsize,
    main_runtime_entry: AtomicUsize,
    main_segments: AtomicUsize,
    main_interpreter_required: AtomicUsize,
    interpreter_path_len: AtomicUsize,
    interpreter_input_len: AtomicUsize,
    interpreter_load_bias: AtomicUsize,
    interpreter_entry: AtomicUsize,
    interpreter_segments: AtomicUsize,
    address_space_state: AtomicUsize,
    mapping_count: AtomicUsize,
    segment_mapping_count: AtomicUsize,
    satp_token: AtomicUsize,
    trap_entry: AtomicUsize,
    trap_sp: AtomicUsize,
    trap_sstatus: AtomicUsize,
    old_satp: AtomicUsize,
    new_satp: AtomicUsize,
    current_satp: AtomicUsize,
    frame_before_sepc: AtomicUsize,
    frame_before_sp: AtomicUsize,
    frame_before_ra: AtomicUsize,
    frame_before_sstatus: AtomicUsize,
    frame_after_sepc: AtomicUsize,
    frame_after_sp: AtomicUsize,
    frame_after_ra: AtomicUsize,
    frame_after_sstatus: AtomicUsize,
    kernel_sp: AtomicUsize,
}

#[cfg(app_user_boot)]
static EXECVE_CHECKPOINT_OBSERVATION: ExecveCheckpointObservationAtomics =
    ExecveCheckpointObservationAtomics {
        stage: AtomicUsize::new(EXECVE_OBS_STAGE_NONE),
        failure_stage: AtomicUsize::new(EXECVE_FAIL_STAGE_NONE),
        failure_reason: AtomicUsize::new(EXECVE_FAIL_REASON_NONE),
        failure_detail: AtomicUsize::new(0),
        failure_free_pages: AtomicUsize::new(0),
        failure_totalram_pages: AtomicUsize::new(0),
        filename_len: AtomicUsize::new(0),
        argv0_len: AtomicUsize::new(0),
        argv_argc: AtomicUsize::new(0),
        argv_total_bytes: AtomicUsize::new(0),
        argv_capacity_exceeded: AtomicUsize::new(0),
        main_elf_type: AtomicUsize::new(0),
        main_input_len: AtomicUsize::new(0),
        main_load_bias: AtomicUsize::new(0),
        main_entry: AtomicUsize::new(0),
        main_runtime_entry: AtomicUsize::new(0),
        main_segments: AtomicUsize::new(0),
        main_interpreter_required: AtomicUsize::new(0),
        interpreter_path_len: AtomicUsize::new(0),
        interpreter_input_len: AtomicUsize::new(0),
        interpreter_load_bias: AtomicUsize::new(0),
        interpreter_entry: AtomicUsize::new(0),
        interpreter_segments: AtomicUsize::new(0),
        address_space_state: AtomicUsize::new(0),
        mapping_count: AtomicUsize::new(0),
        segment_mapping_count: AtomicUsize::new(0),
        satp_token: AtomicUsize::new(0),
        trap_entry: AtomicUsize::new(0),
        trap_sp: AtomicUsize::new(0),
        trap_sstatus: AtomicUsize::new(0),
        old_satp: AtomicUsize::new(0),
        new_satp: AtomicUsize::new(0),
        current_satp: AtomicUsize::new(0),
        frame_before_sepc: AtomicUsize::new(0),
        frame_before_sp: AtomicUsize::new(0),
        frame_before_ra: AtomicUsize::new(0),
        frame_before_sstatus: AtomicUsize::new(0),
        frame_after_sepc: AtomicUsize::new(0),
        frame_after_sp: AtomicUsize::new(0),
        frame_after_ra: AtomicUsize::new(0),
        frame_after_sstatus: AtomicUsize::new(0),
        kernel_sp: AtomicUsize::new(0),
    };

#[cfg(app_user_boot)]
pub fn execve_checkpoint_observation() -> ExecveCheckpointObservation {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    ExecveCheckpointObservation {
        stage: obs.stage.load(Ordering::Acquire),
        failure_stage: obs.failure_stage.load(Ordering::Acquire),
        failure_reason: obs.failure_reason.load(Ordering::Acquire),
        failure_detail: obs.failure_detail.load(Ordering::Acquire),
        failure_free_pages: obs.failure_free_pages.load(Ordering::Acquire),
        failure_totalram_pages: obs.failure_totalram_pages.load(Ordering::Acquire),
        filename_len: obs.filename_len.load(Ordering::Acquire),
        argv0_len: obs.argv0_len.load(Ordering::Acquire),
        argv_argc: obs.argv_argc.load(Ordering::Acquire),
        argv_total_bytes: obs.argv_total_bytes.load(Ordering::Acquire),
        argv_capacity_exceeded: obs.argv_capacity_exceeded.load(Ordering::Acquire),
        main_elf_type: obs.main_elf_type.load(Ordering::Acquire),
        main_input_len: obs.main_input_len.load(Ordering::Acquire),
        main_load_bias: obs.main_load_bias.load(Ordering::Acquire),
        main_entry: obs.main_entry.load(Ordering::Acquire),
        main_runtime_entry: obs.main_runtime_entry.load(Ordering::Acquire),
        main_segments: obs.main_segments.load(Ordering::Acquire),
        main_interpreter_required: obs.main_interpreter_required.load(Ordering::Acquire),
        interpreter_path_len: obs.interpreter_path_len.load(Ordering::Acquire),
        interpreter_input_len: obs.interpreter_input_len.load(Ordering::Acquire),
        interpreter_load_bias: obs.interpreter_load_bias.load(Ordering::Acquire),
        interpreter_entry: obs.interpreter_entry.load(Ordering::Acquire),
        interpreter_segments: obs.interpreter_segments.load(Ordering::Acquire),
        address_space_state: obs.address_space_state.load(Ordering::Acquire),
        mapping_count: obs.mapping_count.load(Ordering::Acquire),
        segment_mapping_count: obs.segment_mapping_count.load(Ordering::Acquire),
        satp_token: obs.satp_token.load(Ordering::Acquire),
        trap_entry: obs.trap_entry.load(Ordering::Acquire),
        trap_sp: obs.trap_sp.load(Ordering::Acquire),
        trap_sstatus: obs.trap_sstatus.load(Ordering::Acquire),
        old_satp: obs.old_satp.load(Ordering::Acquire),
        new_satp: obs.new_satp.load(Ordering::Acquire),
        current_satp: obs.current_satp.load(Ordering::Acquire),
        frame_before_sepc: obs.frame_before_sepc.load(Ordering::Acquire),
        frame_before_sp: obs.frame_before_sp.load(Ordering::Acquire),
        frame_before_ra: obs.frame_before_ra.load(Ordering::Acquire),
        frame_before_sstatus: obs.frame_before_sstatus.load(Ordering::Acquire),
        frame_after_sepc: obs.frame_after_sepc.load(Ordering::Acquire),
        frame_after_sp: obs.frame_after_sp.load(Ordering::Acquire),
        frame_after_ra: obs.frame_after_ra.load(Ordering::Acquire),
        frame_after_sstatus: obs.frame_after_sstatus.load(Ordering::Acquire),
        kernel_sp: obs.kernel_sp.load(Ordering::Acquire),
    }
}

#[cfg(app_user_boot)]
#[derive(Clone, Copy)]
pub struct Wait4CheckpointObservation {
    pub saved: usize,
    pub saved_sepc: usize,
    pub saved_sp: usize,
    pub saved_s2: usize,
    pub saved_s4: usize,
    pub saved_a7: usize,
    pub saved_satp: usize,
    pub saved_status_ptr: usize,
    pub saved_child_pid: usize,
    pub stack_window_saved: usize,
    pub stack_window_start: usize,
    pub stack_window_len: usize,
    pub writable_pages_saved: usize,
    pub writable_pages_count: usize,
    pub writable_pages_truncated: usize,
    pub writable_pages_restored: usize,
    pub resumed: usize,
    pub resumed_sepc: usize,
    pub resumed_sp: usize,
    pub resumed_s2: usize,
    pub resumed_s4: usize,
    pub resumed_a0: usize,
    pub resumed_a7: usize,
    pub resumed_satp: usize,
    pub resumed_status_ptr: usize,
    pub resumed_wait_status: usize,
    pub resumed_status_copied: usize,
    pub child_exit_status: usize,
    pub stack_window_compared: usize,
    pub stack_window_diff_count: usize,
    pub stack_window_first_diff_addr: usize,
    pub stack_window_before_byte: usize,
    pub stack_window_after_byte: usize,
    pub writable_pages_compared: usize,
    pub writable_pages_dirty_count: usize,
    pub writable_pages_stack_dirty_count: usize,
    pub writable_pages_non_stack_dirty_count: usize,
    pub writable_pages_first_non_stack_kind: usize,
    pub writable_pages_first_non_stack_mapping_index: usize,
    pub writable_pages_first_non_stack_page_index: usize,
    pub writable_pages_first_non_stack_addr: usize,
    pub writable_pages_first_non_stack_before_checksum: usize,
    pub writable_pages_first_non_stack_after_checksum: usize,
}

#[cfg(app_user_boot)]
struct Wait4CheckpointObservationAtomics {
    saved: AtomicUsize,
    saved_sepc: AtomicUsize,
    saved_sp: AtomicUsize,
    saved_s2: AtomicUsize,
    saved_s4: AtomicUsize,
    saved_a7: AtomicUsize,
    saved_satp: AtomicUsize,
    saved_status_ptr: AtomicUsize,
    saved_child_pid: AtomicUsize,
    stack_window_saved: AtomicUsize,
    stack_window_start: AtomicUsize,
    stack_window_len: AtomicUsize,
    writable_pages_saved: AtomicUsize,
    writable_pages_count: AtomicUsize,
    writable_pages_truncated: AtomicUsize,
    writable_pages_restored: AtomicUsize,
    resumed: AtomicUsize,
    resumed_sepc: AtomicUsize,
    resumed_sp: AtomicUsize,
    resumed_s2: AtomicUsize,
    resumed_s4: AtomicUsize,
    resumed_a0: AtomicUsize,
    resumed_a7: AtomicUsize,
    resumed_satp: AtomicUsize,
    resumed_status_ptr: AtomicUsize,
    resumed_wait_status: AtomicUsize,
    resumed_status_copied: AtomicUsize,
    child_exit_status: AtomicUsize,
    stack_window_compared: AtomicUsize,
    stack_window_diff_count: AtomicUsize,
    stack_window_first_diff_addr: AtomicUsize,
    stack_window_before_byte: AtomicUsize,
    stack_window_after_byte: AtomicUsize,
    writable_pages_compared: AtomicUsize,
    writable_pages_dirty_count: AtomicUsize,
    writable_pages_stack_dirty_count: AtomicUsize,
    writable_pages_non_stack_dirty_count: AtomicUsize,
    writable_pages_first_non_stack_kind: AtomicUsize,
    writable_pages_first_non_stack_mapping_index: AtomicUsize,
    writable_pages_first_non_stack_page_index: AtomicUsize,
    writable_pages_first_non_stack_addr: AtomicUsize,
    writable_pages_first_non_stack_before_checksum: AtomicUsize,
    writable_pages_first_non_stack_after_checksum: AtomicUsize,
}

#[cfg(app_user_boot)]
static WAIT4_CHECKPOINT_OBSERVATION: Wait4CheckpointObservationAtomics =
    Wait4CheckpointObservationAtomics {
        saved: AtomicUsize::new(0),
        saved_sepc: AtomicUsize::new(0),
        saved_sp: AtomicUsize::new(0),
        saved_s2: AtomicUsize::new(0),
        saved_s4: AtomicUsize::new(0),
        saved_a7: AtomicUsize::new(0),
        saved_satp: AtomicUsize::new(0),
        saved_status_ptr: AtomicUsize::new(0),
        saved_child_pid: AtomicUsize::new(0),
        stack_window_saved: AtomicUsize::new(0),
        stack_window_start: AtomicUsize::new(0),
        stack_window_len: AtomicUsize::new(0),
        writable_pages_saved: AtomicUsize::new(0),
        writable_pages_count: AtomicUsize::new(0),
        writable_pages_truncated: AtomicUsize::new(0),
        writable_pages_restored: AtomicUsize::new(0),
        resumed: AtomicUsize::new(0),
        resumed_sepc: AtomicUsize::new(0),
        resumed_sp: AtomicUsize::new(0),
        resumed_s2: AtomicUsize::new(0),
        resumed_s4: AtomicUsize::new(0),
        resumed_a0: AtomicUsize::new(0),
        resumed_a7: AtomicUsize::new(0),
        resumed_satp: AtomicUsize::new(0),
        resumed_status_ptr: AtomicUsize::new(0),
        resumed_wait_status: AtomicUsize::new(0),
        resumed_status_copied: AtomicUsize::new(0),
        child_exit_status: AtomicUsize::new(0),
        stack_window_compared: AtomicUsize::new(0),
        stack_window_diff_count: AtomicUsize::new(0),
        stack_window_first_diff_addr: AtomicUsize::new(0),
        stack_window_before_byte: AtomicUsize::new(0),
        stack_window_after_byte: AtomicUsize::new(0),
        writable_pages_compared: AtomicUsize::new(0),
        writable_pages_dirty_count: AtomicUsize::new(0),
        writable_pages_stack_dirty_count: AtomicUsize::new(0),
        writable_pages_non_stack_dirty_count: AtomicUsize::new(0),
        writable_pages_first_non_stack_kind: AtomicUsize::new(0),
        writable_pages_first_non_stack_mapping_index: AtomicUsize::new(0),
        writable_pages_first_non_stack_page_index: AtomicUsize::new(0),
        writable_pages_first_non_stack_addr: AtomicUsize::new(0),
        writable_pages_first_non_stack_before_checksum: AtomicUsize::new(0),
        writable_pages_first_non_stack_after_checksum: AtomicUsize::new(0),
    };

#[cfg(app_user_boot)]
pub fn wait4_checkpoint_observation() -> Wait4CheckpointObservation {
    let obs = &WAIT4_CHECKPOINT_OBSERVATION;
    Wait4CheckpointObservation {
        saved: obs.saved.load(Ordering::Acquire),
        saved_sepc: obs.saved_sepc.load(Ordering::Acquire),
        saved_sp: obs.saved_sp.load(Ordering::Acquire),
        saved_s2: obs.saved_s2.load(Ordering::Acquire),
        saved_s4: obs.saved_s4.load(Ordering::Acquire),
        saved_a7: obs.saved_a7.load(Ordering::Acquire),
        saved_satp: obs.saved_satp.load(Ordering::Acquire),
        saved_status_ptr: obs.saved_status_ptr.load(Ordering::Acquire),
        saved_child_pid: obs.saved_child_pid.load(Ordering::Acquire),
        stack_window_saved: obs.stack_window_saved.load(Ordering::Acquire),
        stack_window_start: obs.stack_window_start.load(Ordering::Acquire),
        stack_window_len: obs.stack_window_len.load(Ordering::Acquire),
        writable_pages_saved: obs.writable_pages_saved.load(Ordering::Acquire),
        writable_pages_count: obs.writable_pages_count.load(Ordering::Acquire),
        writable_pages_truncated: obs.writable_pages_truncated.load(Ordering::Acquire),
        writable_pages_restored: obs.writable_pages_restored.load(Ordering::Acquire),
        resumed: obs.resumed.load(Ordering::Acquire),
        resumed_sepc: obs.resumed_sepc.load(Ordering::Acquire),
        resumed_sp: obs.resumed_sp.load(Ordering::Acquire),
        resumed_s2: obs.resumed_s2.load(Ordering::Acquire),
        resumed_s4: obs.resumed_s4.load(Ordering::Acquire),
        resumed_a0: obs.resumed_a0.load(Ordering::Acquire),
        resumed_a7: obs.resumed_a7.load(Ordering::Acquire),
        resumed_satp: obs.resumed_satp.load(Ordering::Acquire),
        resumed_status_ptr: obs.resumed_status_ptr.load(Ordering::Acquire),
        resumed_wait_status: obs.resumed_wait_status.load(Ordering::Acquire),
        resumed_status_copied: obs.resumed_status_copied.load(Ordering::Acquire),
        child_exit_status: obs.child_exit_status.load(Ordering::Acquire),
        stack_window_compared: obs.stack_window_compared.load(Ordering::Acquire),
        stack_window_diff_count: obs.stack_window_diff_count.load(Ordering::Acquire),
        stack_window_first_diff_addr: obs.stack_window_first_diff_addr.load(Ordering::Acquire),
        stack_window_before_byte: obs.stack_window_before_byte.load(Ordering::Acquire),
        stack_window_after_byte: obs.stack_window_after_byte.load(Ordering::Acquire),
        writable_pages_compared: obs.writable_pages_compared.load(Ordering::Acquire),
        writable_pages_dirty_count: obs.writable_pages_dirty_count.load(Ordering::Acquire),
        writable_pages_stack_dirty_count: obs
            .writable_pages_stack_dirty_count
            .load(Ordering::Acquire),
        writable_pages_non_stack_dirty_count: obs
            .writable_pages_non_stack_dirty_count
            .load(Ordering::Acquire),
        writable_pages_first_non_stack_kind: obs
            .writable_pages_first_non_stack_kind
            .load(Ordering::Acquire),
        writable_pages_first_non_stack_mapping_index: obs
            .writable_pages_first_non_stack_mapping_index
            .load(Ordering::Acquire),
        writable_pages_first_non_stack_page_index: obs
            .writable_pages_first_non_stack_page_index
            .load(Ordering::Acquire),
        writable_pages_first_non_stack_addr: obs
            .writable_pages_first_non_stack_addr
            .load(Ordering::Acquire),
        writable_pages_first_non_stack_before_checksum: obs
            .writable_pages_first_non_stack_before_checksum
            .load(Ordering::Acquire),
        writable_pages_first_non_stack_after_checksum: obs
            .writable_pages_first_non_stack_after_checksum
            .load(Ordering::Acquire),
    }
}

static SYSCALL_TABLE_READY: AtomicU8 = AtomicU8::new(0);

pub struct SyscallTable {
    lifecycle: Lifecycle,
    #[allow(dead_code)]
    bound_to_exception: bool,
    write_supported: bool,
    writev_supported: bool,
    openat_supported: bool,
    chdir_supported: bool,
    getdents64_supported: bool,
    read_supported: bool,
    ppoll_supported: bool,
    close_supported: bool,
    newfstatat_supported: bool,
    readlinkat_supported: bool,
    dup3_supported: bool,
    getrandom_supported: bool,
    getcwd_supported: bool,
    getpid_supported: bool,
    getpgid_supported: bool,
    getsid_supported: bool,
    setpgid_supported: bool,
    setsid_supported: bool,
    getppid_supported: bool,
    getuid_supported: bool,
    geteuid_supported: bool,
    getgid_supported: bool,
    getegid_supported: bool,
    getresuid_supported: bool,
    getresgid_supported: bool,
    uname_supported: bool,
    setuid_supported: bool,
    setgid_supported: bool,
    rt_sigprocmask_supported: bool,
    rt_sigaction_supported: bool,
    rt_sigtimedwait_supported: bool,
    clock_gettime_supported: bool,
    gettimeofday_supported: bool,
    nanosleep_supported: bool,
    fstat_supported: bool,
    fcntl_supported: bool,
    ioctl_supported: bool,
    faccessat_supported: bool,
    lseek_supported: bool,
    brk_supported: bool,
    mmap_supported: bool,
    mprotect_supported: bool,
    munmap_supported: bool,
    set_tid_address_supported: bool,
    clone_supported: bool,
    execve_supported: bool,
    wait4_supported: bool,
    exit_supported: bool,
    exit_group_supported: bool,
    write_usercopy_ready: bool,
    writev_usercopy_ready: bool,
    read_usercopy_ready: bool,
    ppoll_usercopy_ready: bool,
    getdents64_usercopy_ready: bool,
    path_usercopy_ready: bool,
    stat_usercopy_ready: bool,
    getrandom_usercopy_ready: bool,
    signal_mask_usercopy_ready: bool,
    signal_action_usercopy_ready: bool,
    time_usercopy_ready: bool,
    nanosleep_usercopy_ready: bool,
    credentials_usercopy_ready: bool,
    utsname_usercopy_ready: bool,
    getcwd_usercopy_ready: bool,
    ioctl_usercopy_ready: bool,
    write_routes_to_console: bool,
    writev_routes_to_files_struct: bool,
    openat_routes_to_files_struct: bool,
    chdir_routes_to_fs_struct: bool,
    chdir_linux_6_12_path_walk_bound: bool,
    chdir_root_first_slice: bool,
    chdir_permissions_lsm_deferred: bool,
    getdents64_routes_to_files_struct: bool,
    read_routes_to_files_struct: bool,
    ppoll_routes_to_files_struct: bool,
    ppoll_ready_data_first_slice: bool,
    ppoll_timeout_parse_first_slice: bool,
    ppoll_sigmask_deferred: bool,
    ppoll_blocking_wait_deferred: bool,
    close_routes_to_files_struct: bool,
    newfstatat_routes_to_files_struct: bool,
    readlinkat_routes_to_files_struct: bool,
    dup3_routes_to_files_struct: bool,
    getrandom_routes_to_hwrng_core: bool,
    getrandom_not_vfs_or_devfs_path: bool,
    getrandom_flags_first_slice_bound: bool,
    getrandom_full_random_core_deferred: bool,
    getcwd_routes_to_user_init_process: bool,
    getcwd_root_first_slice_bound: bool,
    getpid_routes_to_user_init_process: bool,
    getpgid_routes_to_user_init_process: bool,
    getsid_routes_to_user_init_process: bool,
    setpgid_routes_to_user_init_process: bool,
    setsid_routes_to_user_init_process: bool,
    setsid_process_group_leader_eperm_first_slice: bool,
    getppid_routes_to_user_init_process: bool,
    getuid_routes_to_user_init_process: bool,
    geteuid_routes_to_user_init_process: bool,
    getgid_routes_to_user_init_process: bool,
    getegid_routes_to_user_init_process: bool,
    getresuid_routes_to_user_init_process: bool,
    getresgid_routes_to_user_init_process: bool,
    uname_new_utsname_layout_bound: bool,
    uname_static_init_uts_namespace_first_slice: bool,
    uname_full_uts_namespace_deferred: bool,
    setuid_routes_to_user_init_process: bool,
    setgid_routes_to_user_init_process: bool,
    credentials_full_linux_model_deferred: bool,
    rt_sigprocmask_routes_to_user_init_process: bool,
    rt_sigprocmask_sigsetsize_bound: bool,
    rt_sigprocmask_unblockable_signals_cleared: bool,
    rt_sigaction_routes_to_user_init_process: bool,
    rt_sigaction_routes_to_signal_action_table: bool,
    rt_sigaction_sigsetsize_bound: bool,
    rt_sigaction_layout_bound: bool,
    rt_sigaction_unblockable_signals_cleared: bool,
    rt_sigaction_kernel_only_signals_rejected: bool,
    rt_sigtimedwait_routes_to_user_init_process: bool,
    rt_sigtimedwait_sigsetsize_bound: bool,
    rt_sigtimedwait_copies_wait_mask: bool,
    rt_sigtimedwait_uinfo_null_no_copyout_first_slice: bool,
    rt_sigtimedwait_uts_null_infinite_wait_first_slice: bool,
    rt_sigtimedwait_empty_pending_wait_boundary: bool,
    rt_sigtimedwait_waitqueue_sleep_first_slice: bool,
    rt_sigtimedwait_sigchld_pending_first_slice: bool,
    rt_sigtimedwait_return_signal_first_slice: bool,
    signal_delivery_deferred: bool,
    clock_gettime_routes_to_timer_provider: bool,
    gettimeofday_routes_to_timer_provider: bool,
    nanosleep_routes_to_timer_provider: bool,
    clock_gettime_clockid_first_slice_bound: bool,
    time_struct_layout_bound: bool,
    nanosleep_timespec_validated: bool,
    nanosleep_short_relative_first_slice: bool,
    nanosleep_full_hrtimer_deferred: bool,
    time_full_linux_model_deferred: bool,
    fstat_routes_to_files_struct: bool,
    fcntl_routes_to_files_struct: bool,
    ioctl_routes_to_files_struct: bool,
    faccessat_routes_to_files_struct: bool,
    lseek_routes_to_files_struct: bool,
    clone_routes_to_task_creation_core: bool,
    clone_routes_to_user_clone_deferred_boundaries: bool,
    clone_plain_fork_first_slice: bool,
    clone_vfork_vm_first_slice: bool,
    clone_vfork_pidfd_first_slice: bool,
    clone_legacy_pidfd_parent_tidptr_bound: bool,
    clone_vfork_parent_frame_saved: bool,
    clone_pidfd_copyout_first_slice: bool,
    clone_full_vfork_scheduler_deferred: bool,
    clone_full_pidfd_file_ops_deferred: bool,
    execve_child_continuation_first_slice: bool,
    execve_reuses_user_boot_payload_elf_loader: bool,
    execve_replaces_user_address_space_first_slice: bool,
    execve_context_staging_address_space_bound: bool,
    execve_sets_start_thread_frame_first_slice: bool,
    execve_bounded_argv_first_slice: bool,
    execve_envp_full_copy_deferred: bool,
    execve_close_on_exec_deferred: bool,
    execve_old_mm_reclaim_deferred: bool,
    execve_full_linux_model_deferred: bool,
    wait4_parent_wait_chldexit_boundary: bool,
    wait4_yields_to_user_child_continuation: bool,
    wait4_child_exit_status_copyout_first_slice: bool,
    wait4_observed_child_reap_first_slice: bool,
    wait4_no_child_echild_first_slice: bool,
    wait4_blocking_sleep_deferred: bool,
    exit_records_status: bool,
    exit_group_pid1_shutdown_child_wait4_split: bool,
    write_observed: AtomicU8,
    writev_observed: AtomicU8,
    openat_observed: AtomicU8,
    chdir_observed: AtomicU8,
    getdents64_observed: AtomicU8,
    read_observed: AtomicU8,
    ppoll_observed: AtomicU8,
    close_observed: AtomicU8,
    newfstatat_observed: AtomicU8,
    readlinkat_observed: AtomicU8,
    dup3_observed: AtomicU8,
    getrandom_observed: AtomicU8,
    getcwd_observed: AtomicU8,
    getpid_observed: AtomicU8,
    getpgid_observed: AtomicU8,
    getsid_observed: AtomicU8,
    setpgid_observed: AtomicU8,
    setsid_observed: AtomicU8,
    getppid_observed: AtomicU8,
    getuid_observed: AtomicU8,
    geteuid_observed: AtomicU8,
    getgid_observed: AtomicU8,
    getegid_observed: AtomicU8,
    getresuid_observed: AtomicU8,
    getresgid_observed: AtomicU8,
    uname_observed: AtomicU8,
    setuid_observed: AtomicU8,
    setgid_observed: AtomicU8,
    rt_sigprocmask_observed: AtomicU8,
    rt_sigaction_observed: AtomicU8,
    rt_sigtimedwait_observed: AtomicU8,
    clock_gettime_observed: AtomicU8,
    gettimeofday_observed: AtomicU8,
    nanosleep_observed: AtomicU8,
    fstat_observed: AtomicU8,
    fcntl_observed: AtomicU8,
    ioctl_observed: AtomicU8,
    faccessat_observed: AtomicU8,
    lseek_observed: AtomicU8,
    set_tid_address_observed: AtomicU8,
    clone_observed: AtomicU8,
    execve_observed: AtomicU8,
    wait4_observed: AtomicU8,
    exit_observed: AtomicU8,
}

impl SyscallTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            bound_to_exception: false,
            write_supported: false,
            writev_supported: false,
            openat_supported: false,
            chdir_supported: false,
            getdents64_supported: false,
            read_supported: false,
            ppoll_supported: false,
            close_supported: false,
            newfstatat_supported: false,
            readlinkat_supported: false,
            dup3_supported: false,
            getrandom_supported: false,
            getcwd_supported: false,
            getpid_supported: false,
            getpgid_supported: false,
            getsid_supported: false,
            setpgid_supported: false,
            setsid_supported: false,
            getppid_supported: false,
            getuid_supported: false,
            geteuid_supported: false,
            getgid_supported: false,
            getegid_supported: false,
            getresuid_supported: false,
            getresgid_supported: false,
            uname_supported: false,
            setuid_supported: false,
            setgid_supported: false,
            rt_sigprocmask_supported: false,
            rt_sigaction_supported: false,
            rt_sigtimedwait_supported: false,
            clock_gettime_supported: false,
            gettimeofday_supported: false,
            nanosleep_supported: false,
            fstat_supported: false,
            fcntl_supported: false,
            ioctl_supported: false,
            faccessat_supported: false,
            lseek_supported: false,
            brk_supported: false,
            mmap_supported: false,
            mprotect_supported: false,
            munmap_supported: false,
            set_tid_address_supported: false,
            clone_supported: false,
            execve_supported: false,
            wait4_supported: false,
            exit_supported: false,
            exit_group_supported: false,
            write_usercopy_ready: false,
            writev_usercopy_ready: false,
            read_usercopy_ready: false,
            ppoll_usercopy_ready: false,
            getdents64_usercopy_ready: false,
            path_usercopy_ready: false,
            stat_usercopy_ready: false,
            getrandom_usercopy_ready: false,
            signal_mask_usercopy_ready: false,
            signal_action_usercopy_ready: false,
            time_usercopy_ready: false,
            nanosleep_usercopy_ready: false,
            credentials_usercopy_ready: false,
            utsname_usercopy_ready: false,
            getcwd_usercopy_ready: false,
            ioctl_usercopy_ready: false,
            write_routes_to_console: false,
            writev_routes_to_files_struct: false,
            openat_routes_to_files_struct: false,
            chdir_routes_to_fs_struct: false,
            chdir_linux_6_12_path_walk_bound: false,
            chdir_root_first_slice: false,
            chdir_permissions_lsm_deferred: false,
            getdents64_routes_to_files_struct: false,
            read_routes_to_files_struct: false,
            ppoll_routes_to_files_struct: false,
            ppoll_ready_data_first_slice: false,
            ppoll_timeout_parse_first_slice: false,
            ppoll_sigmask_deferred: false,
            ppoll_blocking_wait_deferred: false,
            close_routes_to_files_struct: false,
            newfstatat_routes_to_files_struct: false,
            readlinkat_routes_to_files_struct: false,
            dup3_routes_to_files_struct: false,
            getrandom_routes_to_hwrng_core: false,
            getrandom_not_vfs_or_devfs_path: false,
            getrandom_flags_first_slice_bound: false,
            getrandom_full_random_core_deferred: false,
            getcwd_routes_to_user_init_process: false,
            getcwd_root_first_slice_bound: false,
            getpid_routes_to_user_init_process: false,
            getpgid_routes_to_user_init_process: false,
            getsid_routes_to_user_init_process: false,
            setpgid_routes_to_user_init_process: false,
            setsid_routes_to_user_init_process: false,
            setsid_process_group_leader_eperm_first_slice: false,
            getppid_routes_to_user_init_process: false,
            getuid_routes_to_user_init_process: false,
            geteuid_routes_to_user_init_process: false,
            getgid_routes_to_user_init_process: false,
            getegid_routes_to_user_init_process: false,
            getresuid_routes_to_user_init_process: false,
            getresgid_routes_to_user_init_process: false,
            uname_new_utsname_layout_bound: false,
            uname_static_init_uts_namespace_first_slice: false,
            uname_full_uts_namespace_deferred: false,
            setuid_routes_to_user_init_process: false,
            setgid_routes_to_user_init_process: false,
            credentials_full_linux_model_deferred: false,
            rt_sigprocmask_routes_to_user_init_process: false,
            rt_sigprocmask_sigsetsize_bound: false,
            rt_sigprocmask_unblockable_signals_cleared: false,
            rt_sigaction_routes_to_user_init_process: false,
            rt_sigaction_routes_to_signal_action_table: false,
            rt_sigaction_sigsetsize_bound: false,
            rt_sigaction_layout_bound: false,
            rt_sigaction_unblockable_signals_cleared: false,
            rt_sigaction_kernel_only_signals_rejected: false,
            rt_sigtimedwait_routes_to_user_init_process: false,
            rt_sigtimedwait_sigsetsize_bound: false,
            rt_sigtimedwait_copies_wait_mask: false,
            rt_sigtimedwait_uinfo_null_no_copyout_first_slice: false,
            rt_sigtimedwait_uts_null_infinite_wait_first_slice: false,
            rt_sigtimedwait_empty_pending_wait_boundary: false,
            rt_sigtimedwait_waitqueue_sleep_first_slice: false,
            rt_sigtimedwait_sigchld_pending_first_slice: false,
            rt_sigtimedwait_return_signal_first_slice: false,
            signal_delivery_deferred: false,
            clock_gettime_routes_to_timer_provider: false,
            gettimeofday_routes_to_timer_provider: false,
            nanosleep_routes_to_timer_provider: false,
            clock_gettime_clockid_first_slice_bound: false,
            time_struct_layout_bound: false,
            nanosleep_timespec_validated: false,
            nanosleep_short_relative_first_slice: false,
            nanosleep_full_hrtimer_deferred: false,
            time_full_linux_model_deferred: false,
            fstat_routes_to_files_struct: false,
            fcntl_routes_to_files_struct: false,
            ioctl_routes_to_files_struct: false,
            faccessat_routes_to_files_struct: false,
            lseek_routes_to_files_struct: false,
            clone_routes_to_task_creation_core: false,
            clone_routes_to_user_clone_deferred_boundaries: false,
            clone_plain_fork_first_slice: false,
            clone_vfork_vm_first_slice: false,
            clone_vfork_pidfd_first_slice: false,
            clone_legacy_pidfd_parent_tidptr_bound: false,
            clone_vfork_parent_frame_saved: false,
            clone_pidfd_copyout_first_slice: false,
            clone_full_vfork_scheduler_deferred: false,
            clone_full_pidfd_file_ops_deferred: false,
            execve_child_continuation_first_slice: false,
            execve_reuses_user_boot_payload_elf_loader: false,
            execve_replaces_user_address_space_first_slice: false,
            execve_context_staging_address_space_bound: false,
            execve_sets_start_thread_frame_first_slice: false,
            execve_bounded_argv_first_slice: false,
            execve_envp_full_copy_deferred: false,
            execve_close_on_exec_deferred: false,
            execve_old_mm_reclaim_deferred: false,
            execve_full_linux_model_deferred: false,
            wait4_parent_wait_chldexit_boundary: false,
            wait4_yields_to_user_child_continuation: false,
            wait4_child_exit_status_copyout_first_slice: false,
            wait4_observed_child_reap_first_slice: false,
            wait4_no_child_echild_first_slice: false,
            wait4_blocking_sleep_deferred: false,
            exit_records_status: false,
            exit_group_pid1_shutdown_child_wait4_split: false,
            write_observed: AtomicU8::new(0),
            writev_observed: AtomicU8::new(0),
            openat_observed: AtomicU8::new(0),
            chdir_observed: AtomicU8::new(0),
            getdents64_observed: AtomicU8::new(0),
            read_observed: AtomicU8::new(0),
            ppoll_observed: AtomicU8::new(0),
            close_observed: AtomicU8::new(0),
            newfstatat_observed: AtomicU8::new(0),
            readlinkat_observed: AtomicU8::new(0),
            dup3_observed: AtomicU8::new(0),
            getrandom_observed: AtomicU8::new(0),
            getcwd_observed: AtomicU8::new(0),
            getpid_observed: AtomicU8::new(0),
            getpgid_observed: AtomicU8::new(0),
            getsid_observed: AtomicU8::new(0),
            setpgid_observed: AtomicU8::new(0),
            setsid_observed: AtomicU8::new(0),
            getppid_observed: AtomicU8::new(0),
            getuid_observed: AtomicU8::new(0),
            geteuid_observed: AtomicU8::new(0),
            getgid_observed: AtomicU8::new(0),
            getegid_observed: AtomicU8::new(0),
            getresuid_observed: AtomicU8::new(0),
            getresgid_observed: AtomicU8::new(0),
            uname_observed: AtomicU8::new(0),
            setuid_observed: AtomicU8::new(0),
            setgid_observed: AtomicU8::new(0),
            rt_sigprocmask_observed: AtomicU8::new(0),
            rt_sigaction_observed: AtomicU8::new(0),
            rt_sigtimedwait_observed: AtomicU8::new(0),
            clock_gettime_observed: AtomicU8::new(0),
            gettimeofday_observed: AtomicU8::new(0),
            nanosleep_observed: AtomicU8::new(0),
            fstat_observed: AtomicU8::new(0),
            fcntl_observed: AtomicU8::new(0),
            ioctl_observed: AtomicU8::new(0),
            faccessat_observed: AtomicU8::new(0),
            lseek_observed: AtomicU8::new(0),
            set_tid_address_observed: AtomicU8::new(0),
            clone_observed: AtomicU8::new(0),
            execve_observed: AtomicU8::new(0),
            wait4_observed: AtomicU8::new(0),
            exit_observed: AtomicU8::new(0),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn bound_to_exception(&self) -> bool {
        self.bound_to_exception
    }

    #[allow(dead_code)]
    pub const fn write_supported(&self) -> bool {
        self.write_supported
    }

    #[allow(dead_code)]
    pub const fn writev_supported(&self) -> bool {
        self.writev_supported
    }

    #[allow(dead_code)]
    pub const fn openat_supported(&self) -> bool {
        self.openat_supported
    }

    #[allow(dead_code)]
    pub const fn chdir_supported(&self) -> bool {
        self.chdir_supported
    }

    #[allow(dead_code)]
    pub const fn getdents64_supported(&self) -> bool {
        self.getdents64_supported
    }

    #[allow(dead_code)]
    pub const fn read_supported(&self) -> bool {
        self.read_supported
    }

    #[allow(dead_code)]
    pub const fn ppoll_supported(&self) -> bool {
        self.ppoll_supported
    }

    #[allow(dead_code)]
    pub const fn close_supported(&self) -> bool {
        self.close_supported
    }

    #[allow(dead_code)]
    pub const fn newfstatat_supported(&self) -> bool {
        self.newfstatat_supported
    }

    #[allow(dead_code)]
    pub const fn readlinkat_supported(&self) -> bool {
        self.readlinkat_supported
    }

    #[allow(dead_code)]
    pub const fn dup3_supported(&self) -> bool {
        self.dup3_supported
    }

    #[allow(dead_code)]
    pub const fn getrandom_supported(&self) -> bool {
        self.getrandom_supported
    }

    #[allow(dead_code)]
    pub const fn getuid_supported(&self) -> bool {
        self.getuid_supported
    }

    #[allow(dead_code)]
    pub const fn getgid_supported(&self) -> bool {
        self.getgid_supported
    }

    #[allow(dead_code)]
    pub const fn getsid_supported(&self) -> bool {
        self.getsid_supported
    }

    #[allow(dead_code)]
    pub const fn setsid_supported(&self) -> bool {
        self.setsid_supported
    }

    #[allow(dead_code)]
    pub const fn setuid_supported(&self) -> bool {
        self.setuid_supported
    }

    #[allow(dead_code)]
    pub const fn setgid_supported(&self) -> bool {
        self.setgid_supported
    }

    #[allow(dead_code)]
    pub const fn rt_sigprocmask_supported(&self) -> bool {
        self.rt_sigprocmask_supported
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_supported(&self) -> bool {
        self.rt_sigaction_supported
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_supported(&self) -> bool {
        self.rt_sigtimedwait_supported
    }

    #[allow(dead_code)]
    pub const fn clock_gettime_supported(&self) -> bool {
        self.clock_gettime_supported
    }

    #[allow(dead_code)]
    pub const fn gettimeofday_supported(&self) -> bool {
        self.gettimeofday_supported
    }

    #[allow(dead_code)]
    pub const fn nanosleep_supported(&self) -> bool {
        self.nanosleep_supported
    }

    #[allow(dead_code)]
    pub const fn fstat_supported(&self) -> bool {
        self.fstat_supported
    }

    #[allow(dead_code)]
    pub const fn fcntl_supported(&self) -> bool {
        self.fcntl_supported
    }

    #[allow(dead_code)]
    pub const fn ioctl_supported(&self) -> bool {
        self.ioctl_supported
    }

    #[allow(dead_code)]
    pub const fn faccessat_supported(&self) -> bool {
        self.faccessat_supported
    }

    #[allow(dead_code)]
    pub const fn lseek_supported(&self) -> bool {
        self.lseek_supported
    }

    #[allow(dead_code)]
    pub const fn brk_supported(&self) -> bool {
        self.brk_supported
    }

    #[allow(dead_code)]
    pub const fn mmap_supported(&self) -> bool {
        self.mmap_supported
    }

    #[allow(dead_code)]
    pub const fn mprotect_supported(&self) -> bool {
        self.mprotect_supported
    }

    #[allow(dead_code)]
    pub const fn munmap_supported(&self) -> bool {
        self.munmap_supported
    }

    #[allow(dead_code)]
    pub const fn set_tid_address_supported(&self) -> bool {
        self.set_tid_address_supported
    }

    #[allow(dead_code)]
    pub const fn clone_supported(&self) -> bool {
        self.clone_supported
    }

    #[allow(dead_code)]
    pub const fn clone_routes_to_task_creation_core(&self) -> bool {
        self.clone_routes_to_task_creation_core
    }

    #[allow(dead_code)]
    pub const fn clone_routes_to_user_clone_deferred_boundaries(&self) -> bool {
        self.clone_routes_to_user_clone_deferred_boundaries
    }

    #[allow(dead_code)]
    pub const fn clone_plain_fork_first_slice(&self) -> bool {
        self.clone_plain_fork_first_slice
    }

    #[allow(dead_code)]
    pub const fn clone_vfork_vm_first_slice(&self) -> bool {
        self.clone_vfork_vm_first_slice
    }

    #[allow(dead_code)]
    pub const fn clone_vfork_pidfd_first_slice(&self) -> bool {
        self.clone_vfork_pidfd_first_slice
    }

    #[allow(dead_code)]
    pub const fn clone_legacy_pidfd_parent_tidptr_bound(&self) -> bool {
        self.clone_legacy_pidfd_parent_tidptr_bound
    }

    #[allow(dead_code)]
    pub const fn clone_vfork_parent_frame_saved(&self) -> bool {
        self.clone_vfork_parent_frame_saved
    }

    #[allow(dead_code)]
    pub const fn clone_pidfd_copyout_first_slice(&self) -> bool {
        self.clone_pidfd_copyout_first_slice
    }

    #[allow(dead_code)]
    pub const fn clone_full_vfork_scheduler_deferred(&self) -> bool {
        self.clone_full_vfork_scheduler_deferred
    }

    #[allow(dead_code)]
    pub const fn clone_full_pidfd_file_ops_deferred(&self) -> bool {
        self.clone_full_pidfd_file_ops_deferred
    }

    #[allow(dead_code)]
    pub const fn wait4_supported(&self) -> bool {
        self.wait4_supported
    }

    #[allow(dead_code)]
    pub const fn exit_supported(&self) -> bool {
        self.exit_supported
    }

    #[allow(dead_code)]
    pub const fn exit_group_supported(&self) -> bool {
        self.exit_group_supported
    }

    #[allow(dead_code)]
    pub const fn write_usercopy_ready(&self) -> bool {
        self.write_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn writev_usercopy_ready(&self) -> bool {
        self.writev_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn read_usercopy_ready(&self) -> bool {
        self.read_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn ppoll_usercopy_ready(&self) -> bool {
        self.ppoll_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn getdents64_usercopy_ready(&self) -> bool {
        self.getdents64_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn path_usercopy_ready(&self) -> bool {
        self.path_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn stat_usercopy_ready(&self) -> bool {
        self.stat_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn getrandom_usercopy_ready(&self) -> bool {
        self.getrandom_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn signal_mask_usercopy_ready(&self) -> bool {
        self.signal_mask_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn signal_action_usercopy_ready(&self) -> bool {
        self.signal_action_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn time_usercopy_ready(&self) -> bool {
        self.time_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn nanosleep_usercopy_ready(&self) -> bool {
        self.nanosleep_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn ioctl_usercopy_ready(&self) -> bool {
        self.ioctl_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn write_routes_to_console(&self) -> bool {
        self.write_routes_to_console
    }

    #[allow(dead_code)]
    pub const fn writev_routes_to_files_struct(&self) -> bool {
        self.writev_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn openat_routes_to_files_struct(&self) -> bool {
        self.openat_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn getdents64_routes_to_files_struct(&self) -> bool {
        self.getdents64_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn read_routes_to_files_struct(&self) -> bool {
        self.read_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn ppoll_routes_to_files_struct(&self) -> bool {
        self.ppoll_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn close_routes_to_files_struct(&self) -> bool {
        self.close_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn newfstatat_routes_to_files_struct(&self) -> bool {
        self.newfstatat_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn readlinkat_routes_to_files_struct(&self) -> bool {
        self.readlinkat_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn dup3_routes_to_files_struct(&self) -> bool {
        self.dup3_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn getrandom_routes_to_hwrng_core(&self) -> bool {
        self.getrandom_routes_to_hwrng_core
    }

    #[allow(dead_code)]
    pub const fn getrandom_not_vfs_or_devfs_path(&self) -> bool {
        self.getrandom_not_vfs_or_devfs_path
    }

    #[allow(dead_code)]
    pub const fn getrandom_flags_first_slice_bound(&self) -> bool {
        self.getrandom_flags_first_slice_bound
    }

    #[allow(dead_code)]
    pub const fn getrandom_full_random_core_deferred(&self) -> bool {
        self.getrandom_full_random_core_deferred
    }

    #[allow(dead_code)]
    pub const fn getuid_routes_to_user_init_process(&self) -> bool {
        self.getuid_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn getgid_routes_to_user_init_process(&self) -> bool {
        self.getgid_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn setuid_routes_to_user_init_process(&self) -> bool {
        self.setuid_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn setgid_routes_to_user_init_process(&self) -> bool {
        self.setgid_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn credentials_full_linux_model_deferred(&self) -> bool {
        self.credentials_full_linux_model_deferred
    }

    #[allow(dead_code)]
    pub const fn rt_sigprocmask_routes_to_user_init_process(&self) -> bool {
        self.rt_sigprocmask_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn rt_sigprocmask_sigsetsize_bound(&self) -> bool {
        self.rt_sigprocmask_sigsetsize_bound
    }

    #[allow(dead_code)]
    pub const fn rt_sigprocmask_unblockable_signals_cleared(&self) -> bool {
        self.rt_sigprocmask_unblockable_signals_cleared
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_routes_to_user_init_process(&self) -> bool {
        self.rt_sigaction_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_routes_to_signal_action_table(&self) -> bool {
        self.rt_sigaction_routes_to_signal_action_table
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_sigsetsize_bound(&self) -> bool {
        self.rt_sigaction_sigsetsize_bound
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_layout_bound(&self) -> bool {
        self.rt_sigaction_layout_bound
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_unblockable_signals_cleared(&self) -> bool {
        self.rt_sigaction_unblockable_signals_cleared
    }

    #[allow(dead_code)]
    pub const fn rt_sigaction_kernel_only_signals_rejected(&self) -> bool {
        self.rt_sigaction_kernel_only_signals_rejected
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_routes_to_user_init_process(&self) -> bool {
        self.rt_sigtimedwait_routes_to_user_init_process
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_sigsetsize_bound(&self) -> bool {
        self.rt_sigtimedwait_sigsetsize_bound
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_copies_wait_mask(&self) -> bool {
        self.rt_sigtimedwait_copies_wait_mask
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_uinfo_null_no_copyout_first_slice(&self) -> bool {
        self.rt_sigtimedwait_uinfo_null_no_copyout_first_slice
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_uts_null_infinite_wait_first_slice(&self) -> bool {
        self.rt_sigtimedwait_uts_null_infinite_wait_first_slice
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_empty_pending_wait_boundary(&self) -> bool {
        self.rt_sigtimedwait_empty_pending_wait_boundary
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_waitqueue_sleep_first_slice(&self) -> bool {
        self.rt_sigtimedwait_waitqueue_sleep_first_slice
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_sigchld_pending_first_slice(&self) -> bool {
        self.rt_sigtimedwait_sigchld_pending_first_slice
    }

    #[allow(dead_code)]
    pub const fn rt_sigtimedwait_return_signal_first_slice(&self) -> bool {
        self.rt_sigtimedwait_return_signal_first_slice
    }

    #[allow(dead_code)]
    pub const fn signal_delivery_deferred(&self) -> bool {
        self.signal_delivery_deferred
    }

    #[allow(dead_code)]
    pub const fn clock_gettime_routes_to_timer_provider(&self) -> bool {
        self.clock_gettime_routes_to_timer_provider
    }

    #[allow(dead_code)]
    pub const fn gettimeofday_routes_to_timer_provider(&self) -> bool {
        self.gettimeofday_routes_to_timer_provider
    }

    #[allow(dead_code)]
    pub const fn nanosleep_routes_to_timer_provider(&self) -> bool {
        self.nanosleep_routes_to_timer_provider
    }

    #[allow(dead_code)]
    pub const fn clock_gettime_clockid_first_slice_bound(&self) -> bool {
        self.clock_gettime_clockid_first_slice_bound
    }

    #[allow(dead_code)]
    pub const fn time_struct_layout_bound(&self) -> bool {
        self.time_struct_layout_bound
    }

    #[allow(dead_code)]
    pub const fn nanosleep_timespec_validated(&self) -> bool {
        self.nanosleep_timespec_validated
    }

    #[allow(dead_code)]
    pub const fn nanosleep_short_relative_first_slice(&self) -> bool {
        self.nanosleep_short_relative_first_slice
    }

    #[allow(dead_code)]
    pub const fn nanosleep_full_hrtimer_deferred(&self) -> bool {
        self.nanosleep_full_hrtimer_deferred
    }

    #[allow(dead_code)]
    pub const fn time_full_linux_model_deferred(&self) -> bool {
        self.time_full_linux_model_deferred
    }

    #[allow(dead_code)]
    pub const fn fstat_routes_to_files_struct(&self) -> bool {
        self.fstat_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn fcntl_routes_to_files_struct(&self) -> bool {
        self.fcntl_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn ioctl_routes_to_files_struct(&self) -> bool {
        self.ioctl_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn faccessat_routes_to_files_struct(&self) -> bool {
        self.faccessat_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn lseek_routes_to_files_struct(&self) -> bool {
        self.lseek_routes_to_files_struct
    }

    #[allow(dead_code)]
    pub const fn exit_records_status(&self) -> bool {
        self.exit_records_status
    }

    #[allow(dead_code)]
    pub const fn wait4_parent_wait_chldexit_boundary(&self) -> bool {
        self.wait4_parent_wait_chldexit_boundary
    }

    #[allow(dead_code)]
    pub const fn wait4_yields_to_user_child_continuation(&self) -> bool {
        self.wait4_yields_to_user_child_continuation
    }

    #[allow(dead_code)]
    pub const fn wait4_child_exit_status_copyout_first_slice(&self) -> bool {
        self.wait4_child_exit_status_copyout_first_slice
    }

    #[allow(dead_code)]
    pub const fn wait4_observed_child_reap_first_slice(&self) -> bool {
        self.wait4_observed_child_reap_first_slice
    }

    #[allow(dead_code)]
    pub const fn wait4_no_child_echild_first_slice(&self) -> bool {
        self.wait4_no_child_echild_first_slice
    }

    #[allow(dead_code)]
    pub const fn wait4_blocking_sleep_deferred(&self) -> bool {
        self.wait4_blocking_sleep_deferred
    }

    #[allow(dead_code)]
    pub const fn exit_group_pid1_shutdown_child_wait4_split(&self) -> bool {
        self.exit_group_pid1_shutdown_child_wait4_split
    }

    #[allow(dead_code)]
    pub fn write_observed(&self) -> bool {
        self.write_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn writev_observed(&self) -> bool {
        self.writev_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn openat_observed(&self) -> bool {
        self.openat_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn chdir_observed(&self) -> bool {
        self.chdir_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn getdents64_observed(&self) -> bool {
        self.getdents64_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn read_observed(&self) -> bool {
        self.read_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn ppoll_observed(&self) -> bool {
        self.ppoll_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn close_observed(&self) -> bool {
        self.close_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn newfstatat_observed(&self) -> bool {
        self.newfstatat_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn readlinkat_observed(&self) -> bool {
        self.readlinkat_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn dup3_observed(&self) -> bool {
        self.dup3_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn getrandom_observed(&self) -> bool {
        self.getrandom_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn setpgid_observed(&self) -> bool {
        self.setpgid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn getsid_observed(&self) -> bool {
        self.getsid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn setsid_observed(&self) -> bool {
        self.setsid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn getuid_observed(&self) -> bool {
        self.getuid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn getgid_observed(&self) -> bool {
        self.getgid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn setuid_observed(&self) -> bool {
        self.setuid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn setgid_observed(&self) -> bool {
        self.setgid_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn rt_sigprocmask_observed(&self) -> bool {
        self.rt_sigprocmask_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn rt_sigaction_observed(&self) -> bool {
        self.rt_sigaction_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn rt_sigtimedwait_observed(&self) -> bool {
        self.rt_sigtimedwait_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn clock_gettime_observed(&self) -> bool {
        self.clock_gettime_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn gettimeofday_observed(&self) -> bool {
        self.gettimeofday_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn nanosleep_observed(&self) -> bool {
        self.nanosleep_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn fstat_observed(&self) -> bool {
        self.fstat_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn fcntl_observed(&self) -> bool {
        self.fcntl_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn ioctl_observed(&self) -> bool {
        self.ioctl_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn faccessat_observed(&self) -> bool {
        self.faccessat_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn lseek_observed(&self) -> bool {
        self.lseek_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn set_tid_address_observed(&self) -> bool {
        self.set_tid_address_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn clone_observed(&self) -> bool {
        self.clone_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn wait4_observed(&self) -> bool {
        self.wait4_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn exit_observed(&self) -> bool {
        self.exit_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn setup(&mut self, syscall_exception_state: State) -> EventResult {
        if self.lifecycle.state() != State::Base || syscall_exception_state != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.bound_to_exception = true;
        self.write_supported = true;
        self.writev_supported = true;
        self.openat_supported = true;
        self.chdir_supported = true;
        self.getdents64_supported = true;
        self.read_supported = true;
        self.ppoll_supported = true;
        self.close_supported = true;
        self.newfstatat_supported = true;
        self.readlinkat_supported = true;
        self.dup3_supported = true;
        self.getrandom_supported = true;
        self.getcwd_supported = true;
        self.getpid_supported = true;
        self.getpgid_supported = true;
        self.getsid_supported = true;
        self.setpgid_supported = true;
        self.setsid_supported = true;
        self.getppid_supported = true;
        self.getuid_supported = true;
        self.geteuid_supported = true;
        self.getgid_supported = true;
        self.getegid_supported = true;
        self.getresuid_supported = true;
        self.getresgid_supported = true;
        self.uname_supported = true;
        self.setuid_supported = true;
        self.setgid_supported = true;
        self.rt_sigprocmask_supported = true;
        self.rt_sigaction_supported = true;
        self.rt_sigtimedwait_supported = true;
        self.clock_gettime_supported = true;
        self.gettimeofday_supported = true;
        self.nanosleep_supported = true;
        self.fstat_supported = true;
        self.fcntl_supported = true;
        self.ioctl_supported = true;
        self.faccessat_supported = true;
        self.lseek_supported = true;
        self.brk_supported = true;
        self.mmap_supported = true;
        self.mprotect_supported = true;
        self.munmap_supported = true;
        self.set_tid_address_supported = true;
        self.clone_supported = true;
        self.execve_supported = true;
        self.wait4_supported = true;
        self.exit_supported = true;
        self.exit_group_supported = true;
        self.write_usercopy_ready = true;
        self.writev_usercopy_ready = true;
        self.read_usercopy_ready = true;
        self.ppoll_usercopy_ready = true;
        self.getdents64_usercopy_ready = true;
        self.path_usercopy_ready = true;
        self.stat_usercopy_ready = true;
        self.getrandom_usercopy_ready = true;
        self.signal_mask_usercopy_ready = true;
        self.signal_action_usercopy_ready = true;
        self.time_usercopy_ready = true;
        self.nanosleep_usercopy_ready = true;
        self.credentials_usercopy_ready = true;
        self.utsname_usercopy_ready = true;
        self.getcwd_usercopy_ready = true;
        self.ioctl_usercopy_ready = true;
        self.write_routes_to_console = true;
        self.writev_routes_to_files_struct = true;
        self.openat_routes_to_files_struct = true;
        self.chdir_routes_to_fs_struct = true;
        self.chdir_linux_6_12_path_walk_bound = true;
        self.chdir_root_first_slice = true;
        self.chdir_permissions_lsm_deferred = true;
        self.getdents64_routes_to_files_struct = true;
        self.read_routes_to_files_struct = true;
        self.ppoll_routes_to_files_struct = true;
        self.ppoll_ready_data_first_slice = true;
        self.ppoll_timeout_parse_first_slice = true;
        self.ppoll_sigmask_deferred = true;
        self.ppoll_blocking_wait_deferred = true;
        self.close_routes_to_files_struct = true;
        self.newfstatat_routes_to_files_struct = true;
        self.readlinkat_routes_to_files_struct = true;
        self.dup3_routes_to_files_struct = true;
        self.getrandom_routes_to_hwrng_core = true;
        self.getrandom_not_vfs_or_devfs_path = true;
        self.getrandom_flags_first_slice_bound = true;
        self.getrandom_full_random_core_deferred = true;
        self.getcwd_routes_to_user_init_process = true;
        self.getcwd_root_first_slice_bound = true;
        self.getpid_routes_to_user_init_process = true;
        self.getpgid_routes_to_user_init_process = true;
        self.getsid_routes_to_user_init_process = true;
        self.setpgid_routes_to_user_init_process = true;
        self.setsid_routes_to_user_init_process = true;
        self.setsid_process_group_leader_eperm_first_slice = true;
        self.getppid_routes_to_user_init_process = true;
        self.getuid_routes_to_user_init_process = true;
        self.geteuid_routes_to_user_init_process = true;
        self.getgid_routes_to_user_init_process = true;
        self.getegid_routes_to_user_init_process = true;
        self.getresuid_routes_to_user_init_process = true;
        self.getresgid_routes_to_user_init_process = true;
        self.uname_new_utsname_layout_bound = true;
        self.uname_static_init_uts_namespace_first_slice = true;
        self.uname_full_uts_namespace_deferred = true;
        self.setuid_routes_to_user_init_process = true;
        self.setgid_routes_to_user_init_process = true;
        self.credentials_full_linux_model_deferred = true;
        self.rt_sigprocmask_routes_to_user_init_process = true;
        self.rt_sigprocmask_sigsetsize_bound = true;
        self.rt_sigprocmask_unblockable_signals_cleared = true;
        self.rt_sigaction_routes_to_user_init_process = true;
        self.rt_sigaction_routes_to_signal_action_table = true;
        self.rt_sigaction_sigsetsize_bound = true;
        self.rt_sigaction_layout_bound = true;
        self.rt_sigaction_unblockable_signals_cleared = true;
        self.rt_sigaction_kernel_only_signals_rejected = true;
        self.rt_sigtimedwait_routes_to_user_init_process = true;
        self.rt_sigtimedwait_sigsetsize_bound = true;
        self.rt_sigtimedwait_copies_wait_mask = true;
        self.rt_sigtimedwait_uinfo_null_no_copyout_first_slice = true;
        self.rt_sigtimedwait_uts_null_infinite_wait_first_slice = true;
        self.rt_sigtimedwait_empty_pending_wait_boundary = true;
        self.rt_sigtimedwait_waitqueue_sleep_first_slice = true;
        self.rt_sigtimedwait_sigchld_pending_first_slice = true;
        self.rt_sigtimedwait_return_signal_first_slice = true;
        self.signal_delivery_deferred = true;
        self.clock_gettime_routes_to_timer_provider = true;
        self.gettimeofday_routes_to_timer_provider = true;
        self.nanosleep_routes_to_timer_provider = true;
        self.clock_gettime_clockid_first_slice_bound = true;
        self.time_struct_layout_bound = true;
        self.nanosleep_timespec_validated = true;
        self.nanosleep_short_relative_first_slice = true;
        self.nanosleep_full_hrtimer_deferred = true;
        self.time_full_linux_model_deferred = true;
        self.fstat_routes_to_files_struct = true;
        self.fcntl_routes_to_files_struct = true;
        self.ioctl_routes_to_files_struct = true;
        self.faccessat_routes_to_files_struct = true;
        self.lseek_routes_to_files_struct = true;
        self.clone_routes_to_task_creation_core = true;
        self.clone_routes_to_user_clone_deferred_boundaries = true;
        self.clone_plain_fork_first_slice = true;
        self.clone_vfork_vm_first_slice = true;
        self.clone_vfork_pidfd_first_slice = true;
        self.clone_legacy_pidfd_parent_tidptr_bound = true;
        self.clone_vfork_parent_frame_saved = true;
        self.clone_pidfd_copyout_first_slice = true;
        self.clone_full_vfork_scheduler_deferred = true;
        self.clone_full_pidfd_file_ops_deferred = true;
        self.execve_child_continuation_first_slice = true;
        self.execve_reuses_user_boot_payload_elf_loader = true;
        self.execve_replaces_user_address_space_first_slice = true;
        self.execve_context_staging_address_space_bound = true;
        self.execve_sets_start_thread_frame_first_slice = true;
        self.execve_bounded_argv_first_slice = true;
        self.execve_envp_full_copy_deferred = true;
        self.execve_close_on_exec_deferred = true;
        self.execve_old_mm_reclaim_deferred = true;
        self.execve_full_linux_model_deferred = true;
        self.wait4_parent_wait_chldexit_boundary = true;
        self.wait4_yields_to_user_child_continuation = true;
        self.wait4_child_exit_status_copyout_first_slice = true;
        self.wait4_observed_child_reap_first_slice = true;
        self.wait4_no_child_echild_first_slice = true;
        self.wait4_blocking_sleep_deferred = true;
        self.exit_records_status = true;
        self.exit_group_pid1_shutdown_child_wait4_split = true;
        SYSCALL_TABLE_READY.store(1, Ordering::Relaxed);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn openat(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.openat_supported
            || !self.path_usercopy_ready
            || !self.openat_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_openat(self, frame);
    }

    pub fn chdir(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.chdir_supported
            || !self.path_usercopy_ready
            || !self.chdir_routes_to_fs_struct
            || !self.chdir_linux_6_12_path_walk_bound
            || !self.chdir_root_first_slice
            || !self.chdir_permissions_lsm_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_chdir(self, frame);
    }

    pub fn read(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.read_supported
            || !self.read_usercopy_ready
            || !self.read_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_read(self, frame);
    }

    pub fn ppoll(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.ppoll_supported
            || !self.ppoll_usercopy_ready
            || !self.ppoll_routes_to_files_struct
            || !self.ppoll_ready_data_first_slice
            || !self.ppoll_timeout_parse_first_slice
            || !self.ppoll_sigmask_deferred
            || !self.ppoll_blocking_wait_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_ppoll(self, frame);
    }

    pub fn close(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.close_supported
            || !self.close_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_close(self, frame);
    }

    pub fn getdents64(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getdents64_supported
            || !self.getdents64_usercopy_ready
            || !self.getdents64_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }
        syscall_table_getdents64(self, frame);
    }

    pub fn brk(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready || !self.brk_supported {
            complete_unsupported_syscall(frame);
            return;
        }
        syscall_table_brk(frame);
    }

    pub fn mmap(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready || !self.mmap_supported {
            complete_unsupported_syscall(frame);
            return;
        }
        syscall_table_mmap(frame);
    }

    pub fn mprotect(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready || !self.mprotect_supported {
            complete_unsupported_syscall(frame);
            return;
        }
        syscall_table_mprotect(frame);
    }

    pub fn munmap(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready || !self.munmap_supported {
            complete_unsupported_syscall(frame);
            return;
        }
        syscall_table_munmap(frame);
    }

    pub fn newfstatat(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.newfstatat_supported
            || !self.path_usercopy_ready
            || !self.stat_usercopy_ready
            || !self.newfstatat_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_newfstatat(self, frame);
    }

    pub fn readlinkat(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.readlinkat_supported
            || !self.path_usercopy_ready
            || !self.read_usercopy_ready
            || !self.readlinkat_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_readlinkat(self, frame);
    }

    pub fn dup3(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.dup3_supported
            || !self.dup3_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_dup3(self, frame);
    }

    pub fn getrandom(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getrandom_supported
            || !self.getrandom_usercopy_ready
            || !self.getrandom_routes_to_hwrng_core
            || !self.getrandom_not_vfs_or_devfs_path
            || !self.getrandom_flags_first_slice_bound
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getrandom(self, frame);
    }

    pub fn getcwd(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getcwd_supported
            || !self.getcwd_usercopy_ready
            || !self.getcwd_routes_to_user_init_process
            || !self.getcwd_root_first_slice_bound
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getcwd(self, frame);
    }

    pub fn getpid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getpid_supported
            || !self.getpid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getpid(self, frame);
    }

    pub fn getpgid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getpgid_supported
            || !self.getpgid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getpgid(self, frame);
    }

    pub fn getsid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getsid_supported
            || !self.getsid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getsid(self, frame);
    }

    pub fn setpgid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.setpgid_supported
            || !self.setpgid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_setpgid(self, frame);
    }

    pub fn setsid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.setsid_supported
            || !self.setsid_routes_to_user_init_process
            || !self.setsid_process_group_leader_eperm_first_slice
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_setsid(self, frame);
    }

    pub fn getppid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getppid_supported
            || !self.getppid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getppid(self, frame);
    }

    pub fn getuid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getuid_supported
            || !self.getuid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getuid(self, frame);
    }

    pub fn geteuid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.geteuid_supported
            || !self.geteuid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_geteuid(self, frame);
    }

    pub fn getgid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getgid_supported
            || !self.getgid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getgid(self, frame);
    }

    pub fn getegid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getegid_supported
            || !self.getegid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getegid(self, frame);
    }

    pub fn getresuid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getresuid_supported
            || !self.credentials_usercopy_ready
            || !self.getresuid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getresuid(self, frame);
    }

    pub fn getresgid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.getresgid_supported
            || !self.credentials_usercopy_ready
            || !self.getresgid_routes_to_user_init_process
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_getresgid(self, frame);
    }

    pub fn uname(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.uname_supported
            || !self.utsname_usercopy_ready
            || !self.uname_new_utsname_layout_bound
            || !self.uname_static_init_uts_namespace_first_slice
            || !self.uname_full_uts_namespace_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_uname(self, frame);
    }

    pub fn setuid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.setuid_supported
            || !self.setuid_routes_to_user_init_process
            || !self.credentials_full_linux_model_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_setuid(self, frame);
    }

    pub fn setgid(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.setgid_supported
            || !self.setgid_routes_to_user_init_process
            || !self.credentials_full_linux_model_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_setgid(self, frame);
    }

    pub fn rt_sigprocmask(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.rt_sigprocmask_supported
            || !self.rt_sigprocmask_routes_to_user_init_process
            || !self.rt_sigprocmask_sigsetsize_bound
            || !self.rt_sigprocmask_unblockable_signals_cleared
            || !self.signal_delivery_deferred
            || !self.signal_mask_usercopy_ready
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_rt_sigprocmask(self, frame);
    }

    pub fn rt_sigaction(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.rt_sigaction_supported
            || !self.rt_sigaction_routes_to_user_init_process
            || !self.rt_sigaction_routes_to_signal_action_table
            || !self.rt_sigaction_sigsetsize_bound
            || !self.rt_sigaction_layout_bound
            || !self.rt_sigaction_unblockable_signals_cleared
            || !self.rt_sigaction_kernel_only_signals_rejected
            || !self.signal_delivery_deferred
            || !self.signal_action_usercopy_ready
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_rt_sigaction(self, frame);
    }

    pub fn rt_sigtimedwait(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.rt_sigtimedwait_supported
            || !self.rt_sigtimedwait_routes_to_user_init_process
            || !self.rt_sigtimedwait_sigsetsize_bound
            || !self.rt_sigtimedwait_copies_wait_mask
            || !self.rt_sigtimedwait_uinfo_null_no_copyout_first_slice
            || !self.rt_sigtimedwait_uts_null_infinite_wait_first_slice
            || !self.rt_sigtimedwait_empty_pending_wait_boundary
            || !self.rt_sigtimedwait_waitqueue_sleep_first_slice
            || !self.rt_sigtimedwait_sigchld_pending_first_slice
            || !self.rt_sigtimedwait_return_signal_first_slice
            || !self.signal_delivery_deferred
            || !self.signal_mask_usercopy_ready
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_rt_sigtimedwait(self, frame);
    }

    pub fn clock_gettime(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.clock_gettime_supported
            || !self.time_usercopy_ready
            || !self.clock_gettime_routes_to_timer_provider
            || !self.clock_gettime_clockid_first_slice_bound
            || !self.time_struct_layout_bound
            || !self.time_full_linux_model_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_clock_gettime(self, frame);
    }

    pub fn gettimeofday(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.gettimeofday_supported
            || !self.time_usercopy_ready
            || !self.gettimeofday_routes_to_timer_provider
            || !self.time_struct_layout_bound
            || !self.time_full_linux_model_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_gettimeofday(self, frame);
    }

    pub fn nanosleep(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.nanosleep_supported
            || !self.nanosleep_usercopy_ready
            || !self.nanosleep_routes_to_timer_provider
            || !self.nanosleep_timespec_validated
            || !self.nanosleep_short_relative_first_slice
            || !self.nanosleep_full_hrtimer_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_nanosleep(self, frame);
    }

    pub fn fstat(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.fstat_supported
            || !self.stat_usercopy_ready
            || !self.fstat_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_fstat(self, frame);
    }

    pub fn fcntl(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.fcntl_supported
            || !self.fcntl_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_fcntl(self, frame);
    }

    pub fn ioctl(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.ioctl_supported
            || !self.ioctl_usercopy_ready
            || !self.ioctl_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_ioctl(self, frame);
    }

    pub fn faccessat(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.faccessat_supported
            || !self.path_usercopy_ready
            || !self.faccessat_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_faccessat(self, frame);
    }

    pub fn lseek(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.lseek_supported
            || !self.lseek_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_lseek(self, frame);
    }

    pub fn write(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.write_supported
            || !self.write_usercopy_ready
            || !self.write_routes_to_console
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_write(self, frame);
    }

    pub fn writev(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.writev_supported
            || !self.writev_usercopy_ready
            || !self.writev_routes_to_files_struct
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_writev(self, frame);
    }

    pub fn set_tid_address(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready || !self.set_tid_address_supported {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_set_tid_address(self, frame);
    }

    pub fn clone(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.clone_supported
            || !self.clone_routes_to_task_creation_core
            || !self.clone_routes_to_user_clone_deferred_boundaries
            || !self.clone_plain_fork_first_slice
            || !self.clone_vfork_vm_first_slice
            || !self.clone_vfork_pidfd_first_slice
            || !self.clone_legacy_pidfd_parent_tidptr_bound
            || !self.clone_vfork_parent_frame_saved
            || !self.clone_pidfd_copyout_first_slice
            || !self.clone_full_vfork_scheduler_deferred
            || !self.clone_full_pidfd_file_ops_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_clone(self, frame);
    }

    #[cfg(app_user_boot)]
    pub fn execve(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.execve_supported
            || !self.path_usercopy_ready
            || !self.execve_child_continuation_first_slice
            || !self.execve_reuses_user_boot_payload_elf_loader
            || !self.execve_replaces_user_address_space_first_slice
            || !self.execve_context_staging_address_space_bound
            || !self.execve_sets_start_thread_frame_first_slice
            || !self.execve_bounded_argv_first_slice
            || !self.execve_envp_full_copy_deferred
            || !self.execve_close_on_exec_deferred
            || !self.execve_old_mm_reclaim_deferred
            || !self.execve_full_linux_model_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_execve(self, frame);
    }

    pub fn wait4(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.wait4_supported
            || !self.wait4_parent_wait_chldexit_boundary
            || !self.wait4_yields_to_user_child_continuation
            || !self.wait4_child_exit_status_copyout_first_slice
            || !self.wait4_observed_child_reap_first_slice
            || !self.wait4_no_child_echild_first_slice
            || !self.wait4_blocking_sleep_deferred
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_wait4(self, frame);
    }

    pub fn exit(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.exit_supported
            || !self.exit_records_status
        {
            panic_dispatch("syscall exit table entry not ready\n");
        }

        syscall_table_exit(self, frame)
    }

    pub fn exit_group(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.exit_group_supported
            || !self.exit_records_status
            || !self.exit_group_pid1_shutdown_child_wait4_split
        {
            panic_dispatch("syscall exit_group table entry not ready\n");
        }

        syscall_table_exit(self, frame)
    }
}

pub struct ExceptionStream {
    lifecycle: Lifecycle,
    page_fault: ExceptionKind,
    syscall: ExceptionKind,
    breakpoint: ExceptionKind,
    unexpected: ExceptionKind,
    dispatch_ready: bool,
}

impl ExceptionStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            page_fault: ExceptionKind::new(),
            syscall: ExceptionKind::new(),
            breakpoint: ExceptionKind::new(),
            unexpected: ExceptionKind::new(),
            dispatch_ready: false,
        }
    }

    pub fn preset(&mut self, event_stream: &EventStream, init_stack: &InitStack) -> EventResult {
        if self.lifecycle.state() != State::Base
            || event_stream.state() != State::Prepared
            || init_stack.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.page_fault.preset()?;
        self.syscall.preset()?;
        self.breakpoint.preset()?;
        self.unexpected.preset()?;
        reset_exception_handlers();
        bind_exception_policy(
            ExceptionHandlerBinding::Causes(&SYSCALL_CAUSES),
            SYSCALL_DISABLED_POLICY,
        );

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::ExceptionStreamPrepared,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, event_stream: &EventStream) -> EventResult {
        if self.lifecycle.state() != State::Prepared || event_stream.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.page_fault_setup()?;
        self.breakpoint_setup()?;
        self.unexpected_setup()?;
        self.dispatch_ready = true;
        DISPATCH_READY.store(1, Ordering::Relaxed);
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::ExceptionStreamReady,
        )
    }

    pub fn page_fault_setup(&mut self) -> EventResult {
        self.page_fault.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::Causes(&PAGE_FAULT_CAUSES),
            PAGE_FAULT_POLICY,
        )
    }

    pub fn breakpoint_setup(&mut self) -> EventResult {
        self.breakpoint.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::Causes(&BREAKPOINT_CAUSES),
            BREAKPOINT_POLICY,
        )
    }

    pub fn unexpected_setup(&mut self) -> EventResult {
        self.unexpected.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::RemainingKnown,
            UNEXPECTED_POLICY,
        )
    }

    pub fn syscall_setup(&mut self, table: &mut SyscallTable) -> EventResult {
        self.syscall.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::Causes(&SYSCALL_CAUSES),
            SYSCALL_POLICY,
        )?;
        table.setup(self.syscall.state())
    }

    pub fn syscall_enable(&mut self, table: &SyscallTable) -> EventResult {
        self.syscall.enable(self.lifecycle.state(), table)
    }

    pub fn page_fault_state(&self) -> State {
        self.page_fault.state()
    }

    pub fn syscall_state(&self) -> State {
        self.syscall.state()
    }

    pub fn breakpoint_state(&self) -> State {
        self.breakpoint.state()
    }

    pub fn unexpected_state(&self) -> State {
        self.unexpected.state()
    }

    #[allow(dead_code)]
    pub const fn dispatch_ready(&self) -> bool {
        self.dispatch_ready
    }
}

struct ExceptionKind {
    lifecycle: Lifecycle,
}

enum ExceptionHandlerBinding {
    Causes(&'static [usize]),
    RemainingKnown,
}

impl ExceptionKind {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self) -> EventResult {
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    fn setup(
        &mut self,
        exception_stream_state: State,
        binding: ExceptionHandlerBinding,
        policy: ExceptionPolicy,
    ) -> EventResult {
        if !matches!(exception_stream_state, State::Prepared | State::Ready)
            || self.lifecycle.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        bind_exception_policy(binding, policy);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    fn enable(
        &mut self,
        exception_stream_state: State,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if exception_stream_state != State::Ready
            || self.lifecycle.state() != State::Ready
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

        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub fn dispatch_trap(frame: &mut TrapFrame) {
    if frame.scause & SCAUSE_INTERRUPT_BIT != 0 {
        panic_dispatch("interrupt reached exception stream\n");
    }

    if DISPATCH_READY.load(Ordering::Relaxed) == 0 {
        panic_dispatch("exception dispatch not ready\n");
    }

    dispatch_handler_frame(read_exception_handler_policy(frame.scause), frame);
}

fn reset_exception_handlers() {
    for cause in 0..EXCEPTION_HANDLER_COUNT {
        set_exception_policy(cause, FALLBACK_POLICY);
    }
}

fn bind_exception_policy(binding: ExceptionHandlerBinding, policy: ExceptionPolicy) {
    match binding {
        ExceptionHandlerBinding::Causes(causes) => {
            for &cause in causes {
                set_exception_policy(cause, policy);
            }
        }
        ExceptionHandlerBinding::RemainingKnown => {
            for cause in 0..EXCEPTION_HANDLER_COUNT {
                if !known_mechanism_cause(cause) {
                    set_exception_policy(cause, policy);
                }
            }
        }
    }
}

fn read_exception_handler_policy(scause: usize) -> u8 {
    let cause = scause & !SCAUSE_INTERRUPT_BIT;
    if cause >= EXCEPTION_HANDLER_COUNT {
        return HANDLER_UNEXPECTED;
    }

    EXCEPTION_HANDLER_POLICY[cause].load(Ordering::Relaxed)
}

fn known_mechanism_cause(cause: usize) -> bool {
    PAGE_FAULT_CAUSES.contains(&cause)
        || SYSCALL_CAUSES.contains(&cause)
        || BREAKPOINT_CAUSES.contains(&cause)
}

fn set_exception_policy(cause: usize, policy: ExceptionPolicy) {
    if cause >= EXCEPTION_HANDLER_COUNT {
        return;
    }

    EXCEPTION_HANDLER_POLICY[cause].store(policy.0, Ordering::Relaxed);
}

fn dispatch_handler_frame(handler: u8, frame: &mut TrapFrame) {
    match handler {
        HANDLER_BREAKPOINT => breakpoint_exception_handler(frame),
        HANDLER_PAGE_FAULT => page_fault_exception_handler(frame),
        HANDLER_SYSCALL_DISABLED => syscall_disabled_exception_handler(frame),
        HANDLER_SYSCALL => syscall_exception_handler(frame),
        HANDLER_UNEXPECTED => unexpected_exception_handler(frame),
        _ => default_exception_handler(frame),
    }
}

fn syscall_table_ref() -> Option<&'static SyscallTable> {
    if SYSCALL_TABLE_READY.load(Ordering::Relaxed) == 0 {
        return None;
    }

    Some(&crate::context::context_ref().syscall_table)
}

fn default_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("exception fallback panic", frame)
}

fn page_fault_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("page fault exception", frame)
}

fn syscall_disabled_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("syscall exception not enabled", frame)
}

fn syscall_exception_handler(frame: &mut TrapFrame) {
    let Some(table) = syscall_table_ref() else {
        panic_dispatch("syscall table not ready\n");
    };

    match frame.reg(17) {
        SYSCALL_GETCWD => table.getcwd(frame),
        SYSCALL_DUP3 => table.dup3(frame),
        SYSCALL_FCNTL => table.fcntl(frame),
        SYSCALL_IOCTL => table.ioctl(frame),
        SYSCALL_FACCESSAT => table.faccessat(frame),
        SYSCALL_CHDIR => table.chdir(frame),
        SYSCALL_OPENAT => table.openat(frame),
        SYSCALL_CLOSE => table.close(frame),
        SYSCALL_GETDENTS64 => table.getdents64(frame),
        SYSCALL_LSEEK => table.lseek(frame),
        SYSCALL_READ => table.read(frame),
        SYSCALL_WRITE => table.write(frame),
        SYSCALL_WRITEV => table.writev(frame),
        SYSCALL_PPOLL => table.ppoll(frame),
        SYSCALL_READLINKAT => table.readlinkat(frame),
        SYSCALL_NEWFSTATAT => table.newfstatat(frame),
        SYSCALL_FSTAT => table.fstat(frame),
        SYSCALL_SET_TID_ADDRESS => table.set_tid_address(frame),
        SYSCALL_NANOSLEEP => table.nanosleep(frame),
        SYSCALL_CLOCK_GETTIME => table.clock_gettime(frame),
        SYSCALL_RT_SIGACTION => table.rt_sigaction(frame),
        SYSCALL_RT_SIGPROCMASK => table.rt_sigprocmask(frame),
        SYSCALL_RT_SIGTIMEDWAIT => table.rt_sigtimedwait(frame),
        SYSCALL_SETGID => table.setgid(frame),
        SYSCALL_SETUID => table.setuid(frame),
        SYSCALL_GETRESUID => table.getresuid(frame),
        SYSCALL_GETRESGID => table.getresgid(frame),
        SYSCALL_SETPGID => table.setpgid(frame),
        SYSCALL_GETSID => table.getsid(frame),
        SYSCALL_SETSID => table.setsid(frame),
        SYSCALL_UNAME => table.uname(frame),
        SYSCALL_GETTIMEOFDAY => table.gettimeofday(frame),
        SYSCALL_GETPID => table.getpid(frame),
        SYSCALL_GETPGID => table.getpgid(frame),
        SYSCALL_GETPPID => table.getppid(frame),
        SYSCALL_GETUID => table.getuid(frame),
        SYSCALL_GETEUID => table.geteuid(frame),
        SYSCALL_GETGID => table.getgid(frame),
        SYSCALL_GETEGID => table.getegid(frame),
        SYSCALL_BRK => table.brk(frame),
        SYSCALL_CLONE => table.clone(frame),
        SYSCALL_MMAP => table.mmap(frame),
        SYSCALL_MPROTECT => table.mprotect(frame),
        SYSCALL_MUNMAP => table.munmap(frame),
        SYSCALL_WAIT4 => table.wait4(frame),
        #[cfg(app_user_boot)]
        SYSCALL_EXECVE => table.execve(frame),
        SYSCALL_GETRANDOM => table.getrandom(frame),
        SYSCALL_EXIT => table.exit(frame),
        SYSCALL_EXIT_GROUP => table.exit_group(frame),
        _ => complete_unsupported_syscall(frame),
    }
}

fn complete_unsupported_syscall(frame: &mut TrapFrame) {
    print_unsupported_syscall_diagnostic(frame);
    complete_successful_syscall(frame, 0usize.wrapping_sub(ENOSYS));
}

fn complete_successful_syscall(frame: &mut TrapFrame, value: usize) {
    print_syscall_trace_return(frame, value);
    frame.set_reg(10, value);
    frame.sepc = frame.sepc.wrapping_add(4);
}

fn complete_error_syscall(frame: &mut TrapFrame, errno: usize) {
    print_syscall_error_diagnostic(frame, errno);
    complete_successful_syscall(frame, 0usize.wrapping_sub(errno));
}

fn file_error_to_errno(error: FileError) -> usize {
    match error {
        FileError::BadFd => 9,
        FileError::AlreadyOpen => 24,
        FileError::PathUnavailable => ENOENT,
        FileError::BufferTooSmall => EOVERFLOW,
        FileError::VfsBackendUnavailable => EREMOTEIO,
        FileError::InvalidArgument => EINVAL,
        FileError::IllegalSeek => ESPIPE,
        FileError::NotTty => ENOTTY,
        FileError::PermissionDenied => EACCES,
        FileError::TooManySymlinks => ELOOP,
        FileError::TooManyOpenFiles => EMFILE,
        FileError::NotDirectory => ENOTDIR,
        FileError::NotReady
        | FileError::NotReadable
        | FileError::NotWritable
        | FileError::BackendUnavailable
        | FileError::Unsupported => EIO,
    }
}

fn hwrng_error_to_getrandom_errno(error: HwRngError) -> usize {
    match error {
        HwRngError::CoreNotReady
        | HwRngError::DeviceNotReady
        | HwRngError::NoCurrentDevice
        | HwRngError::ProviderUnavailable
        | HwRngError::EmptyRead => EAGAIN,
        HwRngError::DuplicateName => EIO,
    }
}

fn vfs_error_to_chdir_errno(error: super::vfs::VfsError) -> Option<usize> {
    match error {
        super::vfs::VfsError::NotFound => Some(ENOENT),
        super::vfs::VfsError::NotDirectory => Some(ENOTDIR),
        super::vfs::VfsError::SymlinkLoop => Some(ELOOP),
        _ => None,
    }
}

fn syscall_table_openat(table: &SyscallTable, frame: &mut TrapFrame) {
    let dirfd = frame.reg(10);
    let path_ptr = frame.reg(11);
    let flags = frame.reg(12);
    let supported_flags = O_NONBLOCK | O_LARGEFILE | O_DIRECTORY | O_CLOEXEC | O_ACCMODE;
    if dirfd != AT_FDCWD || flags & !supported_flags != 0 {
        print_openat_reject_detail(dirfd, path_ptr, flags, supported_flags);
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut path = [0u8; USER_PATH_MAX];
    let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };

    let ctx = crate::context::context();
    let fd_result = if is_tty_path(&path[..path_len]) {
        ctx.files_struct
            .open_tty_path(&path[..path_len], flags as u32)
    } else if is_null_path(&path[..path_len]) {
        ctx.files_struct
            .open_null_path(&path[..path_len], flags as u32)
    } else if flags & O_NONBLOCK != 0 {
        Err(FileError::InvalidArgument)
    } else if flags & O_ACCMODE != 0 {
        Err(FileError::PermissionDenied)
    } else {
        ctx.files_struct.open_filesystem_path(
            &ctx.fs_struct,
            &mut ctx.vfs_core,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            &path[..path_len],
            flags as u32,
        )
    };
    let fd = match fd_result {
        Ok(fd) => fd,
        Err(error) => {
            print_openat_path_error_detail(error, &path[..path_len], flags);
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };

    table.openat_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(
        Checkpoint::SyscallTableOpenAt,
        crate::context::context_ref(),
    );
    complete_successful_syscall(frame, fd);
}

fn syscall_table_chdir(table: &SyscallTable, frame: &mut TrapFrame) {
    let path_ptr = frame.reg(10);
    let mut path = [0u8; USER_PATH_MAX];
    let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };

    let result = {
        let ctx = crate::context::context();
        let mut provider = super::virtio_blk::live_provider(&ctx.kernel_image);
        match ctx.vfs_core.walk_path(
            &ctx.fs_struct,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &mut provider,
            &path[..path_len],
        ) {
            Ok(dentry_ref) => ctx.fs_struct.chdir(&ctx.vfs_core, dentry_ref),
            Err(error) => Err(error),
        }
    };

    match result {
        Ok(()) => {
            table.chdir_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, 0);
        }
        Err(error) => match vfs_error_to_chdir_errno(error) {
            Some(errno) => complete_error_syscall(frame, errno),
            None => complete_unsupported_syscall(frame),
        },
    }
}

fn syscall_table_getdents64(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let user_ptr = frame.reg(11);
    let requested = frame.reg(12);
    let len = core::cmp::min(requested, USER_COPY_MAX);
    if len == 0 {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut buffer = [0u8; USER_COPY_MAX];
    let ctx = crate::context::context();
    let read = match ctx.files_struct.getdents64_fd(
        fd,
        &mut buffer[..len],
        &mut ctx.ext2_filesystem,
        &mut ctx.vfs_core,
        &mut ctx.block_device_registry,
        &ctx.kernel_image,
    ) {
        Ok(read) => read,
        Err(error) => {
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };
    if !copy_to_user(user_ptr, &buffer[..read]) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.getdents64_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, read);
}

fn syscall_table_read(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let user_ptr = frame.reg(11);
    let requested = frame.reg(12);
    let len = core::cmp::min(requested, USER_COPY_MAX);
    if len == 0 {
        print_read_trace_success(frame, fd, requested, len, 0);
        complete_successful_syscall(frame, 0);
        return;
    }

    let mut buffer = [0u8; USER_COPY_MAX];
    let first_read = {
        let ctx = crate::context::context();
        ctx.files_struct.read_fd(fd, &mut buffer[..len])
    };
    let read = match first_read {
        Ok(read) => read,
        Err(FileError::NotReady) if tty_input_wait_for_fd_read_ready(fd) => {
            let ctx = crate::context::context();
            match ctx.files_struct.read_fd(fd, &mut buffer[..len]) {
                Ok(read) => read,
                Err(error) => {
                    print_read_trace_file_error(frame, fd, requested, len, error);
                    complete_unsupported_syscall(frame);
                    return;
                }
            }
        }
        Err(error) => {
            print_read_trace_file_error(frame, fd, requested, len, error);
            complete_unsupported_syscall(frame);
            return;
        }
    };
    if !copy_to_user(user_ptr, &buffer[..read]) {
        print_read_trace_copy_error(frame, fd, requested, len, read);
        complete_unsupported_syscall(frame);
        return;
    }

    table.read_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableRead, crate::context::context_ref());
    print_read_trace_success(frame, fd, requested, len, read);
    complete_successful_syscall(frame, read);
}

#[derive(Clone, Copy)]
struct UserPollFd {
    fd: i32,
    events: u16,
}

fn syscall_table_ppoll(table: &SyscallTable, frame: &mut TrapFrame) {
    let fds_ptr = frame.reg(10);
    let nfds = frame.reg(11);
    let timeout_ptr = frame.reg(12);
    let sigmask_ptr = frame.reg(13);
    let mut timeout_value = None;

    if nfds > USER_PPOLL_MAX {
        complete_error_syscall(frame, EINVAL);
        return;
    }
    if timeout_ptr != 0 {
        let Some((sec, nsec)) = read_user_timespec_i64(timeout_ptr) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        if sec < 0 || nsec < 0 || nsec >= NSEC_PER_SEC as i64 {
            complete_error_syscall(frame, EINVAL);
            return;
        }
        timeout_value = Some((sec, nsec));
    }
    if sigmask_ptr != 0 {
        complete_unsupported_syscall(frame);
        return;
    }

    let mut index = 0usize;
    let mut pollfds = [UserPollFd { fd: -1, events: 0 }; USER_PPOLL_MAX];
    let mut entry_ptrs = [0usize; USER_PPOLL_MAX];
    let mut revents_values = [0u16; USER_PPOLL_MAX];
    while index < nfds {
        let Some(entry_offset) = index.checked_mul(POLLFD_SIZE) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        let Some(entry_ptr) = fds_ptr.checked_add(entry_offset) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        let Some(pollfd) = read_user_pollfd(entry_ptr) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };

        pollfds[index] = pollfd;
        entry_ptrs[index] = entry_ptr;
        index += 1;
    }

    let mut ready_count = match poll_user_fds_once(&pollfds, nfds, &mut revents_values) {
        Ok(count) => count,
        Err(()) => {
            complete_unsupported_syscall(frame);
            return;
        }
    };

    if ready_count == 0
        && timeout_value.is_none()
        && tty_input_wait_for_poll_ready(&pollfds, nfds, &mut revents_values)
    {
        ready_count = count_ready_revents(&revents_values, nfds);
    }

    let mut trace_index = 0usize;
    while trace_index < nfds {
        print_ppoll_trace_entry(
            frame,
            trace_index,
            pollfds[trace_index],
            revents_values[trace_index],
        );
        trace_index += 1;
    }

    print_ppoll_trace_summary(
        frame,
        nfds,
        timeout_ptr,
        timeout_value,
        sigmask_ptr,
        ready_count,
    );

    if ready_count == 0 && !ppoll_timeout_allows_immediate_zero(timeout_value) {
        complete_unsupported_syscall(frame);
        return;
    }

    let mut copy_index = 0usize;
    while copy_index < nfds {
        if !write_user_pollfd_revents(entry_ptrs[copy_index], revents_values[copy_index]) {
            complete_error_syscall(frame, EFAULT);
            return;
        }
        copy_index += 1;
    }

    table.ppoll_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, ready_count);
}

fn ppoll_timeout_allows_immediate_zero(timeout_value: Option<(i64, i64)>) -> bool {
    matches!(timeout_value, Some((0, 0)))
}

fn poll_user_fds_once(
    pollfds: &[UserPollFd; USER_PPOLL_MAX],
    nfds: usize,
    revents_values: &mut [u16; USER_PPOLL_MAX],
) -> Result<usize, ()> {
    let mut ready_count = 0usize;
    let mut index = 0usize;
    while index < nfds {
        let pollfd = pollfds[index];
        let revents = if pollfd.fd < 0 {
            0
        } else {
            match crate::context::context_ref()
                .files_struct
                .poll_fd(pollfd.fd as usize, pollfd.events)
            {
                Ok(revents) => revents,
                Err(FileError::BadFd) => POLLNVAL,
                Err(FileError::NotReady) | Err(FileError::BackendUnavailable) => return Err(()),
                Err(_) => 0,
            }
        };
        revents_values[index] = revents;
        if revents != 0 {
            ready_count += 1;
        }
        index += 1;
    }
    Ok(ready_count)
}

fn count_ready_revents(revents_values: &[u16; USER_PPOLL_MAX], nfds: usize) -> usize {
    let mut ready_count = 0usize;
    let mut index = 0usize;
    while index < nfds {
        if revents_values[index] != 0 {
            ready_count += 1;
        }
        index += 1;
    }
    ready_count
}

fn tty_input_wait_enabled() -> bool {
    crate::context::context_ref()
        .files_struct
        .stdin_blocking_wait_enabled()
}

fn tty_input_wait_for_fd_read_ready(fd: usize) -> bool {
    if !tty_input_wait_enabled() {
        return false;
    }

    crate::context::context_ref()
        .files_struct
        .record_tty_read_wait_entry();
    print_tty_wait_trace("read", "enter", fd, None);
    let saved_sstatus = crate::arch::riscv64::csr::read_sstatus();
    crate::arch::riscv64::csr::enable_supervisor_interrupts();
    loop {
        match crate::context::context_ref()
            .files_struct
            .poll_fd(fd, FILE_POLLIN)
        {
            Ok(revents) if revents & FILE_POLLIN != 0 => {
                crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
                crate::context::context_ref()
                    .files_struct
                    .record_tty_read_wait_finish(true);
                print_tty_wait_trace("read", "finish", fd, Some(true));
                return true;
            }
            Ok(_) => {}
            Err(_) => {
                crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
                crate::context::context_ref()
                    .files_struct
                    .record_tty_read_wait_finish(false);
                print_tty_wait_trace("read", "finish", fd, Some(false));
                return false;
            }
        }
        core::hint::spin_loop();
    }
}

fn tty_input_wait_for_poll_ready(
    pollfds: &[UserPollFd; USER_PPOLL_MAX],
    nfds: usize,
    revents_values: &mut [u16; USER_PPOLL_MAX],
) -> bool {
    if !tty_input_wait_enabled() || !pollfds_include_read_interest(pollfds, nfds) {
        return false;
    }

    crate::context::context_ref()
        .files_struct
        .record_tty_poll_wait_table();
    print_tty_wait_trace(
        "ppoll",
        "enter",
        first_read_interest_fd(pollfds, nfds),
        None,
    );
    let saved_sstatus = crate::arch::riscv64::csr::read_sstatus();
    crate::arch::riscv64::csr::enable_supervisor_interrupts();
    loop {
        match poll_user_fds_once(pollfds, nfds, revents_values) {
            Ok(ready_count) if ready_count != 0 => {
                crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
                crate::context::context_ref()
                    .files_struct
                    .record_tty_poll_freewait(true);
                print_tty_wait_trace(
                    "ppoll",
                    "freewait",
                    first_read_interest_fd(pollfds, nfds),
                    Some(true),
                );
                return true;
            }
            Ok(_) => {}
            Err(()) => {
                crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
                crate::context::context_ref()
                    .files_struct
                    .record_tty_poll_freewait(false);
                print_tty_wait_trace(
                    "ppoll",
                    "freewait",
                    first_read_interest_fd(pollfds, nfds),
                    Some(false),
                );
                return false;
            }
        }
        core::hint::spin_loop();
    }
}

fn pollfds_include_read_interest(pollfds: &[UserPollFd; USER_PPOLL_MAX], nfds: usize) -> bool {
    let mut index = 0usize;
    while index < nfds {
        let pollfd = pollfds[index];
        if pollfd.fd >= 0
            && pollfd.events & FILE_POLLIN != 0
            && crate::context::context_ref()
                .files_struct
                .fd_is_tty_read_wait_candidate(pollfd.fd as usize)
        {
            return true;
        }
        index += 1;
    }
    false
}

fn first_read_interest_fd(pollfds: &[UserPollFd; USER_PPOLL_MAX], nfds: usize) -> usize {
    let mut index = 0usize;
    while index < nfds {
        let pollfd = pollfds[index];
        if pollfd.fd >= 0
            && pollfd.events & FILE_POLLIN != 0
            && crate::context::context_ref()
                .files_struct
                .fd_is_tty_read_wait_candidate(pollfd.fd as usize)
        {
            return pollfd.fd as usize;
        }
        index += 1;
    }
    usize::MAX
}

fn syscall_table_close(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let ctx = crate::context::context();
    if let Err(error) = ctx.files_struct.close_fd(fd) {
        let errno = file_error_to_errno(error);
        print_close_error_detail(fd, error, errno);
        complete_error_syscall(frame, errno);
        return;
    }

    table.close_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableClose, crate::context::context_ref());
    complete_successful_syscall(frame, 0);
}

fn syscall_table_newfstatat(table: &SyscallTable, frame: &mut TrapFrame) {
    let dirfd = frame.reg(10);
    let path_ptr = frame.reg(11);
    let stat_ptr = frame.reg(12);
    let flags = frame.reg(13);
    if dirfd != AT_FDCWD || flags & !AT_SYMLINK_NOFOLLOW != 0 {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut path = [0u8; USER_PATH_MAX];
    let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };

    let ctx = crate::context::context();
    let stat = match ctx.files_struct.stat_path(
        &ctx.fs_struct,
        &mut ctx.vfs_core,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &ctx.kernel_image,
        &path[..path_len],
        flags & AT_SYMLINK_NOFOLLOW != 0,
    ) {
        Ok(stat) => stat,
        Err(error) => {
            print_path_syscall_error_detail(error, &path[..path_len]);
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };

    let mut stat_buffer = [0u8; STAT_SIZE];
    write_linux_stat(&mut stat_buffer, stat.size(), stat.mode());
    if !copy_to_user(stat_ptr, &stat_buffer) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.newfstatat_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(
        Checkpoint::SyscallTableNewFstatAt,
        crate::context::context_ref(),
    );
    complete_successful_syscall(frame, 0);
}

fn syscall_table_readlinkat(table: &SyscallTable, frame: &mut TrapFrame) {
    let dirfd = frame.reg(10);
    let path_ptr = frame.reg(11);
    let user_buf = frame.reg(12);
    let bufsiz = frame.reg(13);
    if bufsiz == 0 || bufsiz > isize::MAX as usize {
        complete_error_syscall(frame, EINVAL);
        return;
    }
    if dirfd != AT_FDCWD {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut path = [0u8; USER_PATH_MAX];
    let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };

    let len = core::cmp::min(bufsiz, USER_COPY_MAX);
    let mut buffer = [0u8; USER_COPY_MAX];
    let ctx = crate::context::context();
    let read = match ctx.files_struct.readlink_path(
        &ctx.fs_struct,
        &mut ctx.vfs_core,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &ctx.kernel_image,
        &path[..path_len],
        &mut buffer[..len],
    ) {
        Ok(read) => read,
        Err(error) => {
            print_path_syscall_error_detail(error, &path[..path_len]);
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };
    if !copy_to_user(user_buf, &buffer[..read]) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.readlinkat_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, read);
}

fn syscall_table_dup3(table: &SyscallTable, frame: &mut TrapFrame) {
    let oldfd = frame.reg(10);
    let newfd = frame.reg(11);
    let flags = frame.reg(12);
    if flags & !O_CLOEXEC != 0 {
        print_dup3_error_detail(oldfd, newfd, flags, EINVAL);
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let ctx = crate::context::context();
    let fd = match ctx
        .files_struct
        .dup3_fd(oldfd, newfd, flags & O_CLOEXEC != 0)
    {
        Ok(fd) => fd,
        Err(error) => {
            let errno = file_error_to_errno(error);
            print_dup3_error_detail(oldfd, newfd, flags, errno);
            complete_error_syscall(frame, errno);
            return;
        }
    };

    table.dup3_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, fd);
}

fn syscall_table_getrandom(table: &SyscallTable, frame: &mut TrapFrame) {
    let user_ptr = frame.reg(10);
    let requested = frame.reg(11);
    let flags = frame.reg(12);
    if flags != 0 || requested > USER_COPY_MAX {
        complete_error_syscall(frame, EINVAL);
        return;
    }
    if requested == 0 {
        complete_successful_syscall(frame, 0);
        return;
    }

    let mut buffer = [0u8; USER_COPY_MAX];
    let ctx = crate::context::context();
    let read = match ctx.virtio_rng_runtime.read_current_hwrng(
        &mut ctx.hwrng_core,
        &mut buffer[..requested],
        true,
    ) {
        Ok(read) => read,
        Err(error) => {
            complete_error_syscall(frame, hwrng_error_to_getrandom_errno(error));
            return;
        }
    };
    if !copy_to_user(user_ptr, &buffer[..read]) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.getrandom_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, read);
}

fn syscall_table_getcwd(table: &SyscallTable, frame: &mut TrapFrame) {
    let user_ptr = frame.reg(10);
    let size = frame.reg(11);
    const ROOT_CWD: &[u8; 2] = b"/\0";

    if size < ROOT_CWD.len() {
        complete_error_syscall(frame, ERANGE);
        return;
    }

    {
        let context = crate::context::context_ref();
        if context.fs_struct.state() != State::Ready
            || !context.fs_struct.root_pwd_same()
            || !context.fs_struct.chroot_dot_done()
        {
            complete_unsupported_syscall(frame);
            return;
        }
    }

    if !copy_to_user(user_ptr, ROOT_CWD) {
        complete_error_syscall(frame, EFAULT);
        return;
    }
    if !crate::context::context()
        .user_init_process
        .observe_getcwd_root_slice()
    {
        complete_unsupported_syscall(frame);
        return;
    }

    table.getcwd_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, ROOT_CWD.len());
}

fn syscall_table_getpid(table: &SyscallTable, frame: &mut TrapFrame) {
    let current_child_continuation = crate::context::context()
        .user_child_process
        .current_child_continuation();
    let Some(pid) = crate::context::context()
        .user_init_process
        .read_pid(current_child_continuation)
    else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getpid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, pid);
}

fn syscall_table_getpgid(table: &SyscallTable, frame: &mut TrapFrame) {
    let pid = frame.reg(10);
    let lookup = {
        let ctx = crate::context::context();
        let current_child_continuation = ctx.user_child_process.current_child_continuation();
        ctx.user_init_process
            .read_process_group(pid, current_child_continuation)
    };
    match lookup {
        UserProcessGroupLookup::Found(pgrp) => {
            table.getpgid_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, pgrp);
        }
        UserProcessGroupLookup::NoSuchProcess => complete_error_syscall(frame, ESRCH),
        UserProcessGroupLookup::NotReady => complete_unsupported_syscall(frame),
    }
}

fn syscall_table_getsid(table: &SyscallTable, frame: &mut TrapFrame) {
    let pid = frame.reg(10);
    let lookup = {
        let ctx = crate::context::context();
        let current_child_continuation = ctx.user_child_process.current_child_continuation();
        ctx.user_init_process
            .read_session_id(pid, current_child_continuation)
    };
    match lookup {
        UserProcessGroupLookup::Found(sid) => {
            table.getsid_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, sid);
        }
        UserProcessGroupLookup::NoSuchProcess => complete_error_syscall(frame, ESRCH),
        UserProcessGroupLookup::NotReady => complete_unsupported_syscall(frame),
    }
}

fn syscall_table_setpgid(table: &SyscallTable, frame: &mut TrapFrame) {
    let pid = frame.reg(10);
    let pgid = frame.reg(11);
    let update = {
        let ctx = crate::context::context();
        let current_child_continuation = ctx.user_child_process.current_child_continuation();
        ctx.user_init_process
            .set_process_group_first_slice(pid, pgid, current_child_continuation)
    };
    match update {
        UserProcessGroupUpdate::Updated(_) => {
            table.setpgid_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, 0);
        }
        UserProcessGroupUpdate::Invalid => complete_error_syscall(frame, EINVAL),
        UserProcessGroupUpdate::NoSuchProcess => complete_error_syscall(frame, ESRCH),
        UserProcessGroupUpdate::PermissionDenied => complete_error_syscall(frame, EPERM),
        UserProcessGroupUpdate::NotReady => complete_unsupported_syscall(frame),
    }
}

fn syscall_table_setsid(table: &SyscallTable, frame: &mut TrapFrame) {
    let update = {
        let ctx = crate::context::context();
        let current_child_continuation = ctx.user_child_process.current_child_continuation();
        ctx.user_init_process
            .set_session_id_first_slice(current_child_continuation)
    };
    match update {
        UserProcessGroupUpdate::Updated(sid) => {
            table.setsid_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, sid);
        }
        UserProcessGroupUpdate::PermissionDenied => {
            table.setsid_observed.store(1, Ordering::Release);
            complete_error_syscall(frame, EPERM);
        }
        UserProcessGroupUpdate::NotReady => complete_unsupported_syscall(frame),
        UserProcessGroupUpdate::Invalid | UserProcessGroupUpdate::NoSuchProcess => {
            complete_unsupported_syscall(frame)
        }
    }
}

fn syscall_table_getppid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(ppid) = crate::context::context().user_init_process.read_ppid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getppid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, ppid);
}

fn syscall_table_getuid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(uid) = crate::context::context().user_init_process.read_uid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getuid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, uid);
}

fn syscall_table_geteuid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(euid) = crate::context::context().user_init_process.read_euid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.geteuid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, euid);
}

fn syscall_table_getgid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(gid) = crate::context::context().user_init_process.read_gid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getgid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, gid);
}

fn syscall_table_getegid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(egid) = crate::context::context().user_init_process.read_egid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getegid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, egid);
}

fn syscall_table_getresuid(table: &SyscallTable, frame: &mut TrapFrame) {
    let ruid_ptr = frame.reg(10);
    let euid_ptr = frame.reg(11);
    let suid_ptr = frame.reg(12);
    let Some((ruid, euid, suid)) = crate::context::context().user_init_process.read_resuid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    if !write_user_u32(ruid_ptr, ruid as u32)
        || !write_user_u32(euid_ptr, euid as u32)
        || !write_user_u32(suid_ptr, suid as u32)
    {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.getresuid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_getresgid(table: &SyscallTable, frame: &mut TrapFrame) {
    let rgid_ptr = frame.reg(10);
    let egid_ptr = frame.reg(11);
    let sgid_ptr = frame.reg(12);
    let Some((rgid, egid, sgid)) = crate::context::context().user_init_process.read_resgid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    if !write_user_u32(rgid_ptr, rgid as u32)
        || !write_user_u32(egid_ptr, egid as u32)
        || !write_user_u32(sgid_ptr, sgid as u32)
    {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.getresgid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_uname(table: &SyscallTable, frame: &mut TrapFrame) {
    let user_ptr = frame.reg(10);
    let mut buffer = [0u8; NEW_UTSNAME_SIZE];

    write_uts_field(&mut buffer, 0, b"Linux");
    write_uts_field(&mut buffer, 1, b"(none)");
    write_uts_field(&mut buffer, 2, b"6.12.0+");
    write_uts_field(
        &mut buffer,
        3,
        b"#1 SMP PREEMPT Sun Jun 28 12:25:04 CST 2026",
    );
    write_uts_field(&mut buffer, 4, b"riscv64");
    write_uts_field(&mut buffer, 5, b"(none)");

    if !copy_to_user(user_ptr, &buffer) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.uname_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_setuid(table: &SyscallTable, frame: &mut TrapFrame) {
    let uid = frame.reg(10);
    if !crate::context::context()
        .user_init_process
        .set_uid_root_slice(uid)
    {
        complete_error_syscall(frame, EPERM);
        return;
    }

    table.setuid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_setgid(table: &SyscallTable, frame: &mut TrapFrame) {
    let gid = frame.reg(10);
    if !crate::context::context()
        .user_init_process
        .set_gid_root_slice(gid)
    {
        complete_error_syscall(frame, EPERM);
        return;
    }

    table.setgid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_rt_sigprocmask(table: &SyscallTable, frame: &mut TrapFrame) {
    let how = frame.reg(10);
    let new_set_ptr = frame.reg(11);
    let old_set_ptr = frame.reg(12);
    let sigset_size = frame.reg(13);
    if sigset_size != RT_SIGSET_SIZE {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let old_mask = crate::context::context_ref()
        .user_init_process
        .blocked_signal_mask();
    if new_set_ptr != 0 {
        let Some(mut new_mask) = read_user_usize(new_set_ptr) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        new_mask &= !UNBLOCKABLE_SIGNAL_MASK;

        let current_mask = crate::context::context_ref()
            .user_init_process
            .blocked_signal_mask();
        let next_mask = match how {
            SIG_BLOCK => current_mask | new_mask,
            SIG_UNBLOCK => current_mask & !new_mask,
            SIG_SETMASK => new_mask,
            _ => {
                complete_error_syscall(frame, EINVAL);
                return;
            }
        };
        if !crate::context::context()
            .user_init_process
            .set_blocked_signal_mask(next_mask)
        {
            complete_unsupported_syscall(frame);
            return;
        }
    }

    if old_set_ptr != 0 && !write_user_usize(old_set_ptr, old_mask) {
        complete_error_syscall(frame, EFAULT);
        return;
    }
    if !crate::context::context()
        .user_init_process
        .observe_rt_sigprocmask()
    {
        complete_unsupported_syscall(frame);
        return;
    }

    table.rt_sigprocmask_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_rt_sigaction(table: &SyscallTable, frame: &mut TrapFrame) {
    let signal = frame.reg(10);
    let action_ptr = frame.reg(11);
    let old_action_ptr = frame.reg(12);
    let sigset_size = frame.reg(13);
    if sigset_size != RT_SIGSET_SIZE {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let new_action = if action_ptr != 0 {
        let Some(action) = read_user_signal_action(action_ptr) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        Some(action)
    } else {
        None
    };

    if !valid_rt_signal(signal) || (new_action.is_some() && kernel_only_signal(signal)) {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let Some(old_action) = crate::context::context_ref()
        .user_init_process
        .read_signal_action(signal)
    else {
        complete_unsupported_syscall(frame);
        return;
    };

    if let Some(action) = new_action {
        let stored_action = UserSignalAction::new(
            action.handler(),
            action.flags() & UAPI_SA_FLAGS,
            action.mask() & !UNBLOCKABLE_SIGNAL_MASK,
        );
        if !crate::context::context()
            .user_init_process
            .set_signal_action(signal, stored_action)
        {
            complete_unsupported_syscall(frame);
            return;
        }
    }

    if old_action_ptr != 0 && !write_user_signal_action(old_action_ptr, old_action) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    if !crate::context::context()
        .user_init_process
        .observe_rt_sigaction()
    {
        complete_unsupported_syscall(frame);
        return;
    }

    table.rt_sigaction_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_rt_sigtimedwait(table: &SyscallTable, frame: &mut TrapFrame) {
    let uthese = frame.reg(10);
    let uinfo = frame.reg(11);
    let uts = frame.reg(12);
    let sigset_size = frame.reg(13);
    if sigset_size != RT_SIGSET_SIZE {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let Some(mask) = read_user_usize(uthese) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };
    if uinfo != 0 || uts != 0 {
        complete_unsupported_syscall(frame);
        return;
    }

    let result = crate::context::context()
        .user_init_process
        .begin_rt_sigtimedwait(mask, true, true, frame);
    if result == UserRtSigtimedwaitResult::Unsupported {
        complete_unsupported_syscall(frame);
        return;
    }

    table.rt_sigtimedwait_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(
        Checkpoint::SyscallTableRtSigtimedwait,
        crate::context::context_ref(),
    );

    match result {
        UserRtSigtimedwaitResult::ReturnSignal(signal) => {
            crate::checkpoint::dispatch(
                Checkpoint::SyscallTableRtSigtimedwaitReturnSignal,
                crate::context::context_ref(),
            );
            complete_successful_syscall(frame, signal);
        }
        UserRtSigtimedwaitResult::Sleep => {
            crate::checkpoint::dispatch(
                Checkpoint::UserSignalWaitSleep,
                crate::context::context_ref(),
            );
            print_rt_sigtimedwait_wait_boundary(mask);
            enter_rt_sigtimedwait_wait_boundary(frame);
        }
        UserRtSigtimedwaitResult::Unsupported => {}
    }
}

fn enter_rt_sigtimedwait_wait_boundary(frame: &mut TrapFrame) {
    let saved_sstatus = crate::arch::riscv64::csr::read_sstatus();
    crate::arch::riscv64::csr::enable_supervisor_interrupts();
    loop {
        let signal = crate::context::context()
            .user_init_process
            .complete_rt_sigtimedwait_wake(frame);
        if let Some(signal) = signal {
            crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
            crate::checkpoint::dispatch(
                Checkpoint::SyscallTableRtSigtimedwaitReturnSignal,
                crate::context::context_ref(),
            );
            complete_successful_syscall(frame, signal);
            return;
        }
        wait_for_signal_wait_event();
    }
}

fn wait_for_signal_wait_event() {
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack));
    }
}

fn syscall_table_clock_gettime(table: &SyscallTable, frame: &mut TrapFrame) {
    let clockid = frame.reg(10);
    let timespec_ptr = frame.reg(11);
    if !time_clockid_supported(clockid) {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let Some((sec, nsec)) = read_timer_parts(clockid, NSEC_PER_SEC) else {
        complete_unsupported_syscall(frame);
        return;
    };
    let mut buffer = [0u8; TIMESPEC_SIZE];
    write_u64(&mut buffer, 0, sec);
    write_u64(&mut buffer, 8, nsec);
    if !copy_to_user(timespec_ptr, &buffer) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.clock_gettime_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_gettimeofday(table: &SyscallTable, frame: &mut TrapFrame) {
    let timeval_ptr = frame.reg(10);
    let timezone_ptr = frame.reg(11);
    if timeval_ptr != 0 {
        let Some((sec, usec)) = read_timer_parts(CLOCK_REALTIME, USEC_PER_SEC) else {
            complete_unsupported_syscall(frame);
            return;
        };
        let mut buffer = [0u8; TIMEVAL_SIZE];
        write_u64(&mut buffer, 0, sec);
        write_u64(&mut buffer, 8, usec);
        if !copy_to_user(timeval_ptr, &buffer) {
            complete_error_syscall(frame, EFAULT);
            return;
        }
    }
    if timezone_ptr != 0 {
        let buffer = [0u8; TIMEZONE_SIZE];
        if !copy_to_user(timezone_ptr, &buffer) {
            complete_error_syscall(frame, EFAULT);
            return;
        }
    }

    table.gettimeofday_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_nanosleep(table: &SyscallTable, frame: &mut TrapFrame) {
    let rqtp = frame.reg(10);
    let Some((sec, nsec)) = read_user_timespec_i64(rqtp) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };
    if sec < 0 || nsec < 0 || nsec >= NSEC_PER_SEC as i64 {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let Some(duration_ns) = timespec_duration_ns(sec, nsec) else {
        complete_unsupported_syscall(frame);
        return;
    };
    if duration_ns > NANOSLEEP_FIRST_SLICE_MAX_NS
        || (duration_ns != 0 && !nanosleep_busy_wait(duration_ns))
    {
        complete_unsupported_syscall(frame);
        return;
    }

    table.nanosleep_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn timespec_duration_ns(sec: i64, nsec: i64) -> Option<u64> {
    let sec = sec as u64;
    let nsec = nsec as u64;
    sec.checked_mul(NSEC_PER_SEC)?.checked_add(nsec)
}

fn nanosleep_busy_wait(duration_ns: u64) -> bool {
    let ctx = crate::context::context_ref();
    if ctx.timekeeper.state() != State::Ready || !ctx.timekeeper.monotonic_time_ready() {
        return false;
    }

    let timebase_hz = ctx.riscv_timer_provider.timebase_hz();
    if timebase_hz == 0 {
        return false;
    }
    let Some(start) = ctx.riscv_timer_provider.read_time() else {
        return false;
    };
    let delta_ticks = ((duration_ns as u128 * timebase_hz as u128) + (NSEC_PER_SEC as u128 - 1))
        / NSEC_PER_SEC as u128;
    if delta_ticks == 0 || delta_ticks > u64::MAX as u128 {
        return false;
    }
    let Some(deadline) = start.checked_add(delta_ticks as u64) else {
        return false;
    };

    loop {
        let Some(now) = ctx.riscv_timer_provider.read_time() else {
            return false;
        };
        if now >= deadline {
            return true;
        }
        core::hint::spin_loop();
    }
}

fn time_clockid_supported(clockid: usize) -> bool {
    matches!(clockid, CLOCK_REALTIME | CLOCK_MONOTONIC)
}

fn read_timer_parts(clockid: usize, subsec_scale: u64) -> Option<(u64, u64)> {
    let ctx = crate::context::context_ref();
    if ctx.timekeeper.state() != State::Ready {
        return None;
    }
    match clockid {
        CLOCK_REALTIME if !ctx.timekeeper.wall_time_ready() => return None,
        CLOCK_MONOTONIC if !ctx.timekeeper.monotonic_time_ready() => return None,
        CLOCK_REALTIME | CLOCK_MONOTONIC => {}
        _ => return None,
    }

    let timebase_hz = ctx.riscv_timer_provider.timebase_hz();
    if timebase_hz == 0 {
        return None;
    }
    let ticks = ctx.riscv_timer_provider.read_time()?;
    let sec = ticks / timebase_hz;
    let subsec =
        ((ticks % timebase_hz) as u128 * subsec_scale as u128 / timebase_hz as u128) as u64;
    Some((sec, subsec))
}

fn syscall_table_fstat(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let stat_ptr = frame.reg(11);
    let ctx = crate::context::context();
    let stat = match ctx.files_struct.fstat_fd(fd, &ctx.vfs_core) {
        Ok(stat) => stat,
        Err(error) => {
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };

    let mut stat_buffer = [0u8; STAT_SIZE];
    write_linux_stat(&mut stat_buffer, stat.size(), stat.mode());
    if !copy_to_user(stat_ptr, &stat_buffer) {
        complete_error_syscall(frame, EFAULT);
        return;
    }

    table.fstat_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_fcntl(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let cmd = frame.reg(11);
    let arg = frame.reg(12);
    match cmd {
        F_DUPFD | F_DUPFD_CLOEXEC => {
            let ctx = crate::context::context();
            let new_fd = match ctx
                .files_struct
                .fcntl_dupfd_fd(fd, arg, cmd == F_DUPFD_CLOEXEC)
            {
                Ok(new_fd) => new_fd,
                Err(error) => {
                    let errno = file_error_to_errno(error);
                    print_fcntl_error_detail(fd, cmd, arg, errno);
                    complete_error_syscall(frame, errno);
                    return;
                }
            };

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, new_fd);
        }
        F_GETFL => {
            let ctx = crate::context::context_ref();
            let flags = match ctx.files_struct.fcntl_getfl_fd(fd) {
                Ok(flags) => flags,
                Err(error) => {
                    let errno = file_error_to_errno(error);
                    print_fcntl_error_detail(fd, cmd, arg, errno);
                    complete_error_syscall(frame, errno);
                    return;
                }
            };

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, flags as usize);
        }
        F_SETFL => {
            let ctx = crate::context::context();
            if let Err(error) = ctx.files_struct.fcntl_setfl_fd(fd, arg as u32) {
                let errno = file_error_to_errno(error);
                print_fcntl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, 0);
        }
        F_GETFD => {
            let ctx = crate::context::context_ref();
            let flags = match ctx.files_struct.fcntl_getfd_fd(fd) {
                Ok(flags) => flags,
                Err(error) => {
                    let errno = file_error_to_errno(error);
                    print_fcntl_error_detail(fd, cmd, arg, errno);
                    complete_error_syscall(frame, errno);
                    return;
                }
            };

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, flags as usize);
        }
        F_SETFD => {
            let ctx = crate::context::context();
            if let Err(error) = ctx
                .files_struct
                .fcntl_setfd_fd(fd, (arg & FD_CLOEXEC) as u32)
            {
                let errno = file_error_to_errno(error);
                print_fcntl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, 0);
        }
        _ => {
            print_fcntl_error_detail(fd, cmd, arg, EINVAL);
            complete_error_syscall(frame, EINVAL);
        }
    }
}

fn syscall_table_ioctl(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let cmd = frame.reg(11);
    let arg = frame.reg(12);
    if let Err(error) = crate::context::context_ref()
        .files_struct
        .ioctl_validate_fd(fd)
    {
        let errno = file_error_to_errno(error);
        print_ioctl_error_detail(fd, cmd, arg, errno);
        complete_error_syscall(frame, errno);
        return;
    }
    match cmd {
        TIOCGWINSZ => {
            let winsize = match crate::context::context_ref()
                .files_struct
                .ioctl_tiocgwinsz_fd(fd)
            {
                Ok(winsize) => winsize,
                Err(error) => {
                    let errno = file_error_to_errno(error);
                    print_ioctl_error_detail(fd, cmd, arg, errno);
                    complete_error_syscall(frame, errno);
                    return;
                }
            };

            let mut buffer = [0u8; WINSIZE_SIZE];
            write_u16(&mut buffer, 0, winsize.0);
            write_u16(&mut buffer, 2, winsize.1);
            write_u16(&mut buffer, 4, winsize.2);
            write_u16(&mut buffer, 6, winsize.3);
            if !copy_to_user(arg, &buffer) {
                print_ioctl_error_detail(fd, cmd, arg, EFAULT);
                complete_error_syscall(frame, EFAULT);
                return;
            }
        }
        TCGETS => {
            let termios = match crate::context::context_ref()
                .files_struct
                .ioctl_tcgets_fd(fd)
            {
                Ok(termios) => termios,
                Err(error) => {
                    let errno = file_error_to_errno(error);
                    print_ioctl_error_detail(fd, cmd, arg, errno);
                    complete_error_syscall(frame, errno);
                    return;
                }
            };
            if !copy_to_user(arg, &termios[..TERMIOS_SIZE]) {
                print_ioctl_error_detail(fd, cmd, arg, EFAULT);
                complete_error_syscall(frame, EFAULT);
                return;
            }
        }
        TCSETS => {
            let mut termios = [0u8; TERMIOS_SIZE];
            if !copy_from_user(arg, &mut termios) {
                print_ioctl_error_detail(fd, cmd, arg, EFAULT);
                complete_error_syscall(frame, EFAULT);
                return;
            }
            if let Err(error) = crate::context::context()
                .files_struct
                .ioctl_tcsets_fd(fd, termios)
            {
                let errno = file_error_to_errno(error);
                print_ioctl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }
        }
        TIOCGPGRP => {
            if let Err(error) = crate::context::context_ref()
                .files_struct
                .ioctl_tiocgpgrp_fd(fd)
            {
                let errno = file_error_to_errno(error);
                print_ioctl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }
            let Some(pgrp) = crate::context::context()
                .user_init_process
                .read_foreground_pgrp()
            else {
                print_ioctl_error_detail(fd, cmd, arg, ENOTTY);
                complete_error_syscall(frame, ENOTTY);
                return;
            };
            if !write_user_u32(arg, pgrp as u32) {
                print_ioctl_error_detail(fd, cmd, arg, EFAULT);
                complete_error_syscall(frame, EFAULT);
                return;
            }
        }
        TIOCSPGRP => {
            if let Err(error) = crate::context::context_ref()
                .files_struct
                .ioctl_tiocspgrp_fd(fd)
            {
                let errno = file_error_to_errno(error);
                print_ioctl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }
            let Some(pgrp) = read_user_u32(arg) else {
                print_ioctl_error_detail(fd, cmd, arg, EFAULT);
                complete_error_syscall(frame, EFAULT);
                return;
            };
            let update = crate::context::context()
                .user_init_process
                .set_foreground_pgrp_first_slice(pgrp);
            match update {
                UserProcessGroupUpdate::Updated(_) => {}
                UserProcessGroupUpdate::Invalid => {
                    print_ioctl_error_detail(fd, cmd, arg, EINVAL);
                    complete_error_syscall(frame, EINVAL);
                    return;
                }
                UserProcessGroupUpdate::NoSuchProcess => {
                    print_ioctl_error_detail(fd, cmd, arg, ESRCH);
                    complete_error_syscall(frame, ESRCH);
                    return;
                }
                UserProcessGroupUpdate::PermissionDenied => {
                    print_ioctl_error_detail(fd, cmd, arg, EPERM);
                    complete_error_syscall(frame, EPERM);
                    return;
                }
                UserProcessGroupUpdate::NotReady => {
                    print_ioctl_error_detail(fd, cmd, arg, ENOTTY);
                    complete_error_syscall(frame, ENOTTY);
                    return;
                }
            }
        }
        TIOCGSID => {
            if let Err(error) = crate::context::context_ref()
                .files_struct
                .ioctl_tiocgsid_fd(fd)
            {
                let errno = file_error_to_errno(error);
                print_ioctl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }
            let lookup = {
                let ctx = crate::context::context();
                let current_child_continuation =
                    ctx.user_child_process.current_child_continuation();
                ctx.user_init_process
                    .read_tty_session_id_first_slice(current_child_continuation)
            };
            match lookup {
                UserProcessGroupLookup::Found(sid) => {
                    if !write_user_u32(arg, sid as u32) {
                        print_ioctl_error_detail(fd, cmd, arg, EFAULT);
                        complete_error_syscall(frame, EFAULT);
                        return;
                    }
                }
                UserProcessGroupLookup::NoSuchProcess | UserProcessGroupLookup::NotReady => {
                    print_ioctl_error_detail(fd, cmd, arg, ENOTTY);
                    complete_error_syscall(frame, ENOTTY);
                    return;
                }
            }
        }
        TIOCSCTTY => {
            if let Err(error) = crate::context::context_ref()
                .files_struct
                .ioctl_tiocsctty_fd(fd)
            {
                let errno = file_error_to_errno(error);
                print_ioctl_error_detail(fd, cmd, arg, errno);
                complete_error_syscall(frame, errno);
                return;
            }
            let update = {
                let ctx = crate::context::context();
                let current_child_continuation =
                    ctx.user_child_process.current_child_continuation();
                ctx.user_init_process
                    .bind_controlling_tty_first_slice(current_child_continuation, arg)
            };
            match update {
                UserProcessGroupUpdate::Updated(_) => {}
                UserProcessGroupUpdate::Invalid => {
                    print_ioctl_error_detail(fd, cmd, arg, EINVAL);
                    complete_error_syscall(frame, EINVAL);
                    return;
                }
                UserProcessGroupUpdate::NoSuchProcess => {
                    print_ioctl_error_detail(fd, cmd, arg, ESRCH);
                    complete_error_syscall(frame, ESRCH);
                    return;
                }
                UserProcessGroupUpdate::PermissionDenied => {
                    print_ioctl_error_detail(fd, cmd, arg, EPERM);
                    complete_error_syscall(frame, EPERM);
                    return;
                }
                UserProcessGroupUpdate::NotReady => {
                    print_ioctl_error_detail(fd, cmd, arg, ENOTTY);
                    complete_error_syscall(frame, ENOTTY);
                    return;
                }
            }
        }
        _ => {
            print_ioctl_error_detail(fd, cmd, arg, ENOTTY);
            complete_error_syscall(frame, ENOTTY);
            return;
        }
    }

    table.ioctl_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, 0);
}

fn syscall_table_faccessat(table: &SyscallTable, frame: &mut TrapFrame) {
    let dirfd = frame.reg(10);
    let path_ptr = frame.reg(11);
    let mode = frame.reg(12);
    const ACCESS_MODE_MASK: usize = ACCESS_R_OK | ACCESS_W_OK | ACCESS_X_OK;
    if mode & !ACCESS_MODE_MASK != 0 {
        complete_error_syscall(frame, EINVAL);
        return;
    }
    if dirfd != AT_FDCWD {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut path = [0u8; USER_PATH_MAX];
    let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };

    let ctx = crate::context::context();
    match ctx.files_struct.access_path(
        &ctx.fs_struct,
        &mut ctx.vfs_core,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &ctx.kernel_image,
        &path[..path_len],
        mode,
    ) {
        Ok(()) => {
            table.faccessat_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, 0);
        }
        Err(error) => {
            print_path_syscall_error_detail(error, &path[..path_len]);
            complete_error_syscall(frame, file_error_to_errno(error));
        }
    }
}

fn syscall_table_lseek(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let offset = frame.reg(11) as isize;
    let whence = frame.reg(12);
    let ctx = crate::context::context();
    let target = match ctx.files_struct.lseek_fd(fd, offset, whence, &ctx.vfs_core) {
        Ok(target) => target,
        Err(error) => {
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };

    table.lseek_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, target);
}

fn syscall_table_brk(frame: &mut TrapFrame) {
    let requested = frame.reg(10);
    let ctx = crate::context::context();
    let current = ctx.user_address_space.user_brk(requested);
    complete_successful_syscall(frame, current);
}

fn syscall_table_mmap(frame: &mut TrapFrame) {
    let addr = frame.reg(10);
    let len = frame.reg(11);
    let prot = frame.reg(12);
    let flags = frame.reg(13);
    let fd = frame.reg(14);
    let offset = frame.reg(15);
    let ctx = crate::context::context();
    let mapped = match ctx
        .user_address_space
        .user_mmap(addr, len, prot, flags, fd, offset)
    {
        Ok(mapped) => mapped,
        Err(UserMmapError::Invalid) => {
            complete_error_syscall(frame, EINVAL);
            return;
        }
        Err(UserMmapError::NoMemory) => {
            complete_error_syscall(frame, ENOMEM);
            return;
        }
    };
    complete_successful_syscall(frame, mapped);
}

fn syscall_table_mprotect(frame: &mut TrapFrame) {
    let addr = frame.reg(10);
    let len = frame.reg(11);
    let _prot = frame.reg(12);
    let ctx = crate::context::context_ref();
    if ctx.user_address_space.user_mprotect(addr, len) {
        complete_successful_syscall(frame, 0);
    } else {
        complete_error_syscall(frame, ENOMEM);
    }
}

fn syscall_table_munmap(frame: &mut TrapFrame) {
    let addr = frame.reg(10);
    let len = frame.reg(11);
    let ctx = crate::context::context_ref();
    if ctx.user_address_space.user_munmap(addr, len) {
        complete_successful_syscall(frame, 0);
    } else {
        complete_error_syscall(frame, ENOMEM);
    }
}

fn syscall_table_write(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let user_ptr = frame.reg(11);
    let len = frame.reg(12);
    if len > USER_COPY_MAX {
        complete_unsupported_syscall(frame);
        return;
    }

    let mut buffer = [0u8; USER_COPY_MAX];
    if !copy_from_user(user_ptr, &mut buffer[..len]) {
        complete_unsupported_syscall(frame);
        return;
    }

    let Ok(written) = crate::context::context_ref()
        .files_struct
        .write_fd(fd, &buffer[..len])
    else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.write_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableWrite, crate::context::context_ref());
    complete_successful_syscall(frame, written);
}

fn syscall_table_writev(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let iov_ptr = frame.reg(11);
    let iovcnt = frame.reg(12);
    if iovcnt == 0 {
        complete_successful_syscall(frame, 0);
        return;
    }
    if iovcnt > USER_IOV_MAX {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut total = 0usize;
    let mut index = 0usize;
    while index < iovcnt {
        let entry_ptr = iov_ptr + index * 16;
        let Some(base) = read_user_usize(entry_ptr) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        let Some(len) = read_user_usize(entry_ptr + 8) else {
            complete_error_syscall(frame, EFAULT);
            return;
        };
        if len > USER_COPY_MAX || total.checked_add(len).is_none() {
            complete_error_syscall(frame, EINVAL);
            return;
        }
        if len != 0 {
            let mut buffer = [0u8; USER_COPY_MAX];
            if !copy_from_user(base, &mut buffer[..len]) {
                complete_error_syscall(frame, EFAULT);
                return;
            }
            let Ok(written) = crate::context::context_ref()
                .files_struct
                .write_fd(fd, &buffer[..len])
            else {
                complete_unsupported_syscall(frame);
                return;
            };
            total += written;
            if written != len {
                table.writev_observed.store(1, Ordering::Release);
                crate::checkpoint::dispatch(
                    Checkpoint::SyscallTableWritev,
                    crate::context::context_ref(),
                );
                complete_successful_syscall(frame, total);
                return;
            }
        }
        index += 1;
    }

    table.writev_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(
        Checkpoint::SyscallTableWritev,
        crate::context::context_ref(),
    );
    complete_successful_syscall(frame, total);
}

fn syscall_table_clone(table: &SyscallTable, frame: &mut TrapFrame) {
    let clone_flags = frame.reg(10);
    let newsp = frame.reg(11);
    let parent_tidptr = frame.reg(12);
    let _child_tidptr = frame.reg(13);
    let _tls = frame.reg(14);

    let clone_is_plain_fork = crate::context::context_ref()
        .user_clone_deferred_boundaries
        .accepts_plain_fork_first_slice(clone_flags, newsp);
    let clone_is_vfork_pidfd = crate::context::context_ref()
        .user_clone_deferred_boundaries
        .accepts_vfork_pidfd_first_slice(clone_flags);
    let clone_is_vfork_vm = crate::context::context_ref()
        .user_clone_deferred_boundaries
        .accepts_vfork_vm_first_slice(clone_flags);

    if clone_is_plain_fork {
        let child_pid = {
            let ctx = crate::context::context();
            let copy_result = match ctx.task_creation_core.copy_user_process(
                TaskCopyUserProcessInputs {
                    src_process: &ctx.user_init_process,
                    dst_process: &ctx.user_child_process,
                    root_pid_namespace: &ctx.root_pid_namespace,
                    scheduler: &ctx.scheduler,
                    cpu_group: &ctx.cpu_group,
                    fs_struct: &ctx.fs_struct,
                    files_struct: &ctx.files_struct,
                    address_space: &ctx.user_address_space,
                    trap_frame: &ctx.user_trap_frame,
                    boundaries: &ctx.user_clone_deferred_boundaries,
                    entry: TaskEntry::UserChild,
                    allow_nested_vfork: false,
                },
                ctx.user_child_process.state(),
                TaskEntry::UserChild,
            ) {
                Ok(result) => result,
                Err(_) => {
                    complete_unsupported_syscall(frame);
                    return;
                }
            };

            let Some(child_pid) = ctx.user_child_process.copy_plain_fork_from_parent(
                &ctx.user_init_process,
                &ctx.user_clone_deferred_boundaries,
                &ctx.user_address_space,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &ctx.page_metadata_map,
                frame,
                clone_flags,
                newsp,
                copy_result.task_struct_allocated(),
                copy_result.thread_context_ready(),
                copy_result.sched_entity_ready(),
                copy_result.task_state_new(),
            ) else {
                complete_unsupported_syscall(frame);
                return;
            };

            let runqueue_ref = match ctx
                .scheduler
                .select_runqueue_for_task(child_pid, &ctx.cpu_group)
            {
                Ok(runqueue_ref) => runqueue_ref,
                Err(_) => {
                    complete_unsupported_syscall(frame);
                    return;
                }
            };
            if ctx
                .scheduler
                .enqueue_task_on_runqueue(child_pid, runqueue_ref)
                .is_err()
            {
                complete_unsupported_syscall(frame);
                return;
            }
            if !ctx.user_child_process.mark_enqueued() {
                complete_unsupported_syscall(frame);
                return;
            }
            if !ctx
                .user_init_process
                .observe_child_process_group_visible(child_pid)
            {
                complete_unsupported_syscall(frame);
                return;
            }
            child_pid
        };

        table.clone_observed.store(1, Ordering::Release);
        crate::checkpoint::dispatch(Checkpoint::SyscallTableClone, crate::context::context_ref());
        complete_successful_syscall(frame, child_pid);
        return;
    }

    if !clone_is_vfork_pidfd && !clone_is_vfork_vm {
        print_clone_vfork_boundary(frame, "shape_rejected");
        complete_unsupported_syscall(frame);
        return;
    }
    if clone_is_vfork_pidfd && parent_tidptr == 0 {
        print_clone_vfork_boundary(frame, "parent_tidptr_null");
        complete_error_syscall(frame, EFAULT);
        return;
    }
    if crate::context::context_ref()
        .user_child_process
        .completed_child_records_full()
    {
        print_clone_vfork_boundary(frame, "child_records_full");
        complete_unsupported_syscall(frame);
        return;
    }
    let clone_is_nested_vfork = clone_is_vfork_vm
        && crate::context::context_ref()
            .user_child_process
            .nested_vfork_copy_ready();

    let child_frame = {
        let ctx = crate::context::context();
        let child_pid = ctx.user_child_process.next_child_pid();
        let copy_result = match ctx.task_creation_core.copy_user_process(
            TaskCopyUserProcessInputs {
                src_process: &ctx.user_init_process,
                dst_process: &ctx.user_child_process,
                root_pid_namespace: &ctx.root_pid_namespace,
                scheduler: &ctx.scheduler,
                cpu_group: &ctx.cpu_group,
                fs_struct: &ctx.fs_struct,
                files_struct: &ctx.files_struct,
                address_space: &ctx.user_address_space,
                trap_frame: &ctx.user_trap_frame,
                boundaries: &ctx.user_clone_deferred_boundaries,
                entry: TaskEntry::UserChild,
                allow_nested_vfork: clone_is_nested_vfork,
            },
            ctx.user_child_process.state(),
            TaskEntry::UserChild,
        ) {
            Ok(result) => result,
            Err(_) => {
                print_clone_vfork_boundary(frame, "copy_user_process");
                complete_unsupported_syscall(frame);
                return;
            }
        };

        let (pidfd_fd, pidfd_copyout) = if clone_is_vfork_pidfd {
            let pidfd_fd = match ctx.files_struct.install_pidfd(child_pid) {
                Ok(fd) => fd,
                Err(_) => {
                    print_clone_vfork_boundary(frame, "pidfd_install");
                    complete_unsupported_syscall(frame);
                    return;
                }
            };
            if !write_user_u32(parent_tidptr, pidfd_fd as u32) {
                let _ = ctx.files_struct.close_fd(pidfd_fd);
                print_clone_vfork_boundary(frame, "pidfd_copyout");
                complete_error_syscall(frame, EFAULT);
                return;
            }
            (pidfd_fd, true)
        } else {
            (usize::MAX, false)
        };

        let (child_frame, nested_parent_pid) = if clone_is_nested_vfork {
            let Some((child_frame, parent_pid)) =
                ctx.user_child_process.copy_nested_vfork_from_current_child(
                    &ctx.user_init_process,
                    &ctx.user_clone_deferred_boundaries,
                    &ctx.user_address_space,
                    &ctx.user_trap_frame,
                    &ctx.fs_struct,
                    &ctx.files_struct,
                    &mut ctx.page_allocator,
                    &ctx.page_metadata_map,
                    frame,
                    clone_flags,
                    newsp,
                    copy_result.task_struct_allocated(),
                    copy_result.thread_context_ready(),
                    copy_result.sched_entity_ready(),
                    copy_result.task_state_new(),
                )
            else {
                print_clone_vfork_boundary(frame, "nested_child_copy");
                complete_unsupported_syscall(frame);
                return;
            };
            (child_frame, parent_pid)
        } else {
            let Some(child_frame) = ctx.user_child_process.copy_vfork_from_parent(
                &ctx.user_init_process,
                &ctx.user_clone_deferred_boundaries,
                &ctx.user_address_space,
                &ctx.user_trap_frame,
                &ctx.fs_struct,
                &ctx.files_struct,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                frame,
                clone_flags,
                newsp,
                pidfd_fd,
                pidfd_copyout,
                copy_result.task_struct_allocated(),
                copy_result.thread_context_ready(),
                copy_result.sched_entity_ready(),
                copy_result.task_state_new(),
            ) else {
                print_clone_vfork_boundary(frame, "child_copy");
                complete_unsupported_syscall(frame);
                return;
            };
            (child_frame, 0)
        };

        if !clone_is_nested_vfork {
            let runqueue_ref = match ctx
                .scheduler
                .select_runqueue_for_task(USER_CHILD_PID, &ctx.cpu_group)
            {
                Ok(runqueue_ref) => runqueue_ref,
                Err(_) => {
                    print_clone_vfork_boundary(frame, "select_runqueue");
                    complete_unsupported_syscall(frame);
                    return;
                }
            };
            if ctx
                .scheduler
                .enqueue_task_on_runqueue(USER_CHILD_PID, runqueue_ref)
                .is_err()
            {
                print_clone_vfork_boundary(frame, "enqueue");
                complete_unsupported_syscall(frame);
                return;
            }
            if !ctx.user_child_process.mark_enqueued() {
                print_clone_vfork_boundary(frame, "mark_enqueued");
                complete_unsupported_syscall(frame);
                return;
            }
        }
        let child_visible = if clone_is_nested_vfork {
            ctx.user_init_process
                .observe_nested_child_process_group_visible(nested_parent_pid, child_pid)
        } else {
            ctx.user_init_process
                .observe_child_process_group_visible(child_pid)
        };
        if !child_visible {
            print_clone_vfork_boundary(frame, "process_group_visible");
            complete_unsupported_syscall(frame);
            return;
        }
        child_frame
    };

    table.clone_observed.store(1, Ordering::Release);
    if clone_is_vfork_pidfd {
        crate::checkpoint::dispatch(
            Checkpoint::FilesStructPidfdInstall,
            crate::context::context_ref(),
        );
        crate::checkpoint::dispatch(
            Checkpoint::SyscallTableCloneVforkPidfd,
            crate::context::context_ref(),
        );
    } else {
        crate::checkpoint::dispatch(
            Checkpoint::SyscallTableCloneVforkVm,
            crate::context::context_ref(),
        );
    }
    crate::checkpoint::dispatch(
        Checkpoint::UserCloneVforkChildHandoff,
        crate::context::context_ref(),
    );
    if crate::context::context_ref()
        .user_child_process
        .vfork_next_child_accepted()
    {
        crate::checkpoint::dispatch(
            Checkpoint::UserCloneVforkNextChildAccepted,
            crate::context::context_ref(),
        );
    }
    *frame = child_frame;
}

#[cfg(app_user_boot)]
fn syscall_table_execve(table: &SyscallTable, frame: &mut TrapFrame) {
    if !crate::context::context_ref()
        .user_child_process
        .current_child_continuation()
    {
        reset_execve_checkpoint_observation();
        record_execve_failure(
            EXECVE_FAIL_STAGE_CHILD_CONTINUATION,
            EXECVE_FAIL_REASON_NOT_CHILD_CONTINUATION,
        );
        complete_unsupported_syscall(frame);
        return;
    }

    let mut filename = [0u8; USER_PATH_MAX];
    let Some(filename_len) = copy_execve_cstr(frame.reg(10), &mut filename) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };
    if filename[0] != b'/' {
        reset_execve_checkpoint_observation();
        record_execve_failure(
            EXECVE_FAIL_STAGE_PATH_KIND,
            EXECVE_FAIL_REASON_RELATIVE_PATH,
        );
        complete_unsupported_syscall(frame);
        return;
    }
    let argv = match copy_execve_argv(frame.reg(11)) {
        Ok(argv) => argv,
        Err(ExecveArgvCopyError::Fault) => {
            reset_execve_checkpoint_observation();
            record_execve_failure(EXECVE_FAIL_STAGE_ARGV_COPY, EXECVE_FAIL_REASON_ARGV_COPY);
            complete_error_syscall(frame, EFAULT);
            return;
        }
        Err(ExecveArgvCopyError::CapacityExceeded(argv)) => {
            record_execve_args(
                filename_len,
                argv.argv0_len(),
                argv.argc,
                argv.total_bytes,
                true,
                frame,
            );
            record_execve_failure_detail(
                EXECVE_FAIL_STAGE_ARGV_COPY,
                EXECVE_FAIL_REASON_ARGV_CAPACITY,
                argv.argc,
                0,
                0,
            );
            complete_unsupported_syscall(frame);
            return;
        }
    };
    record_execve_args(
        filename_len,
        argv.argv0_len(),
        argv.argc,
        argv.total_bytes,
        false,
        frame,
    );
    crate::checkpoint::dispatch(
        Checkpoint::SyscallTableExecveArgsReady,
        crate::context::context_ref(),
    );

    let argv_slices = [
        &argv.bytes[0][..argv.lens[0]],
        &argv.bytes[1][..argv.lens[1]],
        &argv.bytes[2][..argv.lens[2]],
        &argv.bytes[3][..argv.lens[3]],
    ];
    let result = replace_current_user_exec_image(
        &filename[..filename_len],
        &argv_slices[..argv.argc],
        frame,
    );
    match result {
        Ok(()) => {
            table.execve_observed.store(1, Ordering::Release);
        }
        Err(ExecveFirstSliceError::Fault) => complete_error_syscall(frame, EFAULT),
        Err(ExecveFirstSliceError::NotFound) => complete_error_syscall(frame, ENOENT),
        Err(ExecveFirstSliceError::Unsupported) => complete_unsupported_syscall(frame),
    }
}

#[cfg(app_user_boot)]
#[derive(Clone, Copy, Eq, PartialEq)]
enum ExecveFirstSliceError {
    Fault,
    NotFound,
    Unsupported,
}

#[cfg(app_user_boot)]
fn reset_execve_checkpoint_observation() {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.stage.store(EXECVE_OBS_STAGE_NONE, Ordering::Release);
    obs.failure_stage
        .store(EXECVE_FAIL_STAGE_NONE, Ordering::Release);
    obs.failure_reason
        .store(EXECVE_FAIL_REASON_NONE, Ordering::Release);
    obs.failure_detail.store(0, Ordering::Release);
    obs.failure_free_pages.store(0, Ordering::Release);
    obs.failure_totalram_pages.store(0, Ordering::Release);
    obs.filename_len.store(0, Ordering::Release);
    obs.argv0_len.store(0, Ordering::Release);
    obs.argv_argc.store(0, Ordering::Release);
    obs.argv_total_bytes.store(0, Ordering::Release);
    obs.argv_capacity_exceeded.store(0, Ordering::Release);
    obs.main_elf_type.store(0, Ordering::Release);
    obs.main_input_len.store(0, Ordering::Release);
    obs.main_load_bias.store(0, Ordering::Release);
    obs.main_entry.store(0, Ordering::Release);
    obs.main_runtime_entry.store(0, Ordering::Release);
    obs.main_segments.store(0, Ordering::Release);
    obs.main_interpreter_required.store(0, Ordering::Release);
    obs.interpreter_path_len.store(0, Ordering::Release);
    obs.interpreter_input_len.store(0, Ordering::Release);
    obs.interpreter_load_bias.store(0, Ordering::Release);
    obs.interpreter_entry.store(0, Ordering::Release);
    obs.interpreter_segments.store(0, Ordering::Release);
    obs.address_space_state.store(0, Ordering::Release);
    obs.mapping_count.store(0, Ordering::Release);
    obs.segment_mapping_count.store(0, Ordering::Release);
    obs.satp_token.store(0, Ordering::Release);
    obs.trap_entry.store(0, Ordering::Release);
    obs.trap_sp.store(0, Ordering::Release);
    obs.trap_sstatus.store(0, Ordering::Release);
    obs.old_satp.store(0, Ordering::Release);
    obs.new_satp.store(0, Ordering::Release);
    obs.current_satp.store(0, Ordering::Release);
    obs.frame_before_sepc.store(0, Ordering::Release);
    obs.frame_before_sp.store(0, Ordering::Release);
    obs.frame_before_ra.store(0, Ordering::Release);
    obs.frame_before_sstatus.store(0, Ordering::Release);
    obs.frame_after_sepc.store(0, Ordering::Release);
    obs.frame_after_sp.store(0, Ordering::Release);
    obs.frame_after_ra.store(0, Ordering::Release);
    obs.frame_after_sstatus.store(0, Ordering::Release);
    obs.kernel_sp.store(0, Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_execve_stage(stage: usize) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.current_satp
        .store(crate::arch::riscv64::csr::read_satp(), Ordering::Release);
    obs.kernel_sp
        .store(crate::arch::riscv64::csr::read_sp(), Ordering::Release);
    obs.stage.store(stage, Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_execve_failure(stage: usize, reason: usize) {
    record_execve_failure_detail(stage, reason, 0, 0, 0);
}

#[cfg(app_user_boot)]
fn record_execve_failure_detail(
    stage: usize,
    reason: usize,
    detail: usize,
    free_pages: usize,
    totalram_pages: usize,
) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.current_satp
        .store(crate::arch::riscv64::csr::read_satp(), Ordering::Release);
    obs.kernel_sp
        .store(crate::arch::riscv64::csr::read_sp(), Ordering::Release);
    obs.failure_stage.store(stage, Ordering::Release);
    obs.failure_reason.store(reason, Ordering::Release);
    obs.failure_detail.store(detail, Ordering::Release);
    obs.failure_free_pages.store(free_pages, Ordering::Release);
    obs.failure_totalram_pages
        .store(totalram_pages, Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_execve_main_image_read(len: usize) {
    EXECVE_CHECKPOINT_OBSERVATION
        .main_input_len
        .store(len, Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_execve_interpreter_path(path: &[u8]) {
    EXECVE_CHECKPOINT_OBSERVATION
        .interpreter_path_len
        .store(path.len(), Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_execve_interpreter_image_read(len: usize) {
    EXECVE_CHECKPOINT_OBSERVATION
        .interpreter_input_len
        .store(len, Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_execve_args(
    filename_len: usize,
    argv0_len: usize,
    argv_argc: usize,
    argv_total_bytes: usize,
    argv_capacity_exceeded: bool,
    frame: &TrapFrame,
) {
    reset_execve_checkpoint_observation();
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.filename_len.store(filename_len, Ordering::Release);
    obs.argv0_len.store(argv0_len, Ordering::Release);
    obs.argv_argc.store(argv_argc, Ordering::Release);
    obs.argv_total_bytes
        .store(argv_total_bytes, Ordering::Release);
    obs.argv_capacity_exceeded.store(
        if argv_capacity_exceeded { 1 } else { 0 },
        Ordering::Release,
    );
    obs.frame_before_sepc.store(frame.sepc, Ordering::Release);
    obs.frame_before_sp.store(frame.reg(2), Ordering::Release);
    obs.frame_before_ra.store(frame.reg(1), Ordering::Release);
    obs.frame_before_sstatus
        .store(frame.sstatus, Ordering::Release);
    record_execve_stage(EXECVE_OBS_STAGE_ARGS_READY);
}

#[cfg(app_user_boot)]
fn record_execve_main_elf(elf: &ElfObject) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.main_elf_type
        .store(elf.elf_type_index(), Ordering::Release);
    obs.main_input_len.store(elf.input_len(), Ordering::Release);
    obs.main_load_bias.store(elf.load_bias(), Ordering::Release);
    obs.main_entry.store(elf.entry(), Ordering::Release);
    obs.main_runtime_entry
        .store(elf.runtime_entry(), Ordering::Release);
    obs.main_segments
        .store(elf.load_segment_count(), Ordering::Release);
    obs.main_interpreter_required
        .store(elf.interpreter_required() as usize, Ordering::Release);
    record_execve_stage(EXECVE_OBS_STAGE_MAIN_ELF_READY);
}

#[cfg(app_user_boot)]
fn record_execve_interpreter(interpreter: &ElfObject) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.interpreter_input_len
        .store(interpreter.input_len(), Ordering::Release);
    obs.interpreter_load_bias
        .store(interpreter.load_bias(), Ordering::Release);
    obs.interpreter_entry
        .store(interpreter.entry(), Ordering::Release);
    obs.interpreter_segments
        .store(interpreter.load_segment_count(), Ordering::Release);
    record_execve_stage(EXECVE_OBS_STAGE_INTERPRETER_READY);
}

#[cfg(app_user_boot)]
fn record_execve_address_space(stage: usize, address_space: &UserAddressSpace) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.address_space_state
        .store(address_space.state() as usize, Ordering::Release);
    obs.mapping_count
        .store(address_space.mapping_count(), Ordering::Release);
    obs.segment_mapping_count
        .store(address_space.segment_mapping_count(), Ordering::Release);
    obs.satp_token
        .store(address_space.satp_token(), Ordering::Release);
    obs.new_satp
        .store(address_space.satp_token(), Ordering::Release);
    record_execve_stage(stage);
}

#[cfg(app_user_boot)]
fn record_execve_trap_frame(trap_frame: &UserTrapFrame) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.trap_entry.store(trap_frame.entry(), Ordering::Release);
    obs.trap_sp.store(trap_frame.sp(), Ordering::Release);
    obs.trap_sstatus
        .store(trap_frame.sstatus(), Ordering::Release);
    record_execve_stage(EXECVE_OBS_STAGE_TRAP_FRAME_READY);
}

#[cfg(app_user_boot)]
fn record_execve_context_replaced(old_satp: usize, new_satp: usize) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.old_satp.store(old_satp, Ordering::Release);
    obs.new_satp.store(new_satp, Ordering::Release);
    record_execve_stage(EXECVE_OBS_STAGE_CONTEXT_REPLACED);
}

#[cfg(app_user_boot)]
fn record_execve_return_frame(frame: &TrapFrame) {
    let obs = &EXECVE_CHECKPOINT_OBSERVATION;
    obs.frame_after_sepc.store(frame.sepc, Ordering::Release);
    obs.frame_after_sp.store(frame.reg(2), Ordering::Release);
    obs.frame_after_ra.store(frame.reg(1), Ordering::Release);
    obs.frame_after_sstatus
        .store(frame.sstatus, Ordering::Release);
    record_execve_stage(EXECVE_OBS_STAGE_RETURN_FRAME_READY);
}

#[cfg(app_user_boot)]
fn replace_current_user_exec_image(
    filename: &[u8],
    argv: &[&[u8]],
    frame: &mut TrapFrame,
) -> Result<(), ExecveFirstSliceError> {
    let ctx = crate::context::context();
    let image = super::user_boot::read_runtime_exec_path_image(
        &mut ctx.vfs_core,
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &ctx.kernel_image,
        filename,
        false,
    )
    .map_err(|_| {
        record_execve_failure(EXECVE_FAIL_STAGE_PATH_READ, EXECVE_FAIL_REASON_VFS_READ);
        ExecveFirstSliceError::NotFound
    })?;
    record_execve_main_image_read(image.len());

    let mut new_elf = ElfObject::new();
    if let Err(error) = new_elf.preset_from_vfs(image) {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_MAIN_PRESET,
            EXECVE_FAIL_REASON_UNSUPPORTED_ELF,
            error.index(),
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        return Err(ExecveFirstSliceError::Unsupported);
    }
    if let Err(error) = new_elf.setup(image) {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_MAIN_SETUP,
            EXECVE_FAIL_REASON_UNSUPPORTED_ELF,
            error.index(),
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        return Err(ExecveFirstSliceError::Unsupported);
    }
    record_execve_main_elf(&new_elf);
    crate::checkpoint::dispatch(Checkpoint::UserExecMainElfReady, ctx);

    let mut new_interpreter = ElfObject::new();
    let interpreter_image = if let Some(interpreter_path) = new_elf.interpreter_path() {
        record_execve_interpreter_path(interpreter_path);
        let image = super::user_boot::read_runtime_exec_path_image(
            &mut ctx.vfs_core,
            &ctx.fs_struct,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            interpreter_path,
            true,
        )
        .map_err(|_| {
            record_execve_failure(
                EXECVE_FAIL_STAGE_INTERPRETER_READ,
                EXECVE_FAIL_REASON_INTERPRETER_READ,
            );
            ExecveFirstSliceError::Unsupported
        })?;
        record_execve_interpreter_image_read(image.len());
        if let Err(error) = new_interpreter.preset_interpreter_from_vfs(image) {
            record_execve_failure_detail(
                EXECVE_FAIL_STAGE_INTERPRETER_PRESET,
                EXECVE_FAIL_REASON_UNSUPPORTED_ELF,
                error.index(),
                ctx.page_allocator.buddy_total_free_pages(),
                ctx.page_allocator.totalram_pages(),
            );
            return Err(ExecveFirstSliceError::Unsupported);
        }
        if let Err(error) = new_interpreter.setup(image) {
            record_execve_failure_detail(
                EXECVE_FAIL_STAGE_INTERPRETER_SETUP,
                EXECVE_FAIL_REASON_UNSUPPORTED_ELF,
                error.index(),
                ctx.page_allocator.buddy_total_free_pages(),
                ctx.page_allocator.totalram_pages(),
            );
            return Err(ExecveFirstSliceError::Unsupported);
        }
        if let Err(error) = new_elf.bind_runtime_interpreter(&new_interpreter) {
            record_execve_failure_detail(
                EXECVE_FAIL_STAGE_INTERPRETER_BIND,
                EXECVE_FAIL_REASON_INVALID_STATE,
                error.index(),
                ctx.page_allocator.buddy_total_free_pages(),
                ctx.page_allocator.totalram_pages(),
            );
            return Err(ExecveFirstSliceError::Unsupported);
        }
        record_execve_interpreter(&new_interpreter);
        crate::checkpoint::dispatch(Checkpoint::UserExecInterpreterReady, ctx);
        Some(image)
    } else {
        None
    };
    let interpreter_ref = interpreter_image.map(|_| &new_interpreter);

    ctx.user_exec_staging_address_space
        .preset(
            ctx.vm.swapper_vm(),
            &ctx.page_allocator,
            &ctx.kernel_global_allocator,
            &ctx.kernel_init_task,
        )
        .map_err(|_| {
            record_execve_failure(
                EXECVE_FAIL_STAGE_ADDRESS_SPACE_PRESET,
                EXECVE_FAIL_REASON_ADDRESS_SPACE,
            );
            discard_execve_staging(ctx);
            ExecveFirstSliceError::Unsupported
        })?;

    let mut new_stack = UserStack::new();
    if new_stack
        .setup(
            &ctx.user_exec_staging_address_space,
            &new_elf,
            interpreter_ref,
            argv,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
        .is_err()
    {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_STACK_SETUP,
            EXECVE_FAIL_REASON_STACK,
            0,
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        discard_execve_staging(ctx);
        return Err(ExecveFirstSliceError::Unsupported);
    }
    if let Err(error) = ctx.user_exec_staging_address_space.setup(
        &new_elf,
        interpreter_ref,
        &new_stack,
        image,
        interpreter_image,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    ) {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_ADDRESS_SPACE_SETUP,
            EXECVE_FAIL_REASON_ADDRESS_SPACE,
            error.index(),
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        discard_execve_staging(ctx);
        return Err(ExecveFirstSliceError::Unsupported);
    }
    record_execve_address_space(
        EXECVE_OBS_STAGE_ADDRESS_SPACE_READY,
        &ctx.user_exec_staging_address_space,
    );
    crate::checkpoint::dispatch(Checkpoint::UserExecAddressSpaceReady, ctx);

    let mut new_trap_frame = UserTrapFrame::new();
    if new_trap_frame
        .setup(&ctx.user_exec_staging_address_space, &new_elf, &new_stack)
        .is_err()
    {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_TRAP_FRAME_SETUP,
            EXECVE_FAIL_REASON_TRAP_FRAME,
            0,
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        discard_execve_staging(ctx);
        return Err(ExecveFirstSliceError::Unsupported);
    }
    if new_elf
        .enable(
            &ctx.user_exec_staging_address_space,
            &new_stack,
            &new_trap_frame,
        )
        .is_err()
    {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_ELF_ENABLE,
            EXECVE_FAIL_REASON_INVALID_STATE,
            0,
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        discard_execve_staging(ctx);
        return Err(ExecveFirstSliceError::Unsupported);
    }
    if let Err(error) = ctx.user_exec_staging_address_space.enable(
        &new_trap_frame,
        ctx.vm.swapper_vm(),
        &ctx.kernel_image,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    ) {
        record_execve_failure_detail(
            EXECVE_FAIL_STAGE_ADDRESS_SPACE_ENABLE,
            EXECVE_FAIL_REASON_ADDRESS_SPACE,
            error.index(),
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
        discard_execve_staging(ctx);
        return Err(ExecveFirstSliceError::Unsupported);
    }

    let entry = new_trap_frame.entry();
    let sp = new_trap_frame.sp();
    let sstatus = new_trap_frame.sstatus();
    let old_satp = crate::arch::riscv64::csr::read_satp();
    let satp = ctx.user_exec_staging_address_space.satp_token();
    let close_on_exec_report = ctx.files_struct.close_on_exec().map_err(|_| {
        record_execve_failure(
            EXECVE_FAIL_STAGE_CLOSE_ON_EXEC,
            EXECVE_FAIL_REASON_CLOSE_ON_EXEC,
        );
        ExecveFirstSliceError::Unsupported
    })?;
    print_execve_close_on_exec_report(close_on_exec_report);
    ctx.elf_object = new_elf;
    ctx.elf_interpreter_object = new_interpreter;
    ctx.user_stack = new_stack;
    commit_execve_staging_address_space(ctx);
    record_execve_context_replaced(old_satp, satp);
    crate::checkpoint::dispatch(Checkpoint::UserExecContextReplaced, ctx);
    record_execve_address_space(EXECVE_OBS_STAGE_SATP_READY, &ctx.user_address_space);
    crate::checkpoint::dispatch(Checkpoint::UserExecSatpReady, ctx);
    ctx.user_trap_frame = new_trap_frame;
    record_execve_trap_frame(&ctx.user_trap_frame);
    crate::checkpoint::dispatch(Checkpoint::UserExecTrapFrameReady, ctx);

    crate::arch::riscv64::csr::write_satp(satp);
    crate::arch::riscv64::csr::sfence_vma();
    record_execve_stage(EXECVE_OBS_STAGE_SATP_SWITCHED);
    crate::checkpoint::dispatch(Checkpoint::UserExecSatpSwitched, ctx);
    reset_frame_for_execve_start_thread(frame, entry, sp, sstatus);
    record_execve_return_frame(frame);
    crate::checkpoint::dispatch(Checkpoint::UserExecReturnFrameReady, ctx);
    Ok(())
}

#[cfg(app_user_boot)]
fn commit_execve_staging_address_space(ctx: &mut crate::context::Context) {
    let staging = &ctx.user_exec_staging_address_space as *const UserAddressSpace;
    let current = &mut ctx.user_address_space as *mut UserAddressSpace;
    // Avoid a whole-UserAddressSpace stack temporary on the trap/syscall stack.
    unsafe {
        core::ptr::copy_nonoverlapping(staging, current, 1);
    }
    ctx.user_exec_staging_address_space
        .reset_staging_after_exec_commit();
}

#[cfg(app_user_boot)]
fn discard_execve_staging(ctx: &mut crate::context::Context) {
    ctx.user_exec_staging_address_space
        .discard_staging_after_exec_failure(&mut ctx.page_allocator, &ctx.page_metadata_map);
}

#[cfg(app_user_boot)]
fn reset_frame_for_execve_start_thread(
    frame: &mut TrapFrame,
    entry: usize,
    sp: usize,
    sstatus: usize,
) {
    let mut index = 1usize;
    while index < 32 {
        frame.set_reg(index, 0);
        index += 1;
    }
    frame.set_reg(2, sp);
    frame.sstatus = sstatus;
    frame.sepc = entry;
    frame.stval = 0;
}

#[cfg(app_user_boot)]
fn record_wait4_parent_wait_saved(
    parent_frame: &TrapFrame,
    status_ptr: usize,
    child_pid: usize,
    stack_window_saved: bool,
    stack_window_start: usize,
    stack_window_len: usize,
    writable_pages_saved: bool,
    writable_pages_count: usize,
    writable_pages_truncated: bool,
) {
    let obs = &WAIT4_CHECKPOINT_OBSERVATION;
    obs.saved_sepc.store(parent_frame.sepc, Ordering::Release);
    obs.saved_sp.store(parent_frame.reg(2), Ordering::Release);
    obs.saved_s2.store(parent_frame.reg(18), Ordering::Release);
    obs.saved_s4.store(parent_frame.reg(20), Ordering::Release);
    obs.saved_a7.store(parent_frame.reg(17), Ordering::Release);
    obs.saved_satp
        .store(crate::arch::riscv64::csr::read_satp(), Ordering::Release);
    obs.saved_status_ptr.store(status_ptr, Ordering::Release);
    obs.saved_child_pid.store(child_pid, Ordering::Release);
    obs.stack_window_saved
        .store(stack_window_saved as usize, Ordering::Release);
    obs.stack_window_start
        .store(stack_window_start, Ordering::Release);
    obs.stack_window_len
        .store(stack_window_len, Ordering::Release);
    obs.writable_pages_saved
        .store(writable_pages_saved as usize, Ordering::Release);
    obs.writable_pages_count
        .store(writable_pages_count, Ordering::Release);
    obs.writable_pages_truncated
        .store(writable_pages_truncated as usize, Ordering::Release);
    obs.writable_pages_restored.store(0, Ordering::Release);
    obs.saved.store(1, Ordering::Release);
}

#[cfg(app_user_boot)]
fn record_wait4_parent_wait_resumed(
    parent_frame: &TrapFrame,
    parent_satp: usize,
    status_ptr: usize,
    wait_status: usize,
    status_copied: bool,
    child_exit_status: usize,
    stack_window_compared: bool,
    stack_window_diff_count: usize,
    stack_window_first_diff_addr: usize,
    stack_window_before_byte: usize,
    stack_window_after_byte: usize,
    writable_pages_compared: bool,
    writable_pages_dirty_count: usize,
    writable_pages_stack_dirty_count: usize,
    writable_pages_non_stack_dirty_count: usize,
    writable_pages_restored: bool,
    writable_pages_first_non_stack_kind: usize,
    writable_pages_first_non_stack_mapping_index: usize,
    writable_pages_first_non_stack_page_index: usize,
    writable_pages_first_non_stack_addr: usize,
    writable_pages_first_non_stack_before_checksum: usize,
    writable_pages_first_non_stack_after_checksum: usize,
) {
    let obs = &WAIT4_CHECKPOINT_OBSERVATION;
    obs.resumed_sepc.store(parent_frame.sepc, Ordering::Release);
    obs.resumed_sp.store(parent_frame.reg(2), Ordering::Release);
    obs.resumed_s2
        .store(parent_frame.reg(18), Ordering::Release);
    obs.resumed_s4
        .store(parent_frame.reg(20), Ordering::Release);
    obs.resumed_a0
        .store(parent_frame.reg(10), Ordering::Release);
    obs.resumed_a7
        .store(parent_frame.reg(17), Ordering::Release);
    obs.resumed_satp.store(parent_satp, Ordering::Release);
    obs.resumed_status_ptr.store(status_ptr, Ordering::Release);
    obs.resumed_wait_status
        .store(wait_status, Ordering::Release);
    obs.resumed_status_copied
        .store(status_copied as usize, Ordering::Release);
    obs.child_exit_status
        .store(child_exit_status, Ordering::Release);
    obs.stack_window_compared
        .store(stack_window_compared as usize, Ordering::Release);
    obs.stack_window_diff_count
        .store(stack_window_diff_count, Ordering::Release);
    obs.stack_window_first_diff_addr
        .store(stack_window_first_diff_addr, Ordering::Release);
    obs.stack_window_before_byte
        .store(stack_window_before_byte, Ordering::Release);
    obs.stack_window_after_byte
        .store(stack_window_after_byte, Ordering::Release);
    obs.writable_pages_compared
        .store(writable_pages_compared as usize, Ordering::Release);
    obs.writable_pages_dirty_count
        .store(writable_pages_dirty_count, Ordering::Release);
    obs.writable_pages_stack_dirty_count
        .store(writable_pages_stack_dirty_count, Ordering::Release);
    obs.writable_pages_non_stack_dirty_count
        .store(writable_pages_non_stack_dirty_count, Ordering::Release);
    obs.writable_pages_restored
        .store(writable_pages_restored as usize, Ordering::Release);
    obs.writable_pages_first_non_stack_kind
        .store(writable_pages_first_non_stack_kind, Ordering::Release);
    obs.writable_pages_first_non_stack_mapping_index.store(
        writable_pages_first_non_stack_mapping_index,
        Ordering::Release,
    );
    obs.writable_pages_first_non_stack_page_index
        .store(writable_pages_first_non_stack_page_index, Ordering::Release);
    obs.writable_pages_first_non_stack_addr
        .store(writable_pages_first_non_stack_addr, Ordering::Release);
    obs.writable_pages_first_non_stack_before_checksum.store(
        writable_pages_first_non_stack_before_checksum,
        Ordering::Release,
    );
    obs.writable_pages_first_non_stack_after_checksum.store(
        writable_pages_first_non_stack_after_checksum,
        Ordering::Release,
    );
    obs.resumed.store(1, Ordering::Release);
}

fn syscall_table_wait4(table: &SyscallTable, frame: &mut TrapFrame) {
    let upid = frame.reg(10);
    let stat_addr = frame.reg(11);
    let options = frame.reg(12);
    let rusage = frame.reg(13);

    if upid != USER_WAIT4_ALL_CHILDREN || rusage != 0 {
        complete_unsupported_syscall(frame);
        return;
    }
    if options & !WAIT4_LINUX_VALID_OPTIONS != 0 {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    if let Some((child_pid, _exit_status, wait_status, _pidfd_fd)) = crate::context::context_ref()
        .user_child_process
        .first_unreaped_completed_child()
    {
        let status_copied = stat_addr == 0 || write_user_u32(stat_addr, wait_status as u32);
        table.wait4_observed.store(1, Ordering::Release);
        crate::checkpoint::dispatch(Checkpoint::SyscallTableWait4, crate::context::context_ref());
        if !status_copied {
            complete_error_syscall(frame, EFAULT);
            return;
        }
        {
            let ctx = crate::context::context();
            if !ctx
                .user_child_process
                .mark_completed_child_reaped(child_pid)
            {
                complete_unsupported_syscall(frame);
                return;
            }
        }
        crate::checkpoint::dispatch(
            Checkpoint::UserChildRecordReaped,
            crate::context::context_ref(),
        );
        complete_successful_syscall(frame, child_pid);
        return;
    }

    let child_eligible_but_not_waitable = {
        let child = &crate::context::context_ref().user_child_process;
        child.state() == State::Ready
            && child.enqueued()
            && !child.child_exit_status_observed()
            && !child.parent_wait_resumed()
    };
    if options & WAIT4_WNOHANG != 0 {
        table.wait4_observed.store(1, Ordering::Release);
        crate::checkpoint::dispatch(Checkpoint::SyscallTableWait4, crate::context::context_ref());
        if child_eligible_but_not_waitable {
            complete_successful_syscall(frame, 0);
        } else {
            complete_error_syscall(frame, ECHILD);
        }
        return;
    }
    if !child_eligible_but_not_waitable {
        table.wait4_observed.store(1, Ordering::Release);
        crate::checkpoint::dispatch(Checkpoint::SyscallTableWait4, crate::context::context_ref());
        complete_error_syscall(frame, ECHILD);
        return;
    }
    if options != USER_WAIT4_WUNTRACED {
        complete_unsupported_syscall(frame);
        return;
    }

    let child_frame = {
        let ctx = crate::context::context();
        let Some(child_frame) = ctx.user_child_process.wait4_yield_to_child_continuation(
            &ctx.user_init_process,
            &ctx.user_address_space,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            frame,
            stat_addr,
            upid,
            options,
            rusage,
        ) else {
            complete_error_syscall(frame, ECHILD);
            return;
        };
        child_frame
    };

    table.wait4_observed.store(1, Ordering::Release);
    #[cfg(app_user_boot)]
    {
        let child = &crate::context::context_ref().user_child_process;
        record_wait4_parent_wait_saved(
            frame,
            stat_addr,
            child.pid(),
            child.parent_wait_stack_window_checkpoint_bound(),
            child.parent_wait_stack_window_start(),
            child.parent_wait_stack_window_len(),
            child.parent_wait_writable_page_snapshot_saved(),
            child.parent_wait_writable_page_count(),
            child.parent_wait_writable_page_snapshot_truncated(),
        );
    }
    crate::checkpoint::dispatch(Checkpoint::SyscallTableWait4, crate::context::context_ref());
    print_wait4_child_handoff_trace(&child_frame);
    *frame = child_frame;
}

fn syscall_table_set_tid_address(table: &SyscallTable, frame: &mut TrapFrame) {
    let tidptr = frame.reg(10);
    let pid = crate::context::context()
        .user_init_process
        .set_clear_child_tid(tidptr);
    if pid == 0 {
        complete_unsupported_syscall(frame);
        return;
    }

    table.set_tid_address_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(
        Checkpoint::SyscallTableSetTidAddress,
        crate::context::context_ref(),
    );
    complete_successful_syscall(frame, pid);
}

fn syscall_table_exit(table: &SyscallTable, frame: &mut TrapFrame) {
    table.exit_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableExit, crate::context::context_ref());
    let status = frame.reg(10);
    print_syscall_trace_exit(frame, status);
    if complete_child_exit_to_vfork_parent_clone(frame, status) {
        return;
    }
    if complete_child_exit_to_parent_wait(frame, status) {
        return;
    }
    crate::arch::riscv64::sbi::putstr("user exit status=");
    print_decimal(status);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

#[cfg(app_user_boot)]
fn complete_child_exit_to_vfork_parent_clone(frame: &mut TrapFrame, status: usize) -> bool {
    let (mut parent_frame, child_pid, parent_satp) = {
        let ctx = crate::context::context();
        let Some((parent_frame, child_pid)) = ctx.user_child_process.child_exit_to_vfork_parent(
            &mut ctx.user_address_space,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
            status,
        ) else {
            return false;
        };
        (parent_frame, child_pid, ctx.user_address_space.satp_token())
    };

    let has_pidfd = crate::context::context_ref().user_child_process.pidfd_fd() != usize::MAX;
    if has_pidfd {
        let ctx = crate::context::context();
        if ctx
            .files_struct
            .mark_pidfd_child_exited(child_pid, status)
            .is_err()
        {
            return false;
        }
    }
    crate::arch::riscv64::csr::write_satp(parent_satp);
    crate::arch::riscv64::csr::sfence_vma();

    let writable_pages_restored = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .restore_parent_wait_writable_page_snapshot(
                &ctx.user_address_space,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
    };
    if !writable_pages_restored {
        return false;
    }

    let fd_snapshot_restored = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .restore_parent_fd_snapshot(&mut ctx.files_struct)
    };
    if !fd_snapshot_restored {
        return false;
    }

    complete_successful_syscall(&mut parent_frame, child_pid);
    {
        let ctx = crate::context::context();
        if !ctx.user_child_process.mark_vfork_parent_resumed() {
            return false;
        }
    }

    *frame = parent_frame;
    crate::checkpoint::dispatch(
        Checkpoint::UserCloneVforkParentResumed,
        crate::context::context_ref(),
    );
    {
        let ctx = crate::context::context();
        if !ctx.user_child_process.archive_completed_child_record() {
            return false;
        }
    }
    crate::checkpoint::dispatch(
        Checkpoint::UserChildRecordArchived,
        crate::context::context_ref(),
    );

    let signal_wait_woken = {
        let ctx = crate::context::context();
        ctx.user_init_process
            .record_child_exit_sigchld()
            .unwrap_or(false)
    };
    if signal_wait_woken {
        crate::checkpoint::dispatch(
            Checkpoint::UserSignalWaitWakeSigchld,
            crate::context::context_ref(),
        );
    }
    if has_pidfd {
        crate::checkpoint::dispatch(Checkpoint::UserPidfdReady, crate::context::context_ref());
    }

    {
        let ctx = crate::context::context();
        if ctx
            .scheduler
            .dequeue_user_child_from_runqueue(&ctx.cpu_group)
            .is_err()
            || !ctx.user_child_process.mark_active_slot_reusable()
        {
            return false;
        }
    }
    crate::checkpoint::dispatch(
        Checkpoint::UserChildSlotReusable,
        crate::context::context_ref(),
    );
    true
}

#[cfg(not(app_user_boot))]
fn complete_child_exit_to_vfork_parent_clone(_frame: &mut TrapFrame, _status: usize) -> bool {
    false
}

#[cfg(app_user_boot)]
fn complete_child_exit_to_parent_wait(frame: &mut TrapFrame, status: usize) -> bool {
    let wait_status = ((status & 0xff) << 8) as u32;
    let (mut parent_frame, status_ptr, child_pid, parent_satp) = {
        let ctx = crate::context::context();
        let Some((parent_frame, status_ptr, child_pid)) =
            ctx.user_child_process.child_exit_to_parent_wait(
                &mut ctx.user_address_space,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
                status,
            )
        else {
            return false;
        };
        (
            parent_frame,
            status_ptr,
            child_pid,
            ctx.user_address_space.satp_token(),
        )
    };

    let signal_wait_woken = {
        let ctx = crate::context::context();
        ctx.user_init_process
            .record_child_exit_sigchld()
            .unwrap_or(false)
    };
    if signal_wait_woken {
        crate::checkpoint::dispatch(
            Checkpoint::UserSignalWaitWakeSigchld,
            crate::context::context_ref(),
        );
    }

    crate::arch::riscv64::csr::write_satp(parent_satp);
    crate::arch::riscv64::csr::sfence_vma();

    let stack_window_compared = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .compare_parent_wait_stack_window(&ctx.user_address_space, &ctx.page_metadata_map)
    };

    let writable_pages_compared = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .compare_parent_wait_writable_pages(&ctx.user_address_space, &ctx.page_metadata_map)
    };

    let parent_stack_restored = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .restore_parent_wait_stack_snapshot(&ctx.user_address_space, &ctx.page_metadata_map)
    };
    if !parent_stack_restored {
        return false;
    }

    let writable_pages_restored = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .restore_parent_wait_writable_page_snapshot(
                &ctx.user_address_space,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
    };
    if !writable_pages_restored {
        return false;
    }

    let fd_snapshot_restored = {
        let ctx = crate::context::context();
        ctx.user_child_process
            .restore_parent_fd_snapshot(&mut ctx.files_struct)
    };
    if !fd_snapshot_restored {
        return false;
    }

    let status_copied = status_ptr == 0 || write_user_u32(status_ptr, wait_status);
    {
        let ctx = crate::context::context();
        if !ctx
            .user_child_process
            .mark_parent_wait_resumed(status_copied)
        {
            return false;
        }
    }
    let (
        stack_window_diff_count,
        stack_window_first_diff_addr,
        stack_window_before_byte,
        stack_window_after_byte,
        writable_pages_dirty_count,
        writable_pages_stack_dirty_count,
        writable_pages_non_stack_dirty_count,
        writable_pages_restored,
        writable_pages_first_non_stack_kind,
        writable_pages_first_non_stack_mapping_index,
        writable_pages_first_non_stack_page_index,
        writable_pages_first_non_stack_addr,
        writable_pages_first_non_stack_before_checksum,
        writable_pages_first_non_stack_after_checksum,
    ) = {
        let child = &crate::context::context_ref().user_child_process;
        (
            child.parent_wait_stack_window_diff_count(),
            child.parent_wait_stack_window_first_diff_addr(),
            child.parent_wait_stack_window_before_byte() as usize,
            child.parent_wait_stack_window_after_byte() as usize,
            child.parent_wait_writable_page_dirty_count(),
            child.parent_wait_writable_page_stack_dirty_count(),
            child.parent_wait_writable_page_non_stack_dirty_count(),
            child.parent_wait_writable_page_snapshot_restored(),
            child.parent_wait_first_non_stack_dirty_kind(),
            child.parent_wait_first_non_stack_dirty_mapping_index(),
            child.parent_wait_first_non_stack_dirty_page_index(),
            child.parent_wait_first_non_stack_dirty_addr(),
            child.parent_wait_first_non_stack_dirty_before_checksum(),
            child.parent_wait_first_non_stack_dirty_after_checksum(),
        )
    };

    if status_copied {
        complete_successful_syscall(&mut parent_frame, child_pid);
    } else {
        complete_error_syscall(&mut parent_frame, EFAULT);
    }
    record_wait4_parent_wait_resumed(
        &parent_frame,
        parent_satp,
        status_ptr,
        wait_status as usize,
        status_copied,
        status,
        stack_window_compared,
        stack_window_diff_count,
        stack_window_first_diff_addr,
        stack_window_before_byte,
        stack_window_after_byte,
        writable_pages_compared,
        writable_pages_dirty_count,
        writable_pages_stack_dirty_count,
        writable_pages_non_stack_dirty_count,
        writable_pages_restored,
        writable_pages_first_non_stack_kind,
        writable_pages_first_non_stack_mapping_index,
        writable_pages_first_non_stack_page_index,
        writable_pages_first_non_stack_addr,
        writable_pages_first_non_stack_before_checksum,
        writable_pages_first_non_stack_after_checksum,
    );
    *frame = parent_frame;
    crate::checkpoint::dispatch(
        Checkpoint::UserChildParentWaitResumed,
        crate::context::context_ref(),
    );
    true
}

#[cfg(not(app_user_boot))]
fn complete_child_exit_to_parent_wait(_frame: &mut TrapFrame, _status: usize) -> bool {
    false
}

fn copy_from_user(user_ptr: usize, dst: &mut [u8]) -> bool {
    if dst.is_empty() {
        return true;
    }
    if !user_copy_range_accessible(user_ptr, dst.len(), UserFaultAccess::Load) {
        return false;
    }

    let saved = crate::arch::riscv64::csr::save_and_enable_user_memory_access();
    let mut index = 0usize;
    while index < dst.len() {
        dst[index] = unsafe { core::ptr::read_volatile((user_ptr + index) as *const u8) };
        index += 1;
    }
    crate::arch::riscv64::csr::restore_user_memory_access(saved);
    true
}

fn copy_to_user(user_ptr: usize, src: &[u8]) -> bool {
    if src.is_empty() {
        return true;
    }
    if !user_copy_range_accessible(user_ptr, src.len(), UserFaultAccess::Store) {
        return false;
    }

    let saved = crate::arch::riscv64::csr::save_and_enable_user_memory_access();
    let mut index = 0usize;
    while index < src.len() {
        unsafe { core::ptr::write_volatile((user_ptr + index) as *mut u8, src[index]) };
        index += 1;
    }
    crate::arch::riscv64::csr::restore_user_memory_access(saved);
    true
}

fn user_copy_range_accessible(user_ptr: usize, len: usize, access: UserFaultAccess) -> bool {
    if len == 0 || user_ptr == 0 {
        return len == 0;
    }
    let Some(end) = user_ptr.checked_add(len) else {
        return false;
    };
    let Some(last) = end.checked_sub(1) else {
        return false;
    };

    let space = &crate::context::context_ref().user_address_space;
    if !space.user_range_mapped(user_ptr, len) {
        return false;
    }
    let first = space.fault_mapping_diagnostic(user_ptr, access);
    let last = space.fault_mapping_diagnostic(last, access);
    first.permission_satisfied() && last.permission_satisfied()
}

fn read_user_usize(user_ptr: usize) -> Option<usize> {
    let mut bytes = [0u8; core::mem::size_of::<usize>()];
    if copy_from_user(user_ptr, &mut bytes) {
        Some(usize::from_le_bytes(bytes))
    } else {
        None
    }
}

fn read_user_u32(user_ptr: usize) -> Option<u32> {
    let mut bytes = [0u8; core::mem::size_of::<u32>()];
    if copy_from_user(user_ptr, &mut bytes) {
        Some(u32::from_le_bytes(bytes))
    } else {
        None
    }
}

fn read_user_timespec_i64(user_ptr: usize) -> Option<(i64, i64)> {
    let mut bytes = [0u8; TIMESPEC_SIZE];
    if !copy_from_user(user_ptr, &mut bytes) {
        return None;
    }
    let mut sec = [0u8; 8];
    let mut nsec = [0u8; 8];
    sec.copy_from_slice(&bytes[0..8]);
    nsec.copy_from_slice(&bytes[8..16]);
    Some((i64::from_le_bytes(sec), i64::from_le_bytes(nsec)))
}

fn read_user_pollfd(user_ptr: usize) -> Option<UserPollFd> {
    let mut bytes = [0u8; POLLFD_SIZE];
    if !copy_from_user(user_ptr, &mut bytes) {
        return None;
    }
    let mut fd = [0u8; 4];
    let mut events = [0u8; 2];
    fd.copy_from_slice(&bytes[0..4]);
    events.copy_from_slice(&bytes[4..6]);
    Some(UserPollFd {
        fd: i32::from_le_bytes(fd),
        events: u16::from_le_bytes(events),
    })
}

fn write_user_pollfd_revents(user_ptr: usize, revents: u16) -> bool {
    let Some(revents_ptr) = user_ptr.checked_add(6) else {
        return false;
    };
    copy_to_user(revents_ptr, &revents.to_le_bytes())
}

fn write_user_usize(user_ptr: usize, value: usize) -> bool {
    copy_to_user(user_ptr, &value.to_le_bytes())
}

fn read_user_signal_action(user_ptr: usize) -> Option<UserSignalAction> {
    let mut bytes = [0u8; RT_SIGACTION_SIZE];
    if !copy_from_user(user_ptr, &mut bytes) {
        return None;
    }
    Some(UserSignalAction::new(
        read_usize_field(&bytes, 0),
        read_usize_field(&bytes, core::mem::size_of::<usize>()),
        read_usize_field(&bytes, core::mem::size_of::<usize>() * 2),
    ))
}

fn write_user_signal_action(user_ptr: usize, action: UserSignalAction) -> bool {
    let mut bytes = [0u8; RT_SIGACTION_SIZE];
    write_usize_field(&mut bytes, 0, action.handler());
    write_usize_field(&mut bytes, core::mem::size_of::<usize>(), action.flags());
    write_usize_field(&mut bytes, core::mem::size_of::<usize>() * 2, action.mask());
    copy_to_user(user_ptr, &bytes)
}

fn read_usize_field(bytes: &[u8], offset: usize) -> usize {
    let mut field = [0u8; core::mem::size_of::<usize>()];
    field.copy_from_slice(&bytes[offset..offset + core::mem::size_of::<usize>()]);
    usize::from_le_bytes(field)
}

fn write_usize_field(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + core::mem::size_of::<usize>()].copy_from_slice(&value.to_le_bytes());
}

fn valid_rt_signal(signal: usize) -> bool {
    signal >= 1 && signal <= USER_SIGNAL_COUNT
}

fn kernel_only_signal(signal: usize) -> bool {
    signal == SIGKILL || signal == SIGSTOP
}

fn write_user_u32(user_ptr: usize, value: u32) -> bool {
    debug_assert_eq!(UID_T_SIZE, core::mem::size_of::<u32>());
    debug_assert_eq!(GID_T_SIZE, core::mem::size_of::<u32>());
    debug_assert_eq!(PID_T_SIZE, core::mem::size_of::<u32>());
    copy_to_user(user_ptr, &value.to_le_bytes())
}

fn write_uts_field(buffer: &mut [u8; NEW_UTSNAME_SIZE], field: usize, value: &[u8]) {
    let start = field * UTS_FIELD_SIZE;
    let max_len = UTS_FIELD_SIZE - 1;
    let copy_len = core::cmp::min(value.len(), max_len);
    buffer[start..start + copy_len].copy_from_slice(&value[..copy_len]);
}

fn copy_cstr_from_user(user_ptr: usize, dst: &mut [u8]) -> Option<usize> {
    if dst.is_empty() || user_ptr == 0 {
        return None;
    }

    let saved = crate::arch::riscv64::csr::save_and_enable_user_memory_access();
    let mut index = 0usize;
    while index < dst.len() {
        let byte = unsafe { core::ptr::read_volatile((user_ptr + index) as *const u8) };
        if byte == 0 {
            crate::arch::riscv64::csr::restore_user_memory_access(saved);
            return (index != 0).then_some(index);
        }
        dst[index] = byte;
        index += 1;
    }
    crate::arch::riscv64::csr::restore_user_memory_access(saved);
    None
}

#[cfg(app_user_boot)]
fn copy_execve_cstr(user_ptr: usize, dst: &mut [u8]) -> Option<usize> {
    if user_ptr == 0
        || !crate::context::context_ref()
            .user_address_space
            .user_range_mapped(user_ptr, dst.len())
    {
        return None;
    }
    copy_cstr_from_user(user_ptr, dst)
}

#[cfg(app_user_boot)]
struct ExecveArgvCopy {
    bytes: [[u8; USER_PATH_MAX]; USER_EXEC_ARG_MAX],
    lens: [usize; USER_EXEC_ARG_MAX],
    argc: usize,
    total_bytes: usize,
}

#[cfg(app_user_boot)]
impl ExecveArgvCopy {
    const fn new() -> Self {
        Self {
            bytes: [[0u8; USER_PATH_MAX]; USER_EXEC_ARG_MAX],
            lens: [0usize; USER_EXEC_ARG_MAX],
            argc: 0,
            total_bytes: 0,
        }
    }

    fn argv0_len(&self) -> usize {
        if self.argc == 0 {
            0
        } else {
            self.lens[0]
        }
    }
}

#[cfg(app_user_boot)]
enum ExecveArgvCopyError {
    Fault,
    CapacityExceeded(ExecveArgvCopy),
}

#[cfg(app_user_boot)]
fn copy_execve_argv(argv_ptr: usize) -> Result<ExecveArgvCopy, ExecveArgvCopyError> {
    if argv_ptr == 0 {
        return Err(ExecveArgvCopyError::Fault);
    }

    let mut copied = ExecveArgvCopy::new();
    let mut index = 0usize;
    while index < USER_EXEC_ARG_MAX {
        let entry_ptr = argv_ptr
            .checked_add(index * core::mem::size_of::<usize>())
            .ok_or(ExecveArgvCopyError::Fault)?;
        if !crate::context::context_ref()
            .user_address_space
            .user_range_mapped(entry_ptr, core::mem::size_of::<usize>())
        {
            return Err(ExecveArgvCopyError::Fault);
        }
        let arg_ptr = read_user_usize(entry_ptr).ok_or(ExecveArgvCopyError::Fault)?;
        if arg_ptr == 0 {
            if copied.argc == 0 {
                return Err(ExecveArgvCopyError::Fault);
            }
            return Ok(copied);
        }
        let arg_len = copy_execve_cstr(arg_ptr, &mut copied.bytes[index])
            .ok_or(ExecveArgvCopyError::Fault)?;
        copied.lens[index] = arg_len;
        copied.argc += 1;
        copied.total_bytes = copied
            .total_bytes
            .checked_add(arg_len)
            .and_then(|bytes| bytes.checked_add(1))
            .ok_or(ExecveArgvCopyError::Fault)?;
        index += 1;
    }

    let sentinel_ptr = argv_ptr
        .checked_add(USER_EXEC_ARG_MAX * core::mem::size_of::<usize>())
        .ok_or(ExecveArgvCopyError::Fault)?;
    if !crate::context::context_ref()
        .user_address_space
        .user_range_mapped(sentinel_ptr, core::mem::size_of::<usize>())
    {
        return Err(ExecveArgvCopyError::Fault);
    }
    let sentinel = read_user_usize(sentinel_ptr).ok_or(ExecveArgvCopyError::Fault)?;
    if sentinel == 0 {
        Ok(copied)
    } else {
        Err(ExecveArgvCopyError::CapacityExceeded(copied))
    }
}

fn write_linux_stat(buffer: &mut [u8; STAT_SIZE], size: usize, mode: u32) {
    write_u64(buffer, 0, 1);
    write_u64(buffer, 8, 1);
    write_u32(buffer, 16, mode);
    write_u32(buffer, 20, 1);
    write_u64(buffer, 48, size as u64);
    write_u32(buffer, 56, 4096);
    write_u64(buffer, 64, size.div_ceil(512) as u64);
}

fn write_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn write_u16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
}

fn write_u64(buffer: &mut [u8], offset: usize, value: u64) {
    buffer[offset..offset + 8].copy_from_slice(&value.to_ne_bytes());
}

fn breakpoint_exception_handler(frame: &mut TrapFrame) {
    if run_breakpoint_hooks(frame) == BreakpointHookResult::Resume {
        return;
    }

    panic_dispatch("unhandled breakpoint exception\n")
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BreakpointHookResult {
    NotHandled,
    Resume,
}

pub type BreakpointHook = fn(&mut TrapFrame) -> BreakpointHookResult;

static mut BREAKPOINT_HOOKS: [Option<BreakpointHook>; 4] = [None; 4];

pub fn register_breakpoint_hook(hook: BreakpointHook) -> bool {
    unsafe {
        for slot in (&raw mut BREAKPOINT_HOOKS).as_mut().unwrap().iter_mut() {
            if slot.is_none() {
                *slot = Some(hook);
                return true;
            }
        }
    }

    false
}

fn run_breakpoint_hooks(frame: &mut TrapFrame) -> BreakpointHookResult {
    unsafe {
        for hook in (&raw const BREAKPOINT_HOOKS)
            .as_ref()
            .unwrap()
            .iter()
            .flatten()
        {
            match hook(frame) {
                BreakpointHookResult::NotHandled => {}
                BreakpointHookResult::Resume => return BreakpointHookResult::Resume,
            }
        }
    }

    BreakpointHookResult::NotHandled
}

pub fn resume_after_breakpoint(frame: &mut TrapFrame) {
    trace::checkpoint(Checkpoint::BreakpointExceptionHandled);
    frame.sepc = frame
        .sepc
        .wrapping_add(breakpoint_instruction_length(frame.sepc));
}

fn unexpected_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("unexpected exception", frame)
}

fn breakpoint_instruction_length(sepc: usize) -> usize {
    let insn = unsafe { core::ptr::read_unaligned(sepc as *const u16) };
    if insn & 0b11 == 0b11 {
        4
    } else {
        2
    }
}

fn panic_dispatch(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
}

fn panic_dispatch_frame(message: &str, frame: &TrapFrame) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    crate::arch::riscv64::sbi::putstr(" scause=0x");
    print_hex(frame.scause);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    crate::arch::riscv64::sbi::putstr(" sstatus=0x");
    print_hex(frame.sstatus);
    crate::arch::riscv64::sbi::putstr(" a7=");
    print_decimal(frame.reg(17));
    print_syscall_arg_registers(frame);
    print_trap_saved_registers(frame);
    crate::arch::riscv64::sbi::putstr(" gp=0x");
    print_hex(crate::arch::riscv64::csr::read_gp());
    crate::arch::riscv64::sbi::putstr(" tp=0x");
    print_hex(crate::arch::riscv64::csr::read_tp());
    crate::arch::riscv64::sbi::putstr(" frame_sp=0x");
    print_hex(frame.reg(2));
    crate::arch::riscv64::sbi::putstr(" frame_gp=0x");
    print_hex(frame.reg(3));
    crate::arch::riscv64::sbi::putstr(" frame_tp=0x");
    print_hex(frame.reg(4));
    print_user_page_fault_diagnostic(frame);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_wait4_child_handoff_trace(frame: &TrapFrame) {
    crate::arch::riscv64::sbi::putstr("wait4 child handoff sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" sp=0x");
    print_hex(frame.reg(2));
    crate::arch::riscv64::sbi::putstr(" gp=0x");
    print_hex(frame.reg(3));
    crate::arch::riscv64::sbi::putstr(" tp=0x");
    print_hex(frame.reg(4));
    crate::arch::riscv64::sbi::putstr(" a7=");
    print_decimal(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" sstatus=0x");
    print_hex(frame.sstatus);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_trace))]
fn print_wait4_child_handoff_trace(_frame: &TrapFrame) {}

fn print_user_page_fault_diagnostic(frame: &TrapFrame) {
    let cause = frame.scause & !SCAUSE_INTERRUPT_BIT;
    if !PAGE_FAULT_CAUSES.contains(&cause) {
        return;
    }

    let ctx = crate::context::context_ref();
    let space = &ctx.user_address_space;
    let current_satp = crate::arch::riscv64::csr::read_satp();
    let expected_satp = space.satp_token();
    crate::arch::riscv64::sbi::putstr(" user_fault_diag current_satp=0x");
    print_hex(current_satp);
    crate::arch::riscv64::sbi::putstr(" expected_satp=0x");
    print_hex(expected_satp);
    crate::arch::riscv64::sbi::putstr(" satp_match=");
    print_bool_digit(current_satp == expected_satp);

    let fault_access = page_fault_access(frame);
    let sepc_mapping = space.fault_mapping_diagnostic(frame.sepc, UserFaultAccess::Instruction);
    let stval_mapping = space.fault_mapping_diagnostic(frame.stval, fault_access);
    print_fault_mapping_diagnostic(" sepc_map=", sepc_mapping);
    print_fault_mapping_diagnostic(" stval_map=", stval_mapping);
}

fn page_fault_access(frame: &TrapFrame) -> UserFaultAccess {
    match frame.scause & !SCAUSE_INTERRUPT_BIT {
        EXC_INSTRUCTION_PAGE_FAULT => UserFaultAccess::Instruction,
        EXC_LOAD_PAGE_FAULT => UserFaultAccess::Load,
        EXC_STORE_PAGE_FAULT => UserFaultAccess::Store,
        _ => UserFaultAccess::Unknown,
    }
}

fn print_fault_mapping_diagnostic(label: &str, diag: UserFaultMappingDiagnostic) {
    crate::arch::riscv64::sbi::putstr(label);
    print_mapping_kind(diag.kind(), diag.mapped());
    crate::arch::riscv64::sbi::putstr(" addr=0x");
    print_hex(diag.address());
    crate::arch::riscv64::sbi::putstr(" need=");
    print_fault_access(diag.access());
    if diag.mapped() {
        crate::arch::riscv64::sbi::putstr(" range=0x");
        print_hex(diag.start());
        crate::arch::riscv64::sbi::putstr("..0x");
        print_hex(diag.end());
        crate::arch::riscv64::sbi::putstr(" r=");
        print_bool_digit(diag.readable());
        crate::arch::riscv64::sbi::putstr(" w=");
        print_bool_digit(diag.writable());
        crate::arch::riscv64::sbi::putstr(" x=");
        print_bool_digit(diag.executable());
        crate::arch::riscv64::sbi::putstr(" u=");
        print_bool_digit(diag.user_accessible());
    }
    crate::arch::riscv64::sbi::putstr(" perm_ok=");
    print_bool_digit(diag.permission_satisfied());
}

fn print_mapping_kind(kind: UserMappingKind, mapped: bool) {
    if !mapped {
        crate::arch::riscv64::sbi::putstr("unmapped");
        return;
    }
    match kind {
        UserMappingKind::ElfSegment => crate::arch::riscv64::sbi::putstr("elf"),
        UserMappingKind::Stack => crate::arch::riscv64::sbi::putstr("stack"),
        UserMappingKind::Heap => crate::arch::riscv64::sbi::putstr("heap"),
        UserMappingKind::Empty => crate::arch::riscv64::sbi::putstr("empty"),
    }
}

fn print_fault_access(access: UserFaultAccess) {
    match access {
        UserFaultAccess::Instruction => crate::arch::riscv64::sbi::putstr("execute"),
        UserFaultAccess::Load => crate::arch::riscv64::sbi::putstr("load"),
        UserFaultAccess::Store => crate::arch::riscv64::sbi::putstr("store"),
        UserFaultAccess::Unknown => crate::arch::riscv64::sbi::putstr("unknown"),
    }
}

fn print_bool_digit(value: bool) {
    crate::arch::riscv64::sbi::putchar(if value { b'1' } else { b'0' });
}

fn print_unsupported_syscall_diagnostic(frame: &TrapFrame) {
    crate::arch::riscv64::sbi::putstr("unsupported syscall");
    crate::arch::riscv64::sbi::putstr(" nr=");
    print_decimal(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    print_syscall_arg_registers(frame);
    print_trap_return_address(frame);
    print_trap_saved_registers(frame);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    print_unsupported_syscall_detail(frame);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

fn print_unsupported_syscall_detail(frame: &TrapFrame) {
    match frame.reg(17) {
        SYSCALL_NANOSLEEP => print_nanosleep_unsupported_detail(frame),
        SYSCALL_RT_SIGTIMEDWAIT => print_rt_sigtimedwait_unsupported_detail(frame),
        SYSCALL_CLONE => print_clone_vfork_boundary(frame, "unsupported_detail"),
        SYSCALL_EXECVE => print_execve_unsupported_detail(frame),
        _ => {}
    }
}

fn print_clone_vfork_boundary(frame: &TrapFrame, stage: &str) {
    let ctx = crate::context::context_ref();
    let boundaries = &ctx.user_clone_deferred_boundaries;
    let flags = frame.reg(10);
    let flags_without_csignal = boundaries.clone_flags_without_csignal(flags);
    let exit_signal = boundaries.exit_signal(flags);
    let child = &ctx.user_child_process;
    let files = &ctx.files_struct;
    let process = &ctx.user_init_process;

    crate::arch::riscv64::sbi::putstr(" clone_vfork stage=");
    crate::arch::riscv64::sbi::putstr(stage);
    crate::arch::riscv64::sbi::putstr(" flags=0x");
    print_hex(flags);
    crate::arch::riscv64::sbi::putstr(" flags_wo_csignal=0x");
    print_hex(flags_without_csignal);
    crate::arch::riscv64::sbi::putstr(" exit_signal=");
    print_decimal(exit_signal);
    crate::arch::riscv64::sbi::putstr(" newsp=0x");
    print_hex(frame.reg(11));
    crate::arch::riscv64::sbi::putstr(" parent_tidptr=0x");
    print_hex(frame.reg(12));
    crate::arch::riscv64::sbi::putstr(" child_tidptr=0x");
    print_hex(frame.reg(13));
    crate::arch::riscv64::sbi::putstr(" pidfd_fd=");
    print_decimal(files.pidfd_fd());
    crate::arch::riscv64::sbi::putstr(" pidfd_installed=");
    print_bool_digit(files.pidfd_installed());
    crate::arch::riscv64::sbi::putstr(" pidfd_ready=");
    print_bool_digit(files.pidfd_ready());
    crate::arch::riscv64::sbi::putstr(" pidfd_copyout=");
    print_bool_digit(child.pidfd_copyout_observed());
    crate::arch::riscv64::sbi::putstr(" parent_frame_saved=");
    print_bool_digit(child.vfork_parent_frame_saved());
    crate::arch::riscv64::sbi::putstr(" child_handoff=");
    print_bool_digit(child.vfork_child_handoff());
    crate::arch::riscv64::sbi::putstr(" current_child=");
    print_bool_digit(child.current_child_continuation());
    crate::arch::riscv64::sbi::putstr(" child_pid=");
    print_decimal(child.pid());
    crate::arch::riscv64::sbi::putstr(" child_exit_status=");
    print_decimal(child.child_exit_status());
    crate::arch::riscv64::sbi::putstr(" pending_sigchld=");
    print_bool_digit(process.pending_sigchld());
    crate::arch::riscv64::sbi::putstr(" parent_clone_return=");
    print_decimal(child.parent_clone_return());
    crate::arch::riscv64::sbi::putstr(" nested_vfork=");
    print_bool_digit(child.nested_vfork_clone());
    crate::arch::riscv64::sbi::putstr(" nested_parent_pid=");
    print_decimal(child.nested_vfork_parent_pid());
    crate::arch::riscv64::sbi::putstr(" vfork_resume_on_exit=");
    print_bool_digit(child.vfork_parent_resume_on_exit());
    crate::arch::riscv64::sbi::putstr(" active_slot_state=");
    print_decimal(child.state() as usize);
    crate::arch::riscv64::sbi::putstr(" active_slot_reusable=");
    print_bool_digit(child.active_slot_reusable());
    crate::arch::riscv64::sbi::putstr(" next_child_pid=");
    print_decimal(child.next_child_pid());
    crate::arch::riscv64::sbi::putstr(" completed_records=");
    print_decimal(child.completed_child_record_count());
    crate::arch::riscv64::sbi::putstr(" completed_records_occupied=");
    print_decimal(child.completed_child_record_occupied_count());
    crate::arch::riscv64::sbi::putstr(" completed_records_total_archived=");
    print_decimal(child.completed_child_record_total_archived());
    crate::arch::riscv64::sbi::putstr(" completed_records_reaped=");
    print_decimal(child.completed_child_record_reaped_count());
    crate::arch::riscv64::sbi::putstr(" completed_records_released=");
    print_decimal(child.completed_child_record_released_count());
    crate::arch::riscv64::sbi::putstr(" completed_record_free_slots=");
    print_decimal(child.completed_child_record_free_count());
    crate::arch::riscv64::sbi::putstr(" record_capacity=");
    print_decimal(child.completed_child_record_capacity());
    if let Some((pid, status, wait_status, pidfd_fd)) = child.first_unreaped_completed_child() {
        crate::arch::riscv64::sbi::putstr(" first_unreaped_pid=");
        print_decimal(pid);
        crate::arch::riscv64::sbi::putstr(" first_unreaped_status=");
        print_decimal(status);
        crate::arch::riscv64::sbi::putstr(" first_unreaped_wait_status=");
        print_decimal(wait_status);
        crate::arch::riscv64::sbi::putstr(" first_unreaped_pidfd_fd=");
        print_decimal(pidfd_fd);
    } else {
        crate::arch::riscv64::sbi::putstr(" first_unreaped_pid=0 first_unreaped_status=0 first_unreaped_wait_status=0 first_unreaped_pidfd_fd=");
        print_decimal(usize::MAX);
    }
}

fn print_nanosleep_unsupported_detail(frame: &TrapFrame) {
    let rqtp = frame.reg(10);
    let rmtp = frame.reg(11);
    crate::arch::riscv64::sbi::putstr(" name=nanosleep rqtp=0x");
    print_hex(rqtp);
    crate::arch::riscv64::sbi::putstr(" rmtp=0x");
    print_hex(rmtp);
    crate::arch::riscv64::sbi::putstr(" req_copy=");
    if rqtp == 0
        || !crate::context::context_ref()
            .user_address_space
            .user_range_mapped(rqtp, TIMESPEC_SIZE)
    {
        crate::arch::riscv64::sbi::putstr("failed");
        return;
    }

    if let Some((sec, nsec)) = read_user_timespec_i64(rqtp) {
        crate::arch::riscv64::sbi::putstr("ok req_sec=");
        print_i64(sec);
        crate::arch::riscv64::sbi::putstr(" req_nsec=");
        print_i64(nsec);
    } else {
        crate::arch::riscv64::sbi::putstr("failed");
    }
}

fn print_rt_sigtimedwait_unsupported_detail(frame: &TrapFrame) {
    let uthese = frame.reg(10);
    let uinfo = frame.reg(11);
    let uts = frame.reg(12);
    let sigsetsize = frame.reg(13);
    crate::arch::riscv64::sbi::putstr(" name=rt_sigtimedwait uthese=0x");
    print_hex(uthese);
    crate::arch::riscv64::sbi::putstr(" uinfo=0x");
    print_hex(uinfo);
    crate::arch::riscv64::sbi::putstr(" uts=0x");
    print_hex(uts);
    crate::arch::riscv64::sbi::putstr(" sigsetsize=");
    print_decimal(sigsetsize);
    crate::arch::riscv64::sbi::putstr(" uthese_copy=");
    if uthese == 0 {
        crate::arch::riscv64::sbi::putstr("NULL");
    } else if !crate::context::context_ref()
        .user_address_space
        .user_range_mapped(uthese, RT_SIGSET_SIZE)
    {
        crate::arch::riscv64::sbi::putstr("skipped");
    } else if let Some(mask) = read_user_usize(uthese) {
        crate::arch::riscv64::sbi::putstr("ok uthese_mask=0x");
        print_hex(mask);
    } else {
        crate::arch::riscv64::sbi::putstr("failed");
    }
    crate::arch::riscv64::sbi::putstr(" uts_copy=");
    if uts == 0 {
        crate::arch::riscv64::sbi::putstr("NULL");
    } else if !crate::context::context_ref()
        .user_address_space
        .user_range_mapped(uts, TIMESPEC_SIZE)
    {
        crate::arch::riscv64::sbi::putstr("skipped");
    } else if let Some((sec, nsec)) = read_user_timespec_i64(uts) {
        crate::arch::riscv64::sbi::putstr("ok uts_sec=");
        print_i64(sec);
        crate::arch::riscv64::sbi::putstr(" uts_nsec=");
        print_i64(nsec);
    } else {
        crate::arch::riscv64::sbi::putstr("failed");
    }
}

fn print_rt_sigtimedwait_wait_boundary(mask: usize) {
    let process = &crate::context::context_ref().user_init_process;
    crate::arch::riscv64::sbi::putstr("rt_sigtimedwait waiting uthese_copy=ok uthese_mask=0x");
    print_hex(mask);
    crate::arch::riscv64::sbi::putstr(" uinfo=NULL uts=NULL sigchld=17 sigchld_mask=0x");
    print_hex(crate::objects::user_boot::USER_SIGCHLD_MASK);
    crate::arch::riscv64::sbi::putstr(" pending_sigchld=");
    print_bool_digit(process.pending_sigchld());
    crate::arch::riscv64::sbi::putstr(" pending_match=");
    print_bool_digit(process.rt_sigtimedwait_pending_match());
    crate::arch::riscv64::sbi::putstr(" waiter_enqueued=");
    print_bool_digit(process.rt_sigtimedwait_waiter_enqueued());
    crate::arch::riscv64::sbi::putstr(" sleep_reason=");
    if process.rt_sigtimedwait_sleep_reason()
        == USER_SIGNAL_WAIT_REASON_RT_SIGTIMEDWAIT_SIGCHLD_INFINITE
    {
        crate::arch::riscv64::sbi::putstr("rt_sigtimedwait/SIGCHLD/infinite");
    } else {
        crate::arch::riscv64::sbi::putstr("none");
    }
    crate::arch::riscv64::sbi::putstr(" wait=");
    if process.rt_sigtimedwait_infinite_wait() {
        crate::arch::riscv64::sbi::putstr("infinite");
    } else {
        crate::arch::riscv64::sbi::putstr("bounded");
    }
    crate::arch::riscv64::sbi::putchar(b'\n');
}

fn print_execve_unsupported_detail(frame: &TrapFrame) {
    let filename_ptr = frame.reg(10);
    let argv_ptr = frame.reg(11);
    let envp_ptr = frame.reg(12);
    let child_continuation = crate::context::context_ref()
        .user_child_process
        .current_child_continuation();
    crate::arch::riscv64::sbi::putstr(" name=execve filename_ptr=0x");
    print_hex(filename_ptr);
    crate::arch::riscv64::sbi::putstr(" argv_ptr=0x");
    print_hex(argv_ptr);
    crate::arch::riscv64::sbi::putstr(" envp_ptr=0x");
    print_hex(envp_ptr);
    crate::arch::riscv64::sbi::putstr(" child_cont=");
    print_bool_digit(child_continuation);
    #[cfg(app_user_boot)]
    {
        let obs = execve_checkpoint_observation();
        crate::arch::riscv64::sbi::putstr(" exec_stage=");
        print_execve_obs_stage(obs.stage);
        crate::arch::riscv64::sbi::putstr(" fail_stage=");
        print_execve_fail_stage(obs.failure_stage);
        crate::arch::riscv64::sbi::putstr(" fail_reason=");
        print_execve_fail_reason(obs.failure_reason);
        crate::arch::riscv64::sbi::putstr(" fail_detail=");
        print_execve_failure_detail(obs.failure_detail);
        crate::arch::riscv64::sbi::putstr(" fail_free_pages=");
        print_decimal(obs.failure_free_pages);
        crate::arch::riscv64::sbi::putstr(" fail_totalram_pages=");
        print_decimal(obs.failure_totalram_pages);
        crate::arch::riscv64::sbi::putstr(" filename_len=");
        print_decimal(obs.filename_len);
        crate::arch::riscv64::sbi::putstr(" argv0_len=");
        print_decimal(obs.argv0_len);
        crate::arch::riscv64::sbi::putstr(" argc=");
        print_decimal(obs.argv_argc);
        crate::arch::riscv64::sbi::putstr(" argv_total_bytes=");
        print_decimal(obs.argv_total_bytes);
        crate::arch::riscv64::sbi::putstr(" argv_capacity_exceeded=");
        print_decimal(obs.argv_capacity_exceeded);
        crate::arch::riscv64::sbi::putstr(" main_len=");
        print_decimal(obs.main_input_len);
        crate::arch::riscv64::sbi::putstr(" main_elf_type=");
        print_decimal(obs.main_elf_type);
        crate::arch::riscv64::sbi::putstr(" main_interp=");
        print_decimal(obs.main_interpreter_required);
        crate::arch::riscv64::sbi::putstr(" interp_path_len=");
        print_decimal(obs.interpreter_path_len);
        crate::arch::riscv64::sbi::putstr(" interp_len=");
        print_decimal(obs.interpreter_input_len);
        crate::arch::riscv64::sbi::putstr(" mappings=");
        print_decimal(obs.mapping_count);
    }
    print_execve_cstr_copy(" filename", filename_ptr);
    print_execve_vector_prefix(" argv", argv_ptr);
    print_execve_vector_prefix(" envp", envp_ptr);
}

#[cfg(app_user_boot)]
fn print_execve_obs_stage(stage: usize) {
    match stage {
        EXECVE_OBS_STAGE_NONE => crate::arch::riscv64::sbi::putstr("none"),
        EXECVE_OBS_STAGE_ARGS_READY => crate::arch::riscv64::sbi::putstr("args_ready"),
        EXECVE_OBS_STAGE_MAIN_ELF_READY => crate::arch::riscv64::sbi::putstr("main_elf_ready"),
        EXECVE_OBS_STAGE_INTERPRETER_READY => {
            crate::arch::riscv64::sbi::putstr("interpreter_ready");
        }
        EXECVE_OBS_STAGE_ADDRESS_SPACE_READY => {
            crate::arch::riscv64::sbi::putstr("address_space_ready");
        }
        EXECVE_OBS_STAGE_CONTEXT_REPLACED => {
            crate::arch::riscv64::sbi::putstr("context_replaced");
        }
        EXECVE_OBS_STAGE_SATP_READY => crate::arch::riscv64::sbi::putstr("satp_ready"),
        EXECVE_OBS_STAGE_TRAP_FRAME_READY => {
            crate::arch::riscv64::sbi::putstr("trap_frame_ready");
        }
        EXECVE_OBS_STAGE_SATP_SWITCHED => crate::arch::riscv64::sbi::putstr("satp_switched"),
        EXECVE_OBS_STAGE_RETURN_FRAME_READY => {
            crate::arch::riscv64::sbi::putstr("return_frame_ready");
        }
        _ => print_decimal(stage),
    }
}

#[cfg(app_user_boot)]
fn print_execve_fail_stage(stage: usize) {
    match stage {
        EXECVE_FAIL_STAGE_NONE => crate::arch::riscv64::sbi::putstr("none"),
        EXECVE_FAIL_STAGE_CHILD_CONTINUATION => {
            crate::arch::riscv64::sbi::putstr("child_continuation");
        }
        EXECVE_FAIL_STAGE_PATH_KIND => crate::arch::riscv64::sbi::putstr("path_kind"),
        EXECVE_FAIL_STAGE_PATH_READ => crate::arch::riscv64::sbi::putstr("path_read"),
        EXECVE_FAIL_STAGE_MAIN_PRESET => crate::arch::riscv64::sbi::putstr("main_preset"),
        EXECVE_FAIL_STAGE_MAIN_SETUP => crate::arch::riscv64::sbi::putstr("main_setup"),
        EXECVE_FAIL_STAGE_INTERPRETER_READ => {
            crate::arch::riscv64::sbi::putstr("interpreter_read");
        }
        EXECVE_FAIL_STAGE_INTERPRETER_PRESET => {
            crate::arch::riscv64::sbi::putstr("interpreter_preset");
        }
        EXECVE_FAIL_STAGE_INTERPRETER_SETUP => {
            crate::arch::riscv64::sbi::putstr("interpreter_setup");
        }
        EXECVE_FAIL_STAGE_INTERPRETER_BIND => {
            crate::arch::riscv64::sbi::putstr("interpreter_bind");
        }
        EXECVE_FAIL_STAGE_ADDRESS_SPACE_PRESET => {
            crate::arch::riscv64::sbi::putstr("address_space_preset");
        }
        EXECVE_FAIL_STAGE_STACK_SETUP => crate::arch::riscv64::sbi::putstr("stack_setup"),
        EXECVE_FAIL_STAGE_ADDRESS_SPACE_SETUP => {
            crate::arch::riscv64::sbi::putstr("address_space_setup");
        }
        EXECVE_FAIL_STAGE_TRAP_FRAME_SETUP => {
            crate::arch::riscv64::sbi::putstr("trap_frame_setup");
        }
        EXECVE_FAIL_STAGE_ELF_ENABLE => crate::arch::riscv64::sbi::putstr("elf_enable"),
        EXECVE_FAIL_STAGE_ADDRESS_SPACE_ENABLE => {
            crate::arch::riscv64::sbi::putstr("address_space_enable");
        }
        EXECVE_FAIL_STAGE_CLOSE_ON_EXEC => crate::arch::riscv64::sbi::putstr("close_on_exec"),
        EXECVE_FAIL_STAGE_ARGV_COPY => crate::arch::riscv64::sbi::putstr("argv_copy"),
        _ => print_decimal(stage),
    }
}

#[cfg(app_user_boot)]
fn print_execve_fail_reason(reason: usize) {
    match reason {
        EXECVE_FAIL_REASON_NONE => crate::arch::riscv64::sbi::putstr("none"),
        EXECVE_FAIL_REASON_NOT_CHILD_CONTINUATION => {
            crate::arch::riscv64::sbi::putstr("not_child_continuation");
        }
        EXECVE_FAIL_REASON_RELATIVE_PATH => crate::arch::riscv64::sbi::putstr("relative_path"),
        EXECVE_FAIL_REASON_VFS_READ => crate::arch::riscv64::sbi::putstr("vfs_read"),
        EXECVE_FAIL_REASON_UNSUPPORTED_ELF => {
            crate::arch::riscv64::sbi::putstr("unsupported_elf");
        }
        EXECVE_FAIL_REASON_INVALID_STATE => crate::arch::riscv64::sbi::putstr("invalid_state"),
        EXECVE_FAIL_REASON_INTERPRETER_READ => {
            crate::arch::riscv64::sbi::putstr("interpreter_read");
        }
        EXECVE_FAIL_REASON_ADDRESS_SPACE => crate::arch::riscv64::sbi::putstr("address_space"),
        EXECVE_FAIL_REASON_STACK => crate::arch::riscv64::sbi::putstr("stack"),
        EXECVE_FAIL_REASON_TRAP_FRAME => crate::arch::riscv64::sbi::putstr("trap_frame"),
        EXECVE_FAIL_REASON_CLOSE_ON_EXEC => crate::arch::riscv64::sbi::putstr("close_on_exec"),
        EXECVE_FAIL_REASON_ARGV_COPY => crate::arch::riscv64::sbi::putstr("argv_copy"),
        EXECVE_FAIL_REASON_ARGV_CAPACITY => {
            crate::arch::riscv64::sbi::putstr("argv_capacity");
        }
        _ => print_decimal(reason),
    }
}

#[cfg(app_user_boot)]
fn print_execve_failure_detail(detail: usize) {
    match detail {
        0 => crate::arch::riscv64::sbi::putstr("none"),
        1 => crate::arch::riscv64::sbi::putstr(ElfError::InvalidState.name()),
        2 => crate::arch::riscv64::sbi::putstr(ElfError::ShortInput.name()),
        3 => crate::arch::riscv64::sbi::putstr(ElfError::BadMagic.name()),
        4 => crate::arch::riscv64::sbi::putstr(ElfError::UnsupportedClass.name()),
        5 => crate::arch::riscv64::sbi::putstr(ElfError::UnsupportedEndian.name()),
        6 => crate::arch::riscv64::sbi::putstr(ElfError::UnsupportedVersion.name()),
        7 => crate::arch::riscv64::sbi::putstr(ElfError::UnsupportedType.name()),
        8 => crate::arch::riscv64::sbi::putstr(ElfError::UnsupportedMachine.name()),
        9 => crate::arch::riscv64::sbi::putstr(ElfError::InvalidHeader.name()),
        10 => crate::arch::riscv64::sbi::putstr(ElfError::InvalidProgramHeader.name()),
        11 => crate::arch::riscv64::sbi::putstr(ElfError::TooManyLoadSegments.name()),
        12 => crate::arch::riscv64::sbi::putstr(ElfError::MissingLoadSegment.name()),
        13 => crate::arch::riscv64::sbi::putstr(ElfError::EntryOutsideExecutableSegment.name()),
        14 => crate::arch::riscv64::sbi::putstr(ElfError::TooManyMappings.name()),
        15 => crate::arch::riscv64::sbi::putstr(ElfError::TooManyMappingPages.name()),
        16 => crate::arch::riscv64::sbi::putstr(ElfError::MissingExecutableEntryMapping.name()),
        17 => crate::arch::riscv64::sbi::putstr(ElfError::InvalidStack.name()),
        18 => crate::arch::riscv64::sbi::putstr(ElfError::BackingAllocationFailed.name()),
        19 => crate::arch::riscv64::sbi::putstr(ElfError::PageTableAllocationFailed.name()),
        20 => crate::arch::riscv64::sbi::putstr(ElfError::PageTableInstallFailed.name()),
        21 => crate::arch::riscv64::sbi::putstr(ElfError::UserCopyOutOfRange.name()),
        _ => print_decimal(detail),
    }
}

fn print_execve_vector_prefix(label: &str, vector_ptr: usize) {
    crate::arch::riscv64::sbi::putstr(" ");
    crate::arch::riscv64::sbi::putstr(label);
    crate::arch::riscv64::sbi::putstr("=");
    if vector_ptr == 0 {
        crate::arch::riscv64::sbi::putstr("NULL");
        return;
    }
    crate::arch::riscv64::sbi::putchar(b'[');
    let mut index = 0usize;
    while index < USER_EXECVE_VECTOR_DIAG_MAX {
        if index != 0 {
            crate::arch::riscv64::sbi::putchar(b',');
        }
        let Some(entry_ptr_addr) = vector_ptr.checked_add(index * core::mem::size_of::<usize>())
        else {
            crate::arch::riscv64::sbi::putstr("overflow");
            break;
        };
        if !crate::context::context_ref()
            .user_address_space
            .user_range_mapped(entry_ptr_addr, core::mem::size_of::<usize>())
        {
            crate::arch::riscv64::sbi::putstr("skipped");
            break;
        }
        let Some(entry_ptr) = read_user_usize(entry_ptr_addr) else {
            crate::arch::riscv64::sbi::putstr("failed");
            break;
        };
        crate::arch::riscv64::sbi::putstr("{ptr=0x");
        print_hex(entry_ptr);
        if entry_ptr == 0 {
            crate::arch::riscv64::sbi::putstr(",value=NULL}");
            break;
        }
        print_execve_cstr_copy(",value", entry_ptr);
        crate::arch::riscv64::sbi::putchar(b'}');
        index += 1;
    }
    crate::arch::riscv64::sbi::putchar(b']');
}

fn print_execve_cstr_copy(label: &str, user_ptr: usize) {
    crate::arch::riscv64::sbi::putstr(label);
    crate::arch::riscv64::sbi::putchar(b'=');
    if user_ptr == 0 {
        crate::arch::riscv64::sbi::putstr("NULL");
        return;
    }
    if !crate::context::context_ref()
        .user_address_space
        .user_range_mapped(user_ptr, USER_PATH_MAX)
    {
        crate::arch::riscv64::sbi::putstr("skipped");
        return;
    }
    let mut bytes = [0u8; USER_PATH_MAX];
    if let Some(len) = copy_cstr_from_user(user_ptr, &mut bytes) {
        crate::arch::riscv64::sbi::putchar(b'"');
        print_path_bytes(&bytes[..len]);
        crate::arch::riscv64::sbi::putchar(b'"');
    } else {
        crate::arch::riscv64::sbi::putstr("failed");
    }
}

#[cfg(checkpoint_handler_user_read_trace)]
fn print_read_trace_success(
    frame: &TrapFrame,
    fd: usize,
    requested: usize,
    capped: usize,
    result: usize,
) {
    print_read_trace_prefix(frame, fd, requested, capped);
    crate::arch::riscv64::sbi::putstr(" result=");
    print_decimal(result);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_read_trace))]
fn print_read_trace_success(
    _frame: &TrapFrame,
    _fd: usize,
    _requested: usize,
    _capped: usize,
    _result: usize,
) {
}

#[cfg(checkpoint_handler_user_read_trace)]
fn print_read_trace_file_error(
    frame: &TrapFrame,
    fd: usize,
    requested: usize,
    capped: usize,
    error: FileError,
) {
    print_read_trace_prefix(frame, fd, requested, capped);
    crate::arch::riscv64::sbi::putstr(" failure=file_error file_error=");
    print_file_error_name(error);
    crate::arch::riscv64::sbi::putstr(" unsupported_errno=");
    print_decimal(ENOSYS);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_read_trace))]
fn print_read_trace_file_error(
    _frame: &TrapFrame,
    _fd: usize,
    _requested: usize,
    _capped: usize,
    _error: FileError,
) {
}

#[cfg(checkpoint_handler_user_read_trace)]
fn print_read_trace_copy_error(
    frame: &TrapFrame,
    fd: usize,
    requested: usize,
    capped: usize,
    read: usize,
) {
    print_read_trace_prefix(frame, fd, requested, capped);
    crate::arch::riscv64::sbi::putstr(" failure=copy_to_user copied_len=");
    print_decimal(read);
    crate::arch::riscv64::sbi::putstr(" unsupported_errno=");
    print_decimal(ENOSYS);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_read_trace))]
fn print_read_trace_copy_error(
    _frame: &TrapFrame,
    _fd: usize,
    _requested: usize,
    _capped: usize,
    _read: usize,
) {
}

#[cfg(checkpoint_handler_user_read_trace)]
fn print_read_trace_prefix(frame: &TrapFrame, fd: usize, requested: usize, capped: usize) {
    crate::arch::riscv64::sbi::putstr("syscall read trace nr=");
    print_decimal(SYSCALL_READ);
    crate::arch::riscv64::sbi::putstr(" name=read fd=");
    print_decimal(fd);
    crate::arch::riscv64::sbi::putstr(" requested=");
    print_decimal(requested);
    crate::arch::riscv64::sbi::putstr(" capped=");
    print_decimal(capped);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    crate::arch::riscv64::sbi::putstr(" a0=0x");
    print_hex(frame.reg(10));
    crate::arch::riscv64::sbi::putstr(" a1=0x");
    print_hex(frame.reg(11));
    crate::arch::riscv64::sbi::putstr(" a2=0x");
    print_hex(frame.reg(12));
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_syscall_trace_return(frame: &TrapFrame, value: usize) {
    crate::arch::riscv64::sbi::putstr("syscall trace nr=");
    print_decimal(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" name=");
    print_syscall_name(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" ret=");
    print_syscall_trace_value(value);
    crate::arch::riscv64::sbi::putstr(" ret_hex=0x");
    print_hex(value);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    print_syscall_arg_registers(frame);
    print_trap_return_address(frame);
    print_trap_saved_registers(frame);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_trace))]
fn print_syscall_trace_return(_frame: &TrapFrame, _value: usize) {}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_syscall_trace_exit(frame: &TrapFrame, status: usize) {
    crate::arch::riscv64::sbi::putstr("syscall trace nr=");
    print_decimal(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" name=");
    print_syscall_name(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" exit_status=");
    print_decimal(status);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    print_syscall_arg_registers(frame);
    print_trap_return_address(frame);
    print_trap_saved_registers(frame);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_trace))]
fn print_syscall_trace_exit(_frame: &TrapFrame, _status: usize) {}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_ppoll_trace_summary(
    frame: &TrapFrame,
    nfds: usize,
    timeout_ptr: usize,
    timeout_value: Option<(i64, i64)>,
    sigmask_ptr: usize,
    ready_count: usize,
) {
    crate::arch::riscv64::sbi::putstr("syscall ppoll trace nfds=");
    print_decimal(nfds);
    crate::arch::riscv64::sbi::putstr(" timeout_ptr=0x");
    print_hex(timeout_ptr);
    crate::arch::riscv64::sbi::putstr(" timeout=");
    if let Some((sec, nsec)) = timeout_value {
        crate::arch::riscv64::sbi::putstr("{sec=");
        print_i64(sec);
        crate::arch::riscv64::sbi::putstr(",nsec=");
        print_i64(nsec);
        crate::arch::riscv64::sbi::putchar(b'}');
    } else {
        crate::arch::riscv64::sbi::putstr("NULL");
    }
    crate::arch::riscv64::sbi::putstr(" sigmask_ptr=0x");
    print_hex(sigmask_ptr);
    crate::arch::riscv64::sbi::putstr(" ready=");
    print_decimal(ready_count);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_trace))]
fn print_ppoll_trace_summary(
    _frame: &TrapFrame,
    _nfds: usize,
    _timeout_ptr: usize,
    _timeout_value: Option<(i64, i64)>,
    _sigmask_ptr: usize,
    _ready_count: usize,
) {
}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_ppoll_trace_entry(frame: &TrapFrame, index: usize, pollfd: UserPollFd, revents: u16) {
    crate::arch::riscv64::sbi::putstr("syscall ppoll trace entry=");
    print_decimal(index);
    crate::arch::riscv64::sbi::putstr(" fd=");
    print_i64(pollfd.fd as i64);
    crate::arch::riscv64::sbi::putstr(" events=0x");
    print_hex(pollfd.events as usize);
    crate::arch::riscv64::sbi::putstr(" revents=0x");
    print_hex(revents as usize);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_trace))]
fn print_ppoll_trace_entry(_frame: &TrapFrame, _index: usize, _pollfd: UserPollFd, _revents: u16) {}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_tty_wait_trace(kind: &str, event: &str, fd: usize, ready: Option<bool>) {
    crate::arch::riscv64::sbi::putstr("tty wait trace kind=");
    crate::arch::riscv64::sbi::putstr(kind);
    crate::arch::riscv64::sbi::putstr(" event=");
    crate::arch::riscv64::sbi::putstr(event);
    crate::arch::riscv64::sbi::putstr(" fd=");
    if fd == usize::MAX {
        crate::arch::riscv64::sbi::putstr("none");
    } else {
        print_decimal(fd);
    }
    crate::arch::riscv64::sbi::putstr(" ready=");
    match ready {
        Some(true) => crate::arch::riscv64::sbi::putstr("1"),
        Some(false) => crate::arch::riscv64::sbi::putstr("0"),
        None => crate::arch::riscv64::sbi::putstr("pending"),
    }
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_trace))]
fn print_tty_wait_trace(_kind: &str, _event: &str, _fd: usize, _ready: Option<bool>) {}

#[cfg(checkpoint_handler_user_syscall_trace)]
fn print_syscall_trace_value(value: usize) {
    if value > usize::MAX - 4095 {
        crate::arch::riscv64::sbi::putchar(b'-');
        print_decimal(0usize.wrapping_sub(value));
    } else {
        print_decimal(value);
    }
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_syscall_error_diagnostic(frame: &TrapFrame, errno: usize) {
    crate::arch::riscv64::sbi::putstr("syscall error");
    crate::arch::riscv64::sbi::putstr(" nr=");
    print_decimal(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" name=");
    print_syscall_name(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" errno=");
    print_decimal(errno);
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    print_syscall_arg_registers(frame);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_syscall_error_diagnostic(_frame: &TrapFrame, _errno: usize) {}

#[cfg(any(
    checkpoint_handler_user_syscall_error,
    checkpoint_handler_user_syscall_trace
))]
fn print_syscall_name(nr: usize) {
    let name = match nr {
        SYSCALL_GETCWD => "getcwd",
        SYSCALL_DUP3 => "dup3",
        SYSCALL_FCNTL => "fcntl",
        SYSCALL_IOCTL => "ioctl",
        SYSCALL_FACCESSAT => "faccessat",
        SYSCALL_CHDIR => "chdir",
        SYSCALL_OPENAT => "openat",
        SYSCALL_CLOSE => "close",
        SYSCALL_GETDENTS64 => "getdents64",
        SYSCALL_LSEEK => "lseek",
        SYSCALL_READ => "read",
        SYSCALL_WRITE => "write",
        SYSCALL_WRITEV => "writev",
        SYSCALL_PPOLL => "ppoll",
        SYSCALL_READLINKAT => "readlinkat",
        SYSCALL_NEWFSTATAT => "newfstatat",
        SYSCALL_FSTAT => "fstat",
        SYSCALL_SET_TID_ADDRESS => "set_tid_address",
        SYSCALL_NANOSLEEP => "nanosleep",
        SYSCALL_CLOCK_GETTIME => "clock_gettime",
        SYSCALL_RT_SIGACTION => "rt_sigaction",
        SYSCALL_RT_SIGPROCMASK => "rt_sigprocmask",
        SYSCALL_RT_SIGTIMEDWAIT => "rt_sigtimedwait",
        SYSCALL_SETGID => "setgid",
        SYSCALL_SETUID => "setuid",
        SYSCALL_GETRESUID => "getresuid",
        SYSCALL_GETRESGID => "getresgid",
        SYSCALL_SETPGID => "setpgid",
        SYSCALL_GETSID => "getsid",
        SYSCALL_SETSID => "setsid",
        SYSCALL_UNAME => "uname",
        SYSCALL_GETTIMEOFDAY => "gettimeofday",
        SYSCALL_GETPID => "getpid",
        SYSCALL_GETPGID => "getpgid",
        SYSCALL_GETPPID => "getppid",
        SYSCALL_GETUID => "getuid",
        SYSCALL_GETEUID => "geteuid",
        SYSCALL_GETGID => "getgid",
        SYSCALL_GETEGID => "getegid",
        SYSCALL_BRK => "brk",
        SYSCALL_MUNMAP => "munmap",
        SYSCALL_CLONE => "clone",
        SYSCALL_EXECVE => "execve",
        SYSCALL_MMAP => "mmap",
        SYSCALL_MPROTECT => "mprotect",
        SYSCALL_WAIT4 => "wait4",
        SYSCALL_GETRANDOM => "getrandom",
        SYSCALL_EXIT => "exit",
        SYSCALL_EXIT_GROUP => "exit_group",
        _ => "unknown",
    };
    crate::arch::riscv64::sbi::putstr(name);
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_openat_reject_detail(dirfd: usize, path_ptr: usize, flags: usize, supported_flags: usize) {
    let unsupported_flags = flags & !(supported_flags | O_ACCMODE);
    let access_mode = flags & O_ACCMODE;
    crate::arch::riscv64::sbi::putstr("syscall openat reject");
    crate::arch::riscv64::sbi::putstr(" reason=");
    print_openat_reject_reason(dirfd, unsupported_flags, access_mode);
    crate::arch::riscv64::sbi::putstr(" dirfd=");
    print_dirfd(dirfd);
    crate::arch::riscv64::sbi::putstr(" flags=0x");
    print_hex(flags);
    crate::arch::riscv64::sbi::putstr(" unsupported_flags=0x");
    print_hex(unsupported_flags);
    crate::arch::riscv64::sbi::putstr(" access_mode=0x");
    print_hex(access_mode);
    crate::arch::riscv64::sbi::putstr(" path_ptr=0x");
    print_hex(path_ptr);
    print_probe_path_copy(path_ptr);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_openat_reject_detail(
    _dirfd: usize,
    _path_ptr: usize,
    _flags: usize,
    _supported_flags: usize,
) {
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_fcntl_error_detail(fd: usize, cmd: usize, arg: usize, errno: usize) {
    crate::arch::riscv64::sbi::putstr("syscall fcntl detail");
    crate::arch::riscv64::sbi::putstr(" fd=");
    print_decimal(fd);
    crate::arch::riscv64::sbi::putstr(" cmd=0x");
    print_hex(cmd);
    crate::arch::riscv64::sbi::putstr(" cmd_name=");
    print_fcntl_cmd_name(cmd);
    crate::arch::riscv64::sbi::putstr(" arg=0x");
    print_hex(arg);
    crate::arch::riscv64::sbi::putstr(" errno=");
    print_decimal(errno);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_fcntl_error_detail(_fd: usize, _cmd: usize, _arg: usize, _errno: usize) {}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_dup3_error_detail(oldfd: usize, newfd: usize, flags: usize, errno: usize) {
    crate::arch::riscv64::sbi::putstr("syscall dup3 detail oldfd=");
    print_decimal(oldfd);
    crate::arch::riscv64::sbi::putstr(" newfd=");
    print_decimal(newfd);
    crate::arch::riscv64::sbi::putstr(" flags=0x");
    print_hex(flags);
    crate::arch::riscv64::sbi::putstr(" errno=");
    print_decimal(errno);
    print_fd_table_diagnostic(true);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_dup3_error_detail(_oldfd: usize, _newfd: usize, _flags: usize, _errno: usize) {}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_ioctl_error_detail(fd: usize, cmd: usize, arg: usize, errno: usize) {
    crate::arch::riscv64::sbi::putstr("syscall ioctl detail");
    crate::arch::riscv64::sbi::putstr(" fd=");
    print_decimal(fd);
    crate::arch::riscv64::sbi::putstr(" cmd=0x");
    print_hex(cmd);
    crate::arch::riscv64::sbi::putstr(" cmd_name=");
    print_ioctl_cmd_name(cmd);
    crate::arch::riscv64::sbi::putstr(" arg=0x");
    print_hex(arg);
    crate::arch::riscv64::sbi::putstr(" errno=");
    print_decimal(errno);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_ioctl_error_detail(_fd: usize, _cmd: usize, _arg: usize, _errno: usize) {}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_close_error_detail(fd: usize, error: FileError, errno: usize) {
    crate::arch::riscv64::sbi::putstr("syscall close detail fd=");
    print_decimal(fd);
    crate::arch::riscv64::sbi::putstr(" file_error=");
    print_file_error_name(error);
    crate::arch::riscv64::sbi::putstr(" errno=");
    print_decimal(errno);
    print_fd_table_diagnostic(true);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_close_error_detail(_fd: usize, _error: FileError, _errno: usize) {}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_execve_close_on_exec_report(report: CloseOnExecReport) {
    crate::arch::riscv64::sbi::putstr("execve close_on_exec scanned=");
    print_decimal(report.scanned);
    crate::arch::riscv64::sbi::putstr(" closed=");
    print_decimal(report.closed);
    crate::arch::riscv64::sbi::putstr(" first_closed_fd=");
    if report.first_closed_fd == usize::MAX {
        crate::arch::riscv64::sbi::putstr("none");
    } else {
        print_decimal(report.first_closed_fd);
    }
    crate::arch::riscv64::sbi::putstr(" remaining_open=");
    print_decimal(report.remaining_open);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_execve_close_on_exec_report(_report: CloseOnExecReport) {}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_dirfd(dirfd: usize) {
    if dirfd == AT_FDCWD {
        crate::arch::riscv64::sbi::putstr("AT_FDCWD(-100)");
    } else {
        crate::arch::riscv64::sbi::putstr("0x");
        print_hex(dirfd);
    }
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_openat_reject_reason(dirfd: usize, unsupported_flags: usize, access_mode: usize) {
    let mut printed = false;
    if dirfd != AT_FDCWD {
        crate::arch::riscv64::sbi::putstr("dirfd");
        printed = true;
    }
    if access_mode != 0 {
        if printed {
            crate::arch::riscv64::sbi::putchar(b'+');
        }
        crate::arch::riscv64::sbi::putstr("access_mode");
        printed = true;
    }
    if unsupported_flags != 0 {
        if printed {
            crate::arch::riscv64::sbi::putchar(b'+');
        }
        crate::arch::riscv64::sbi::putstr("unsupported_flags");
        printed = true;
    }
    if !printed {
        crate::arch::riscv64::sbi::putstr("unknown");
    }
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_probe_path_copy(path_ptr: usize) {
    crate::arch::riscv64::sbi::putstr(" path_copy=");
    if path_ptr == 0 {
        crate::arch::riscv64::sbi::putstr("failed");
        return;
    }
    if !crate::context::context_ref()
        .user_address_space
        .user_range_mapped(path_ptr, USER_PATH_MAX)
    {
        crate::arch::riscv64::sbi::putstr("skipped");
        return;
    }

    let mut path = [0u8; USER_PATH_MAX];
    if let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) {
        crate::arch::riscv64::sbi::putstr("ok path=\"");
        print_path_bytes(&path[..path_len]);
        crate::arch::riscv64::sbi::putchar(b'"');
    } else {
        crate::arch::riscv64::sbi::putstr("failed");
    }
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_fd_table_diagnostic(detailed: bool) {
    let files = &crate::context::context_ref().files_struct;
    let capacity = files.fd_table_capacity();
    crate::arch::riscv64::sbi::putstr(" fd_open=");
    print_decimal(files.fd_table_open_count());
    crate::arch::riscv64::sbi::putstr(" fd_capacity=");
    print_decimal(capacity);
    if !detailed {
        return;
    }

    crate::arch::riscv64::sbi::putstr(" fd_entries=[");
    let mut first = true;
    let mut fd = 0usize;
    while fd < capacity {
        if let Some(entry) = files.fd_table_entry_diagnostic(fd) {
            if !first {
                crate::arch::riscv64::sbi::putchar(b',');
            }
            first = false;
            crate::arch::riscv64::sbi::putstr("{fd=");
            print_decimal(fd);
            crate::arch::riscv64::sbi::putstr(",ofd=");
            print_ofd_name(entry.ofd);
            crate::arch::riscv64::sbi::putstr(",r=");
            print_bool_digit(entry.readable);
            crate::arch::riscv64::sbi::putstr(",w=");
            print_bool_digit(entry.writable);
            crate::arch::riscv64::sbi::putstr(",flags=0x");
            print_hex(entry.flags as usize);
            crate::arch::riscv64::sbi::putstr(",cloexec=");
            print_bool_digit(entry.close_on_exec);
            if entry.pid != 0 {
                crate::arch::riscv64::sbi::putstr(",pid=");
                print_decimal(entry.pid);
            }
            crate::arch::riscv64::sbi::putchar(b'}');
        }
        fd += 1;
    }
    crate::arch::riscv64::sbi::putchar(b']');
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_ofd_name(ofd: OpenFileDescriptionRef) {
    let name = match ofd {
        OpenFileDescriptionRef::Stdin => "stdin",
        OpenFileDescriptionRef::Stdout => "stdout",
        OpenFileDescriptionRef::Stderr => "stderr",
        OpenFileDescriptionRef::Regular0 => "regular0",
        OpenFileDescriptionRef::Null => "null",
        OpenFileDescriptionRef::Tty0 => "tty0",
        OpenFileDescriptionRef::Pidfd0 => "pidfd0",
    };
    crate::arch::riscv64::sbi::putstr(name);
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_fcntl_cmd_name(cmd: usize) {
    let name = match cmd {
        F_DUPFD => "F_DUPFD",
        F_GETFD => "F_GETFD",
        F_SETFD => "F_SETFD",
        F_GETFL => "F_GETFL",
        F_SETFL => "F_SETFL",
        F_DUPFD_CLOEXEC => "F_DUPFD_CLOEXEC",
        _ => "unknown",
    };
    crate::arch::riscv64::sbi::putstr(name);
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_ioctl_cmd_name(cmd: usize) {
    let name = match cmd {
        TCGETS => "TCGETS",
        TCSETS => "TCSETS",
        TIOCSCTTY => "TIOCSCTTY",
        TIOCGPGRP => "TIOCGPGRP",
        TIOCSPGRP => "TIOCSPGRP",
        TIOCGWINSZ => "TIOCGWINSZ",
        TIOCGSID => "TIOCGSID",
        _ => "unknown",
    };
    crate::arch::riscv64::sbi::putstr(name);
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_path_syscall_error_detail(error: FileError, path: &[u8]) {
    crate::arch::riscv64::sbi::putstr("syscall path error");
    crate::arch::riscv64::sbi::putstr(" file_error=");
    print_file_error_name(error);
    crate::arch::riscv64::sbi::putstr(" path=\"");
    print_path_bytes(path);
    crate::arch::riscv64::sbi::putstr("\"\n");
}

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_openat_path_error_detail(error: FileError, path: &[u8], flags: usize) {
    crate::arch::riscv64::sbi::putstr("syscall openat path error");
    crate::arch::riscv64::sbi::putstr(" file_error=");
    print_file_error_name(error);
    crate::arch::riscv64::sbi::putstr(" flags=0x");
    print_hex(flags);
    crate::arch::riscv64::sbi::putstr(" access_mode=0x");
    print_hex(flags & O_ACCMODE);
    crate::arch::riscv64::sbi::putstr(" path=\"");
    print_path_bytes(path);
    crate::arch::riscv64::sbi::putchar(b'"');
    print_fd_table_diagnostic(error == FileError::TooManyOpenFiles);
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_openat_path_error_detail(_error: FileError, _path: &[u8], _flags: usize) {}

fn print_path_bytes(path: &[u8]) {
    for &byte in path {
        if byte.is_ascii_graphic() || byte == b' ' {
            crate::arch::riscv64::sbi::putchar(byte);
        } else {
            crate::arch::riscv64::sbi::putchar(b'?');
        }
    }
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_path_syscall_error_detail(_error: FileError, _path: &[u8]) {}

#[cfg(any(
    checkpoint_handler_user_syscall_error,
    checkpoint_handler_user_read_trace
))]
fn print_file_error_name(error: FileError) {
    let name = match error {
        FileError::NotReady => "NotReady",
        FileError::BadFd => "BadFd",
        FileError::AlreadyOpen => "AlreadyOpen",
        FileError::NotReadable => "NotReadable",
        FileError::NotWritable => "NotWritable",
        FileError::PathUnavailable => "PathUnavailable",
        FileError::BufferTooSmall => "BufferTooSmall",
        FileError::VfsBackendUnavailable => "VfsBackendUnavailable",
        FileError::BackendUnavailable => "BackendUnavailable",
        FileError::Unsupported => "Unsupported",
        FileError::InvalidArgument => "InvalidArgument",
        FileError::IllegalSeek => "IllegalSeek",
        FileError::NotTty => "NotTty",
        FileError::PermissionDenied => "PermissionDenied",
        FileError::TooManySymlinks => "TooManySymlinks",
        FileError::TooManyOpenFiles => "TooManyOpenFiles",
        FileError::NotDirectory => "NotDirectory",
    };
    crate::arch::riscv64::sbi::putstr(name);
}

fn print_trap_mode(frame: &TrapFrame) {
    if frame.sstatus & crate::arch::riscv64::csr::SSTATUS_SPP == 0 {
        crate::arch::riscv64::sbi::putstr("user");
    } else {
        crate::arch::riscv64::sbi::putstr("supervisor");
    }
}

fn print_syscall_arg_registers(frame: &TrapFrame) {
    crate::arch::riscv64::sbi::putstr(" a0=0x");
    print_hex(frame.reg(10));
    crate::arch::riscv64::sbi::putstr(" a1=0x");
    print_hex(frame.reg(11));
    crate::arch::riscv64::sbi::putstr(" a2=0x");
    print_hex(frame.reg(12));
    crate::arch::riscv64::sbi::putstr(" a3=0x");
    print_hex(frame.reg(13));
    crate::arch::riscv64::sbi::putstr(" a4=0x");
    print_hex(frame.reg(14));
    crate::arch::riscv64::sbi::putstr(" a5=0x");
    print_hex(frame.reg(15));
}

fn print_trap_return_address(frame: &TrapFrame) {
    crate::arch::riscv64::sbi::putstr(" ra=0x");
    print_hex(frame.reg(1));
}

fn print_trap_saved_registers(frame: &TrapFrame) {
    crate::arch::riscv64::sbi::putstr(" s2=0x");
    print_hex(frame.reg(18));
    crate::arch::riscv64::sbi::putstr(" s3=0x");
    print_hex(frame.reg(19));
    crate::arch::riscv64::sbi::putstr(" s4=0x");
    print_hex(frame.reg(20));
}

fn print_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        crate::arch::riscv64::sbi::putchar(HEX[(value >> shift) & 0xf]);
    }
}

fn print_decimal(mut value: usize) {
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

fn print_i64(value: i64) {
    if value < 0 {
        crate::arch::riscv64::sbi::putchar(b'-');
        print_decimal(value.unsigned_abs() as usize);
    } else {
        print_decimal(value as usize);
    }
}
