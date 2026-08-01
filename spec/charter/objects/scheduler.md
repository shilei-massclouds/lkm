# Scheduler

`Scheduler` 对应 Linux 每 CPU `struct rq`。每个 possible CPU 恰有一个 Scheduler；它拥有 CPU-local
lock、`curr/idle/stop` TaskRef 和 stop/DL/RT/fair/idle 队列。root/sched domain 是独立共享资源，不
复制 CPU 或 Scheduler。队列、curr、idle、stop 只保存 generation-checked TaskRef。

## lifecycle、队列与 Task 状态

`sched_init()` 使 possible CPU Scheduler Ready；CPU0 随 boot CPU 可调度边界 Online，AP Scheduler 在
对应 CPU online commit 后 Online。class priority 固定为 stop → DL → RT → fair → idle。

Task lifecycle 与 runnable/on-rq/blocked 正交。当前执行者为 OnCpu；所有当前不在 CPU 的已发布 Task
统一为 Online，不区分首次与恢复。blocked Task 可以保持 Online/None/Valid，但
在 wake/enqueue 恢复资格前不得被选择。

## Schedule 与 `yields`

只有当前 Task 的固定 TaskFlow 可以根据其 CpuRef 对本 Scheduler 执行
`yields Scheduler.Action::Schedule()`。Schedule 无 payload；每次调用创建独立 Signal occurrence 和
模型 YieldToken。Scheduler 从 sender Flow、CpuRef、CPU-local CurrentTask/CurrentStack 推导 prev，
并要求同一 CPU、同一 OnCpu/Live Task、固定 FlowRef 和 generation 全部一致。跨 CPU、stale Flow、错误
CurrentTask 或 ineffective Flow 在 token 提交前拒绝。

`yields` 本身只挂起 source `TaskFlowLane` 的模型 handler continuation，不保存或恢复寄存器、不读写
TaskThreadContext、不改变 Task/TaskFlow lifecycle，也不修改 CpuRef、runqueue、锁或中断状态。所有这些
调度效果必须由 Schedule/SwitchTo 的显式步骤产生。

Schedule 顺序为：

1. PreparePrev(prev) → PrevDisposition；
2. PickNextTask(prev, disposition) → next；
3. next != prev 时 SwitchTo(prev,next)；
4. next == prev 时完成 handler，由通用 yield default resume attempt 返回 source。

identity 不进入 SwitchTo，不保存/恢复 context，不交付 Task/TaskFlow Continue，不改变 lifecycle、
authority、Flow/CpuRef、CurrentTask/CurrentStack、runqueue 选择结果或观察计数。目标 handler 完成后，
source execution binding 仍有效，因此通用 yields 逻辑立即精确一次消费 token。

## PreparePrev 与 PickNextTask

PreparePrev 对齐 `__schedule()` 的 prev disposition：抢占或仍 running 返回 Runnable；sleeping 但有匹配
pending wake 时一次性消费 wake 并返回 Runnable；非抢占 sleeping 且无 wake 时先 DeactivateTask，
移出 class queue，返回 Blocked。它不改变 Task lifecycle、不保存 context、不调用 class put/set。

PickNextTask 保留组合 `pick_next_task` callback，或 fallback 的
`pick_task -> prev_class.PutPrevTask -> next_class.SetNextTask`。Blocked/on-rq=false prev 不得重新入队。
identity 路径可以完成 class bookkeeping，但不产生 switch effects。

## SwitchTo 与统一 Continue

non-identity SwitchTo 在任何不可逆效果前完整预检：prev/next TaskRef、双方固定 FlowRef/generation、
prev OnCpu/Live/Invalid、next Online/None/Valid、context epoch、dispatch record、stack、CPU-local binding
和后续 Signal 容量。Scheduler 不保存任何 first/resume dispatch kind；
next 只有一个固定 Flow 和一个 Continue 路径。

预检成功后严格执行：

1. `SaveCoreContext(prev)` 保存 `ra/sp/s0..s11`、固定 FlowRef、可能的 root TrapFlowRef，并推进 prev
   context epoch/dispatch record；
2. `Task.Suspend(prev)` 提交 `OnCpu/Live/Invalid -> Online/None/Valid`，不改变 PreparePrev 的
   Runnable/Blocked 结果；
3. `RestoreCoreContext(next)` 恢复寄存器，并在同一 switch commit 原子提交 next Flow CpuRef、
   CurrentTask 与由 live sp 校验的 CurrentStack；
4. 在 next stack 上完成 finish-task-switch 清理；
5. `Task.Continue(next)` 提交 `Online/None/Valid -> OnCpu/Live/Invalid`；
6. 向 next 固定 FlowRef 交付 contextual `TaskFlow.Action::Continue`。

contextual Continue 不携带机器入口；TaskThreadContext 决定首次入口、普通保存入口或嵌套 Trap leaf。
若 next FlowLane 有 pending YieldToken，它必须与 TaskRef、FlowRef、generation、CPU、context epoch 和
dispatch record 匹配后才恢复模型 continuation；否则 Continue 从 context 指定入口开始。

Schedule occurrence 所创建的 YieldToken 绑定 source response identity、TaskRef/FlowRef/generation、
target occurrence、模型 resume coordinate、CPU/TaskFlowLane 和 context epoch。TaskThreadContext 独立
保存真实寄存器。两者只能通过 epoch/dispatch record 交叉校验，不得互相复制。

non-identity Schedule handler 完成时 source binding 已被上述显式步骤改变，因此默认 resume attempt
只保持 token pending。未来切回 source 时 contextual Continue 精确一次消费 token并从 `yields` 后继续，
形成 A→B→A 的 schedule return。rejection 必须在 token commit 前无状态变化；post-commit failure、
stale、错误 Flow/CPU/epoch 或重复恢复终止失败，不回滚、不重试。

terminal prev 走 OnCpu→Offline 并在 next 侧 Cleanup，不先发布不可恢复的 Online context。陷入中真实
切出保存当前 trap leaf；恢复 next context 后先落到该 leaf。普通 IRQ 若没有真实 task switch，不改变
Task/TaskFlow 或 CPU-local binding。

## 当前能力边界

本轮闭合单 CPU schedule、prev disposition、class handoff、固定 TaskFlow、显式 context switch 与
通用 yields return。GlobalArbiter、cross-CPU mailbox、迁移与完整 schedule replay 保持 P2 延期。

## Mapping

- Model: `spec/model/objects/scheduler.spec`
- Coding: `spec/coding/objects/scheduler.md`
- Implementation: `impl/arceos_ex/src/objects/scheduler.rs`
