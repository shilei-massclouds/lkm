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
 *   SyscallTable.Action::Dup3
 *     -> FilesStruct.Action::Dup3Fd
 *     -> FileDescriptorTable.Action::Dup3
 *   SyscallTable.Action::NewFstatAt
 *     -> FilesStruct.Action::StatPath
 *     -> VfsCore.Action::ReadPath(Path, FsStruct)
 *   SyscallTable.Action::ReadlinkAt
 *     -> FilesStruct.Action::ReadlinkPath
 *     -> VfsCore.Action::ReadlinkPath(Path, FsStruct)
 *
 * Character-device stdin/readiness path goes:
 *
 *   SyscallTable.Action::Read / Ppoll
 *     -> FilesStruct.Action::ReadFd / LookupFd
 *     -> FileDescriptorTable.Action::Lookup
 *     -> OpenFileDescription.Action::ReadCharDevice
 *     -> FileBackend.Action::ReadCharDevice
 *     -> NTtyLineDiscipline.Action::ReadLineOrBytes / EvaluateReadiness
 *     -> TtyFlipBuffer bounded ready-data
 *
 * The current TTY open slice treats /dev/tty and /dev/tty[0-9]+ as aliases for
 * the same console-like Tty0 FileBackend::CharDevice. Opening an alias installs
 * a new fd entry in the first free fixed-capacity fd table slot. With stdio
 * still installed this means the existing 3 through 15 range; after a Linux
 * close(0/1/2) the vacated stdio slot may be reused by the next TTY open.
 * Each fd entry keeps independent status flags and close-on-exec state, while
 * all entries share the same staged backend. This is only for staged
 * BusyBox/OpenRC probing; it is not devtmpfs, VT allocation, /dev/console,
 * major/minor lookup or a multiple-TTY driver registry.
 *
 * Plain fork/vfork child execution currently reuses the same runtime
 * FilesStruct object. To preserve Linux's user-visible rule that a child's
 * execve close-on-exec pass does not close the parent's fd entries, the single
 * active child slice saves a bounded parent fd table and regular-slot metadata
 * snapshot at clone time and restores it when the child returns to the parent.
 * This is a local rollback for the observed child continuation, not full
 * copy_files(), CLONE_FILES, files_struct refcounting, fdtable expansion or
 * OFD lifetime management.
 *
 * dup3(2) first slice follows local Linux 6.12 ksys_dup3()/do_dup2() only at
 * fixed fd-table entry granularity: oldfd and newfd must differ, flags may be
 * only 0 or O_CLOEXEC, newfd must fit the current fixed table, and success
 * makes newfd point at the same staged OpenFileDescription/backend entry as
 * oldfd. Replacing an already-open newfd only overwrites the fd table entry;
 * full filp_close/fput/refcount/lock/EBUSY/rlimit/expand_files semantics stay
 * deferred.
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
    Null,
    Pidfd0,
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
predicate files_struct_null_fd_installed<T>(files: T) -> bool;
predicate files_struct_null_device_read_eof_observed<T>(files: T) -> bool;
predicate files_struct_null_device_write_discard_observed<T>(files: T) -> bool;
predicate files_struct_null_device_fstat_device_node<T>(files: T) -> bool;
predicate files_struct_null_device_tty_ioctl_enotty<T>(files: T) -> bool;
predicate files_struct_tty_alias_fd_installed<T>(files: T) -> bool;
predicate files_struct_tty_alias_entries_share_backend<T>(files: T) -> bool;
predicate files_struct_read_fd_routes_to_table<T, F>(files: T, table: F) -> bool;
predicate files_struct_close_fd_routes_to_table<T, F>(files: T, table: F) -> bool;
predicate files_struct_dup3_routes_to_table<T, F>(files: T, table: F) -> bool;
predicate files_struct_stdio_fd_close_supported<T>(files: T) -> bool;
predicate files_struct_close_on_exec_observed<T>(files: T) -> bool;
predicate files_struct_parent_fd_snapshot_saved<T, C>(files: T, child: C) -> bool;
predicate files_struct_parent_fd_snapshot_restored<T, C>(files: T, child: C) -> bool;
predicate files_struct_stat_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_readlink_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_regular_file_read_observed<T>(files: T) -> bool;
predicate files_struct_regular_file_closed<T>(files: T) -> bool;
predicate files_struct_regular_file_stat_observed<T>(files: T) -> bool;
predicate files_struct_pidfd_installed<T>(files: T) -> bool;
predicate files_struct_pidfd_child_bound<T, C>(files: T, child: C) -> bool;
predicate files_struct_pidfd_readable_after_exit<T, C>(files: T, child: C) -> bool;
predicate user_pidfd_ready<T, C>(files: T, child: C) -> bool;
predicate files_struct_stdin_probe_ready_data_cleared<T>(files: T) -> bool;
predicate files_struct_user_smoke_stdin_fixture_bound<T>(files: T) -> bool;
predicate files_struct_stdin_blocking_wait_enabled<T>(files: T) -> bool;
predicate files_struct_stdin_blocking_wait_fixture_opt_out<T>(files: T) -> bool;

predicate fd_table_allocated<T>(table: T) -> bool;
predicate fd_table_capacity_bound<T>(table: T) -> bool;
predicate fd_table_fd_bound<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_lookup_returns<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_stdio_fds_bound<T>(table: T) -> bool;
predicate fd_table_fd_installed<T, O>(table: T, fd: FdRef, ofd: O) -> bool;
predicate fd_table_first_free_user_fd_installed<T, O>(table: T, ofd: O) -> bool;
predicate fd_table_fd_entries_have_independent_status_flags<T>(table: T) -> bool;
predicate fd_table_cloexec_bit_set_on_install<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_cloexec_not_reported_by_fgetfl<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_cloexec_bit_returned_by_fgetfd<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_cloexec_bit_updated_by_fsetfd<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_fd_closed<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_stdio_fd_closed<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_fd_duplicated<T>(table: T, oldfd: FdRef, newfd: FdRef) -> bool;
predicate fd_table_dup3_close_on_exec_bound<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_close_on_exec_scanned<T>(table: T) -> bool;
predicate fd_table_close_on_exec_closed<T>(table: T) -> bool;
predicate fd_table_parent_snapshot_saved<T>(table: T) -> bool;
predicate fd_table_parent_snapshot_restored<T>(table: T) -> bool;
predicate fd_table_pidfd_entry_installed<T>(table: T, fd: FdRef) -> bool;
predicate fd_table_pidfd_entry_closed<T>(table: T, fd: FdRef) -> bool;

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
predicate file_backend_char_device_null_bound<T>(backend: T) -> bool;
predicate file_backend_regular_file_deferred<T>(backend: T) -> bool;
predicate file_backend_block_device_deferred<T>(backend: T) -> bool;
predicate file_backend_char_device_write_supported<T>(backend: T) -> bool;
predicate file_backend_char_device_read_supported<T>(backend: T) -> bool;
predicate file_backend_write_to_console<T>(backend: T) -> bool;
predicate file_backend_null_device_read_returns_eof<T>(backend: T) -> bool;
predicate file_backend_null_device_write_discards_data<T>(backend: T) -> bool;
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

            on Action::OpenTtyAliasPath {
                /*
                 * /dev/tty and /dev/tty[0-9]+ are staged aliases for the
                 * existing console-like Tty0 backend. This action allocates
                 * only a fd-table entry: it does not create a real virtual
                 * terminal, a devtmpfs inode, a /dev/console object, or a new
                 * TTY driver instance. The fd table chooses the first free
                 * user fd slot in its fixed 3..15 range; no free slot is the
                 * EMFILE boundary. Each fd entry owns its status flags, so
                 * F_SETFL on one alias fd must not mutate another alias fd.
                 */
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(FileBackend, FileBackendKind::CharDevice);
                    file_backend_char_device_console_bound(FileBackend);
                }

                drives {
                    FileDescriptorTable.Action::InstallFirstFreeCharDevice;
                }

                ensures {
                    files_struct_tty_alias_fd_installed(self);
                    files_struct_tty_alias_entries_share_backend(self);
                    fd_table_first_free_user_fd_installed(FileDescriptorTable, OpenFileDescription);
                    fd_table_fd_entries_have_independent_status_flags(FileDescriptorTable);
                }
            }

            on Action::OpenNullPath {
                /*
                 * /dev/null is a staged built-in character-device alias for
                 * OpenRC/getty stdio redirection. It allocates only an fd
                 * table entry pointing at the Null OFD/backend, preserves
                 * status flags and close-on-exec, and does not create devtmpfs,
                 * device-number, permission, LSM or generic char-device
                 * registry state. O_DIRECTORY is rejected at the directory
                 * target boundary instead of falling through to ordinary
                 * filesystem permission denial.
                 */
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(FileBackend, FileBackendKind::CharDevice);
                    file_backend_char_device_null_bound(FileBackend);
                }

                drives {
                    FileDescriptorTable.Action::InstallFirstFreeCharDevice;
                }

                ensures {
                    files_struct_null_fd_installed(self);
                    fd_table_first_free_user_fd_installed(FileDescriptorTable, OpenFileDescription);
                    fd_table_fd_entries_have_independent_status_flags(FileDescriptorTable);
                }
            }

            on Action::ReadNullFd {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, FdRef::Null, OpenFileDescription);
                    open_file_description_readable(OpenFileDescription);
                    file_backend_char_device_null_bound(FileBackend);
                }

                drives {
                    FileDescriptorTable.Action::Lookup(FdRef::Null);
                    FileBackend.Action::ReadNullDevice;
                }

                ensures {
                    files_struct_read_fd_routes_to_table(self, FileDescriptorTable);
                    files_struct_null_device_read_eof_observed(self);
                    file_backend_null_device_read_returns_eof(FileBackend);
                }
            }

            on Action::WriteNullFd {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, FdRef::Null, OpenFileDescription);
                    file_backend_char_device_null_bound(FileBackend);
                }

                drives {
                    FileDescriptorTable.Action::Lookup(FdRef::Null);
                    FileBackend.Action::WriteNullDevice;
                }

                ensures {
                    files_struct_fd_lookup_routes_to_table(self, FileDescriptorTable);
                    files_struct_null_device_write_discard_observed(self);
                    file_backend_null_device_write_discards_data(FileBackend);
                }
            }

            on Action::InstallPidfd {
                /*
                 * Linux 6.12 CLONE_PIDFD allocates a pidfd in the parent's fd
                 * table and writes that fd to legacy clone parent_tidptr.
                 * This first slice installs one pidfd-like fd table entry for
                 * the single UserChildProcess slot. It supports fd visibility,
                 * close, F_GETFD/F_SETFD through the common fd flag path, and
                 * immediate ppoll readability after the child has exited.
                 * Full pidfs file operations, pidfd_send_signal, pidfd_getfd,
                 * waitid(P_PIDFD), refcounting and waitqueue poll remain
                 * deferred.
                 */
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    UserChildProcess.state == State::Ready;
                }

                drives {
                    FileDescriptorTable.Action::Install(FdRef::Pidfd0);
                }

                ensures {
                    files_struct_pidfd_installed(self);
                    files_struct_pidfd_child_bound(self, UserChildProcess);
                    fd_table_pidfd_entry_installed(FileDescriptorTable, FdRef::Pidfd0);
                    fd_table_cloexec_bit_set_on_install(FileDescriptorTable, FdRef::Pidfd0);
                }
            }

            on Action::MarkPidfdReady {
                depends_on {
                    FilesStruct.state == State::Ready;
                    UserChildProcess.state == State::Ready;
                    files_struct_pidfd_child_bound(self, UserChildProcess);
                }

                ensures {
                    files_struct_pidfd_readable_after_exit(self, UserChildProcess);
                    user_pidfd_ready(self, UserChildProcess);
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
                    OpenFileDescription.Action::ReadCharDevice;
                    FileBackend.Action::ReadRegularFile;
                    FileBackend.Action::ReadCharDevice;
                    NTtyLineDiscipline.Action::ReadLineOrBytes;
                }

                ensures {
                    files_struct_read_fd_routes_to_table(self, FileDescriptorTable);
                    files_struct_regular_file_read_observed(self);
                    files_struct_stdin_char_device_read_observed(self);
                    open_file_description_read_observed(OpenFileDescription);
                    file_backend_regular_file_read_returns_data(FileBackend);
                    file_backend_char_device_read_returns_ready_data(FileBackend);
                    n_tty_canonical_read_returns_through_newline(NTtyLineDiscipline);
                    n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
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
                    files_struct_stdio_fd_close_supported(self);
                    fd_table_fd_closed(FileDescriptorTable, fd);
                }
            }

            on Action::Dup3Fd(oldfd: FdRef, newfd: FdRef) {
                /*
                 * Linux 6.12 ksys_dup3()/do_dup2() performs a targeted
                 * descriptor-table replacement, not a lowest-free-slot
                 * allocation. The current model captures that fd-entry
                 * replacement and close-on-exec bit update, while trimming
                 * expand_files(), rlimit, EBUSY larval-fd detection,
                 * get_file()/filp_close()/fput() and concurrent locking.
                 */
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, oldfd, OpenFileDescription);
                }

                drives {
                    FileDescriptorTable.Action::Dup3(oldfd, newfd);
                }

                ensures {
                    files_struct_dup3_routes_to_table(self, FileDescriptorTable);
                    fd_table_fd_duplicated(FileDescriptorTable, oldfd, newfd);
                    fd_table_dup3_close_on_exec_bound(FileDescriptorTable, newfd);
                }
            }

            on Action::CloseOnExec {
                /*
                 * Runtime execve success follows Linux do_close_on_exec() only
                 * far enough for the current fixed fd table: scan every slot,
                 * clear entries whose fd close-on-exec bit is set, and retain
                 * diagnostic scanned/closed/remaining facts. If the runtime is
                 * executing the single child continuation, the parent fd table
                 * must have a saved snapshot so the child's close-on-exec pass
                 * can be rolled back before parent resume. Full files unshare,
                 * file refcounts, locking and delayed fput remain deferred.
                 */
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FileDescriptorTable.Action::CloseOnExec;
                }

                ensures {
                    files_struct_close_on_exec_observed(self);
                    fd_table_close_on_exec_scanned(FileDescriptorTable);
                    fd_table_close_on_exec_closed(FileDescriptorTable);
                }
            }

            on Action::SaveParentFdSnapshot {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    UserChildProcess.state == State::Prepared;
                }

                drives {
                    FileDescriptorTable.Action::SaveParentSnapshot;
                }

                ensures {
                    files_struct_parent_fd_snapshot_saved(self, UserChildProcess);
                    fd_table_parent_snapshot_saved(FileDescriptorTable);
                }
            }

            on Action::RestoreParentFdSnapshot {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    UserChildProcess.state == State::Ready;
                    files_struct_parent_fd_snapshot_saved(self, UserChildProcess);
                }

                drives {
                    FileDescriptorTable.Action::RestoreParentSnapshot;
                }

                ensures {
                    files_struct_parent_fd_snapshot_restored(self, UserChildProcess);
                    fd_table_parent_snapshot_restored(FileDescriptorTable);
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

            on Action::ClearStdinReadyData {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    files_struct_fd_table_bound(self, FileDescriptorTable);
                    files_struct_stdio_bound(self);
                    TtyFlipBuffer.state == State::Ready;
                }

                drives {
                    TtyFlipBuffer.Action::ClearReadyData;
                }

                ensures {
                    files_struct_stdin_probe_ready_data_cleared(self);
                    tty_flip_buffer_ready_data_cleared(TtyFlipBuffer);
                    tty_flip_buffer_probe_bytes_not_user_stdin(TtyFlipBuffer);
                }
            }

            on Action::PrepareDefaultStdinReadyData {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    files_struct_fd_table_bound(self, FileDescriptorTable);
                    files_struct_stdio_bound(self);
                    files_struct_stdin_probe_ready_data_cleared(self);
                    TtyFlipBuffer.state == State::Ready;
                }

                drives {
                    TtyFlipBuffer.Action::SeedUserSmokeReadyDataFixture;
                }

                ensures {
                    files_struct_user_smoke_stdin_fixture_bound(self);
                    files_struct_stdin_blocking_wait_fixture_opt_out(self);
                    tty_flip_buffer_user_smoke_fixture_ready_data_bound(TtyFlipBuffer);
                }
            }

            on Action::EnableStdinBlockingWait {
                depends_on {
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    files_struct_fd_table_bound(self, FileDescriptorTable);
                    files_struct_stdio_bound(self);
                    files_struct_stdin_probe_ready_data_cleared(self);
                    TtyInputWait.state == State::Ready;
                    NTtyLineDiscipline.state == State::Ready;
                }

                ensures {
                    files_struct_stdin_blocking_wait_enabled(self);
                    tty_input_wait_bound_to_flip_buffer(TtyInputWait, TtyFlipBuffer);
                    tty_input_wait_irq_rx_wakeup_first_slice(TtyInputWait);
                    tty_input_wait_read_wait_entry_first_slice(TtyInputWait);
                    tty_input_wait_read_wait_finish_first_slice(TtyInputWait);
                    tty_input_wait_poll_table_first_slice(TtyInputWait);
                    tty_input_wait_poll_freewait_first_slice(TtyInputWait);
                    tty_input_wait_scheduler_sleep_deferred(TtyInputWait);
                    n_tty_canonical_line_readiness_first_slice(NTtyLineDiscipline);
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

            on Action::InstallFirstFreeCharDevice {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    OpenFileDescription.state == State::Ready;
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(FileBackend, FileBackendKind::CharDevice);
                    file_backend_char_device_console_bound(FileBackend) ||
                        file_backend_char_device_null_bound(FileBackend);
                }

                ensures {
                    fd_table_first_free_user_fd_installed(self, OpenFileDescription);
                    fd_table_fd_entries_have_independent_status_flags(self);
                }
            }

            on Action::Close(fd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(self, fd, OpenFileDescription);
                }

                ensures {
                    fd_table_fd_closed(self, fd);
                    fd_table_stdio_fd_closed(self, fd);
                }
            }

            on Action::Dup3(oldfd: FdRef, newfd: FdRef) {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(self, oldfd, OpenFileDescription);
                }

                ensures {
                    fd_table_fd_duplicated(self, oldfd, newfd);
                    fd_table_fd_bound(self, newfd, OpenFileDescription);
                    fd_table_dup3_close_on_exec_bound(self, newfd);
                }
            }

            on Action::CloseOnExec {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                }

                ensures {
                    fd_table_close_on_exec_scanned(self);
                    fd_table_close_on_exec_closed(self);
                }
            }

            on Action::SaveParentSnapshot {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                }

                ensures {
                    fd_table_parent_snapshot_saved(self);
                }
            }

            on Action::RestoreParentSnapshot {
                depends_on {
                    FileDescriptorTable.state == State::Ready;
                    fd_table_parent_snapshot_saved(self);
                }

                ensures {
                    fd_table_parent_snapshot_restored(self);
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

            on Action::ReadCharDevice {
                depends_on {
                    OpenFileDescription.state == State::Ready;
                    FileBackend.state == State::Ready;
                    open_file_description_backend_bound(self, FileBackend);
                    open_file_description_readable(self);
                    file_backend_kind_bound(FileBackend, FileBackendKind::CharDevice);
                    file_backend_char_device_read_supported(FileBackend);
                    NTtyLineDiscipline.state == State::Ready;
                }

                drives {
                    FileBackend.Action::ReadCharDevice;
                    NTtyLineDiscipline.Action::ReadLineOrBytes;
                }

                ensures {
                    open_file_description_read_dispatches_backend(self, FileBackend);
                    open_file_description_read_observed(self);
                    file_backend_char_device_read_returns_ready_data(FileBackend);
                    n_tty_canonical_read_returns_through_newline(NTtyLineDiscipline);
                    n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
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
                    file_backend_char_device_read_supported(self);
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
            file_backend_char_device_read_supported(self);
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

            on Action::ReadCharDevice {
                depends_on {
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(self, FileBackendKind::CharDevice);
                    file_backend_char_device_read_supported(self);
                    NTtyLineDiscipline.state == State::Ready;
                }

                drives {
                    NTtyLineDiscipline.Action::ReadLineOrBytes;
                }

                ensures {
                    file_backend_char_device_read_returns_ready_data(self);
                    n_tty_canonical_read_returns_through_newline(NTtyLineDiscipline);
                    n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
                }
            }

            on Action::ReadNullDevice {
                depends_on {
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(self, FileBackendKind::CharDevice);
                    file_backend_char_device_null_bound(self);
                    file_backend_char_device_read_supported(self);
                }

                ensures {
                    file_backend_null_device_read_returns_eof(self);
                }
            }

            on Action::WriteNullDevice {
                depends_on {
                    FileBackend.state == State::Ready;
                    file_backend_kind_bound(self, FileBackendKind::CharDevice);
                    file_backend_char_device_null_bound(self);
                    file_backend_char_device_write_supported(self);
                }

                ensures {
                    file_backend_null_device_write_discards_data(self);
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
