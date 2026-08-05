# TaskFlow

`TaskFlow` 是 Task 终身唯一的长期执行 Flow。每个 TaskFlow 恰有一个 owner/parent Task，每个 Task 也
恰有一个创建后不可变的 `flow: TaskFlow`；双方 identity、lifecycle 和 teardown 事实分别保存，但不得
替换、共享或建立历史链。`TaskFlow` 继承 `PhaseObject`，状态集保持
Base/Prepared/Ready/Online/Offline/Destroyed。

Task carrier、TaskRef、TaskThreadContext 和 Task lifecycle 见 [`Task`](task.md)。

## 固定实例与 UserAppRuntime

- `BootInitFlow` 终身属于 `BootTask`，从 `_start` 编排 boot，Online 后继续承载 idle setup、首次
  Schedule 的返回 continuation、`BootIdleEntryPhase` 和 idle loop；不创建第二个长期 boot Flow。
- `KernelInitFlow` 终身属于 `KernelInitTask`。Task 发布时 Flow 已 Online；首次及后续 dispatch 都以
  contextual Enter 进入；首次 Enter 消费 Setup 绑定的 `RunKernelInit` 正文坐标。successful exec 不替换它。
- `KthreaddFlow` 终身属于 `KthreaddTask`，发布和派发规则相同。
- 每次 fork/clone 创建 fresh child Task 与 fresh 普通 `TaskFlow`；child exec 不创建后继 Flow。
- `ApIdleFlow[logical_id]` 与 `ApIdleTask[logical_id]` pointwise 固定，在 HSM 交付前完成不可调度的
  Online 预初始化。
- 每个普通动态内核 Task 创建 fresh `KernelTaskFlow`，其初始正文坐标绑定调用者提供的内核入口；
  blocked、wake、yield 和恢复不替换该 Flow。

每个用户型 TaskFlow 最多创建一个终身稳定、不可共享的被动 `UserAppRuntime` owned child。PID 1 的 Runtime
属于 `KernelInitFlow`，child Runtime 属于各自的普通 `TaskFlow`。exec 只以事务方式替换 Runtime 内部
`ApplicationInstance`；fork 创建新的 Task、Flow、Runtime。应用映像名称不得提升为 Flow 类型或
Flow identity。

Runtime Online 表示应用 continuation 已经建立且尚未 terminal；它覆盖应用正在运行、runnable/待调度、
阻塞挂起以及被 CPU-owned trap overlay 临时覆盖。Runtime 不进入 effective-flow 栈，也不以
Running/Paused/Trapped 子状态复制 Task、ContextCoordinate 或 overlay 状态。Runtime Setup/Enable 只用于
从头建立应用 continuation 的路径；fork snapshot 直接物化 Online Runtime，不调用这些迁移。

## owner、FlowRef 与 CpuRef

owner/parent 和 `Task.flow` 在声明时原子建立，此后不可变。`TaskFlowRef` 使用 private slot 加非零
generation；静态 Flow 使用固定 slot/generation，动态 slot 复用时递增 generation。lookup 同时校验
两者，旧引用稳定失效。

TaskFlow 是任务路径中唯一保存 `cpu_ref` 的长期对象；Task 不保存同义字段。架构入口 adoption、真实
scheduler switch 或未来正式 migration commit 可以写 cpu_ref，普通 Flow action 只读。Flow 离开 CPU
仍保留最后归属；prepare 不得预写 next。`CurrentCPU` 从 effective Flow 的 cpu_ref 解引用，同步 drives
子树继承解析来源，emits 不继承。

## Online、contextual Enter 与正文坐标

普通可调度 Flow 在所属 Task 发布前完成 Preset/Setup/Enable 并保持 Online；运行主体不是再次启动的
lifecycle Transition，而是可挂起 Online Action。BootInitFlow 和 ApIdleFlow 允许在不可被 Scheduler
切走的架构前初始化中推进到 Online；它们随后也只用 Online Actions 承载可调度 continuation。
`TaskFlow.Online` 表示该 Flow 的逻辑执行生命期已经开始且尚未终止，既包括实际在 CPU 上执行，也包括
持有可恢复 continuation 而挂起；当前实际拥有 CPU 只由 parent `Task.OnCpu` 与
`TaskExecutionAuthority::Live` 表达，存在下一次可消费上下文只由 `TaskBreakpoint::Valid` 表达。

所有首次与恢复派发统一接受：

```text
TaskFlow.State::Online / Action::Enter
```

Enter 是带 `contextual_entry: true` 的 contextual Action。receiver 是 Task 固定 FlowRef；机器入口与
可能嵌套的 Trap leaf 来自 Scheduler 已恢复的 TaskThreadContext。Signal 不携带 PC、SP、寄存器、函数名、
entry role、checkpoint 或 first/resume 枚举。每次派发统一为
`Restore -> Dispatch -> Enter -> resume current coordinate`：Enter 必须用 CPU、TaskRef、FlowRef、
generation、dispatch ordinal 与 context epoch 校验并精确一次消费本轮 Dispatch proof，再消费
TaskThreadContext 的当前 `ContextCoordinate`。初始 coordinate 指向 Flow 以 `initial_context` 声明绑定的
Online、`StateEffect::None` 正文 Action；保存后的 coordinate 指向 YieldToken 或机器 continuation 的恢复
坐标。Scheduler 与 Enter 都不得按 first/resume、具体 Flow 类型、函数或固定入口分支。

`KernelInitFlow.RunKernelInit` 驱动 kernel-init 阶段链；`KthreaddFlow.RunScheduleLoop` 进入调度循环；
普通 TaskFlow 的 user continuation 坐标恢复已准备的用户现场；`ApIdleFlow.RunIdle` 承载 AP idle 正文。
这些都是普通正文 Action，不携带一次性属性。BootInitFlow 继续由 `_start` 驱动 Preset/Setup/Enable，
不制造首次 Enter；BootTask 与 AP idle 的首次架构直入不伪造 Dispatch/Enter，只有首次真实切出后的恢复
才走通用 Dispatch/Enter。

`ApIdleFlow.RunIdle` 在三段 AP 初始化完成后持续执行 owner CPU 的通用 idle/scheduler loop：消费本地
inbound mailbox，观察本地 runqueue 与 `need_resched`，必要时请求 owner Scheduler；无工作时采用关中断
重检后 `wfi` 的不丢唤醒协议。reschedule IPI handler 只清除硬件 pending 并记录本地
`need_resched`，不在 hardirq handler 内切换 Task。

除声明期 Bind 和明确的 Boot/AP 架构入口例外，普通 lifecycle/action 每次执行都即时要求：固定 parent
Task 为 OnCpu/Live、Flow 为该 Task 的唯一 flow、FlowRef/generation 有效、CpuRef 与 CPU-local
CurrentTask/CurrentStack 一致，并且当前 effective-flow 栈的长期底层是该 Flow。普通 TaskFlow Action
不能在 Trap/Interrupt/Exception leaf 仍有效时越过 leaf 执行；返回或调度恢复必须先落到有效 leaf。

## 陷入与 effective-flow 栈

陷入不改变 Task.OnCpu 或 TaskFlow.Online。活动执行栈只叠加短期 Flow：

```text
TaskFlow -> TrapFlow -> InterruptFlow/ExceptionFlow -> concrete leaf
```

TaskFlowLane 的 source handler continuation 可以因 `yields` awaiting-resume，但这不是 TaskFlow lifecycle
挂起。Trap 清理完成且一次性返回 token 被消费后回到同一个固定 TaskFlow；exec 只替换 Runtime 内部
ApplicationInstance，因此不存在 successful-exec successor/predecessor Flow 例外。

若陷入内发生真实 task switch，Scheduler 保存的 TaskThreadContext 指向当前短期 leaf。未来 Task
Dispatch 后，contextual TaskFlow.Enter 先恢复 leaf 模型/架构 continuation；普通 IRQ 未切换 Task 时
Task lifecycle、Flow lifecycle 和 CPU binding 都不变。

TaskFlow.Enter 只用于真实 task-switch 后的恢复。若 `TaskThreadContext` 不含 root TrapFlowRef，Enter
恢复普通 TaskFlow coordinate；若含 root，则必须先验证 root generation、入口 Task/固定 Flow、owner
CpuRef、活动 child 与 concrete leaf、context epoch 以及 leaf 尚未 Cleanup，再记录精确一次 leaf-resume
proof。无 Task switch 的 trap 返回只按 leaf→root Cleanup、token、`sret` 回到原机器坐标，不发送
Dispatch/Enter，也不重新执行 trap 入口。

## `yields` 与 Schedule

任意 StateEffect::None Action 可用通用 `yields Target.Signal`：立即预检并交付目标，创建可序列化
YieldToken，令 source TaskFlowLane awaiting-resume。`yields` 不保存/恢复架构 context，不改 Task、
TaskFlow、CurrentTask、CurrentStack、CpuRef、runqueue、锁或中断状态。

只有当前 Task 的固定 TaskFlow 可对其 CpuRef 所指 Scheduler 执行 `yields Schedule()`。Scheduler 是
该通用原语的使用者而非语言特例：

- identity：Schedule handler 没改变 source execution binding，目标完成后的默认 resume attempt
  立即消费 token，从 `yields` 后返回；不交付 Dispatch/Enter。
- non-identity：Scheduler handler 显式 Save/Suspend/Restore/commit/Dispatch/Enter，source binding 已改变，
  token 保持 pending；Save 发布其 resume coordinate，未来匹配的 contextual TaskFlow.Enter 恢复它。

目标 Signal 不进入 emits FIFO。preflight rejection 不创建 token；post-commit failure、stale、错误绑定
或重复 resume 终止失败，不回滚、不重试。

## lifecycle 与 teardown

TaskFlow 必须显式经历 Preset/Setup/Enable/Disable/Cleanup。Enable 必须验证所属 Task 的初始
TaskThreadContext 已具备完整寄存器、固定 FlowRef/generation、epoch 与 `initial_context` 正文坐标，然后
发布 Online；它不在创建者栈执行正文。Enter 才消费当前 contextual continuation。切出固定由
`Task.Save -> Task.Suspend` 表达；不定义 `TaskFlow.Exit`。Disable 只允许 terminal 路径，要求无活动
Trap child、无 pending YieldToken、Runtime 已完成退出；Cleanup 从 Offline 回收到 Destroyed。

上述生命周期适用于从头创建的 Flow。fork child 由原子 snapshot 直接 materialize 为 Online，完整候选
一次性建立 owner/parent、FlowRef/generation、初始 ContextCoordinate 与 Online invariant，不产生
Preset/Setup/Enable transition。fork 后 parent 为 `OnCpu/Online/Online`，child 为
`Online/Online/Online`；首次选择 child 和任意后续选择完全一样，统一执行
`Restore -> Dispatch -> TaskFlow.Enter -> ContextCoordinate`，不增加 Runtime Enter。

Task terminal Disable 前固定 Flow 必须不再 Online；用户型 terminal 还要求先显式 quiesce、Disable 和
Cleanup Runtime，再 Disable/Cleanup Flow，最后 Disable/Cleanup Task。Task Cleanup 前 Flow 必须 Destroyed。storage 或
alias 离开词法范围不表示销毁。下一动态实例使用新 generation。

## 当前能力边界

本轮在既有 SMP runtime 上闭合用户 Task 的固定跨 CPU placement、per-CPU inbox/runqueue 与 10 ms
用户态抢占。普通 fork 的 Flow CpuRef 按 PID 公式一次写入，vfork/CLONE_VM 继承 parent CpuRef；两者
均不迁移。GlobalArbiter、自动负载选择、运行中迁移、内核态立即抢占和 deterministic replay 保持延期。
