# Stress Test Framework

This directory contains the stress/nightly runner for intermittent defects.
It is intentionally separate from KUnit and smoke tests: the goal is to
repeat ordinary commands, archive run evidence, cluster event sequences, and
compare failure classes against successful runs.

Default suite from the repository root:

```sh
make test-stress
```

When no case is specified, the runner executes the standard stress suite. The
current suite covers:

- `cases/df-0001-user-boot.toml`
- `cases/df-0002-smoke-initcall.toml`
- `cases/df-0003-distro-sh-ls.toml`

`make stress-test` remains as a compatibility alias. `make test-stress` defaults
to `STRESS_RUNS=10`, which applies to each case in
the suite. Use `STRESS_RUNS=N` to override it:

```sh
make test-stress STRESS_RUNS=30
```

`STRESS_TIMEOUT` is optional; when it is unset, each case keeps its own
`timeout_seconds` value. The current cases default to 120 seconds.

Fast configuration check without executing QEMU:

```sh
make test-stress STRESS_RUNS=0
```

A case can still be passed explicitly; in that mode only the requested case
runs:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  STRESS_RUNS=30
```

DF-0002 can also be selected directly:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml \
  STRESS_RUNS=30
```

DF-0003 exercises the non-PTY distro shell delayed-input path:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0003-distro-sh-ls.toml \
  STRESS_RUNS=30
```

OpenRC login shell closure is kept as an explicit focused diagnostic case. It
uses the checked-in account overlay, staged login/password/shell input, and the
`user-syscall-trace,user-syscall-error` probes. It is not part of the default
stress suite and must be selected manually:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/openrc-login-focused.toml \
  STRESS_RUNS=1
```

Linux/arceos_ex baseline differential testing has a dedicated entry point:

```sh
make difftest
```

`make difftest` defaults to one real paired run of
`cases/rc-local-difftest.toml`, the direct inittab rc.local paired diff. Use
`DIFFTEST_RUNS=0` for a fast
configuration check without executing QEMU:

```sh
make difftest DIFFTEST_RUNS=0
```

The default rc.local case carries an exact checkpoint coverage audit. The
current accounting is `required_total=103`, `in_scope=59`,
`accounted_outside_scope=44`, and `unaccounted=0`: every exact-mapped
checkpoint is either in the default rc.local hard scope or explicitly accounted
outside that scope. This means the default hard scope is fully accounted; it
does not mean all 103 exact checkpoints are compared by the default case.

The long-term shell baseline remains available explicitly:

```sh
make difftest \
  DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/linux-exact-baseline-difftest.toml
```

`DIFFTEST_TIMEOUT` is optional; when it is unset, the case keeps its own
`timeout_seconds` value.

The earlier Linux paired `pd-*` cases were staged by exact-mapped runtime scope
and served as progression scaffolds. Completed staged cases are deleted after a
passing paired run. `pd-0005` has now converged into the long-term
`linux-exact-baseline-difftest.toml` regression asset and is kept as an
explicit `DIFFTEST_CASE` selection.

Completed stages:

- `pd-0001-linux-entry-prelude-paired.toml`: entry-prelude front segment. Its
  stage is complete; later boot-path `pd-*` cases continue from deeper and
  partially overlapping checkpoints.
- `pd-0002-linux-boot-c-paired.toml`: boot C phase landmarks from
  `CorePreparePhase.Started` through `ProcessPreparePhase.Started`. Its stage
  is complete; `StartupTimeline.Started` remains a Linux-side runtime marker
  outside the current hard diff intersection.
- `pd-0003-linux-runtime-rootfs-paired.toml`: SMP, runtime core, initcall,
  rootfs, finalize, and `PayloadPhase.Ready` landmarks. Its stage is complete;
  `PayloadPhase.Online` is not part of that requested-init hard scope because
  the current Linux anchor is on the default fallback `/sbin/init` block, while
  `init=/sbin/init` succeeds before that block.
- `pd-0004-linux-payload-syscall-paired.toml`: distro `/bin/sh` with delayed
  `/bin/ls\nexit\n`; runtime exec comparison now extends through the exact
  `UserExec.MainElfReady`, `UserExec.InterpreterReady`,
  `UserExec.ContextReplaced`, `UserExec.SatpReady`, and
  `UserExec.TrapFrameReady` anchors. Other syscall and repeated exec markers
  are reported as observed coverage unless explicitly in scope. The harness
  uses a 2-vCPU topology on both sides and waits for the observed BusyBox
  prompt `~ #`. `UserExec.AddressSpaceReady` remains outside hard scope
  because its Linux mapping is range, not exact. Linux runtime instrumentation
  must keep `kernel_execve()` ownership separate from user
  `execve()/execveat()` ownership before extending this stage deeper.
  `PayloadPhase.Online` remains in hard scope because Linux now records the
  selected-payload handoff before the requested-init
  `run_init_process(execute_command)` path as well as before the default
  fallback block; the single-fingerprint `LKM_CHECKPOINT` comment remains on
  the canonical mapping anchor.
  The latest recorded run at
  `impl/arceos_ex/tests/stress/out/20260705T122918Z-pd-0004-linux-payload-syscall-paired/`
  completed with `success: 1` and `failure: 0`. The case file has been deleted;
  its payload/syscall hard scope is now absorbed by the long-term
  `linux-exact-baseline-difftest.toml` case. Future payload/syscall
  localization should use baseline first-divergence data and
  `observed_but_not_compared` coverage instead of rerunning this completed
  stage.

Baseline difftest cases:

- `rc-local-difftest.toml`: default `make difftest` paired differential test
  for direct BusyBox inittab rc.local support. It compares the stable
  checkpoint prefix through the rc.local shell and `/bin/ls` execs. Its exact
  checkpoint coverage audit currently reports `required_total=103`,
  `in_scope=59`, `accounted_outside_scope=44`, and `unaccounted=0`; the 59
  in-scope checkpoints are the hard comparison set, while the other 44 exact
  checkpoints are intentionally accounted outside the default rc.local hard
  scope.
- `linux-exact-baseline-difftest.toml`: long-term baseline differential test
  for the cumulative stable exact-mapped intersection on the distro `/bin/sh`
  path with delayed `/bin/ls\nexit\n`. It uses the shell rootfs image without
  the default `/sbin/init` overlay, 2-vCPU topology on both sides, arceos_ex
  prompt marker `/ #`, and Linux BusyBox prompt marker `~ #`. Its hard scope is
  the converged `pd-0005` cumulative boot/rootfs/payload checkpoint set and
  includes the exact `UserExec.*` anchors completed by `pd-0004`. Range and
  unmapped checkpoints remain outside hard scope and are only reported as
  observed coverage. Architecture-front markers remain formal RISC-V exact
  anchors with arceos_ex early-byte or Linux mapping coverage, but the current
  shell cumulative paired runner does not capture them as a stable one-to-one
  hard-gate sequence. `EntryPreludePhase.Started` is covered by the completed
  `pd-0001` stage; `TrampolineVm.Online`, `KernelImage.Online`,
  `EventStream.Ready`, and `ExceptionStream.Ready` stay outside this hard scope
  because Linux 2-vCPU runs can report the same head.S markers again on the AP
  path or in a different entry-vs-C ordering. The Linux-only
  `StartupTimeline.Started` C-entry marker is likewise observed coverage, not a
  baseline hard gate.
- `openrc-native-init-difftest.toml`: focused manual paired differential test
  for native Alpine `/sbin/init` / OpenRC with no `init=/bin/sh` override. It
  is intentionally separate from the long-term `/bin/sh -> /bin/ls` baseline
  so OpenRC-specific signal/wait/process lifecycle gaps can be localized
  without weakening the shell regression asset. Its initial hard scope extends
  the boot/payload intersection only to the first OpenRC wait boundary currently
  modeled as `SyscallTable.Wait4`; syscall trace/error lines remain diagnostic
  evidence outside the checkpoint comparator.

Focused opt-in stress cases:

- `openrc-login-focused.toml`: opt-in stress/focused diagnostic for the
  OpenRC getty/login shell path. It uses
  `tests/rootfs-overlays/openrc-login`, waits for `login:`, `Password:`, and
  the BusyBox shell prompt before sending `/bin/ls\nexit\n`, and classifies
  success only when the Alpine greeting, getgroups trace, rootfs listing
  marker, and `user exit status=0` are all present. This case is not in the
  default `make test-stress` suite.

Paired diffs only compare the declared `checkpoint_scope` for each case.
Runtime checkpoints outside that scope are listed as
`observed_but_not_compared` with `outside_checkpoint_scope`; they are useful
coverage evidence, but they do not make a run fail until a case explicitly adds
them to its scope.

A paired mismatch caused by an arceos_ex/Linux semantic or implementation gap is
treated as diagnostic evidence, not as an instruction to automatically extend
arceos_ex. Preserve the report and summarize the first divergence, missing or
extra scoped checkpoints, and observed coverage. Only runner/reporting defects
or incorrect Linux checkpoint instrumentation should be fixed as part of these
paired cases.

Each case output defaults to `impl/arceos_ex/tests/stress/out/<timestamp>-<case>/` and
contains:

- `manifest.json`: case, command, git metadata, run count, and output paths.
- `runs/run-NNNN/`: per-run raw log, events, metadata, and classification.
- `sequences/<result>/<class>/<hash>.json`: deduplicated representative
  event sequences with occurrence counts and run ids.
- `summary.json`: class counts, sequence counts, features, and failure-vs-success
  comparisons.
- `report.md`: compact human-readable summary.

DF-0001 must keep the ordinary non-probe `APP=user-boot` path that executes
the overlay `/sbin/init` and reaches `user exit status=0`. The plain
`make run APP=user-boot` default is currently an interactive distro
`init=/bin/sh` path, so DF-0001 cases must set `QEMU_APPEND=earlycon=sbi`
explicitly and prepare a dedicated overlay disk in `setup_command`. Probe
variants can be added later as separate cases, but they must not replace the
ordinary non-probe path because probes can change timing.

DF-0002 must keep the ordinary non-probe `APP=smoke` path for the same reason.
It also uses a dedicated default-overlay disk prepared in `setup_command`, so
stress results are not affected by a developer's stale `build/virtio-blk.raw`
from distro or diagnostic runs.

DF-0003 must keep the ordinary non-probe distro shell path and the explicit
external command payload `/bin/ls\nexit\n`. The case uses `delayed_stdin` only to
wait for the BusyBox prompt before writing that payload; it must not fake
terminal responses or change the command shape to make the case pass.
