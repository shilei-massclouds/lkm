# KernelInitFlow 的 SMP/runtime 叶子阶段

不存在 `SmpRuntimePhase` wrapper。固定 `KernelInitFlow` 在 PID 1 的真实 execution continuation 上直接
编排 `PreSmpInitPhase`、`SmpBringupPhase`、`RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、
`FinalizePhase`、`PayloadPreparePhase` 和 `PayloadHandoffPreparePhase`。

`KernelInitTask` 发布时已为 Online/None/Valid，KernelInitFlow 也已 Online。首次真实 dispatch 与恢复
统一由 Scheduler 恢复 TaskThreadContext、提交 CurrentTask/CurrentStack、drives Task.Continue，再向
KernelInitFlow 交付 contextual Continue。首个 context 令 Continue 从 `kernel_init_entry()` 开始；保存
context 则回到相应 continuation。不存在 Activate/Startup 分支。

runtime lowering 必须在 PID 1 vmalloc stack 上执行，不能在 BootTask 栈预执行。叶子完成后
KernelInitFlow 保持 Online；Kernel.Enable 随后提交 Kernel.Online 并作为唯一发送者发出
CommitPayloadHandoff。exec 只替换 Flow-owned UserAppRuntime 内的 ApplicationInstance。

`ApIdleFlow[logical_id]` 与 `ApIdleTask[logical_id]` pointwise 固定。BP 发布 Linux
`{task_ptr, stack_ptr}` boot data并异步发出 keyed HSM entry；AP 验证后建立 tp/sp、激活 Live authority，
再对已经 Online 的固定 Flow 交付 contextual Continue。ApEntryPrelude、ApSmpCallin、ApOnlineIdle 由该
Flow 承载。AP 首次切出才保存 context并进入 Online，以后使用普通 Continue/Suspend。

stopped AP 的 active translation controller 必须 absent。AP 入口按 PhysicalDirect → TrampolineVm →
SwapperVm pointwise 激活，不经过 EarlyVm；CPU-local journal、SATP 和 committed count 不得由全局
Ready/Online 事实代替。

本轮不引入 GlobalArbiter、cross-CPU mailbox、migration 或 schedule replay；这些保持 P2 延期，且不
为其保留 initial/active Flow 或首次/恢复 dispatch 兼容字段。

## 引用

- [阶段范式](../phase-paradigm.md)
- [TaskFlow](../objects/task-flow.md)
- [smp/runtime leaf model](../../model/phases/smp-runtime/)
