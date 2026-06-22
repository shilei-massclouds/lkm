use core::sync::atomic::{AtomicUsize, Ordering};

use super::{
    rest_init::KernelInitTask,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};

pub const STDIN_FD: usize = 0;
pub const STDOUT_FD: usize = 1;
pub const STDERR_FD: usize = 2;
const STDIO_FD_COUNT: usize = 3;

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
}

impl FdRef {
    const fn index(self) -> usize {
        match self {
            Self::Stdin => 0,
            Self::Stdout => 1,
            Self::Stderr => 2,
        }
    }

    pub const fn from_fd(fd: usize) -> Option<Self> {
        match fd {
            STDIN_FD => Some(Self::Stdin),
            STDOUT_FD => Some(Self::Stdout),
            STDERR_FD => Some(Self::Stderr),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FileError {
    NotReady,
    BadFd,
    NotWritable,
    BackendUnavailable,
}

pub type FileResult<T> = Result<T, FileError>;

#[derive(Clone, Copy)]
struct FileDescriptorEntry {
    ofd: OpenFileDescriptionRef,
    writable: bool,
}

impl FileDescriptorEntry {
    const fn stdio(ofd: OpenFileDescriptionRef, writable: bool) -> Self {
        Self { ofd, writable }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OpenFileDescriptionRef {
    Stdin,
    Stdout,
    Stderr,
}

pub struct FileBackend {
    lifecycle: Lifecycle,
    kind: FileBackendKind,
    allocated: bool,
    char_device_console_bound: bool,
    char_device_write_supported: bool,
    regular_file_deferred: bool,
    block_device_deferred: bool,
    write_to_console: AtomicUsize,
    last_write_len: AtomicUsize,
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
            regular_file_deferred: false,
            block_device_deferred: false,
            write_to_console: AtomicUsize::new(0),
            last_write_len: AtomicUsize::new(0),
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
}

pub struct OpenFileDescription {
    lifecycle: Lifecycle,
    allocated: bool,
    backend_bound: bool,
    flags_bound: bool,
    writable: bool,
    offset_ready: bool,
    write_dispatches_backend: AtomicUsize,
    write_observed: AtomicUsize,
    last_write_len: AtomicUsize,
}

#[allow(dead_code)]
impl OpenFileDescription {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            backend_bound: false,
            flags_bound: false,
            writable: false,
            offset_ready: false,
            write_dispatches_backend: AtomicUsize::new(0),
            write_observed: AtomicUsize::new(0),
            last_write_len: AtomicUsize::new(0),
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

    pub const fn writable(&self) -> bool {
        self.writable
    }

    pub const fn offset_ready(&self) -> bool {
        self.offset_ready
    }

    pub fn write_dispatches_backend(&self) -> bool {
        self.write_dispatches_backend.load(Ordering::Acquire) != 0
    }

    pub fn write_observed(&self) -> bool {
        self.write_observed.load(Ordering::Acquire) != 0
    }

    pub fn last_write_len(&self) -> usize {
        self.last_write_len.load(Ordering::Acquire)
    }

    fn setup(&mut self, backend: &FileBackend, writable: bool) -> EventResult {
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
        self.writable = writable;
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
}

pub struct FileDescriptorTable {
    lifecycle: Lifecycle,
    allocated: bool,
    capacity_bound: bool,
    stdio_fds_bound: bool,
    entries: [Option<FileDescriptorEntry>; STDIO_FD_COUNT],
    lookup_returns: AtomicUsize,
}

#[allow(dead_code)]
impl FileDescriptorTable {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            capacity_bound: false,
            stdio_fds_bound: false,
            entries: [None; STDIO_FD_COUNT],
            lookup_returns: AtomicUsize::new(0),
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
            false,
        ));
        self.entries[FdRef::Stdout.index()] = Some(FileDescriptorEntry::stdio(
            OpenFileDescriptionRef::Stdout,
            true,
        ));
        self.entries[FdRef::Stderr.index()] = Some(FileDescriptorEntry::stdio(
            OpenFileDescriptionRef::Stderr,
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
}

pub struct FilesStruct {
    lifecycle: Lifecycle,
    fd_table: FileDescriptorTable,
    stdin: OpenFileDescription,
    stdout: OpenFileDescription,
    stderr: OpenFileDescription,
    stdin_backend: FileBackend,
    stdout_backend: FileBackend,
    stderr_backend: FileBackend,
    allocated: bool,
    owned_by_kernel_init_task: bool,
    fd_table_bound: bool,
    stdio_bound: bool,
    next_fd_ready: bool,
    close_on_exec_ready: bool,
    shared_deferred: bool,
    fd_lookup_routes_to_table: AtomicUsize,
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
            stdin_backend: FileBackend::new(FileBackendKind::CharDevice),
            stdout_backend: FileBackend::new(FileBackendKind::CharDevice),
            stderr_backend: FileBackend::new(FileBackendKind::CharDevice),
            allocated: false,
            owned_by_kernel_init_task: false,
            fd_table_bound: false,
            stdio_bound: false,
            next_fd_ready: false,
            close_on_exec_ready: false,
            shared_deferred: false,
            fd_lookup_routes_to_table: AtomicUsize::new(0),
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

    pub const fn stdin_backend(&self) -> &FileBackend {
        &self.stdin_backend
    }

    pub const fn stdout_backend(&self) -> &FileBackend {
        &self.stdout_backend
    }

    pub const fn stderr_backend(&self) -> &FileBackend {
        &self.stderr_backend
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
        self.stdin.setup(&self.stdin_backend, false)?;
        self.stdout.setup(&self.stdout_backend, true)?;
        self.stderr.setup(&self.stderr_backend, true)?;
        self.fd_table
            .install_stdio(&self.stdin, &self.stdout, &self.stderr)?;

        self.allocated = true;
        self.owned_by_kernel_init_task = true;
        self.fd_table_bound = true;
        self.stdio_bound = true;
        self.next_fd_ready = true;
        self.close_on_exec_ready = true;
        self.shared_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
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
        }
    }
}
