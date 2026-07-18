# Composite basic-test 独立验收归档

本文归档 2026-07-18 在提交 `d8f7a41a` 上完成的 composite basic-test v2 独立验收。
本轮没有修改命令、TOML schema、runner 行为、test identity 或 Linux differential scope；验收使用默认
Linux provider `../linux-6.12`，只把已实现的编排、集合化报告和 paired difftest 结果收口为完成里程碑。

## 验收顺序与判据

验收先运行一次 `make disk`，再逐项运行 composite 引用的 12 个 standalone basic tests。六个 stress
basic 完成后，三对 difftest basic 均按 Linux reference 在前、arceos_ex 在后的顺序执行。只有全部
standalone gate 通过后才运行 composite。

每个 standalone artifact 均确认 `execution_status=completed`、`expectations.passed=true`、QEMU stage
success、`timed_out=false`、`process_group_reaped=true`，并且所有 `private-copy` 磁盘均已删除。
`template-readonly` 没有私有副本，因而 `private_disk_removed=null` 是预期值。Acceptance basic 的 verdict
为 `passed`；checkpoint/reference basic 的 purpose 是 `diagnostic`，按 result schema v2 成功 verdict
固定为 `inconclusive`，其 standalone gate 以 completed、expectations 和 cleanup 事实通过，不能改写为
`passed`。

实际执行命令如下：

```sh
make disk

make run TEST=user-smoke-native
make run TEST=kernel-smoke-native
make run TEST=distro-sh-native
make run TEST=user-smoke-stress-mem
make run TEST=kernel-smoke-stress-mem
make run TEST=busybox-init-login-native

make run TEST=rc-local-linux
make run TEST=rc-local-native
make run TEST=distro-sh-checkpoints-linux
make run TEST=distro-sh-checkpoints
make run TEST=busybox-init-checkpoints-linux
make run TEST=busybox-init-checkpoints
```

## Standalone basic 结果

下表中的路径均相对仓库根目录；每个目录都包含完整的 `manifest.json`、`result.json` 和 `qemu.log`。

| basic test | purpose / verdict | artifact 目录 |
| --- | --- | --- |
| `user-smoke-native` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140138.517858Z-user-smoke-native-107568` |
| `kernel-smoke-native` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140145.427738Z-kernel-smoke-native-107655` |
| `distro-sh-native` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140158.046694Z-distro-sh-native-107760` |
| `user-smoke-stress-mem` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140210.738638Z-user-smoke-stress-mem-107814` |
| `kernel-smoke-stress-mem` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140222.377180Z-kernel-smoke-stress-mem-107852` |
| `busybox-init-login-native` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140233.121170Z-busybox-init-login-native-107908` |
| `rc-local-linux` | diagnostic / inconclusive | `impl/arceos_ex/tests/basic/out/20260718T140328.459779Z-rc-local-linux-108233` |
| `rc-local-native` | acceptance / passed | `impl/arceos_ex/tests/basic/out/20260718T140339.835214Z-rc-local-native-109618` |
| `distro-sh-checkpoints-linux` | diagnostic / inconclusive | `impl/arceos_ex/tests/basic/out/20260718T140351.996282Z-distro-sh-checkpoints-linux-109666` |
| `distro-sh-checkpoints` | diagnostic / inconclusive | `impl/arceos_ex/tests/basic/out/20260718T140403.114756Z-distro-sh-checkpoints-111054` |
| `busybox-init-checkpoints-linux` | diagnostic / inconclusive | `impl/arceos_ex/tests/basic/out/20260718T140416.449895Z-busybox-init-checkpoints-linux-111102` |
| `busybox-init-checkpoints` | diagnostic / inconclusive | `impl/arceos_ex/tests/basic/out/20260718T140431.259601Z-busybox-init-checkpoints-112487` |

## Stress 与历史 baseline

默认三项、两个 `stress-mem` case 和 BusyBox-init focused case 以同一 composite invocation 各运行一轮：

```sh
make test-stress \
  STRESS_CASES="impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml impl/arceos_ex/tests/stress/cases/df-0003-distro-sh-ls.toml impl/arceos_ex/tests/stress/cases/df-0001-user-boot-stress-mem.toml impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall-stress-mem.toml impl/arceos_ex/tests/stress/cases/busybox-init-login-focused.toml" \
  STRESS_RUNS=1
```

六个 schema-v2 report 均为 `completed_runs=1`、`success=1`、`failure=0`；每轮 nested basic 的
manifest/result/QEMU log 完整，standalone gate 与 cleanup 通过。

| case | class | report 目录 |
| --- | --- | --- |
| `df-0001-user-boot` | `user-boot-success` | `impl/arceos_ex/tests/stress/out/20260718T140539.818796Z-df-0001-user-boot` |
| `df-0002-smoke-initcall` | `smoke-success` | `impl/arceos_ex/tests/stress/out/20260718T140540.800537Z-df-0002-smoke-initcall` |
| `df-0003-distro-sh-ls` | `distro-sh-ls-success` | `impl/arceos_ex/tests/stress/out/20260718T140542.357165Z-df-0003-distro-sh-ls` |
| `df-0001-user-boot-stress-mem` | `user-boot-success` | `impl/arceos_ex/tests/stress/out/20260718T140543.589759Z-df-0001-user-boot-stress-mem` |
| `df-0002-smoke-initcall-stress-mem` | `smoke-success` | `impl/arceos_ex/tests/stress/out/20260718T140544.472423Z-df-0002-smoke-initcall-stress-mem` |
| `busybox-init-login-focused` | `busybox-init-login-focused-success` | `impl/arceos_ex/tests/stress/out/20260718T140545.649782Z-busybox-init-login-focused` |

首个 `df-0001-user-boot` report 随后作为同 identity 的新 schema-v2 baseline：

```sh
make test-stress \
  STRESS_CASES=impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml \
  STRESS_RUNS=1 \
  STRESS_BASELINE=impl/arceos_ex/tests/stress/out/20260718T140539.818796Z-df-0001-user-boot
```

新报告位于
`impl/arceos_ex/tests/stress/out/20260718T140558.343886Z-df-0001-user-boot`。Runner 接受相同 case、
basic identity、classifier hash `1cab6723170332d95dfb0c6a1ea78cc43f86c594de102ad0187fc761795da852`
和 config fingerprint `d1f0520cfcfe360d7ccb7fcd161e28bd930a7fd5111b65a0393f3696c3e2bd86`；
baseline aggregate content hash 为
`9548b4fd2835eb6b4b0a0ee619d22375e62645332a989fa2e4289e6aee377070`。本次 failure rate 仍为 0，
delta 为 0，class/sequence 无增删，最近序列 `first_divergence=null`，且
`affects_exit_status=false`。

## Paired difftest

三组 difftest 各运行一轮；每次 preflight 均确认 485 个 checkpoint mapping 当前有效，其中 exact 103、
range 14、unmapped 368，并确认 103 个 Linux marker 无 missing、stale 或 mismatch。

```sh
make difftest DIFFTEST_RUNS=1
make difftest \
  DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/linux-exact-baseline-difftest.toml \
  DIFFTEST_RUNS=1
make difftest \
  DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/busybox-init-difftest.toml \
  DIFFTEST_RUNS=1
```

| case | compared checkpoints | diff 结果 | report 目录 |
| --- | ---: | --- | --- |
| `rc-local-difftest` | 68 / 68 | passed；first divergence null；missing/extra 均为空 | `impl/arceos_ex/tests/stress/out/20260718T140630.770413Z-rc-local-difftest` |
| `linux-exact-baseline-difftest` | 69 / 69 | passed；first divergence null；missing/extra 均为空 | `impl/arceos_ex/tests/stress/out/20260718T140707.001882Z-linux-exact-baseline-difftest` |
| `busybox-init-difftest` | 55 / 55 | passed；first divergence null；missing/extra 均为空 | `impl/arceos_ex/tests/stress/out/20260718T140726.753657Z-busybox-init-difftest` |

每个 report 的 `runs/run-0001/{left,right}/` 都包含两侧完整 basic artifacts；两侧均
`gate_passed=true`、`execution_status=completed`、`expectations_passed=true`，private disk 和进程组
cleanup 成功。rc.local 的 explicit-accounting coverage 同时确认 exact required 103 项中 scope 内 58、
scope 外有理由记录 45、unaccounted 0；三个 paired result 均为 `paired-checkpoint-diff-ok`。这也验证了
当前实现只有在两侧 standalone gate 与 checkpoint diff 同时成功时才报告成功，单侧失败传播语义则由
已有 composite runner 单元测试继续覆盖，本轮没有制造失败来改变真实 scope。

## 收口边界

本轮只要求一轮真实编排验收，不把 ordinary 30 轮或 `stress-mem` 500 轮提升为归档门槛。DF-0001、
DF-0002、DF-0003 ordinary/stress-mem 多轮复现和 Linux paired difftest 继续由主 roadmap 的 nightly、
长期回归条目承担。

归档变更后的最终 gate 在仓库根目录直接执行 `make test`，结果为 overall 178/178 passed；随后
`git diff --check` 返回 0。`pgrep -af qemu-system-riscv64` 无输出，basic/stress out 下查找 `*.raw`
也无结果，确认没有 QEMU 进程或 private disk 残留。本归档只提交文档收口，不提交 ignored 的运行
artifact。
