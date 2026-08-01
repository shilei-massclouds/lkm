# BootInitFlow

`BootInitFlow` 是静态 `BootTask.flow` 指向的终身 TaskFlow。它从 `_start` 编排 boot execution，提交
Online 后仍承载同一 PID 0 的 idle setup、首次 Schedule 返回、`BootIdleEntryPhase` 和 idle loop。
不存在第二个 boot idle Flow；`BootTask` 首次真实切出时才保存 context并进入 Online，未来通过统一
Continue 返回同一个 BootInitFlow continuation。

## 入口与 Preset

OpenSBI 发出 `Kernel.Enable` 后，Kernel 在 Ready 的同一 handler 中依次完成：

1. `Kernel.Action::AcceptEnable`；
2. `BootInitFlow.Action::AssignCpuRef(BootCPURef)`；
3. `PhysicalDirect.Action::ActivateOnCpu(BootCPURef)` 的 InitialActivation；
4. canonical `BootInitFlow.Transition::Preset`。

Preset 接受时 Flow 必须 Base、parent 必须是 OnCpu/Live BootTask；`Started` 只是在第一个 child 前记录的
checkpoint。入口依次完成 BootCPU interrupt route mask/pending clear、浮点/向量关闭、BSS 清零、hartid
记录、PhysicalDirect 下 boot-only `CurrentTask.BindTaskStack(BootTask, BootTask.stack)`、临时 trap、
Vm.Preset/Setup、正式 TrapType.Setup、EarlyVm 下 `RefreshTaskStack`、Soc.Preset，最后提交 Prepared。
失败不得部分提交 Flow、CurrentTask/CurrentStack 或 translation controller。

## Setup 与发布

Setup 顺序驱动 `EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、
`IrqTimeInitPhase`、`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase` 和
`BootInitRestInitPhase`，随后提交 Ready。所有叶子 parent 均直接是 BootInitFlow；物理目录中的 `boot`
只是 namespace。

`BootInitRestInitPhase` 完整创建、发布 `KernelInitTask`/`KernelInitFlow` 和
`KthreaddTask`/`KthreaddFlow`。Task 发布时已是 Online/None/Valid，固定 Flow 也已 Online；首次派发不再
发送 Startup 或 Activate，而是恢复其首个 TaskThreadContext、提交 Task.Continue，再交付 contextual
TaskFlow.Continue。

Enable 驱动 `BootInitScheduleHandoffPhase`，只完成 CPU0 Scheduler idle/curr metadata、runqueue 与首次
调度的可逆预检；不创建 Flow、active binding 或 dispatch kind。随后提交 BootInitFlow.Online。

## Online Actions、首次调度与 idle

BootInitFlow.Online 后按固定顺序执行同一 Flow 的 Actions：

1. 完成 boot idle setup 和退出 inherited preempt-disabled guard；
2. `yields CpuGroup.cpus[0].scheduler.Action::Schedule()`；
3. identity 时由目标完成后的通用 resume attempt 立即从 yields 后继续；
4. non-identity 时 Scheduler 显式保存 BootTask context、Task.Suspend、恢复 next context、提交 bindings 和
   next Task.Continue；BootInitFlow lane token 保持 pending；
5. 未来 Scheduler 恢复 BootTask 后，contextual BootInitFlow.Continue 校验 context epoch/token，先回到
   `schedule()` 返回 continuation，再驱动 `BootIdleEntryPhase` 和 idle loop。

本轮 canonical before-send 边界固定在第 2 步 token/Signal 尚未创建的位置。发送 Schedule 本身不改变
BootTask 或 Flow；只有 non-identity SwitchTo 的显式步骤改变 Task/CPU binding。`BootIdleEntryPhase`
现在是 BootInitFlow 的 Online child，而不是另一个 Flow 的子对象。

陷入期间 BootTask 保持 OnCpu、BootInitFlow 保持 Online；effective-flow 栈切到 Trap/Interrupt/
Exception leaf。若陷入中调度切出，恢复先落到该 leaf，再回到 BootInitFlow continuation。

## 生命周期与边界

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

每个 transition/action 都重新校验固定 parent、FlowRef/generation、CpuRef 与 effective-flow guard。
BootInitFlow 不退出，终身属于 BootTask。完整 SMP arbitration、迁移与 replay 保持 P2 延期。

## 引用

- [阶段范式](../phase-paradigm.md)
- [BootInitFlow model](../../model/phases/boot-init/phase.spec)
- [BootInitFlow coding](../../coding/phases/boot-init.md)
- [Kernel 系统](../systems/kernel.md)
