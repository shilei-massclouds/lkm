# Stress Test Framework

This directory contains the stress/nightly runner for intermittent defects.
It is intentionally separate from KUnit and smoke tests: the goal is to
repeat ordinary commands, archive run evidence, cluster event sequences, and
compare failure classes against successful runs.

Initial case:

```sh
impl/arceos_ex/tests/stress/runner.py --runs 30
```

Fast configuration check without executing QEMU:

```sh
impl/arceos_ex/tests/stress/runner.py --runs 0
```

The default case is `cases/df-0001-user-boot.toml`. A case can still be passed
explicitly:

```sh
impl/arceos_ex/tests/stress/runner.py \
  impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  --runs 30
```

Output defaults to `impl/arceos_ex/tests/stress/out/<timestamp>-<case>/` and
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
