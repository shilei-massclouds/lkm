# Task

`Task` 是内核中唯一的 task_struct-like carrier 类型。调度器调度 Task；Task 保存稳定身份、PID、
CPU 归属、调度状态、任务资源引用和 `TaskThreadContext`。`BootTask`、`KernelInitTask`、
`KthreaddTask` 以及用户 child 都是同一 `Task` 类型的独立实例，不再建立并行的 task kind、
process persona 或 idle-task carrier 类型。

Task 当前执行 continuation 的独立生命周期由 [`TaskFlow`](task-flow.md) 承载。exec 或 boot idle
handoff 可以替换 active Flow，但不能替换 Task；fork/clone 才创建新的 Task，并为它建立独立 Flow。

## 静态与动态实例的稳定身份

- `BootTask` 对应静态 `init_task` / PID 0 / swapper。入口期由 `RootStream` 承载 boot init Flow；
  `sched_init()` 只为同一 Task 建立 idle 角色，之后 handoff 到 `BootIdleFlow`，不建立第二个 idle
  Task carrier。
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
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
Online --Disable--> Offline --Cleanup--> Destroyed
```

`KernelInitTask`、`KthreaddTask` 等静态实例以及 `declare` 创建的 runtime child 都完整继承这张
状态图；实例不得重复声明、局部合并或遮蔽其中任一状态、迁移、ensures 或 invariant。静态实例与
运行期实例进入继承状态时都必须检查同一类型 invariant，其中 `self` 绑定到实际 instance identity，
而不是类型名或 declaration site。

`BootTask` 是唯一允许的静态 Task lifecycle override。它必须显式替换整张状态图，以承载静态
`init_task` storage、固定 `TaskRef` 以及入口 binding milestone 的受控提交；这些 boot-only
语义不得合并进普通 Task lifecycle。`tp` 的物理/虚拟绑定与早期 preemption 事实归
`EntryPreludePhase` 私有的 `BootTaskEntryBinding`，不属于 Task carrier lifecycle。除这一完整
override 外，不存在实例级 Task lifecycle 权威。

`BootTaskEntryBinding` 只协调同一静态 carrier 的入口可寻址性：Base 表示尚未绑定，Prepared 表示
`tp` 使用物理地址且初始抢占关闭条件已建立，Ready 表示 `EarlyVm` 下的虚拟地址绑定已提交。它不
建立新的 Task identity/storage/Flow ownership，也不进入公共 Task 或 Context API。

普通 Task 的 `Preset` 统一建立 fresh identity、`TaskRef`、初始 Flow ownership 与 clone
specification；`Setup` 统一消费 `TaskCreationCore` 已提交的 copy-process 事实，并建立 PID、thread
context、scheduler entity 和 New/not-enqueued 状态；`Enable` 统一消费 running、runqueue publication
与初始 Flow binding，并保证恰有一个 active Flow。PID 1 入口、`CLONE_FS`、kthreadd flags、provider
与 schedule-loop 等角色事实属于创建它们的 Phase，不得成为 `Task` 类型 invariant。

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

Task `Disable` 前必须保证所有 owned Flow 已退出 Online，且 exit 路径已按序清除 active binding；
Task `Cleanup` 前必须保证所有 owned Flow 已到达 Destroyed。Flow alias 或内部存储退出词法/表槽范围
不表示 Flow 已被销毁。具体 Flow lifecycle、handoff 和 exec replacement 规则见
[`TaskFlow`](task-flow.md)。

## 当前能力边界

本轮只引入顺序 `drives` 中的运行期实例声明，不同时引入循环、并发调度、通用垃圾回收或 Signal
新语法。运行期 alias 离开词法作用域不销毁实例；Task teardown 继续只由显式
`Disable/Cleanup`、TaskRef generation 校验和正式所有权事实决定。
