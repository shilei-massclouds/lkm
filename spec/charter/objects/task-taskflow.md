# Task 与 TaskFlow

`Task` 是内核中唯一的 task_struct-like carrier 类型。调度器调度 Task；Task 保存稳定身份、PID、
CPU 归属、调度状态、任务资源引用和 `TaskThreadContext`。`BootTask`、`KernelInitTask`、
`KthreaddTask` 以及用户 child 都是同一 `Task` 类型的独立实例，不再建立并行的 task kind、
process persona 或 idle-task carrier 类型。

`TaskFlow` 是 Task 当前执行 continuation 的独立生命周期载体。Task 与 Flow 必须分开：exec 或
boot idle handoff 可以替换 active Flow，但不能替换 Task；fork/clone 则创建新的 Task，并为它
建立独立 Flow。

## 静态与动态实例的稳定身份

- `BootTask` 对应静态 `init_task` / PID 0 / swapper。入口期由 `RootStream` 承载 boot init Flow；
  `sched_init()` 只为同一 Task 建立 idle 角色，之后 handoff 到 `BootIdleFlow`，不建立第二个
  idle Task carrier。
- `KernelInitTask` 对应 `copy_process()` 创建的稳定 PID 1。`KernelInitFlow` 承载
  `kernel_init()`、pre-SMP、initcall 和 exec 前的内核 continuation；首次成功 exec 在执行点声明
  fresh `UserAppFlow`，Task 身份仍是 `KernelInitTask`，active Flow 改为该运行期实例。
- `KthreaddTask` 是独立 Task，`KthreaddFlow` 承载其服务循环。
- `UserTaskSet` 是 Task 集合，不是一个可复用 child carrier。每次 fork/clone 都通过 `declare`
  创建 fresh、PID 和 lifecycle 独立的 `Task`，并为该 Task 声明 fresh fork-continuation
  `UserAppFlow`。后续 exec 保持 Task identity 并声明另一个 fresh `UserAppFlow`。

PID 1 首个用户 Flow、child Task、fork continuation Flow 和 child exec Flow 的临时具名见证
不再是正式静态对象，也不保留 compatibility alias。运行期实例 identity 和声明规则由
[运行期实例声明](dynamic-instance-declaration.md)统一定义；Task/Flow 的 owner、集合成员和 active
binding 仍必须由显式 lifecycle/action 与事实提交。

`RootStream` 当前仍是 `BootInitFlowType` 的临时具名实例。它的重命名以及全仓 Stream -> Flow
迁移不属于本轮，不得借 Task/TaskFlow 闭合顺带完成。

## 类型级生命周期权威

`Task` 类型拥有普通 Task 的唯一完整 lifecycle 定义：

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
Online --Disable--> Offline --Cleanup--> Destroyed
```

`KernelInitTask`、`KthreaddTask` 等静态实例以及 `declare` 创建的 runtime child 都完整继承这张
状态图；实例不得重复声明、局部合并或遮蔽其中任一状态、迁移、ensures 或 invariant。静态实例与
运行期实例进入继承状态时都必须检查同一类型 invariant，其中 `self` 绑定到实际 instance identity，
而不是类型名或 declaration site。

`BootTask` 是唯一允许的静态 Task lifecycle override。它必须显式替换整张状态图，以承载静态
`init_task` storage、`tp` 的物理地址到虚拟地址切换以及早期 preemption 事实；这些 boot-only
语义不得合并进普通 Task lifecycle。除这一完整 override 外，不存在实例级 Task lifecycle 权威。

普通 Task 的 `Preset` 统一建立 fresh identity、`TaskRef`、初始 Flow ownership 与 clone
specification；`Setup` 统一消费 `TaskCreationCore` 已提交的 copy-process 事实，并建立 PID、thread
context、scheduler entity 和 New/not-enqueued 状态；`Enable` 统一消费 running、runqueue publication
与初始 Flow binding，并保证恰有一个 active Flow。`Disable/Cleanup` 继续要求所有 owned Flow 先
分别退出 Online / 到达 Destroyed，Task 才能离线与销毁。PID 1 入口、`CLONE_FS`、kthreadd flags、
provider 与 schedule-loop 等角色事实属于创建它们的 Phase，不得成为 `Task` 类型 invariant。

## 所有权与 active binding

每个 Flow 恰有一个 owner Task。`task_owns_flow(task, flow)` 记录 Task 曾经拥有该 Flow；
`task_active_flow_is(task, flow)` 记录当前 binding；`task_flow_handoff(task, old, new)` 记录历史。
一个 Task 可以按 exec 顺序拥有多个 Flow，但任一时刻最多一个 owned Flow Online。不同 Task 不得
共享同一 Flow 实例，应用映像名称也不得被提升为新的 Flow 类型。

初始 Flow 和替换 Flow 的 lifecycle state 独立。Task 退出必须先 Disable/Cleanup 所有 owned
Flow；存在 Online Flow 时不得 Disable Task，存在未 Destroyed Flow 时不得 Cleanup Task。Flow
alias 或内部存储退出词法/表槽范围不表示 Flow 已被销毁。

## Handoff 与 exec 顺序

boot idle handoff 保持 `BootTask` 身份，顺序为旧 `RootStream` 停止、提交 active binding 到
`BootIdleFlow`，再进入 idle continuation。实现可以保留 scheduler-owned 的 idle metadata、锁或
runqueue 投影视图，但不得把它们暴露成第二个 Task carrier。

successful exec 也保持 Task 身份，并使用固定顺序：

1. fresh `UserAppFlow.Preset/Setup`；
2. 旧 Flow `Disable`；
3. Task 提交 old -> new active handoff；
4. 新 Flow `Enable` 并进入用户应用黑盒；
5. 旧 Flow `Cleanup`。

应用内部指令不进入 `UserAppFlow` 状态机。syscall、trap、files、credentials、signal 和地址空间
操作由发生该操作时的实际当前 Task 及相应内核资源对象承载；PID 1 路径的 owner 是
`KernelInitTask`，child 路径的 owner 是对应 fresh 用户 Task。Task 上的用户资源不会因为 exec
产生 persona wrapper。fork continuation 和每次后续 exec 必须使用不同的 Flow 实例。

Task 与 Flow 的引用都必须使用 storage slot 加非零 generation。静态 Task/Flow 使用固定 slot 和
固定 generation；动态 slot 每次分配或回收后再声明时递增 generation。lookup 必须同时校验两者，
所以保存旧 generation 的 current/runqueue/wait/checkpoint 引用在 slot 回收后稳定失效。PID、角色名
和 storage 地址都不能替代这种 identity。

## 当前能力边界

本轮只引入顺序 `drives` 中的运行期实例声明，不同时引入循环、并发调度、通用垃圾回收或 Signal
新语法。`RootStream` 的命名以及全仓 Stream -> Flow 迁移仍保持 deferred。运行期 alias 离开词法
作用域不销毁实例；Task/Flow teardown 继续只由显式 `Disable/Cleanup` 和正式所有权事实决定。
