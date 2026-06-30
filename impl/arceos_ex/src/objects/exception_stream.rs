use core::sync::atomic::{AtomicU8, Ordering};

use crate::trace::{self, Checkpoint};

use super::{
    event_stream::{EventStream, TrapFrame},
    files::FileError,
    hwrng::HwRngError,
    init_stack::InitStack,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
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
const SYSCALL_FCNTL: usize = 25;
const SYSCALL_IOCTL: usize = 29;
const SYSCALL_FACCESSAT: usize = 48;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_GETDENTS64: usize = 61;
const SYSCALL_LSEEK: usize = 62;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_WRITEV: usize = 66;
const SYSCALL_READLINKAT: usize = 78;
const SYSCALL_NEWFSTATAT: usize = 79;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_EXIT_GROUP: usize = 94;
const SYSCALL_SET_TID_ADDRESS: usize = 96;
const SYSCALL_RT_SIGPROCMASK: usize = 135;
const SYSCALL_SETGID: usize = 144;
const SYSCALL_SETUID: usize = 146;
const SYSCALL_GETUID: usize = 174;
const SYSCALL_GETGID: usize = 176;
const SYSCALL_BRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MPROTECT: usize = 226;
const SYSCALL_GETRANDOM: usize = 278;
const USER_COPY_MAX: usize = 256;
const USER_IOV_MAX: usize = 4;
const USER_PATH_MAX: usize = crate::objects::files::FILE_PATH_MAX;
const AT_FDCWD: usize = usize::MAX - 99;
const ACCESS_X_OK: usize = 1;
const ACCESS_W_OK: usize = 2;
const ACCESS_R_OK: usize = 4;
const O_ACCMODE: usize = 0o3;
const O_LARGEFILE: usize = 0o100000;
const O_DIRECTORY: usize = 0o200000;
const O_CLOEXEC: usize = 0o2000000;
const AT_SYMLINK_NOFOLLOW: usize = 0x100;
const F_GETFD: usize = 1;
const F_SETFD: usize = 2;
const F_GETFL: usize = 3;
const FD_CLOEXEC: usize = 1;
const TIOCGWINSZ: usize = 0x5413;
const WINSIZE_SIZE: usize = 8;
const STAT_SIZE: usize = 128;
const RT_SIGSET_SIZE: usize = core::mem::size_of::<usize>();
const SIG_BLOCK: usize = 0;
const SIG_UNBLOCK: usize = 1;
const SIG_SETMASK: usize = 2;
const SIGKILL: usize = 9;
const SIGSTOP: usize = 19;
const UNBLOCKABLE_SIGNAL_MASK: usize = (1usize << (SIGKILL - 1)) | (1usize << (SIGSTOP - 1));
const EPERM: usize = 1;
const EACCES: usize = 13;
const EFAULT: usize = 14;
const EINVAL: usize = 22;
const EIO: usize = 5;
const ENOSYS: usize = 38;
const ENOMEM: usize = 12;
const EAGAIN: usize = 11;
const ENOENT: usize = 2;
const EOVERFLOW: usize = 75;
const EREMOTEIO: usize = 121;
const ESPIPE: usize = 29;
const ENOTTY: usize = 25;
const ELOOP: usize = 40;

static SYSCALL_TABLE_READY: AtomicU8 = AtomicU8::new(0);

pub struct SyscallTable {
    lifecycle: Lifecycle,
    #[allow(dead_code)]
    bound_to_exception: bool,
    write_supported: bool,
    writev_supported: bool,
    openat_supported: bool,
    getdents64_supported: bool,
    read_supported: bool,
    close_supported: bool,
    newfstatat_supported: bool,
    readlinkat_supported: bool,
    getrandom_supported: bool,
    getuid_supported: bool,
    getgid_supported: bool,
    setuid_supported: bool,
    setgid_supported: bool,
    rt_sigprocmask_supported: bool,
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
    exit_supported: bool,
    exit_group_supported: bool,
    write_usercopy_ready: bool,
    writev_usercopy_ready: bool,
    read_usercopy_ready: bool,
    getdents64_usercopy_ready: bool,
    path_usercopy_ready: bool,
    stat_usercopy_ready: bool,
    getrandom_usercopy_ready: bool,
    signal_mask_usercopy_ready: bool,
    ioctl_usercopy_ready: bool,
    write_routes_to_console: bool,
    writev_routes_to_files_struct: bool,
    openat_routes_to_files_struct: bool,
    getdents64_routes_to_files_struct: bool,
    read_routes_to_files_struct: bool,
    close_routes_to_files_struct: bool,
    newfstatat_routes_to_files_struct: bool,
    readlinkat_routes_to_files_struct: bool,
    getrandom_routes_to_hwrng_core: bool,
    getrandom_not_vfs_or_devfs_path: bool,
    getrandom_flags_first_slice_bound: bool,
    getrandom_full_random_core_deferred: bool,
    getuid_routes_to_user_init_process: bool,
    getgid_routes_to_user_init_process: bool,
    setuid_routes_to_user_init_process: bool,
    setgid_routes_to_user_init_process: bool,
    credentials_full_linux_model_deferred: bool,
    rt_sigprocmask_routes_to_user_init_process: bool,
    rt_sigprocmask_sigsetsize_bound: bool,
    rt_sigprocmask_unblockable_signals_cleared: bool,
    signal_delivery_deferred: bool,
    fstat_routes_to_files_struct: bool,
    fcntl_routes_to_files_struct: bool,
    ioctl_routes_to_files_struct: bool,
    faccessat_routes_to_files_struct: bool,
    lseek_routes_to_files_struct: bool,
    exit_records_status: bool,
    write_observed: AtomicU8,
    writev_observed: AtomicU8,
    openat_observed: AtomicU8,
    getdents64_observed: AtomicU8,
    read_observed: AtomicU8,
    close_observed: AtomicU8,
    newfstatat_observed: AtomicU8,
    readlinkat_observed: AtomicU8,
    getrandom_observed: AtomicU8,
    getuid_observed: AtomicU8,
    getgid_observed: AtomicU8,
    setuid_observed: AtomicU8,
    setgid_observed: AtomicU8,
    rt_sigprocmask_observed: AtomicU8,
    fstat_observed: AtomicU8,
    fcntl_observed: AtomicU8,
    ioctl_observed: AtomicU8,
    faccessat_observed: AtomicU8,
    lseek_observed: AtomicU8,
    set_tid_address_observed: AtomicU8,
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
            getdents64_supported: false,
            read_supported: false,
            close_supported: false,
            newfstatat_supported: false,
            readlinkat_supported: false,
            getrandom_supported: false,
            getuid_supported: false,
            getgid_supported: false,
            setuid_supported: false,
            setgid_supported: false,
            rt_sigprocmask_supported: false,
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
            exit_supported: false,
            exit_group_supported: false,
            write_usercopy_ready: false,
            writev_usercopy_ready: false,
            read_usercopy_ready: false,
            getdents64_usercopy_ready: false,
            path_usercopy_ready: false,
            stat_usercopy_ready: false,
            getrandom_usercopy_ready: false,
            signal_mask_usercopy_ready: false,
            ioctl_usercopy_ready: false,
            write_routes_to_console: false,
            writev_routes_to_files_struct: false,
            openat_routes_to_files_struct: false,
            getdents64_routes_to_files_struct: false,
            read_routes_to_files_struct: false,
            close_routes_to_files_struct: false,
            newfstatat_routes_to_files_struct: false,
            readlinkat_routes_to_files_struct: false,
            getrandom_routes_to_hwrng_core: false,
            getrandom_not_vfs_or_devfs_path: false,
            getrandom_flags_first_slice_bound: false,
            getrandom_full_random_core_deferred: false,
            getuid_routes_to_user_init_process: false,
            getgid_routes_to_user_init_process: false,
            setuid_routes_to_user_init_process: false,
            setgid_routes_to_user_init_process: false,
            credentials_full_linux_model_deferred: false,
            rt_sigprocmask_routes_to_user_init_process: false,
            rt_sigprocmask_sigsetsize_bound: false,
            rt_sigprocmask_unblockable_signals_cleared: false,
            signal_delivery_deferred: false,
            fstat_routes_to_files_struct: false,
            fcntl_routes_to_files_struct: false,
            ioctl_routes_to_files_struct: false,
            faccessat_routes_to_files_struct: false,
            lseek_routes_to_files_struct: false,
            exit_records_status: false,
            write_observed: AtomicU8::new(0),
            writev_observed: AtomicU8::new(0),
            openat_observed: AtomicU8::new(0),
            getdents64_observed: AtomicU8::new(0),
            read_observed: AtomicU8::new(0),
            close_observed: AtomicU8::new(0),
            newfstatat_observed: AtomicU8::new(0),
            readlinkat_observed: AtomicU8::new(0),
            getrandom_observed: AtomicU8::new(0),
            getuid_observed: AtomicU8::new(0),
            getgid_observed: AtomicU8::new(0),
            setuid_observed: AtomicU8::new(0),
            setgid_observed: AtomicU8::new(0),
            rt_sigprocmask_observed: AtomicU8::new(0),
            fstat_observed: AtomicU8::new(0),
            fcntl_observed: AtomicU8::new(0),
            ioctl_observed: AtomicU8::new(0),
            faccessat_observed: AtomicU8::new(0),
            lseek_observed: AtomicU8::new(0),
            set_tid_address_observed: AtomicU8::new(0),
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
    pub const fn getdents64_supported(&self) -> bool {
        self.getdents64_supported
    }

    #[allow(dead_code)]
    pub const fn read_supported(&self) -> bool {
        self.read_supported
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
    pub const fn signal_delivery_deferred(&self) -> bool {
        self.signal_delivery_deferred
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
    pub fn getdents64_observed(&self) -> bool {
        self.getdents64_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn read_observed(&self) -> bool {
        self.read_observed.load(Ordering::Acquire) != 0
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
    pub fn getrandom_observed(&self) -> bool {
        self.getrandom_observed.load(Ordering::Acquire) != 0
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
        self.getdents64_supported = true;
        self.read_supported = true;
        self.close_supported = true;
        self.newfstatat_supported = true;
        self.readlinkat_supported = true;
        self.getrandom_supported = true;
        self.getuid_supported = true;
        self.getgid_supported = true;
        self.setuid_supported = true;
        self.setgid_supported = true;
        self.rt_sigprocmask_supported = true;
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
        self.exit_supported = true;
        self.exit_group_supported = true;
        self.write_usercopy_ready = true;
        self.writev_usercopy_ready = true;
        self.read_usercopy_ready = true;
        self.getdents64_usercopy_ready = true;
        self.path_usercopy_ready = true;
        self.stat_usercopy_ready = true;
        self.getrandom_usercopy_ready = true;
        self.signal_mask_usercopy_ready = true;
        self.ioctl_usercopy_ready = true;
        self.write_routes_to_console = true;
        self.writev_routes_to_files_struct = true;
        self.openat_routes_to_files_struct = true;
        self.getdents64_routes_to_files_struct = true;
        self.read_routes_to_files_struct = true;
        self.close_routes_to_files_struct = true;
        self.newfstatat_routes_to_files_struct = true;
        self.readlinkat_routes_to_files_struct = true;
        self.getrandom_routes_to_hwrng_core = true;
        self.getrandom_not_vfs_or_devfs_path = true;
        self.getrandom_flags_first_slice_bound = true;
        self.getrandom_full_random_core_deferred = true;
        self.getuid_routes_to_user_init_process = true;
        self.getgid_routes_to_user_init_process = true;
        self.setuid_routes_to_user_init_process = true;
        self.setgid_routes_to_user_init_process = true;
        self.credentials_full_linux_model_deferred = true;
        self.rt_sigprocmask_routes_to_user_init_process = true;
        self.rt_sigprocmask_sigsetsize_bound = true;
        self.rt_sigprocmask_unblockable_signals_cleared = true;
        self.signal_delivery_deferred = true;
        self.fstat_routes_to_files_struct = true;
        self.fcntl_routes_to_files_struct = true;
        self.ioctl_routes_to_files_struct = true;
        self.faccessat_routes_to_files_struct = true;
        self.lseek_routes_to_files_struct = true;
        self.exit_records_status = true;
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

    pub fn exit(&self, frame: &mut TrapFrame) -> ! {
        if self.lifecycle.state() != State::Ready
            || !self.exit_supported
            || !self.exit_records_status
        {
            panic_dispatch("syscall exit table entry not ready\n");
        }

        syscall_table_exit(self, frame)
    }

    pub fn exit_group(&self, frame: &mut TrapFrame) -> ! {
        if self.lifecycle.state() != State::Ready
            || !self.exit_group_supported
            || !self.exit_records_status
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
        SYSCALL_FCNTL => table.fcntl(frame),
        SYSCALL_IOCTL => table.ioctl(frame),
        SYSCALL_FACCESSAT => table.faccessat(frame),
        SYSCALL_OPENAT => table.openat(frame),
        SYSCALL_CLOSE => table.close(frame),
        SYSCALL_GETDENTS64 => table.getdents64(frame),
        SYSCALL_LSEEK => table.lseek(frame),
        SYSCALL_READ => table.read(frame),
        SYSCALL_WRITE => table.write(frame),
        SYSCALL_WRITEV => table.writev(frame),
        SYSCALL_READLINKAT => table.readlinkat(frame),
        SYSCALL_NEWFSTATAT => table.newfstatat(frame),
        SYSCALL_FSTAT => table.fstat(frame),
        SYSCALL_SET_TID_ADDRESS => table.set_tid_address(frame),
        SYSCALL_RT_SIGPROCMASK => table.rt_sigprocmask(frame),
        SYSCALL_SETGID => table.setgid(frame),
        SYSCALL_SETUID => table.setuid(frame),
        SYSCALL_GETUID => table.getuid(frame),
        SYSCALL_GETGID => table.getgid(frame),
        SYSCALL_BRK => table.brk(frame),
        SYSCALL_MMAP => table.mmap(frame),
        SYSCALL_MPROTECT => table.mprotect(frame),
        SYSCALL_MUNMAP => table.munmap(frame),
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

fn syscall_table_openat(table: &SyscallTable, frame: &mut TrapFrame) {
    let dirfd = frame.reg(10);
    let path_ptr = frame.reg(11);
    let flags = frame.reg(12);
    let supported_flags = O_LARGEFILE | O_DIRECTORY | O_CLOEXEC;
    if dirfd != AT_FDCWD || flags & !supported_flags != 0 || flags & O_ACCMODE != 0 {
        complete_error_syscall(frame, EINVAL);
        return;
    }

    let mut path = [0u8; USER_PATH_MAX];
    let Some(path_len) = copy_cstr_from_user(path_ptr, &mut path) else {
        complete_error_syscall(frame, EFAULT);
        return;
    };

    let ctx = crate::context::context();
    let fd_result = if flags & O_DIRECTORY != 0 {
        ctx.files_struct.open_directory_path(
            &ctx.fs_struct,
            &mut ctx.vfs_core,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &ctx.kernel_image,
            &path[..path_len],
            flags as u32,
        )
    } else {
        ctx.files_struct.open_regular_path(
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
            print_path_syscall_error_detail(error, &path[..path_len]);
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
        complete_successful_syscall(frame, 0);
        return;
    }

    let mut buffer = [0u8; USER_COPY_MAX];
    let ctx = crate::context::context();
    let Ok(read) = ctx.files_struct.read_fd(fd, &mut buffer[..len]) else {
        complete_unsupported_syscall(frame);
        return;
    };
    if !copy_to_user(user_ptr, &buffer[..read]) {
        complete_unsupported_syscall(frame);
        return;
    }

    table.read_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableRead, crate::context::context_ref());
    complete_successful_syscall(frame, read);
}

fn syscall_table_close(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let ctx = crate::context::context();
    if ctx.files_struct.close_fd(fd).is_err() {
        complete_unsupported_syscall(frame);
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

fn syscall_table_getuid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(uid) = crate::context::context().user_init_process.read_uid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getuid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, uid);
}

fn syscall_table_getgid(table: &SyscallTable, frame: &mut TrapFrame) {
    let Some(gid) = crate::context::context().user_init_process.read_gid() else {
        complete_unsupported_syscall(frame);
        return;
    };

    table.getgid_observed.store(1, Ordering::Release);
    complete_successful_syscall(frame, gid);
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
        F_GETFL => {
            let ctx = crate::context::context_ref();
            let flags = match ctx.files_struct.fcntl_getfl_fd(fd) {
                Ok(flags) => flags,
                Err(error) => {
                    complete_error_syscall(frame, file_error_to_errno(error));
                    return;
                }
            };

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, flags as usize);
        }
        F_GETFD => {
            let ctx = crate::context::context_ref();
            let flags = match ctx.files_struct.fcntl_getfd_fd(fd) {
                Ok(flags) => flags,
                Err(error) => {
                    complete_error_syscall(frame, file_error_to_errno(error));
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
                complete_error_syscall(frame, file_error_to_errno(error));
                return;
            }

            table.fcntl_observed.store(1, Ordering::Release);
            complete_successful_syscall(frame, 0);
        }
        _ => complete_error_syscall(frame, EINVAL),
    }
}

fn syscall_table_ioctl(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let cmd = frame.reg(11);
    let arg = frame.reg(12);
    let ctx = crate::context::context_ref();
    if let Err(error) = ctx.files_struct.ioctl_validate_fd(fd) {
        complete_error_syscall(frame, file_error_to_errno(error));
        return;
    }
    if cmd != TIOCGWINSZ {
        complete_error_syscall(frame, ENOTTY);
        return;
    }

    let winsize = match ctx.files_struct.ioctl_tiocgwinsz_fd(fd) {
        Ok(winsize) => winsize,
        Err(error) => {
            complete_error_syscall(frame, file_error_to_errno(error));
            return;
        }
    };

    let mut buffer = [0u8; WINSIZE_SIZE];
    write_u16(&mut buffer, 0, winsize.0);
    write_u16(&mut buffer, 2, winsize.1);
    write_u16(&mut buffer, 4, winsize.2);
    write_u16(&mut buffer, 6, winsize.3);
    if !copy_to_user(arg, &buffer) {
        complete_error_syscall(frame, EFAULT);
        return;
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
    let _prot = frame.reg(12);
    let flags = frame.reg(13);
    let fd = frame.reg(14);
    let offset = frame.reg(15);
    let ctx = crate::context::context();
    let Some(mapped) = ctx
        .user_address_space
        .user_mmap(addr, len, flags, fd, offset)
    else {
        complete_error_syscall(frame, ENOMEM);
        return;
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

fn syscall_table_exit(table: &SyscallTable, frame: &mut TrapFrame) -> ! {
    table.exit_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableExit, crate::context::context_ref());
    let status = frame.reg(10);
    crate::arch::riscv64::sbi::putstr("user exit status=");
    print_decimal(status);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn copy_from_user(user_ptr: usize, dst: &mut [u8]) -> bool {
    if dst.is_empty() {
        return true;
    }
    if user_ptr == 0 || user_ptr.checked_add(dst.len()).is_none() {
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
    if user_ptr == 0 || user_ptr.checked_add(src.len()).is_none() {
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

fn read_user_usize(user_ptr: usize) -> Option<usize> {
    let mut bytes = [0u8; core::mem::size_of::<usize>()];
    if copy_from_user(user_ptr, &mut bytes) {
        Some(usize::from_le_bytes(bytes))
    } else {
        None
    }
}

fn write_user_usize(user_ptr: usize, value: usize) -> bool {
    copy_to_user(user_ptr, &value.to_le_bytes())
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
    crate::arch::riscv64::sbi::putstr(" gp=0x");
    print_hex(crate::arch::riscv64::csr::read_gp());
    crate::arch::riscv64::sbi::putstr(" tp=0x");
    print_hex(crate::arch::riscv64::csr::read_tp());
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn print_unsupported_syscall_diagnostic(frame: &TrapFrame) {
    crate::arch::riscv64::sbi::putstr("unsupported syscall");
    crate::arch::riscv64::sbi::putstr(" nr=");
    print_decimal(frame.reg(17));
    crate::arch::riscv64::sbi::putstr(" mode=");
    print_trap_mode(frame);
    print_syscall_arg_registers(frame);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
    crate::arch::riscv64::sbi::putchar(b'\n');
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

#[cfg(checkpoint_handler_user_syscall_error)]
fn print_syscall_name(nr: usize) {
    let name = match nr {
        SYSCALL_FCNTL => "fcntl",
        SYSCALL_IOCTL => "ioctl",
        SYSCALL_FACCESSAT => "faccessat",
        SYSCALL_OPENAT => "openat",
        SYSCALL_CLOSE => "close",
        SYSCALL_GETDENTS64 => "getdents64",
        SYSCALL_LSEEK => "lseek",
        SYSCALL_READ => "read",
        SYSCALL_WRITE => "write",
        SYSCALL_WRITEV => "writev",
        SYSCALL_READLINKAT => "readlinkat",
        SYSCALL_NEWFSTATAT => "newfstatat",
        SYSCALL_FSTAT => "fstat",
        SYSCALL_SET_TID_ADDRESS => "set_tid_address",
        SYSCALL_RT_SIGPROCMASK => "rt_sigprocmask",
        SYSCALL_SETGID => "setgid",
        SYSCALL_SETUID => "setuid",
        SYSCALL_GETUID => "getuid",
        SYSCALL_GETGID => "getgid",
        SYSCALL_BRK => "brk",
        SYSCALL_MUNMAP => "munmap",
        SYSCALL_MMAP => "mmap",
        SYSCALL_MPROTECT => "mprotect",
        SYSCALL_GETRANDOM => "getrandom",
        SYSCALL_EXIT => "exit",
        SYSCALL_EXIT_GROUP => "exit_group",
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
    for &byte in path {
        if byte.is_ascii_graphic() || byte == b' ' {
            crate::arch::riscv64::sbi::putchar(byte);
        } else {
            crate::arch::riscv64::sbi::putchar(b'?');
        }
    }
    crate::arch::riscv64::sbi::putstr("\"\n");
}

#[cfg(not(checkpoint_handler_user_syscall_error))]
fn print_path_syscall_error_detail(_error: FileError, _path: &[u8]) {}

#[cfg(checkpoint_handler_user_syscall_error)]
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
