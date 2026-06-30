/*
 * Task-owned open-file context model.
 *
 * FsStruct carries root/pwd path context. FilesStruct is its sibling object for
 * opened files: it belongs to the KernelInitTask execution line, survives the
 * exec/user-init handoff, and owns a FileDescriptorTable. The fd table maps fd
 * numbers to OpenFileDescription instances. OpenFileDescription represents the
 * Linux-like opened file instance, with flags, offset and a FileBackend.
 *
 * The first slice only preinstalls fd 0/1/2 and binds them to a character
 * device backend for console-like stdio. The next read-only slice allocates one
 * regular-file opened instance from a VFS path and lets syscall openat/read/
 * close/newfstatat consume that instance. Block-device and richer character-
 * device backends remain deferred. Concrete syscall write therefore goes:
 *
 *   SyscallTable.Action::Write
 *     -> FilesStruct.Action::LookupFd
 *     -> FileDescriptorTable.Action::Lookup
 *     -> OpenFileDescription.Action::Write
 *     -> FileBackend.Action::WriteCharDevice
 *
 * Read-only regular-file syscall path goes:
 *
 *   SyscallTable.Action::OpenAt
 *     -> FilesStruct.Action::OpenPath
 *     -> VfsCore.Action::ReadPath(Path, FsStruct)
 *     -> FileBackend.Action::BindRegularFile
 *     -> OpenFileDescription.Transition::Setup
 *     -> FileDescriptorTable.Action::Install(FdRef::Regular0)
 *   SyscallTable.Action::Read
 *     -> FilesStruct.Action::ReadFd(FdRef::Regular0)
 *     -> OpenFileDescription.Action::Read
 *     -> FileBackend.Action::ReadRegularFile
 *   SyscallTable.Action::Close
 *     -> FileDescriptorTable.Action::Close(FdRef::Regular0)
 *   SyscallTable.Action::NewFstatAt
 *     -> FilesStruct.Action::StatPath
 *     -> VfsCore.Action::ReadPath(Path, FsStruct)
 *   SyscallTable.Action::ReadlinkAt
 *     -> FilesStruct.Action::ReadlinkPath
 *     -> VfsCore.Action::ReadlinkPath(Path, FsStruct)
 */

enum FileBackendKind {
    CharDevice,
    RegularFile,
    BlockDevice,
}

enum FdRef {
    Stdin,
    Stdout,
    Stderr,
    Regular0,
}

predicate files_struct_allocated<T>(files: T) -> bool;
predicate files_struct_owned_by_kernel_init_task<T, K>(files: T, task: K) -> bool;
predicate files_struct_inherited_by_user_init<T, U>(files: T, process: U) -> bool;
predicate files_struct_fd_table_bound<T, F>(files: T, table: F) -> bool;
predicate files_struct_stdio_bound<T>(files: T) -> bool;
predicate files_struct_next_fd_ready<T>(files: T) -> bool;
predicate files_struct_close_on_exec_ready<T>(files: T) -> bool;
predicate files_struct_shared_deferred<T>(files: T) -> bool;
predicate files_struct_fd_lookup_routes_to_table<T, F>(files: T, table: F) -> bool;
predicate files_struct_regular_file_slot_ready<T>(files: T) -> bool;
predicate files_struct_open_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_regular_fd_installed<T>(files: T) -> bool;
predicate files_struct_read_fd_routes_to_table<T, F>(files: T, table: F) -> bool;
predicate files_struct_close_fd_routes_to_table<T, F>(files: T, table: F) -> bool;
predicate files_struct_stat_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_readlink_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_regular_file_read_observed<T>(files: T) -> bool;
predicate files_struct_regular_file_closed<T>(files: T) -> bool;
predicate files_struct_regular_file_stat_observed<T>(files: T) -> bool;

predicate fd_table_allocated<T>(table: T) -> bool;
predicate fd_table_capacity_bound<T>(table: T) -> bool;
predicate fd_table_fd_bound<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_lookup_returns<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_stdio_fds_bound<T>(table: T) -> bool;
predicate fd_table_fd_installed<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_cloexec_bit_set_on_install<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_cloexec_not_reported_by_fgetfl<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_cloexec_bit_returned_by_fgetfd<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_cloexec_bit_updated_by_fsetfd<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_fd_closed<T>(table: T, fd: FdRef) -> bool;

predicate open_file_description_allocated<T>(ofd: T) -> bool;
predicate open_file_description_backend_bound<T, B>(ofd: T, backend: B) -> bool;
predicate open_file_description_flags_bound<T>(ofd: T) -> bool;
predicate open_file_description_offset_ready<T>(ofd: T) -> bool;
predicate open_file_description_write_dispatches_backend<T, B>(ofd: T, backend: B) -> bool;
predicate open_file_description_write_observed<T>(ofd: T) -> bool;
predicate open_file_description_readable<T>(ofd: T) -> bool;
predicate open_file_description_read_dispatches_backend<T, B>(ofd: T, backend: B) -> bool;
predicate open_file_description_read_observed<T>(ofd: T) -> bool;

predicate file_backend_allocated<T>(backend: T) -> bool;
predicate file_backend_kind_bound<T>(backend: T, kind: FileBackendKind) -> bool;
predicate file_backend_char_device_console_bound<T>(backend: T) -> bool;
predicate file_backend_regular_file_deferred<T>(backend: T) -> bool;
predicate file_backend_block_device_deferred<T>(backend: T) -> bool;
predicate file_backend_char_device_write_supported<T>(backend: T) -> bool;
predicate file_backend_write_to_console<T>(backend: T) -> bool;
predicate file_backend_regular_file_bound<T>(backend: T) -> bool;
predicate file_backend_regular_file_read_supported<T>(backend: T) -> bool;
predicate file_backend_regular_file_stat_supported<T>(backend: T) -> bool;
predicate file_backend_regular_file_read_returns_data<T>(backend: T) -> bool;
predicate file_backend_regular_file_stat_returns_metadata<T>(backend: T) -> bool;

object FilesStruct: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    ConsoleRegistry.state == State::Ready;
                }

                drives {
                    FileDescriptorTable.Transition::Setup;
                    FileBackend.Transition::Setup;
                    OpenFileDescription.Transition::Setup;
                    FileDescriptorTable.Action::InstallStdio;
                }

                ensures {
                    files_struct_allocated(self);
                    files_struct_owned_by_kernel_init_task(self, KernelInitTask);
                    files_struct_fd_table_bound(self, FileDescriptorTable);
                    files_struct_stdio_bound(self);
                    files_struct_next_fd_ready(self);
                    files_struct_close_on_exec_ready(self);
                    files_struct_shared_deferred(self);
                    files_struct_regular_file_slot_ready(self);
                    fd_table_stdio_fds_bound(FileDescriptorTable);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            files_struct_allocated(self);
            files_struct_owned_by_kernel_init_task(self, KernelInitTask);
            files_struct_fd_table_bound(self, FileDescriptorTable);
            files_struct_stdio_bound(self);
            files_struct_next_fd_ready(self);
            files_struct_close_on_exec_ready(self);
            files_struct_regular_file_slot_ready(self);
        }

        actions {
            on Action::LookupFd(fd: FdRef) {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    files_struct_fd_table_bound(self, FileDescriptorTable);
                }

                drives {
                    FileDescriptorTable.Action::Lookup(fd);
                }

                ensures {
                    files_struct_fd_lookup_routes_to_table(self, FileDescriptorTable);
                    fd_table_lookup_returns(FileDescriptorTable, fd, OpenFileDescription);
                }
            }

            on Action::OpenPath {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    files_struct_regular_file_slot_ready(self);
                }

                drives {
                    VfsCore.Action::ReadPath(Path, FsStruct);
                    FileBackend.Action::BindRegularFile;
                    OpenFileDescription.Transition::Setup;
                    FileDescriptorTable.Action::Install(FdRef::Regular0);
                }

                ensures {
                    files_struct_open_path_routes_to_vfs(self, VfsCore);
                    files_struct_regular_fd_installed(self);
                    file_backend_regular_file_bound(FileBackend);
                    file_backend_regular_file_read_supported(FileBackend);
                    file_backend_regular_file_stat_supported(FileBackend);
                    fd_table_fd_installed(FileDescriptorTable, FdRef::Regular0, OpenFileDescription);
                }
            }

            on Action::ReadFd(fd: FdRef) {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, fd, OpenFileDescription);
                    open_file_description_readable(OpenFileDescription);
                }

                drives {
                    FileDescriptorTable.Action::Lookup(fd);
                    OpenFileDescription.Action::Read;
                    FileBackend.Action::ReadRegularFile;
                }

                ensures {
                    files_struct_read_fd_routes_to_table(self, FileDescriptorTable);
                    files_struct_regular_file_read_observed(self);
                    open_file_description_read_observed(OpenFileDescription);
                    file_backend_regular_file_read_returns_data(FileBackend);
                }
            }

            on Action::CloseFd(fd: FdRef) {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, fd, OpenFileDescription);
                }

                drives {
                    FileDescriptorTable.Action::Close(fd);
                }

                ensures {
                    files_struct_close_fd_routes_to_table(self, FileDescriptorTable);
                    files_struct_regular_file_closed(self);
                    fd_table_fd_closed(FileDescriptorTable, fd);
                }
            }

            on Action::GetFdFlags(fd: FdRef) {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, fd, OpenFileDescription);
                }

                drives {
                    FileDescriptorTable.Action::GetFdFlags(fd);
                }

                ensures {
                    files_struct_fd_lookup_routes_to_table(self, FileDescriptorTable);
                    fd_table_cloexec_bit_returned_by_fgetfd(FileDescriptorTable, fd);
                }
            }

            on Action::SetFdFlags(fd: FdRef) {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, fd, OpenFileDescription);
                }

                drives {
                    FileDescriptorTable.Action::SetFdFlags(fd);
                }

                ensures {
                    files_struct_fd_lookup_routes_to_table(self, FileDescriptorTable);
                    fd_table_cloexec_bit_updated_by_fsetfd(FileDescriptorTable, fd);
                }
            }

            on Action::StatPath {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                }

                drives {
                    VfsCore.Action::ReadPath(Path, FsStruct);
                    FileBackend.Action::StatRegularFile;
                }

                ensures {
                    files_struct_stat_path_routes_to_vfs(self, VfsCore);
                    files_struct_regular_file_stat_observed(self);
                    file_backend_regular_file_stat_returns_metadata(FileBackend);
                }
            }

            on Action::ReadlinkPath {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                }

                drives {
                    VfsCore.Action::ReadlinkPath(Path, FsStruct);
                }

                ensures {
                    files_struct_readlink_path_routes_to_vfs(self, VfsCore);
                }
            }
        }
    }
}

object FileDescriptorTable: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    fd_table_allocated(self);
                    fd_table_capacity_bound(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            fd_table_allocated(self);
            fd_table_capacity_bound(self);
        }

        actions {
            on Action::InstallStdio {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    OpenFileDescription.state == State::Ready;
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(FileBackend, FileBackendKind::CharDevice);
                    file_backend_char_device_console_bound(FileBackend);
                }

                ensures {
                    fd_table_fd_bound(self, FdRef::Stdin, OpenFileDescription);
                    fd_table_fd_bound(self, FdRef::Stdout, OpenFileDescription);
                    fd_table_fd_bound(self, FdRef::Stderr, OpenFileDescription);
                    fd_table_stdio_fds_bound(self);
                }
            }

            on Action::Lookup(fd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(self, fd, OpenFileDescription);
                }

                ensures {
                    fd_table_lookup_returns(self, fd, OpenFileDescription);
                }
            }

            on Action::GetFdFlags(fd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(self, fd, OpenFileDescription);
                }

                ensures {
                    fd_table_cloexec_bit_returned_by_fgetfd(self, fd);
                }
            }

            on Action::SetFdFlags(fd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(self, fd, OpenFileDescription);
                }

                ensures {
                    fd_table_cloexec_bit_updated_by_fsetfd(self, fd);
                }
            }

            on Action::Install(fd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    OpenFileDescription.state == State::Ready;
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(FileBackend, FileBackendKind::RegularFile);
                }

                ensures {
                    fd_table_fd_installed(self, fd, OpenFileDescription);
                    fd_table_fd_bound(self, fd, OpenFileDescription);
                    fd_table_cloexec_bit_set_on_install(self, fd);
                    fd_table_cloexec_not_reported_by_fgetfl(self, fd);
                }
            }

            on Action::Close(fd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(self, fd, OpenFileDescription);
                }

                ensures {
                    fd_table_fd_closed(self, fd);
                }
            }
        }
    }
}

object OpenFileDescription: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    FileBackend.state == State::Ready;
                }

                ensures {
                    open_file_description_allocated(self);
                    open_file_description_backend_bound(self, FileBackend);
                    open_file_description_flags_bound(self);
                    open_file_description_offset_ready(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            open_file_description_allocated(self);
            open_file_description_backend_bound(self, FileBackend);
            open_file_description_flags_bound(self);
            open_file_description_offset_ready(self);
        }

        actions {
            on Action::Write {
                depends_on {
                    OpenFileDescription.state == State::Ready;
                    FileBackend.state == State::Ready;
                    open_file_description_backend_bound(self, FileBackend);
                    file_backend_kind_bound(FileBackend, FileBackendKind::CharDevice);
                    file_backend_char_device_write_supported(FileBackend);
                }

                drives {
                    FileBackend.Action::WriteCharDevice;
                }

                ensures {
                    open_file_description_write_dispatches_backend(self, FileBackend);
                    open_file_description_write_observed(self);
                }
            }

            on Action::Read {
                depends_on {
                    OpenFileDescription.state == State::Ready;
                    FileBackend.state == State::Ready;
                    open_file_description_backend_bound(self, FileBackend);
                    open_file_description_readable(self);
                    file_backend_kind_bound(FileBackend, FileBackendKind::RegularFile);
                    file_backend_regular_file_read_supported(FileBackend);
                }

                drives {
                    FileBackend.Action::ReadRegularFile;
                }

                ensures {
                    open_file_description_read_dispatches_backend(self, FileBackend);
                    open_file_description_read_observed(self);
                    file_backend_regular_file_read_returns_data(FileBackend);
                }
            }
        }
    }
}

object FileBackend: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                }

                ensures {
                    file_backend_allocated(self);
                    file_backend_kind_bound(self, FileBackendKind::CharDevice);
                    file_backend_char_device_console_bound(self);
                    file_backend_char_device_write_supported(self);
                    file_backend_regular_file_deferred(self);
                    file_backend_block_device_deferred(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            file_backend_allocated(self);
            file_backend_kind_bound(self, FileBackendKind::CharDevice);
            file_backend_char_device_console_bound(self);
            file_backend_char_device_write_supported(self);
        }

        actions {
            on Action::WriteCharDevice {
                depends_on {
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(self, FileBackendKind::CharDevice);
                    file_backend_char_device_write_supported(self);
                }

                ensures {
                    file_backend_write_to_console(self);
                }
            }

            on Action::BindRegularFile {
                depends_on {
                    VfsCore.state == State::Ready;
                }

                ensures {
                    file_backend_allocated(self);
                    file_backend_kind_bound(self, FileBackendKind::RegularFile);
                    file_backend_regular_file_bound(self);
                    file_backend_regular_file_read_supported(self);
                    file_backend_regular_file_stat_supported(self);
                }
            }

            on Action::ReadRegularFile {
                depends_on {
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(self, FileBackendKind::RegularFile);
                    file_backend_regular_file_read_supported(self);
                }

                ensures {
                    file_backend_regular_file_read_returns_data(self);
                }
            }

            on Action::StatRegularFile {
                depends_on {
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(self, FileBackendKind::RegularFile);
                    file_backend_regular_file_stat_supported(self);
                }

                ensures {
                    file_backend_regular_file_stat_returns_metadata(self);
                }
            }
        }
    }
}
