use core::sync::atomic::{AtomicUsize, Ordering};

use super::{
    block_device::BlockDeviceRegistry,
    ext2::Ext2FileSystem,
    kernel_image::KernelImage,
    rest_init::KernelInitTask,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vfs::{FsStruct, VfsCore, VfsError},
    virtio_blk,
};

pub const STDIN_FD: usize = 0;
pub const STDOUT_FD: usize = 1;
pub const STDERR_FD: usize = 2;
pub const REGULAR0_FD: usize = 3;
pub const FILE_PATH_MAX: usize = 128;
pub const REGULAR_FILE_BUFFER_SIZE: usize = 4096;
const FILE_FD_COUNT: usize = 4;

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
}

pub type FileResult<T> = Result<T, FileError>;

fn vfs_error_to_file_error(error: VfsError) -> FileError {
    match error {
        VfsError::NotFound => FileError::PathUnavailable,
        VfsError::ShortBuffer => FileError::BufferTooSmall,
        VfsError::Backend => FileError::VfsBackendUnavailable,
        _ => FileError::BackendUnavailable,
    }
}

#[derive(Clone, Copy)]
struct FileDescriptorEntry {
    ofd: OpenFileDescriptionRef,
    readable: bool,
    writable: bool,
}

impl FileDescriptorEntry {
    const fn stdio(ofd: OpenFileDescriptionRef, readable: bool, writable: bool) -> Self {
        Self {
            ofd,
            readable,
            writable,
        }
    }

    const fn regular(ofd: OpenFileDescriptionRef) -> Self {
        Self {
            ofd,
            readable: true,
            writable: false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OpenFileDescriptionRef {
    Stdin,
    Stdout,
    Stderr,
    Regular0,
}

#[derive(Clone, Copy)]
pub struct FileStat {
    size: usize,
    mode: u32,
}

impl FileStat {
    const fn regular(size: usize) -> Self {
        Self {
            size,
            mode: 0o100444,
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
    regular_file_bound: bool,
    regular_file_read_supported: bool,
    regular_file_stat_supported: bool,
    regular_file_deferred: bool,
    block_device_deferred: bool,
    write_to_console: AtomicUsize,
    last_write_len: AtomicUsize,
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
            regular_file_bound: false,
            regular_file_read_supported: false,
            regular_file_stat_supported: false,
            regular_file_deferred: false,
            block_device_deferred: false,
            write_to_console: AtomicUsize::new(0),
            last_write_len: AtomicUsize::new(0),
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

        let fd_ref = FdRef::from_fd(fd).ok_or(FileError::BadFd)?;
        let entry = self.entries[fd_ref.index()].ok_or(FileError::BadFd)?;
        self.lookup_returns.fetch_add(1, Ordering::AcqRel);
        Ok(entry)
    }

    fn install_regular(&mut self, ofd: &OpenFileDescription) -> FileResult<usize> {
        if self.lifecycle.state() != State::Ready || ofd.state() != State::Ready || !ofd.readable()
        {
            return Err(FileError::NotReady);
        }
        if self.entries[FdRef::Regular0.index()].is_some() {
            return Err(FileError::AlreadyOpen);
        }

        self.entries[FdRef::Regular0.index()] = Some(FileDescriptorEntry::regular(
            OpenFileDescriptionRef::Regular0,
        ));
        self.fd_installed.fetch_add(1, Ordering::AcqRel);
        Ok(REGULAR0_FD)
    }

    fn close(&mut self, fd: usize) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready {
            return Err(FileError::NotReady);
        }

        let fd_ref = FdRef::from_fd(fd).ok_or(FileError::BadFd)?;
        if matches!(fd_ref, FdRef::Stdin | FdRef::Stdout | FdRef::Stderr) {
            return Err(FileError::Unsupported);
        }
        if self.entries[fd_ref.index()].is_none() {
            return Err(FileError::BadFd);
        }

        self.entries[fd_ref.index()] = None;
        self.fd_closed.fetch_add(1, Ordering::AcqRel);
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
    stdin_backend: FileBackend,
    stdout_backend: FileBackend,
    stderr_backend: FileBackend,
    regular0_backend: FileBackend,
    regular0_buffer: [u8; REGULAR_FILE_BUFFER_SIZE],
    regular0_len: usize,
    regular0_offset: usize,
    regular0_path: [u8; FILE_PATH_MAX],
    regular0_path_len: usize,
    allocated: bool,
    owned_by_kernel_init_task: bool,
    fd_table_bound: bool,
    stdio_bound: bool,
    next_fd_ready: bool,
    close_on_exec_ready: bool,
    shared_deferred: bool,
    fd_lookup_routes_to_table: AtomicUsize,
    regular_file_slot_ready: bool,
    open_path_routes_to_vfs: AtomicUsize,
    read_fd_routes_to_table: AtomicUsize,
    close_fd_routes_to_table: AtomicUsize,
    stat_path_routes_to_vfs: AtomicUsize,
    regular_fd_installed: AtomicUsize,
    regular_file_read_observed: AtomicUsize,
    regular_file_closed: AtomicUsize,
    regular_file_stat_observed: AtomicUsize,
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
            stdin_backend: FileBackend::new(FileBackendKind::CharDevice),
            stdout_backend: FileBackend::new(FileBackendKind::CharDevice),
            stderr_backend: FileBackend::new(FileBackendKind::CharDevice),
            regular0_backend: FileBackend::new(FileBackendKind::RegularFile),
            regular0_buffer: [0; REGULAR_FILE_BUFFER_SIZE],
            regular0_len: 0,
            regular0_offset: 0,
            regular0_path: [0; FILE_PATH_MAX],
            regular0_path_len: 0,
            allocated: false,
            owned_by_kernel_init_task: false,
            fd_table_bound: false,
            stdio_bound: false,
            next_fd_ready: false,
            close_on_exec_ready: false,
            shared_deferred: false,
            fd_lookup_routes_to_table: AtomicUsize::new(0),
            regular_file_slot_ready: false,
            open_path_routes_to_vfs: AtomicUsize::new(0),
            read_fd_routes_to_table: AtomicUsize::new(0),
            close_fd_routes_to_table: AtomicUsize::new(0),
            stat_path_routes_to_vfs: AtomicUsize::new(0),
            regular_fd_installed: AtomicUsize::new(0),
            regular_file_read_observed: AtomicUsize::new(0),
            regular_file_closed: AtomicUsize::new(0),
            regular_file_stat_observed: AtomicUsize::new(0),
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

    pub fn regular_fd_installed(&self) -> bool {
        self.regular_fd_installed.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_read_observed(&self) -> bool {
        self.regular_file_read_observed.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_closed(&self) -> bool {
        self.regular_file_closed.load(Ordering::Acquire) != 0
    }

    pub fn regular_file_stat_observed(&self) -> bool {
        self.regular_file_stat_observed.load(Ordering::Acquire) != 0
    }

    pub const fn regular0_len(&self) -> usize {
        self.regular0_len
    }

    pub const fn regular0_offset(&self) -> usize {
        self.regular0_offset
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
        self.stdin.setup_stdio(&self.stdin_backend, true, false)?;
        self.stdout.setup_stdio(&self.stdout_backend, false, true)?;
        self.stderr.setup_stdio(&self.stderr_backend, false, true)?;
        self.fd_table
            .install_stdio(&self.stdin, &self.stdout, &self.stderr)?;

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

    #[allow(clippy::too_many_arguments)]
    pub fn open_regular_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
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

        let fd = self.fd_table.install_regular(&self.regular0)?;
        self.regular0_len = len;
        self.regular0_offset = 0;
        self.regular0_path.fill(0);
        self.regular0_path[..path.len()].copy_from_slice(path);
        self.regular0_path_len = path.len();
        self.open_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        self.regular_fd_installed.fetch_add(1, Ordering::AcqRel);
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
            OpenFileDescriptionRef::Regular0 => {
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

    pub fn close_fd(&mut self, fd: usize) -> FileResult<()> {
        if self.lifecycle.state() != State::Ready || !self.fd_table_bound {
            return Err(FileError::NotReady);
        }

        self.fd_table.close(fd)?;
        self.regular0_offset = 0;
        self.close_fd_routes_to_table.fetch_add(1, Ordering::AcqRel);
        self.regular_file_closed.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stat_regular_path(
        &mut self,
        fs_struct: &FsStruct,
        vfs_core: &mut VfsCore,
        ext2_filesystem: &mut Ext2FileSystem,
        block_device_registry: &mut BlockDeviceRegistry,
        kernel_image: &KernelImage,
        path: &[u8],
    ) -> FileResult<FileStat> {
        if self.lifecycle.state() != State::Ready
            || !self.fd_table_bound
            || path.is_empty()
            || path.len() > FILE_PATH_MAX
        {
            return Err(FileError::NotReady);
        }

        let mut scratch = [0u8; REGULAR_FILE_BUFFER_SIZE];
        let mut provider = virtio_blk::live_provider(kernel_image);
        let len = vfs_core
            .read_path(
                fs_struct,
                ext2_filesystem,
                block_device_registry,
                &mut provider,
                path,
                &mut scratch,
            )
            .map_err(vfs_error_to_file_error)?;

        if self.regular0_backend.state() == State::Base {
            self.regular0_backend
                .bind_regular_file()
                .map_err(|_| FileError::BackendUnavailable)?;
        }
        let stat = self
            .regular0_backend
            .stat_regular_file(FileStat::regular(len))?;
        self.stat_path_routes_to_vfs.fetch_add(1, Ordering::AcqRel);
        self.regular_file_stat_observed
            .fetch_add(1, Ordering::AcqRel);
        Ok(stat)
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
        }
    }
}
