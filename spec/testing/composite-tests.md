# Composite basic-test orchestration

本文档规定 stress 与 difftest 的复合边界。复合 runner 不是第三种 kernel/QEMU runner；它只编排
[`basic-tests.md`](basic-tests.md) 规定的独立基本测试并分析其冻结结果。

## Schema v2 only

所有 checked-in composite TOML 必须声明 `schema_version = 2`。不保留任意 command schema v1
执行兼容，也禁止 `command`、`setup_command`、working directory、env、delayed stdin、private disk、
Linux build argv 和 QEMU argv。未知字段必须在任何 build/disk/QEMU 动作前失败。

Stress case 顶层只允许 `schema_version`、`name`、`description`、`mode = "stress"`、一个
basic `test`、`runs`、`classifier` 和 `metadata`。每轮在固定
nested artifact 目录运行同一个 basic test，之后只读取其 `result.json` 与 `qemu.log`。Stress 保留 defect
classifier、事件提取、序列聚类和本次 failure-vs-success 分析。

Difftest case 顶层只允许 `schema_version`、`name`、`description`、`mode = "difftest"`、
`runs`、`left_test`、`left_label`、`right_test`、`right_label`、`checkpoint_scope`、可选
`checkpoint_scope_max_counts`、`checkpoint_coverage` 和 `metadata`。每轮必须先完整执行 left，
再完整执行 right；
两侧各自拥有 basic manifest/result/QEMU log/private-copy/process-group/cleanup。即使一侧失败也继续保留
另一侧可获得的独立诊断，但只有两侧 `execution_status = "completed"`、expectations 可用且通过、并且
checkpoint diff 通过时，本轮才成功。Difftest 不得自行构建 Linux、启动或回收 QEMU、复制磁盘或发送
stdin。

## Entry points and runs

保留 `STRESS_RUNS`、`STRESS_CASES`、`DIFFTEST_RUNS` 和 `DIFFTEST_CASE`。删除 composite timeout 覆盖；
timeout 只来自 basic TOML。非零 runs 的顶层 stress/difftest invocation 在执行任何 selected case 前统一
运行一次 canonical `make disk`；runs=0 只校验所有 composite 配置及其引用的 basic TOML。Difftest
checkpoint preflight 仍先于 paired runner。

basic runner 由 composite 传入精确 `--output-dir`：stress 为
`runs/run-NNNN/basic`，difftest 为 `runs/run-NNNN/left` 和 `runs/run-NNNN/right`。Composite 不从
stdout 推断 artifact，也不采信 basic 目录之外的 QEMU 日志。

## Historical stress baseline

`STRESS_BASELINE=<report-dir>` 只允许与单个 selected stress case 一起使用。Runner 必须拒绝 schema v1
历史报告，并校验同 case、同 basic identity、同 classifier/config fingerprint。Baseline 路径及全部比较
输入的内容 hash 冻结进新 manifest。报告比较 failure rate、class/sequence 增删及最近序列的首个分歧；
历史差异只提供分析信息，不额外改变当前 invocation 的退出状态。
