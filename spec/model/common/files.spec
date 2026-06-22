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
 * device backend for console-like stdio. Regular-file, block-device and richer
 * character-device backends are modeled as backend variants but deferred until
 * open/read paths need them. Concrete syscall write therefore goes:
 *
 *   SyscallTable.Action::Write
 *     -> FilesStruct.Action::LookupFd
 *     -> FileDescriptorTable.Action::Lookup
 *     -> OpenFileDescription.Action::Write
 *     -> FileBackend.Action::WriteCharDevice
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

predicate fd_table_allocated<T>(table: T) -> bool;
predicate fd_table_capacity_bound<T>(table: T) -> bool;
predicate fd_table_fd_bound<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_lookup_returns<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_stdio_fds_bound<T>(table: T) -> bool;

predicate open_file_description_allocated<T>(ofd: T) -> bool;
predicate open_file_description_backend_bound<T, B>(ofd: T, backend: B) -> bool;
predicate open_file_description_flags_bound<T>(ofd: T) -> bool;
predicate open_file_description_offset_ready<T>(ofd: T) -> bool;
predicate open_file_description_write_dispatches_backend<T, B>(ofd: T, backend: B) -> bool;
predicate open_file_description_write_observed<T>(ofd: T) -> bool;

predicate file_backend_allocated<T>(backend: T) -> bool;
predicate file_backend_kind_bound<T>(backend: T, kind: FileBackendKind) -> bool;
predicate file_backend_char_device_console_bound<T>(backend: T) -> bool;
predicate file_backend_regular_file_deferred<T>(backend: T) -> bool;
predicate file_backend_block_device_deferred<T>(backend: T) -> bool;
predicate file_backend_char_device_write_supported<T>(backend: T) -> bool;
predicate file_backend_write_to_console<T>(backend: T) -> bool;

object FilesStruct: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    ConsoleRegistry.state == State::Ready;
                }

                drives {
                    FileDescriptorTable.Event::Setup;
                    FileBackend.Event::Setup;
                    OpenFileDescription.Event::Setup;
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
        }
    }
}

object FileDescriptorTable: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
        }
    }
}

object OpenFileDescription: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
        }
    }
}

object FileBackend: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
        }
    }
}
