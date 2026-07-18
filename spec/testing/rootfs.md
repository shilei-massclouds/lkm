# Rootfs and user-mode acceptance testing

本文件是 rootfs/user fixture、发行版 smoke、OpenRC 输入编排和 checkpoint difftest 的权威测试规格。
镜像构造见 [`../coding/projects/rootfs-image.md`](../coding/projects/rootfs-image.md)；基本测试生命周期见
[`basic-tests.md`](basic-tests.md)；对象行为见 `spec/coding/objects/` 对应文档。

## User fixture output and analysis

Rule IDs (MUST):

- `arceos_ex_must_user_probe_print_per_syscall_success_marker`
- `arceos_ex_must_user_smoke_ecall_preserve_callee_saved_registers`
- `arceos_ex_must_user_probe_cover_directory_openat_getdents64`
- `arceos_ex_must_user_syscall_analysis_use_existing_static_tools`
- `arceos_ex_must_user_syscall_analysis_stay_out_of_default_build_path`
- `arceos_ex_must_user_syscall_analysis_mark_busybox_candidates_conservative`
- `arceos_ex_must_user_syscall_vfs_specs_reference_linux_6_12`

Staged syscall subtests print an explicit success marker after each validated path. The user-smoke wrapper
uses the `user-smoke:` prefix, begin/end/status markers and blank-line case separation; the host also checks
`user exit status=N`. Directory probing uses the RISC-V ABI for openat/getdents64 validation and close. The
fixture performs an explicit `ecall` with sentinels in all callee-saved integer registers and requires both
the syscall result and sentinels to survive the ordinary user trap round trip.

The libc-linked smoke requires its auxv/argv/HWCAP/credential/random/stack probes. Object smoke separately
covers the modeled ASLR, auxv, stack alignment, entropy rollback and stack fault/snapshot boundaries.
Distribution-command analysis uses existing static host tools first. BusyBox whole-binary symbols are only
conservative candidates; promoted syscall/VFS behavior is checked against local Linux 6.12 and records
locking/RCU/permission/namespace/LSM/errno omissions before implementation.

## Basic user and distro smoke

Rule IDs (MUST):

- `arceos_ex_must_test_harness_pin_user_smoke_qemu_append`
- `arceos_ex_must_test_harness_cover_no_overlay_bin_ls`
- `arceos_ex_must_test_harness_cover_no_overlay_bin_sh_with_host_input`
- `arceos_ex_must_keep_shell_external_commands_and_native_init_diagnostic_until_specified`

The user-smoke basic TOML uses a private canonical copy and `init=/opt/lkm/tests/user-smoke`; requested-init
selects `/opt/lkm/tests/init-hello`. Distribution cases attach canonical read-only: one runs `init=/bin/ls`,
and one uses scripted interaction to send `/bin/ls`, `/bin/ls`, `exit` after the BusyBox prompt. The latter
requires both listings, no visible unsupported/panic marker and guest exit 0. Host input is orchestration,
not kernel-side ready data.

Kernel object smoke reads the real `/opt/lkm/tests/user-smoke` ELF. It may assert fallback ordering but cannot
claim `/sbin/init` was replaced. DF-0003 repeats the two-command shell configuration. The exact paired shell
baseline sends the same structured delayed input to both sides and retains the complete announce stream.

`user-smoke-native` and `user-smoke-linux-object` are the only user-smoke acceptance identities. They remain
non-interactive, boot `/opt/lkm/tests/user-smoke`, and enter the default regression gate explicitly by TEST
name. DF-0001 likewise repeats `TEST=user-smoke-native`; it must not depend on a TTY, the APP variable-name
alias, or a retired test-name mapping.

`shell-native` and `shell-linux-object` are separate terminal diagnostic identities. Each uses a writable
private copy of the canonical template, boots `init=/bin/sh`, uses a real PTY, has a 3600-second overall
limit, and forbids the panic marker. A normal `exit` completes execution with verdict inconclusive; cleanup
must restore the terminal and delete the private image. They are opt-in and never enter default regression
or formal difftest. This shell identity remains independent of the LTP manual configurations below even when
their current QEMU behavior is similar.

## rc.local basic acceptance and paired difftest

Rule IDs (MUST):

- `arceos_ex_must_rc_local_use_canonical_elf_launcher`
- `arceos_ex_must_rc_local_be_dual_provider_basic_acceptance`
- `arceos_ex_must_rc_local_difftest_be_default_case`
- `arceos_ex_must_rc_local_difftest_report_hard_scope_coverage_counts`

Canonical `/opt/lkm/tests/rc-local-init` is a static ELF launcher. With fixed argv/envp it only execs
`/bin/sh /opt/lkm/tests/rc-local.sh`; exec failure prints a stable diagnostic and exits nonzero. The script
runs `/bin/ls /` and emits ordered `lkm-rc-local: begin`, rootfs listing and `end status=0` markers. It does
not replace or edit `/etc`.

`rc-local-native` and `rc-local-linux-object` are automatic acceptance tests with
`init=/opt/lkm/tests/rc-local-init`. They require the script markers, clean guest outcome, and absence of panic,
visible unsupported or launcher-failure markers. Both enter default `make test`.

The same launcher path is the default paired difftest. Linux and arceos_ex each consume a private copy of
the canonical template; neither side constructs an overlay or changes inittab. The checkpoint hard scope is
based on the observed paired run through launcher -> shell -> `/bin/ls`, not the deleted direct-inittab path.
Because the launcher is static, `UserBoot.InterpreterReady` is explicitly outside hard scope; three matched
`UserExec.*` groups cover launcher-to-shell, `/bin/ls`, and `poweroff`.
Every required exact checkpoint is either in scope or explicitly accounted outside it; reports retain
`required_total`, `in_scope`, `accounted_outside_scope` and `unaccounted` counts.

## OpenRC login basic acceptance

Rule IDs (MUST):

- `arceos_ex_must_canonical_rootfs_include_locked_root_and_test_account`
- `arceos_ex_must_openrc_login_be_dual_provider_basic_acceptance`

The canonical image retains the distribution `/sbin/init` and `/etc/inittab`, keeps root locked, and includes
the stable non-root `test/test` account. No test-stage account overlay is permitted. `openrc-login-native` and
`openrc-login-linux-object` explicitly boot `init=/sbin/init` from a private canonical copy. Scripted input
waits in order for `login:`, `Password:` and the non-root shell prompt before sending fixed `/bin/ls`/`exit`.

Acceptance requires the Alpine/OpenRC greeting, login/password/prompt steps, non-root credential observation,
successful pending-child `setpgid`, foreground-pgrp handling, `lost+found`, guest exit 0, and no panic,
unsupported or `reason=pid_not_visible` marker. Both providers enter default `make test`. The focused stress
case repeats this basic-test entry; it does not build an account overlay. This bounded closure does not claim a
general task graph, multiple pending children, post-exec parent setpgid, job-control signals or full pgrp lookup.

## Manual LTP shell basic tests

`ltp-shell-manual-native` and `ltp-shell-manual-linux-object` are terminal diagnostic configurations with
`init=/bin/sh` and a private canonical copy. They are opt-in and never enter default regression or formal
difftest. A normal terminal session ends with verdict inconclusive. The initial manual workflow is:

```sh
cd /opt/ltp
./run-syscalls.sh 'getpid*' 'uname*'
```

This establishes only that the installed LTP tree can be booted and invoked. Individual FAIL/BROK/TCONF or
missing `/proc`/`sys`/device capabilities do not expand acceptance scope. Build/manifest may validate either
provider without a TTY; `run` requires a real terminal.

## Composite and observation policy

Current composite tests are stress repetition and automatic paired difftest. Checkpoint KUnit is a basic
checkpoint-callback profile, not a composite test. A formal paired side always has closed stdin or structured
delayed stdin; terminal interaction is forbidden. Historical wording “manual paired” is normalized to
“opt-in paired”.

Long-term checkpoints follow model/coding contracts and cover payload/VFS/Ext2 reads, block submit/wait,
completion source and structured errors. Non-interactive capture uses DEVNULL unless structured input is
declared; checkpoint handlers only read production facts and write their sink.
