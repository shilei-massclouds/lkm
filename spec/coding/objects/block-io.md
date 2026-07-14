# Block I/O coding

本文件是 Block I/O、只读 Ext2、rootfs fixture 与相关差分入口的权威 coding 规格。迁移自旧索引的稳定 rule ID、原 `ArceosExBlockIoCodingMust` type 分组和 MUST 层级在此保留；这些 ID 用于评审和追踪，不是 pyveri predicate。旧索引 invariant 中 8 个未配套顶层 predicate 声明的 ID 也按其有效 MUST 使用点完整保留。

## Rule catalog

### ArceosExBlockIoCodingMust

#### Linux-like block I/O adapter

Rule ID: `arceos_ex_must_block_io_model_bio_buffer_head_before_ext2` (MUST).

Before read-only ext2 is introduced, the first filesystem-facing
block I/O surface must be modeled as Bio, submit_bio_wait(),
minimal blk_mq_submit_bio(), and BufferHead/sb_bread(). It must not
introduce BlockReadRequest or BlockIoBuffer as substitute Linux
top-level objects.

#### Registry read role

Rule ID: `arceos_ex_must_block_io_registry_read_remain_lower_level_adapter` (MUST).

BlockDeviceRegistry::read_default()/read_by_devt(), or equivalent
direct registry reads, are a lower-level synchronous adapter below
submit_bio_wait(). They may continue to perform default/dev_t lookup
and provider dispatch, but higher filesystem-facing paths should
enter through Bio/BufferHead rather than treating registry reads as
the public block layer.

#### Smoke entry

Rule ID: `arceos_ex_must_block_io_smoke_use_sb_bread_path` (MUST).

App smoke coverage for the current ext2-superblock read must use
sb_bread()/BufferHead over submit_bio_wait(). It may still validate
registry facts produced underneath, but it must not bypass the new
block I/O adapter by directly calling registry read APIs.

#### BufferHead storage

Rule ID: `arceos_ex_must_buffer_head_data_not_be_large_stack_storage` (MUST).

BufferHead may carry up to 4KiB ext2 blocks, so its data payload
must not be embedded as a large stack-allocated array or returned
through nested stack frames. The BufferHead object should own
heap-backed or equivalent exclusive dynamic storage for block data.

#### Read-only ext2 first slice

Rule ID: `arceos_ex_must_ext2_first_slice_read_only_and_buffer_head_based` (MUST).

The next filesystem step must model and implement the Linux
ext2_fill_super()/ext2_iget()/ext2_find_entry()/direct-block read
shape over BufferHead. It must remain read-only and must not bypass
the Bio/BufferHead layer by calling VirtioBlkDevice private reads.

#### Ext2 object model

Rule ID: `arceos_ex_must_ext2_model_driver_volume_filesystem_lifecycle` (MUST).

Ext2Driver must replace the older Ext2Type role and hold the
filesystem driver/operation-set facts. Ext2Volume must model the
on-disk ext2 volume discovered through BlockDeviceRegistry and
BufferHead; absence or invalid layout is an ordinary non-fatal
result. Ext2FileSystem must model the mounted in-memory filesystem
instance: Preset depends on Ext2Volume, Setup expands metadata/root
entry facts, and Enable mounts the read-only ext2 instance into
VFS through the minimal mount-to-parent boundary.

#### 4K Buffer / ext2 block-size support

Rule ID: `arceos_ex_must_ext2_support_4k_buffer_and_block_sizes` (MUST).

The next ext2 step must raise BufferHead and virtio-blk read buffers
to at least 4KiB, and Ext2Volume/Ext2FileSystem must accept ext2
block_size values 1024, 2048 and 4096. The superblock is still
discovered at byte
offset 1024; after parsing it, group descriptor and inode/data block
reads must use the actual filesystem block size and block-number
layout. make disk must not force 1KiB blocks by default.

#### Direct-block read path generalization

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

#### Stable Alpine smoke targets

Rule ID: `arceos_ex_must_ext2_smoke_use_stable_alpine_rootfs_files` (MUST).

make disk must construct the ext2 image from the configured Alpine
minirootfs tarball instead of creating smoke-only files. The ext2
smoke must keep using the read-only Ext2FileSystem path through
BufferHead, and must choose stable files from that rootfs, including
a regular file whose size spans more than one ext2 block so the
multi-direct-block path is actually observed.

#### Build-time rootfs overlay

Rule IDs (MUST):

- `arceos_ex_must_rootfs_overlay_copy_fixture_outputs_at_image_build`
- `arceos_ex_must_rootfs_overlay_config_allow_none_and_target_overrides`
- `arceos_ex_must_rootfs_overlay_read_default_map_unless_disabled`
- `arceos_ex_must_rootfs_overlay_user_tests_live_under_tests_user`
- `arceos_ex_must_rootfs_overlay_user_tests_build_via_dedicated_makefile`
- `arceos_ex_must_rootfs_overlay_user_tests_select_toolchain_and_link_mode`
- `arceos_ex_must_user_probe_print_per_syscall_success_marker`
- `arceos_ex_must_user_probe_cover_directory_openat_getdents64`
- `arceos_ex_must_user_syscall_analysis_use_existing_static_tools`
- `arceos_ex_must_user_syscall_analysis_stay_out_of_default_build_path`
- `arceos_ex_must_user_syscall_analysis_mark_busybox_candidates_conservative`
- `arceos_ex_must_user_syscall_vfs_specs_reference_linux_6_12`
- `arceos_ex_must_disk_build_default_not_rebuild_existing_image`
- `arceos_ex_must_qemu_append_default_user_boot_to_bin_sh_and_passthrough`
- `arceos_ex_must_test_harness_pin_user_smoke_qemu_append`
- `arceos_ex_must_test_harness_cover_no_overlay_bin_ls`
- `arceos_ex_must_test_harness_cover_no_overlay_bin_sh_with_host_input`
- `arceos_ex_must_rootfs_file_overlay_apply_after_fixture_overlay`
- `arceos_ex_must_rc_local_test_use_inittab_direct_marker_only`
- `arceos_ex_must_rc_local_difftest_be_default_case`
- `arceos_ex_must_rc_local_difftest_report_hard_scope_coverage_counts`
- `arceos_ex_must_openrc_login_test_use_explicit_account_overlay`
- `arceos_ex_must_keep_shell_external_commands_and_native_init_diagnostic_until_specified`
- `arceos_ex_must_keep_overlay_as_fixture_injection_after_init_cmdline_support`

The current temporary user init fixture is supplied by a rootfs
image-construction overlay, not by runtime overlayfs. The overlay
operation MUST copy a built fixture output into the staged rootfs
target path; if the target exists, including a symlink, the target
path itself must be replaced. The default temporary target is
/sbin/init. ROOTFS_OVERLAY=none MUST skip overlay processing;
otherwise the kernel Makefile MUST read ROOTFS_OVERLAY_MAP, whose
default is impl/arceos_ex/tests/user/rootfs-overlay.map. Each
non-comment map row declares a rootfs target path, user test name,
and optional toolchain/link mode; omitted toolchain/link fields use
ROOTFS_OVERLAY_TOOLCHAIN and ROOTFS_OVERLAY_LINK defaults. The
special user-test token __absent__ deletes the target path from the
staging rootfs instead of building or copying a fixture; it is only
for explicit negative/fallback images, not for the default overlay.
User-mode overlay test programs MUST live under
impl/arceos_ex/tests/user/ and MUST be built through a dedicated
user-test Makefile, so the kernel Makefile does not own user-mode
compiler details. The default checked-in map MUST be the only
persistent overlay map and MUST install user_smoke as /sbin/init.
user_smoke lives under impl/arceos_ex/tests/user/smoke/, where
smoke.c owns main() and calls subtests such as fileio and
sh_probe. Fallback-specific maps that delete /sbin/init, /etc/init
or /bin/init MUST be generated as temporary harness/manual inputs,
not kept as long-lived checked-in maps. The user-test Makefile MUST
expose toolchain and link-mode selection for GNU vs musl GCC and
static vs dynamic linking. Staged syscall probe subtests such as
sh_probe must print an explicit success marker after each syscall
path they validate, so guest output distinguishes a loaded fixture
from per-syscall support. Any user-test build failure MUST abort
overlay processing and disk construction immediately; the Makefile
must not continue with a stale fixture output. The user_smoke
framework output MUST use the "user-smoke:" prefix, print begin/end
markers for the whole run and each case, include "status=N" on every
end marker, and use blank lines to separate the run and case
boundaries. The host harness still determines pass/fail from "user
exit status=N", not from these human-readable markers. The
directory-enumeration
sh_probe slice must use the Linux 6.12/RISC-V syscall ABI directly for
openat(AT_FDCWD, "/", O_RDONLY|O_DIRECTORY) and getdents64(61),
validate linux_dirent64 records, and then close the directory fd. A
failing probe is diagnostic evidence for the first unsupported
point, not permission to infer the cause without checking the
implementation and Linux reference.
Distribution command probes such as /bin/ls must first follow a
local static/semi-static analysis flow using existing tools such as
file, readelf, objdump, local RISC-V Linux syscall headers, and
optional read-only sysroot path inspection. This flow records target
ELF identity, optional guest symlink resolution, PT_INTERP, program
headers, dynamic section, dynamic symbols, visible ecall/a7 evidence,
and mapped syscall names in a temporary note. It must not introduce a
custom analysis tool, generated long-lived manifest, Makefile target,
rootfs staging rebuild, overlay staging change, or default
disk/run/test construction change unless a later reviewed spec
explicitly requires that engineering investment.
For BusyBox applets, whole-binary dynamic symbols are conservative
candidates and must be marked as such; the temporary analysis note
must not claim them as the applet's complete runtime syscall trace
without a later path-sensitive or guest validation step.
Each syscall/VFS item promoted from analysis into model/coding specs
must be checked against the local Linux 6.12 reference tree at
../linux-6.12, including the RISC-V syscall table/header and the
concrete fs/open.c, fs/readdir.c, fs/stat.c, fs/file.c and fs/namei.c
entry points relevant to the item. Any first-slice omission of Linux
locking, RCU, permission, mount namespace, LSM or errno behavior must
be recorded as trimmed/deferred before implementation.
make disk must only create the disk image when it is missing by
default; overlay configuration changes must not silently rebuild an
existing disk image. Explicit rebuild remains a command decision
through FORCE=1 or disk-clean.

ROOTFS_FILE_OVERLAY_DIR is a separate static file overlay for distro
acceptance fixtures. It defaults to empty. When set, the Makefile
must copy that directory's contents into the staged rootfs after the
Alpine tarball is unpacked and after the existing compiled fixture
overlay has run. ROOTFS_OVERLAY=none only disables the compiled
fixture map; without ROOTFS_FILE_OVERLAY_DIR, the bare Alpine
minirootfs account state, including locked root entries such as
root:*, must remain unchanged. OpenRC getty/login shell acceptance
must use an explicit static account overlay to provide a loginable
test account, preferably a dedicated test user. If BusyBox login
rejects an empty password, the overlay may use a checked-in
test-only password hash, and the host harness must wait for the
Password: marker before sending that test password. Kernel code must
not bypass authentication.

QEMU_APPEND defaults to "earlycon=sbi" for ordinary APPs. For a
manual APP=user-boot run, the default MUST be
"earlycon=sbi init=/bin/sh" so plain "make run APP=user-boot"
enters the distro BusyBox shell through the Linux-like requested-init
branch. The run target must pass QEMU_APPEND through to QEMU as
-append "$(QEMU_APPEND)", and an explicit command-line QEMU_APPEND
must continue to override the default; for example,
"earlycon=sbi init=/bin/ls" selects /bin/ls, while "earlycon=sbi"
lets the fallback list select /sbin/init.

The test harness MUST NOT inherit the manual user-boot /bin/sh
default for the user_smoke fixture. make test user-boot smoke cases
must pass a case-local disk image, the default overlay map, FORCE=1,
and QEMU_APPEND="earlycon=sbi" explicitly so they exercise the
overlay-installed /sbin/init fixture and remain non-interactive.

make test MUST also include distro rootfs smoke entries that are
separate from overlay fixtures. The first distro smoke runs
ROOTFS_OVERLAY=none with QEMU_APPEND="earlycon=sbi init=/bin/ls" on
a case-local disk and treats user exit status 0 as success. A second
distro smoke may run ROOTFS_OVERLAY=none with init=/bin/sh by waiting
for a visible shell-ready marker and then feeding a bounded host-side
stdin script such as "echo OK\nexit\n"; this input belongs to the host
harness, not to kernel-side ready-data fixtures, and it must not be
injected before early serial diagnostics and initcalls have finished.
The case must require both user exit status 0 and the expected output
marker. init= is not a substitute for rootfs overlay, which remains
the stable fixture injection mechanism for user_smoke and staged
probes.

rc.local first-stage acceptance is a focused diagnostic, not a
default make test gate. It must use ROOTFS_OVERLAY=none with a
checked-in ROOTFS_FILE_OVERLAY_DIR that replaces /etc/inittab and
/etc/rc.local only. The inittab entry must directly execute
/bin/sh /etc/rc.local as a BusyBox init sysinit action, then the host
harness may terminate QEMU successfully after observing the ordered
script markers, including the rootfs listing marker. This first
slice does not claim OpenRC local.d, /sbin/openrc service graph,
daemon supervision, runlevel completion, script binfmt, or the full
PID1 lifecycle.

make difftest's default case must be this rc.local
direct-inittab paired checkpoint baseline. The older distro
init=/bin/sh delayed /bin/ls baseline must remain available by
explicitly setting DIFFTEST_CASE to that TOML path, and the same
Makefile variable may list multiple case paths. If the Linux side
needs a post-marker /sbin/poweroff -f to dump its checkpoint buffer,
that dump-only exec must be excluded from hard checkpoint comparison
by explicit count limits rather than folded into the rc.local
acceptance claim. This default case must also enable exact
checkpoint coverage audit: checkpoint_scope remains the hard
comparison scope, not the full latest exact mapping set, and every
exact checkpoint must either be in scope or registered in explicit
outside-scope accounting. Reports must state the hard-scope result
separately from coverage counts.

OpenRC getty/login shell closure is a distinct acceptance slice from
the bare ROOTFS_OVERLAY=none distro image. The test harness may use
ROOTFS_OVERLAY=none together with ROOTFS_FILE_OVERLAY_DIR pointing
at a checked-in account-only overlay, then drive login and shell
input in ordered host-side delayed-stdin stages: wait for login,
provide the test account (and blank password when needed), wait for
the BusyBox shell prompt, then run /bin/ls and exit. The case must
require user exit status 0 and a stable rootfs marker such as
lost+found. This harness input must not be injected before the
relevant prompt marker and must not be replaced by kernel-side
ready-data fixtures.

The current focused run has advanced past login/password input and
authentication. The old reproducible boundary was BusyBox login's
post-auth clone(220) vfork while the existing OpenRC/getty child
still occupied the single active child slot. The nested
vfork/child-slot slice is now specified here. The following
fd-local fchown(55)/fchmod(52) slice is also specified and focused
trace confirms both syscalls return 0. The new post-auth boundary is
socket(198) with AF_UNIX/SOCK_STREAM/SOCK_CLOEXEC/protocol 0; the
guest text still says "login: can't set groups: Function not
implemented", but local asm-generic confirms 198 is socket and 159
is setgroups, so that text is not a reliable syscall name for the
next slice. The current socket slice is limited to fd creation; the
focused rerun records connect(203) as the new first boundary after
socket returns fd 3. The current connect slice advances only the
pathname failure errno case: it copies no more than the Linux
sockaddr_storage boundary, accepts only valid AF_UNIX pathname
sockaddr_un values on UnixSocket0, routes existence lookup through
current FsStruct.root without opening or allocating FileRef, follows
existing fast symlinks and . / .. path components only as pathname
lookup semantics for cases such as /var/run -> ../run, returns
ENOENT for missing pathname and ECONNREFUSED for existing pathname
with no socket/listener model, and keeps all other connect/sendto
shapes as ENOSYS diagnostics. That nscd pathname miss now returns
ENOENT; setgroups(159) now returns 0 for the observed
gidsetsize=1 shape, setgid(144) with gid=100 returns 0, and the
post-auth direct boundary is setuid(146) with uid=1000 returning
EPERM before this slice. The current bounded setuid slice accepts
that root-euid transition. The focused rerun after this slice
confirms setuid(146, uid=1000) returns 0, login reaches MOTD output,
and later execve diagnostics identify the login shell EFAULT as
fail_stage=filename_copy / fail_reason=filename_copy. The following
execve C-string copy slice closes that boundary with per-byte
readability; its focused rerun reaches the login shell prompt and
records the next boundary as shell /bin/ls clone(220) unsupported
plus job-control errors, not as part of the setuid change. Current
focused evidence has since crossed that job-control slice and now
identifies login-shell getgroups(158) readback as the next direct
boundary before the prompt.
The OpenRC login shell focused case is an explicit opt-in diagnostic
asset selected through STRESS_CASES; it must not enter the default
make test gate or the default make test-stress suite. After that
behavior is closed, the staged-input case should become the
login-shell acceptance gate with /bin/ls, lost+found and user exit
status 0 as pass criteria.

#### Directory path lookup

Rule ID: `arceos_ex_must_ext2_lookup_support_path_components_from_directories` (MUST).

Since the rootfs is no longer a handcrafted root directory fixture,
VFS/ext2 lookup must support path components below arbitrary ext2
directories backed by direct blocks. Smoke must not require artificial
root-directory filler entries just to move a dirent into a later
block.

#### Minimal VFS read-only mount

Rule ID: `arceos_ex_must_ext2_support_minimal_vfs_read_only_mount` (MUST).

The next ext2 step must let VfsCore mount a prepared Ext2FileSystem
at a normal VFS dentry and route lookup/open/read through VFS before
dispatching to Ext2FileSystem's BufferHead-backed direct-block
backend. Smoke must read the deterministic ext2 file through that
VFS mount path instead of treating direct Ext2FileSystem calls as
the acceptance boundary.

#### Minimal pathname walk/read

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

#### Long-term observation checkpoints

Rule IDs (MUST):

- `arceos_ex_must_long_term_checkpoints_follow_model_coding_contracts`
- `arceos_ex_must_payload_vfs_ext2_read_emit_observation_checkpoints`
- `arceos_ex_must_block_io_task_wait_checkpoints_cover_submit_wait_and_timeout`
- `arceos_ex_must_block_io_irq_completion_checkpoints_cover_begin_end_failure`
- `arceos_ex_must_block_io_completion_source_distinguish_irq_and_task_poll`
- `arceos_ex_must_read_path_error_classification_checkpoint_be_structured`

Checkpoints used by nightly/stress longitudinal comparison and
Linux-like cross comparison must be specified in model/coding
before implementation. The first batch covers the user payload
image read, VFS path read, ext2 lookup/read, block task-side
submit/wait, and virtio-blk completion source. Failure and timeout
observations must use structured classes instead of temporary log
text so repeated event sequences can be grouped and compared.

#### Virtio-blk synchronous request lifecycle

Rule ID: `arceos_ex_must_virtio_blk_sync_reads_submit_wait_complete_before_return` (MUST).

The current read-only block path is synchronous from Bio/
BufferHead's perspective. Every live virtio-blk read must submit one
request, wait for that exact pending token to complete through IRQ
or bounded task-side polling, verify the status byte, release the
descriptor chain, and return to the caller only after the request is
no longer pending. A later read must not merely spin on an inherited
pending flag; it must first converge that inherited request or fail
with a structured block I/O error.

#### Initcall superblock probe convergence

Rule ID: `arceos_ex_must_virtio_blk_initcall_superblock_probe_converge_before_ready` (MUST).

The initcall-time virtio-blk ext2 superblock probe is the first
production read request. It must complete before VirtioBlkReady and
before RootfsPhase/VFS/ext2 consumers can issue their own reads.
This follows Linux's request lifecycle shape where a request handed
to the queue is eventually ended before synchronous callers proceed,
rather than leaving a fire-and-forget used-ring entry for a later
phase to inherit.

#### Single completion consumer

Rule ID: `arceos_ex_must_virtio_blk_completion_consumer_be_single_owner` (MUST).

IRQ completion and task-side polling are both valid observation
sources, but the same pending token may be consumed only once. The
implementation must guard the virtqueue/device/static read-buffer
completion path so an IRQ handler and the waiting task cannot race
through get_buf/status validation/descriptors release for the same
request.

#### Virtqueue memory ordering

Rule IDs (MUST):

- `arceos_ex_must_virtqueue_publish_avail_before_notify_with_release_order`
- `arceos_ex_must_virtqueue_observe_used_with_acquire_order`

Publishing a descriptor chain must order descriptor and avail-ring
stores before avail idx and MMIO notify. Observing a used-ring idx
from the device must acquire-order subsequent reads of the used
element, status byte and data buffer. The current static coherent
backing keeps cache maintenance deferred, but it must not omit these
ordering boundaries.

#### Deferred ext2 scope

Rule ID: `arceos_ex_must_ext2_defer_page_cache_indirect_and_writes` (MUST).

Page cache/folios, indirect blocks, slow symlinks, permissions,
xattrs, quotas, allocation, writes and remount/error recovery
remain explicit deferred scope in this slice. Fast symlink target
extraction from inline i_block bytes is part of the current VFS
path-walk slice and must not be implemented by treating the symlink
inode as a regular file.
