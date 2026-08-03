# Deterministic SMP TaskFlow lanes roadmap

本页记录本轮已经闭合的 UP `yields Schedule` 基础，以及仍为 P2 的 SMP 仲裁、迁移与 replay。
它是 roadmap，不覆盖 Charter、Model 或 Coding。

## 已完成：固定 lane 与 UP return

- 每个 Task 终身绑定一个 TaskFlow；Task 只保存固定 FlowRef，TaskFlow 保存唯一 owner/parent 与
  CpuRef。首次派发和恢复派发都使用 contextual `TaskFlow.Action::Continue`。
- Task lifecycle 统一为 `Online --Continue--> OnCpu --Suspend--> Online`。blocked/runnable 由
  Scheduler/runqueue facts 区分，不增加第二套 Task lifecycle 状态。
- `yields` 创建可序列化 YieldToken，并挂起 source TaskFlowLane 的 handler continuation；它不保存
  架构寄存器，也不修改 Task、TaskFlow、CPU-local binding、runqueue 或锁/中断状态。
- identity Schedule 由通用 target-completion resume attempt 立即、精确一次恢复 source lane，不发送
  contextual Continue。
- non-identity Schedule 在目标 handler 中显式 SaveCoreContext、Suspend prev、RestoreCoreContext、提交
  CurrentTask/CurrentStack、Continue next；source token 保持 pending，直至未来匹配 Continue。
- token 绑定 source response identity、TaskRef、FlowRef、generation、Schedule occurrence、resume
  coordinate、CPU/lane，并与独立 TaskThreadContext epoch/dispatch record 交叉校验。
- UP 覆盖 rejection-before-commit、post-commit terminal failure、stale/错误绑定/重复恢复、A→B→A、
  nested yield、blocked/wakeup、trap leaf 恢复与普通 IRQ 无 Task lifecycle delta。

## 已闭合：显式目标 AP activation/wake mailbox

首轮已闭合普通内核 Task 的显式目标 CpuRef、generation-checked inbound mailbox、reschedule IPI 与 AP
idle/scheduler continuation。该路径不选择目标 CPU、不迁移运行中 Task，也不引入全局仲裁；它只是把
调用者已经选择的 online CPU 安全交付给 owner Scheduler。

## P2：GlobalArbiter、负载选择与迁移

P2 引入确定性的全局仲裁 occurrence，而不改变上述 Task/Flow/yields 语义：

1. 每个 CPU Scheduler lane 只发布本地候选、runqueue epoch 与不可变 TaskRef/FlowRef dispatch record。
2. GlobalArbiter 按 artifact 中显式 total order 选择获胜 occurrence；不得依赖宿主线程调度或容器遍历顺序。
3. 跨 CPU wake/migrate 通过 generation-checked mailbox 交付，目标 CPU 在持有本地 rq lock 的提交点消费。
4. 迁移只更新同一 lifetime Flow 的 CpuRef，并与 source dequeue、destination enqueue、context epoch 和
   CPU-local binding 构成不可拆分 commit；不存在 Flow replacement 分支。
5. stale migration mailbox、重复 occurrence、错误 source/destination epoch 和部分提交一律 terminal failure，不回滚、
   不重试，也不静默重算仲裁。

## P2 artifact/replay

预留 artifact envelope：`schema=lkm.spec.schedule`、`producer=tools2`、version 1。未来内容至少包括：

- CPU lane inventory 与 epochs；
- runnable candidates、class decision、chosen TaskRef/FlowRef；
- YieldToken/dispatch/context epoch 交叉引用；
- mailbox send/receive 与 migration commit occurrence；
- exact total order、failure outcome 和最终 snapshot fingerprint。

Replay 必须只消费 artifact 决策，验证而不重新仲裁；record/replay 的对象状态、Signal/YieldToken 轨迹和
failure position 必须一致。本轮不实现 artifact writer、GlobalArbiter、自动负载选择、迁移 commit
或完整 replay。

## Closure record

- Charter：changed，固定一对一 Flow、contextual Continue 与纯模型 yields 已闭合。
- Model：changed，UP token/lane/Schedule return 与显式 switch 已闭合。
- Coding：changed，固定 Ref、context epoch、switch commit 与 Runtime storage 已闭合。
- Impl：changed，arceos_ex UP 路径与 smoke 已闭合。
- Compose：reviewed, no change；未新增 crate/facade。
- Testing：changed；P2 项保持延期，不作为本轮已实现能力宣称。
