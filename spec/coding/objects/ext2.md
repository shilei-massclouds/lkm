# Ext2 coding

本文件是 `spec/model/objects/ext2.spec` 的权威 coding 映射，保留从旧
`ArceosExBlockIoCodingMust` 分组迁入的稳定 rule ID 和 MUST 层级。

## Read-only first slice

Rule ID: `arceos_ex_must_ext2_first_slice_read_only_and_buffer_head_based` (MUST).

The first filesystem slice must model and implement the Linux
ext2_fill_super()/ext2_iget()/ext2_find_entry()/direct-block read
shape over BufferHead. It must remain read-only and must not bypass
the Bio/BufferHead layer by calling VirtioBlkDevice private reads.

## Object model

Rule ID: `arceos_ex_must_ext2_model_driver_volume_filesystem_lifecycle` (MUST).

Ext2Driver must replace the older Ext2Type role and hold the
filesystem driver/operation-set facts. Ext2Volume must model the
on-disk ext2 volume discovered through BlockDeviceRegistry and
BufferHead; absence or invalid layout is an ordinary non-fatal
result. Ext2FileSystem must model the mounted in-memory filesystem
instance: Preset depends on Ext2Volume, Setup expands metadata/root
entry facts, and Enable mounts the read-only ext2 instance into
VFS through the minimal mount-to-parent boundary.

## 4K Buffer and block-size support

Rule ID: `arceos_ex_must_ext2_support_4k_buffer_and_block_sizes` (MUST).

BufferHead and virtio-blk read buffers must support at least 4KiB,
and Ext2Volume/Ext2FileSystem must accept ext2 block_size values
1024, 2048 and 4096. The superblock is still discovered at byte
offset 1024; after parsing it, group descriptor and inode/data block
reads must use the actual filesystem block size and block-number
layout. make disk must not force 1KiB blocks by default.

## Direct and single-indirect reads

Rule IDs (MUST):

- `arceos_ex_must_ext2_read_path_support_multi_direct_blocks`
- `arceos_ex_must_ext2_read_path_support_single_indirect_blocks`

Ext2FileSystem directory lookup must scan direct blocks of the
current ext2 directory inode until a matching dirent is found or the
direct range is exhausted. Ext2FileSystem file read must read a
regular file across multiple direct blocks up to inode size, reject
too-small caller buffers with ShortBuffer, and support the first
single-indirect block for read-only regular files. Double/triple
indirect blocks, allocation and writes remain explicit deferred
scope.

## Stable Alpine smoke targets

Rule ID: `arceos_ex_must_ext2_smoke_use_stable_alpine_rootfs_files` (MUST).

make disk must construct the ext2 image from the configured Alpine
minirootfs tarball instead of creating smoke-only files. The ext2
smoke must keep using the read-only Ext2FileSystem path through
BufferHead, and must choose stable files from that rootfs, including
a regular file whose size spans more than one ext2 block so the
multi-direct-block path is actually observed.

## Directory path lookup

Rule ID: `arceos_ex_must_ext2_lookup_support_path_components_from_directories` (MUST).

Since the rootfs is no longer a handcrafted root directory fixture,
VFS/ext2 lookup must support path components below arbitrary ext2
directories backed by direct blocks. Smoke must not require artificial
root-directory filler entries just to move a dirent into a later
block.

## Deferred scope

Rule ID: `arceos_ex_must_ext2_defer_page_cache_indirect_and_writes` (MUST).

Page cache/folios, double/triple indirect blocks, slow symlinks,
permissions, xattrs, quotas, allocation, writes and remount/error
recovery remain explicit deferred scope in this slice. Fast symlink
target extraction from inline i_block bytes is part of the current
VFS path-walk slice and must not be implemented by treating the
symlink inode as a regular file.
