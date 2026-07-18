# Rootfs image construction coding

本文件是正式 rootfs profile、fixture 安装和 input manifest 的权威 coding 规格。运行期对象规则见
`spec/coding/objects/`，验收编排见 [`../../testing/rootfs.md`](../../testing/rootfs.md)。

## Profiles and construction boundary

Rule IDs (MUST):

- `arceos_ex_must_rootfs_profile_have_complete_builder`
- `arceos_ex_must_reject_unknown_rootfs_profile`
- `arceos_ex_must_keep_basic_runner_out_of_rootfs_construction`

`make disk ROOTFS=<profile>` is independent of test execution and produces a read-only reusable template.
`ROOTFS` defaults to `canonical`; it is currently the only registered profile. Each registered profile maps
to one complete construction script responsible for extraction, configuration, fixture/LTP installation,
mkfs, atomic publication and input manifest. Unknown profiles fail immediately. The basic runner may only
verify the template manifest and make a private copy; it must never call a builder or apply an overlay.

## Canonical distribution and fixtures

Rule IDs (MUST):

- `arceos_ex_must_rootfs_overlay_copy_fixture_outputs_at_image_build`
- `arceos_ex_must_rootfs_overlay_user_tests_live_under_tests_user`
- `arceos_ex_must_rootfs_overlay_user_tests_build_via_dedicated_makefile`
- `arceos_ex_must_rootfs_overlay_user_tests_select_toolchain_and_link_mode`
- `arceos_ex_must_canonical_keep_distribution_busybox_init`
- `arceos_ex_must_canonical_install_deterministic_inittab`
- `arceos_ex_must_canonical_merge_stable_test_account`

Canonical is an Alpine minirootfs with the distribution BusyBox `/sbin/init` and ordinary `/etc`. Construction
must replace the minirootfs inittab (whose OpenRC commands reference a package not present in minirootfs) with
a checked-in deterministic BusyBox-init profile. The profile contains no `/sbin/openrc` reference, starts the
minimum numeric-TTY and serial gettys needed by arceos_ex and reference Linux QEMU, and retains a reboot action.
Construction keeps `root:*` locked and idempotently merges the stable non-root `test/test` account and password
hash. No rc.local-specific inittab exists.

Repository programs live below `impl/arceos_ex/tests/user/`, build through that directory's Makefile, and are
installed below `/opt/lkm/tests`. Current fixtures include user-smoke, init-hello, nolibc probes and the static
`rc-local-init` ELF. Canonical configuration installs `/opt/lkm/tests/rc-local.sh`. The launcher has fixed
argv/envp and only invokes that script through `/bin/sh`; failure prints a stable diagnostic and exits nonzero.
Any fixture failure aborts construction rather than reusing stale output.

Legacy arbitrary target-map/file-overlay mechanisms may remain only while other specialized stress assets
consume them. rc.local/BusyBox-init basic, focused stress and paired difftest must not use those mechanisms;
their obsolete `/etc` overlays are deleted.

## Input-sensitive template lifecycle

Rule ID: `arceos_ex_must_disk_build_default_not_rebuild_existing_image` (MUST).

The canonical input manifest records content hashes for the complete builder, Alpine tarball, canonical
configuration, repository fixtures/build inputs, configured LTP tree and relevant tool/config settings.
Unchanged inputs reuse image+manifest; any input change or `FORCE=1` rebuilds. Construction occurs in private
staging/image/manifest paths and only a successful mkfs publishes replacements. Failure must leave the old
template usable and must not publish partial output.

Runner validation recomputes the same current-input manifest without invoking the builder. Missing image,
missing/malformed manifest or fingerprint mismatch is “missing/stale template” and directs the operator to
`make disk ROOTFS=canonical`.

## Canonical LTP installation

Rule IDs (MUST):

- `arceos_ex_must_ltp_rootfs_overlay_validate_staging_before_copy`
- `arceos_ex_must_ltp_rootfs_overlay_use_dedicated_sized_image`

Canonical consumes an unpacked LTP staging tree whose `opt/ltp/run-syscalls.sh` exists and is executable. It
installs the full tree at `/opt/ltp` after Alpine extraction and repository fixtures without overwriting
Alpine `/etc`. Missing/malformed LTP is a specific failure. The configured image size must accommodate it and
all LTP input content participates in the manifest.

## Kernel command line separation

Basic-test `kernel_cmdline` is frozen in TOML and passed as one QEMU append value. `init=` selects a normal
distribution binary or stable `/opt/lkm/tests` fixture. Make command-line cmdline/rootfs-overlay overrides are
forbidden for basic tests.
