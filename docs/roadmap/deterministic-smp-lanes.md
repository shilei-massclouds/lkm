# Deterministic SMP TaskFlow lanes roadmap

本页记录已经闭合的 UP `yields Schedule`、显式目标 AP mailbox 与首轮 TaskFlow trap overlay，
以及仍为 P2 的 SMP 仲裁、迁移与 replay。
它是 roadmap，不覆盖 Charter、Model 或 Coding。

## 已完成：固定 lane 与 UP return

- 每个 Task 终身绑定一个 TaskFlow；Task 只保存固定 FlowRef，TaskFlow 保存唯一 owner/parent 与
  CpuRef。首次派发和恢复派发都使用 contextual `TaskFlow.Action::Enter`。
- Task lifecycle 统一为 `Online --Enter--> OnCpu --Suspend--> Online`。blocked/runnable 由
  Scheduler/runqueue facts 区分，不增加第二套 Task lifecycle 状态。
- `yields` 创建可序列化 YieldToken，并挂起 source TaskFlowLane 的 handler continuation；它不保存
  架构寄存器，也不修改 Task、TaskFlow、CPU-local binding、runqueue 或锁/中断状态。
- identity Schedule 由通用 target-completion resume attempt 立即、精确一次恢复 source lane，不发送
  contextual Continue。
- non-identity Schedule 在目标 handler 中显式 SaveCoreContext、Suspend prev、RestoreCoreContext、提交
  CurrentTask/CurrentStack、Enter next；source token 保持 pending，直至未来匹配 Enter。
- token 绑定 source response identity、TaskRef、FlowRef、generation、Schedule occurrence、resume
  coordinate、CPU/lane，并与独立 TaskThreadContext epoch/dispatch record 交叉校验。
- UP 覆盖 rejection-before-commit、post-commit terminal failure、stale/错误绑定/重复恢复、A→B→A、
  nested yield、blocked/wakeup、trap leaf 恢复与普通 IRQ 无 Task lifecycle delta。

## 已闭合：显式目标 AP activation/wake mailbox

首轮已闭合普通内核 Task 的显式目标 CpuRef、generation-checked inbound mailbox、reschedule IPI 与 AP
idle/scheduler continuation。该路径不选择目标 CPU、不迁移运行中 Task，也不引入全局仲裁；它只是把
调用者已经选择的 online CPU 安全交付给 owner Scheduler。

## 已闭合：TaskFlow trap overlay 与 leaf 内切换返回

- reschedule SSIP 不再绕过 TrapFlow。每次正式 SSIP 都建立 fresh、generation-checked 的
  `TrapFlow -> InterruptFlow` 栈；handler 只清 pending 并合并 `need_resched`，不消费 mailbox、
  不直接切换 Task。
- 无切换路径按 leaf 到 root 顺序 Cleanup，精确消费一个 TrapReturnToken 后 `sret` 回原 TaskFlow
  机器坐标；这条路径不制造 Dispatch/Enter。
- task-switch 恢复携带可选 root TrapFlowRef。contextual Enter 在提交前验证 root generation、active
  child、concrete leaf、Task/Flow/CpuRef、context epoch 与未 Cleanup 状态，然后由保存的机器
  `ra/sp` 恢复原 leaf continuation。
- kernel page fault 的 source 与 atomic 属性分离：非嵌套、非 hardirq、入口前可中断且命中正式
  exception-table 的 task-context fault 可以在 leaf 内正常调度；nested、hardirq 或入口前关中断的
  kernel fault 只能立即 fixup；缺失或 stale fixup 保持 terminal。
- CPU1 focused acceptance 使用两个普通动态内核 Task：A 的真实 `ld` fault 在 page-fault leaf 内消费
  inbound A->B，B 通过普通 cooperative yield 返回 A，A 经 Dispatch/Enter 恢复同一 leaf、提交
  exception-table fixup 并返回 fault-site 后续坐标；随后 A、B 正常退出并回到 idle。
- 正式 trap 只持有入口 CPU 的窄 runtime lease；AP 从已发布的 per-CPU TrapType 与目标 CPU task
  registry 解析权限，不取得全局可变 Context。稳定 per-CPU observation 记录 root/leaf/SSIP、token、
  leaf resume 与最后 generation，仅用于诊断和验收。

### 首轮验证证据

- `smp=2` focused QEMU smoke 通过 58/58，并精确观察 3 次 SSIP、4 个 root/token、1 次 exception、
  1 次 leaf resume、5 次 non-identity switch、2 个 mailbox consume 与最终 CPU1 idle。
- `smp=8` 的 native 与 Linux-object provider kernel smoke 各通过 58/58；同一 CPU1 路径闭合，
  其余 online AP 保持 idle。
- tools2 Python 全量 106/106、前端单测 16/16、浏览器 E2E 9/9，并通过完整 Model derivation、
  Charter lock、Coding、rustfmt 与双 provider Clippy 门禁。
- 默认四组压力验收各 10/10（合计 40/40）；共享 boot/SMP/user checkpoint 的默认 Linux paired
  difftest 1/1。AP trap-switch 没有 Linux 同构 checkpoint，不建立伪映射。

## P2：GlobalArbiter、负载选择与迁移

P2 引入确定性的全局仲裁 occurrence，而不改变上述 Task/Flow/yields 语义：

1. 每个 CPU Scheduler lane 只发布本地候选、runqueue epoch 与不可变 TaskRef/FlowRef dispatch record。
2. GlobalArbiter 按 artifact 中显式 total order 选择获胜 occurrence；不得依赖宿主线程调度或容器遍历顺序。
3. 跨 CPU migration 扩展 generation-checked mailbox，目标 CPU 在持有本地 rq lock 的提交点消费；
   已闭合的显式目标 activation/wake 交付语义保持不变。
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

- Charter：changed；固定一对一 Flow、contextual Enter、纯模型 yields、正式 SSIP overlay、
  TrapReturnToken 与 page-fault leaf 内切换返回已闭合。锁定的 `systems/computer.md` reviewed, no change。
- Model：changed；UP token/lane/Schedule return、TrapFlow generation/leaf proof、SSIP handler 边界与
  page-fault atomic/schedulable matrix 已闭合。
- Coding：changed；固定 Ref、context epoch、switch commit、CPU-local trap lease、exception-table
  lookup 与 observation storage 已闭合。
- Impl：changed；arceos_ex UP、显式目标 CPU1 mailbox、正式 SSIP、真实 exception-table fault 的
  A->B->A leaf resume focused smoke 已闭合。
- Compose：reviewed, no change；未新增 crate/facade。
- Testing：changed；同栈返回、切换恢复、atomic/nested failure matrix 与 CPU1 ordinary-task 验收已闭合；
  P2 arbiter/migration/replay 保持延期，不作为已实现能力宣称。
