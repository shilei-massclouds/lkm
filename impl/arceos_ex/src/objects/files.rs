use core::sync::atomic::{AtomicUsize, Ordering};

use super::{
    block_device::BlockDeviceRegistry,
    ext2::{Ext2FileSystem, Ext2FileType},
    kernel_image::KernelImage,
    rest_init::KernelInitTask,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vfs::{FileRef, FsStruct, VfsCore, VfsError, VfsInodeKind},
    virtio_blk,
};

pub const STDIN_FD: usize = 0;
pub const STDOUT_FD: usize = 1;
pub const STDERR_FD: usize = 2;
pub const REGULAR0_FD: usize = 3;
pub const FILE_PATH_MAX: usize = 128;
pub const REGULAR_FILE_BUFFER_SIZE: usize = 4096;
pub const LINUX_DIRENT64_HEADER_SIZE: usize = 19;
pub const TERMIOS_SIZE: usize = 36;
pub const STDIN_READY_FIXTURE: &[u8] = b"stdin\n";
const FILE_FD_COUNT: usize = 16;
pub const FILE_POLLIN: u16 = 0x0001;
pub const FILE_POLLOUT: u16 = 0x0004;
pub const FILE_POLLERR: u16 = 0x0008;
pub const FILE_POLLHUP: u16 = 0x0010;
pub const FILE_POLLRDNORM: u16 = 0x0040;
pub const FILE_POLLWRNORM: u16 = 0x0100;
const DT_UNKNOWN: u8 = 0;
const DT_DIR: u8 = 4;
const DT_REG: u8 = 8;
const DT_LNK: u8 = 10;
const DEV_TTY_PATH: &[u8] = b"/dev/tty";
const FILE_O_RDONLY: u32 = 0;
const FILE_O_WRONLY: u32 = 1;
const FILE_O_RDWR: u32 = 2;
const FILE_O_ACCMODE: u32 = 0o3;
const FILE_O_LARGEFILE: u32 = 0o100000;
const FILE_O_DIRECTORY: u32 = 0o200000;
const FILE_O_CLOEXEC: u32 = 0o2000000;
const FILE_FD_CLOEXEC: u32 = 1;
const TERMIOS_ICRNL: u32 = 0x100;
const TERMIOS_IXON: u32 = 0x400;
const TERMIOS_OPOST: u32 = 0x1;
const TERMIOS_ONLCR: u32 = 0x4;
const TERMIOS_B38400: u32 = 0x0000000f;
const TERMIOS_CS8: u32 = 0x00000030;
const TERMIOS_CREAD: u32 = 0x00000080;
const TERMIOS_HUPCL: u32 = 0x00000400;
const TERMIOS_ISIG: u32 = 0x00001;
const TERMIOS_ICANON: u32 = 0x00002;
const TERMIOS_ECHO: u32 = 0x00008;
const TERMIOS_ECHOE: u32 = 0x00010;
const TERMIOS_ECHOK: u32 = 0x00020;
const TERMIOS_ECHOCTL: u32 = 0x00200;
const TERMIOS_ECHOKE: u32 = 0x00800;
const TERMIOS_IEXTEN: u32 = 0x08000;
const SEEK_SET: usize = 0;
const SEEK_CUR: usize = 1;
const SEEK_END: usize = 2;
pub const TTY_WINSIZE_ROW: u16 = 24;
pub const TTY_WINSIZE_COL: u16 = 80;
pub const TTY_WINSIZE_XPIXEL: u16 = 0;
pub const TTY_WINSIZE_YPIXEL: u16 = 0;

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FileBackendKind {
    CharDevice,
    RegularFile,
    BlockDevice,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FdRef {
    Stdin,
    Stdout,
    Stderr,
    Regular0,
}

impl FdRef {
    const fn index(self) -> usize {
        match self {
            Self::Stdin => 0,
            Self::Stdout => 1,
            Self::Stderr => 2,
            Self::Regular0 => 3,
        }
    }

    pub const fn from_fd(fd: usize) -> Option<Self> {
        match fd {
            STDIN_FD => Some(Self::Stdin),
            STDOUT_FD => Some(Self::Stdout),
            STDERR_FD => Some(Self::Stderr),
            REGULAR0_FD => Some(Self::Regular0),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FileError {
    NotReady,
    BadFd,
    AlreadyOpen,
    NotReadable,
    NotWritable,
    PathUnavailable,
    BufferTooSmall,
    VfsBackendUnavailable,
    BackendUnavailable,
    Unsupported,
    InvalidArgument,
    IllegalSeek,
    NotTty,
    PermissionDenied,
    TooManySymlinks,
    TooManyOpenFiles,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FilesystemFdKind {
    None,
    RegularFile,
    Directory,
}

pub type FileResult<T> = Result<T, FileError>;

fn vfs_error_to_file_error(error: VfsError) -> FileError {
    match error {
        VfsError::NotFound => FileError::PathUnavailable,
        VfsError::ShortBuffer => FileError::BufferTooSmall,
        VfsError::Backend => FileError::VfsBackendUnavailable,
        VfsError::SymlinkLoop => FileError::TooManySymlinks,
        _ => FileError::BackendUnavailable,
    }
}

fn vfs_readlink_error_to_file_error(error: VfsError) -> FileError {
    match error {
        VfsError::NotFound => FileError::PathUnavailable,
        VfsError::UnsupportedPath | VfsError::NotFile => FileError::InvalidArgument,
        VfsError::ShortBuffer => FileError::BufferTooSmall,
        VfsError::Backend => FileError::VfsBackendUnavailable,
        VfsError::SymlinkLoop => FileError::TooManySymlinks,
        _ => FileError::BackendUnavailable,
    }
}

#[derive(Clone, Copy)]
struct FileDescriptorEntry {
    ofd: OpenFileDescriptionRef,
    readable: bool,
    writable: bool,
    flags: u32,
    close_on_exec: bool,
}

impl FileDescriptorEntry {
    const fn stdio(ofd: OpenFileDescriptionRef, readable: bool, writable: bool) -> Self {
        let access_mode = if readable && writable {
            FILE_O_RDWR
        } else if writable {
            FILE_O_WRONLY
        } else {
            FILE_O_RDONLY
        };
        Self {
            ofd,
            readable,
            writable,
            flags: access_mode,
            close_on_exec: false,
        }
    }

    const fn regular(ofd: OpenFileDescriptionRef, flags: u32, close_on_exec: bool) -> Self {
        Self {
            ofd,
            readable: true,
            writable: false,
            flags,
            close_on_exec,
        }
    }

    const fn opened(
        ofd: OpenFileDescriptionRef,
        readable: bool,
        writable: bool,
        flags: u32,
        close_on_exec: bool,
    ) -> Self {
        Self {
            ofd,
            readable,
            writable,
            flags,
            close_on_exec,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OpenFileDescriptionRef {
    Stdin,
    Stdout,
    Stderr,
    Regular0,
    Tty0,
}

#[derive(Clone, Copy)]
pub struct FileStat {
    size: usize,
    mode: u32,
}

impl FileStat {
    const fn new(size: usize, kind: VfsInodeKind) -> Self {
        match kind {
            VfsInodeKind::Directory => Self::directory(size),
            VfsInodeKind::RegularFile => Self::regular(size),
            VfsInodeKind::DeviceNode => Self {
                size,
                mode: 0o020444,
            },
            VfsInodeKind::Symlink => Self {
                size,
                mode: 0o120777,
            },
        }
    }

    const fn regular(size: usize) -> Self {
        Self {
            size,
            mode: 0o100444,
        }
    }

    const fn directory(size: usize) -> Self {
        Self {
            size,
            mode: 0o040555,
        }
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn mode(&self) -> u32 {
        self.mode
    }
}

pub struct FileBackend {
    lifecycle: Lifecycle,
    kind: FileBackendKind,
    allocated: bool,
    char_device_console_bound: bool,
    char_device_write_supported: bool,
    char_device_read_supported: bool,
    regular_file_bound: bool,
    regular_file_read_supported: bool,
    regular_file_stat_supported: bool,
    regular_file_deferred: bool,
    block_device_deferred: bool,
    write_to_console: AtomicUsize,
    last_write_len: AtomicUsize,
    char_device_read_returns_data: AtomicUsize,
    regular_file_read_returns_data: AtomicUsize,
    regular_file_stat_returns_metadata: AtomicUsize,
    last_read_len: AtomicUsize,
    last_stat_size: AtomicUsize,
}

#[allow(dead_code)]
impl FileBackend {
    const fn new(kind: FileBackendKind) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kind,
            allocated: false,
            char_device_console_bound: false,
            char_device_write_supported: false,
            char_device_read_supported: false,
            regular_file_bound: false,
            regular_file_read_supported: false,
            regular_file_stat_supported: false,
            regular_file_deferred: false,
            block_device_deferred: false,
            write_to_console: AtomicUsize::new(0),
            last_write_len: AtomicUsize::new(0),
            char_device_read_returns_data: AtomicUsize::new(0),
            regular_file_read_returns_data: AtomicUsize::new(0),
            regular_file_stat_returns_metadata: AtomicUsize::new(0),
            last_read_len: AtomicUsize::new(0),
            last_stat_size: AtomicUsize::new(0),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kind(&self) -> FileBackendKind {
        self.kind
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn char_device_console_bound(&self) -> bool {
        self.char_device_console_bound
    }

    pub const fn char_device_write_supported(&self) -> bool {
        self.char_device_write_supported
    }

    pub const fn char_device_read_supported(&self) -> bool {
        self.char_device_read_supported
    }

    pub const fn regular_file_bound(&self) -> bool {
        self.regular_file_bound
    }

    pub const fn regular_file_read_supported(&self) -> bool {
        self.regular_file_read_supported
    }

    pub const fn regular_file_stat_supported(&self) -> bool {
        self.regular_file_stat_supported
    }

    pub const fn regular_file_deferred(&self) -> bool {
        self.regular_file_deferred
    }

    pub const fn block_device_deferred(&self) -> bool {
        self.block_device_deferred
    }

    pub fn write_to_console(&self) -> bool {
        self.write_to_console.load(Ordering::Acquire) != 0
    }

    pub fn last_write_len(&self) -> usize {
        self.last_write_len.load(Ordering::Acquire)
    }

    pub fn regular_file_read_returns_data(&self) -> bool {
        self.regular_file_read_returns_data.load(Ordering::Acquire) != 0
    }

    pub fn char_device_read_returns_data(&self) -> bool {
        self.char_device_read_returns_data.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_stat_returns_metadata(&self) -> bool {
        self.regular_file_stat_returns_metadata
            .load(Ordering::Acquire)
            != 0
    }

    pub fn last_read_len(&self) -> usize {
        self.last_read_len.load(Ordering::Acquire)
    }

    pub fn last_stat_size(&self) -> usize {
        self.last_stat_size.load(Ordering::Acquire)
    }

    fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || self.kind != FileBackendKind::CharDevice {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.char_device_console_bound = true;
        self.char_device_write_supported = true;
        self.char_device_read_supported = true;
        self.regular_file_deferred = true;
        self.block_device_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn bind_regular_file(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || self.kind != FileBackendKind::RegularFile {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.regular_file_bound = true;
        self.regular_file_read_supported = true;
        self.regular_file_stat_supported = true;
        self.block_device_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn write_char_device(&self, bytes: &[u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_write_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        crate::objects::printk::write_bytes(bytes);
        self.last_write_len.store(bytes.len(), Ordering::Release);
        self.write_to_console.fetch_add(1, Ordering::AcqRel);
        Ok(bytes.len())
    }

    fn read_regular_file(&self, len: usize) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::RegularFile
            || !self.regular_file_read_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        self.last_read_len.store(len, Ordering::Release);
        if len != 0 {
            self.regular_file_read_returns_data
                .fetch_add(1, Ordering::AcqRel);
        }
        Ok(len)
    }

    fn read_char_device(&self, buffer: &mut [u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_read_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        let len = crate::objects::ns16550a::read_tty_ready_data(buffer)
            .ok_or(FileError::BackendUnavailable)?;
        self.last_read_len.store(len, Ordering::Release);
        if len != 0 {
            self.char_device_read_returns_data
                .fetch_add(1, Ordering::AcqRel);
        }
        Ok(len)
    }

    fn char_device_read_ready(&self) -> FileResult<bool> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_read_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        crate::objects::ns16550a::tty_ready_data_available().ok_or(FileError::BackendUnavailable)
    }

    fn char_device_write_ready(&self) -> FileResult<bool> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_write_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        Ok(true)
    }

    fn stat_regular_file(&self, stat: FileStat) -> FileResult<FileStat> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::RegularFile
            || !self.regular_file_stat_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        self.last_stat_size.store(stat.size(), Ordering::Release);
        self.regular_file_stat_returns_metadata
            .fetch_add(1, Ordering::AcqRel);
        Ok(stat)
    }
}

pub struct OpenFileDescription {
    lifecycle: Lifecycle,
    allocated: bool,
    backend_bound: bool,
    flags_bound: bool,
    readable: bool,
    writable: bool,
    offset_ready: bool,
    write_dispatches_backend: AtomicUsize,
    read_dispatches_backend: AtomicUsize,
    write_observed: AtomicUsize,
    read_observed: AtomicUsize,
    last_write_len: AtomicUsize,
    last_read_len: AtomicUsize,
}

#[allow(dead_code)]
impl OpenFileDescription {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            backend_bound: false,
            flags_bound: false,
            readable: false,
            writable: false,
            offset_ready: false,
            write_dispatches_backend: AtomicUsize::new(0),
            read_dispatches_backend: AtomicUsize::new(0),
            write_observed: AtomicUsize::new(0),
            read_observed: AtomicUsize::new(0),
            last_write_len: AtomicUsize::new(0),
            last_read_len: AtomicUsize::new(0),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn backend_bound(&self) -> bool {
        self.backend_bound
    }

    pub const fn flags_bound(&self) -> bool {
        self.flags_bound
    }

    pub const fn readable(&self) -> bool {
        self.readable
    }

    pub const fn writable(&self) -> bool {
        self.writable
    }

    pub const fn offset_ready(&self) -> bool {
        self.offset_ready
    }

    pub fn write_dispatches_backend(&self) -> bool {
        self.write_dispatches_backend.load(Ordering::Acquire) != 0
    }

    pub fn read_dispatches_backend(&self) -> bool {
        self.read_dispatches_backend.load(Ordering::Acquire) != 0
    }

    pub fn write_observed(&self) -> bool {
        self.write_observed.load(Ordering::Acquire) != 0
    }

    pub fn read_observed(&self) -> bool {
        self.read_observed.load(Ordering::Acquire) != 0
    }

    pub fn last_write_len(&self) -> usize {
        self.last_write_len.load(Ordering::Acquire)
    }

    pub fn last_read_len(&self) -> usize {
        self.last_read_len.load(Ordering::Acquire)
    }

    fn setup_stdio(
        &mut self,
        backend: &FileBackend,
        readable: bool,
        writable: bool,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || backend.state() != State::Ready
            || backend.kind() != FileBackendKind::CharDevice
            || !backend.char_device_console_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.backend_bound = true;
        self.flags_bound = true;
        self.readable = readable;
        self.writable = writable;
        self.offset_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn setup_regular(&mut self, backend: &FileBackend) -> EventResult {
        if self.lifecycle.state() != State::Base
            || backend.state() != State::Ready
            || backend.kind() != FileBackendKind::RegularFile
            || !backend.regular_file_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.backend_bound = true;
        self.flags_bound = true;
        self.readable = true;
        self.writable = false;
        self.offset_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn write(&self, backend: &FileBackend, bytes: &[u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.backend_bound {
            return Err(FileError::NotReady);
        }
        if !self.writable {
            return Err(FileError::NotWritable);
        }

        let written = backend.write_char_device(bytes)?;
        self.last_write_len.store(written, Ordering::Release);
        self.write_dispatches_backend.fetch_add(1, Ordering::AcqRel);
        self.write_observed.fetch_add(1, Ordering::AcqRel);
        Ok(written)
    }

    fn read(&self, backend: &FileBackend, len: usize) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.backend_bound {
            return Err(FileError::NotReady);
        }
        if !self.readable {
            return Err(FileError::NotReadable);
        }

        let read = backend.read_regular_file(len)?;
        self.last_read_len.store(read, Ordering::Release);
        self.read_dispatches_backend.fetch_add(1, Ordering::AcqRel);
        self.read_observed.fetch_add(1, Ordering::AcqRel);
        Ok(read)
    }

    fn read_char_device(&self, backend: &FileBackend, buffer: &mut [u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.backend_bound {
            return Err(FileError::NotReady);
        }
        if !self.readable {
            return Err(FileError::NotReadable);
        }

        let read = backend.read_char_device(buffer)?;
        self.last_read_len.store(read, Ordering::Release);
        self.read_dispatches_backend.fetch_add(1, Ordering::AcqRel);
        self.read_observed.fetch_add(1, Ordering::AcqRel);
        Ok(read)
    }
}

pub struct FileDescriptorTable {
    lifecycle: Lifecycle,
    allocated: bool,
    capacity_bound: bool,
    stdio_fds_bound: bool,
    entries: [Option<FileDescriptorEntry>; FILE_FD_COUNT],
    lookup_returns: AtomicUsize,
    fd_installed: AtomicUsize,
    fd_closed: AtomicUsize,
}

#[allow(dead_code)]
impl FileDescriptorTable {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            capacity_bound: false,
            stdio_fds_bound: false,
            entries: [None; FILE_FD_COUNT],
            lookup_returns: AtomicUsize::new(0),
            fd_installed: AtomicUsize::new(0),
            fd_closed: AtomicUsize::new(0),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn capacity_bound(&self) -> bool {
        self.capacity_bound
    }

    pub const fn stdio_fds_bound(&self) -> bool {
        self.stdio_fds_bound
    }

    pub fn lookup_returns(&self) -> bool {
        self.lookup_returns.load(Ordering::Acquire) != 0
    }

    pub fn fd_installed(&self) -> bool {
        self.fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn fd_closed(&self) -> bool {
        self.fd_closed.load(Ordering::Acquire) != 0
    }

    const fn fd_bound(&self, fd: FdRef) -> bool {
        self.entries[fd.index()].is_some()
    }

    fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.capacity_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn install_stdio(
        &mut self,
        stdin: &OpenFileDescription,
        stdout: &OpenFileDescription,
        stderr: &OpenFileDescription,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || stdin.state() != State::Ready
            || stdout.state() != State::Ready
            || stderr.state() != State::Ready
            || !stdout.writable()
            || !stderr.writable()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.entries[FdRef::Stdin.index()] = Some(FileDescriptorEntry::stdio(
            OpenFileDescriptionRef::Stdin,
            true,
            false,
        ));
        self.entries[FdRef::Stdout.index()] = Some(FileDescriptorEntry::stdio(
            OpenFileDescriptionRef::Stdout,
            false,
            true,
        ));
        self.entries[FdRef::Stderr.index()] = Some(FileDescriptorEntry::stdio(
            OpenFileDescriptionRef::Stderr,
            false,
            true,
        ));
        self.stdio_fds_bound = true;
        Ok(())
    }

    fn lookup(&self, fd: usize) -> FileResult<FileDescriptorEntry> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        let entry = self.entries[fd].ok_or(FileError::BadFd)?;
        self.lookup_returns.fetch_add(1, Ordering::AcqRel);
        Ok(entry)
    }

    fn install_regular(
        &mut self,
        ofd: &OpenFileDescription,
        flags: u32,
        close_on_exec: bool,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || ofd.state() != State::Ready || !ofd.readable()
        {
            return Err(FileError::NotReady);
        }
        if self.entries[FdRef::Regular0.index()].is_some() {
            return Err(FileError::AlreadyOpen);
        }

        self.entries[FdRef::Regular0.index()] = Some(FileDescriptorEntry::regular(
            OpenFileDescriptionRef::Regular0,
            flags,
            close_on_exec,
        ));
        self.fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(REGULAR0_FD)
    }

    fn install_opened(
        &mut self,
        ofd: &OpenFileDescription,
        ofd_ref: OpenFileDescriptionRef,
        readable: bool,
        writable: bool,
        flags: u32,
        close_on_exec: bool,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || ofd.state() != State::Ready
            || (!readable && !writable)
        {
            return Err(FileError::NotReady);
        }
        if self.entries[FdRef::Regular0.index()].is_some() {
            return Err(FileError::AlreadyOpen);
        }

        self.entries[FdRef::Regular0.index()] = Some(FileDescriptorEntry::opened(
            ofd_ref,
            readable,
            writable,
            flags,
            close_on_exec,
        ));
        self.fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(REGULAR0_FD)
    }

    fn dup_fd(&mut self, fd: usize, min_fd: usize, close_on_exec: bool) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }
        if min_fd >= FILE_FD_COUNT {
            return Err(FileError::InvalidArgument);
        }

        let source = self.lookup(fd)?;
        let mut candidate = min_fd;
        while candidate < FILE_FD_COUNT {
            if self.entries[candidate].is_none() {
                let mut duplicate = source;
                duplicate.close_on_exec = close_on_exec;
                self.entries[candidate] = Some(duplicate);
                self.fd_installed.fetch_add(1, Ordering::AcqRel);
                return Ok(candidate);
            }
            candidate += 1;
        }
        Err(FileError::TooManyOpenFiles)
    }

    fn get_fd_flags(&self, fd: usize) -> FileResult<u32> {
        let entry = self.lookup(fd)?;
        Ok(if entry.close_on_exec {
            FILE_FD_CLOEXEC
        } else {
            0
        })
    }

    fn set_fd_flags(&mut self, fd: usize, flags: u32) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        let entry = self.entries[fd].as_mut().ok_or(FileError::BadFd)?;
        entry.close_on_exec = flags & FILE_FD_CLOEXEC != 0;
        self.lookup_returns.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn close(&mut self, fd: usize) -> FileResult<FileDescriptorEntry> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        if matches!(fd, STDIN_FD | STDOUT_FD | STDERR_FD) {
            return Err(FileError::Unsupported);
        }
        let entry = self.entries[fd].ok_or(FileError::BadFd)?;

        self.entries[fd] = None;
        self.fd_closed.fetch_add(1, Ordering::AcqRel);
        Ok(entry)
    }
}

pub struct FilesStruct {
    lifecycle: Lifecycle,
    fd_table: FileDescriptorTable,
    stdin: OpenFileDescription,
    stdout: OpenFileDescription,
    stderr: OpenFileDescription,
    regular0: OpenFileDescription,
    tty0: OpenFileDescription,
    stdin_backend: FileBackend,
    stdout_backend: FileBackend,
    stderr_backend: FileBackend,
    regular0_backend: FileBackend,
    tty0_backend: FileBackend,
    regular0_buffer: [u8; REGULAR_FILE_BUFFER_SIZE],
    regular0_len: usize,
    regular0_offset: usize,
    regular0_path: [u8; FILE_PATH_MAX],
    regular0_path_len: usize,
    filesystem0_kind: FilesystemFdKind,
    directory0_file_ref: Option<FileRef>,
    directory0_offset: usize,
    directory0_last_getdents_len: usize,
    tty_termios: [u8; TERMIOS_SIZE],
    allocated: bool,
    owned_by_kernel_init_task: bool,
    fd_table_bound: bool,
    stdio_bound: bool,
    next_fd_ready: bool,
    close_on_exec_ready: bool,
    shared_deferred: bool,
    stdin_ready_data_bound: bool,
    fd_lookup_routes_to_table: AtomicUsize,
    regular_file_slot_ready: bool,
    open_path_routes_to_vfs: AtomicUsize,
    read_fd_routes_to_table: AtomicUsize,
    close_fd_routes_to_table: AtomicUsize,
    stat_path_routes_to_vfs: AtomicUsize,
    readlink_path_routes_to_vfs: AtomicUsize,
    regular_fd_installed: AtomicUsize,
    regular_file_read_observed: AtomicUsize,
    stdin_char_read_observed: AtomicUsize,
    regular_file_closed: AtomicUsize,
    regular_file_stat_observed: AtomicUsize,
    directory_fd_installed: AtomicUsize,
    directory_getdents_observed: AtomicUsize,
    tty_termios_mutation_observed: AtomicUsize,
}

#[allow(dead_code)]
impl FilesStruct {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fd_table: FileDescriptorTable::new(),
            stdin: OpenFileDescription::new(),
            stdout: OpenFileDescription::new(),
            stderr: OpenFileDescription::new(),
            regular0: OpenFileDescription::new(),
            tty0: OpenFileDescription::new(),
            stdin_backend: FileBackend::new(FileBackendKind::CharDevice),
            stdout_backend: FileBackend::new(FileBackendKind::CharDevice),
            stderr_backend: FileBackend::new(FileBackendKind::CharDevice),
            regular0_backend: FileBackend::new(FileBackendKind::RegularFile),
            tty0_backend: FileBackend::new(FileBackendKind::CharDevice),
            regular0_buffer: [0; REGULAR_FILE_BUFFER_SIZE],
            regular0_len: 0,
            regular0_offset: 0,
            regular0_path: [0; FILE_PATH_MAX],
            regular0_path_len: 0,
            filesystem0_kind: FilesystemFdKind::None,
            directory0_file_ref: None,
            directory0_offset: 0,
            directory0_last_getdents_len: 0,
            tty_termios: [0; TERMIOS_SIZE],
            allocated: false,
            owned_by_kernel_init_task: false,
            fd_table_bound: false,
            stdio_bound: false,
            next_fd_ready: false,
            close_on_exec_ready: false,
            shared_deferred: false,
            stdin_ready_data_bound: false,
            fd_lookup_routes_to_table: AtomicUsize::new(0),
            regular_file_slot_ready: false,
            open_path_routes_to_vfs: AtomicUsize::new(0),
            read_fd_routes_to_table: AtomicUsize::new(0),
            close_fd_routes_to_table: AtomicUsize::new(0),
            stat_path_routes_to_vfs: AtomicUsize::new(0),
            readlink_path_routes_to_vfs: AtomicUsize::new(0),
            regular_fd_installed: AtomicUsize::new(0),
            regular_file_read_observed: AtomicUsize::new(0),
            stdin_char_read_observed: AtomicUsize::new(0),
            regular_file_closed: AtomicUsize::new(0),
            regular_file_stat_observed: AtomicUsize::new(0),
            directory_fd_installed: AtomicUsize::new(0),
            directory_getdents_observed: AtomicUsize::new(0),
            tty_termios_mutation_observed: AtomicUsize::new(0),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn owned_by_kernel_init_task(&self) -> bool {
        self.owned_by_kernel_init_task
    }

    pub const fn fd_table_bound(&self) -> bool {
        self.fd_table_bound
    }

    pub const fn stdio_bound(&self) -> bool {
        self.stdio_bound
    }

    pub const fn next_fd_ready(&self) -> bool {
        self.next_fd_ready
    }

    pub const fn close_on_exec_ready(&self) -> bool {
        self.close_on_exec_ready
    }

    pub const fn shared_deferred(&self) -> bool {
        self.shared_deferred
    }

    pub const fn stdin_ready_data_bound(&self) -> bool {
        self.stdin_ready_data_bound
    }

    pub fn fd_lookup_routes_to_table(&self) -> bool {
        self.fd_lookup_routes_to_table.load(Ordering::Acquire) != 0
    }

    pub const fn regular_file_slot_ready(&self) -> bool {
        self.regular_file_slot_ready
    }

    pub fn open_path_routes_to_vfs(&self) -> bool {
        self.open_path_routes_to_vfs.load(Ordering::Acquire) != 0
    }

    pub fn read_fd_routes_to_table(&self) -> bool {
        self.read_fd_routes_to_table.load(Ordering::Acquire) != 0
    }

    pub fn close_fd_routes_to_table(&self) -> bool {
        self.close_fd_routes_to_table.load(Ordering::Acquire) != 0
    }

    pub fn stat_path_routes_to_vfs(&self) -> bool {
        self.stat_path_routes_to_vfs.load(Ordering::Acquire) != 0
    }

    pub fn readlink_path_routes_to_vfs(&self) -> bool {
        self.readlink_path_routes_to_vfs.load(Ordering::Acquire) != 0
    }

    pub fn regular_fd_installed(&self) -> bool {
        self.regular_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_read_observed(&self) -> bool {
        self.regular_file_read_observed.load(Ordering::Acquire) != 0
    }

    pub fn stdin_char_read_observed(&self) -> bool {
        self.stdin_char_read_observed.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_closed(&self) -> bool {
        self.regular_file_closed.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_stat_observed(&self) -> bool {
        self.regular_file_stat_observed.load(Ordering::Acquire) != 0
    }

    pub fn directory_fd_installed(&self) -> bool {
        self.directory_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn directory_getdents_observed(&self) -> bool {
        self.directory_getdents_observed.load(Ordering::Acquire) != 0
    }

    pub fn tty_termios_state_bound(&self) -> bool {
        self.lifecycle.state() == State::Ready
    }

    pub fn tty_termios_mutation_observed(&self) -> bool {
        self.tty_termios_mutation_observed.load(Ordering::Acquire) != 0
    }

    pub const fn regular0_len(&self) -> usize {
        self.regular0_len
    }

    pub const fn regular0_offset(&self) -> usize {
        self.regular0_offset
    }

    pub const fn directory0_offset(&self) -> usize {
        self.directory0_offset
    }

    pub const fn directory0_last_getdents_len(&self) -> usize {
        self.directory0_last_getdents_len
    }

    pub const fn fd_table(&self) -> &FileDescriptorTable {
        &self.fd_table
    }

    pub const fn stdin(&self) -> &OpenFileDescription {
        &self.stdin
    }

    pub const fn stdout(&self) -> &OpenFileDescription {
        &self.stdout
    }

    pub const fn stderr(&self) -> &OpenFileDescription {
        &self.stderr
    }

    pub const fn regular0(&self) -> &OpenFileDescription {
        &self.regular0
    }

    pub const fn tty0(&self) -> &OpenFileDescription {
        &self.tty0
    }

    pub const fn stdin_backend(&self) -> &FileBackend {
        &self.stdin_backend
    }

    pub const fn stdout_backend(&self) -> &FileBackend {
        &self.stdout_backend
    }

    pub const fn stderr_backend(&self) -> &FileBackend {
        &self.stderr_backend
    }

    pub const fn regular0_backend(&self) -> &FileBackend {
        &self.regular0_backend
    }

    pub const fn tty0_backend(&self) -> &FileBackend {
        &self.tty0_backend
    }

    pub const fn fd_bound(&self, fd: FdRef) -> bool {
        self.fd_table.fd_bound(fd)
    }

    pub fn setup(&mut self, kernel_init_task: &KernelInitTask) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || !super::printk::is_ready()
            || !super::printk::console_handoff_complete()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.fd_table.setup()?;
        self.stdin_backend.setup()?;
        self.stdout_backend.setup()?;
        self.stderr_backend.setup()?;
        self.tty0_backend.setup()?;
        self.stdin.setup_stdio(&self.stdin_backend, true, false)?;
        self.stdout.setup_stdio(&self.stdout_backend, false, true)?;
        self.stderr.setup_stdio(&self.stderr_backend, false, true)?;
        self.tty0.setup_stdio(&self.tty0_backend, true, true)?;
        self.fd_table
            .install_stdio(&self.stdin, &self.stdout, &self.stderr)?;
        self.tty_termios = linux_std_termios();

        self.allocated = true;
        self.owned_by_kernel_init_task = true;
        self.fd_table_bound = true;
        self.stdio_bound = true;
        self.next_fd_ready = true;
        self.close_on_exec_ready = true;
        self.shared_deferred = true;
        self.regular_file_slot_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn prepare_default_stdin_ready_data(&mut self, bytes: &[u8]) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.stdio_bound
            || bytes.is_empty()
        {
            return Err(FileError::NotReady);
        }
        if !self.fd_table.fd_bound(FdRef::Stdin)
            || self.stdin.state() != State::Ready
            || self.stdin_backend.state() != State::Ready
            || !self.stdin_backend.char_device_read_supported()
        {
            return Err(FileError::BackendUnavailable);
        }
        if !crate::objects::ns16550a::seed_tty_ready_data_fixture(bytes) {
            return Err(FileError::BackendUnavailable);
        }

        self.stdin_ready_data_bound = true;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn open_regular_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
        open_flags: u32,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.regular_file_slot_ready
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
        {
            return Err(FileError::NotReady);
        }
        if self.fd_table.fd_bound(FdRef::Regular0) {
            return Err(FileError::AlreadyOpen);
        }
        if open_flags & FILE_O_ACCMODE != FILE_O_RDONLY {
            return Err(FileError::PermissionDenied);
        }

        let mut provider = virtio_blk::live_provider(kernel_image);
        self.regular0_buffer.fill(0);
        let len = vfs_core
            .read_path(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
                &mut self.regular0_buffer,
            )
            .map_err(vfs_error_to_file_error)?;

        if self.regular0_backend.state() == State::Base {
            self.regular0_backend
                .bind_regular_file()
                .map_err(|_| FileError::BackendUnavailable)?;
        }
        if self.regular0.state() == State::Base {
            self.regular0
                .setup_regular(&self.regular0_backend)
                .map_err(|_| FileError::BackendUnavailable)?;
        }

        let fd = self.fd_table.install_regular(
            &self.regular0,
            persistent_open_flags(open_flags),
            open_flags & FILE_O_CLOEXEC != 0,
        )?;
        self.regular0_len = len;
        self.regular0_offset = 0;
        self.filesystem0_kind = FilesystemFdKind::RegularFile;
        self.directory0_file_ref = None;
        self.directory0_offset = 0;
        self.directory0_last_getdents_len = 0;
        self.regular0_path.fill(0);
        self.regular0_path[..path.len()].copy_from_slice(path);
        self.regular0_path_len = path.len();
        self.open_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        self.regular_fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn open_directory_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
        open_flags: u32,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.regular_file_slot_ready
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
        {
            return Err(FileError::NotReady);
        }
        if self.fd_table.fd_bound(FdRef::Regular0) {
            return Err(FileError::AlreadyOpen);
        }
        if open_flags & FILE_O_ACCMODE != FILE_O_RDONLY {
            return Err(FileError::PermissionDenied);
        }

        let mut provider = virtio_blk::live_provider(kernel_image);
        let file_ref = vfs_core
            .open_directory_path(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
            )
            .map_err(vfs_error_to_file_error)?;

        if self.regular0_backend.state() == State::Base {
            self.regular0_backend
                .bind_regular_file()
                .map_err(|_| FileError::BackendUnavailable)?;
        }
        if self.regular0.state() == State::Base {
            self.regular0
                .setup_regular(&self.regular0_backend)
                .map_err(|_| FileError::BackendUnavailable)?;
        }

        let fd = self.fd_table.install_regular(
            &self.regular0,
            persistent_open_flags(open_flags) | FILE_O_DIRECTORY,
            open_flags & FILE_O_CLOEXEC != 0,
        )?;
        self.regular0_len = 0;
        self.regular0_offset = 0;
        self.filesystem0_kind = FilesystemFdKind::Directory;
        self.directory0_file_ref = Some(file_ref);
        self.directory0_offset = 0;
        self.directory0_last_getdents_len = 0;
        self.regular0_path.fill(0);
        self.regular0_path[..path.len()].copy_from_slice(path);
        self.regular0_path_len = path.len();
        self.open_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        self.directory_fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn open_tty_path(&mut self, path: &[u8], open_flags: u32) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.regular_file_slot_ready
            || path != DEV_TTY_PATH
        {
            return Err(FileError::PathUnavailable);
        }
        if self.fd_table.fd_bound(FdRef::Regular0) {
            return Err(FileError::AlreadyOpen);
        }
        if open_flags & FILE_O_DIRECTORY != 0 {
            return Err(FileError::InvalidArgument);
        }
        if open_flags & FILE_O_ACCMODE != FILE_O_RDWR {
            return Err(FileError::PermissionDenied);
        }

        let fd = self.fd_table.install_opened(
            &self.tty0,
            OpenFileDescriptionRef::Tty0,
            true,
            true,
            persistent_open_flags(open_flags),
            open_flags & FILE_O_CLOEXEC != 0,
        )?;
        self.filesystem0_kind = FilesystemFdKind::None;
        self.directory0_file_ref = None;
        self.directory0_offset = 0;
        self.directory0_last_getdents_len = 0;
        self.regular0_path.fill(0);
        self.regular0_path[..path.len()].copy_from_slice(path);
        self.regular0_path_len = path.len();
        self.open_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn read_fd(&mut self, fd: usize, buffer: &mut [u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        self.read_fd_routes_to_table.fetch_add(1, Ordering::AcqRel);
        if !entry.readable {
            return Err(FileError::NotReadable);
        }

        match entry.ofd {
            OpenFileDescriptionRef::Stdin => {
                let read = self.stdin.read_char_device(&self.stdin_backend, buffer)?;
                if read != 0 {
                    self.stdin_char_read_observed.fetch_add(1, Ordering::AcqRel);
                }
                Ok(read)
            }
            OpenFileDescriptionRef::Tty0 => {
                let read = self.tty0.read_char_device(&self.tty0_backend, buffer)?;
                if read != 0 {
                    self.stdin_char_read_observed.fetch_add(1, Ordering::AcqRel);
                }
                Ok(read)
            }
            OpenFileDescriptionRef::Regular0 => {
                if self.filesystem0_kind != FilesystemFdKind::RegularFile {
                    return Err(FileError::NotReadable);
                }
                let available = self.regular0_len.saturating_sub(self.regular0_offset);
                let len = core::cmp::min(buffer.len(), available);
                let end = self.regular0_offset + len;
                buffer[..len].copy_from_slice(&self.regular0_buffer[self.regular0_offset..end]);
                self.regular0_offset = end;
                let read = self.regular0.read(&self.regular0_backend, len)?;
                if read != 0 {
                    self.regular_file_read_observed
                        .fetch_add(1, Ordering::AcqRel);
                }
                Ok(read)
            }
            _ => Err(FileError::Unsupported),
        }
    }

    pub fn poll_fd(&self, fd: usize, events: u16) -> FileResult<u16> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        self.fd_lookup_routes_to_table
            .fetch_add(1, Ordering::AcqRel);
        let mut ready = 0u16;
        if entry.readable {
            match entry.ofd {
                OpenFileDescriptionRef::Regular0 => match self.filesystem0_kind {
                    FilesystemFdKind::RegularFile | FilesystemFdKind::Directory => {
                        ready |= FILE_POLLIN | FILE_POLLRDNORM;
                    }
                    FilesystemFdKind::None => return Err(FileError::BadFd),
                },
                OpenFileDescriptionRef::Stdin | OpenFileDescriptionRef::Tty0 => {
                    let backend = self.char_backend_for_entry(entry)?;
                    if backend.char_device_read_ready()? {
                        ready |= FILE_POLLIN | FILE_POLLRDNORM;
                    }
                }
                OpenFileDescriptionRef::Stdout | OpenFileDescriptionRef::Stderr => {}
            }
        }
        if entry.writable {
            match entry.ofd {
                OpenFileDescriptionRef::Stdout
                | OpenFileDescriptionRef::Stderr
                | OpenFileDescriptionRef::Tty0 => {
                    let backend = self.char_backend_for_entry(entry)?;
                    if backend.char_device_write_ready()? {
                        ready |= FILE_POLLOUT | FILE_POLLWRNORM;
                    }
                }
                OpenFileDescriptionRef::Stdin | OpenFileDescriptionRef::Regular0 => {}
            }
        }

        Ok(ready & (events | FILE_POLLERR | FILE_POLLHUP))
    }

    pub fn getdents64_fd(
        &mut self,
        fd: usize,
        buffer: &mut [u8],
        ext2_filesystem: &mut Ext2FileSystem,
        vfs_core: &mut VfsCore,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        self.read_fd_routes_to_table.fetch_add(1, Ordering::AcqRel);
        if !entry.readable || entry.ofd != OpenFileDescriptionRef::Regular0 {
            return Err(FileError::NotReadable);
        }
        if self.filesystem0_kind != FilesystemFdKind::Directory {
            return Err(FileError::NotReadable);
        }
        let file_ref = self.directory0_file_ref.ok_or(FileError::BadFd)?;
        let mut provider = virtio_blk::live_provider(kernel_image);
        let entries = vfs_core
            .read_ext2_dir(
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                file_ref,
                self.directory0_offset,
            )
            .map_err(vfs_error_to_file_error)?;
        let (written, next_offset) =
            serialize_linux_dirents64(entries.iter(), self.directory0_offset, buffer)?;
        if written == 0 {
            return Ok(0);
        }
        self.directory0_offset = next_offset;
        self.directory0_last_getdents_len = written;
        self.directory_getdents_observed
            .fetch_add(1, Ordering::AcqRel);
        Ok(written)
    }

    pub fn close_fd(&mut self, fd: usize) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.close(fd)?;
        if fd == REGULAR0_FD && entry.ofd == OpenFileDescriptionRef::Regular0 {
            self.regular0_offset = 0;
            self.directory0_offset = 0;
            self.directory0_file_ref = None;
            self.filesystem0_kind = FilesystemFdKind::None;
        }
        self.close_fd_routes_to_table.fetch_add(1, Ordering::AcqRel);
        if entry.ofd == OpenFileDescriptionRef::Regular0 {
            self.regular_file_closed.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }

    pub fn fcntl_getfl_fd(&self, fd: usize) -> FileResult<u32> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        Ok(entry.flags)
    }

    pub fn fcntl_getfd_fd(&self, fd: usize) -> FileResult<u32> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.get_fd_flags(fd)
    }

    pub fn fcntl_setfd_fd(&mut self, fd: usize, flags: u32) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.set_fd_flags(fd, flags)
    }

    pub fn fcntl_dupfd_fd(
        &mut self,
        fd: usize,
        min_fd: usize,
        close_on_exec: bool,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.dup_fd(fd, min_fd, close_on_exec)
    }

    pub fn ioctl_validate_fd(&self, fd: usize) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.lookup(fd)?;
        Ok(())
    }

    fn char_backend_for_fd(&self, fd: usize) -> FileResult<&FileBackend> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        self.char_backend_for_entry(entry)
    }

    fn char_backend_for_entry(&self, entry: FileDescriptorEntry) -> FileResult<&FileBackend> {
        let backend = match entry.ofd {
            OpenFileDescriptionRef::Stdin => &self.stdin_backend,
            OpenFileDescriptionRef::Stdout => &self.stdout_backend,
            OpenFileDescriptionRef::Stderr => &self.stderr_backend,
            OpenFileDescriptionRef::Regular0 => &self.regular0_backend,
            OpenFileDescriptionRef::Tty0 => &self.tty0_backend,
        };
        if backend.state() != State::Ready || backend.kind() != FileBackendKind::CharDevice {
            return Err(FileError::NotTty);
        }

        Ok(backend)
    }

    pub fn ioctl_tiocgwinsz_fd(&self, fd: usize) -> FileResult<(u16, u16, u16, u16)> {
        self.char_backend_for_fd(fd)?;

        Ok((
            TTY_WINSIZE_ROW,
            TTY_WINSIZE_COL,
            TTY_WINSIZE_XPIXEL,
            TTY_WINSIZE_YPIXEL,
        ))
    }

    pub fn ioctl_tcgets_fd(&self, fd: usize) -> FileResult<[u8; TERMIOS_SIZE]> {
        self.char_backend_for_fd(fd)?;

        Ok(self.tty_termios)
    }

    pub fn ioctl_tcsets_fd(&mut self, fd: usize, termios: [u8; TERMIOS_SIZE]) -> FileResult<()> {
        self.char_backend_for_fd(fd)?;

        self.tty_termios = termios;
        self.tty_termios_mutation_observed
            .fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    pub fn ioctl_tiocgpgrp_fd(&self, fd: usize) -> FileResult<()> {
        self.char_backend_for_fd(fd)?;
        Ok(())
    }

    pub fn ioctl_tiocspgrp_fd(&self, fd: usize) -> FileResult<()> {
        self.char_backend_for_fd(fd)?;
        Ok(())
    }

    pub fn lseek_fd(
        &mut self,
        fd: usize,
        offset: isize,
        whence: usize,
        vfs_core: &VfsCore,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        if entry.ofd != OpenFileDescriptionRef::Regular0 {
            return Err(FileError::IllegalSeek);
        }

        let (current, end) = match self.filesystem0_kind {
            FilesystemFdKind::RegularFile => (self.regular0_offset, self.regular0_len),
            FilesystemFdKind::Directory => {
                let file_ref = self.directory0_file_ref.ok_or(FileError::BadFd)?;
                let stat = vfs_core
                    .file_stat(file_ref)
                    .map_err(vfs_error_to_file_error)?;
                (self.directory0_offset, stat.size())
            }
            FilesystemFdKind::None => return Err(FileError::BadFd),
        };
        let base = match whence {
            SEEK_SET => 0i128,
            SEEK_CUR => current as i128,
            SEEK_END => end as i128,
            _ => return Err(FileError::InvalidArgument),
        };
        let target = base + offset as i128;
        if target < 0 || target > usize::MAX as i128 {
            return Err(FileError::InvalidArgument);
        }

        let target = target as usize;
        match self.filesystem0_kind {
            FilesystemFdKind::RegularFile => self.regular0_offset = target,
            FilesystemFdKind::Directory => self.directory0_offset = target,
            FilesystemFdKind::None => return Err(FileError::BadFd),
        }
        Ok(target)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stat_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
        nofollow_final_symlink: bool,
    ) -> FileResult<FileStat> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
        {
            return Err(FileError::NotReady);
        }

        let mut provider = virtio_blk::live_provider(kernel_image);
        let vfs_stat = vfs_core
            .stat_path(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
                nofollow_final_symlink,
            )
            .map_err(vfs_error_to_file_error)?;

        if self.regular0_backend.state() == State::Base {
            self.regular0_backend
                .bind_regular_file()
                .map_err(|_| FileError::BackendUnavailable)?;
        }
        let stat = self
            .regular0_backend
            .stat_regular_file(FileStat::new(vfs_stat.size(), vfs_stat.kind()))?;
        self.stat_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        if stat.mode() & 0o170000 == 0o100000 {
            self.regular_file_stat_observed
                .fetch_add(1, Ordering::AcqRel);
        }
        Ok(stat)
    }

    pub fn fstat_fd(&mut self, fd: usize, vfs_core: &VfsCore) -> FileResult<FileStat> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        self.stat_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        match entry.ofd {
            OpenFileDescriptionRef::Regular0 => {
                let stat = match self.filesystem0_kind {
                    FilesystemFdKind::RegularFile => FileStat::regular(self.regular0_len),
                    FilesystemFdKind::Directory => {
                        let file_ref = self.directory0_file_ref.ok_or(FileError::BadFd)?;
                        let vfs_stat = vfs_core
                            .file_stat(file_ref)
                            .map_err(vfs_error_to_file_error)?;
                        FileStat::new(vfs_stat.size(), vfs_stat.kind())
                    }
                    FilesystemFdKind::None => return Err(FileError::BadFd),
                };
                if self.regular0_backend.state() == State::Base {
                    self.regular0_backend
                        .bind_regular_file()
                        .map_err(|_| FileError::BackendUnavailable)?;
                }
                let stat = self.regular0_backend.stat_regular_file(stat)?;
                if stat.mode() & 0o170000 == 0o100000 {
                    self.regular_file_stat_observed
                        .fetch_add(1, Ordering::AcqRel);
                }
                Ok(stat)
            }
            OpenFileDescriptionRef::Stdin
            | OpenFileDescriptionRef::Stdout
            | OpenFileDescriptionRef::Stderr
            | OpenFileDescriptionRef::Tty0 => Ok(FileStat::new(0, VfsInodeKind::DeviceNode)),
        }
    }

    pub fn write_fd(&self, fd: usize, bytes: &[u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        self.fd_lookup_routes_to_table
            .fetch_add(1, Ordering::AcqRel);
        if !entry.writable {
            return Err(FileError::NotWritable);
        }

        match entry.ofd {
            OpenFileDescriptionRef::Stdin => self.stdin.write(&self.stdin_backend, bytes),
            OpenFileDescriptionRef::Stdout => self.stdout.write(&self.stdout_backend, bytes),
            OpenFileDescriptionRef::Stderr => self.stderr.write(&self.stderr_backend, bytes),
            OpenFileDescriptionRef::Regular0 => Err(FileError::NotWritable),
            OpenFileDescriptionRef::Tty0 => self.tty0.write(&self.tty0_backend, bytes),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn access_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
        mode: usize,
    ) -> FileResult<()> {
        let stat = self.stat_path(
            fs_struct,
            vfs_core,
            ext2_filesystem,
            block_device_registry,
            kernel_image,
            path,
            false,
        )?;
        if file_mode_allows_access(stat.mode(), mode) {
            Ok(())
        } else {
            Err(FileError::PermissionDenied)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn readlink_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
        buffer: &mut [u8],
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
            || buffer.is_empty()
        {
            return Err(FileError::InvalidArgument);
        }

        let mut provider = virtio_blk::live_provider(kernel_image);
        let len = vfs_core
            .readlink_path(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
                buffer,
            )
            .map_err(vfs_readlink_error_to_file_error)?;
        self.readlink_path_routes_to_vfs
            .fetch_add(1, Ordering::AcqRel);
        Ok(len)
    }
}

fn file_mode_allows_access(file_mode: u32, access_mode: usize) -> bool {
    let permission_bits = (file_mode & 0o777) as usize;
    access_mode & !permission_bits == 0
}

fn linux_std_termios() -> [u8; TERMIOS_SIZE] {
    let mut termios = [0u8; TERMIOS_SIZE];
    write_u32_raw(&mut termios, 0, TERMIOS_ICRNL | TERMIOS_IXON);
    write_u32_raw(&mut termios, 4, TERMIOS_OPOST | TERMIOS_ONLCR);
    write_u32_raw(
        &mut termios,
        8,
        TERMIOS_B38400 | TERMIOS_CS8 | TERMIOS_CREAD | TERMIOS_HUPCL,
    );
    write_u32_raw(
        &mut termios,
        12,
        TERMIOS_ISIG
            | TERMIOS_ICANON
            | TERMIOS_ECHO
            | TERMIOS_ECHOE
            | TERMIOS_ECHOK
            | TERMIOS_ECHOCTL
            | TERMIOS_ECHOKE
            | TERMIOS_IEXTEN,
    );
    termios[16] = 0;
    termios[17] = b'C' - 0x40;
    termios[18] = b'\\' - 0x40;
    termios[19] = 0x7f;
    termios[20] = b'U' - 0x40;
    termios[21] = b'D' - 0x40;
    termios[23] = 1;
    termios[25] = b'Q' - 0x40;
    termios[26] = b'S' - 0x40;
    termios[27] = b'Z' - 0x40;
    termios[29] = b'R' - 0x40;
    termios[30] = b'O' - 0x40;
    termios[31] = b'W' - 0x40;
    termios[32] = b'V' - 0x40;
    termios
}

fn write_u32_raw(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + core::mem::size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
}

fn serialize_linux_dirents64<'a>(
    entries: impl Iterator<Item = &'a super::ext2::Ext2DirectoryEntry>,
    current_offset: usize,
    buffer: &mut [u8],
) -> FileResult<(usize, usize)> {
    let mut written = 0usize;
    let mut next_offset = current_offset;
    for entry in entries {
        let record = entry.record();
        let name = record.name();
        let reclen = linux_dirent64_reclen(name.len());
        if reclen > buffer.len().saturating_sub(written) {
            break;
        }
        let dst = &mut buffer[written..written + reclen];
        dst.fill(0);
        write_u64(dst, 0, u64::from(record.inode()))?;
        write_u64(dst, 8, entry.next_offset() as u64)?;
        write_u16(dst, 16, reclen as u16)?;
        dst[18] = linux_dtype(record.file_type());
        dst[LINUX_DIRENT64_HEADER_SIZE..LINUX_DIRENT64_HEADER_SIZE + name.len()]
            .copy_from_slice(name);
        written += reclen;
        next_offset = entry.next_offset();
    }
    Ok((written, next_offset))
}

const fn persistent_open_flags(flags: u32) -> u32 {
    flags & (FILE_O_ACCMODE | FILE_O_LARGEFILE | FILE_O_DIRECTORY) & !FILE_O_CLOEXEC
}

const fn linux_dirent64_reclen(name_len: usize) -> usize {
    align_up(
        LINUX_DIRENT64_HEADER_SIZE + name_len + 1,
        core::mem::size_of::<u64>(),
    )
}

const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

const fn linux_dtype(file_type: Ext2FileType) -> u8 {
    match file_type {
        Ext2FileType::RegularFile => DT_REG,
        Ext2FileType::Directory => DT_DIR,
        Ext2FileType::Symlink => DT_LNK,
        Ext2FileType::Unknown | Ext2FileType::Other => DT_UNKNOWN,
    }
}

fn write_u16(buffer: &mut [u8], offset: usize, value: u16) -> FileResult<()> {
    let bytes = value.to_le_bytes();
    let Some(dst) = buffer.get_mut(offset..offset + bytes.len()) else {
        return Err(FileError::BufferTooSmall);
    };
    dst.copy_from_slice(&bytes);
    Ok(())
}

fn write_u64(buffer: &mut [u8], offset: usize, value: u64) -> FileResult<()> {
    let bytes = value.to_le_bytes();
    let Some(dst) = buffer.get_mut(offset..offset + bytes.len()) else {
        return Err(FileError::BufferTooSmall);
    };
    dst.copy_from_slice(&bytes);
    Ok(())
}
