# tools2 确定性 TaskFlow/CpuLane 与 SMP schedule replay

## 目标与责任边界

本专题规划 tools2 对可挂起、可恢复的 TaskFlow execution lane 以及确定性 SMP 推导/重放的后续扩展。
它只保存完整设计与未来闭合条件；当前优先级和状态以
[`../ROADMAP.md`](../ROADMAP.md) 的唯一活跃条目为准。

本专题只负责两类相互依赖的语义：

- `Scheduler.Schedule` 返回点上的 TaskFlow lane 挂起、首次派发、恢复和同一 occurrence 校验；
- 多个 CpuLane 的全局确定性仲裁、cross-CPU mailbox、迁移事务、schedule artifact 与严格 replay。

通用 Signal 在被接受后等待任意未来 Signal 的 pending、timeout、cancel、队列所有权和 snapshot 续跑，
仍属于 [`signal-driven-tools2.md`](signal-driven-tools2.md) 的独立 continuation 里程碑。普通
`depends_on` 不成立继续表示拒绝；本专题不得把任意 guard failure 自动改写为等待。

本轮只登记 roadmap，不修改 Charter、Model、Coding、Compose、Impl、Testing、tools2 协议或生成产物。
未来实施每一批都必须重新执行 charter-first。锁定的
[`computer.md`](../../spec/charter/systems/computer.md) 不因本专题或主 roadmap 条目获得编辑授权。

## 当前权威语义基线

lane 设计必须从现有 Scheduler、Task 与 TaskFlow 权威语义向下细化，不能从旧提案或当前工具实现反推：

| 边界 | 必须保持的语义 |
| --- | --- |
| Task `Online` | 普通 Task 已发布且具有绑定 initial Flow 的有效首次上下文，但从未获得 CPU；不表示 runnable、on-rq 或已经启动 Flow。 |
| Task `Suspended` | Task 曾经执行，保存了绑定唯一 active Flow 的有效可恢复上下文；它可以是 runnable，也可以仍处于 blocked。 |
| 首次非 identity 派发 | Scheduler 先 `drives Task.Activate`，Task 提交 `Online/None/Valid -> OnCpu/Live/Invalid`；Scheduler 随后直接向预检固定的 initial Flow `emits Startup`（canonical `Preset`）。 |
| 恢复非 identity 派发 | Scheduler 先 `drives Task.Continue`，Task 提交 `Suspended/None/Valid -> OnCpu/Live/Invalid`；Scheduler 随后直接向预检固定的 active Flow `emits Continue`。 |
| identity schedule return | 不进入 SwitchTo，不保存/恢复 context，不驱动 Task lifecycle；Scheduler 只向原 active Flow `emits Continue`，恢复原 Flow 的同一 schedule occurrence。 |

Task 的 Activate/Continue 只提交自身 lifecycle、authority 与 breakpoint，不转发 Flow Startup/Continue。
TaskFlow lane 必须保持这一 sender/receiver path，不能增加 Task 中继层。Schedule 发出、PreparePrev 和
PickNextTask 本身也不改变 Task lifecycle；只有真实 `next != prev` 的 switch 才保存 prev 并推进两侧
Task 状态。

## 确定性执行架构

### 职责分层

```text
ScheduleGenerator
  -> immutable ScheduleArtifact
      -> Deterministic GlobalArbiter
          -> CpuLane[cpu]
              -> selected TaskFlowLane
```

`ScheduleGenerator` 只产生和固化选择序列。`GlobalArbiter` 是唯一语义提交者，一次只推进一个 CPU 的
一个原子步骤。`CpuLane` 表示 CPU-local Scheduler、current binding、class queue 与 mailbox；
`TaskFlowLane` 表示全局稳定 Task/Flow identity 上的可恢复 continuation。CpuLane 可以持有当前 granted
lane 的引用，但不能拥有、复制或因迁移重建 TaskFlowLane。

宿主实现不得借助真实线程竞争、`asyncio` 默认调度、墙钟或 Python hash iteration 决定语义顺序。
实现可以用普通函数组织代码，但 replay 的每次选择和 resume 都必须由 GlobalArbiter 显式提交。

### TaskFlowLane

lane 的稳定键由以下四元组组成：

```text
(TaskRef.slot, TaskRef.generation, TaskFlowRef.slot, TaskFlowRef.generation)
```

可序列化状态至少包括：

```text
status: Ready | Running | AwaitingScheduleReturn | Completed | Failed
effective_cpu_ref
handler_id
member_path / program_counter
serializable bindings
wait_reason
schedule_occurrence
last_committed_signal_occurrence
```

continuation 只能保存 Model handler/member path、显式 program counter、值绑定、稳定引用与等待原因。
Python coroutine/generator frame、宿主线程栈、对象地址、闭包 identity 或未提交 handler locals 都不能
进入 snapshot、fingerprint 或 replay 输入。

### CpuLane

每个 CpuLane 至少保存：

```text
CpuRef and CPU lifecycle
SchedulerRef
CurrentTaskRef / CurrentFlowRef / CurrentStack binding
CPU-local class queues
remote Signal mailbox
CPU-local occurrence sequence
currently granted TaskFlowLane ref
```

这些字段必须通过现有 stable refs 和 per-CPU ownership 解析。Task/TaskFlow identity 位于全局 registry；
CpuLane 与 Scheduler 队列只保存稳定引用，不形成第二套 carrier 或 continuation storage。

## Schedule return 与 lane 恢复

当前 active TaskFlow 发出 `Scheduler.Schedule` 后，未来 lane-aware derive 在发送动作提交处建立绑定该
Schedule occurrence 的 `AwaitingScheduleReturn` token。该 token 是明确 Schedule-return 边界，不是
通用 Signal pending。

Scheduler 继续执行正式主序：

```text
PreparePrev(prev)
PickNextTask(prev, disposition)
if next != prev: SwitchTo(prev, next)
else: resume original active Flow
```

- identity 路径保持 Task、context、CurrentTask/CurrentStack 和 lane identity 不变；原 active Flow 的
  Continue 只消费匹配 occurrence 的等待 token。
- 非 identity 路径先完成 prev SaveCoreContext/Suspend，再恢复 next context、提交 CPU-local binding 并在
  next stack 完成 finish。next 是 Online 时只允许 initial Flow Startup；next 是 Suspended 时只允许
  active Flow Continue。对应 Flow Signal 被接受后，目标 lane 才从 Ready/等待状态继续。
- sleeping/blocked prev 的 lane 可以继续等待，但 wake/enqueue 只恢复调度资格；只有后续真实 switch 与
  匹配 Flow continuation 才恢复执行 lane。
- stale Flow/Task ref、错误 generation、错误 CPU、错误 CurrentTask binding、错误 active/initial Flow、
  occurrence 不匹配或重复 Continue 必须在唤醒 lane 前拒绝，并保持稳定 snapshot 不变。

## GlobalArbiter 与 SMP 因果

GlobalArbiter 每次提交一个原子语义步骤。普通同步 `drives` 子树仍在同一同步响应规则下完成；只有规格
明确的 Schedule return、yield 或 wait 边界才能把 continuation 持久化后交还仲裁器。post-commit
`emits` 才进入全局 FIFO 或目标 CpuLane mailbox。

当一个稳定边界只有一个 CpuLane enabled 时，Signal cause/FIFO 事实直接决定下一步；至少两个 CpuLane
同时 enabled 时才创建 choice point。每个 choice point 必须记录完整 enabled set，由 schedule artifact
选择 CPU。CPU 内的 next Task 仍由该 CPU Scheduler 的 class/queue 规则决定，artifact 不得绕过
PreparePrev、PickNextTask 或 class handoff。

互不影响的 CPU steps 可以在 Model 中没有 happens-before，但 canonical replay 仍按 artifact 固定顺序
逐一提交并在产物中保留“无顺序约束”的证据。若多个 lane 竞争同一 lock、Task、migration 或共享对象，
而 Model 没有给出顺序，derive 必须形成 unresolved ordering obligation 或显式枚举的确定分支；不得用
隐式 cpu-id 排序掩盖语义缺口。

## Cross-CPU mailbox

remote wake、IPI、timer 或 scheduler request 采用统一因果链：

```text
source CpuLane commits emit
  -> global FIFO occurrence
  -> destination CpuLane mailbox position
  -> destination consumption at a recorded choice point
```

结构化产物必须同时保留全局 occurrence、source CPU local sequence、destination mailbox position、cause、
enabled set 和实际消费 step。异步接收方不继承 source Flow 的 effective CPU context，而在消费时从目标
CpuLane 的稳定 binding 重新解析。相同输入重放时，上述字段必须逐项一致。

## Task migration 事务

TaskFlowLane 保持在全局 Task registry。迁移是 source/destination 两个 Scheduler 之间不可交错的正式
事务，稳定 snapshot 不得暴露能被普通 Signal 观察或执行的半提交状态：

```text
preflight task and dispatch Flow
  -> acquire/validate source and destination scheduler state
  -> remove TaskRef from source class queue
  -> commit selected dispatch Flow cpu_ref and scheduler placement
  -> enqueue TaskRef into destination class queue
  -> optionally enqueue remote notification
```

迁移预检必须按 Task 状态固定将来派发使用的 Flow：Online Task 选择其 Base initial Flow；Suspended Task
选择唯一 Online active Flow。事务只更新这条预检选定 dispatch Flow 的 `cpu_ref` 与 Scheduler placement；
TaskRef、TaskFlowRef、两者 generation、lane key、program counter、schedule occurrence 和 continuation
identity 全部不变。

迁移还必须满足：

- Task 当前不持有 CPU 执行权；OnCpu Task 必须先真实切出；
- TaskRef 任一时刻最多存在于一个 Scheduler class queue；
- destination 在 commit 前不可 pick，source 在 commit 后不可使用 stale membership；
- CurrentTask/CurrentStack 只在 destination 以后实际 SwitchTo/Restore 时改变；
- destination 首次派发 Online Task 时走 Activate + initial Flow Startup，恢复 Suspended Task 时走 Continue
  + active Flow Continue；
- 双 rq-lock 获取、验证、移除、cpu_ref 更新和入队作为一个不可交错事务提交。

动画未来可以投影 transaction entry/commit/exit，但不能反向把显示帧变成中间可执行状态。

## Schedule artifact 与 v10 协议族

schedule artifact 使用独立协议身份：`schema = "lkm.spec.schedule"`、`producer = "tools2"`、
`version = 1`。最小结构为：

```json
{
  "schema": "lkm.spec.schedule",
  "version": 1,
  "producer": "tools2",
  "model_fingerprint": "sha256:...",
  "scenario_fingerprint": "sha256:...",
  "generator": {
    "kind": "manual|seeded-random|bounded|systematic",
    "version": 1,
    "seed": 12345
  },
  "decisions": [
    {
      "choice_point": "cp-0001",
      "enabled_cpu_lanes": [0, 1],
      "enabled_set_fingerprint": "sha256:...",
      "selected_cpu": 1,
      "expected_signal_occurrence": "sig-0042"
    }
  ]
}
```

seed 只解释 artifact 如何生成；replay 必须消费完整 decisions，不能重新运行 generator。每个 choice point
都校验 enabled set、CPU/Task/Flow generation、expected Signal occurrence、artifact cursor 和前序稳定
状态。任何不一致返回结构化 `schedule_divergence`，不得跳过 decision、重编号或自动选择另一 lane。

lane、choice point、artifact cursor、mailbox 与迁移事实会改变各阶段公开结构，因此未来 lane-aware
AST、Model、Derive、Check、View、Snapshot 必须作为同一个 tools2 v10 协议族统一规划和升级；不能只让
部分消费者接受新字段。schedule artifact 自身保持上述 version 1。当前仓库仍是严格 v9 消费者，尚未
接受任何 lane、mailbox、cursor 或 v10 snapshot 字段；本专题不表示 v9 已具有这些能力。v9/v10 必须
互相严格拒绝，不能静默升级或兼容读取。

给定同一 Model、scenario、完整 schedule artifact 和预算，AST 之后的 derive/check/view 及适用展示
输入必须字节级可重演。canonical stage-entry snapshot 仍只能在原子 handler 之间生成；已经提交的
Schedule-return lane token 可以进入 v10 snapshot，未提交 ancestor、正在执行的 Python frame 或未消费
FIFO handler 不得进入。

## 七阶段实施路线

1. **UP lane 基础**：建立可序列化 TaskFlowLane、program counter、Schedule occurrence 与 identity/non-
   identity 恢复；用负例证明普通 guard rejection 不会变成等待。
2. **多 CpuLane 仲裁**：建立 GlobalArbiter、per-CPU current/scheduler/mailbox 与 manual artifact；先在
   无迁移场景中重放多个独立 TaskFlow。
3. **严格 replay 协议**：统一升级 tools2 v10 AST/Model/Derive/Check/View/Snapshot，加入 artifact
   读写、fingerprint、cursor、divergence 和重复重放的 canonical-byte 验收。
4. **Cross-CPU Signal**：闭合 remote wake/IPI/mailbox 的发送、排队、choice 与消费因果链。
5. **Migration**：闭合两个 Scheduler 的原子事务、按 Task 状态选择 dispatch Flow、`cpu_ref` commit 与
   destination dispatch。
6. **Schedule generator**：依次提供 manual、seeded-random、bounded、systematic/coverage-guided；所有
   generator 都先固化 artifact，再调用同一个 replay engine。
7. **展示与接管判定**：JSON、text、view、animation 和 testing 各自闭合后，才评估 tools2 lane/SMP
   验收责任；旧工具仍按既定逐责任退役流程处理。

每一阶段开始前都必须重新确定最高受影响核心层，按 Charter → Model → Coding → Impl 关闭，并单独复核
Compose 与 Testing。若需要调整锁定 Charter，必须取得该文件的明确授权并走 lock tool；roadmap 登记
不是授权。

## 验收矩阵

- 单 CPU identity Schedule 恢复原 lane 和同一 occurrence，无 Task/context switch。
- 单 CPU non-identity 分别覆盖 Online next 的 initial Startup 和 Suspended next 的 active Continue。
- blocked prev 在 wake/enqueue 后只恢复资格，直到后续真实 switch 才恢复原 continuation。
- stale ref/generation、错误 CPU/binding/Flow、重复或错误 occurrence 全部拒绝且 snapshot 不变。
- 两 CPU 按 manual artifact 交错，重复运行产生 byte-identical 结构化产物。
- 同 generator version/seed 产生同一固化 artifact；replay 不调用随机 generator。
- enabled set、fingerprint、Signal occurrence 或 cursor 变化均产生精确 schedule divergence。
- cross-CPU emit 的 global occurrence、mailbox position 与消费 choice 可逐项复核。
- Online/Suspended migration 分别更新 initial/active dispatch Flow，且无双 on-rq、双 OnCpu 或半提交状态。
- 无 Model 顺序的共享 lock/migration 竞争产生明确 obligation 或确定分支，不使用隐式 CPU tie-break。
- v9/v10 与 schedule artifact 的 schema/version/producer 错配被严格拒绝。
- v10 snapshot 不含宿主 frame、对象地址、未提交 handler 或隐式 pending Signal。

未来实现代码变更后，focused tools2 pipeline/check/view/animation、`make -C tools2 test-all`、
`make verify`、`make charter-lock-check`、`git diff --check` 和仓库根直接 `make test` 都是适用门禁；
具体组合按阶段风险增加，不能用 replay 成功替代 core 语义和负例验证。

## 首轮非目标

- 不修改或要求旧 `tools/` 理解 lane、mailbox、artifact 或 v10；旧工具继续承担现有 shadow/门禁责任。
- 不修改 Linux checkpoint 或把 checkpoint mapping 写入 Coding；需要 Linux 差分时另按 Testing/
  cross-reference 责任立项。
- 不使用真实宿主线程并行模拟 SMP，也不以运行速度为由改变 arbiter 的确定提交顺序。
- 不完成全部 SMP interleaving model checking、Linux load balance、公平性、带宽、CPU hotplug teardown 或
  SCX；systematic generator 的首轮范围必须有显式 bound。
- 不把一次 random schedule 成功当作 SMP 正确性证明；每个 artifact 只证明对应的确定执行序列。
- 不让 text、animation 或旧工具输出覆盖结构化 derive/check 的语义结论。

## 本轮层级复核记录

- Charter：reviewed, no change。核对 Scheduler、Task、TaskFlow、SMP/runtime 与 tools2 Signal 章程；
  `computer.md` 保持 locked 且未修改。
- Model：reviewed, no change。核对 Scheduler dispatch、Task Online/Suspended、TaskFlow CpuRef/Continue 与
  当前 v9 Signal 推导边界。
- Coding：reviewed, no change。核对 Scheduler/Task/TaskFlow lowering 和 tools2 v9 协议。
- Impl：reviewed, no change。当前 tools2 schema 常量仍为 v9，且没有 lane/CpuLane 实现。
- Compose：reviewed, no change。未来 lane 是 tools2 内部结构，本轮不改变内核组件边界。
- Testing：reviewed, no change。现有 scheduler、Task/TaskFlow、CPU 合同继续作为未来验收上界；本轮不
  修改测试或 snapshot。
