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

## CpuRef 与 CurrentCPU

TaskFlow 是任务执行路径中唯一保存 `cpu_ref` 的对象；Task 不保存同义 CPU assignment 字段。
架构入口 adoption 和 Scheduler 的真实 switch/migration commit 可以写 `cpu_ref`，Flow 的普通
lifecycle/action 以及它同步 `drives` 的子树只能读取。Flow 即使离开 OnCpu 也保留最后归属 CpuRef；
调度 prepare 不得预写 next Flow，只有 switch commit 或正式 migration commit 可以改变它。

Flow handoff 在激活新 Flow 前必须把旧 Flow 的 CpuRef 复制给新 Flow。initial Flow 首次启动则从实际
入口或调度提交取得 CpuRef。`CurrentCPU` 只是在执行上下文中对 effective Flow 的 cpu_ref 解引用；
同步 drives 子孙继承这一解析来源，异步 emits 不继承。完整 CPU ownership、别名和解引用规则见
[`CPU`](cpu.md) 与 [`CpuGroup`](cpu-group.md)。

## CurrentTask、CurrentStack 与执行绑定

`CurrentTask` 表示 CPU 执行上下文已经提交的当前 Task binding，不直接等同于 effective TaskFlow 的
parent。`CurrentTask.Action::BindTask(task: Task)` 只执行 `BindCurrentTask(task)`：在 Scheduler 的正式
switch commit 边界把 CPU-local CurrentTask 换绑到即将运行的普通 Task；它不绑定 stack、不写 `sp`，
也不声明 CurrentTask object、owned child、lifecycle 或 `CurrentTaskSlot`。架构 switch 在同一个外层
commit 中独立恢复 next Task 的 `sp`，因此 finish continuation 开始前，CurrentTask 与从 live `sp`
校验得到的 `CurrentStack == task.stack` 必须同时可解析。不同 CPU 的 binding 彼此隔离。

TaskFlow parent 仍只表达结构归属，不是 current-task 存储。普通执行期解析时，所选 Task 必须同时是
effective TaskFlow 的 parent 和 owner，且该 Flow 必须是 Task 的 active Flow；目标必须为该 CPU 上的
`OnCpu/Live` 执行主体，并具有恰好一个有效、指向自身的 TaskRef。Scheduler RestoreCoreContext 的正式
switch commit 是唯一短暂例外：它可以把已完整预检的 `Online/None/Valid` 或
`Suspended/None/Valid` next Task 绑定为 CurrentTask。Suspended 使用预检的 active Flow；Online 尚无
active Flow 时使用预检固定的 initial FlowRef、CpuRef 与 TaskRef，并只建立
initial-startup-pending binding。Task 接受 Activate/Continue 前不得把该 binding 当成普通可执行 Flow
context；Online initial Flow Enable 提交 active binding 后才结束 pending。`CurrentTaskRef` 只从已绑定
Task 的唯一有效 TaskRef 派生。缺失 CPU 执行上下文、尚未绑定、错误 parent/owner、错误 CPU、非执行期
`OnCpu/Live` 或非 switch-commit `Online/None/Valid`/`Suspended/None/Valid`、悬空/过期/重复 TaskRef 都必须拒绝，失败不得提交
部分 binding。

TaskFlow 及其同步 `drives` continuation 继承同一 CPU-local binding；异步 `emits` 不继承，接收方从
自己的执行上下文重新解析。trace 必须记录 canonical CPU、Task、source Flow 和 source TaskRef。
BootTask 首次绑定前，`CurrentCPU` 可以独立从 `BootInitFlow.cpu_ref` 解引用，不能反向依赖尚未建立的
CurrentTask。BootTask 的静态初始禁止抢占属性不是 BindTask 的副作用。

`CurrentStack` 表示同一 CPU 执行上下文已经提交的当前内核栈 binding；它不是 TaskFlow parent、
`sp` 数值的全局副本，也不声明 object、owned child、lifecycle 或 `CurrentStackSlot`。Stack 是
`Task.stack` 的值类型属性，不是对象；目标值必须精确等于已绑定 `CurrentTask.stack`，live `sp` 必须
落在该属性描述的当前地址表示有效范围内。`CurrentStack` 解析先读取 CPU-local stack binding，再与
CurrentTask、effective TaskFlow、`CurrentTask.stack`、active translation controller 和 live `sp`
交叉校验；任何不一致都拒绝。不存在可单独调用或发送 Signal 的公开 `BindStack` Action。

`BootTask.stack` 是与静态 `init_task` 一起在镜像中构造的属性值，对应 Linux
`init_thread_union` / `init_stack`；不存在独立的启动栈对象、状态或引用。BootTask 是
`BindTask` 的入口例外：BootInitFlow 必须用两个 boot-only 上下文 Action 完成执行绑定。
`CurrentTask.Action::BindTaskStack(BootTask, BootTask.stack)` 只能在 PhysicalDirect 下首次调用，原子
建立 task/stack pair。`CurrentTask.Action::RefreshTaskStack(BootTask, BootTask.stack)` 只能在 EarlyVm
接管后调用；调用时，当前 CPU 已绑定的 pair 必须恰好仍是 BootTask 与 `BootTask.stack`，BootTask 也
必须仍然拥有当前执行权。该动作保持同一 pair 的 binding identity，原子刷新二者在当前地址环境中的
表示；成功后 CurrentTask 与 CurrentStack 在 EarlyVm 下继续解析为同一个 BootTask 及其 stack。
两种入口 Action 都是单个 Action/Signal，内部 task/stack 操作不可单独调用，continuation 不得观察
半绑定状态。任一校验或提交失败都必须保留调用前的 task binding、stack binding 和执行现场，且不得
创建新 Task、Stack 对象或独立 stack identity。

普通调度切换在架构 restore 中从 next TaskThreadContext 恢复 next `sp`，并在同一正式 switch commit
中调用 `BindTask(next)` 写入 next `tp`。`BindTask` 本身只提交 CurrentTask；外层架构 commit 依据已恢复
的 live `sp` 与 `next.stack` 提交 CurrentStack，finish continuation 只能在 task/stack pair 同时可解析
后运行。不同目标的 CurrentStack 换绑只允许发生在该 commit 或 AP 正式入口提交边界；不同 CPU 的两种
binding 都彼此隔离，snapshot 只保留 CPU-keyed contextual facts。

## 陷入期间的底层执行权

短期 Trap/Interrupt/Exception Flow 不替换 Task 的长期 continuation。陷入期间
`Task.active_flow` 与其 `TaskFlowRef` 保持绑定但暂停推进；effective TaskFlow 仍提供结构/CPU 解析来源，
CurrentTask/CurrentTaskRef 则从该 CPU 已提交的 task binding 解析并与 Flow 结构相互校验。活动执行上下文
只在其上叠加：

```text
TaskFlow -> TrapFlow -> InterruptFlow/ExceptionFlow -> concrete exception Flow
```

Trap 清理完成且架构返回检查点被一次性消费后，底层 TaskFlow 才从保存断点恢复。可调度异常携带整条
短期链迁移时，只更新底层 TaskFlow 的 CpuRef 和入口上下文；root TrapFlow 的正式 parent 始终是最初
接收陷入的 CPU Trap 资源。

## TaskFlowRef 与 lifecycle

`TaskFlowRef` 使用 private storage slot 加非零 generation。静态 Flow 使用固定 slot 和固定
generation；动态 slot 每次分配或回收后再声明时递增 generation。lookup 同时校验 slot 和
generation，旧引用在 slot 回收后必须稳定失效。

初始 Flow 和替换 Flow 的 lifecycle state 独立。Flow 必须显式经历其类型定义的
Preset/Setup/Enable/Disable/Cleanup；预分配 storage 不是已声明实例，alias 离开词法范围也不代表
Cleanup。Flow `Disable` 必须清除 active binding，`Cleanup` 只允许从 Offline 且不再 active 的实例
释放资源并到达 Destroyed。Destroyed storage 可以复用，但下一次声明必须使用新的 generation。
共同的 Preset guard 校验“可启动 binding”而不是只校验 `initial_flow`：初始 Flow 由 Task 的
`initial_flow` association 满足，exec/fork replacement Flow 则由此前完成的 `Bind` owner/parent 事实
满足。派生 Flow 不得通过覆盖 Preset 来绕过该共同 guard。

Task 退出必须先 Disable/Cleanup 所有 owned Flow；存在 Online Flow 时不得 terminal Disable Task，
存在未 Destroyed Flow 时不得 Cleanup Task。终止 Task 不经由 Online breakpoint 状态退出。

## OnCpu 执行边界与严格启动信号

`TaskFlow` 不保存 guard 字段或 guard 状态，也不声明 `process_guard` 类型块。除 Base-only structural
`Bind` 外，每次 lifecycle transition 或执行期 action 都即时要求 `flow.parent` 为 `OnCpu` 且
`TaskExecutionAuthority::Live`。Reserved AP carrier 不能执行普通 Flow action；HSM Startup 的架构接收
边界必须先把 authority 激活为 Live。fresh dynamic Flow 的 Bind 只建立 owner/parent/entry-source，
不执行 continuation，因此不受执行边界阻止。

普通 Task 的 initial Flow 只有一个严格 Startup 来源：Scheduler 在 Task 首次真实获得 CPU 后同步
drives Task.Activate；Task 从 Online 提交 OnCpu 后，Scheduler 直接向预检固定、仍为 Base 的 initial
Flow 发出 Startup（canonical Preset）。initial Flow 的 Enable 才提交 active binding。Task 再次获得
CPU 时必须从 Suspended 接受 Continue，随后由 Scheduler 直接向唯一 Online active Flow 发出 Continue。
Task 不转发两种 Flow Signal。两条候选在切换预检中必须恰有一条可接受；任何已发送 Signal被拒绝或
处理失败都会使根执行失败，不排队、不重试。

BootTask 是入口特例：它从模型初态已经 OnCpu，因此首次执行不经过 Scheduler 或 Task.Activate。
OpenSBI 发出 Kernel.Enable 后，Kernel 在仍为 Ready 的同一迁移过程中依次完成 Enable acceptance、
把 BootCPURef 赋给 BootInitFlow，以及 PhysicalDirect InitialActivation；三项全部提交后才同步驱动
BootInitFlow.Preset。这不是 Kernel 提交后的异步事件。`Startup` 只是 Preset 的显示名，`Started` 只是
Preset 接受 checkpoint。接收方必须仍为 Base 且 parent BootTask 必须为 OnCpu；重复启动、绕过
Kernel.Enable、缺失 CpuRef/controller association 或执行权不匹配立即失败。

只有当前 Task 的 active TaskFlow 可以根据自己的 CpuRef 向该 CPU owned Scheduler `emits Schedule()`；
Schedule 无 payload，Scheduler 必须从 sender Flow、CpuRef 和 CPU-local CurrentTask binding 推导 prev。
identity 选择由 Scheduler 向原 active Flow `emits Continue`，表达 schedule 返回；非 identity 选择由
Scheduler 在完成 switch 后按预检结果同步 drives next Task Activate/Continue，再直接向预检的
initial/active Flow emits Startup/Continue。

PID 1 的首次 dispatch 是跨栈 continuation：scheduler prepare 校验两侧 FlowRef/context，架构 switch
依次保存并 Suspend BootTask、恢复 PID 1 并提交 `CurrentTask.BindTask(KernelInitTask)` 与 CurrentStack，
随后在 `kernel_init_entry()` 的 finish 边界完成清理、同步 drives PID 1 Activate，再严格向
KernelInitFlow emits Startup。PID 1 Activate 只提交 OnCpu 与断点消费；KernelInitFlow Enable 才提交
active Flow binding。不得在 BootTask 栈上
预提交 PID 1 OnCpu，也不得预执行 `PreSmpInitPhase` 或任何后续叶子阶段。

每个 `ApIdleFlow[logical_id]` 与 `ApIdleTask[logical_id]` pointwise 绑定。BP 发出的 HSM Startup 是异步、
按 key 交付的 Signal；AP 由 boot-data `task_ptr.initial_flow` 解析并校验接收者，入口把 Reserved authority
变为 Live 后才接受 Startup。Flow 的 Startup pointwise 驱动同 key 的 `ApEntryPreludePhase`、
`ApSmpCallinPhase`、`ApOnlineIdlePhase`；BP 只能发起 HSM 并等待 completion，不得同步执行 AP phase。

## 首个 binding 与 exec replacement

boot idle successor binding 保持 `BootTask` 身份。`BootIdleFlow.Setup` 在首次真实调度切换前直接提交
owner 和 active binding，并到达 Ready；`BootTask.initial_flow` 仍指向 `BootInitFlow`。
`BootInitFlow.Online` 随后在发出首次 Schedule 之前发布；发布和 Schedule 请求都不改变 BootTask 的
OnCpu lifecycle。只有调度器实际选择 `next != prev` 并 Suspend BootTask 后，它才变为 Suspended。只有调度器未来恢复 `BootTask` 时，`BootIdleFlow` continuation 才
在 Task.Continue 从 Suspended 提交 OnCpu 后直接收到 Scheduler 的 Continue，驱动 `BootIdleEntryPhase` 并进入 idle loop。实现可以
保留 scheduler-owned 的 idle metadata、锁或
runqueue 投影视图，但不得把它们暴露成第二个 Task carrier。

successful exec 同样保持 owner Task identity。`KernelInitFlow.Enable` 只完成
`PayloadHandoffPreparePhase` 的可逆 precommit；它不发送 handoff action。真正 replacement 是
Kernel 与 `KernelInitFlow` 均已 Online 时、由已提交的 `Kernel.Enable` 唯一发出、仍受 parent Task
OnCpu 约束的 `CommitPayloadHandoff` action，并使用固定顺序：

1. fresh `UserAppFlow.Preset/Setup`；
2. 旧 Flow `Disable`；
3. Task 提交 old -> new active handoff；
4. 新 Flow `Enable` 并进入用户应用黑盒；
5. 旧 Flow `Cleanup`。

因此 `UserAppFlow` 使用完整 lifecycle override：它保留共同的 owner、`OnCpu/Live`、state 和完成事实，
但 Setup 到达 Ready 后不会自动发送 Enable；只有第 3 步 active handoff 已提交后，第 4 步才能显式发送
Enable。该完整替换不能实现为对子类型继承 guard 的局部削弱。

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

本轮不引入完整 AP offline/hotplug teardown、通用垃圾回收或 Signal 新语法。Flow teardown 继续只由显式
`Disable/Cleanup`、非零 generation 和正式 owner/binding 事实决定。
