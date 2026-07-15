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

## LTP rootfs overlay

Rule IDs (MUST):

- `arceos_ex_must_user_boot_default_to_ltp_rootfs_overlay`
- `arceos_ex_must_ltp_rootfs_overlay_apply_after_fixture_and_file_overlays`
- `arceos_ex_must_ltp_rootfs_overlay_validate_staging_before_copy`
- `arceos_ex_must_ltp_rootfs_overlay_use_dedicated_sized_image`
- `arceos_ex_must_automated_rootfs_workflows_disable_ltp_overlay`

`ROOTFS_LTP_OVERLAY` accepts only `default` and `none`. It defaults to `default` for `APP=user-boot` and
to `none` for every other app. `default` consumes an already unpacked sibling-repository staging tree;
`ROOTFS_LTP_OVERLAY_DIR` defaults to `../ltp/build-riscv64-musl-syscalls/rootfs` relative to the repository
root. Image construction must fail with a specific diagnostic when that directory is absent, when
`opt/ltp/run-syscalls.sh` is absent, or when the runner is not executable. It must not silently fall back
to a rootfs without LTP.

The LTP staging tree is copied with the same semantics as `ROOTFS_FILE_OVERLAY_DIR`, but in a dedicated
final overlay stage after the compiled fixture and ordinary file overlay. This order lets LTP staging
provide its intended final files without changing the fixture and static-file interfaces.

An enabled LTP overlay defaults to the separate `build/virtio-blk-ltp.raw` image and a `320M` image size.
With LTP disabled, the defaults remain `build/virtio-blk.raw` and `64M`. Existing images are still reused
according to the deterministic disk lifecycle rule, so a rebuilt or replaced LTP staging tree requires
`FORCE=1` to refresh an existing image. Repository `make test`, stress, and difftest user-boot image
commands must explicitly set `ROOTFS_LTP_OVERLAY=none`; they retain their existing fixture selection,
image sizes, execution scope, and independence from the sibling LTP build.

## Kernel command line and fixture separation

Rule IDs (MUST):

- `arceos_ex_must_qemu_append_default_user_boot_to_bin_sh_and_passthrough`
- `arceos_ex_must_keep_overlay_as_fixture_injection_after_init_cmdline_support`

`QEMU_APPEND` defaults to `earlycon=sbi` for ordinary apps and to `earlycon=sbi init=/bin/sh` for a manual
`APP=user-boot` run. The run target passes the value through to QEMU and preserves command-line overrides.
`init=` selects the Linux-like requested-init/fallback path; it is not a replacement for image overlay.
The overlay remains the stable injection mechanism for `user_smoke` and staged fixtures.
