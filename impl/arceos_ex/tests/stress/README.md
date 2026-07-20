# Composite Stress and Difftest

This directory repeats standalone basic tests and analyzes their frozen results.
It does not build kernels, launch QEMU, send stdin, copy disks, or own process
cleanup. Those responsibilities remain in the basic runner.

## Stress

From the repository root:

```sh
make test-stress
```

The default suite repeats `user-smoke-native`, `kernel-smoke-native`, and
`scripted-shell`. `STRESS_RUNS=N` overrides the configured run count for each
selected case. `STRESS_RUNS=0` validates every selected composite TOML,
classifier, and referenced basic TOML without building a disk or running QEMU.

Focused cases are selected explicitly:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot-stress-mem.toml \
  STRESS_RUNS=1

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall-stress-mem.toml \
  STRESS_RUNS=1

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/busybox-init-login-focused.toml \
  STRESS_RUNS=1
```

For one selected stress case, a schema-v2 historical report can be compared
without changing the current invocation's exit status:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  STRESS_RUNS=10 \
  STRESS_BASELINE=impl/arceos_ex/tests/stress/out/<previous-report>
```

The runner verifies the same case, basic-test identity, classifier hash, and
analysis configuration fingerprint. It reports failure-rate changes,
class/sequence additions and removals, and the most recent sequences' first
divergence. Schema-v1 reports are rejected.

## Difftest

```sh
make difftest
```

The default `rc-local-difftest.toml` executes `rc-local-native` completely, then
executes `rc-local-linux` completely, and only then compares their declared
checkpoint scope. Preflight remains `make test-checkpoints` followed by
`make checkpoints-linux-check`.

Other paired cases are explicit:

```sh
make difftest \
  DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/linux-exact-baseline-difftest.toml

make difftest \
  DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/busybox-init-difftest.toml
```

The long-term shell pair uses `distro-sh-checkpoints` and
`distro-sh-checkpoints-linux`; both send `/bin/ls` twice followed by
`/sbin/poweroff -f`, so termination does not introduce a one-sided exec. The focused BusyBox-init pair uses
`busybox-init-checkpoints` and `busybox-init-checkpoints-linux`; its hard scope
ends at the stable `SyscallTable.Wait4` boundary. Each side must independently
complete and satisfy its own expectations. A side failure is retained alongside
all available diff diagnostics, but the paired run cannot pass.

`DIFFTEST_RUNS=0` performs configuration and referenced-basic validation only.
Composite timeout overrides do not exist; every timeout is owned by the basic
TOML.

## Artifacts

For non-zero runs, the runner first invokes one canonical `make disk` for the
whole selected suite. Each case report is stored below
`impl/arceos_ex/tests/stress/out/<timestamp>-<case>/`:

- `manifest.json`: schema-v2 identities, hashes, run count, metadata, and an
  optional frozen historical-baseline hash set.
- `runs/run-NNNN/basic/`: the complete standalone basic artifact for stress.
- `runs/run-NNNN/{left,right}/`: the two standalone basic artifacts for
  difftest, in strict execution order.
- `runs/run-NNNN/result.json` and `events.json`: composite analysis only.
- `sequences/`, `summary.json`, and `report.md`: sequence clusters, class
  counts, failure-vs-success analysis, checkpoint diffs, and baseline changes.

The terminal always ends with `stress suite summary:` and one
`success=<n> failure=<n> report=<path>` line per completed case.
