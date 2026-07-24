# KernelInitFlow 的 SMP/runtime 叶子阶段

本目录保留 smp-runtime 命名空间，但不再存在 `SmpRuntimePhase` 包装对象或 lifecycle。
`KernelInitFlow` 在 PID 1 的真实 execution continuation 上直接编排六个 BP 叶子：

- Preset 依次驱动 `PreSmpInitPhase`、`SmpBringupPhase`；两者 Online 后提交
  `KernelInitFlow.Prepared`。
- Setup 依次驱动 `RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、`FinalizePhase`，随后进入
  `PayloadPreparePhase`；后者 Online 后提交 `KernelInitFlow.Ready`。

首次真实 dispatch 先同步驱动 BootTask.Suspend；完成栈切换后，`KernelInitTask.Continue` 在 PID 1
真实获得 CPU 的入口提交 OnCpu 并严格启动 `KernelInitFlow.Preset`。runtime lowering 必须在
`kernel_init_entry()` 验证 PID 1 vmalloc stack 后执行 Preset body 和所有
叶子代码；不得在 BootTask 栈上预执行。

SmpBringup 的 BP 协调属于 `KernelInitFlow` continuation。`ApEntryPreludePhase`、
`ApSmpCallinPhase`、`ApOnlineIdlePhase` 由按 logical-id replicated 的 `ApIdleFlow` pointwise 驱动，
不属于 KernelInitFlow 的 execution ownership。每个 `ApIdleTask` 在 HSM 前已经是
`OnCpu/Reserved/Invalid`，其 Flow 为 Base；BP 只发布 Linux `{task_ptr, stack_ptr}` boot data 并异步
发出 keyed HSM Startup。AP 架构入口验证 boot data、建立 `tp/sp`、激活 Live authority 后启动同 key
Flow；BP 仅通过 cpu_running/done_up wait/barrier 观察完成。

## 引用

- [阶段范式](../phase-paradigm.md)
- [TaskFlow](../objects/task-flow.md)
- [smp/runtime leaf model](../../model/phases/smp-runtime/)
