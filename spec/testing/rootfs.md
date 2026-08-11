# Rootfs and user-mode acceptance testing

本文件是 rootfs/user fixture、发行版 smoke、BusyBox init 输入编排和 checkpoint difftest 的权威测试规格。
镜像构造见 [`../coding/rootfs-image.md`](../coding/rootfs-image.md)；基本测试生命周期见
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

`user-smoke-native` and `user-smoke-linux-object` are the default 8-CPU user-smoke acceptance identities. They
remain non-interactive, boot `/opt/lkm/tests/user-smoke`, and enter the default regression gate explicitly by
TEST name. The explicit `user-smoke-smp2-*` and `user-smoke-smp16-*` variants run the same payload with both
providers at the lower and upper supported CPU-count boundaries. DF-0001 likewise repeats
`TEST=user-smoke-native`; it must not depend on a TTY, the APP variable-name alias, or a retired test-name
mapping.

The pure-compute preemption subtest publishes 17 consecutive children and selects the first and seventeenth as
its only competitors. Their PIDs differ by 16, so fixed PID-modulo placement puts them on one CPU for each
supported validation count 2, 8 and 16. Both children wait for one of two start tokens at the head of the single
shared pipe until all children are published. The parent writes those tokens and reaps all children before it
reads the later A/B bytes, so the parent cannot consume a start token intended for a target; a leaked start token
is nevertheless an explicit test failure. Acceptance requires both byte counts and at least two A/B transitions.
Parallel execution on different CPUs or completion of the first child before publication of the second cannot
satisfy this evidence.

`scripted-shell` and `scripted-shell-lo` are the default scripted shell acceptance identities. They use the
native and linux-object providers, respectively, attach the canonical template read-only, wait for `~ #`,
send exactly `echo "OK"\nls\nls /lib\nls /\nexit\n`, and apply a 120-second limit. Both enter default
`make test`. Neither provider may emit an Ext2 block-read failure even if a later retry happens to make the
fixed command loop complete; this guards the synchronous single-request owner against leaking transient
completion contention into VFS. The removed public spellings `script-shell`, `script-shell-lo`, `distro-sh-native` and
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

The focused native `announce,stress-mem` stability case repeats `rc-local-native` 50 times. Every repetition must
retain one contiguous `lost+found` listing marker, complete named checkpoint records and the ordered begin/end
markers, with zero timeout, panic, corrupt checkpoint name or marker-only failure. This repetition validates the
shared Printk/diagnostic write-batch boundary; unique byte-interleaving traces are failures, not acceptable test
jitter.

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
successful `setpgid` of the exact published child, foreground-pgrp handling, `lost+found`, the deterministic
wrapper reaching successful `sync` before poweroff, a clean QEMU guest-shutdown exit, and no panic, unsupported,
init child exec failure or `reason=pid_not_visible` marker. A child `exit_group` must return through wait/reap to
the wrapper; it is not a PID1 `user exit status` shutdown boundary. Both providers enter default `make test`.
The DF-0005 default stress case repeats the frozen native basic-test entry and preserves timeout/QMP artifacts;
the older focused identity remains an explicit selector. Neither builds an account overlay. This bounded closure does not claim a
general task graph, multiple pending children, post-exec parent setpgid, job-control signals or full pgrp lookup.

The DF-0006 default stress case separately repeats `user-smoke-native` for the intermittent preemption pipe-read
timeout first frozen after both compute children printed their write-complete markers but before the coordinator
printed bytes-collected. It preserves the unchanged 8-CPU basic identity and timeout/QMP artifacts; the historical
DF-0001 classifier for ELF-read failure does not subsume this scheduler/pipe observation.

The DF-0007 default stress case separately repeats the frozen first-batch native `ltp` supported identity for the
child wait/reap routing failure first observed after `uname01` printed both TPASS records; an explicit non-default
case repeats `ltp-lo` for the independently observed linux-object side. Historical DF case names retain
`frontier` to identify their first artifacts, but their current basic references must not follow the movable
frontier. Each provider identity has a
routine sample of 50 runs; 100/200/300/500-run expansion is reserved for an unresolved probabilistic investigation.
The stock LTP harness, exact list, 8-CPU topology, canonical rootfs, provider
and completion marker remain owned by the referenced basic cases; stress classification may preserve the failure
but may not alter those inputs.

The DF-0015 default stress case independently repeats that frozen native `ltp` supported identity to preserve the
console-record boundary first observed when a stock runner PASS line was inserted into a fragmented `wait4
registry` diagnostic. Its routine sample is 50 runs. Classification requires each of the four exact PASS records
and the clean four-entry summary to occupy standalone lines; it cannot infer success from LTP-internal TPASS text.
The 100/200/300/500-run staircase is only for an unresolved probabilistic recurrence, while a Linux/native
horizontal comparison remains capped at ten runs.

The DF-0016 default stress case repeats the unchanged linux-object `ltp-lo` supported identity for the
generation-stale root `TrapFlowRef` first observed during `geteuid01` when an SSIP entered
`TrapOccurrence.BindRoot`. The first supported acceptance stress batch reproduced once within 39 classified
linux-object samples; the subsequent frozen investigation completed 100/100 without recurrence. That accepted
sample moves the checked-in case to the routine 50-run regression size without erasing the first failure or claiming
a causal implementation repair. Classification requires both the formal trap-occurrence failure and
`installed_root_stale`; success requires all four exact PASS records, the clean summary and completion protocol.
The stock harness, exact four-entry supported list, canonical rootfs, eight CPUs, provider, timeout and marker
protocol remain unchanged. Expansion to 100/200/300/500 resumes only if the same class recurs and requires renewed
location work; it is not routine acceptance or differential testing.

The DF-0008 default stress case separately repeats the unchanged `user-smoke-linux-object` basic identity for
the intermittent plain-fork child enqueue failure observed during the repeated COW fork cycle. Its case-local
sample is 100 runs. The linux-object provider, 8-CPU topology, canonical rootfs, 120-second timeout, payload and
acceptance markers remain owned by the referenced basic case; this is distinct from the native DF-0006 pipe-read
timeout and must retain its own artifacts and classification.

The DF-0009 default stress case separately repeats the unchanged `user-smoke-linux-object` basic identity for
the intermittent preemption pipe-read timeout first frozen after both compute children printed their
write-complete markers but before the coordinator printed bytes-collected. Its case-local sample is 100 runs.
The linux-object provider, 8-CPU topology, canonical rootfs, 120-second timeout, payload and acceptance markers
remain owned by the referenced basic case. This observation has the same terminal marker shape as native
DF-0006 but retains an independent provider-specific entry, and it does not replace the distinct DF-0008
plain-fork enqueue failure.

The DF-0010 default stress case repeats `ltp-supported-post-read-lo`: the initial shell reproduces the first
observation's complete child history (length check, failed multi-operand hash child, interrupted partial command,
then the successful stock-script hash and `sh -n`), invokes the frozen first-batch supported selector unchanged,
and finally launches a second hash child from that same shell. This diagnostic identity preserves the post-batch
runqueue-publication failure independently from the stock harness parse error and cleanup/unsupported-syscall
boundaries. It must retain the canonical rootfs, eight CPUs, linux-object provider, exact four-entry supported
selection, command order, completion marker, and full basic artifacts; it is not an LTP acceptance identity and
must not follow the movable frontier.

The DF-0011 default stress case separately repeats the unchanged `user-smoke-linux-object` basic identity for
the intermittent timeout after pipe bytes were collected, both compute children were reaped, and the A-B-A
round-robin check passed, but before the preempt case and guest finalized. Its case-local sample is 100 runs.
The linux-object provider, eight CPUs, canonical rootfs, 120-second timeout, payload and acceptance markers remain
owned by the referenced basic case. This later terminal boundary is independent from DF-0009's pre-collection
timeout and must retain its own artifacts and classification.

`busybox-init-checkpoints` and `busybox-init-checkpoints-linux` are standalone diagnostic basic tests for the
same `/sbin/init` profile. Each side must independently complete, satisfy its expectations, emit a complete
non-overflowing `stress_mem` record containing `SyscallTable.Wait4`, and clean up its private image before the
paired case may compare them. The hard scope ends at the first stable `SyscallTable.Wait4`; it represents
BusyBox init waiting/reaping, not OpenRC behavior. The arceos_ex side captures the checkpoint stream with
`announce,stress-mem`; syscall trace/error probes are outside this differential observation and must not consume
the bounded checkpoint buffer. Its declared recorder capacity is 524288 bytes, which must retain the complete
session without overflow rather than truncating the sequence at the comparison boundary.

## Stock LTP syscall batch acceptance

Rule IDs (MUST):

- `arceos_ex_must_ltp_supported_batch_run_stock_harness_on_three_targets`
- `arceos_ex_must_ltp_frontier_remain_diagnostic_until_promotion`
- `arceos_ex_must_ltp_selection_use_exact_unique_known_entry_names`
- `arceos_ex_must_ltp_acceptance_require_exact_clean_summary_and_per_entry_pass`
- `arceos_ex_must_ltp_marker_not_be_triggered_by_command_echo`

`ltp`, `ltp-lo` and `ltp-linux` are scripted acceptance identities for arceos_ex native, arceos_ex
linux-object and sibling Linux, respectively. `linux-object` remains an arceos_ex provider; only the `-linux`
identity selects the sibling kernel. All three attach private copies of the same canonical rootfs, use eight
vCPUs, consume the same `supported` exact-name list and stop through the same marker protocol. They are invoked
only by the independent `make test-ltp` gate and never by root `make test`.

`ltp-frontier`, `ltp-frontier-lo` and `ltp-frontier-linux` are diagnostic identities with the same three target
mapping. They consume only the `frontier` list and never enter an acceptance or stress gate. A frontier batch may
move into the cumulative supported list only after its stock LTP run passes on all three targets and both
arceos_ex providers satisfy the required stress count.

All arceos_ex LTP identities enable only the error-only syscall probe. Full per-syscall tracing is excluded from
both frontier diagnostics and supported acceptance: the stock runner scans its runtest file through byte-oriented
reads, so tracing every successful read perturbs the run enough to hide the first semantic boundary or exhaust the
acceptance timeout before the unchanged harness completes.

Canonical construction validates every line in both lists as one nonempty exact runtest entry name, rejects
duplicates within either list, and requires the name to occur exactly once in sibling LTP's `runtest/syscalls`.
The installed selector performs only the conversion from those lines to
`/opt/ltp/run-syscalls.sh -- <exact names...>` argv: it must not edit output, replace a test binary, glob a
broader set, change the stock runner, or suppress a harness result.

All three identities also see the same read-only canonical `/proc/meminfo` harness fixture. It exists only
because the stock LTP new API consults `MemAvailable` before these tests even when their declared minimum is
zero; using the fixture is not acceptance of procfs, `/proc/self`, or `/proc/sys` behavior.

For a supported list of size `N`, acceptance requires exactly one
`Summary: TOTAL=N PASS=N FAIL=0 BROK=0 WARN=0 CONF=0`, exactly one stock runner
`--- <entry>: PASS (exit 0)` record for every selected entry, and a zero-status completion record. Unsupported
syscall/ENOSYS, TFAIL, TBROK, TWARN, TCONF, non-pass runner records and either kernel's panic marker are forbidden.
The fixed completion marker is emitted only after the runner and status record; the scripted stdin payload must
not contain that marker, so shell command echo cannot terminate QEMU early.

`make test-ltp-stress LTP_STRESS_RUNS=30` repeats the supported acceptance independently for native and
linux-object. Thirty clean runs per provider are the default promotion threshold; a batch involving scheduling,
blocking or timeouts uses fifty. Sibling Linux must complete at least one matching full batch before promotion.

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
