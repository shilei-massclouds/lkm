# Rootfs image construction coding

本文件是 rootfs 镜像构造、fixture overlay 与 QEMU init 选择的权威 coding 规格。保留从旧
`ArceosExBlockIoCodingMust` 分组迁入的稳定 rule ID 和 MUST 层级。运行期 VFS/Ext2 规则分别见
[`../objects/vfs.md`](../objects/vfs.md) 与 [`../objects/ext2.md`](../objects/ext2.md)，验收编排见
[`../../testing/rootfs.md`](../../testing/rootfs.md)。

## Build-time fixture overlay

Rule IDs (MUST):

- `arceos_ex_must_rootfs_overlay_copy_fixture_outputs_at_image_build`
- `arceos_ex_must_rootfs_overlay_config_allow_none_and_target_overrides`
- `arceos_ex_must_rootfs_overlay_read_default_map_unless_disabled`
- `arceos_ex_must_rootfs_overlay_user_tests_live_under_tests_user`
- `arceos_ex_must_rootfs_overlay_user_tests_build_via_dedicated_makefile`
- `arceos_ex_must_rootfs_overlay_user_tests_select_toolchain_and_link_mode`

The temporary user init fixture is supplied by image construction, not runtime overlayfs. The operation
must copy a built fixture into the staged rootfs target, replacing the target itself when it is a symlink.
`ROOTFS_OVERLAY=none` skips the map; otherwise the Makefile reads `ROOTFS_OVERLAY_MAP`, defaulting to
`impl/arceos_ex/tests/user/rootfs-overlay.map`. Each non-comment row declares target, user-test name and
optional toolchain/link mode. `__absent__` deletes the target only for explicit negative/fallback images.

User programs live under `impl/arceos_ex/tests/user/` and are built by that directory's Makefile. The
kernel Makefile does not own compiler details. The checked-in default map installs `user_smoke` as
`/sbin/init`; fallback-specific maps are temporary harness/manual inputs. The user-test Makefile exposes
GNU/musl and static/dynamic selection, and any build failure aborts image construction instead of reusing
a stale output.

## Deterministic disk lifecycle

Rule ID: `arceos_ex_must_disk_build_default_not_rebuild_existing_image` (MUST).

`make disk` creates an image only when it is missing by default. Overlay changes do not silently rebuild
an existing image; explicit rebuild uses `FORCE=1` or `disk-clean` followed by `make disk`.

## Static file overlay order

Rule ID: `arceos_ex_must_rootfs_file_overlay_apply_after_fixture_overlay` (MUST).

`ROOTFS_FILE_OVERLAY_DIR` defaults to empty. When set, its contents are copied after the Alpine tarball is
unpacked and after compiled fixture overlay processing. `ROOTFS_OVERLAY=none` disables only the compiled
map; without a file overlay the bare Alpine account state, including locked `root:*`, remains unchanged.

## Kernel command line and fixture separation

Rule IDs (MUST):

- `arceos_ex_must_qemu_append_default_user_boot_to_bin_sh_and_passthrough`
- `arceos_ex_must_keep_overlay_as_fixture_injection_after_init_cmdline_support`

`QEMU_APPEND` defaults to `earlycon=sbi` for ordinary apps and to `earlycon=sbi init=/bin/sh` for a manual
`APP=user-boot` run. The run target passes the value through to QEMU and preserves command-line overrides.
`init=` selects the Linux-like requested-init/fallback path; it is not a replacement for image overlay.
The overlay remains the stable injection mechanism for `user_smoke` and staged fixtures.
