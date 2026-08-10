# Composite Stress and Difftest

This directory repeats standalone basic tests and analyzes their frozen results.
It does not build kernels, launch QEMU, send stdin, copy disks, or own process
cleanup. Those responsibilities remain in the basic runner.

## Stress

From the repository root:

```sh
make test-stress
```

The default suite repeats `user-smoke-native`, `kernel-smoke-native`,
`scripted-shell`, the DF-0004 `rc-local-native` timeout-observation case, and
the DF-0005 `busybox-init-login-native` post-login timeout-observation case,
the DF-0006 native user-smoke preemption/pipe timeout-observation case, and the
DF-0007 native LTP frontier child-wait routing case, the DF-0008
linux-object user-smoke plain-fork enqueue-observation case, and the DF-0009
linux-object user-smoke preemption/pipe timeout-observation case, plus the
DF-0010 linux-object LTP frontier post-read runqueue-publication case, and the
DF-0011 linux-object user-smoke post-validation finalization timeout case.
The DF-0012 native rc.local boot-direct-setup case independently preserves the
non-timeout early shutdown observed immediately after `DmaCachePolicy.Ready`.
Its stack-underflow root cause is fixed, but the exact classifier and frozen
identity remain as a 500-run focused regression.
DF-0013 has one default native stress identity and one explicitly selected
linux-object identity. They share one classifier and preserve the same
post-first-read child-return timeout without creating a second defect.
DF-0014 independently repeats `rc-local-native` and classifies the frozen
post-`Kernel.Online` cross-CPU checkpoint-consumer overlap; it is distinct from
DF-0012's pre-runtime direct-setup shutdown and DF-0004's timeout.
DF-0015 repeats the native LTP frontier and requires all four stock-harness PASS
records to remain standalone while wait4 diagnostics are emitted concurrently.
The root Make entry defaults to `STRESS_RUNS=10`, so the normal default sample
is 10 runs per case (150 QEMU runs total). `STRESS_RUNS=N` overrides the
configured run count for each selected case. `STRESS_RUNS=0` validates every
selected composite TOML, classifier, and referenced basic TOML without building
a disk or running QEMU.

Focused cases are selected explicitly:

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot-stress-mem.toml \
  STRESS_RUNS=1

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall-stress-mem.toml \
  STRESS_RUNS=1

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0005-busybox-init-login-native.toml \
  STRESS_RUNS=100

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0006-user-smoke-preempt-native.toml \
  STRESS_RUNS=100

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0007-ltp-frontier-child-wait-native.toml \
  STRESS_RUNS=50

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0007-ltp-frontier-child-wait-linux-object.toml \
  STRESS_RUNS=50

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0008-user-smoke-fork-enqueue-linux-object.toml \
  STRESS_RUNS=100

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0009-user-smoke-preempt-linux-object.toml \
  STRESS_RUNS=100

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0010-ltp-frontier-post-read-runqueue-linux-object.toml \
  STRESS_RUNS=50

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0011-user-smoke-preempt-finalize-linux-object.toml \
  STRESS_RUNS=100

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0012-rc-local-direct-setup-native.toml \
  STRESS_RUNS=500

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0013-fork-ofd-offset-child-return-native.toml \
  STRESS_RUNS=50

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0013-fork-ofd-offset-child-return-linux-object.toml \
  STRESS_RUNS=50

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0014-checkpoint-cross-cpu-reentry-native.toml \
  STRESS_RUNS=500

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0015-ltp-frontier-console-record-interleave-native.toml \
  STRESS_RUNS=50

make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/rc-local-native-timeout-focused.toml \
  STRESS_RUNS=100
```

The focused commands run DF-0005, DF-0006, both provider identities for the
scheduling-sensitive DF-0007, DF-0008, DF-0009, DF-0010, DF-0011, DF-0012,
DF-0013, DF-0014, DF-0015, and DF-0004 at their declared focused sample sizes.
The 100/200/300/500 staircase is reserved for an unresolved probabilistic
investigation; it is not a mechanical post-fix gate. Each case keeps every
standalone artifact and, on a timeout, the basic runner freezes
`qemu-timeout-diagnostics.json` before it terminates QEMU. A run with no
failures reports only that the timeout was not observed in that sample; it is
not a root-cause verdict.

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

The focused DF-0013 behavior pair compares sibling Linux with native arceos_ex
using only the exact `R1=#!/`, `R2=bin`, and `R3=/sh` output lines. Command
echoes do not satisfy the observable sequence. Difftest is a horizontal
semantic comparison, so it defaults to 10 paired runs and the runner rejects
case-local or command-line counts above 10. Rare concurrency reproduction and
high-round sampling belong to the provider stress identities:

```sh
make difftest \
  DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/fork-ofd-offset-difftest.toml \
  DIFFTEST_RUNS=10
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
- `runs/run-NNNN/basic/qemu-timeout-diagnostics.json`: the QMP CPU/register/IRQ
  snapshot captured before terminating a timed-out QEMU, when a timeout occurs.
- `runs/run-NNNN/{left,right}/`: the two standalone basic artifacts for
  difftest, in strict execution order.
- `runs/run-NNNN/result.json` and `events.json`: composite analysis only.
- `sequences/`, `summary.json`, and `report.md`: sequence clusters, class
  counts, failure-vs-success analysis, checkpoint diffs, and baseline changes.

The terminal always ends with `stress suite summary:` and one
`success=<n> failure=<n> report=<path>` line per completed case.
