# Stress Test Framework

This directory contains the stress/nightly runner for intermittent defects.
It is intentionally separate from KUnit and smoke tests: the goal is to
repeat ordinary commands, archive run evidence, cluster event sequences, and
compare failure classes against successful runs.

Default suite from the repository root:

```sh
make stress-test
```

When no case is specified, the runner executes the standard stress suite. The
current suite covers:

- `cases/df-0001-user-boot.toml`
- `cases/df-0002-smoke-initcall.toml`

`make stress-test` defaults to `STRESS_RUNS=10`, which applies to each case in
the suite. Use `STRESS_RUNS=N` to override it:

```sh
make stress-test STRESS_RUNS=30
```

`STRESS_TIMEOUT` is optional; when it is unset, each case keeps its own
`timeout_seconds` value. The current cases default to 120 seconds.

Fast configuration check without executing QEMU:

```sh
make stress-test STRESS_RUNS=0
```

A case can still be passed explicitly; in that mode only the requested case
runs:

```sh
make stress-test \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  STRESS_RUNS=30
```

DF-0002 can also be selected directly:

```sh
make stress-test \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml \
  STRESS_RUNS=30
```

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
