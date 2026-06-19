/*
 * Read-only ext2 mounted filesystem slice.
 *
 * The ext2 model is split into three lifecycle objects:
 * - Ext2Driver is the Linux-like filesystem driver and operation table holder.
 *   It replaces the older Ext2Type name and covers file_system_type plus the
 *   currently modeled super/inode/file operations.
 * - Ext2Volume is the block-backed on-disk ext2 volume. Preset confirms that a
 *   default block device contains a recognizable ext2 layout. Absence or an
 *   invalid layout is not a kernel-fatal condition; the successful branch is
 *   represented by Ext2Volume.Ready, while implementation returns an ordinary
 *   error and later Ext2FileSystem setup does not proceed.
 * - Ext2FileSystem is the mounted in-memory ext2 filesystem instance. It owns
 *   the parsed metadata, operation-set binding and root dentry/inode entry
 *   point. Superblock fields, group descriptor, inode records, dirents and
 *   file-read results are still structured records/facts owned by
 *   Ext2FileSystem in this slice. The current VFS integration is a minimal
 *   read-only mount/read backend; later inode cache, iget refcounting, eviction
 *   and full path walk/page-cache behavior may promote more of them if needed.
 */

enum Ext2InodeRef {
    Root,
    LookupFile,
}

enum Ext2DirEntryRef {
    RootLookup,
}

enum Ext2FileReadRef {
    LookupFile,
}

predicate ext2_driver_registered<T>(driver: T) -> bool;
predicate ext2_driver_read_only<T>(driver: T) -> bool;
predicate ext2_driver_mount_callback_bound<T>(driver: T) -> bool;
predicate ext2_driver_super_operations_bound<T>(driver: T) -> bool;
predicate ext2_driver_inode_operations_bound<T>(driver: T) -> bool;
predicate ext2_driver_file_operations_bound<T>(driver: T) -> bool;

predicate ext2_volume_probe_attempted<T>(volume: T) -> bool;
predicate ext2_volume_targets_block_device<T, D>(volume: T, device: D) -> bool;
predicate ext2_volume_superblock_read<T, B>(volume: T, bh: B) -> bool;
predicate ext2_volume_magic_valid<T>(volume: T) -> bool;
predicate ext2_volume_layout_valid<T>(volume: T) -> bool;
predicate ext2_volume_block_size_supported<T>(volume: T) -> bool;
predicate ext2_volume_block_size_1024_or_2048_or_4096<T>(volume: T) -> bool;
predicate ext2_volume_not_found_is_nonfatal<T>(volume: T) -> bool;

predicate ext2_filesystem_allocated<T>(fs: T) -> bool;
predicate ext2_filesystem_driver_bound<T, D>(fs: T, driver: D) -> bool;
predicate ext2_filesystem_volume_bound<T, V>(fs: T, volume: V) -> bool;
predicate ext2_filesystem_targets_block_device<T, D>(fs: T, device: D) -> bool;
predicate ext2_filesystem_superblock_read<T, B>(fs: T, bh: B) -> bool;
predicate ext2_filesystem_magic_valid<T>(fs: T) -> bool;
predicate ext2_filesystem_block_size_supported<T>(fs: T) -> bool;
predicate ext2_filesystem_block_size_1024_or_2048_or_4096<T>(fs: T) -> bool;
predicate ext2_filesystem_group_desc_read<T, B>(fs: T, bh: B) -> bool;
predicate ext2_filesystem_operations_bound<T, D>(fs: T, driver: D) -> bool;
predicate ext2_filesystem_root_dentry_bound<T>(fs: T) -> bool;
predicate ext2_filesystem_ready<T>(fs: T) -> bool;
predicate ext2_filesystem_mount_boundary_recorded<T>(fs: T) -> bool;
predicate ext2_filesystem_vfs_mount_bound<T, V>(fs: T, vfs: V) -> bool;
predicate ext2_filesystem_vfs_lookup_entry_bound<T>(fs: T) -> bool;
predicate ext2_filesystem_vfs_read_entry_bound<T>(fs: T) -> bool;
predicate ext2_filesystem_page_cache_deferred<T>(fs: T) -> bool;
predicate ext2_filesystem_write_paths_deferred<T>(fs: T) -> bool;

predicate ext2_inode_record_ready<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_number_bound<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_mode_bound<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_size_bound<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_direct_blocks_bound<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_is_root_dir<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_is_regular_file<T, I>(fs: T, inode: I) -> bool;
predicate ext2_inode_indirect_blocks_deferred<T, I>(fs: T, inode: I) -> bool;

predicate ext2_root_lookup_name_bound<T, D>(fs: T, dirent: D) -> bool;
predicate ext2_root_lookup_reads_root_dir<T, I>(fs: T, root: I) -> bool;
predicate ext2_root_lookup_scans_direct_blocks<T, I>(fs: T, root: I) -> bool;
predicate ext2_root_lookup_multi_direct_block_supported<T>(fs: T) -> bool;
predicate ext2_root_lookup_multi_direct_block_observed<T>(fs: T) -> bool;
predicate ext2_root_lookup_not_found_is_nonfatal<T>(fs: T) -> bool;
predicate ext2_root_lookup_indirect_blocks_deferred<T>(fs: T) -> bool;
predicate ext2_root_lookup_dirent_valid<T, D>(fs: T, dirent: D) -> bool;
predicate ext2_root_lookup_returns_inode<T, D, I>(fs: T, dirent: D, inode: I) -> bool;

predicate ext2_file_read_uses_direct_block<T, R, I>(fs: T, read: R, file: I) -> bool;
predicate ext2_file_read_scans_direct_blocks<T, R, I>(fs: T, read: R, file: I) -> bool;
predicate ext2_file_read_multi_direct_block_supported<T, R>(fs: T, read: R) -> bool;
predicate ext2_file_read_uses_buffer_head<T, R, B>(fs: T, read: R, bh: B) -> bool;
predicate ext2_file_read_copies_to_caller<T, R>(fs: T, read: R) -> bool;
predicate ext2_file_read_len_matches_inode_size<T, R, I>(fs: T, read: R, file: I) -> bool;
predicate ext2_file_read_short_buffer_rejected<T, R>(fs: T, read: R) -> bool;
predicate ext2_file_read_indirect_blocks_deferred<T, R>(fs: T, read: R) -> bool;
predicate ext2_file_read_entered_from_vfs<T, R>(fs: T, read: R) -> bool;

object Ext2Driver: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    Bio.state == State::Ready;
                    BufferHead.state == State::Ready;
                }

                ensures {
                    ext2_driver_registered(self);
                    ext2_driver_read_only(self);
                    ext2_driver_mount_callback_bound(self);
                    ext2_driver_super_operations_bound(self);
                    ext2_driver_inode_operations_bound(self);
                    ext2_driver_file_operations_bound(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ext2_driver_registered(self);
            ext2_driver_read_only(self);
            ext2_driver_mount_callback_bound(self);
            ext2_driver_super_operations_bound(self);
            ext2_driver_inode_operations_bound(self);
            ext2_driver_file_operations_bound(self);
        }
    }
}

object Ext2Volume: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Ready {
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_volume_probe_attempted(self);
                    ext2_volume_targets_block_device(self, BlockDevice);
                    ext2_volume_superblock_read(self, BufferHead);
                    ext2_volume_magic_valid(self);
                    ext2_volume_layout_valid(self);
                    ext2_volume_block_size_supported(self);
                    ext2_volume_block_size_1024_or_2048_or_4096(self);
                    ext2_volume_not_found_is_nonfatal(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ext2_volume_probe_attempted(self);
            ext2_volume_magic_valid(self);
            ext2_volume_layout_valid(self);
            ext2_volume_block_size_supported(self);
            ext2_volume_block_size_1024_or_2048_or_4096(self);
            ext2_volume_not_found_is_nonfatal(self);
        }
    }
}

object Ext2FileSystem: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    Ext2Driver.state == State::Ready;
                    Ext2Volume.state == State::Ready;
                }
                ensures {
                    ext2_filesystem_allocated(self);
                    ext2_filesystem_driver_bound(self, Ext2Driver);
                    ext2_filesystem_volume_bound(self, Ext2Volume);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ext2_filesystem_allocated(self);
            ext2_filesystem_driver_bound(self, Ext2Driver);
            ext2_filesystem_volume_bound(self, Ext2Volume);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Ext2Driver.state == State::Ready;
                    Ext2Volume.state == State::Ready;
                    BlockDeviceRegistry.state == State::Ready;
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_filesystem_targets_block_device(self, BlockDevice);
                    ext2_filesystem_superblock_read(self, BufferHead);
                    ext2_filesystem_magic_valid(self);
                    ext2_filesystem_block_size_supported(self);
                    ext2_filesystem_block_size_1024_or_2048_or_4096(self);
                    ext2_filesystem_group_desc_read(self, BufferHead);
                    ext2_filesystem_operations_bound(self, Ext2Driver);
                    ext2_filesystem_root_dentry_bound(self);
                    ext2_inode_record_ready(self, Ext2InodeRef::Root);
                    ext2_inode_number_bound(self, Ext2InodeRef::Root);
                    ext2_inode_mode_bound(self, Ext2InodeRef::Root);
                    ext2_inode_size_bound(self, Ext2InodeRef::Root);
                    ext2_inode_direct_blocks_bound(self, Ext2InodeRef::Root);
                    ext2_inode_is_root_dir(self, Ext2InodeRef::Root);
                    ext2_inode_indirect_blocks_deferred(self, Ext2InodeRef::Root);
                    ext2_filesystem_ready(self);
                    ext2_filesystem_page_cache_deferred(self);
                    ext2_filesystem_write_paths_deferred(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ext2_filesystem_allocated(self);
            ext2_filesystem_driver_bound(self, Ext2Driver);
            ext2_filesystem_volume_bound(self, Ext2Volume);
            ext2_filesystem_magic_valid(self);
            ext2_filesystem_block_size_supported(self);
            ext2_filesystem_block_size_1024_or_2048_or_4096(self);
            ext2_filesystem_operations_bound(self, Ext2Driver);
            ext2_filesystem_root_dentry_bound(self);
            ext2_inode_record_ready(self, Ext2InodeRef::Root);
            ext2_inode_is_root_dir(self, Ext2InodeRef::Root);
            ext2_filesystem_ready(self);
            ext2_filesystem_page_cache_deferred(self);
            ext2_filesystem_write_paths_deferred(self);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    VfsCore.state == State::Ready;
                    Ext2FileSystem.state == State::Ready;
                    ext2_filesystem_root_dentry_bound(self);
                }
                drives {
                    VfsCore.Action::MountExt2At;
                }
                ensures {
                    ext2_filesystem_mount_boundary_recorded(self);
                    ext2_filesystem_vfs_mount_bound(self, VfsCore);
                    ext2_filesystem_vfs_lookup_entry_bound(self);
                    ext2_filesystem_vfs_read_entry_bound(self);
                    ext2_filesystem_page_cache_deferred(self);
                    ext2_filesystem_write_paths_deferred(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            ext2_filesystem_allocated(self);
            ext2_filesystem_ready(self);
            ext2_filesystem_mount_boundary_recorded(self);
            ext2_filesystem_vfs_mount_bound(self, VfsCore);
            ext2_filesystem_vfs_lookup_entry_bound(self);
            ext2_filesystem_vfs_read_entry_bound(self);
            ext2_filesystem_page_cache_deferred(self);
            ext2_filesystem_write_paths_deferred(self);
        }

        actions {
            Action::LookupRootName {
                state_effect: StateEffect::None;
                depends_on {
                    Ext2FileSystem.state == State::Online;
                    ext2_inode_is_root_dir(self, Ext2InodeRef::Root);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_root_lookup_name_bound(self, Ext2DirEntryRef::RootLookup);
                    ext2_root_lookup_reads_root_dir(self, Ext2InodeRef::Root);
                    ext2_root_lookup_scans_direct_blocks(self, Ext2InodeRef::Root);
                    ext2_root_lookup_multi_direct_block_supported(self);
                    ext2_root_lookup_multi_direct_block_observed(self);
                    ext2_root_lookup_not_found_is_nonfatal(self);
                    ext2_root_lookup_indirect_blocks_deferred(self);
                    ext2_root_lookup_dirent_valid(self, Ext2DirEntryRef::RootLookup);
                    ext2_inode_record_ready(self, Ext2InodeRef::LookupFile);
                    ext2_inode_number_bound(self, Ext2InodeRef::LookupFile);
                    ext2_inode_mode_bound(self, Ext2InodeRef::LookupFile);
                    ext2_inode_size_bound(self, Ext2InodeRef::LookupFile);
                    ext2_inode_direct_blocks_bound(self, Ext2InodeRef::LookupFile);
                    ext2_inode_is_regular_file(self, Ext2InodeRef::LookupFile);
                    ext2_inode_indirect_blocks_deferred(self, Ext2InodeRef::LookupFile);
                    ext2_root_lookup_returns_inode(self, Ext2DirEntryRef::RootLookup, Ext2InodeRef::LookupFile);
                }
            }

            Action::ReadLookupFile {
                state_effect: StateEffect::None;
                depends_on {
                    Ext2FileSystem.state == State::Online;
                    ext2_inode_is_regular_file(self, Ext2InodeRef::LookupFile);
                    ext2_inode_direct_blocks_bound(self, Ext2InodeRef::LookupFile);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_file_read_uses_direct_block(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_scans_direct_blocks(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_multi_direct_block_supported(self, Ext2FileReadRef::LookupFile);
                    ext2_file_read_uses_buffer_head(self, Ext2FileReadRef::LookupFile, BufferHead);
                    ext2_file_read_copies_to_caller(self, Ext2FileReadRef::LookupFile);
                    ext2_file_read_len_matches_inode_size(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_indirect_blocks_deferred(self, Ext2FileReadRef::LookupFile);
                }
            }

            Action::ReadVfsFile {
                state_effect: StateEffect::None;
                depends_on {
                    Ext2FileSystem.state == State::Online;
                    ext2_filesystem_vfs_read_entry_bound(self);
                    ext2_inode_is_regular_file(self, Ext2InodeRef::LookupFile);
                    ext2_inode_direct_blocks_bound(self, Ext2InodeRef::LookupFile);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_file_read_entered_from_vfs(self, Ext2FileReadRef::LookupFile);
                    ext2_file_read_uses_direct_block(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_scans_direct_blocks(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_multi_direct_block_supported(self, Ext2FileReadRef::LookupFile);
                    ext2_file_read_uses_buffer_head(self, Ext2FileReadRef::LookupFile, BufferHead);
                    ext2_file_read_copies_to_caller(self, Ext2FileReadRef::LookupFile);
                    ext2_file_read_len_matches_inode_size(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_indirect_blocks_deferred(self, Ext2FileReadRef::LookupFile);
                }
            }

            Action::RejectLookupFileShortBuffer {
                state_effect: StateEffect::None;
                depends_on {
                    Ext2FileSystem.state == State::Online;
                    ext2_inode_is_regular_file(self, Ext2InodeRef::LookupFile);
                    ext2_inode_size_bound(self, Ext2InodeRef::LookupFile);
                }
                ensures {
                    ext2_file_read_short_buffer_rejected(self, Ext2FileReadRef::LookupFile);
                }
            }
        }
    }
}
