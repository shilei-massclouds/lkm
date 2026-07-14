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
