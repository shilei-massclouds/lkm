# KernelInitFlow 的 SMP/runtime 叶子阶段

不存在 `SmpRuntimePhase` wrapper。固定 `KernelInitFlow` 在 PID 1 的真实 execution continuation 上直接
编排 `PreSmpInitPhase`、`SmpBringupPhase`、`RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、
`FinalizePhase`、`PayloadPreparePhase` 和 `PayloadHandoffPreparePhase`。

`KernelInitTask` 发布时已为 Online/None/Valid，KernelInitFlow 也已 Online。首次真实 dispatch 与恢复
统一由 Scheduler 恢复 TaskThreadContext、提交 CurrentTask/CurrentStack、drives Task.Dispatch，再向
KernelInitFlow 交付 contextual Enter。首个 context 令 Enter 转到 `kernel_init_entry()` 对应的
`KernelInitFlow.RunKernelInit` 正文坐标；保存
context 则回到相应 continuation。不存在 Activate/Startup 分支。

runtime lowering 必须在 PID 1 vmalloc stack 上执行，不能在 BootTask 栈预执行。叶子完成后
KernelInitFlow 保持 Online；Kernel.Enable 随后提交 Kernel.Online 并作为唯一发送者发出
CommitPayloadHandoff。exec 只替换 Flow-owned UserAppRuntime 内的 ApplicationInstance。

`ApIdleFlow[logical_id]` 与 `ApIdleTask[logical_id]` pointwise 固定。BP 发布 Linux
`{task_ptr, stack_ptr}` boot data并异步发出 keyed HSM entry；AP 验证后建立 tp/sp、激活 Live authority，
再由架构入口直接进入已经 Online 的固定 Flow 的 `RunIdle` 正文坐标。ApEntryPrelude、ApSmpCallin、
ApOnlineIdle 由该 Action 承载。AP 首次架构直入不发送 Dispatch/Enter；首次切出才保存 context 并进入 Online，
以后恢复使用普通 Dispatch/Enter/Suspend。

三段 AP 初始化完成后，`RunIdle` 不进入永久 park loop，而是持续执行通用 per-CPU idle/scheduler loop。
AP 先建立 software-interrupt 接收与 CPU-local scheduler lease，再发布 idle-loop-ready；随后消费 inbound
mailbox、在 owner Scheduler 上运行显式目标 CPU 的普通内核 Task，并以安全 `wfi` 协议等待 reschedule
IPI。AP idle 首次切出保存真实 continuation，之后 idle 与普通内核 Task 都只走通用
Save/Suspend/Restore/Dispatch/Enter 协议。

stopped AP 的 active translation controller 必须 absent。AP 入口按 PhysicalDirect → TrampolineVm →
SwapperVm pointwise 激活，不经过 EarlyVm；CPU-local journal、SATP 和 committed count 不得由全局
Ready/Online 事实代替。

SMP bringup 完成后按 logical ID 冻结 online CpuRef 序列，并为每个 online CPU 开放可容纳全部可投递
Task 的 inbox、用户 Task registry lease、timer clockevent 与用户返回安全点。PID 1 固定 CPU0；普通
fork 依据 PID 公式跨 CPU 发布，vfork/CLONE_VM 固定父核。GlobalArbiter、自动负载选择、运行中
migration、CPU hotplug、内核态立即抢占或 schedule replay 仍不引入，且不为其保留 initial/active Flow
或首次/恢复 dispatch 兼容字段。

## 引用

- [阶段范式](../phase-paradigm.md)
- [TaskFlow](../objects/task-flow.md)
- [smp/runtime leaf model](../../model/phases/smp-runtime/)
