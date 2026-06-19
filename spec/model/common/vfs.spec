/*
 * Minimal VFS, ramfs and devfs mount substrate model.
 *
 * This first slice follows the Linux VFS shape only far enough to mount an
 * in-memory ramfs instance and exercise pathname operations through VFS
 * objects. FileSystemType is the type/driver descriptor; Mount and SuperBlock
 * are per-mount instance state; SuperBlock owns the root dentry/root inode
 * anchors used as the traversal start. Directory entries are represented by
 * Dentry objects bound to Inode objects. Device nodes are only named VFS
 * entries in this slice; device file operations stay with the owning device
 * subsystems. The only block-backed filesystem path currently admitted is a
 * read-only ext2 mount whose lookup/read operations dispatch to Ext2FileSystem.
 * Page cache, mount namespace, permissions, credentials, path walk corner
 * cases, rename, symlink, hardlink, open flags and file descriptor tables stay
 * deferred.
 */

enum VfsInodeKind {
    Directory,
    RegularFile,
    DeviceNode,
}

predicate vfs_core_initialized<T>(core: T) -> bool;
predicate vfs_core_fs_type_registry_ready<T>(core: T) -> bool;
predicate vfs_core_mount_table_ready<T>(core: T) -> bool;
predicate vfs_core_dentry_cache_ready<T>(core: T) -> bool;
predicate vfs_core_inode_table_ready<T>(core: T) -> bool;
predicate vfs_core_file_table_ready<T>(core: T) -> bool;
predicate vfs_core_page_cache_deferred<T>(core: T) -> bool;
predicate vfs_core_permissions_deferred<T>(core: T) -> bool;
predicate vfs_core_mount_namespace_deferred<T>(core: T) -> bool;

predicate fs_type_registered<T, F>(core: T, fs_type: F) -> bool;
predicate fs_type_name_bound<T>(fs_type: T) -> bool;
predicate fs_type_mount_callback_bound<T>(fs_type: T) -> bool;
predicate ramfs_type_is_memory_backed<T>(fs_type: T) -> bool;
predicate ramfs_type_registered<T, F>(core: T, fs_type: F) -> bool;
predicate rootfs_fs_type_uses_ramfs<T>(fs_type: T) -> bool;
predicate rootfs_mount_created<T>(core: T) -> bool;
predicate devfs_mount_created<T>(core: T) -> bool;

predicate mount_allocated<T>(mount: T) -> bool;
predicate mount_fs_type_bound<T, F>(mount: T, fs_type: F) -> bool;
predicate mount_devfs_type_bound<T>(mount: T) -> bool;
predicate mount_ext2_type_bound<T>(mount: T) -> bool;
predicate mount_superblock_bound<T, S>(mount: T, superblock: S) -> bool;
predicate mount_root_dentry_bound<T, D>(mount: T, dentry: D) -> bool;
predicate mount_point_bound<T, D>(mount: T, mount_point: D) -> bool;
predicate vfs_current_root_mount_set<T, M>(core: T, mount: M) -> bool;
predicate vfs_current_root_dentry_set<T, D>(core: T, dentry: D) -> bool;
predicate vfs_ext2_mount_created<T, F>(core: T, fs: F) -> bool;
predicate vfs_ext2_lookup_dispatches_backend<T, F>(core: T, fs: F) -> bool;
predicate vfs_ext2_read_dispatches_backend<T, F>(core: T, fs: F) -> bool;

predicate superblock_allocated<T>(superblock: T) -> bool;
predicate superblock_fs_type_bound<T, F>(superblock: T, fs_type: F) -> bool;
predicate superblock_root_dentry_bound<T, D>(superblock: T, dentry: D) -> bool;
predicate superblock_root_inode_bound<T, I>(superblock: T, inode: I) -> bool;
predicate superblock_root_dentry_inode_matches<T, D, I>(superblock: T, dentry: D, inode: I) -> bool;
predicate superblock_ramfs_private_bound<T>(superblock: T) -> bool;
predicate superblock_devfs_private_bound<T>(superblock: T) -> bool;
predicate superblock_ext2_private_bound<T>(superblock: T) -> bool;

predicate inode_allocated<T>(inode: T) -> bool;
predicate inode_superblock_bound<T, S>(inode: T, superblock: S) -> bool;
predicate inode_kind_is<T>(inode: T, kind: VfsInodeKind) -> bool;
predicate inode_directory_children_ready<T>(inode: T) -> bool;
predicate inode_file_data_ready<T>(inode: T) -> bool;
predicate inode_size_updated<T>(inode: T) -> bool;
predicate inode_ext2_inode_bound<T, I>(inode: T, ext2_inode: I) -> bool;
predicate inode_read_only_backed<T>(inode: T) -> bool;

predicate dentry_allocated<T>(dentry: T) -> bool;
predicate dentry_name_bound<T>(dentry: T) -> bool;
predicate dentry_parent_bound<T, P>(dentry: T, parent: P) -> bool;
predicate dentry_inode_bound<T, I>(dentry: T, inode: I) -> bool;
predicate dentry_positive<T>(dentry: T) -> bool;
predicate dentry_child_inserted<T, P>(dentry: T, parent: P) -> bool;
predicate dentry_child_removed<T, P>(dentry: T, parent: P) -> bool;
predicate dentry_mount_root_redirects<T, D>(mount_point: T, root: D) -> bool;
predicate dentry_device_node_bound<T>(dentry: T) -> bool;
predicate dentry_lookup_returns<T, D>(core: T, dentry: D) -> bool;
predicate dentry_readdir_lists<T, D>(core: T, dentry: D) -> bool;

predicate file_allocated<T>(file: T) -> bool;
predicate file_dentry_bound<T, D>(file: T, dentry: D) -> bool;
predicate file_inode_bound<T, I>(file: T, inode: I) -> bool;
predicate file_position_ready<T>(file: T) -> bool;
predicate file_write_committed<T>(file: T) -> bool;
predicate file_read_returns_written_data<T>(file: T) -> bool;
predicate file_read_returns_backend_data<T>(file: T) -> bool;

object FileSystemType: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                ensures {
                    fs_type_name_bound(FileSystemType);
                    fs_type_mount_callback_bound(FileSystemType);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            fs_type_name_bound(FileSystemType);
            fs_type_mount_callback_bound(FileSystemType);
        }
    }
}

object RamFsType: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                ensures {
                    fs_type_name_bound(RamFsType);
                    fs_type_mount_callback_bound(RamFsType);
                    ramfs_type_is_memory_backed(RamFsType);
                    rootfs_fs_type_uses_ramfs(RamFsType);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            fs_type_name_bound(RamFsType);
            fs_type_mount_callback_bound(RamFsType);
            ramfs_type_is_memory_backed(RamFsType);
            rootfs_fs_type_uses_ramfs(RamFsType);
        }
    }
}

object VfsCore: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                ensures {
                    vfs_core_initialized(VfsCore);
                    vfs_core_fs_type_registry_ready(VfsCore);
                    vfs_core_mount_table_ready(VfsCore);
                    vfs_core_dentry_cache_ready(VfsCore);
                    vfs_core_inode_table_ready(VfsCore);
                    vfs_core_file_table_ready(VfsCore);
                    vfs_core_page_cache_deferred(VfsCore);
                    vfs_core_permissions_deferred(VfsCore);
                    vfs_core_mount_namespace_deferred(VfsCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vfs_core_initialized(VfsCore);
            vfs_core_fs_type_registry_ready(VfsCore);
            vfs_core_mount_table_ready(VfsCore);
            vfs_core_dentry_cache_ready(VfsCore);
            vfs_core_inode_table_ready(VfsCore);
            vfs_core_file_table_ready(VfsCore);
        }

        actions {
            Action::RegisterRamFsType(fs_type: RamFsType) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    RamFsType.state == State::Ready;
                    fs_type_name_bound(fs_type);
                    fs_type_mount_callback_bound(fs_type);
                    ramfs_type_is_memory_backed(fs_type);
                }
                ensures {
                    fs_type_registered(VfsCore, fs_type);
                    ramfs_type_registered(VfsCore, fs_type);
                }
            }

            Action::MountInitialRamFsRoot {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    ramfs_type_registered(VfsCore, RamFsType);
                }
                drives {
                    SuperBlock.Event::Setup;
                    Inode.Event::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Event::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Event::Setup;
                }
                ensures {
                    mount_allocated(Mount);
                    mount_fs_type_bound(Mount, RamFsType);
                    mount_superblock_bound(Mount, SuperBlock);
                    mount_root_dentry_bound(Mount, Dentry);
                    superblock_allocated(SuperBlock);
                    superblock_fs_type_bound(SuperBlock, RamFsType);
                    superblock_root_dentry_bound(SuperBlock, Dentry);
                    superblock_root_inode_bound(SuperBlock, Inode);
                    superblock_root_dentry_inode_matches(SuperBlock, Dentry, Inode);
                    superblock_ramfs_private_bound(SuperBlock);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_directory_children_ready(Inode);
                    vfs_current_root_mount_set(VfsCore, Mount);
                    vfs_current_root_dentry_set(VfsCore, Dentry);
                    rootfs_mount_created(VfsCore);
                }
            }

            Action::MountRamFsAt(mount_point: Dentry) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    ramfs_type_registered(VfsCore, RamFsType);
                    dentry_positive(mount_point);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    SuperBlock.Event::Setup;
                    Inode.Event::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Event::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Event::Setup;
                }
                ensures {
                    mount_allocated(Mount);
                    mount_fs_type_bound(Mount, RamFsType);
                    mount_superblock_bound(Mount, SuperBlock);
                    mount_root_dentry_bound(Mount, Dentry);
                    mount_point_bound(Mount, mount_point);
                    dentry_mount_root_redirects(mount_point, Dentry);
                    superblock_allocated(SuperBlock);
                    superblock_fs_type_bound(SuperBlock, RamFsType);
                    superblock_root_dentry_bound(SuperBlock, Dentry);
                    superblock_root_inode_bound(SuperBlock, Inode);
                    superblock_root_dentry_inode_matches(SuperBlock, Dentry, Inode);
                    superblock_ramfs_private_bound(SuperBlock);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_directory_children_ready(Inode);
                }
            }

            Action::MountDevFsAt(mount_point: Dentry) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    dentry_positive(mount_point);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    SuperBlock.Event::Setup;
                    Inode.Event::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Event::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Event::Setup;
                }
                ensures {
                    mount_allocated(Mount);
                    mount_devfs_type_bound(Mount);
                    mount_superblock_bound(Mount, SuperBlock);
                    mount_root_dentry_bound(Mount, Dentry);
                    mount_point_bound(Mount, mount_point);
                    dentry_mount_root_redirects(mount_point, Dentry);
                    superblock_allocated(SuperBlock);
                    superblock_devfs_private_bound(SuperBlock);
                    superblock_root_dentry_bound(SuperBlock, Dentry);
                    superblock_root_inode_bound(SuperBlock, Inode);
                    superblock_root_dentry_inode_matches(SuperBlock, Dentry, Inode);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_directory_children_ready(Inode);
                    devfs_mount_created(VfsCore);
                }
            }

            Action::MountExt2At(mount_point: Dentry, fs: Ext2FileSystem) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    Ext2FileSystem.state == State::Ready;
                    dentry_positive(mount_point);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    ext2_filesystem_root_dentry_bound(fs);
                    ext2_inode_is_root_dir(fs, Ext2InodeRef::Root);
                }
                drives {
                    SuperBlock.Event::Setup;
                    Inode.Event::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Event::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Event::Setup;
                }
                ensures {
                    mount_allocated(Mount);
                    mount_ext2_type_bound(Mount);
                    mount_superblock_bound(Mount, SuperBlock);
                    mount_root_dentry_bound(Mount, Dentry);
                    mount_point_bound(Mount, mount_point);
                    dentry_mount_root_redirects(mount_point, Dentry);
                    superblock_allocated(SuperBlock);
                    superblock_ext2_private_bound(SuperBlock);
                    superblock_root_dentry_bound(SuperBlock, Dentry);
                    superblock_root_inode_bound(SuperBlock, Inode);
                    superblock_root_dentry_inode_matches(SuperBlock, Dentry, Inode);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_ext2_inode_bound(Inode, Ext2InodeRef::Root);
                    inode_read_only_backed(Inode);
                    vfs_ext2_mount_created(VfsCore, fs);
                }
            }

            Action::SetRoot {
                state_effect: StateEffect::None;
                depends_on {
                    mount_allocated(Mount);
                    mount_root_dentry_bound(Mount, Dentry);
                }
                ensures {
                    vfs_current_root_mount_set(VfsCore, Mount);
                    vfs_current_root_dentry_set(VfsCore, Dentry);
                }
            }

            Action::Lookup {
                state_effect: StateEffect::None;
                depends_on {
                    vfs_current_root_dentry_set(VfsCore, Dentry);
                    dentry_positive(Dentry);
                }
                ensures {
                    dentry_lookup_returns(VfsCore, Dentry);
                }
            }

            Action::LookupExt2ReadOnly {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    Ext2FileSystem.state == State::Online;
                    mount_ext2_type_bound(Mount);
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Ext2FileSystem.Action::LookupRootName;
                    Inode.Event::Setup;
                    Inode.Action::CreateFile;
                    Dentry.Event::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    vfs_ext2_lookup_dispatches_backend(VfsCore, Ext2FileSystem);
                    dentry_lookup_returns(VfsCore, Dentry);
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                    inode_ext2_inode_bound(Inode, Ext2InodeRef::LookupFile);
                    inode_read_only_backed(Inode);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }

            Action::CreateDirectory {
                state_effect: StateEffect::None;
                depends_on {
                    vfs_current_root_dentry_set(VfsCore, Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Event::Setup;
                    Inode.Action::CreateDirectory;
                    Dentry.Event::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_directory_children_ready(Inode);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }

            Action::CreateFile {
                state_effect: StateEffect::None;
                depends_on {
                    vfs_current_root_dentry_set(VfsCore, Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Event::Setup;
                    Inode.Action::CreateFile;
                    Dentry.Event::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                    inode_file_data_ready(Inode);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }

            Action::CreateDeviceNode {
                state_effect: StateEffect::None;
                depends_on {
                    vfs_current_root_dentry_set(VfsCore, Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Event::Setup;
                    Inode.Action::CreateDeviceNode;
                    Dentry.Event::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::DeviceNode);
                    dentry_device_node_bound(Dentry);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }

            Action::OpenFile {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                }
                drives {
                    File.Event::Setup;
                }
                ensures {
                    file_allocated(File);
                    file_dentry_bound(File, Dentry);
                    file_inode_bound(File, Inode);
                    file_position_ready(File);
                }
            }

            Action::WriteFile {
                state_effect: StateEffect::None;
                depends_on {
                    file_allocated(File);
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                }
                ensures {
                    file_write_committed(File);
                    inode_size_updated(Inode);
                }
            }

            Action::ReadFile {
                state_effect: StateEffect::None;
                depends_on {
                    file_allocated(File);
                    file_write_committed(File);
                }
                ensures {
                    file_read_returns_written_data(File);
                }
            }

            Action::ReadExt2File {
                state_effect: StateEffect::None;
                depends_on {
                    file_allocated(File);
                    Ext2FileSystem.state == State::Online;
                    mount_ext2_type_bound(Mount);
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                    inode_read_only_backed(Inode);
                    inode_ext2_inode_bound(Inode, Ext2InodeRef::LookupFile);
                }
                drives {
                    Ext2FileSystem.Action::ReadVfsFile;
                }
                ensures {
                    vfs_ext2_read_dispatches_backend(VfsCore, Ext2FileSystem);
                    file_read_returns_backend_data(File);
                }
            }

            Action::ReadDir {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                ensures {
                    dentry_readdir_lists(VfsCore, Dentry);
                }
            }

            Action::Remove {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(Dentry);
                    dentry_child_inserted(Dentry, Dentry);
                }
                ensures {
                    dentry_child_removed(Dentry, Dentry);
                }
            }
        }
    }
}

object Mount: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SuperBlock.state == State::Ready;
                    Dentry.state == State::Ready;
                }
                ensures {
                    mount_allocated(Mount);
                    mount_superblock_bound(Mount, SuperBlock);
                    mount_root_dentry_bound(Mount, Dentry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mount_allocated(Mount);
            mount_superblock_bound(Mount, SuperBlock);
            mount_root_dentry_bound(Mount, Dentry);
        }
    }
}

object SuperBlock: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                ensures {
                    superblock_allocated(SuperBlock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            superblock_allocated(SuperBlock);
        }
    }
}

object Inode: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SuperBlock.state == State::Ready;
                }
                ensures {
                    inode_allocated(Inode);
                    inode_superblock_bound(Inode, SuperBlock);
                }
            }
        }

        actions {
            Action::CreateRootDirectory {
                state_effect: StateEffect::None;
                depends_on {
                    Inode.state == State::Ready;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_directory_children_ready(Inode);
                }
            }

            Action::CreateDirectory {
                state_effect: StateEffect::None;
                depends_on {
                    Inode.state == State::Ready;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                    inode_directory_children_ready(Inode);
                }
            }

            Action::CreateFile {
                state_effect: StateEffect::None;
                depends_on {
                    Inode.state == State::Ready;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                    inode_file_data_ready(Inode);
                }
            }

            Action::CreateDeviceNode {
                state_effect: StateEffect::None;
                depends_on {
                    Inode.state == State::Ready;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::DeviceNode);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            inode_allocated(Inode);
            inode_superblock_bound(Inode, SuperBlock);
        }
    }
}

object Dentry: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Inode.state == State::Ready;
                }
                ensures {
                    dentry_allocated(Dentry);
                    dentry_name_bound(Dentry);
                    dentry_inode_bound(Dentry, Inode);
                    dentry_positive(Dentry);
                }
            }
        }

        actions {
            Action::CreateRoot {
                state_effect: StateEffect::None;
                depends_on {
                    Dentry.state == State::Ready;
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                ensures {
                    dentry_inode_bound(Dentry, Inode);
                }
            }

            Action::InsertChild {
                state_effect: StateEffect::None;
                depends_on {
                    Dentry.state == State::Ready;
                    Inode.state == State::Ready;
                }
                ensures {
                    dentry_parent_bound(Dentry, Dentry);
                    dentry_inode_bound(Dentry, Inode);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            dentry_allocated(Dentry);
            dentry_name_bound(Dentry);
            dentry_inode_bound(Dentry, Inode);
            dentry_positive(Dentry);
        }
    }
}

object File: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Dentry.state == State::Ready;
                    Inode.state == State::Ready;
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                }
                ensures {
                    file_allocated(File);
                    file_dentry_bound(File, Dentry);
                    file_inode_bound(File, Inode);
                    file_position_ready(File);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            file_allocated(File);
            file_dentry_bound(File, Dentry);
            file_inode_bound(File, Inode);
            file_position_ready(File);
        }
    }
}
