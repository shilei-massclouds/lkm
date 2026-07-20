use core::sync::atomic::{AtomicUsize, Ordering};

use super::{
    block_device::BlockDeviceRegistry,
    ext2::{Ext2FileSystem, Ext2FileType},
    kernel_image::KernelImage,
    rest_init::KernelInitTask,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    vfs::{FileRef, FsStruct, VfsCore, VfsError, VfsInodeKind},
    virtio_blk,
};

pub const STDIN_FD: usize = 0;
pub const STDOUT_FD: usize = 1;
pub const STDERR_FD: usize = 2;
pub const REGULAR0_FD: usize = 3;
pub const FILE_PATH_MAX: usize = 128;
pub const REGULAR_FILE_BUFFER_SIZE: usize = 64 * 1024;
pub const PIPE_BUFFER_SIZE: usize = 4096;
pub const LINUX_DIRENT64_HEADER_SIZE: usize = 19;
pub const TERMIOS_SIZE: usize = 36;
// Consumed by the user-boot stdin fixture configuration.
#[allow(dead_code)]
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
const DEV_NULL_PATH: &[u8] = b"/dev/null";
const FILE_O_RDONLY: u32 = 0;
const FILE_O_WRONLY: u32 = 1;
const FILE_O_RDWR: u32 = 2;
const FILE_O_ACCMODE: u32 = 0o3;
const FILE_O_NONBLOCK: u32 = 0o4000;
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
    UnixSocket,
    Pipe,
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
    NotDirectory,
    BrokenPipe,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FilesystemFdKind {
    None,
    RegularFile,
    Directory,
}

pub type FileResult<T> = Result<T, FileError>;

struct PipeBuffer {
    bytes: [u8; PIPE_BUFFER_SIZE],
    read_offset: usize,
    len: usize,
}

impl PipeBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; PIPE_BUFFER_SIZE],
            read_offset: 0,
            len: 0,
        }
    }

    fn reset(&mut self) {
        self.read_offset = 0;
        self.len = 0;
    }

    const fn len(&self) -> usize {
        self.len
    }

    fn write(&mut self, input: &[u8]) -> FileResult<usize> {
        if input.is_empty() {
            return Ok(0);
        }
        if self.read_offset != 0 && self.read_offset + self.len == PIPE_BUFFER_SIZE {
            self.bytes
                .copy_within(self.read_offset..self.read_offset + self.len, 0);
            self.read_offset = 0;
        }
        let write_offset = self.read_offset + self.len;
        let available = PIPE_BUFFER_SIZE.saturating_sub(write_offset);
        if available == 0 {
            return Err(FileError::NotReady);
        }
        let written = core::cmp::min(input.len(), available);
        self.bytes[write_offset..write_offset + written].copy_from_slice(&input[..written]);
        self.len += written;
        Ok(written)
    }

    fn read(&mut self, output: &mut [u8]) -> usize {
        let read = core::cmp::min(output.len(), self.len);
        output[..read].copy_from_slice(&self.bytes[self.read_offset..self.read_offset + read]);
        self.read_offset += read;
        self.len -= read;
        if self.len == 0 {
            self.read_offset = 0;
        }
        read
    }
}

pub fn is_tty_path(path: &[u8]) -> bool {
    if path == DEV_TTY_PATH {
        return true;
    }
    if path.len() <= DEV_TTY_PATH.len() || !path.starts_with(DEV_TTY_PATH) {
        return false;
    }

    path[DEV_TTY_PATH.len()..].iter().all(u8::is_ascii_digit)
}

pub fn is_null_path(path: &[u8]) -> bool {
    path == DEV_NULL_PATH
}

fn vfs_error_to_file_error(error: VfsError) -> FileError {
    match error {
        VfsError::NotFound => FileError::PathUnavailable,
        VfsError::ShortBuffer => FileError::BufferTooSmall,
        VfsError::Backend => FileError::VfsBackendUnavailable,
        VfsError::SymlinkLoop => FileError::TooManySymlinks,
        VfsError::NotDirectory => FileError::NotDirectory,
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
    pid: usize,
    owner_valid: bool,
    owner_uid: usize,
    owner_gid: usize,
    mode_override_valid: bool,
    mode_override: u32,
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
            pid: 0,
            owner_valid: false,
            owner_uid: 0,
            owner_gid: 0,
            mode_override_valid: false,
            mode_override: 0,
        }
    }

    const fn regular(ofd: OpenFileDescriptionRef, flags: u32, close_on_exec: bool) -> Self {
        Self {
            ofd,
            readable: true,
            writable: false,
            flags,
            close_on_exec,
            pid: 0,
            owner_valid: false,
            owner_uid: 0,
            owner_gid: 0,
            mode_override_valid: false,
            mode_override: 0,
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
            pid: 0,
            owner_valid: false,
            owner_uid: 0,
            owner_gid: 0,
            mode_override_valid: false,
            mode_override: 0,
        }
    }

    const fn pidfd(child_pid: usize) -> Self {
        Self {
            ofd: OpenFileDescriptionRef::Pidfd0,
            readable: true,
            writable: false,
            flags: FILE_O_RDWR,
            close_on_exec: true,
            pid: child_pid,
            owner_valid: false,
            owner_uid: 0,
            owner_gid: 0,
            mode_override_valid: false,
            mode_override: 0,
        }
    }

    const fn pipe_end(ofd: OpenFileDescriptionRef, readable: bool, writable: bool) -> Self {
        Self {
            ofd,
            readable,
            writable,
            flags: if writable {
                FILE_O_WRONLY
            } else {
                FILE_O_RDONLY
            },
            close_on_exec: false,
            pid: 0,
            owner_valid: false,
            owner_uid: 0,
            owner_gid: 0,
            mode_override_valid: false,
            mode_override: 0,
        }
    }

    fn record_owner(&mut self, uid: usize, gid: usize) {
        self.owner_valid = true;
        self.owner_uid = uid;
        self.owner_gid = gid;
    }

    fn record_mode_override(&mut self, mode: u32) {
        self.mode_override_valid = true;
        self.mode_override = mode;
    }

    fn apply_mode_override(&self, stat: FileStat) -> FileStat {
        if self.mode_override_valid {
            stat.with_permission_mode(self.mode_override)
        } else {
            stat
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OpenFileDescriptionRef {
    Stdin,
    Stdout,
    Stderr,
    Regular0,
    Null,
    Tty0,
    Pidfd0,
    UnixSocket0,
    PipeRead0,
    PipeWrite0,
}

// The complete fd entry view is consumed by smoke and optional syscall diagnostics.
#[cfg_attr(not(app_smoke), allow(dead_code))]
#[derive(Clone, Copy)]
pub struct FdEntryDiagnostic {
    pub ofd: OpenFileDescriptionRef,
    pub readable: bool,
    pub writable: bool,
    // Read by the optional detailed user-syscall diagnostic handler.
    #[allow(dead_code)]
    pub flags: u32,
    pub close_on_exec: bool,
    #[allow(dead_code)]
    pub pid: usize,
    pub owner_valid: bool,
    pub owner_uid: usize,
    pub owner_gid: usize,
    pub mode_override_valid: bool,
    pub mode_override: u32,
}

#[derive(Clone, Copy)]
pub struct CloseOnExecReport {
    pub scanned: usize,
    pub closed: usize,
    pub first_closed_fd: usize,
    pub remaining_open: usize,
}

impl CloseOnExecReport {
    const fn empty() -> Self {
        Self {
            scanned: 0,
            closed: 0,
            first_closed_fd: usize::MAX,
            remaining_open: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct FilesStructSnapshot {
    entries: [Option<FileDescriptorEntry>; FILE_FD_COUNT],
    regular0_len: usize,
    regular0_offset: usize,
    regular0_path: [u8; FILE_PATH_MAX],
    regular0_path_len: usize,
    filesystem0_kind: FilesystemFdKind,
    directory0_file_ref: Option<FileRef>,
    directory0_offset: usize,
    directory0_last_getdents_len: usize,
    pidfd_fd: usize,
    pidfd_child_pid: usize,
    pidfd_exit_status: usize,
    socket0_fd: usize,
    pipe_read_end_open: bool,
    pipe_write_end_open: bool,
}

impl FilesStructSnapshot {
    pub const fn empty() -> Self {
        Self {
            entries: [None; FILE_FD_COUNT],
            regular0_len: 0,
            regular0_offset: 0,
            regular0_path: [0; FILE_PATH_MAX],
            regular0_path_len: 0,
            filesystem0_kind: FilesystemFdKind::None,
            directory0_file_ref: None,
            directory0_offset: 0,
            directory0_last_getdents_len: 0,
            pidfd_fd: usize::MAX,
            pidfd_child_pid: 0,
            pidfd_exit_status: 0,
            socket0_fd: usize::MAX,
            pipe_read_end_open: false,
            pipe_write_end_open: false,
        }
    }
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

    const fn socket(size: usize) -> Self {
        Self {
            size,
            mode: 0o140777,
        }
    }

    const fn fifo(size: usize) -> Self {
        Self {
            size,
            mode: 0o010600,
        }
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn mode(&self) -> u32 {
        self.mode
    }

    const fn with_permission_mode(self, mode: u32) -> Self {
        Self {
            size: self.size,
            mode: (self.mode & 0o170000) | (mode & 0o7777),
        }
    }
}

pub struct FileBackend {
    lifecycle: Lifecycle,
    kind: FileBackendKind,
    allocated: bool,
    char_device_console_bound: bool,
    char_device_null_bound: bool,
    char_device_write_supported: bool,
    char_device_read_supported: bool,
    regular_file_bound: bool,
    regular_file_read_supported: bool,
    regular_file_stat_supported: bool,
    unix_socket_bound: bool,
    unix_socket_unconnected: bool,
    regular_file_deferred: bool,
    block_device_deferred: bool,
    write_to_console: AtomicUsize,
    last_write_len: AtomicUsize,
    char_device_read_returns_data: AtomicUsize,
    regular_file_read_returns_data: AtomicUsize,
    regular_file_stat_returns_metadata: AtomicUsize,
    null_device_read_returns_eof: AtomicUsize,
    null_device_write_discards_data: AtomicUsize,
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
            char_device_null_bound: false,
            char_device_write_supported: false,
            char_device_read_supported: false,
            regular_file_bound: false,
            regular_file_read_supported: false,
            regular_file_stat_supported: false,
            unix_socket_bound: false,
            unix_socket_unconnected: false,
            regular_file_deferred: false,
            block_device_deferred: false,
            write_to_console: AtomicUsize::new(0),
            last_write_len: AtomicUsize::new(0),
            char_device_read_returns_data: AtomicUsize::new(0),
            regular_file_read_returns_data: AtomicUsize::new(0),
            regular_file_stat_returns_metadata: AtomicUsize::new(0),
            null_device_read_returns_eof: AtomicUsize::new(0),
            null_device_write_discards_data: AtomicUsize::new(0),
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

    pub const fn char_device_null_bound(&self) -> bool {
        self.char_device_null_bound
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

    pub const fn unix_socket_bound(&self) -> bool {
        self.unix_socket_bound
    }

    pub const fn unix_socket_unconnected(&self) -> bool {
        self.unix_socket_unconnected
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

    pub fn null_device_read_returns_eof(&self) -> bool {
        self.null_device_read_returns_eof.load(Ordering::Acquire) != 0
    }

    pub fn null_device_write_discards_data(&self) -> bool {
        self.null_device_write_discards_data.load(Ordering::Acquire) != 0
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

    fn setup_null_device(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || self.kind != FileBackendKind::CharDevice {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.char_device_null_bound = true;
        self.char_device_write_supported = true;
        self.char_device_read_supported = true;
        self.regular_file_deferred = true;
        self.block_device_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn bind_unix_stream_socket(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || self.kind != FileBackendKind::UnixSocket {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.allocated = true;
        self.unix_socket_bound = true;
        self.unix_socket_unconnected = true;
        self.regular_file_deferred = true;
        self.block_device_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn write_char_device(&self, bytes: &[u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_console_bound
            || !self.char_device_write_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        crate::objects::printk::write_bytes(bytes);
        self.last_write_len.store(bytes.len(), Ordering::Release);
        self.write_to_console.fetch_add(1, Ordering::AcqRel);
        Ok(bytes.len())
    }

    fn write_null_device(&self, bytes: &[u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_null_bound
            || !self.char_device_write_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        self.last_write_len.store(bytes.len(), Ordering::Release);
        self.null_device_write_discards_data
            .fetch_add(1, Ordering::AcqRel);
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

    fn read_char_device(&self, buffer: &mut [u8], canonical: bool) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_console_bound
            || !self.char_device_read_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        let len = crate::objects::ns16550a::read_tty_ready_data_with_mode(buffer, canonical)
            .ok_or(FileError::BackendUnavailable)?;
        if len == 0 && !buffer.is_empty() {
            return Err(FileError::NotReady);
        }
        self.last_read_len.store(len, Ordering::Release);
        if len != 0 {
            self.char_device_read_returns_data
                .fetch_add(1, Ordering::AcqRel);
        }
        Ok(len)
    }

    fn read_null_device(&self) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_null_bound
            || !self.char_device_read_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        self.last_read_len.store(0, Ordering::Release);
        self.null_device_read_returns_eof
            .fetch_add(1, Ordering::AcqRel);
        Ok(0)
    }

    fn char_device_read_ready(&self, canonical: bool) -> FileResult<bool> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_console_bound
            || !self.char_device_read_supported
        {
            return Err(FileError::BackendUnavailable);
        }

        crate::objects::ns16550a::tty_ready_data_available_with_mode(canonical)
            .ok_or(FileError::BackendUnavailable)
    }

    fn char_device_write_ready(&self) -> FileResult<bool> {
        if self.lifecycle.state() != State::Ready
            || self.kind != FileBackendKind::CharDevice
            || !self.char_device_console_bound
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

    fn setup_null_device(&mut self, backend: &FileBackend) -> EventResult {
        if self.lifecycle.state() != State::Base
            || backend.state() != State::Ready
            || backend.kind() != FileBackendKind::CharDevice
            || !backend.char_device_null_bound()
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
        self.writable = true;
        self.offset_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn setup_unix_socket(&mut self, backend: &FileBackend) -> EventResult {
        if self.lifecycle.state() != State::Base
            || backend.state() != State::Ready
            || backend.kind() != FileBackendKind::UnixSocket
            || !backend.unix_socket_bound()
            || !backend.unix_socket_unconnected()
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
        self.writable = true;
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

    fn write_null_device(&self, backend: &FileBackend, bytes: &[u8]) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.backend_bound {
            return Err(FileError::NotReady);
        }
        if !self.writable {
            return Err(FileError::NotWritable);
        }

        let written = backend.write_null_device(bytes)?;
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

    fn read_char_device(
        &self,
        backend: &FileBackend,
        buffer: &mut [u8],
        canonical: bool,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.backend_bound {
            return Err(FileError::NotReady);
        }
        if !self.readable {
            return Err(FileError::NotReadable);
        }

        let read = backend.read_char_device(buffer, canonical)?;
        self.last_read_len.store(read, Ordering::Release);
        self.read_dispatches_backend.fetch_add(1, Ordering::AcqRel);
        self.read_observed.fetch_add(1, Ordering::AcqRel);
        Ok(read)
    }

    fn read_null_device(&self, backend: &FileBackend) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.backend_bound {
            return Err(FileError::NotReady);
        }
        if !self.readable {
            return Err(FileError::NotReadable);
        }

        let read = backend.read_null_device()?;
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
    fd_duplicated: AtomicUsize,
    dup3_close_on_exec_bound: AtomicUsize,
    parent_snapshot_saved: AtomicUsize,
    parent_snapshot_restored: AtomicUsize,
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
            fd_duplicated: AtomicUsize::new(0),
            dup3_close_on_exec_bound: AtomicUsize::new(0),
            parent_snapshot_saved: AtomicUsize::new(0),
            parent_snapshot_restored: AtomicUsize::new(0),
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

    pub const fn capacity(&self) -> usize {
        FILE_FD_COUNT
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

    pub fn fd_duplicated(&self) -> bool {
        self.fd_duplicated.load(Ordering::Acquire) != 0
    }

    pub fn dup3_close_on_exec_bound(&self) -> bool {
        self.dup3_close_on_exec_bound.load(Ordering::Acquire) != 0
    }

    pub fn parent_snapshot_saved(&self) -> bool {
        self.parent_snapshot_saved.load(Ordering::Acquire) != 0
    }

    pub fn parent_snapshot_restored(&self) -> bool {
        self.parent_snapshot_restored.load(Ordering::Acquire) != 0
    }

    pub fn open_count(&self) -> usize {
        let mut count = 0usize;
        let mut fd = 0usize;
        while fd < FILE_FD_COUNT {
            if self.entries[fd].is_some() {
                count += 1;
            }
            fd += 1;
        }
        count
    }

    pub fn entry_diagnostic(&self, fd: usize) -> Option<FdEntryDiagnostic> {
        if fd >= FILE_FD_COUNT {
            return None;
        }
        self.entries[fd].map(|entry| FdEntryDiagnostic {
            ofd: entry.ofd,
            readable: entry.readable,
            writable: entry.writable,
            flags: entry.flags,
            close_on_exec: entry.close_on_exec,
            pid: entry.pid,
            owner_valid: entry.owner_valid,
            owner_uid: entry.owner_uid,
            owner_gid: entry.owner_gid,
            mode_override_valid: entry.mode_override_valid,
            mode_override: entry.mode_override,
        })
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

    fn lowest_free_fd_from(&self, min_fd: usize) -> Option<usize> {
        let mut candidate = min_fd;
        while candidate < FILE_FD_COUNT {
            if self.entries[candidate].is_none() {
                return Some(candidate);
            }
            candidate += 1;
        }
        None
    }

    fn install_new_fd_entry(&mut self, fd: usize, entry: FileDescriptorEntry) -> usize {
        self.entries[fd] = Some(entry);
        self.fd_installed.fetch_add(1, Ordering::AcqRel);
        fd
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

        Ok(self.install_new_fd_entry(
            REGULAR0_FD,
            FileDescriptorEntry::regular(OpenFileDescriptionRef::Regular0, flags, close_on_exec),
        ))
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

        let fd = self
            .lowest_free_fd_from(0)
            .ok_or(FileError::TooManyOpenFiles)?;
        Ok(self.install_new_fd_entry(
            fd,
            FileDescriptorEntry::opened(ofd_ref, readable, writable, flags, close_on_exec),
        ))
    }

    fn dup_fd(&mut self, fd: usize, min_fd: usize, close_on_exec: bool) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }
        if min_fd >= FILE_FD_COUNT {
            return Err(FileError::InvalidArgument);
        }

        let source = self.lookup(fd)?;
        let newfd = self
            .lowest_free_fd_from(min_fd)
            .ok_or(FileError::TooManyOpenFiles)?;
        let mut duplicate = source;
        duplicate.close_on_exec = close_on_exec;
        Ok(self.install_new_fd_entry(newfd, duplicate))
    }

    fn dup3_fd(
        &mut self,
        oldfd: usize,
        newfd: usize,
        close_on_exec: bool,
    ) -> FileResult<(usize, Option<FileDescriptorEntry>)> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }
        if oldfd == newfd {
            return Err(FileError::InvalidArgument);
        }
        if newfd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }

        let source = self.lookup(oldfd)?;
        let replaced = self.entries[newfd];
        let mut duplicate = source;
        duplicate.close_on_exec = close_on_exec;
        self.entries[newfd] = Some(duplicate);
        self.fd_duplicated.fetch_add(1, Ordering::AcqRel);
        self.dup3_close_on_exec_bound.fetch_add(1, Ordering::AcqRel);
        Ok((newfd, replaced))
    }

    fn install_pidfd(&mut self, child_pid: usize) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || child_pid == 0 {
            return Err(FileError::NotReady);
        }

        let fd = self
            .lowest_free_fd_from(REGULAR0_FD)
            .ok_or(FileError::TooManyOpenFiles)?;
        Ok(self.install_new_fd_entry(fd, FileDescriptorEntry::pidfd(child_pid)))
    }

    fn install_pipe_pair(&mut self) -> FileResult<[usize; 2]> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        let mut pair = [usize::MAX; 2];
        let mut candidate = 0usize;
        let mut found = 0usize;
        while candidate < FILE_FD_COUNT && found < pair.len() {
            if self.entries[candidate].is_none() {
                pair[found] = candidate;
                found += 1;
            }
            candidate += 1;
        }
        if found != pair.len() {
            return Err(FileError::TooManyOpenFiles);
        }

        self.entries[pair[0]] = Some(FileDescriptorEntry::pipe_end(
            OpenFileDescriptionRef::PipeRead0,
            true,
            false,
        ));
        self.entries[pair[1]] = Some(FileDescriptorEntry::pipe_end(
            OpenFileDescriptionRef::PipeWrite0,
            false,
            true,
        ));
        self.fd_installed.fetch_add(2, Ordering::AcqRel);
        Ok(pair)
    }

    fn first_fd_for_ofd(&self, ofd: OpenFileDescriptionRef) -> Option<usize> {
        let mut fd = 0usize;
        while fd < FILE_FD_COUNT {
            if let Some(entry) = self.entries[fd]
                && entry.ofd == ofd
            {
                return Some(fd);
            }
            fd += 1;
        }
        None
    }

    fn pipe_end_open(&self, ofd: OpenFileDescriptionRef) -> bool {
        self.first_fd_for_ofd(ofd).is_some()
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

    fn set_status_flags(&mut self, fd: usize, flags: u32) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        let entry = self.entries[fd].as_mut().ok_or(FileError::BadFd)?;
        if entry.ofd != OpenFileDescriptionRef::Tty0 {
            return Err(FileError::InvalidArgument);
        }

        entry.flags = (entry.flags & !FILE_O_NONBLOCK) | (flags & FILE_O_NONBLOCK);
        self.lookup_returns.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn update_owner(&mut self, fd: usize, uid: usize, gid: usize) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        let entry = self.entries[fd].as_mut().ok_or(FileError::BadFd)?;
        entry.record_owner(uid, gid);
        self.lookup_returns.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn update_mode(&mut self, fd: usize, mode: u32) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        let entry = self.entries[fd].as_mut().ok_or(FileError::BadFd)?;
        entry.record_mode_override(mode & 0o7777);
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
        let entry = self.entries[fd].ok_or(FileError::BadFd)?;

        self.entries[fd] = None;
        self.fd_closed.fetch_add(1, Ordering::AcqRel);
        Ok(entry)
    }

    fn close_on_exec_set(&self, fd: usize) -> FileResult<bool> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }
        if fd >= FILE_FD_COUNT {
            return Err(FileError::BadFd);
        }
        Ok(self.entries[fd]
            .map(|entry| entry.close_on_exec)
            .unwrap_or(false))
    }

    fn snapshot_entries(&self) -> FileResult<[Option<FileDescriptorEntry>; FILE_FD_COUNT]> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }
        self.parent_snapshot_saved.fetch_add(1, Ordering::AcqRel);
        Ok(self.entries)
    }

    fn restore_entries(
        &mut self,
        entries: [Option<FileDescriptorEntry>; FILE_FD_COUNT],
    ) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }
        self.entries = entries;
        self.parent_snapshot_restored.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

pub struct FilesStruct {
    lifecycle: Lifecycle,
    fd_table: FileDescriptorTable,
    stdin: OpenFileDescription,
    stdout: OpenFileDescription,
    stderr: OpenFileDescription,
    regular0: OpenFileDescription,
    null: OpenFileDescription,
    tty0: OpenFileDescription,
    socket0: OpenFileDescription,
    stdin_backend: FileBackend,
    stdout_backend: FileBackend,
    stderr_backend: FileBackend,
    regular0_backend: FileBackend,
    null_backend: FileBackend,
    tty0_backend: FileBackend,
    socket0_backend: FileBackend,
    regular0_buffer: [u8; REGULAR_FILE_BUFFER_SIZE],
    regular0_len: usize,
    regular0_offset: usize,
    regular0_path: [u8; FILE_PATH_MAX],
    regular0_path_len: usize,
    filesystem0_kind: FilesystemFdKind,
    directory0_file_ref: Option<FileRef>,
    directory0_offset: usize,
    directory0_last_getdents_len: usize,
    pidfd_fd: usize,
    pidfd_child_pid: usize,
    pidfd_exit_status: usize,
    socket0_fd: usize,
    pipe0_buffer: PipeBuffer,
    tty_termios: [u8; TERMIOS_SIZE],
    allocated: bool,
    owned_by_kernel_init_task: bool,
    fd_table_bound: bool,
    stdio_bound: bool,
    next_fd_ready: bool,
    close_on_exec_ready: bool,
    shared_deferred: bool,
    stdin_probe_ready_data_cleared: bool,
    stdin_ready_data_bound: bool,
    stdin_blocking_wait_enabled: bool,
    tty_read_wait_entries: AtomicUsize,
    tty_read_wait_finishes: AtomicUsize,
    tty_poll_wait_tables: AtomicUsize,
    tty_poll_freewaits: AtomicUsize,
    tty_wait_interruptible_windows: AtomicUsize,
    tty_wait_ready_wakeups: AtomicUsize,
    fd_lookup_routes_to_table: AtomicUsize,
    regular_file_slot_ready: bool,
    open_path_routes_to_vfs: AtomicUsize,
    read_fd_routes_to_table: AtomicUsize,
    close_fd_routes_to_table: AtomicUsize,
    dup3_routes_to_table: AtomicUsize,
    fchown_fd_routes_to_table: AtomicUsize,
    fchmod_fd_routes_to_table: AtomicUsize,
    stat_path_routes_to_vfs: AtomicUsize,
    readlink_path_routes_to_vfs: AtomicUsize,
    lookup_path_routes_to_vfs: AtomicUsize,
    regular_fd_installed: AtomicUsize,
    null_fd_installed: AtomicUsize,
    fd_owner_recorded: AtomicUsize,
    fd_mode_override_recorded: AtomicUsize,
    fchmod_mode_visible_to_fstat: AtomicUsize,
    null_device_read_eof_observed: AtomicUsize,
    null_device_write_discard_observed: AtomicUsize,
    null_device_fstat_device_node: AtomicUsize,
    null_device_tty_ioctl_enotty: AtomicUsize,
    regular_file_read_observed: AtomicUsize,
    stdin_char_read_observed: AtomicUsize,
    regular_file_closed: AtomicUsize,
    regular_file_stat_observed: AtomicUsize,
    directory_fd_installed: AtomicUsize,
    directory_getdents_observed: AtomicUsize,
    tty_alias_fd_installed: AtomicUsize,
    pidfd_installed: AtomicUsize,
    pidfd_ready: AtomicUsize,
    pidfd_closed: AtomicUsize,
    unix_stream_socket_fd_installed: AtomicUsize,
    unix_stream_socket_fd_closed: AtomicUsize,
    stdio_fd_closed: AtomicUsize,
    close_on_exec_observed: AtomicUsize,
    close_on_exec_scanned: AtomicUsize,
    close_on_exec_closed: AtomicUsize,
    close_on_exec_first_closed_fd: AtomicUsize,
    close_on_exec_remaining_open: AtomicUsize,
    parent_fd_snapshot_saved: AtomicUsize,
    parent_fd_snapshot_restored: AtomicUsize,
    parent_fd_snapshot_live: AtomicUsize,
    parent_pipe_read_snapshot_live: AtomicUsize,
    parent_pipe_write_snapshot_live: AtomicUsize,
    pipe_pairs_installed: AtomicUsize,
    pipe_usercopy_rollbacks: AtomicUsize,
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
            null: OpenFileDescription::new(),
            tty0: OpenFileDescription::new(),
            socket0: OpenFileDescription::new(),
            stdin_backend: FileBackend::new(FileBackendKind::CharDevice),
            stdout_backend: FileBackend::new(FileBackendKind::CharDevice),
            stderr_backend: FileBackend::new(FileBackendKind::CharDevice),
            regular0_backend: FileBackend::new(FileBackendKind::RegularFile),
            null_backend: FileBackend::new(FileBackendKind::CharDevice),
            tty0_backend: FileBackend::new(FileBackendKind::CharDevice),
            socket0_backend: FileBackend::new(FileBackendKind::UnixSocket),
            regular0_buffer: [0; REGULAR_FILE_BUFFER_SIZE],
            regular0_len: 0,
            regular0_offset: 0,
            regular0_path: [0; FILE_PATH_MAX],
            regular0_path_len: 0,
            filesystem0_kind: FilesystemFdKind::None,
            directory0_file_ref: None,
            directory0_offset: 0,
            directory0_last_getdents_len: 0,
            pidfd_fd: usize::MAX,
            pidfd_child_pid: 0,
            pidfd_exit_status: 0,
            socket0_fd: usize::MAX,
            pipe0_buffer: PipeBuffer::new(),
            tty_termios: [0; TERMIOS_SIZE],
            allocated: false,
            owned_by_kernel_init_task: false,
            fd_table_bound: false,
            stdio_bound: false,
            next_fd_ready: false,
            close_on_exec_ready: false,
            shared_deferred: false,
            stdin_probe_ready_data_cleared: false,
            stdin_ready_data_bound: false,
            stdin_blocking_wait_enabled: false,
            tty_read_wait_entries: AtomicUsize::new(0),
            tty_read_wait_finishes: AtomicUsize::new(0),
            tty_poll_wait_tables: AtomicUsize::new(0),
            tty_poll_freewaits: AtomicUsize::new(0),
            tty_wait_interruptible_windows: AtomicUsize::new(0),
            tty_wait_ready_wakeups: AtomicUsize::new(0),
            fd_lookup_routes_to_table: AtomicUsize::new(0),
            regular_file_slot_ready: false,
            open_path_routes_to_vfs: AtomicUsize::new(0),
            read_fd_routes_to_table: AtomicUsize::new(0),
            close_fd_routes_to_table: AtomicUsize::new(0),
            dup3_routes_to_table: AtomicUsize::new(0),
            fchown_fd_routes_to_table: AtomicUsize::new(0),
            fchmod_fd_routes_to_table: AtomicUsize::new(0),
            stat_path_routes_to_vfs: AtomicUsize::new(0),
            readlink_path_routes_to_vfs: AtomicUsize::new(0),
            lookup_path_routes_to_vfs: AtomicUsize::new(0),
            regular_fd_installed: AtomicUsize::new(0),
            null_fd_installed: AtomicUsize::new(0),
            fd_owner_recorded: AtomicUsize::new(0),
            fd_mode_override_recorded: AtomicUsize::new(0),
            fchmod_mode_visible_to_fstat: AtomicUsize::new(0),
            null_device_read_eof_observed: AtomicUsize::new(0),
            null_device_write_discard_observed: AtomicUsize::new(0),
            null_device_fstat_device_node: AtomicUsize::new(0),
            null_device_tty_ioctl_enotty: AtomicUsize::new(0),
            regular_file_read_observed: AtomicUsize::new(0),
            stdin_char_read_observed: AtomicUsize::new(0),
            regular_file_closed: AtomicUsize::new(0),
            regular_file_stat_observed: AtomicUsize::new(0),
            directory_fd_installed: AtomicUsize::new(0),
            directory_getdents_observed: AtomicUsize::new(0),
            tty_alias_fd_installed: AtomicUsize::new(0),
            pidfd_installed: AtomicUsize::new(0),
            pidfd_ready: AtomicUsize::new(0),
            pidfd_closed: AtomicUsize::new(0),
            unix_stream_socket_fd_installed: AtomicUsize::new(0),
            unix_stream_socket_fd_closed: AtomicUsize::new(0),
            stdio_fd_closed: AtomicUsize::new(0),
            close_on_exec_observed: AtomicUsize::new(0),
            close_on_exec_scanned: AtomicUsize::new(0),
            close_on_exec_closed: AtomicUsize::new(0),
            close_on_exec_first_closed_fd: AtomicUsize::new(usize::MAX),
            close_on_exec_remaining_open: AtomicUsize::new(0),
            parent_fd_snapshot_saved: AtomicUsize::new(0),
            parent_fd_snapshot_restored: AtomicUsize::new(0),
            parent_fd_snapshot_live: AtomicUsize::new(0),
            parent_pipe_read_snapshot_live: AtomicUsize::new(0),
            parent_pipe_write_snapshot_live: AtomicUsize::new(0),
            pipe_pairs_installed: AtomicUsize::new(0),
            pipe_usercopy_rollbacks: AtomicUsize::new(0),
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

    pub const fn stdin_probe_ready_data_cleared(&self) -> bool {
        self.stdin_probe_ready_data_cleared
    }

    pub const fn stdin_ready_data_bound(&self) -> bool {
        self.stdin_ready_data_bound
    }

    pub const fn stdin_blocking_wait_enabled(&self) -> bool {
        self.stdin_blocking_wait_enabled
    }

    pub fn tty_read_wait_entry_observed(&self) -> bool {
        self.tty_read_wait_entries.load(Ordering::Acquire) != 0
    }

    pub fn tty_read_wait_finish_observed(&self) -> bool {
        self.tty_read_wait_finishes.load(Ordering::Acquire) != 0
    }

    pub fn tty_poll_wait_table_observed(&self) -> bool {
        self.tty_poll_wait_tables.load(Ordering::Acquire) != 0
    }

    pub fn tty_poll_freewait_observed(&self) -> bool {
        self.tty_poll_freewaits.load(Ordering::Acquire) != 0
    }

    pub fn tty_wait_interruptible_window_observed(&self) -> bool {
        self.tty_wait_interruptible_windows.load(Ordering::Acquire) != 0
    }

    pub fn tty_wait_ready_wakeup_observed(&self) -> bool {
        self.tty_wait_ready_wakeups.load(Ordering::Acquire) != 0
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

    pub fn dup3_routes_to_table(&self) -> bool {
        self.dup3_routes_to_table.load(Ordering::Acquire) != 0
    }

    pub fn fchown_fd_routes_to_table(&self) -> bool {
        self.fchown_fd_routes_to_table.load(Ordering::Acquire) != 0
    }

    pub fn fchmod_fd_routes_to_table(&self) -> bool {
        self.fchmod_fd_routes_to_table.load(Ordering::Acquire) != 0
    }

    pub fn stat_path_routes_to_vfs(&self) -> bool {
        self.stat_path_routes_to_vfs.load(Ordering::Acquire) != 0
    }

    pub fn readlink_path_routes_to_vfs(&self) -> bool {
        self.readlink_path_routes_to_vfs.load(Ordering::Acquire) != 0
    }

    pub fn lookup_path_routes_to_vfs(&self) -> bool {
        self.lookup_path_routes_to_vfs.load(Ordering::Acquire) != 0
    }

    pub fn regular_fd_installed(&self) -> bool {
        self.regular_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn null_fd_installed(&self) -> bool {
        self.null_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn fd_owner_recorded(&self) -> bool {
        self.fd_owner_recorded.load(Ordering::Acquire) != 0
    }

    pub fn fd_mode_override_recorded(&self) -> bool {
        self.fd_mode_override_recorded.load(Ordering::Acquire) != 0
    }

    pub fn fchmod_mode_visible_to_fstat(&self) -> bool {
        self.fchmod_mode_visible_to_fstat.load(Ordering::Acquire) != 0
    }

    pub fn null_device_read_eof_observed(&self) -> bool {
        self.null_device_read_eof_observed.load(Ordering::Acquire) != 0
    }

    pub fn null_device_write_discard_observed(&self) -> bool {
        self.null_device_write_discard_observed
            .load(Ordering::Acquire)
            != 0
    }

    pub fn null_device_fstat_device_node(&self) -> bool {
        self.null_device_fstat_device_node.load(Ordering::Acquire) != 0
    }

    pub fn null_device_tty_ioctl_enotty(&self) -> bool {
        self.null_device_tty_ioctl_enotty.load(Ordering::Acquire) != 0
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

    pub fn tty_alias_fd_installed(&self) -> bool {
        self.tty_alias_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn pidfd_installed(&self) -> bool {
        self.pidfd_installed.load(Ordering::Acquire) != 0
    }

    pub fn pidfd_ready(&self) -> bool {
        self.pidfd_ready.load(Ordering::Acquire) != 0
    }

    pub fn pidfd_closed(&self) -> bool {
        self.pidfd_closed.load(Ordering::Acquire) != 0
    }

    pub fn unix_stream_socket_fd_installed(&self) -> bool {
        self.unix_stream_socket_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn unix_stream_socket_fd_closed(&self) -> bool {
        self.unix_stream_socket_fd_closed.load(Ordering::Acquire) != 0
    }

    pub fn stdio_fd_closed(&self) -> bool {
        self.stdio_fd_closed.load(Ordering::Acquire) != 0
    }

    pub fn close_on_exec_observed(&self) -> bool {
        self.close_on_exec_observed.load(Ordering::Acquire) != 0
    }

    pub fn close_on_exec_report(&self) -> CloseOnExecReport {
        CloseOnExecReport {
            scanned: self.close_on_exec_scanned.load(Ordering::Acquire),
            closed: self.close_on_exec_closed.load(Ordering::Acquire),
            first_closed_fd: self.close_on_exec_first_closed_fd.load(Ordering::Acquire),
            remaining_open: self.close_on_exec_remaining_open.load(Ordering::Acquire),
        }
    }

    pub fn parent_fd_snapshot_saved(&self) -> bool {
        self.parent_fd_snapshot_saved.load(Ordering::Acquire) != 0
            && self.fd_table.parent_snapshot_saved()
    }

    pub fn parent_fd_snapshot_restored(&self) -> bool {
        self.parent_fd_snapshot_restored.load(Ordering::Acquire) != 0
            && self.fd_table.parent_snapshot_restored()
    }

    pub fn pipe_pair_installed(&self) -> bool {
        self.pipe_pairs_installed.load(Ordering::Acquire) != 0
    }

    pub fn pipe_usercopy_rollback_observed(&self) -> bool {
        self.pipe_usercopy_rollbacks.load(Ordering::Acquire) != 0
    }

    pub const fn pipe_buffer_len(&self) -> usize {
        self.pipe0_buffer.len()
    }

    pub fn pipe_read_end_open(&self) -> bool {
        self.fd_table
            .pipe_end_open(OpenFileDescriptionRef::PipeRead0)
    }

    pub fn pipe_write_end_open(&self) -> bool {
        self.fd_table
            .pipe_end_open(OpenFileDescriptionRef::PipeWrite0)
    }

    fn pipe_reader_available(&self) -> bool {
        self.pipe_read_end_open()
            || self.parent_pipe_read_snapshot_live.load(Ordering::Acquire) != 0
    }

    fn pipe_writer_available(&self) -> bool {
        self.pipe_write_end_open()
            || self.parent_pipe_write_snapshot_live.load(Ordering::Acquire) != 0
    }

    pub const fn pidfd_fd(&self) -> usize {
        self.pidfd_fd
    }

    pub const fn pidfd_child_pid(&self) -> usize {
        self.pidfd_child_pid
    }

    pub const fn pidfd_exit_status(&self) -> usize {
        self.pidfd_exit_status
    }

    pub const fn socket0_fd(&self) -> usize {
        self.socket0_fd
    }

    pub fn fd_is_unix_socket0(&self, fd: usize) -> bool {
        matches!(
            self.fd_table_entry_diagnostic(fd),
            Some(entry) if entry.ofd == OpenFileDescriptionRef::UnixSocket0
        )
    }

    pub fn tty_termios_state_bound(&self) -> bool {
        self.lifecycle.state() == State::Ready
    }

    pub fn tty_termios_mutation_observed(&self) -> bool {
        self.tty_termios_mutation_observed.load(Ordering::Acquire) != 0
    }

    fn tty_canonical_mode(&self) -> bool {
        read_u32_raw(&self.tty_termios, 12) & TERMIOS_ICANON != 0
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

    pub fn fd_table_open_count(&self) -> usize {
        self.fd_table.open_count()
    }

    pub fn fd_table_capacity(&self) -> usize {
        self.fd_table.capacity()
    }

    pub fn fd_table_entry_diagnostic(&self, fd: usize) -> Option<FdEntryDiagnostic> {
        self.fd_table.entry_diagnostic(fd)
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

    pub const fn null(&self) -> &OpenFileDescription {
        &self.null
    }

    pub const fn tty0(&self) -> &OpenFileDescription {
        &self.tty0
    }

    pub const fn socket0(&self) -> &OpenFileDescription {
        &self.socket0
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

    pub const fn null_backend(&self) -> &FileBackend {
        &self.null_backend
    }

    pub const fn tty0_backend(&self) -> &FileBackend {
        &self.tty0_backend
    }

    pub const fn socket0_backend(&self) -> &FileBackend {
        &self.socket0_backend
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
        self.null_backend.setup_null_device()?;
        self.stdin.setup_stdio(&self.stdin_backend, true, false)?;
        self.stdout.setup_stdio(&self.stdout_backend, false, true)?;
        self.stderr.setup_stdio(&self.stderr_backend, false, true)?;
        self.tty0.setup_stdio(&self.tty0_backend, true, true)?;
        self.null.setup_null_device(&self.null_backend)?;
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

    pub fn clear_stdin_ready_data(&mut self) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound || !self.stdio_bound {
            return Err(FileError::NotReady);
        }
        if !self.fd_table.fd_bound(FdRef::Stdin)
            || self.stdin.state() != State::Ready
            || self.stdin_backend.state() != State::Ready
            || !self.stdin_backend.char_device_read_supported()
        {
            return Err(FileError::BackendUnavailable);
        }
        if !crate::objects::ns16550a::clear_tty_ready_data() {
            return Err(FileError::BackendUnavailable);
        }

        self.stdin_probe_ready_data_cleared = true;
        self.stdin_ready_data_bound = false;
        Ok(())
    }

    pub fn enable_stdin_blocking_wait(&mut self) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.stdio_bound
            || !self.stdin_probe_ready_data_cleared
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

        self.stdin_blocking_wait_enabled = true;
        Ok(())
    }

    pub fn record_tty_read_wait_entry(&self) {
        self.tty_read_wait_entries.fetch_add(1, Ordering::AcqRel);
        self.tty_wait_interruptible_windows
            .fetch_add(1, Ordering::AcqRel);
    }

    pub fn record_tty_read_wait_finish(&self, ready: bool) {
        self.tty_read_wait_finishes.fetch_add(1, Ordering::AcqRel);
        if ready {
            self.tty_wait_ready_wakeups.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub fn record_tty_poll_wait_table(&self) {
        self.tty_poll_wait_tables.fetch_add(1, Ordering::AcqRel);
        self.tty_wait_interruptible_windows
            .fetch_add(1, Ordering::AcqRel);
    }

    pub fn record_tty_poll_freewait(&self, ready: bool) {
        self.tty_poll_freewaits.fetch_add(1, Ordering::AcqRel);
        if ready {
            self.tty_wait_ready_wakeups.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub fn prepare_default_stdin_ready_data(&mut self, bytes: &[u8]) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.stdio_bound
            || !self.stdin_probe_ready_data_cleared
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

    // Regular-file open keeps the VFS, block provider and pathname boundary explicit.
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

    // Filesystem open preserves the common VFS lookup inputs used for files and directories.
    #[allow(clippy::too_many_arguments)]
    pub fn open_filesystem_path(
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
        let (file_ref, kind) = vfs_core
            .open_existing_path(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
                open_flags & FILE_O_DIRECTORY != 0,
            )
            .map_err(vfs_error_to_file_error)?;

        let regular_len = match kind {
            VfsInodeKind::RegularFile => {
                self.regular0_buffer.fill(0);
                Some(
                    vfs_core
                        .read_opened_file(
                            ext2_filesystem,
                            block_device_registry,
                            &mut provider,
                            file_ref,
                            0,
                            &mut self.regular0_buffer,
                        )
                        .map_err(vfs_error_to_file_error)?,
                )
            }
            VfsInodeKind::Directory => None,
            VfsInodeKind::DeviceNode | VfsInodeKind::Symlink => {
                return Err(FileError::BackendUnavailable);
            }
        };

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
        self.regular0_path.fill(0);
        self.regular0_path[..path.len()].copy_from_slice(path);
        self.regular0_path_len = path.len();
        self.open_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);

        match kind {
            VfsInodeKind::RegularFile => {
                let len = regular_len.ok_or(FileError::BackendUnavailable)?;
                self.regular0_len = len;
                self.regular0_offset = 0;
                self.filesystem0_kind = FilesystemFdKind::RegularFile;
                self.directory0_file_ref = None;
                self.directory0_offset = 0;
                self.directory0_last_getdents_len = 0;
                self.regular_fd_installed.fetch_add(1, Ordering::AcqRel);
            }
            VfsInodeKind::Directory => {
                self.regular0_len = 0;
                self.regular0_offset = 0;
                self.filesystem0_kind = FilesystemFdKind::Directory;
                self.directory0_file_ref = Some(file_ref);
                self.directory0_offset = 0;
                self.directory0_last_getdents_len = 0;
                self.directory_fd_installed.fetch_add(1, Ordering::AcqRel);
            }
            VfsInodeKind::DeviceNode | VfsInodeKind::Symlink => {
                return Err(FileError::BackendUnavailable);
            }
        }

        Ok(fd)
    }

    // Directory open uses the same explicit VFS and block-provider boundary as regular open.
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
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
            || !is_tty_path(path)
        {
            return Err(FileError::PathUnavailable);
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
        self.tty_alias_fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn open_null_path(&mut self, path: &[u8], open_flags: u32) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
            || !is_null_path(path)
        {
            return Err(FileError::PathUnavailable);
        }
        if open_flags & FILE_O_DIRECTORY != 0 {
            return Err(FileError::NotDirectory);
        }

        let access_mode = open_flags & FILE_O_ACCMODE;
        if access_mode == FILE_O_ACCMODE {
            return Err(FileError::InvalidArgument);
        }
        let readable = access_mode != FILE_O_WRONLY;
        let writable = access_mode != FILE_O_RDONLY;
        let fd = self.fd_table.install_opened(
            &self.null,
            OpenFileDescriptionRef::Null,
            readable,
            writable,
            persistent_open_flags(open_flags),
            open_flags & FILE_O_CLOEXEC != 0,
        )?;
        self.null_fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn open_unix_stream_socket(
        &mut self,
        nonblock: bool,
        close_on_exec: bool,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.socket0_fd = self
            .fd_table
            .first_fd_for_ofd(OpenFileDescriptionRef::UnixSocket0)
            .unwrap_or(usize::MAX);
        if self.socket0_fd != usize::MAX {
            return Err(FileError::AlreadyOpen);
        }

        if self.socket0_backend.state() == State::Base {
            self.socket0_backend
                .bind_unix_stream_socket()
                .map_err(|_| FileError::BackendUnavailable)?;
        }
        if self.socket0.state() == State::Base {
            self.socket0
                .setup_unix_socket(&self.socket0_backend)
                .map_err(|_| FileError::BackendUnavailable)?;
        }

        let mut flags = FILE_O_RDWR;
        if nonblock {
            flags |= FILE_O_NONBLOCK;
        }
        let fd = self.fd_table.install_opened(
            &self.socket0,
            OpenFileDescriptionRef::UnixSocket0,
            true,
            true,
            flags,
            close_on_exec,
        )?;
        self.socket0_fd = fd;
        self.unix_stream_socket_fd_installed
            .fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn install_pidfd(&mut self, child_pid: usize) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound || child_pid == 0 {
            return Err(FileError::NotReady);
        }
        if self.pidfd_child_pid != 0 && self.pidfd_fd != usize::MAX {
            return Err(FileError::AlreadyOpen);
        }

        let fd = self.fd_table.install_pidfd(child_pid)?;
        self.pidfd_fd = fd;
        self.pidfd_child_pid = child_pid;
        self.pidfd_exit_status = 0;
        self.pidfd_ready.store(0, Ordering::Release);
        self.pidfd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn mark_pidfd_child_exited(
        &mut self,
        child_pid: usize,
        exit_status: usize,
    ) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }
        if self.pidfd_child_pid != child_pid || self.pidfd_child_pid == 0 {
            return Err(FileError::BadFd);
        }

        self.pidfd_exit_status = exit_status;
        self.pidfd_ready.store(1, Ordering::Release);
        Ok(())
    }

    pub fn pipe2_fd_pair(&mut self, flags: u32) -> FileResult<[usize; 2]> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }
        if flags != 0 {
            return Err(FileError::InvalidArgument);
        }
        if self.pipe_reader_available() || self.pipe_writer_available() {
            return Err(FileError::TooManyOpenFiles);
        }

        let pair = self.fd_table.install_pipe_pair()?;
        self.pipe0_buffer.reset();
        self.pipe_pairs_installed.fetch_add(1, Ordering::AcqRel);
        Ok(pair)
    }

    pub fn rollback_pipe2_usercopy(&mut self, pair: [usize; 2]) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }
        let read_entry = self.fd_table.lookup(pair[0])?;
        let write_entry = self.fd_table.lookup(pair[1])?;
        if read_entry.ofd != OpenFileDescriptionRef::PipeRead0
            || write_entry.ofd != OpenFileDescriptionRef::PipeWrite0
        {
            return Err(FileError::InvalidArgument);
        }

        let read_entry = self.fd_table.close(pair[0])?;
        self.finish_closed_entry(pair[0], read_entry);
        let write_entry = self.fd_table.close(pair[1])?;
        self.finish_closed_entry(pair[1], write_entry);
        self.pipe0_buffer.reset();
        self.pipe_usercopy_rollbacks.fetch_add(1, Ordering::AcqRel);
        Ok(())
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
                let canonical = self.tty_canonical_mode();
                let read = self
                    .stdin
                    .read_char_device(&self.stdin_backend, buffer, canonical)?;
                if read != 0 {
                    self.stdin_char_read_observed.fetch_add(1, Ordering::AcqRel);
                }
                Ok(read)
            }
            OpenFileDescriptionRef::Tty0 => {
                let canonical = self.tty_canonical_mode();
                let read = self
                    .tty0
                    .read_char_device(&self.tty0_backend, buffer, canonical)?;
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
            OpenFileDescriptionRef::Null => {
                let read = self.null.read_null_device(&self.null_backend)?;
                self.null_device_read_eof_observed
                    .fetch_add(1, Ordering::AcqRel);
                Ok(read)
            }
            OpenFileDescriptionRef::UnixSocket0 => Err(FileError::Unsupported),
            OpenFileDescriptionRef::Pidfd0 => Err(FileError::Unsupported),
            OpenFileDescriptionRef::PipeRead0 => {
                let read = self.pipe0_buffer.read(buffer);
                if read != 0 || !self.pipe_writer_available() {
                    Ok(read)
                } else {
                    Err(FileError::NotReady)
                }
            }
            OpenFileDescriptionRef::PipeWrite0 => Err(FileError::NotReadable),
            OpenFileDescriptionRef::Stdout | OpenFileDescriptionRef::Stderr => {
                Err(FileError::NotReadable)
            }
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
                    if backend.char_device_read_ready(self.tty_canonical_mode())? {
                        ready |= FILE_POLLIN | FILE_POLLRDNORM;
                    }
                }
                OpenFileDescriptionRef::Pidfd0 => {
                    if entry.pid == self.pidfd_child_pid
                        && self.pidfd_ready.load(Ordering::Acquire) != 0
                    {
                        ready |= FILE_POLLIN | FILE_POLLRDNORM;
                    }
                }
                OpenFileDescriptionRef::Null => {
                    ready |= FILE_POLLIN | FILE_POLLRDNORM;
                }
                OpenFileDescriptionRef::PipeRead0 => {
                    if self.pipe0_buffer.len() != 0 || !self.pipe_writer_available() {
                        ready |= FILE_POLLIN | FILE_POLLRDNORM;
                    }
                }
                OpenFileDescriptionRef::PipeWrite0 => {}
                OpenFileDescriptionRef::UnixSocket0 => return Err(FileError::Unsupported),
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
                OpenFileDescriptionRef::Stdin
                | OpenFileDescriptionRef::Regular0
                | OpenFileDescriptionRef::Null
                | OpenFileDescriptionRef::Pidfd0
                | OpenFileDescriptionRef::PipeRead0 => {}
                OpenFileDescriptionRef::PipeWrite0 => {
                    if self.pipe_reader_available() && self.pipe0_buffer.len() < PIPE_BUFFER_SIZE {
                        ready |= FILE_POLLOUT | FILE_POLLWRNORM;
                    }
                }
                OpenFileDescriptionRef::UnixSocket0 => return Err(FileError::Unsupported),
            }
            if entry.ofd == OpenFileDescriptionRef::Null {
                ready |= FILE_POLLOUT | FILE_POLLWRNORM;
            }
        }

        Ok(ready & (events | FILE_POLLERR | FILE_POLLHUP))
    }

    pub fn fd_is_tty_read_wait_candidate(&self, fd: usize) -> bool {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return false;
        }

        match self.fd_table.lookup(fd) {
            Ok(entry) => {
                entry.readable
                    && matches!(
                        entry.ofd,
                        OpenFileDescriptionRef::Stdin | OpenFileDescriptionRef::Tty0
                    )
            }
            Err(_) => false,
        }
    }

    pub fn fd_is_pipe_read_wait_candidate(&self, fd: usize) -> bool {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return false;
        }
        self.fd_table
            .lookup(fd)
            .is_ok_and(|entry| entry.readable && entry.ofd == OpenFileDescriptionRef::PipeRead0)
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
        self.finish_closed_entry(fd, entry);
        self.close_fd_routes_to_table.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn finish_closed_entry(&mut self, fd: usize, entry: FileDescriptorEntry) {
        if entry.ofd == OpenFileDescriptionRef::Regular0
            && self
                .fd_table
                .first_fd_for_ofd(OpenFileDescriptionRef::Regular0)
                .is_none()
        {
            self.regular0_offset = 0;
            self.directory0_offset = 0;
            self.directory0_file_ref = None;
            self.filesystem0_kind = FilesystemFdKind::None;
        }
        if entry.ofd == OpenFileDescriptionRef::Pidfd0 {
            self.pidfd_fd = usize::MAX;
            self.pidfd_closed.fetch_add(1, Ordering::AcqRel);
        }
        if entry.ofd == OpenFileDescriptionRef::UnixSocket0 {
            self.socket0_fd = self
                .fd_table
                .first_fd_for_ofd(OpenFileDescriptionRef::UnixSocket0)
                .unwrap_or(usize::MAX);
            if self.socket0_fd == usize::MAX {
                self.unix_stream_socket_fd_closed
                    .fetch_add(1, Ordering::AcqRel);
            }
        }
        if matches!(
            entry.ofd,
            OpenFileDescriptionRef::PipeRead0 | OpenFileDescriptionRef::PipeWrite0
        ) && !self.pipe_read_end_open()
            && !self.pipe_write_end_open()
            && self.parent_fd_snapshot_live.load(Ordering::Acquire) == 0
        {
            self.pipe0_buffer.reset();
        }
        if matches!(fd, STDIN_FD | STDOUT_FD | STDERR_FD) {
            self.stdio_fd_closed.fetch_add(1, Ordering::AcqRel);
        }
        if entry.ofd == OpenFileDescriptionRef::Regular0 {
            self.regular_file_closed.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub fn close_on_exec(&mut self) -> FileResult<CloseOnExecReport> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.close_on_exec_ready
        {
            return Err(FileError::NotReady);
        }

        let mut report = CloseOnExecReport::empty();
        let mut fd = 0usize;
        while fd < self.fd_table.capacity() {
            report.scanned += 1;
            if self.fd_table.close_on_exec_set(fd)? {
                let entry = self.fd_table.close(fd)?;
                self.finish_closed_entry(fd, entry);
                if report.closed == 0 {
                    report.first_closed_fd = fd;
                }
                report.closed += 1;
            }
            fd += 1;
        }
        report.remaining_open = self.fd_table.open_count();

        self.close_on_exec_observed.fetch_add(1, Ordering::AcqRel);
        self.close_on_exec_scanned
            .store(report.scanned, Ordering::Release);
        self.close_on_exec_closed
            .store(report.closed, Ordering::Release);
        self.close_on_exec_first_closed_fd
            .store(report.first_closed_fd, Ordering::Release);
        self.close_on_exec_remaining_open
            .store(report.remaining_open, Ordering::Release);
        Ok(report)
    }

    pub fn precheck_close_on_exec(&self) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || !self.close_on_exec_ready
            || self.fd_table.capacity() == 0
        {
            return Err(FileError::NotReady);
        }
        Ok(())
    }

    pub fn save_parent_fd_snapshot(&self) -> FileResult<FilesStructSnapshot> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let snapshot = FilesStructSnapshot {
            entries: self.fd_table.snapshot_entries()?,
            regular0_len: self.regular0_len,
            regular0_offset: self.regular0_offset,
            regular0_path: self.regular0_path,
            regular0_path_len: self.regular0_path_len,
            filesystem0_kind: self.filesystem0_kind,
            directory0_file_ref: self.directory0_file_ref,
            directory0_offset: self.directory0_offset,
            directory0_last_getdents_len: self.directory0_last_getdents_len,
            pidfd_fd: self.pidfd_fd,
            pidfd_child_pid: self.pidfd_child_pid,
            pidfd_exit_status: self.pidfd_exit_status,
            socket0_fd: self.socket0_fd,
            pipe_read_end_open: self.pipe_read_end_open(),
            pipe_write_end_open: self.pipe_write_end_open(),
        };
        self.parent_fd_snapshot_live.fetch_add(1, Ordering::AcqRel);
        if snapshot.pipe_read_end_open {
            self.parent_pipe_read_snapshot_live
                .fetch_add(1, Ordering::AcqRel);
        }
        if snapshot.pipe_write_end_open {
            self.parent_pipe_write_snapshot_live
                .fetch_add(1, Ordering::AcqRel);
        }
        self.parent_fd_snapshot_saved.fetch_add(1, Ordering::AcqRel);
        Ok(snapshot)
    }

    pub fn restore_parent_fd_snapshot(&mut self, snapshot: &FilesStructSnapshot) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.restore_entries(snapshot.entries)?;
        self.regular0_len = snapshot.regular0_len;
        self.regular0_offset = snapshot.regular0_offset;
        self.regular0_path = snapshot.regular0_path;
        self.regular0_path_len = snapshot.regular0_path_len;
        self.filesystem0_kind = snapshot.filesystem0_kind;
        self.directory0_file_ref = snapshot.directory0_file_ref;
        self.directory0_offset = snapshot.directory0_offset;
        self.directory0_last_getdents_len = snapshot.directory0_last_getdents_len;
        self.pidfd_fd = snapshot.pidfd_fd;
        self.pidfd_child_pid = snapshot.pidfd_child_pid;
        self.pidfd_exit_status = snapshot.pidfd_exit_status;
        self.socket0_fd = snapshot.socket0_fd;
        if snapshot.pipe_read_end_open
            && self.parent_pipe_read_snapshot_live.load(Ordering::Acquire) != 0
        {
            self.parent_pipe_read_snapshot_live
                .fetch_sub(1, Ordering::AcqRel);
        }
        if snapshot.pipe_write_end_open
            && self.parent_pipe_write_snapshot_live.load(Ordering::Acquire) != 0
        {
            self.parent_pipe_write_snapshot_live
                .fetch_sub(1, Ordering::AcqRel);
        }
        if self.parent_fd_snapshot_live.load(Ordering::Acquire) != 0 {
            self.parent_fd_snapshot_live.fetch_sub(1, Ordering::AcqRel);
        }
        self.parent_fd_snapshot_restored
            .fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    pub fn discard_parent_fd_snapshot(&self, snapshot: &FilesStructSnapshot) {
        if snapshot.pipe_read_end_open
            && self.parent_pipe_read_snapshot_live.load(Ordering::Acquire) != 0
        {
            self.parent_pipe_read_snapshot_live
                .fetch_sub(1, Ordering::AcqRel);
        }
        if snapshot.pipe_write_end_open
            && self.parent_pipe_write_snapshot_live.load(Ordering::Acquire) != 0
        {
            self.parent_pipe_write_snapshot_live
                .fetch_sub(1, Ordering::AcqRel);
        }
        if self.parent_fd_snapshot_live.load(Ordering::Acquire) != 0 {
            self.parent_fd_snapshot_live.fetch_sub(1, Ordering::AcqRel);
        }
    }

    pub fn fcntl_getfl_fd(&self, fd: usize) -> FileResult<u32> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        Ok(entry.flags)
    }

    pub fn fcntl_setfl_fd(&mut self, fd: usize, flags: u32) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let entry = self.fd_table.lookup(fd)?;
        if entry.ofd != OpenFileDescriptionRef::Tty0 {
            return Err(FileError::InvalidArgument);
        }
        self.char_backend_for_entry(entry)?;
        self.fd_table.set_status_flags(fd, flags)
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

    pub fn dup3_fd(
        &mut self,
        oldfd: usize,
        newfd: usize,
        close_on_exec: bool,
    ) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        let (fd, replaced) = self.fd_table.dup3_fd(oldfd, newfd, close_on_exec)?;
        if let Some(entry) = replaced {
            self.finish_closed_entry(fd, entry);
        }
        self.dup3_routes_to_table.fetch_add(1, Ordering::AcqRel);
        Ok(fd)
    }

    pub fn fchown_fd(&mut self, fd: usize, uid: usize, gid: usize) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.update_owner(fd, uid, gid)?;
        self.fchown_fd_routes_to_table
            .fetch_add(1, Ordering::AcqRel);
        self.fd_owner_recorded.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    pub fn fchmod_fd(&mut self, fd: usize, mode: u32) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.update_mode(fd, mode)?;
        self.fchmod_fd_routes_to_table
            .fetch_add(1, Ordering::AcqRel);
        self.fd_mode_override_recorded
            .fetch_add(1, Ordering::AcqRel);
        Ok(())
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
            OpenFileDescriptionRef::Null => {
                self.null_device_tty_ioctl_enotty
                    .fetch_add(1, Ordering::AcqRel);
                return Err(FileError::NotTty);
            }
            OpenFileDescriptionRef::Pidfd0 => return Err(FileError::NotTty),
            OpenFileDescriptionRef::UnixSocket0 => return Err(FileError::NotTty),
            OpenFileDescriptionRef::PipeRead0 | OpenFileDescriptionRef::PipeWrite0 => {
                return Err(FileError::NotTty);
            }
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

    pub fn ioctl_tiocgsid_fd(&self, fd: usize) -> FileResult<()> {
        self.char_backend_for_fd(fd)?;
        Ok(())
    }

    pub fn ioctl_tiocsctty_fd(&self, fd: usize) -> FileResult<()> {
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

    // Path stat keeps lookup policy and every backing filesystem object explicit.
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
        let stat = match entry.ofd {
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
                stat
            }
            OpenFileDescriptionRef::Pidfd0 => FileStat::new(0, VfsInodeKind::DeviceNode),
            OpenFileDescriptionRef::Null => {
                self.null_device_fstat_device_node
                    .fetch_add(1, Ordering::AcqRel);
                FileStat::new(0, VfsInodeKind::DeviceNode)
            }
            OpenFileDescriptionRef::UnixSocket0 => FileStat::socket(0),
            OpenFileDescriptionRef::PipeRead0 | OpenFileDescriptionRef::PipeWrite0 => {
                FileStat::fifo(self.pipe0_buffer.len())
            }
            OpenFileDescriptionRef::Stdin
            | OpenFileDescriptionRef::Stdout
            | OpenFileDescriptionRef::Stderr
            | OpenFileDescriptionRef::Tty0 => FileStat::new(0, VfsInodeKind::DeviceNode),
        };
        let stat = entry.apply_mode_override(stat);
        if entry.mode_override_valid {
            self.fchmod_mode_visible_to_fstat
                .fetch_add(1, Ordering::AcqRel);
        }
        Ok(stat)
    }

    pub fn write_fd(&mut self, fd: usize, bytes: &[u8]) -> FileResult<usize> {
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
            OpenFileDescriptionRef::Null => {
                let written = self.null.write_null_device(&self.null_backend, bytes)?;
                self.null_device_write_discard_observed
                    .fetch_add(1, Ordering::AcqRel);
                Ok(written)
            }
            OpenFileDescriptionRef::Tty0 => self.tty0.write(&self.tty0_backend, bytes),
            OpenFileDescriptionRef::Pidfd0 => Err(FileError::NotWritable),
            OpenFileDescriptionRef::UnixSocket0 => Err(FileError::Unsupported),
            OpenFileDescriptionRef::PipeRead0 => Err(FileError::NotWritable),
            OpenFileDescriptionRef::PipeWrite0 => {
                if !self.pipe_reader_available() {
                    return Err(FileError::BrokenPipe);
                }
                self.pipe0_buffer.write(bytes)
            }
        }
    }

    // Access checks retain the complete pathname lookup boundary before applying mode policy.
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

    // Kind lookup forwards the complete VFS and live block-provider context.
    #[allow(clippy::too_many_arguments)]
    pub fn lookup_path_kind(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
    ) -> FileResult<VfsInodeKind> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
        {
            return Err(FileError::NotReady);
        }

        let mut provider = virtio_blk::live_provider(kernel_image);
        let kind = vfs_core
            .lookup_path_kind(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
            )
            .map_err(vfs_error_to_file_error)?;
        self.lookup_path_routes_to_vfs
            .fetch_add(1, Ordering::AcqRel);
        Ok(kind)
    }

    // Symlink reads keep the pathname, output buffer and backing filesystem context explicit.
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

fn read_u32_raw(buffer: &[u8], offset: usize) -> u32 {
    let len = core::mem::size_of::<u32>();
    let mut bytes = [0u8; core::mem::size_of::<u32>()];
    bytes.copy_from_slice(&buffer[offset..offset + len]);
    u32::from_le_bytes(bytes)
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
    flags
        & (FILE_O_ACCMODE | FILE_O_NONBLOCK | FILE_O_LARGEFILE | FILE_O_DIRECTORY)
        & !FILE_O_CLOEXEC
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
