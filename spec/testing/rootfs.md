# Rootfs and user-mode acceptance testing

本文件是 rootfs/user fixture、发行版 smoke、OpenRC 输入编排和 checkpoint difftest 的权威测试规格。
镜像构造见 [`../coding/projects/rootfs-image.md`](../coding/projects/rootfs-image.md)；对象行为见
[`../coding/objects/user-boot.md`](../coding/objects/user-boot.md)、
[`../coding/objects/vfs.md`](../coding/objects/vfs.md) 和
[`../coding/objects/ext2.md`](../coding/objects/ext2.md)。本文件保留从旧
`ArceosExBlockIoCodingMust` 分组迁入的稳定 rule ID 和 MUST 层级。

## User fixture output and analysis

Rule IDs (MUST):

- `arceos_ex_must_user_probe_print_per_syscall_success_marker`
- `arceos_ex_must_user_probe_cover_directory_openat_getdents64`
- `arceos_ex_must_user_syscall_analysis_use_existing_static_tools`
- `arceos_ex_must_user_syscall_analysis_stay_out_of_default_build_path`
- `arceos_ex_must_user_syscall_analysis_mark_busybox_candidates_conservative`
- `arceos_ex_must_user_syscall_vfs_specs_reference_linux_6_12`

Staged syscall subtests print an explicit success marker after each validated path. The user-smoke wrapper
uses the `user-smoke:` prefix, begin/end markers, `status=N` and blank-line case separation; the host still
determines success from `user exit status=N`. Directory probing uses the RISC-V ABI for
`openat(AT_FDCWD, "/", O_RDONLY|O_DIRECTORY)`, `getdents64(61)`, record validation and close.

The libc-linked stack smoke reads auxv through `getauxval()` and requires the independent exec filename to
match that fixture invocation's startup `argv[0]` (the same binary is installed as both `/sbin/init` and
`/bin/ls` by existing cases), plus common RISC-V HWCAP, real/effective UID/GID, and a nonzero 16-byte
`AT_RANDOM` block matching the running process. It keeps the compiler stack-protector path and the 256 KiB
demand-growth plus cross-page usercopy probes. Object smoke separately covers ASLR endpoints/alignment/
layout, deterministic seed selection, independence of ASLR seed and `AT_RANDOM`, filename differing from
`argv[0]`, every supported auxv key/value and final `AT_NULL`, 16-byte SP alignment, exact-24/short/
unavailable entropy rollback, and stack fault/snapshot behavior under randomized tops.

Distribution-command analysis first uses existing local tools (`file`, `readelf`, `objdump`, headers and
read-only sysroot inspection) and keeps temporary notes out of the default build/image/test path. BusyBox
whole-binary symbols are conservative candidates, not an applet trace. Any promoted syscall/VFS slice is
checked against the local Linux 6.12 RISC-V syscall table and relevant fs implementation, with locking,
RCU, permission, namespace, LSM and errno omissions recorded as trimmed/deferred before implementation.

## Default user and distro smoke

Rule IDs (MUST):

- `arceos_ex_must_test_harness_pin_user_smoke_qemu_append`
- `arceos_ex_must_test_harness_cover_no_overlay_bin_ls`
- `arceos_ex_must_test_harness_cover_no_overlay_bin_sh_with_host_input`
- `arceos_ex_must_keep_shell_external_commands_and_native_init_diagnostic_until_specified`

The user-smoke harness uses a case-local disk, the default overlay map, `FORCE=1` and case-local
`QEMU_APPEND="earlycon=sbi"`; it does not inherit the manual `/bin/sh` default. Separate distro cases use
`ROOTFS_OVERLAY=none`: one runs `init=/bin/ls`, and one waits for the BusyBox prompt before sending bounded
host-side `/bin/ls`, `/bin/ls` and `exit`. The latter requires both external commands to complete, a stable
rootfs marker from each listing, no `Function not implemented`, and `user exit status=0`.
Host input belongs to delayed-stdin orchestration and is not kernel-side ready data. These ordinary shell
and native-init diagnostics remain until their replacement behavior is separately specified.

DF-0003 uses that exact two-command delayed-input payload for 30-run ordinary-path stress. The
Linux/arceos_ex exact paired shell baseline sends the same payload to both sides and compares the stable
exact checkpoint sequence. Its arceos_ex `stress-mem` capture uses 256 KiB so the complete two-command
announce stream is retained without overflow; a truncated one-command prefix is not an acceptable paired
baseline. Object smoke must additionally complete one observed child fork/wait/exit/
parent-restore round, create a second child from the same shell continuation, and assert increasing pids,
preserved shell identity/runqueue visibility and cleared first-round snapshot/wait/exit facts.

## rc.local and paired difftest

Rule IDs (MUST):

- `arceos_ex_must_rc_local_test_use_inittab_direct_marker_only`
- `arceos_ex_must_rc_local_difftest_be_default_case`
- `arceos_ex_must_rc_local_difftest_report_hard_scope_coverage_counts`

The first rc.local slice is a focused diagnostic, not a default `make test` gate. It uses
`ROOTFS_OVERLAY=none` and a checked-in file overlay that changes only `/etc/inittab` and `/etc/rc.local`.
BusyBox init directly executes `/bin/sh /etc/rc.local`; success is the ordered script markers, including
the rootfs listing marker, rather than a claim about the OpenRC service graph or full PID1 lifecycle.

This direct-inittab case is the default paired difftest. The former delayed `/bin/sh` case remains selectable
through `DIFFTEST_CASE`. `checkpoint_scope` is only the hard comparison set: every required exact checkpoint
is either in scope or has explicit outside-scope accounting. Reports separate hard-scope comparison from
`required_total`, `in_scope`, `accounted_outside_scope` and `unaccounted` coverage counts.

## OpenRC login input orchestration

Rule ID: `arceos_ex_must_openrc_login_test_use_explicit_account_overlay` (MUST).

OpenRC getty/login acceptance uses `ROOTFS_OVERLAY=none` plus an explicit account-only file overlay; it
does not change the locked-root semantics of the bare image or bypass authentication. The host waits for
`login:`, optionally `Password:`, then the shell prompt before sending `/bin/ls` and `exit`. It requires a
stable rootfs marker and `user exit status=0`. The focused case remains opt-in through `STRESS_CASES` until
explicitly selected; it is not part of default `make test` or test-stress.

The job-control acceptance requires an explicit trace fact classifying the shell parent's successful
`setpgid(child_pid, child_pid)` target as the pending child, no `reason=pid_not_visible` rejection for that
operation, successful foreground-pgrp handling, the `lost+found` rootfs marker and clean user exit. The
case remains opt-in after this bounded closure; it does not claim multiple pending children, a general
task graph, post-exec parent setpgid, job-control signal delivery or full process-group lookup.

## Manual LTP shell

Ordinary `make run APP=user-boot` is the manual syscall-test entry and uses the unpacked sibling LTP
staging tree by default. It builds a dedicated 320 MiB image, boots `init=/bin/sh`, and leaves test
selection to the operator. From the BusyBox shell the initial workflow is:

```sh
cd /opt/ltp
./run-syscalls.sh 'getpid*' 'uname*'
```

This entry establishes only that the LTP tree can be staged, booted, and invoked manually. Individual
LTP `FAIL`, `BROK`, or `TCONF` results and missing syscall, `/proc`, `/sys`, or `/dev` capabilities do not
expand the acceptance scope of this image-construction change.

All repository-owned automated user-boot cases in `make test`, stress, and paired difftest explicitly set
`ROOTFS_LTP_OVERLAY=none`. Their existing 64 MiB or explicitly named images remain independent from the
sibling LTP repository and do not execute LTP. Focused image checks for this interface must cover the
default LTP image contents, the disabled lean-rootfs path, and a clear failure for missing or malformed
LTP staging.

## Long-term observation checkpoints

Rule IDs (MUST):

- `arceos_ex_must_long_term_checkpoints_follow_model_coding_contracts`
- `arceos_ex_must_payload_vfs_ext2_read_emit_observation_checkpoints`
- `arceos_ex_must_block_io_task_wait_checkpoints_cover_submit_wait_and_timeout`
- `arceos_ex_must_block_io_irq_completion_checkpoints_cover_begin_end_failure`
- `arceos_ex_must_block_io_completion_source_distinguish_irq_and_task_poll`
- `arceos_ex_must_read_path_error_classification_checkpoint_be_structured`

Nightly/stress and Linux-like comparison checkpoints are specified by model/coding before implementation.
The stable set covers payload image read, VFS path resolution, Ext2 lookup/read, block task submit/wait and
virtio-blk completion source. Failures/timeouts use structured classes and request identity so repeated
sequences can be grouped; IRQ and task-poll completion remain distinguishable.

Non-interactive runner capture uses a closed/`DEVNULL` stdin unless a case explicitly declares
`delayed_stdin`. Default observation stays lightweight; heavy consumers are opt-in, and handlers only read
object/provider facts and write through their sink.
