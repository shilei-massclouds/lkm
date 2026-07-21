# Task 与 TaskFlow

`Task` 是内核中唯一的 task_struct-like carrier 类型。调度器调度 Task；Task 保存稳定身份、PID、
CPU 归属、调度状态、任务资源引用和 `TaskThreadContext`。`BootTask`、`KernelInitTask`、
`KthreaddTask` 以及用户 child 都是同一 `Task` 类型的独立实例，不再建立并行的 task kind、
process persona 或 idle-task carrier 类型。

`TaskFlow` 是 Task 当前执行 continuation 的独立生命周期载体。Task 与 Flow 必须分开：exec 或
boot idle handoff 可以替换 active Flow，但不能替换 Task；fork/clone 则创建新的 Task，并为它
建立独立 Flow。

## 静态实例与稳定身份

- `BootTask` 对应静态 `init_task` / PID 0 / swapper。入口期由 `RootStream` 承载 boot init Flow；
  `sched_init()` 只为同一 Task 建立 idle 角色，之后 handoff 到 `BootIdleFlow`，不建立第二个
  idle Task carrier。
- `KernelInitTask` 对应 `copy_process()` 创建的稳定 PID 1。`KernelInitFlow` 承载
  `kernel_init()`、pre-SMP、initcall 和 exec 前的内核 continuation；首次成功 exec 后，
  Task 身份仍是 `KernelInitTask`，active Flow 变为 fresh `UserAppFlow`。
- `KthreaddTask` 是独立 Task，`KthreaddFlow` 承载其服务循环。
- `UserTaskSet` 是 Task 集合，不是一个可复用 child carrier。每次 fork/clone 都创建 fresh、PID
  和 lifecycle 独立的 Task。当前 DSL 尚不能动态声明实例，因此 `UserChildTask1` 只是一条不可
  复用的具名见证；它不能表示下一次 fork 的身份。

`RootStream` 当前仍是 `BootInitFlowType` 的临时具名实例。它的重命名以及全仓 Stream -> Flow
迁移不属于本轮，不得借 Task/TaskFlow 闭合顺带完成。

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
操作仍由 `KernelInitTask` 及相应内核资源对象承载；Task 上的用户资源不会因为 exec 产生 persona
wrapper。fork continuation 和每次后续 exec 必须使用不同的 Flow 实例。

## 当前能力边界

动态匿名 Task/Flow 创建、owned Flow 集合的 verifier 量化以及通过运行期选择实例调用通用
lifecycle，仍由 model 中的结构化 deferred 条目记录。在这些能力闭合前，具名 child/Flow 只能
证明一个具体实例的所有权、handoff 和销毁顺序，不能作为 compatibility alias 或可复用 slot。
