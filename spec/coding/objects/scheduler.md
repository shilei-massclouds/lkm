# Scheduler 编码映射

本文件是 [`scheduler.spec`](../../model/objects/scheduler.spec) 的唯一权威 Coding 映射。每个 possible
CPU 的 canonical `Cpu` 存储槽恰好内嵌一个 `Scheduler`；它直接表示 Linux CPU-local `struct rq`，
不得同时存在全局 Scheduler singleton、独立 RunQueue body 或 `[CpuRunQueueMetadata; MAX_CPUS]` 镜像。
root/sched domain、全局 topology 和测试 Task storage 是 Scheduler 之外的共享/所属对象。

## 引用与访问边界

Scheduler 公开边界只接收 `FlowRef`、`CpuRef`、`TaskRef` 和 `SchedClassRef`。`curr/idle/stop`
及 stop/DL/RT/fair/idle 类队列只存稳定 `TaskRef`；不存储 Task 裸指针、role wrapper 或具名
`BootTask`/`KernelInitTask` 字段。`SchedulerRef` 降低为所属 `CpuRef`，解引用必须经过
`CpuGroup.cpus[logical_id].scheduler` 并校验 generation/owner。

`SchedulerTaskAccess` 是唯一 Task bridge：它按 TaskRef 解析 Task lifecycle/authority、调度状态、pending
wake signal、initial/active FlowRef 和 `TaskThreadContext`。它不复制这些事实，也不对外暴露可变
Task。需要同时修改 prev/next 时，必须先拒绝 alias，再通过 split-at/index 或等价的不重叠
借用获得两个 carrier；identity path 不取得成对可变借用。

## lifecycle 与队列资格

`sched_init` 为全部 possible CPU 构造并准备 Scheduler。CPU0 在 boot CPU scheduling handoff 中
Ready→Online；AP 只在对应 CPU online handoff 时转为 Online。Task lifecycle 与调度资格正交：
Online 只意味着从未运行的 initial context，Suspended 才表示已保存的 active context；两者均与
`on_rq`/runnable/blocked 正交，资格由 Scheduler 的 class membership 及 PreparePrev 结果表示。
wake/select/enqueue 始终定位目标 CPU 所属 Scheduler。

## Schedule 处理

`schedule(flow_ref)` 无调度 payload。入口依次解引用 sender Flow、其 CpuRef、该 CPU 的 Scheduler 和
CurrentTask binding，必须证明 sender 是 current Task 的唯一 active Flow，并拒绝 cross-CPU、stale
Flow 或错误 current binding。持有 CPU-local rq lock 后按固定主序执行：

```text
PreparePrev(prev) -> PrevDisposition
PickNextTask(prev, disposition) -> next
if next != prev: SwitchTo(prev, next)
else: emit sender Flow.Continue
```

Schedule 入口不得预先或无条件把 prev 重新入队。`PreparePrev` 对 running/runnable 或 preempt
prev 返回 Runnable；对 sleeping prev，若存在匹配 pending wake signal，一次性消费并恢复
running；否则在 pick 之前 deactivate，从 class queue 移除并返回 Blocked。PreparePrev 不保存
context，不改 Task lifecycle。

Pick 按 stop→DL→RT→fair→idle 扫描。类实现可使用组合 `pick_next_task` callback，或使用
`pick_task -> prev_class.put_prev_task(prev,next) -> next_class.set_next_task(next)` fallback。put/set 是 pick
内部类协议，不得出现在 PreparePrev 之前。Blocked/on-rq=false prev 在任何 put callback 中都有
硬门禁，不得重入队；Runnable prev 由所属类决定保留/重放可选结构。identity 选择可执行
Linux callback 所需的 put/set 记账，但不得伪造 context switch。

## SwitchTo 与 continuation

非 identity 路径在任何写操作前完整预检双方引用、lifecycle/authority、context/stack、
FlowRef 与后续一次性信号容量，并保存 `NextDispatchKind`、稳定 TaskRef/TaskFlowRef/generation，然后
严格按以下顺序实现：

1. `prev.SaveCoreContext`；
2. `prev.Suspend`，仅使 OnCpu→Suspended/Live→None/Invalid→Valid，保持 PreparePrev 的资格决定；
3. `next.RestoreCoreContext`，切换架构寄存器/stack 并提交 CPU-local CurrentTask/CurrentStack；
4. 在 next stack 上完成 finish-task-switch 清理；
5. Online next 同步 `Task.Activate`，Suspended next 同步 `Task.Continue`；
6. Scheduler 直接向预检 initial/active Flow 一次性投递 Startup/Continue。

Task 只提交自身 state/authority/breakpoint，不转发 Flow Signal。Online 只接受 Activate，Suspended 只
接受 Continue。identity path 不保存/恢复 context，
不改 Task lifecycle/authority/breakpoint/CurrentTask/CurrentStack，也不发 `Task.Continue`；Scheduler 处理完成后
只向原 sender Flow 投递 Continue，表示 Linux `schedule()` 返回。

fairness、bandwidth、migration 和更细类内算法保持 Deferred；参考配置未启用的 SCX 保持 Trimmed。
