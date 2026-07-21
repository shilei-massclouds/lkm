# TaskFlow

`TaskFlow` 是 Task 当前执行 continuation 的独立生命周期载体。每个 Flow 恰有一个 owner Task，
而 Task 与 Flow 的 identity、lifecycle state 和 teardown 事实分别保存。exec 或 boot idle handoff
只替换 active Flow，不替换 Task；fork/clone 则创建新的 Task 和新的初始 Flow。

Task carrier、TaskRef、类型级 lifecycle 及 Task teardown 前置条件见 [`Task`](task.md)。

## 静态与动态 Flow 实例

- `RootStream` 承载 `BootTask` 的 boot init Flow。它当前仍是 `BootInitFlowType` 的临时具名实例；
  其重命名以及全仓 Stream -> Flow 迁移不属于本轮。
- `BootIdleFlow` 是同一 `BootTask` 的 idle continuation；handoff 不建立第二个 Task carrier。
- `KernelInitFlow` 承载 `KernelInitTask` 的 `kernel_init()`、pre-SMP、initcall 和 exec 前内核
  continuation。
- `KthreaddFlow` 承载 `KthreaddTask` 的服务循环。
- PID 1 首次成功 exec 在执行点声明 fresh `UserAppFlow`。每次 fork/clone 为 fresh child Task 声明
  fresh fork-continuation `UserAppFlow`；child 后续每次 exec 再声明另一个 fresh `UserAppFlow`。

PID 1 首个用户 Flow、fork continuation Flow 和 child exec Flow 的临时具名见证不再是正式静态
对象，也不保留 compatibility alias。运行期实例 declaration/identity 规则由
[运行期实例声明](dynamic-instance-declaration.md)统一定义。

## 所有权与 active binding

每个 Flow 恰有一个 owner Task。`task_owns_flow(task, flow)` 记录 Task 曾经拥有该 Flow；
`task_active_flow_is(task, flow)` 记录当前 binding；`task_flow_handoff(task, old, new)` 记录历史。
一个 Task 可以按 exec 顺序拥有多个 Flow，但任一时刻最多一个 owned Flow Online。不同 Task 不得
共享同一 Flow 实例，应用映像名称也不得被提升为新的 Flow 类型。

owner、active binding 和 handoff 必须通过明确 action 与事实提交；不得通过 Flow 存储位置、调用栈
归属、角色名或 application persona 推导。Task 只保留这些受控协作事实，不复制 Flow lifecycle。

## TaskFlowRef 与 lifecycle

`TaskFlowRef` 使用 private storage slot 加非零 generation。静态 Flow 使用固定 slot 和固定
generation；动态 slot 每次分配或回收后再声明时递增 generation。lookup 同时校验 slot 和
generation，旧引用在 slot 回收后必须稳定失效。

初始 Flow 和替换 Flow 的 lifecycle state 独立。Flow 必须显式经历其类型定义的
Preset/Setup/Enable/Disable/Cleanup；预分配 storage 不是已声明实例，alias 离开词法范围也不代表
Cleanup。Flow `Disable` 必须清除 active binding，`Cleanup` 只允许从 Offline 且不再 active 的实例
释放资源并到达 Destroyed。Destroyed storage 可以复用，但下一次声明必须使用新的 generation。

Task 退出必须先 Disable/Cleanup 所有 owned Flow；存在 Online Flow 时不得 Disable Task，存在未
Destroyed Flow 时不得 Cleanup Task。

## Handoff 与 exec replacement

boot idle handoff 保持 `BootTask` 身份，顺序为旧 `RootStream` 停止、提交 active binding 到
`BootIdleFlow`，再进入 idle continuation。实现可以保留 scheduler-owned 的 idle metadata、锁或
runqueue 投影视图，但不得把它们暴露成第二个 Task carrier。

successful exec 同样保持 owner Task identity，并使用固定顺序：

1. fresh `UserAppFlow.Preset/Setup`；
2. 旧 Flow `Disable`；
3. Task 提交 old -> new active handoff；
4. 新 Flow `Enable` 并进入用户应用黑盒；
5. 旧 Flow `Cleanup`。

应用内部指令不进入 `UserAppFlow` 状态机。syscall、trap、files、credentials、signal 和地址空间
操作由发生该操作时的实际当前 Task 及相应内核资源对象承载；PID 1 路径的 owner 是
`KernelInitTask`，child 路径的 owner 是对应 fresh 用户 Task。Task 上的用户资源不会因为 exec
产生 persona wrapper。fork continuation 和每次后续 exec 必须使用不同的 Flow 实例。

所有可恢复的路径、映像和容量 precheck 必须在 Flow declaration 前完成。声明后的
owner/generation/lifecycle 不变量失败属于终止错误，不允许通过隐式 rollback 把同一 identity 恢复为
未声明。

## 当前能力边界

本轮不引入循环、并发调度、通用垃圾回收或 Signal 新语法。`RootStream` 命名以及全仓
Stream -> Flow 迁移仍保持 deferred；Flow teardown 继续只由显式 `Disable/Cleanup`、非零 generation
和正式 owner/binding 事实决定。
