# Stress Test Framework

This directory contains the stress/nightly runner for intermittent defects.
It is intentionally separate from KUnit and smoke tests: the goal is to
repeat ordinary commands, archive run evidence, cluster event sequences, and
compare failure classes against successful runs.

Default suite:

```sh
impl/arceos_ex/tests/stress/runner.py --runs 30
```

When no case is specified, the runner executes the standard stress suite. The
current suite covers:

- `cases/df-0001-user-boot.toml`
- `cases/df-0002-smoke-initcall.toml`

`--runs N` applies to each case in the suite.

Fast configuration check without executing QEMU:

```sh
impl/arceos_ex/tests/stress/runner.py --runs 0
```

A case can still be passed explicitly; in that mode only the requested case
runs:

```sh
impl/arceos_ex/tests/stress/runner.py \
  impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  --runs 30
```

DF-0002 can also be selected directly:

```sh
impl/arceos_ex/tests/stress/runner.py \
  impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml \
  --runs 30
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

DF-0001 must keep the ordinary `make run APP=user-boot` path. Probe variants
can be added later as separate cases, but they must not replace the ordinary
path because probes can change timing.

DF-0002 must keep the ordinary `make run APP=smoke` path for the same reason.
