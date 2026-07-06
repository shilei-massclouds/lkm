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
 * FsStruct carries the task-visible root and pwd dentry references. The
 * current path-walk slice supports absolute paths from FsStruct.root, the
 * first cwd-relative AT_FDCWD subset from FsStruct.pwd for ".", single
 * relative components and "./component", direct child lookup, mount crossing and Linux-like
 * symlink restart semantics for the modeled read-only ext2 subset.
 * Readlink-style lookup is a separate
 * operation: it walks all parent components normally, does not follow the
 * final symlink, and copies that symlink body. Page cache, mount namespace,
 * permissions, credentials, full relative dirfd/cwd walking including "..",
 * rename, hardlink,
 * open flags and complete file descriptor tables stay deferred.
 */

enum VfsInodeKind {
    Directory,
    RegularFile,
    DeviceNode,
    Symlink,
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
predicate vfs_rootfs_mount_set<T, M>(core: T, mount: M) -> bool;
predicate vfs_rootfs_dentry_set<T, D>(core: T, dentry: D) -> bool;
predicate vfs_ext2_mount_created<T, F>(core: T, fs: F) -> bool;
predicate vfs_ext2_lookup_dispatches_backend<T, F>(core: T, fs: F) -> bool;
predicate vfs_ext2_read_dispatches_backend<T, F>(core: T, fs: F) -> bool;
predicate vfs_mount_moved_to_root<T, M, D>(core: T, mount: M, root_dentry: D) -> bool;
predicate vfs_absolute_path_walk_supported<T>(core: T) -> bool;
predicate vfs_at_fdcwd_relative_dot_supported<T>(core: T) -> bool;
predicate vfs_at_fdcwd_relative_single_component_supported<T>(core: T) -> bool;
predicate vfs_at_fdcwd_relative_dot_component_supported<T>(core: T) -> bool;
predicate vfs_relative_path_walk_full_linux_model_deferred<T>(core: T) -> bool;
predicate vfs_path_absolute<T>(path: T) -> bool;
predicate vfs_path_components_bound<T>(path: T) -> bool;
predicate vfs_path_walk_resolves<T, D>(core: T, dentry: D) -> bool;
predicate vfs_path_walk_crosses_mount<T, M>(core: T, mount: M) -> bool;
predicate vfs_path_walk_follows_symlink<T, D>(core: T, dentry: D) -> bool;
predicate vfs_path_walk_symlink_budget_matches_linux_6_12<T>(core: T) -> bool;
predicate vfs_symlink_absolute_target_restarts_at_root<T>(core: T) -> bool;
predicate vfs_symlink_relative_target_restarts_at_parent<T>(core: T) -> bool;
predicate vfs_symlink_remaining_path_preserved<T>(core: T) -> bool;
predicate vfs_symlink_loop_returns_eloop<T>(core: T) -> bool;
predicate vfs_readlink_final_symlink_not_followed<T, D>(core: T, dentry: D) -> bool;
predicate vfs_readlink_returns_symlink_target<T, D>(core: T, dentry: D) -> bool;
predicate vfs_readlink_truncates_to_user_buffer<T>(core: T) -> bool;
predicate vfs_stat_final_symlink_nofollow_supported<T, D>(core: T, dentry: D) -> bool;
predicate vfs_stat_follows_final_symlink_by_default<T, D>(core: T, dentry: D) -> bool;
predicate vfs_open_path_allocates_file<T, F>(core: T, file: F) -> bool;
predicate vfs_read_path_returns_data<T, F>(core: T, file: F) -> bool;
predicate vfs_path_read_start_checkpoint<T, P>(core: T, path: P) -> bool;
predicate vfs_path_read_resolved_checkpoint<T, D>(core: T, dentry: D) -> bool;
predicate vfs_path_read_failed_checkpoint_defined<T>(core: T) -> bool;
predicate vfs_path_read_error_classification_contract_ready<T>(core: T) -> bool;

predicate fs_struct_allocated<T>(fs: T) -> bool;
predicate fs_struct_initial_root_bound<T, D>(fs: T, dentry: D) -> bool;
predicate fs_struct_root_dentry_set<T, D>(fs: T, dentry: D) -> bool;
predicate fs_struct_pwd_dentry_set<T, D>(fs: T, dentry: D) -> bool;
predicate fs_struct_root_pwd_same<T>(fs: T) -> bool;
predicate fs_struct_root_pwd_same_recomputed<T>(fs: T) -> bool;
predicate fs_struct_pwd_chdir_to_real_root<T, D>(fs: T, dentry: D) -> bool;
predicate fs_struct_chroot_dot_done<T, D>(fs: T, dentry: D) -> bool;

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
predicate inode_symlink_target_ready<T>(inode: T) -> bool;
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
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    vfs_core_initialized(VfsCore);
                    vfs_core_fs_type_registry_ready(VfsCore);
                    vfs_core_mount_table_ready(VfsCore);
                    vfs_core_dentry_cache_ready(VfsCore);
                    vfs_core_inode_table_ready(VfsCore);
                    vfs_core_file_table_ready(VfsCore);
                    vfs_absolute_path_walk_supported(VfsCore);
                    vfs_path_walk_symlink_budget_matches_linux_6_12(VfsCore);
                    vfs_at_fdcwd_relative_dot_supported(VfsCore);
                    vfs_at_fdcwd_relative_single_component_supported(VfsCore);
                    vfs_at_fdcwd_relative_dot_component_supported(VfsCore);
                    vfs_core_page_cache_deferred(VfsCore);
                    vfs_core_permissions_deferred(VfsCore);
                    vfs_core_mount_namespace_deferred(VfsCore);
                    vfs_path_read_failed_checkpoint_defined(VfsCore);
                    vfs_path_read_error_classification_contract_ready(VfsCore);
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
            vfs_absolute_path_walk_supported(VfsCore);
            vfs_path_walk_symlink_budget_matches_linux_6_12(VfsCore);
            vfs_path_read_failed_checkpoint_defined(VfsCore);
            vfs_path_read_error_classification_contract_ready(VfsCore);
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
                    SuperBlock.Transition::Setup;
                    Inode.Transition::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Transition::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Transition::Setup;
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
                    vfs_rootfs_mount_set(VfsCore, Mount);
                    vfs_rootfs_dentry_set(VfsCore, Dentry);
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
                    SuperBlock.Transition::Setup;
                    Inode.Transition::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Transition::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Transition::Setup;
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
                    SuperBlock.Transition::Setup;
                    Inode.Transition::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Transition::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Transition::Setup;
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
                    SuperBlock.Transition::Setup;
                    Inode.Transition::Setup;
                    Inode.Action::CreateRootDirectory;
                    Dentry.Transition::Setup;
                    Dentry.Action::CreateRoot;
                    Mount.Transition::Setup;
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

            Action::MoveMountToRoot(mount: Mount, root_dentry: Dentry) {
                state_effect: StateEffect::None;
                depends_on {
                    mount_allocated(Mount);
                    mount_root_dentry_bound(Mount, Dentry);
                    dentry_positive(root_dentry);
                }
                ensures {
                    mount_point_bound(mount, root_dentry);
                    vfs_mount_moved_to_root(VfsCore, mount, root_dentry);
                }
            }

            Action::Lookup {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(Dentry);
                }
                ensures {
                    dentry_lookup_returns(VfsCore, Dentry);
                }
            }

            Action::WalkPath(path: Path, fs: FsStruct) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    fs_struct_root_dentry_set(fs, Dentry);
                    fs_struct_pwd_dentry_set(fs, Dentry);
                    dentry_positive(Dentry);
                    vfs_path_components_bound(path);
                    vfs_absolute_path_walk_supported(VfsCore);
                    vfs_at_fdcwd_relative_dot_supported(VfsCore);
                    vfs_at_fdcwd_relative_single_component_supported(VfsCore);
                    vfs_at_fdcwd_relative_dot_component_supported(VfsCore);
                }
                drives {
                    PathWalk.Transition::Setup;
                    VfsCore.Action::Lookup;
                    VfsCore.Action::FollowMount;
                    VfsCore.Action::FollowSymlink;
                }
                ensures {
                    vfs_path_walk_resolves(VfsCore, Dentry);
                    vfs_relative_path_walk_full_linux_model_deferred(VfsCore);
                }
            }

            Action::StatPathNoFollow(path: Path, fs: FsStruct) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    fs_struct_root_dentry_set(fs, Dentry);
                    fs_struct_pwd_dentry_set(fs, Dentry);
                    dentry_positive(Dentry);
                    vfs_path_components_bound(path);
                }
                drives {
                    VfsCore.Action::Lookup;
                    VfsCore.Action::FollowMount;
                }
                ensures {
                    vfs_stat_final_symlink_nofollow_supported(VfsCore, Dentry);
                    vfs_stat_follows_final_symlink_by_default(VfsCore, Dentry);
                }
            }

            Action::FollowSymlink(link: Dentry) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    dentry_positive(link);
                    inode_kind_is(Inode, VfsInodeKind::Symlink);
                    inode_symlink_target_ready(Inode);
                    vfs_path_walk_symlink_budget_matches_linux_6_12(VfsCore);
                }
                ensures {
                    vfs_path_walk_follows_symlink(VfsCore, link);
                    vfs_symlink_absolute_target_restarts_at_root(VfsCore);
                    vfs_symlink_relative_target_restarts_at_parent(VfsCore);
                    vfs_symlink_remaining_path_preserved(VfsCore);
                    vfs_symlink_loop_returns_eloop(VfsCore);
                }
            }

            Action::FollowMount(mount_point: Dentry) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    dentry_positive(mount_point);
                }
                ensures {
                    vfs_path_walk_crosses_mount(VfsCore, Mount);
                    dentry_mount_root_redirects(mount_point, Dentry);
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
                    Inode.Transition::Setup;
                    Inode.Action::CreateFile;
                    Dentry.Transition::Setup;
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

            Action::LookupExt2SymlinkReadOnly {
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
                    Inode.Transition::Setup;
                    Inode.Action::CreateSymlink;
                    Dentry.Transition::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    vfs_ext2_lookup_dispatches_backend(VfsCore, Ext2FileSystem);
                    dentry_lookup_returns(VfsCore, Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Symlink);
                    inode_symlink_target_ready(Inode);
                    inode_ext2_inode_bound(Inode, Ext2InodeRef::LookupFile);
                    inode_read_only_backed(Inode);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }

            Action::CreateDirectory {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Transition::Setup;
                    Inode.Action::CreateDirectory;
                    Dentry.Transition::Setup;
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
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Transition::Setup;
                    Inode.Action::CreateFile;
                    Dentry.Transition::Setup;
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
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Transition::Setup;
                    Inode.Action::CreateDeviceNode;
                    Dentry.Transition::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::DeviceNode);
                    dentry_device_node_bound(Dentry);
                    dentry_child_inserted(Dentry, Dentry);
                }
            }

            Action::CreateSymlink {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                drives {
                    Inode.Transition::Setup;
                    Inode.Action::CreateSymlink;
                    Dentry.Transition::Setup;
                    Dentry.Action::InsertChild;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::Symlink);
                    inode_symlink_target_ready(Inode);
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
                    File.Transition::Setup;
                }
                ensures {
                    file_allocated(File);
                    file_dentry_bound(File, Dentry);
                    file_inode_bound(File, Inode);
                    file_position_ready(File);
                }
            }

            Action::OpenPath(path: Path, fs: FsStruct) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    vfs_path_components_bound(path);
                    vfs_path_walk_resolves(VfsCore, Dentry);
                    inode_kind_is(Inode, VfsInodeKind::RegularFile);
                }
                drives {
                    VfsCore.Action::WalkPath(path, fs);
                    File.Transition::Setup;
                }
                ensures {
                    file_allocated(File);
                    file_dentry_bound(File, Dentry);
                    file_inode_bound(File, Inode);
                    file_position_ready(File);
                    vfs_open_path_allocates_file(VfsCore, File);
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

            Action::ReadPath(path: Path, fs: FsStruct) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    vfs_path_absolute(path);
                    vfs_path_walk_resolves(VfsCore, Dentry);
                    file_allocated(File);
                }
                drives {
                    VfsCore.Action::WalkPath(path, fs);
                    VfsCore.Action::OpenPath(path, fs);
                    VfsCore.Action::ReadFile;
                    VfsCore.Action::ReadExt2File;
                }
                ensures {
                    vfs_path_read_start_checkpoint(VfsCore, path);
                    vfs_read_path_returns_data(VfsCore, File);
                    vfs_path_read_resolved_checkpoint(VfsCore, Dentry);
                }
            }

            Action::ReadlinkPath(path: Path, fs: FsStruct) {
                state_effect: StateEffect::None;
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    vfs_path_absolute(path);
                    inode_kind_is(Inode, VfsInodeKind::Symlink);
                    inode_symlink_target_ready(Inode);
                }
                drives {
                    VfsCore.Action::Lookup;
                    VfsCore.Action::FollowMount;
                }
                ensures {
                    vfs_readlink_final_symlink_not_followed(VfsCore, Dentry);
                    vfs_readlink_returns_symlink_target(VfsCore, Dentry);
                    vfs_readlink_truncates_to_user_buffer(VfsCore);
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

object FsStruct: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VfsCore.state == State::Ready;
                }

                ensures {
                    fs_struct_allocated(FsStruct);
                    fs_struct_initial_root_bound(FsStruct, Dentry);
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                    fs_struct_pwd_dentry_set(FsStruct, Dentry);
                    fs_struct_root_pwd_same(FsStruct);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            fs_struct_allocated(FsStruct);
            fs_struct_root_dentry_set(FsStruct, Dentry);
            fs_struct_pwd_dentry_set(FsStruct, Dentry);
        }

        actions {
            Action::Chdir(dentry: Dentry) {
                state_effect: StateEffect::None;
                depends_on {
                    dentry_positive(dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                ensures {
                    fs_struct_pwd_dentry_set(FsStruct, dentry);
                    fs_struct_root_pwd_same_recomputed(FsStruct);
                    fs_struct_pwd_chdir_to_real_root(FsStruct, dentry);
                }
            }

            Action::ChrootDot {
                state_effect: StateEffect::None;
                depends_on {
                    fs_struct_pwd_dentry_set(FsStruct, Dentry);
                    dentry_positive(Dentry);
                    inode_kind_is(Inode, VfsInodeKind::Directory);
                }
                ensures {
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                    fs_struct_pwd_dentry_set(FsStruct, Dentry);
                    fs_struct_root_pwd_same(FsStruct);
                    fs_struct_chroot_dot_done(FsStruct, Dentry);
                }
            }
        }
    }
}

object Path: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    vfs_path_absolute(Path);
                    vfs_path_components_bound(Path);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vfs_path_absolute(Path);
            vfs_path_components_bound(Path);
        }
    }
}

object PathWalk: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    Path.state == State::Ready;
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                }
                ensures {
                    vfs_absolute_path_walk_supported(VfsCore);
                    vfs_path_walk_resolves(VfsCore, Dentry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            vfs_absolute_path_walk_supported(VfsCore);
            vfs_path_walk_resolves(VfsCore, Dentry);
        }
    }
}

object Mount: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
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

            Action::CreateSymlink {
                state_effect: StateEffect::None;
                depends_on {
                    Inode.state == State::Ready;
                }
                ensures {
                    inode_kind_is(Inode, VfsInodeKind::Symlink);
                    inode_symlink_target_ready(Inode);
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
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
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
