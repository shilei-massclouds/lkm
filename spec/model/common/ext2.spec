/*
 * Read-only ext2 first slice.
 *
 * Only Ext2Type and Ext2Mount carry lifecycle here:
 * - Ext2Type represents the Linux file_system_type/mount callback descriptor.
 * - Ext2Mount represents one successful ext2_fill_super() result.
 *
 * Superblock fields, group descriptor, inodes, dirents and file-read results
 * are structured records/facts owned by Ext2Mount. They are not lifecycle
 * objects in this first slice; later inode cache, iget refcounting, eviction
 * or VFS mount integration may promote some of them if needed.
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

predicate ext2_type_declared<T>(fs_type: T) -> bool;
predicate ext2_type_read_only<T>(fs_type: T) -> bool;
predicate ext2_type_mount_callback_bound<T>(fs_type: T) -> bool;

predicate ext2_mount_allocated<T>(mount: T) -> bool;
predicate ext2_mount_targets_block_device<T, D>(mount: T, device: D) -> bool;
predicate ext2_mount_superblock_read<T, B>(mount: T, bh: B) -> bool;
predicate ext2_mount_magic_valid<T>(mount: T) -> bool;
predicate ext2_mount_block_size_supported<T>(mount: T) -> bool;
predicate ext2_mount_block_size_1024_or_2048_or_4096<T>(mount: T) -> bool;
predicate ext2_mount_group_desc_read<T, B>(mount: T, bh: B) -> bool;
predicate ext2_mount_ready<T>(mount: T) -> bool;
predicate ext2_mount_vfs_integration_deferred<T>(mount: T) -> bool;
predicate ext2_mount_page_cache_deferred<T>(mount: T) -> bool;
predicate ext2_mount_write_paths_deferred<T>(mount: T) -> bool;

predicate ext2_inode_record_ready<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_number_bound<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_mode_bound<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_size_bound<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_direct_blocks_bound<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_is_root_dir<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_is_regular_file<T, I>(mount: T, inode: I) -> bool;
predicate ext2_inode_indirect_blocks_deferred<T, I>(mount: T, inode: I) -> bool;

predicate ext2_root_lookup_name_bound<T, D>(mount: T, dirent: D) -> bool;
predicate ext2_root_lookup_reads_root_dir<T, I>(mount: T, root: I) -> bool;
predicate ext2_root_lookup_dirent_valid<T, D>(mount: T, dirent: D) -> bool;
predicate ext2_root_lookup_returns_inode<T, D, I>(mount: T, dirent: D, inode: I) -> bool;

predicate ext2_file_read_uses_direct_block<T, R, I>(mount: T, read: R, file: I) -> bool;
predicate ext2_file_read_uses_buffer_head<T, R, B>(mount: T, read: R, bh: B) -> bool;
predicate ext2_file_read_copies_to_caller<T, R>(mount: T, read: R) -> bool;
predicate ext2_file_read_len_matches_inode_size<T, R, I>(mount: T, read: R, file: I) -> bool;

object Ext2Type: ResourceObject {
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
                    ext2_type_declared(self);
                    ext2_type_read_only(self);
                    ext2_type_mount_callback_bound(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ext2_type_declared(self);
            ext2_type_read_only(self);
            ext2_type_mount_callback_bound(self);
        }
    }
}

object Ext2Mount: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Ext2Type.state == State::Ready;
                    BlockDeviceRegistry.state == State::Ready;
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_mount_allocated(self);
                    ext2_mount_targets_block_device(self, BlockDevice);
                    ext2_mount_superblock_read(self, BufferHead);
                    ext2_mount_magic_valid(self);
                    ext2_mount_block_size_supported(self);
                    ext2_mount_block_size_1024_or_2048_or_4096(self);
                    ext2_mount_group_desc_read(self, BufferHead);
                    ext2_inode_record_ready(self, Ext2InodeRef::Root);
                    ext2_inode_number_bound(self, Ext2InodeRef::Root);
                    ext2_inode_mode_bound(self, Ext2InodeRef::Root);
                    ext2_inode_size_bound(self, Ext2InodeRef::Root);
                    ext2_inode_direct_blocks_bound(self, Ext2InodeRef::Root);
                    ext2_inode_is_root_dir(self, Ext2InodeRef::Root);
                    ext2_inode_indirect_blocks_deferred(self, Ext2InodeRef::Root);
                    ext2_mount_ready(self);
                    ext2_mount_vfs_integration_deferred(self);
                    ext2_mount_page_cache_deferred(self);
                    ext2_mount_write_paths_deferred(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ext2_mount_allocated(self);
            ext2_mount_magic_valid(self);
            ext2_mount_block_size_supported(self);
            ext2_mount_block_size_1024_or_2048_or_4096(self);
            ext2_inode_record_ready(self, Ext2InodeRef::Root);
            ext2_inode_is_root_dir(self, Ext2InodeRef::Root);
            ext2_mount_ready(self);
            ext2_mount_vfs_integration_deferred(self);
            ext2_mount_page_cache_deferred(self);
            ext2_mount_write_paths_deferred(self);
        }

        actions {
            Action::LookupRootName {
                state_effect: StateEffect::None;
                depends_on {
                    Ext2Mount.state == State::Ready;
                    ext2_inode_is_root_dir(self, Ext2InodeRef::Root);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_root_lookup_name_bound(self, Ext2DirEntryRef::RootLookup);
                    ext2_root_lookup_reads_root_dir(self, Ext2InodeRef::Root);
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
                    Ext2Mount.state == State::Ready;
                    ext2_inode_is_regular_file(self, Ext2InodeRef::LookupFile);
                    ext2_inode_direct_blocks_bound(self, Ext2InodeRef::LookupFile);
                }
                drives {
                    BufferHead.Action::SbBreadByMajorMinor;
                }
                ensures {
                    ext2_file_read_uses_direct_block(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                    ext2_file_read_uses_buffer_head(self, Ext2FileReadRef::LookupFile, BufferHead);
                    ext2_file_read_copies_to_caller(self, Ext2FileReadRef::LookupFile);
                    ext2_file_read_len_matches_inode_size(self, Ext2FileReadRef::LookupFile, Ext2InodeRef::LookupFile);
                }
            }
        }
    }
}
