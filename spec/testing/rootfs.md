# Rootfs and user-mode acceptance testing

本文件是 rootfs/user fixture、发行版 smoke、BusyBox init 输入编排和 checkpoint difftest 的权威测试规格。
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
and one uses scripted interaction to send `echo "OK"`, `ls`, `ls /lib`, `ls /`, `exit` after the BusyBox
prompt. The latter requires one standalone `OK` output line, one `ld-musl-riscv64.so.1`, two `lost+found`
occurrences, no visible unsupported/panic marker and guest exit 0. The standalone-line check must not accept
the echoed `echo "OK"` command. Host input is orchestration, not kernel-side ready data.

Kernel object smoke reads the real `/opt/lkm/tests/user-smoke` ELF. It may assert fallback ordering but cannot
claim `/sbin/init` was replaced. The historically named DF-0003 case repeats `scripted-shell` and its complete
five-line command sequence. The exact paired shell baseline remains independent: it sends the existing
structured delayed input to both sides and retains the complete announce stream.

`user-smoke-native` and `user-smoke-linux-object` are the only user-smoke acceptance identities. They remain
non-interactive, boot `/opt/lkm/tests/user-smoke`, and enter the default regression gate explicitly by TEST
name. DF-0001 likewise repeats `TEST=user-smoke-native`; it must not depend on a TTY, the APP variable-name
alias, or a retired test-name mapping.

`scripted-shell` and `scripted-shell-lo` are the default scripted shell acceptance identities. They use the
native and linux-object providers, respectively, attach the canonical template read-only, wait for `~ #`,
send exactly `echo "OK"\nls\nls /lib\nls /\nexit\n`, and apply a 120-second limit. Both enter default
`make test`. The removed public spellings `script-shell`, `script-shell-lo`, `distro-sh-native` and
`distro-sh-linux-object` are unknown tests; they have no alias, retired-name mapping or alias TOML.

`shell` and `shell-lo` are their provider-matched terminal diagnostic counterparts. In each pair, the
user-boot app, provider, release profile, canonical template initial contents, 128 MiB memory, eight vCPUs,
`earlycon=sbi init=/bin/sh`, RNG device and guest-shutdown policy are the same. The terminal identity instead
uses a writable private copy, a real PTY with no scripted stdin steps, and a 3600-second limit. A normal
`exit` completes execution with verdict inconclusive; cleanup must restore the terminal and delete the
private image. Terminal identities are opt-in and never enter default regression or formal difftest. The
scripted result proves only its fixed command loop in the shared startup environment; it does not prove that
the pipe and PTY transports, or arbitrary terminal input, are behaviorally equivalent. These shell identities
remain independent of the LTP scripted acceptance below.

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

## BusyBox init login basic acceptance

Rule IDs (MUST):

- `arceos_ex_must_canonical_rootfs_include_locked_root_and_test_account`
- `arceos_ex_must_busybox_init_login_be_dual_provider_basic_acceptance`

The canonical image retains the distribution BusyBox `/sbin/init`, replaces the minirootfs inittab with the
checked-in deterministic BusyBox-init profile, keeps root locked, and includes the stable non-root `test/test`
account. The profile must not reference `/sbin/openrc`; it runs one numeric-TTY getty for arceos_ex and one
serial getty for reference Linux QEMU. No test-stage account overlay is permitted.
`busybox-init-login-native` and `busybox-init-login-linux-object` explicitly boot `init=/sbin/init` from a
private canonical copy. Scripted input
waits in order for `login:`, `Password:` and the non-root shell prompt before sending fixed `/bin/ls`/`exit`.

Acceptance requires the Alpine greeting, login/password/prompt steps, non-root credential observation,
successful pending-child `setpgid`, foreground-pgrp handling, `lost+found`, guest exit 0, and no panic,
unsupported, init child exec failure or `reason=pid_not_visible` marker. Both providers enter default `make test`. The focused stress
case repeats this basic-test entry; it does not build an account overlay. This bounded closure does not claim a
general task graph, multiple pending children, post-exec parent setpgid, job-control signals or full pgrp lookup.

`busybox-init-checkpoints` and `busybox-init-checkpoints-linux` are standalone diagnostic basic tests for the
same `/sbin/init` profile. Each side must independently complete, satisfy its expectations, emit a complete
non-overflowing `stress_mem` record containing `SyscallTable.Wait4`, and clean up its private image before the
paired case may compare them. The hard scope ends at the first stable `SyscallTable.Wait4`; it represents
BusyBox init waiting/reaping, not OpenRC behavior. The arceos_ex side captures the checkpoint stream with
`announce,stress-mem`; syscall trace/error probes are outside this differential observation and must not consume
the bounded checkpoint buffer. Its declared recorder capacity is 524288 bytes, which must retain the complete
session without overflow rather than truncating the sequence at the comparison boundary.

## LTP syscall list acceptance

Rule IDs (MUST):

- `arceos_ex_must_ltp_close_list_be_dual_provider_scripted_acceptance`
- `arceos_ex_must_ltp_close_list_exact_entries`
- `arceos_ex_must_ltp_close_list_exit_with_list_status`
- `arceos_ex_must_ltp_list_command_substitution_preserve_outer_wait_snapshot`
- `arceos_ex_must_ltp_list_grandchild_smoke_use_independent_scenario_stack`

`ltp` and `ltp-lo` are scripted acceptance identities for the native and linux-object providers,
respectively. The retired `ltp-shell-manual-native` and `ltp-shell-manual-linux-object` names have no
compatibility entry. Both use `init=/bin/sh`, a writable private copy of the canonical template and guest
shutdown, and both enter the default root `make test` gate.

After the first shell prompt, the scripted payload changes to `/opt/ltp` and runs exactly
`./run-syscalls.sh --list -- 'close*'`. The shell captures that command's status and exits with the same
status. The list must contain exactly one entry each for `close01` and `close02`, with their matching
commands; other entries selected by the `close*` pattern are outside the marker contract. Acceptance requires guest status 0 and no unsupported syscall,
ENOSYS/Function-not-implemented, panic or LTP non-pass marker. The payload must not invoke
`./run-syscalls.sh -- 'close*'` or directly execute either selected binary. This list-only gate proves LTP
discovery and the bounded shell/list integration path; it does not claim that `close(57)`, `close01`,
`close02`, the LTP runtime harness, or any other LTP syscall test executes successfully.

The list command's BusyBox command substitution is covered by the bounded two-level plain-fork slice: the
PID1-originated script child may create one builtin-only grandchild for `cd`/`pwd`, pipe/stdio and exit. Tests
must cover distinct outer/inner snapshot ownership, blocking parent-read handoff, child-write/parent-read pipe data, two-level wait/exit
restore, sequential pid monotonicity, illegal clone arguments, a second pending child, deeper nesting,
builtin-grandchild `cd`/absolute `pwd`, `/dev/null` stderr redirection, and the canonical 33,110-byte runtest list
read. A separate builtin-grandchild-exec smoke scenario must cover first-exec retention without changing outer PID1
ownership, two consecutive execs retaining the same script parent while releasing the intermediate image,
child-view-only close-on-exec, wait4 and pipe-read resume, script exit restoring PID1, and atomic rollback for
argument/staging/ELF/address-space failures without page or fd-reference leaks. Non-builtin sources, deeper clone,
and a second pending child remain rejected. Capture and exec failure injection must leave the current executable,
both snapshot layers, pending identity and fd views unchanged. The object-smoke two-level
plain-fork coverage is a separate smoke scenario, so its bounded snapshot call chain does not inherit the
large canonical-ELF scenario frame or cross the fixed 16 KiB kernel-init stack boundary. It runs before the
separate legacy child-lifecycle scenario, whose observed-plain-fork coverage intentionally finishes with a
second child pending from the current shell continuation rather than an idle reusable top-level slot.

## Composite and observation policy

Current composite tests are stress repetition and automatic paired difftest. Checkpoint KUnit is a basic
checkpoint-callback profile, not a composite test. A formal paired side always has closed stdin or structured
delayed stdin; terminal interaction is forbidden. Each paired side is first a complete standalone basic-test
execution with its own manifest, result, QEMU log and cleanup. Difftest must not build a side, construct its
disk, own its QEMU process or compare checkpoints when either side did not complete with usable expectations.
Historical wording “manual paired” is normalized to “opt-in paired”.

Long-term checkpoints follow model/coding contracts and cover payload/VFS/Ext2 reads, block submit/wait,
completion source and structured errors. Non-interactive capture uses DEVNULL unless structured input is
declared; checkpoint handlers only read production facts and write their sink.
