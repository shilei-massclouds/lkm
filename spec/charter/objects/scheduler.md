# Scheduler

`Scheduler` 是 Linux 每 CPU `struct rq` 的对象语义对应物。每个 possible `CPU` 恰好拥有一个
Scheduler；它不是全局 singleton，也不再与一个独立 `RunQueue` 对象形成两套 CPU-local 拓扑。Scheduler
拥有 CPU-local lock、`curr/idle/stop` 引用和 stop/DL/RT/fair/idle 五类队列。root domain、sched
domain 与其它跨 CPU 协调资源是独立共享对象，只通过 `CpuRef`/`SchedulerRef` 覆盖 CPU 集合，不拥有或
复制 CPU/Scheduler 本体。

## Lifecycle 与 CPU ownership

`sched_init()` 为所有 possible CPU 建立 Scheduler，并使它们到达 Ready。CPU0 Scheduler 随 boot CPU
进入可调度状态而 Online；AP Scheduler 保持 Ready，直到对应 CPU online handoff 才进入 Online。CPU
offline/hotplug teardown 不在本轮能力范围内。

Scheduler 的队列只保存稳定 `TaskRef`。`curr`、`idle`、`stop` 也只保存引用；Task storage、lifecycle、
Flow 和可恢复上下文仍由 Task/TaskFlow owner 解析。class priority 固定为：

```text
stop -> DL -> RT -> fair -> idle
```

具体公平性、带宽控制、迁移和 SMP balance 算法只有在存在可复核 Linux evidence 时才收口，否则保持
Deferred。参考配置未启用的 SCX 保持 Trimmed。

## Task lifecycle 与调度资格

Task lifecycle 与 runnable/on-rq/blocked 资格正交。`OnCpu` 只表示 Task 当前拥有 CPU 执行权；`Online`
只表示 Task 已发布但从未获得 CPU，`Suspended` 表示 Task 曾执行且拥有有效、可恢复的 active
continuation。两者都不保证它在 runnable queue。Scheduler 的 class queue membership 与一次
Schedule 的 prev disposition 共同决定 runnable/on-rq/blocked。blocked Task 可以保持
`Suspended/None/Valid`，但在 wake/enqueue 恢复资格前不得被选中。

## Schedule 信号与 sender 解析

只有当前 Task 的 active TaskFlow 可以向该 Flow 的 `CpuRef` 所指 CPU Scheduler `emits Schedule()`。
Schedule 没有 payload；每次发送产生独立 occurrence。Scheduler 从 sender Flow、其 `CpuRef` 与 CPU-local
CurrentTask binding 推导 `prev: TaskRef`，并要求三者解析为同一 CPU 上的同一 `OnCpu/Live` Task。跨 CPU、
stale Flow、非 active Flow 或错误 CurrentTask binding 必须在任何调度状态修改前拒绝。

一次 Schedule 固定按以下顺序执行：

1. `drives PreparePrev(prev) -> PrevDisposition`；
2. `drives PickNextTask(prev, disposition) -> next`；
3. 仅当 `next != prev` 时 `drives SwitchTo(prev, next)`；
4. 当 `next == prev` 时不执行 SwitchTo，Scheduler 向原 active TaskFlow `emits Continue`，表达 Linux
   `schedule()` 返回。

发出 Schedule 和 PreparePrev 都不改变 Task lifecycle。BootTask 在 `schedule_preempt_disabled()` 对应路径
为 Runnable；它在请求发送、PreparePrev 和 PickNextTask 期间始终保持 `OnCpu/Live/Invalid`。只有实际
选择 `next != prev` 并执行 SwitchTo 时，它才 Suspend 为 Suspended。

## PreparePrev

`PreparePrev` 对齐 Linux `__schedule()` 对 prev state 的先行处理，并返回稳定
`PrevDisposition::{Runnable, Blocked}`：

- 抢占调度，或 prev 仍声明 running/runnable：返回 Runnable，保留其调度资格；
- 非抢占调度、prev 已声明 sleeping，但存在匹配的 pending wake signal：恢复 running并返回
  Runnable；pending signal 的消费是一次性的；
- 非抢占调度、prev sleeping 且没有 matching pending signal：在选择 next 前
  `drives DeactivateTask(prev)`，从所属 class queue 移除并令 on-rq=false，然后返回 Blocked。

PreparePrev 不改变 Task lifecycle，不保存上下文，也不调用 class put/set。DeactivateTask 已确定的
blocked/on-rq=false 结果是后续 class protocol 的硬门禁。

## PickNextTask 与 class handoff

PickNextTask 保留 Linux 两种合法路径：class 提供组合 `pick_next_task` callback；或 fallback 执行
`pick_task -> prev_class.PutPrevTask(prev,next) -> next_class.SetNextTask(next)`。`PutPrevTask` 属于
PickNextTask 内部的 class protocol，禁止在 PreparePrev 之前或选择 next 之前无条件重新入队 prev。

Runnable prev 可按所属 class 规则保留或重新进入可选结构；Blocked/on-rq=false prev 绝不能被
PutPrevTask 放回。identity 选择是否执行 put/set 记账由所走 Linux callback 路径决定；无论是否记账，
都不能发生真正 context switch 或 Task lifecycle/CurrentTask/context 变化。

## SwitchTo 与 continuation

identity path 不进入 SwitchTo，也不产生 Task lifecycle、context、CurrentTask/CurrentStack binding 或
Task Activate/Suspend/Continue。非 identity SwitchTo 必须先完整预检双方 TaskRef、状态、Flow/context
generation、CPU-local binding、stack 和后续 Signal 容量，并一次性保存稳定 TaskRef、TaskFlowRef、Flow
generation 与 `NextDispatchKind::{ActivateInitial,ContinueActive}`；任一失败不得留下部分提交。

- next=Online 时只允许 ActivateInitial：initial Flow 必须 Base、owned、generation 有效，active Flow
  必须无效且 Startup 容量可接受；
- next=Suspended 时只允许 ContinueActive：active Flow 必须唯一、Online、owned、generation 有效且
  Continue 容量可接受；
- 状态与 Flow 事实不一致、两条路径同时成立或均不成立时，都必须在物理切换前拒绝。

预检成功后严格执行：

1. `drives prev.SaveCoreContext`；
2. `drives prev.Suspend`，使 prev `OnCpu -> Suspended`，同时保持 PreparePrev 已确定的 Runnable/Blocked
   资格；
3. `drives next.RestoreCoreContext`，提交寄存器、stack 与 CPU-local CurrentTask/CurrentStack binding；
4. 在 next stack 上完成 Linux finish-task-switch 对应清理；
5. Online next：Scheduler `drives next.Activate`，随后直接向预检 initial Flow `emits Startup`；
6. Suspended next：Scheduler `drives next.Continue`，随后直接向预检 active Flow `emits Continue`。

next Task 接受 Activate/Continue 后分别提交 `Online/None/Valid -> OnCpu/Live/Invalid` 或
`Suspended/None/Valid -> OnCpu/Live/Invalid`。Task 不产生 TaskFlow Startup/Continue；跨执行主体的
TaskFlow→Scheduler 与 Scheduler→TaskFlow 使用 `emits`，Scheduler 对 Task lifecycle 及内部状态检查、
选择、put/set、保存与恢复使用 `drives`。

## 当前能力边界

本轮只闭合可复核的 Linux schedule 主序、prev disposition、class handoff、per-CPU ownership 与
task-stack switch。完整 SMP balancing、带宽、公平性和 SCX 不由实现症状推导。具体 Linux symbol、函数
或 checkpoint 映射只属于 Testing/cross-reference，不进入本 Charter 或 Coding 核心规格。

## Mapping

- Model: `spec/model/objects/scheduler.spec`
- Coding: `spec/coding/objects/scheduler.md`
- Implementation: `impl/arceos_ex/src/objects/scheduler.rs`
