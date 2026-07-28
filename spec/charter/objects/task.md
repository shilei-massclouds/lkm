# Task

`Task` 是内核中唯一的 task_struct-like carrier 类型。调度器调度 Task；Task 物理拥有稳定身份、PID、
调度状态、任务资源引用和 `TaskThreadContext`。CPU 归属只保存在当前/可恢复 `TaskFlow.cpu_ref`；
Task 不保存同义 CPU assignment 字段。`BootTask`、`KernelInitTask`、
`KthreaddTask` 以及用户 child 都是同一 `Task` 类型的独立实例，不再建立并行的 task kind、
process persona 或 idle-task carrier 类型。

Task 当前执行 continuation 的独立生命周期由 [`TaskFlow`](task-flow.md) 承载。exec 可以替换
active Flow，但不能替换 Task；fork/clone 才创建新的 Task，并为它建立独立 Flow。每个 Task 以
typed `initial_flow` association 固定记录创建时的初始 Flow；`BootInitFlow` 是 `BootTask` 的初始
TaskFlow，同时因 TaskFlow 继承 PhaseObject 而编排启动阶段。

## 静态与动态实例的稳定身份

- `BootTask` 对应静态 `init_task` / PID 0 / swapper，在镜像入口前已经存在并从首个内核指令起
  处于 `OnCpu`。`BootInitFlow` 是它的初始 TaskFlow；`sched_init()` 只为
  同一 Task 建立 idle 角色，`BootIdleFlow` 是该 Task 的后继 owned/active TaskFlow，不建立第二个
  idle Task carrier。
- `KernelInitTask` 对应 `copy_process()` 创建的稳定 PID 1。首次成功 exec 只替换 active Flow，Task
  身份仍是 `KernelInitTask`。
- `KthreaddTask` 是具有独立 PID、调度状态与生命周期的 Task。
- `UserTaskSet` 是 Task 集合，不是一个可复用 child carrier。每次 fork/clone 都通过 `declare`
  创建 fresh、PID 和 lifecycle 独立的 `Task`；后续 exec 保持该 Task identity。

PID 1、child Task 以及各自 Flow 的临时具名见证不再是正式静态对象，也不保留 compatibility alias。
运行期实例 identity 和声明规则由[运行期实例声明](dynamic-instance-declaration.md)统一定义；Task 的
集合成员、Flow ownership 与 active binding 仍必须由显式 lifecycle/action 与事实提交。

## 类型级生命周期权威

`Task` 类型拥有普通 Task 的唯一完整 lifecycle 定义：

```text
Base --Preset--> Prepared --Setup--> Ready --Enable(initial Flow breakpoint)--> Online
Online --Continue--> OnCpu --Suspend--> Online
OnCpu --Disable(terminal)--> Offline --Cleanup--> Destroyed
```

`KernelInitTask`、`KthreaddTask` 等静态实例以及 `declare` 创建的 runtime child 都完整继承这张
状态图；实例不得重复声明、局部合并或遮蔽其中任一状态、迁移、ensures 或 invariant。静态实例与
运行期实例进入继承状态时都必须检查同一类型 invariant，其中 `self` 绑定到实际 instance identity，
而不是类型名或 declaration site。

`Online` 是普通 Task 唯一持有有效、可恢复 TaskFlow 断点的状态；它同时表示 Task 已发布并可被
Scheduler 派发。idle task 可以是 runqueue 的 `rq->idle` / `rq->curr` 而不属于普通 runnable class
queue，因此 Online 不承诺普通 RunQ membership。`OnCpu` 表示该 Task 是 CPU 唯一 current/执行权
carrier；它是 Task 专属扩展状态，不得扩展为 `Object`、`PhaseObject` 或其它类型的通用生命周期状态。

Task lifecycle 之外必须保存两组正交状态：

- `TaskExecutionAuthority::{None, Reserved, Live}`：普通 Online Task 为 None，真实执行的普通 Task 为
  Live；Reserved 只用于已经成为某 CPU `rq->idle/rq->curr` 但尚未收到 HSM 执行权的 AP idle Task。
- `TaskBreakpointState::{Invalid, Prepared, Valid}`：Setup 只建立 Prepared context；Enable 把它绑定到
  `initial_flow` 并发布 Valid；Continue 校验并消费 Valid context，OnCpu 期间为 Invalid；Suspend 保存
  active Flow context 并再次发布 Valid。

`TaskThreadContext` 固定物理保存长期 Task continuation 的核心寄存器集合、breakpoint state、经
slot/generation 校验的 `TaskFlowRef`、可选且同样校验 generation 的 root `TrapFlowRef`，以及真实
save/restore 观察计数。root 引用通过活动 child FlowRef 链定位当前叶 Flow；不存在活动陷入时必须为
空。`tp` 及由 effective TaskFlow 解析的 CurrentTask/CPU
identity 都不属于可恢复寄存器现场。Valid context 必须绑定恰好一个仍由该 Task 拥有的
FlowRef；Prepared/Invalid context 不得被 Scheduler 恢复。

`BootTask` 是允许的静态 Task lifecycle override。它没有 Preset、Setup 或 Enable，初始状态为
`OnCpu/Live/Invalid`，并保留 `OnCpu --Suspend--> Online --Continue--> OnCpu`
往返。入口初态表示固件/架构入口已把 boot CPU 执行权直接交给该 Task；它不由 Scheduler Continue
建立，也不依赖尚未建立的 BootRunQueue。该状态同时保证静态 `init_task` storage、
固定 PID 0、`TaskRef::BOOT` 和 canonical identity；不得把会变化的 `tp`、entry role、preemption 状态
或 active TaskFlow 放入 invariant。boot-only const 初始化器直接构造这一状态，不复用普通 Task
lifecycle 方法。除这一完整 override 外，不存在实例级 Task lifecycle 权威。

每个 secondary CPU 的 `ApIdleTask[logical_id]` 是按 logical-id 解释的 replicated family。
`init_idle()`/BP 预建结束时它已经是该 CPU 的 `rq->idle/rq->curr`，初态直接为
`OnCpu/Reserved/Invalid`，不是普通 `Base -> ... -> Online` 发布，也不进入普通 runnable class
queue。对应 `ApIdleFlow[logical_id]` 已完成 owner/parent/initial binding，但保持 Base。SBI HSM 入口
只验证 Linux `{task_ptr, stack_ptr}` boot data、加载独立 `tp/sp` 并把 authority 从 Reserved 激活为
Live；它不改变 Task lifecycle，也不发送 Task Enable/Continue。随后 keyed HSM Startup 严格启动同一
logical-id 的 initial idle Flow。AP 首次 Suspend 才产生第一个可恢复的 Online/Valid 断点，后续切换
完全使用普通 Suspend/Continue。

`tp` 的物理/虚拟绑定与早期 preemption 事实归 `BootInitFlow.Preset` 私有的
`BootTaskEntryBinding`，不属于 Task carrier lifecycle。入口各阶段只能验证 `BootTask.OnCpu`
稳定 invariant，不能推进或重放其生命周期。

`BootTaskEntryBinding` 只协调同一静态 carrier 的入口可寻址性：Base 表示尚未绑定，Prepared 表示
`tp` 使用物理地址且初始抢占关闭条件已建立，Ready 表示 `EarlyVm` 下的虚拟地址绑定已提交。它不
建立新的 Task identity/storage/Flow ownership，也不进入公共 Task 或 Context API。

普通 Task 的 `Preset` 统一建立 fresh identity、`TaskRef`、typed initial Flow association、初始 Flow ownership 与 clone
specification；`Setup` 统一消费 `TaskCreationCore` 已提交的 copy-process 事实，并建立 PID、thread
context、首次寄存器字节、Prepared breakpoint storage、scheduler entity 和 New/not-enqueued 状态；
`Enable` 统一消费 running、runqueue publication 与初始 Flow binding，把 Prepared context 绑定该
FlowRef 并原子发布 `Online/None/Valid`；它不启动 Flow。`BootInitRestInitPhase` 必须完整驱动新 Task
的 Preset/Setup/Enable：其中 Preset/Setup 对应 carrier 创建与 copy-process 收口，Enable 对应
`wake_up_new_task()` 后的 Online 发布。PID 1 与 kthreadd 的 initial Flow 都只能在各自 Task 首次真实
获得 CPU 后启动；不得把 Task Online 误写成 initial Flow 已启动。PID 1 入口、`CLONE_FS`、kthreadd
flags、provider
与 schedule-loop 等角色事实属于创建它们的 Phase，不得成为 `Task` 类型 invariant。

Scheduler 是普通 `Task.Continue` 与 `Task.Suspend` 的唯一发送者。一次真实切换分为 prepare、物理
switch、finish：prepare 通过 CurrentTask 选择器校验 prev `OnCpu/Live/Invalid`、prev active Flow 与
next `Online/None/Valid` 的 FlowRef/generation；架构 switch 保存 prev、恢复 next 的
`ra/sp/s0..s11` 并从 next Task identity 单独建立 `tp`；next 栈上的 finish 原子提交 prev
`Online/None/Valid`、next `OnCpu/Live/Invalid`、next Flow CpuRef、active Flow 与观察事实，然后严格
Startup/Continue next Flow。若 prev 是
终止 Task，finish 提交 `OnCpu --Disable--> Offline`，context 保持 Invalid，并在 next 侧 Cleanup，
不得先制造不可恢复的 Online。`prev == next` 是无动作路径：不发送 Suspend/Continue，不保存/恢复
context，也不改变 lifecycle、authority、Flow binding、CurrentTask 选择结果或计数。

调度 finish 是普通 Signal 不可观察中间态的原子提交边界：prev 失去执行权、next 获得
`OnCpu/Live`、next Flow 激活与 CurrentTask 切换必须同时可见。terminal switch 必须在回收 prev 前
先使 next 的 CurrentTask 可解析；identity switch 不改变解析结果。

Task 接受 Continue 并提交 OnCpu 后必须严格启动恰好一个 execution continuation：若 initial Flow 仍为
Base，则向它发出 Startup（canonical Preset）；否则向唯一 active Flow 发出 Continue。发送前两条候选
必须恰有一条可接受；缺失、歧义或处理失败都使当前 Signal 根执行失败，不排队重试，也不静默忽略。

## CopyProcess 的源执行权

`TaskCreationCore.CopyProcess` 从当前正在执行的 Task 复制，而不是从一个仅可调度的 Online Task
复制。调用时 `src_task` 必须同时满足：lifecycle 为 `OnCpu`、execution authority 为 `Live`，其
active Flow 等于 effective TaskFlow，且其 `TaskRef` 与 CurrentTaskRef 的解析结果一致。`Online`、
`Reserved`、非 current 或 TaskRef 不匹配都必须在任何 destination/copy-process 状态修改前拒绝。

该契约对 BootTask、PID 1 和后续用户 Task 一视同仁；BootTask 只因入口时确实是
`OnCpu/Live/current` 而可作为 rest-init 的源，不存在按名称放宽或把底层 persistent carrier
lifecycle 当成执行权的特判。成功事实必须记录实际 source Task identity 与经 generation 校验的
TaskRef。

## TaskRef 与身份存储

`TaskRef` 必须使用 private storage slot 加非零 generation。静态 Task 使用固定 slot 和固定
generation；动态 slot 每次分配或回收后再声明时递增 generation。lookup 必须同时校验两者，所以
保存旧 generation 的 current/runqueue/wait/checkpoint 引用在 slot 回收后稳定失效。PID、角色名和
storage 地址都不能替代这种 identity。

Task 的 scheduler/current/wait 等使用者只保存 `TaskRef`，再通过 owner storage 验证引用并读取
metadata；不得以角色 enum、persona wrapper 或公开存储字段形成平行 identity。

## Flow 协作与 teardown 前置条件

Task 只维护 Flow ownership 与唯一 active binding，不复制 Flow lifecycle 状态。一个 Task 可以按
exec 顺序拥有多个 Flow，但任一时刻最多一个 owned Flow Online；不同 Task 不得共享同一 Flow。

普通 Task terminal `Disable` 必须从 OnCpu/Live/Invalid 直接到 Offline/None/Invalid，并保证所有 owned
Flow 已退出 Online且 active binding 已清除；Task `Cleanup` 前必须保证所有 owned Flow 已到达
Destroyed。Flow alias 或内部存储退出词法/表槽范围
不表示 Flow 已被销毁。`BootTask` 不退出；其 initial Flow 是 `BootInitFlow`，后继
`BootIdleFlow` 的 active binding 在 `BootInitFlow.Enable` 的调度切换预检中建立。TaskFlow 的每次
推进还必须动态检查 parent Task 为 OnCpu。具体 Flow lifecycle、严格 initial-flow 启动和 exec
replacement 规则见
[`TaskFlow`](task-flow.md)。

## 当前能力边界

本轮只引入顺序 `drives` 中的运行期实例声明，不同时引入循环、并发调度、通用垃圾回收或 Signal
新语法。运行期 alias 离开词法作用域不销毁实例；Task teardown 继续只由显式
`Disable/Cleanup`、TaskRef generation 校验和正式所有权事实决定。
