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

Classifier rule 只允许 `id`、`result`、`contains`、`regex`、可选布尔 `timed_out` 和
`description`。`timed_out = true` 必须读取 basic `result.json` 的结构化 timeout 状态，不能用 QEMU
signal-15 文本代替，因为 marker 正常退出也会由 host 终止 QEMU；该条件与文本条件必须同时满足。

`rc-local-native-timeout-focused` 是默认 stress suite 中的 DF-0004 timeout 长期观察项：它只重复正式
`rc-local-native` basic identity，case-local 深采样轮次为 100；根 Make 入口默认以统一
`STRESS_RUNS=10` 采样它和其它默认 case，且不把 stress suite 纳入根 `make test`。它不得复制或覆盖
basic 的 180 秒 timeout、QEMU 参数、probe、rootfs 或期望；每轮必须保留完整 basic artifact，发生
timeout 时连同 `qemu-timeout-diagnostics.json` 一并冻结。该项的目的首先是取得可复现的停机
PC/寄存器/IRQ 证据；连续成功只能报告本次未复现和观测上界，不能证明 UART、PLIC、SBI、sandbox
或其它候选根因。

`df-0005-busybox-init-login-native` 是同一默认 suite 中的 BusyBox init/login timeout 长期观察项。
它只重复正式 `busybox-init-login-native` basic identity，case-local 深采样轮次为 100，并同样接受
顶层 `STRESS_RUNS` 覆盖。它不得修改三段 scripted stdin、180 秒 timeout、rootfs、provider、QEMU
参数或 acceptance marker。timeout 必须保存完整 basic artifact 和 QMP snapshot；已完成登录、目录
输出但未到达 wrapper `sync`/guest shutdown 的样本仍是失败，随后一次成功不能抵消或关闭该缺陷。

`df-0006-user-smoke-preempt-native` 是默认 suite 中独立的 native user-smoke preemption/pipe timeout
长期观察项。它只重复正式 `user-smoke-native` basic identity，case-local 深采样轮次为 100，不修改
120 秒 timeout、8 CPU、provider、rootfs、payload 或 acceptance marker。首次冻结证据在两个 compute
child 都打印 write-complete 后停止，coordinator 没有打印 bytes-collected；timeout 时必须保留完整 basic
artifact 与 QMP snapshot。DF-0001 对同一 basic identity 的旧 ELF-read 分类不能替代这个新边界，且后续
成功样本不能抵消或关闭 DF-0006。

`df-0007-ltp-frontier-child-wait-native` 是默认 suite 中独立的 native LTP frontier 调度失败观察项，
`df-0007-ltp-frontier-child-wait-linux-object` 是同一 defect 的非默认第二 provider 入口。两者只重复正式
basic identity，日常 case-local 样本均为 50；仅在概率问题尚未定位时按证据把专项批次扩到
100/200/300/500。两者不修改 stock LTP harness、frontier 精确名单、300 秒 timeout、
8 CPU、provider、canonical rootfs 或完成 marker。首次两侧样本都在 `uname01` 已打印两条 TPASS 后，
把当前 SMP wait/reap 错送入 PID1 legacy simulated dispatch。单变量负对照恢复旧路由后首次运行即恢复
相同 TaskRef/generation 形状和 `live-sp-in-user-carrier-stack` 首失败；当前实现把 generation-checked SMP
父进程严格绑定到自己的 registry child。修复前后保留两个 provider 压力入口。

`df-0015-ltp-frontier-console-record-interleave-native` 是默认 suite 中独立的 native LTP frontier
console record 原子性观察项。它只重复正式 `ltp-frontier` basic identity，日常 case-local 样本为
50 轮，不修改 stock harness、四项精确名单、8 CPU、native provider、rootfs、timeout 或 marker。
成功必须同时看到四条从行首到行尾精确匹配的 stock runner PASS 记录、精确 clean summary 和完成
marker；任何 PASS 被拼接到 `wait4 registry` 诊断中的历史形状必须单独分类为 DF-0015，不能因 LTP
内部 TPASS 已出现而算作成功。100/200/300/500 只在该概率问题仍未定位或将来以相同 class 复发时
用于逐档定位，修复后的常规压力门禁仍为 50；Linux/native 横向差分最多 10 轮。

`df-0008-user-smoke-fork-enqueue-linux-object` 是默认 suite 中独立的 linux-object user-smoke fork
publication 失败观察项。它只重复正式 `user-smoke-linux-object` basic identity，case-local 深采样轮次为
100，不修改 120 秒 timeout、8 CPU、provider、rootfs、payload 或 acceptance marker。首次样本在
`fork_mm` 重复 COW fork 中已完成 child snapshot publication，但 `mark_enqueued` 拒绝，随后输出
`declared child enqueue invariant failed`。该项与 native DF-0006 的 pipe-read timeout 是不同边界；
修复前后均保留，后续成功样本不能抵消或关闭 DF-0008。

`df-0009-user-smoke-preempt-linux-object` 是默认 suite 中独立的 linux-object user-smoke
preemption/pipe timeout 长期观察项。它只重复正式 `user-smoke-linux-object` basic identity，case-local
深采样轮次为 100，不修改 120 秒 timeout、8 CPU、provider、rootfs、payload 或 acceptance marker。
首次冻结证据来自 DF-0008 深采样的第 13 轮：`fork_mm` 已完整通过，两个 compute child 均打印
write-complete，但 coordinator 未打印 bytes-collected，QMP 捕获时 VM 仍在运行。该形状与 native
DF-0006 相同，但 provider 与长期入口独立；它也不得替代不同边界的 DF-0008 enqueue failure。
修复前后均保留，后续成功样本不能抵消或关闭 DF-0009。

`df-0010-ltp-frontier-post-read-runqueue-linux-object` 是默认 suite 中独立的 linux-object LTP
frontier 后续 child runqueue-publication 失败观察项。它只重复 diagnostic basic identity
`ltp-frontier-post-read-lo`，case-local 深采样轮次为 50：先原样重放首次样本的长度检查、带三个
不存在参数的失败 hash child、由 Ctrl-C 中断的未完成命令，再核对 stock `run-syscalls.sh` 的固定
SHA-256 并执行 `sh -n`；之后通过原项目 selector 运行未修改的 frontier 精确名单，最后由同一父
shell 启动第二次 SHA-256 读取。首次样本在四项 TPASS 及 stock harness
第 203 行 parse error 后，第二个 hash child 终止于 `declared child runqueue publish invariant failed`。
classifier 必须把带 `Scheduler/Enqueue.Publish/TaskRunqueue` 首失败诊断的样本与早先的 harness parse
error、unsupported syscall、TWARN 和 DF-0007/DF-0008 分开保存。不得更换 provider、SMP、rootfs、
名单、harness、命令顺序或 marker；修复前后均保留该压力入口。

`df-0011-user-smoke-preempt-finalize-linux-object` 是默认 suite 中独立的 linux-object user-smoke
晚期 finalization timeout 观察项。它只重复正式 `user-smoke-linux-object` basic identity，case-local
深采样轮次为 100，不修改 120 秒 timeout、8 CPU、provider、rootfs、payload 或 acceptance marker。
首次样本已经打印 bytes-collected、children-reaped 与 A-B-A round-robin success，却未打印 preempt case
end 或 guest exit；timeout classifier 必须用该完整终态与 host termination marker 区分 DF-0009 的更早
pipe-read 停顿。修复前后均保留，后续成功样本不能抵消或关闭 DF-0011。

`df-0012-rc-local-direct-setup-native` 与
`df-0014-checkpoint-cross-cpu-reentry-native` 共享正式 `rc-local-native` basic identity，但必须
保持两个独立 classifier 和 defect。DF-0012 只识别到达 `DmaCachePolicy.Ready` 后、
`Kernel.Online` 前的 direct-setup 主动关机；DF-0014 只识别已到达 `Kernel.Online` 和
`lkm-rc-local: begin` 后的 `checkpoint reentry`。一个 case 如果交叉触发另一类失败，
classifier 应保留对方的精确 class，不得折叠成 `nonzero-exit`。DF-0014 的冻结样本
来自 DF-0012 最终 500 轮的 `run-0448`；深采样因已观测到约 1/500 失败而固定为
500 轮，默认 suite 仍受顶层 `STRESS_RUNS=10` 覆盖。修复后依然使用
`50 -> 100 -> 200 -> 300 -> 500` 阶梯独立验证。

DF-0012 的首失败已由通用诊断锁定为 `boot_task.stack_guard_intact`，并由诊断前构建的硬件写观察点
定位到 `CpuGroup::setup_smp` 的完整 `Cpu` 栈上构造。修复为权威 slot 就地构造后，该 case 的
case-local 深采样同样固定为 500 轮；默认 suite 的顶层覆盖规则不变。其成功条件、原失败
classifier、provider、SMP、rootfs 和 timeout 均不得因已修复而放宽。

Difftest case 顶层只允许 `schema_version`、`name`、`description`、`mode = "difftest"`、
`runs`、`left_test`、`left_label`、`right_test`、`right_label`、可选 `checkpoint_scope`、
`checkpoint_scope_max_counts`、`checkpoint_coverage`、`observable_scope`、`observable_patterns`
和 `metadata`。`checkpoint_scope` 与 `observable_scope` 至少一个非空。每轮必须先完整执行
left，再完整执行 right；
两侧各自拥有 basic manifest/result/QEMU log/private-copy/process-group/cleanup。即使一侧失败也继续保留
另一侧可获得的独立诊断，但只有两侧 `execution_status = "completed"`、expectations 可用且通过、并且
全部已配置的 diff 通过时，本轮才成功。Difftest 不得自行构建 Linux、启动或回收
QEMU、复制磁盘或发送 stdin。

`observable_scope` 是有序、无重复的稳定行为 token；`observable_patterns` 必须为其每一项
提供且只提供一个 Python regular expression。Runner 对两侧的 basic `qemu.log` 做 ANSI 与
换行归一化，按 match 位置提取 token 序列；命令回显、重复 marker、缺失 marker 或顺序
不同都必须形成差分，不得用 `contains` 式的宽松集合判定掩盖。用户态/Linux 差分
可以只配置 observable diff；实现内部生命周期差分可同时配置 checkpoint diff。两类差分
都必须把两侧序列、missing/extra 和首个分歧写入每轮 artifact。

## Entry points and runs

未闭合的概率性 DF case 严格串行处理，不把多个缺陷的行为修改或深采样混在同一轮。队列按冻结 artifact
中的实际失败率从高到低排序；确定性复现视为最高优先级。每个 case 先取得可重复首边界，并在适用时用
sibling Linux、native 与 linux-object 的同输入 observable/checkpoint 差分排除 provider 或 ABI 差异，
再按 Charter -> Model -> Coding -> Impl 闭合。该 case 未稳定前不得推进下一 case 或扩大 LTP supported
集合。

DF-0013 使用一个默认 native stress identity 和一个显式选择的 linux-object stress identity；两者引用
同一 classifier、同一 canonical rootfs、8 CPU、三次外部 `dd` 读取、120 秒 timeout 与完成 marker，
linux-object identity 不扩大默认 suite，也不建立新的 defect 编号。其 sibling Linux/native difftest 使用
无 probe 的用户可见行为 basic identities，避免内核诊断串口记录插入 `printf` 与 child 输出之间；诊断
identity 可在压力失败后复现首边界，但不得放宽精确的 `R1=#!/`、`R2=bin`、`R3=/sh` observable。

`50 -> 100 -> 200 -> 300 -> 500` 逐级扩样只用于尚未定位的概率问题：增加轮数的目的必须是复现并
取得新的首边界证据。任一级出现一次失败即停止扩样，保留该轮完整 artifact，并回到首边界定位；后续
成功样本不能抵消这次失败。根因已由可重复边界或单变量负对照确定后，修复验证回到该 case 规定的常规
轮数，不得机械继续 100/200/300/500。即使定位阶段达到 500 轮，也只能证明冻结负载下的观测结果，
不得声称统计上消除了所有可能竞态；长期 case 与 classifier 继续保留。

Difftest 的职责是横向比较 sibling Linux 与 arceos_ex 的同输入 observable/checkpoint 语义，不承担以大量
重复轮次发现小概率调度竞态的职责。每个 difftest case 默认不超过 10 轮，case-local `runs` 与
`DIFFTEST_RUNS` 覆盖均不得大于 10；稀有失败的复现和概率收敛必须放在对应 provider stress identity 中。
只有要求的 provider 最高级别零失败、零 timeout、零未分类结果，difftest 在最多 10 轮内完全一致，且根
回归通过，才可把该 DF 标记为本轮稳定。

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
