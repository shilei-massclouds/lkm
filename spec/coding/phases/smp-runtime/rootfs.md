# RootfsPhase coding

RootfsPhase 是 SmpRuntimePhase 的第 5 个直接子阶段，由 KernelInitTask 执行。model 路径为
`spec/model/phases/smp-runtime/rootfs/`，实现落点为
`impl/arceos_ex/src/phases/smp_runtime/rootfs.rs`。

## 生命周期映射

`preset()` 依赖 InitcallPhase 精确 Online；依赖接受后立即发出 RootfsPhase.Started，再从
KUnitRuntimeTrimmed 开始驱动全部对象并提交 Prepared。因此 Started 表示 Rootfs Preset/KUnit
入口，不再表示 `prepare_namespace()` 前边界。`RamdiskExecuteCommand.EaccessCheckpoint` 继续位于
RootfsConsole 之后、prepare_namespace 分类与 RootFS.Enable 之前。Setup/Enable 只检查 rootfs
结果并提交 Ready/Online；Online 只返回 `smp_runtime::enable_after_rootfs()`。

#### Entry gate

RootfsPhase must run after InitcallPhase.Online and preserve the
kunit_run_all_tests() entry position inside RootfsPhase.
Its RootfsPhase.Started paired-diff checkpoint maps to the Linux
kunit_run_all_tests() call at the phase entrance. The later
RamdiskExecuteCommand.EaccessCheckpoint remains the explicit
prepare_namespace pre-boundary.

#### KUnit runtime

CONFIG_KUNIT=n in the current Linux-like configuration, so
kunit_run_all_tests() must be encoded as a trimmed/no-op object
inside RootfsPhase, not as a standalone KUnitPhase.

#### Deferred initramfs and console details

wait_for_initramfs() and console_on_rootfs() must preserve their
Linux order but remain deferred in this round. wait_for_initramfs()
must explicitly preserve Linux's async cookie/domain wait boundary;
console_on_rootfs() must preserve the /dev/console and PID 1 fd
duplication position without pretending the file path is implemented.

#### Required branch checkpoint

init_eaccess(ramdisk_execute_command) must force the supported
Linux-like path toward prepare_namespace().

#### RootFS enable

prepare_namespace() must be represented by RootFS.Transition::Enable in
this round, not by a separate enable-position object. It must use the
already mounted DevFs and the
BlockDeviceRegistry default device as the root device candidate,
construct the Ext2Driver/Ext2Volume/Ext2FileSystem chain, and mount
that ext2 filesystem at Linux's temporary /root mount point.
The initial ramfs-backed rootfs mount belongs to ProcessPreparePhase
vfs_caches_init()/mnt_init(), not to this RootfsPhase enable step.

#### prepare_namespace() path classification

RootfsPhase must keep prepare_namespace() as one formal subphase
event, but it must still expose a structured
RootfsPrepareNamespacePaths-style fact set for the Linux calls around
the supported block-root path. The implementation and smoke observer
must distinguish:

- root_delay/rootwait as cmdline-absent trimmed paths, with the
  root_wait polling protocol deferred rather than erased;
- wait_for_device_probe() as deferred, including probe_count atomic,
  probe_waitqueue, deferred_probe_work flush, and worker interaction;
- md_run_setup() as deferred under CONFIG_MD=y;
- initrd_load() as trimmed under CONFIG_BLK_DEV_INITRD=n;
- NFS root as deferred under CONFIG_ROOT_NFS=y but inactive for the
  current root device, CIFS root as trimmed when CONFIG_CIFS_ROOT=n,
  and nodev root as deferred;
- Linux CONFIG_EXT2_FS=n / CONFIG_EXT4_USE_FOR_EXT2=y versus the
  arceos_ex Ext2Driver substitution;
- devtmpfs_mount() as deferred under CONFIG_DEVTMPFS=y, while the
  current arceos_ex DevFs is not remounted below the new ext2 root.

#### Root switch

After the Linux-like temporary /root ext2 staging mount,
RootFS.Transition::Enable must drive VfsCore.Action::MoveMountToRoot and
FsStruct.Action::ChrootDot as separate model actions. FsStruct owns
the task-visible root/pwd dentry refs; VfsCore must not keep a
parallel current-root singleton. Smoke must observe current root as
ext2 and read /etc/alpine-release directly without remounting /dev
under the new root.

#### Integrity keys

integrity_load_keys() must preserve CONFIG_INTEGRITY=y timing but
keep IMA/EVM keyring and certificate loading details deferred. The
current CONFIG_IMA=n and CONFIG_EVM=n trimmed facts must be visible
in model and implementation checks.

#### Boundary

RootfsBoundary must mark the next boundary as FinalizePhase.
