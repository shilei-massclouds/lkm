# VFS, devfs and files coding

本文件共同承载 `vfs.spec` 与 `devfs.spec` 的权威 coding 映射，并说明
`files.spec` 中 VFS/file backend 与 `user-boot.md` 的边界。多个 model 文件的唯一归属见
[`README.md`](README.md)。

`VfsCore` owns filesystem type registration, the initial ramfs root, mount operations and pathname
resolution. `DevFs` is mounted only after the initial root exists and hwrng/block providers are
discoverable. It creates namespace-visible nodes without adding test-only read backdoors or claiming
complete device file operations, uevent/sysfs, permissions or devtmpfs-thread behavior.

`FilesStruct`/fd table/OFD/syscall dispatch are owned by [`user-boot.md`](user-boot.md); this file owns
the VFS `File`, pathname, mount, dentry/inode and backend contract consumed by that syscall layer.

## Minimal VFS read-only mount

Rule ID: `arceos_ex_must_ext2_support_minimal_vfs_read_only_mount` (MUST).

VfsCore must let a prepared Ext2FileSystem mount at a normal VFS
dentry and route lookup/open/read through VFS before dispatching to
Ext2FileSystem's BufferHead-backed read-only backend. Smoke must read
the deterministic ext2 file through that VFS mount path instead of
treating direct Ext2FileSystem calls as the acceptance boundary.

## Transient namespace overlay on a read-only mount

Rule ID: `arceos_ex_must_vfs_publish_transient_directories_without_writing_ext2` (MUST).

`VfsCore` may satisfy the first writable-namespace slice by attaching a
memory-backed directory or regular-file inode/dentry to the live VFS child list
of an ext2 directory. Lookup must consult that live child list before the ext2
backend, and the transient inode must not carry an ext2 backing binding. This
is a namespace overlay only: it must never issue an ext2 write or claim
persistent block-backed mutation.

Before publishing a new child, the implementation must resolve the parent and
check both the live child cache and ext2 backing for an existing final name.
It must reserve inode-table, dentry-table and parent-child-list capacity before
the first visible mutation. Any reservation failure returns `NoSpace` with no
live dentry or inode published. The pathname-facing create operation holds the
shared files/fs/VFS IRQ-safe resource lock for the entire resolve/check/commit
transaction.

Exclusive regular-file creation uses that same transaction. It checks the live
child list and the ext2 backing before publication and returns `AlreadyExists`
for either positive result. It reserves the VFS file-table slot together with
the inode, dentry and parent-child capacities before publishing the child. The
new inode is regular, empty and memory-backed; its type bits are fixed to
`S_IFREG`, its low `07777` bits come from the syscall mode, and uid/gid come
from the current aggregate's fsuid/fsgid snapshot. No post-publication
allocation is permitted on this path.

Every materialized inode carries uid/gid and full mode metadata. Transient nodes start with the creating root identity in the
current bounded slice; a successful path ownership change mutates both requested fields in one VFS operation and
keeps a field unchanged for the 32-bit all-ones sentinel. The operation rejects read-only-backed inodes and does
not partially update either field on lookup or validation failure. Path stat exposes the stored ids so the side
effect is observable rather than a harness-only success response.

A transient pathname chmod follows the ordinary final symlink, rejects read-only-backed inodes, preserves the
stored file-type bits and replaces only the low `07777` mode bits. Validation completes before mutation, and path
stat plus VFS-backed file stat expose the stored mode. Permission enforcement for unrelated read/write/open paths
remains deferred; this metadata mutation does not claim the complete Linux ACL, idmapped-mount, LSM or ctime model.

Path-based filesystem statistics follow the ordinary final symlink, then resolve the positive dentry's inode and
superblock rather than classifying a pathname string. The ext2 first slice may return metadata only when that
superblock is the mounted ext2 instance and the supplied `Ext2FileSystem` is ready. Its type, block size and total
block/inode counts come from the bound filesystem objects; the name limit comes from the VFS implementation.
Unavailable free-space/free-inode accounting remains zero, and ramfs/devfs return the existing unsupported class.

The number and allocation of regular-file open descriptions are not VFS
namespace semantics; they remain owned by `user-boot.md`. VFS creation receives
one already-reserved caller file slot and does not infer or reuse an fd/OFD.

`VfsCore::truncate_file` accepts only a live memory-backed regular `FileRef`.
It rejects read-only/ext2, removed and non-regular inodes before mutation,
reserves any required vector growth, then resizes data and inode size in one
guarded commit. Extension bytes are zero, shrink discards the suffix, and the
`File` position is unchanged. The bounded maximum is the syscall layer's fixed
64 KiB regular-file capacity; allocation failure is `NoSpace` and leaves the
inode unchanged.

`VfsCore::read_file_range` is the non-positioned read used to seed a file
mapping. It accepts a live memory-backed regular `FileRef`, validates the
requested offset range without mutating either the inode or `File.position`,
copies the available inode bytes, and zero-fills the caller's remaining range.
Validation failure has no VFS side effect. This does not introduce a general
page cache or make ext2 mappings writable.

`VfsCore::remove_path` resolves the parent and final component while the caller
holds the shared files/fs/VFS IRQ-safe resource lock. It admits only a live
transient memory-backed inode. File removal rejects a directory as
`IsDirectory`; directory removal rejects a non-directory as `NotDirectory` and
a nonempty directory as `DirectoryNotEmpty`; a read-only/ext2-backed target is
`ReadOnly`. All type, backing and emptiness checks complete before the parent
child list or target dentry changes. Commit detaches the child and makes its
name unreachable, but retains the inode and file-table objects so an already
open `FileRef` remains usable and an already materialized shared mapping keeps
its frame. Positioned read, ordinary read/write, truncate and file stat through
an existing `FileRef` therefore do not reject an inode merely because its
pathname was removed. Repeated pathname lookup returns `NotFound`.

## Minimal pathname walk/read

Rule ID: `arceos_ex_must_vfs_support_minimal_absolute_path_walk_and_read` (MUST).

VfsCore must support absolute path walk from current root, direct
component lookup, mount crossing, open-by-path and read-by-path for
the currently modeled ramfs/devfs/ext2 subset. Symlink follow must
keep the Linux 6.12 namei skeleton: detect S_IFLNK, obtain the link
target, restart absolute targets at FsStruct.root, restart relative
targets at the symlink parent, preserve remaining path components,
and cap follow count at MAXSYMLINKS == 40 with an ELOOP-like error.
The current implementation slice may limit the backend to read-only
ext2 fast symlinks whose target lives in raw i_block bytes and is
truncated by inode size. readlinkat is the separate no-follow final
symlink operation and may use the same fast-symlink target source
while preserving the Linux 6.12 copy/truncate contract. The current
cwd-relative slice also supports FsStruct.pwd starts for ".", single
relative components, and "./component"; newfstatat must accept
AT_SYMLINK_NOFOLLOW for the final component and return fast symlink
metadata without following it. Slow symlink page/block reads, magic
links, RCU walk, permissions, mount namespace, full dotdot, arbitrary
multi-component relative paths and complete errno semantics remain
trimmed/deferred. Relative dirfd paths, permissions, full fd tables
and page cache remain deferred.
