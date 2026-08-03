# Task

`Task` 是内核中唯一的 task_struct-like carrier 类型。Scheduler 调度 Task；Task 物理拥有稳定身份、
PID、调度状态、任务资源引用、唯一内核栈、`TaskThreadContext`，以及创建时绑定后终身不可改写的
`flow: TaskFlow` association。Flow 的 owner/parent 唯一指回该 Task；Task 与 TaskFlow 是终身一对一，
不存在 initial/active 两套 Flow、owned-Flow 历史、predecessor 或 replacement/handoff。

CPU 归属只保存在固定 `TaskFlow.cpu_ref`；Task 不保存同义 CPU assignment 字段。`BootTask`、
`KernelInitTask`、`KthreaddTask`、每个 AP idle Task 和用户 child 都是同一 `Task` 类型的独立实例，
不建立 process persona、idle carrier 或可替换 Flow wrapper。

## 稳定身份与固定 Flow

- `BootTask` 是静态 `init_task` / PID 0 / swapper，从首个内核指令起为 `OnCpu`，固定 Flow 是
  `BootInitFlow`。`sched_init()` 只给同一 Task 建立 idle 调度角色；idle setup、首次 schedule 返回和
  idle loop 都继续由 `BootInitFlow` 的 Online Actions 承载。
- `KernelInitTask` 是 `copy_process()` 创建的稳定 PID 1，固定 Flow 是 `KernelInitFlow`。任意次数的
  successful exec 都保持 Task、Flow 和 Flow-owned `UserAppRuntime` identity，只替换 Runtime 内部
  `ApplicationInstance`。
- `KthreaddTask` 固定绑定 `KthreaddFlow`。
- 每次 fork/clone 都创建 fresh Task、fresh `UserTaskFlow` 和该 Flow 创建的 fresh
  `UserAppRuntime`；三者都不得与 parent 或其它 child 共享。child 后续 exec 仍不替换 Flow。
- `ApIdleTask[logical_id]` 与 `ApIdleFlow[logical_id]` pointwise、终身绑定。

运行期实例 identity、slot/generation 和声明规则由
[运行期实例声明](dynamic-instance-declaration.md)统一定义。Task 的 `flow` 必须在 Preset 时一次性绑定；
重新绑定、清空或绑定到另一个 owner 都是终止错误。

## 类型级 lifecycle 与执行权

普通 Task 的唯一 lifecycle 为：

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
Online --Dispatch--> OnCpu --Suspend--> Online
OnCpu --Disable(terminal)--> Offline --Cleanup--> Destroyed
```

`Online` 统一表示已发布、当前不在 CPU 的 Task，不再区分从未派发和已经切出。runnable、on-rq、
blocked 与 lifecycle 正交：Online Task 可以 runnable，也可以因阻塞而
不在 class queue。`OnCpu` 表示该 Task 是某 CPU 唯一 current/执行权 carrier；它是 Task 专属状态。

Task 还保存两组正交状态：

- `TaskExecutionAuthority::{None, Reserved, Live}`：普通 Online Task 为 None，OnCpu Task 为 Live；
  Reserved 只用于已成为 AP `rq->idle/rq->curr`、但尚未收到 HSM 执行权的 idle Task。
- `TaskBreakpointState::{Invalid, Prepared, Valid}`：Setup 构造 Prepared context，建立初始 `ra/sp`、
  固定 FlowRef/generation、context epoch、dispatch record，并把该 Flow 实例唯一的 `Start` 坐标绑定为
  首次恢复坐标；Enable 只把 context 永久绑定到 `flow` 并发布 Valid；Dispatch 只从 Online 校验并消费
  Valid context，提交
  `OnCpu/Live/Invalid`；Suspend 只从 OnCpu 保存 context 并发布 `Online/None/Valid`。

首次入口与恢复入口使用同一个 Dispatch/Enter 路径。Scheduler 不保存 first/resume dispatch kind，也不根据
Task 历史选择不同 Signal。实际入口由已经恢复的 `TaskThreadContext` 中 `ra/sp/s0..s11` 决定；新 Task
在发布前已拥有指向其固定 FlowRef 的首个 Valid context，已运行 Task 的 Suspend 则覆盖为新的 Valid
context。

`BootTask` 是允许的静态 override：初态为 `OnCpu/Live/Invalid`，没有 Preset/Setup/Enable。它首次
切出时才由 Scheduler 保存第一个 context，此后同样经 `Online --Dispatch--> OnCpu --Suspend--> Online`
往返。boot-only const 初始化器直接构造静态 task、PID 0、`TaskRef::BOOT` 和 `stack`，不复用普通
Task lifecycle 方法。

每个 `ApIdleTask` 在 BP 预建完成时为 `OnCpu/Reserved/Invalid`。HSM 入口验证 boot data、原子建立
CurrentTask/CurrentStack 并把 authority 激活为 Live，不改变 Task lifecycle；它首次切出时才产生
Valid context，之后使用普通 Dispatch/Suspend。HSM 架构入口直接调用 `ApIdleFlow.Start`，不发送
Task.Dispatch 或 TaskFlow.Enter。

## TaskThreadContext

`TaskThreadContext` 固定保存长期 Task continuation 的核心寄存器集合、breakpoint state、不可变
`TaskFlowRef`、Flow generation、context epoch、dispatch record、可选 root `TrapFlowRef` 和真实
save/restore 观察计数。Valid context 必须绑定 Task 的唯一 `flow`；Prepared/Invalid context 不可恢复。
FlowRef、generation 或 epoch 不匹配必须在提交前拒绝。

root TrapFlowRef 通过短期 child FlowRef 链定位当前 Trap/Interrupt/Exception leaf；不存在活动陷入时
为空。陷入本身不改变 Task.OnCpu、TaskFlow.Online 或 TaskThreadContext 的长期所有权。若陷入中发生
真实 task switch，Suspend 保存的是当前 trap leaf 的架构 continuation；未来恢复先回到该 leaf，再按
trap return token 返回固定 TaskFlow。

每个 Task 恰有一个 `stack: Stack` 值类型属性。context 中 `sp` 是该 storage/range 内的恢复游标，
不是 CurrentStack 副本。`tp`、CPU-local CurrentTask/CurrentStack binding、runqueue、锁和中断状态都
不属于 TaskThreadContext 的可恢复核心寄存器集合。

## 栈底保护 Action

`Task.Action::EnableStackGuard` 是 Task owner 的通用 Action。它只接受该 Task 唯一拥有、范围非空、
方向有效、足以容纳一个机器字且满足机器字对齐的 `stack`；调用方给出的范围还必须与 Task 已记录的
stack 范围完全相同。任一前置条件不满足时，Action 必须在第一次内存写入前失败，不建立保护事实。

Action 在向低地址增长的内核栈底安装固定保护值并建立 `task_stack_guard_ready(Task, Task.stack)`，
不推进 Task 或 stack lifecycle。对同一完整保护字重复调用成功且不再次改写；已经安装后若只读完整性
检查发现保护值损坏，后续调用必须报告损坏，不得把它当成未安装状态自动修复。安装状态属于 Task，
完整性检查只读取保护位置，不改变安装状态、保护值或其它 Task 属性。

该保护字是栈边界溢出的确定性哨兵，不是不可访问的 guard page，也不是由随机秘密派生、用于检测
函数局部覆盖的 stack canary；三者不得共享状态或把各自的成功事实互相替代。普通 Task 何时触发
该 Action 由各自 TaskFlow 规定，本 owner 不把它限定为 BootTask 或 current Task。

## 创建与发布

普通 Task.Preset 建立 fresh identity、TaskRef、不可变 Flow association、Flow owner/parent 和 clone
specification。Setup 消费 `TaskCreationCore` 的 copy-process 事实，建立 PID、stack、首个寄存器字节、
Prepared context、scheduler entity 和 New/not-enqueued 状态。Enable 在 wake/runqueue publication 后
把 Prepared context 绑定固定 FlowRef，原子发布 `Online/None/Valid`；同时要求该 TaskFlow 已按所属
类型完成发布并为 Online。Enable 不执行 Flow 主体。

`BootInitRestInitPhase` 完整驱动 PID 1 与 kthreadd 的创建和发布。它们首次真正被选择时与以后恢复时
完全相同：Scheduler 恢复 context、提交 CurrentTask/CurrentStack，再 drives Task.Dispatch，随后向固定
Flow 交付 contextual `Action::Enter`。首次 Enter 后由 Setup 绑定的坐标执行实例 `Start`；后续 Enter
只恢复保存坐标。

`TaskCreationCore.CopyProcess` 的 source 必须是当前 `OnCpu/Live` Task；其固定 Flow 必须是 effective
TaskFlow，TaskRef 必须与 CurrentTaskRef 一致。Online、Reserved、非 current 或 stale reference 在修改
destination 前拒绝。

## Scheduler switch 与 `yields`

固定 TaskFlow 的可挂起 Action 用 `yields Scheduler.Schedule` 表达立即交付 Schedule 并挂起 source
`TaskFlowLane` continuation。`yields` 是纯模型控制原语，不保存或恢复架构寄存器、不读写
TaskThreadContext、不改变 Task/TaskFlow lifecycle，也不切换 CurrentTask/CurrentStack。

identity Schedule 不保存 context、不改变 Task 状态或 CPU binding。Scheduler handler 完成后，通用
yield resume attempt 发现 source execution binding 仍有效，立即精确一次消费 YieldToken，并从
`yields` 后继续；不发送 Task.Dispatch 或 TaskFlow.Enter。

non-identity switch 必须由 Scheduler 显式完成：

1. 完整预检 prev/next TaskRef、固定 FlowRef、generation、context epoch、stack 与后续容量；
2. SaveCoreContext(prev)；
3. Task.Suspend(prev)，提交 `OnCpu/Live/Invalid -> Online/None/Valid`；
4. RestoreCoreContext(next)，并原子提交 CurrentTask/CurrentStack 和固定 Flow 的 CpuRef；
5. 在 next stack 上完成 finish；
6. Task.Dispatch(next)，提交 `Online/None/Valid -> OnCpu/Live/Invalid` 并生成不可伪造、一次性的私有
   Enter proof；
7. 用该 proof 向 next 的固定 Flow 交付 contextual `Action::Enter`。

Schedule 目标处理结束时，prev 的执行绑定已经改变，因此 prev 的 YieldToken 保持 pending。未来同一
Task context 恢复时，contextual Enter 以 TaskRef、FlowRef、generation、CPU、dispatch record 和
context epoch 交叉校验 token 后精确一次恢复 source 模型 continuation。模型 token 与
TaskThreadContext 不互相复制：前者只保存可序列化模型游标，后者只保存真实寄存器 continuation。

preflight rejection 发生在 token/任何对象提交前。post-commit failure、stale、错误 CPU/Flow/epoch 或
重复恢复都是终止失败，不回滚、不重试。

## CurrentTask、teardown 与用户资源

不同目标的 CurrentTask/CurrentStack 换绑只能发生在正式 switch commit 或 AP 入口 commit；普通 Signal
不得观察半绑定 pair。BootInitFlow 在 PhysicalDirect 下使用 boot-only BindTaskStack，并在 EarlyVm
接管后用 RefreshTaskStack 保持同一 pair identity；两者都不改变 Task lifecycle 或 preempt count。

Task 只维护唯一 Flow association，不复制 Flow lifecycle。terminal Disable 要求固定 Flow 已 Offline，
Cleanup 要求 Flow 已 Destroyed；终止路径不先制造可恢复 Online context。BootTask 不退出。

用户地址空间、files、credentials、signal 和 exec transaction 属于稳定 Task/Runtime 资源。每个用户型
TaskFlow 最多创建一个终身稳定、不可共享的 `UserAppRuntime` owned child；exec 只替换 Runtime 内的
ApplicationInstance，fork 才创建新的 Task/Flow/Runtime。

## 当前能力边界

本轮关闭单 CPU `yields`/schedule-return 基础。完整 SMP GlobalArbiter、cross-CPU mailbox、迁移仲裁与
schedule replay 保持 `P2 / 延期`；Task/Flow 固定关系不得为这些未来能力重新引入兼容字段或双写路径。
