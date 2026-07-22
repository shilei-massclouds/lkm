# TaskFlow

`TaskFlow` 是 Task 当前执行 continuation 的独立生命周期载体。每个 Flow 恰有一个 owner Task，
而 Task 与 Flow 的 identity、lifecycle state 和 teardown 事实分别保存。exec 只替换 active Flow，
不替换 Task；fork/clone 则创建新的 Task 和新的初始 Flow。`TaskFlow` 是 `PhaseObject` 的子类型，
并以 typed `parent: Task` 约束每个 Flow 的 owner carrier；`BootInitFlow` 是 `BootTask` 的初始
TaskFlow，同时继续承担启动阶段编排。

Task carrier、TaskRef、类型级 lifecycle 及 Task teardown 前置条件见 [`Task`](task.md)。

## 静态与动态 Flow 实例

- `BootInitFlow` 是 `BootTask.initial_flow` 指向的初始 TaskFlow，从 `_start` 直接编排 boot execution
  continuation 的叶子阶段，直到首次真实 PID 1 switch 的 pre-commit 边界。
- `BootIdleFlow` 是 `BootTask` 的后继 idle Flow。`BootInitFlow.Enable` 在不可逆切换前完成完整预检，
  直接建立其 owner/active binding；这不建立第二个 Task carrier，也不改写 `BootTask.initial_flow`。
- `KernelInitFlow` 是 `KernelInitTask.initial_flow` 指向的初始 Flow，直接编排 `kernel_init()`、
  pre-SMP、initcall、payload prepare 和 exec 前内核 continuation；不经 `SmpRuntimePhase` 或
  `PayloadPhase` 包装 lifecycle。
- `KthreaddFlow` 是 `KthreaddTask.initial_flow` 指向的初始 Flow，承载服务循环。
- PID 1 首次成功 exec 在执行点建立 fresh `UserAppFlow` occurrence；formal model 以唯一具体实例
  `Pid1UserAppFlow` 表示这个首个 occurrence，implementation 仍须在专用 storage slot 上执行
  `declare()`/`bind()`。每次 fork/clone 为 fresh child Task 声明 fresh fork-continuation
  `UserAppFlow`；child 后续每次 exec 再声明另一个 fresh `UserAppFlow`。

`Pid1UserAppFlow` 是首个 exec occurrence 的正式身份，不是 persona/wrapper 或 compatibility alias；
fork continuation Flow 和 child exec Flow 的临时具名见证不再是正式静态对象。运行期实例
declaration/identity 规则由
[运行期实例声明](dynamic-instance-declaration.md)统一定义。

## 所有权与 active binding

每个 Flow 恰有一个 owner Task。`Task.initial_flow` 是创建时绑定且不随 exec/idle successor 改写的
typed association；`task_owns_flow(task, flow)` 记录 Task 曾经拥有该 Flow；
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

## Dispatch guard 与 lossy 启动信号

`TaskFlow` 不保存 guard 字段或 guard 状态，也不声明 `process_guard` 类型块。统一谓词
`task_flow_dispatch_guard_satisfied(flow, window)` 在每次 lifecycle transition 或执行期 action
尝试时即时求值，并且只有在以下两个条件同时满足时成立：

1. `flow.parent` 的当前 lifecycle state 是 `Online`；
2. `window.current_task` 的当前 `TaskRef` 解引用后，其 runtime identity 等于 `flow.parent`。

每个 TaskFlow 推进点必须在自己的 `depends_on` 中显式依赖该谓词；类型继承只复用 parent 类型约束
和已声明 process，不为 Flow 实例物化 guard。fresh dynamic Flow 的 Base-only structural `Bind` 只
建立 owner/parent/entry-source，不执行 continuation，因此不受 dispatch guard 阻止。

每个 Task 的初始 Flow 有两个 lossy `Preset` 信号来源：Task 提交 `Online` 后，以及对应
DispatchWindow 在真实 switch commit 后更新 `current_task` 时。接收方只在自身仍为 `Base` 且即时
dispatch guard 成立时推进；否则立即记录 discarded，不排队、不重试，也不阻塞发出方。Task 再次
获得 CPU 时，非 Base 的 initial Flow 丢弃重复启动信号，并从 active Flow 保存的 continuation 恢复。

PID 1 的首次 dispatch 是跨栈 continuation：scheduler 在 switch commit 更新 CurrentTaskSlot 与
DispatchWindow 后接受 `KernelInitFlow.Preset`，但 runtime lowering 必须把 Preset body 及其叶子阶段
代码放到 `kernel_init_entry()` 验证 PID 1 vmalloc stack 之后执行。不得为了同步实现 signal 而在
BootTask 栈上预执行 `PreSmpInitPhase` 或任何后续叶子阶段。

## 首个 binding 与 exec replacement

boot idle successor binding 保持 `BootTask` 身份。`BootIdleFlow.Setup` 在首次真实调度切换前直接提交
owner 和 active binding，并到达 Ready；`BootTask.initial_flow` 仍指向 `BootInitFlow`。
`BootInitFlow.Online` 随后在真实 BootTask→KernelInitTask
switch commit 的紧邻边界发布。只有调度器未来恢复 `BootTask` 时，`BootIdleFlow` continuation 才
驱动 `BootIdleEntryPhase` 并进入 idle loop。实现可以保留 scheduler-owned 的 idle metadata、锁或
runqueue 投影视图，但不得把它们暴露成第二个 Task carrier。

successful exec 同样保持 owner Task identity。`KernelInitFlow.Enable` 只完成
`PayloadHandoffPreparePhase` 的可逆 precommit；真正 replacement 是 `KernelInitFlow` 已 Online 时、
仍受 dynamic dispatch guard 约束的 `CommitPayloadHandoff` action，并使用固定顺序：

1. fresh `UserAppFlow.Preset/Setup`；
2. 旧 Flow `Disable`；
3. Task 提交 old -> new active handoff；
4. 新 Flow `Enable` 并进入用户应用黑盒；
5. 旧 Flow `Cleanup`。

Hello/Smoke 不执行 Flow replacement：`KernelInitFlow` 保持 Online，action 只进入已经绑定的内核态
no-return entry。UserBoot 完成上述顺序后提交 `KernelInitFlow.PayloadHandoffCommitted`；该 checkpoint
是成功 exec 的 commit 边界，不能由 precommit 阶段伪造。

应用内部指令不进入 `UserAppFlow` 状态机。syscall、trap、files、credentials、signal 和地址空间
操作由发生该操作时的实际当前 Task 及相应内核资源对象承载；PID 1 路径的 owner 是
`KernelInitTask`，child 路径的 owner 是对应 fresh 用户 Task。Task 上的用户资源不会因为 exec
产生 persona wrapper。fork continuation 和每次后续 exec 必须使用不同的 Flow 实例。

所有可恢复的路径、映像和容量 precheck 必须在 Flow declaration 前完成。声明后的
owner/generation/lifecycle 不变量失败属于终止错误，不允许通过隐式 rollback 把同一 identity 恢复为
未声明。

## 当前能力边界

本轮不引入循环、并发调度、通用垃圾回收或 Signal 新语法。Flow teardown 继续只由显式
`Disable/Cleanup`、非零 generation 和正式 owner/binding 事实决定。
