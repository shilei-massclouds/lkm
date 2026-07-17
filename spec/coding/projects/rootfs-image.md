# Rootfs image construction coding

本文件是 rootfs 镜像构造、fixture overlay 与 QEMU init 选择的权威 coding 规格。保留从旧
`ArceosExBlockIoCodingMust` 分组迁入的稳定 rule ID 和 MUST 层级。运行期 VFS/Ext2 规则分别见
[`../objects/vfs.md`](../objects/vfs.md) 与 [`../objects/ext2.md`](../objects/ext2.md)，验收编排见
[`../../testing/rootfs.md`](../../testing/rootfs.md)。

## Canonical rootfs and stable fixture paths

Rule IDs (MUST):

- `arceos_ex_must_rootfs_overlay_copy_fixture_outputs_at_image_build`
- `arceos_ex_must_rootfs_overlay_config_allow_none_and_target_overrides`
- `arceos_ex_must_rootfs_overlay_read_default_map_unless_disabled`
- `arceos_ex_must_rootfs_overlay_user_tests_live_under_tests_user`
- `arceos_ex_must_rootfs_overlay_user_tests_build_via_dedicated_makefile`
- `arceos_ex_must_rootfs_overlay_user_tests_select_toolchain_and_link_mode`

The canonical image is a normal Alpine/OpenRC rootfs: image construction must not replace `/sbin/init`,
`/etc/inittab` or the distribution `/etc`. Repository user programs live under
`impl/arceos_ex/tests/user/`, are compiled through that directory's Makefile, and are installed under stable
`/opt/lkm/tests/` names. User-mode basic tests select them with an absolute kernel `init=` value. The user-test
Makefile continues to expose GNU/musl and static/dynamic selection, and any build failure aborts image
construction instead of reusing stale fixture output.

The old target-map and arbitrary file-overlay mechanisms remain available only to unmigrated stress,
rc.local, OpenRC login and paired-difftest paths. They are not inputs to the canonical image and new basic
tests must not use them to replace a system entry point.

## Input-sensitive disk lifecycle

Rule ID: `arceos_ex_must_disk_build_default_not_rebuild_existing_image` (MUST).

`make disk` records a canonical input fingerprint. It rebuilds when the Alpine tarball, rootfs configuration,
builder implementation, repository fixture sources/build files or configured LTP staging tree changes.
Unchanged inputs reuse the existing image and `FORCE=1` forces a rebuild. A successful rebuild atomically
replaces the canonical image and its fingerprint; failure must not publish a partially built image.

## Legacy static file overlay order

Rule ID: `arceos_ex_must_rootfs_file_overlay_apply_after_fixture_overlay` (MUST).

`ROOTFS_FILE_OVERLAY_DIR` defaults to empty. When set, its contents are copied after the Alpine tarball is
unpacked and after compiled fixture overlay processing. `ROOTFS_OVERLAY=none` disables only the compiled
map; without a file overlay the bare Alpine account state, including locked `root:*`, remains unchanged.

## Canonical LTP installation

Rule IDs (MUST):

- `arceos_ex_must_user_boot_default_to_ltp_rootfs_overlay`
- `arceos_ex_must_ltp_rootfs_overlay_apply_after_fixture_and_file_overlays`
- `arceos_ex_must_ltp_rootfs_overlay_validate_staging_before_copy`
- `arceos_ex_must_ltp_rootfs_overlay_use_dedicated_sized_image`
- `arceos_ex_must_automated_rootfs_workflows_disable_ltp_overlay`

The canonical builder consumes an unpacked LTP staging tree whose `opt/ltp/run-syscalls.sh` exists and is
executable. It installs that tree at `/opt/ltp` after Alpine extraction and repository fixtures, without
overwriting Alpine `/etc`. A missing or malformed LTP source is a specific build failure, not a silent lean
image. The canonical size must accommodate the configured LTP tree. Changes anywhere in that source are
part of the canonical fingerprint.

Legacy specialized images may still explicitly enable/disable the old LTP overlay while they await
migration; that behavior does not redefine the canonical image.

## Kernel command line and fixture separation

Rule IDs (MUST):

- `arceos_ex_must_qemu_append_default_user_boot_to_bin_sh_and_passthrough`
- `arceos_ex_must_keep_overlay_as_fixture_injection_after_init_cmdline_support`

Basic-test `kernel_cmdline` is fixed in TOML and passed as one QEMU append value. `init=` selects the
Linux-like requested-init/fallback path. Repository fixtures are chosen through stable absolute paths such
as `/opt/lkm/tests/user-smoke`; distribution commands continue to use their normal `/bin` or `/sbin` paths.
Make command-line QEMU cmdline overrides are forbidden for basic tests.
