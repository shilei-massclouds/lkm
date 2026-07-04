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

Paired differential cases use the `pd-*` prefix and are not part of the default
suite. Select them explicitly, for example:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/pd-0004-linux-payload-syscall-paired.toml \
  STRESS_RUNS=1 \
  STRESS_TIMEOUT=240
```

The Linux paired cases are staged by exact-mapped runtime scope and are
temporary progression scaffolds, not long-term regression assets. Completed
stages are deleted after a passing paired run; later stages stay queued until
the current active case passes and is removed.

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

Current active stage:

- `pd-0004-linux-payload-syscall-paired.toml`: distro `/bin/sh` with delayed
  `/bin/ls\nexit\n`; syscall and repeated exec markers are reported as
  observed coverage unless explicitly in scope. This queued case now needs a
  harness refresh before semantic payload work: it still runs arceos_ex with
  `QEMU_SMP=1` after the SMP bringup baseline moved to a 2-vCPU topology, and
  its Linux delayed-stdin marker `/ #` does not match the observed BusyBox
  prompt `~ #`. After that refresh, the next expected semantic gap is the
  requested-init payload handoff boundary: the case still scopes
  `PayloadPhase.Online`, but the current Linux anchor is fallback-only.

Queued later stages:

- `pd-0005-linux-exact-cumulative-paired.toml`: cumulative stable exact-mapped
  intersection for the default-overlay rootfs path.

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
