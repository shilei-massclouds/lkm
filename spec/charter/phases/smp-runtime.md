# KernelInitFlow 的 SMP/runtime 叶子阶段

本目录保留 smp-runtime 命名空间，但不再存在 `SmpRuntimePhase` 包装对象或 lifecycle。
`KernelInitFlow` 在 PID 1 的真实 execution continuation 上直接编排六个 BP 叶子：

- Preset 依次驱动 `PreSmpInitPhase`、`SmpBringupPhase`；两者 Online 后提交
  `KernelInitFlow.Prepared`。
- Setup 依次驱动 `RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、`FinalizePhase`，随后进入
  `PayloadPreparePhase`；后者 Online 后提交 `KernelInitFlow.Ready`。
- Enable 驱动 `PayloadHandoffPreparePhase` 并提交 `KernelInitFlow.Online`。Preset/Setup/Enable 全部
  运行在 Kernel Ready 的 Enable 执行上下文中，不错误要求 Kernel 已 Online。

首次真实 dispatch 先同步驱动 BootTask.Suspend；完成栈切换后，`KernelInitTask.Continue` 在 PID 1
真实获得 CPU 的入口提交 OnCpu 并严格启动 `KernelInitFlow.Preset`。runtime lowering 必须在
`kernel_init_entry()` 验证 PID 1 vmalloc stack 后执行 Preset body 和所有
叶子代码；不得在 BootTask 栈上预执行。

KernelInitFlow.Online 只把应用环境准备结果返回 Kernel.Enable，不自行发送 payload handoff。Kernel
随后提交 Online 并作为唯一发送者异步发送 `CommitPayloadHandoff`。

SmpBringup 的 BP 协调属于 `KernelInitFlow` continuation。`ApEntryPreludePhase`、
`ApSmpCallinPhase`、`ApOnlineIdlePhase` 由按 logical-id replicated 的 `ApIdleFlow` pointwise 驱动，
不属于 KernelInitFlow 的 execution ownership。每个 `ApIdleTask` 在 HSM 前已经是
`OnCpu/Reserved/Invalid`，其 Flow 为 Base；BP 只发布 Linux `{task_ptr, stack_ptr}` boot data 并异步
发出 keyed HSM Startup。AP 架构入口验证 boot data、建立 `tp/sp`、激活 Live authority 后启动同 key
Flow；BP 仅通过 cpu_running/done_up wait/barrier 观察完成。

stopped AP 的 `active_translation_controller` association 必须 absent，并且没有 live SATP；BP 发布的
boot data 只保存 AP 入口预期 SATP，不代表 AP 已执行 CSR 写入或激活任何 controller。真实
`ApEntryPreludePhase` 架构入口接受该 CPU 时，必须原子提交 PhysicalDirect 的
`ActivateOnCpu(cpu_ref, InitialActivation)`，随后以两个 Handoff 依次激活 TrampolineVm 与
SwapperVm。AP 不经过 EarlyVm；每个 AP 的 association、live SATP、同步事实、journal 和 committed
count 与 BP 及其它 AP 隔离。`CpuGroup`、`KernelAddrSpace`、`Vm` 或任一 controller 的全局
Ready/Online 不能代替这些 CPU-local activation 事实。

## 引用

- [阶段范式](../phase-paradigm.md)
- [TaskFlow](../objects/task-flow.md)
- [smp/runtime leaf model](../../model/phases/smp-runtime/)
