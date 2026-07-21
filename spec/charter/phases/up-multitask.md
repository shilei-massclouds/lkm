# UpMultitaskPhase 单核多任务阶段

UpMultitaskPhase 是 Kernel 的第三个直接子阶段，由唯一的 `BootTask` 执行其启动
continuation 和 idle continuation。它负责准备 `KernelInitTask` 与 `KthreaddTask`、启用多任务
支持，并把 `BootTask` 的 active Flow 从 `RootStream` handoff 到 `BootIdleFlow`；该 handoff
不创建或替换 Task。

## 边界与职责

- 入口：`Kernel.Enable` 在 `InterruptPhase.Online` 后驱动 `UpMultitaskPhase.Preset`。
- 直接子阶段固定为 `BootInitRestInitPhase`、`BootInitScheduleHandoffPhase` 和
  `BootIdleEntryPhase`；不建立 `RestInitPhase` wrapper。
- 出口：`UpMultitaskPhase.Online` 已发布后，BootTask 保存自己的 switch context，恢复
  KernelInitTask 的 vmalloc stack，并进入具名 `kernel::enable_after_up_multitask()`
  continuation。该 continuation 才能启动现有 SmpRuntime 入口。
- secondary CPU、workqueue worker 和后续 runtime 不属于本阶段。

三个子阶段按执行主体划分：

1. `BootInitRestInitPhase` 由运行 `RootStream` 的 BootTask 执行 `rcu_scheduler_starting()`，创建并唤醒
   KernelInitTask/KthreaddTask，发布 `SYSTEM_SCHEDULING` 并完成 `kthreadd_done`。
2. `BootInitScheduleHandoffPhase` 仍由运行 `RootStream` 的 BootTask 执行
   `BootIdlePreemption.EnableNoResched` 和首次 `Scheduler.Schedule`，只提交调度/dispatch 事实。
3. `BootIdleEntryPhase` 由同一 BootTask 的 `BootIdleFlow` continuation 在
   `BootIdleStartupContext` 中进入
   `cpu_startup_entry()` 与一轮代表性 idle loop，然后返回父 Enable continuation。

叶子阶段 Online 后只返回 UpMultitask 持有的 continuation，不直接启动 sibling。

`BootInitRestInitPhase` 是两个普通 Task 创建与启用顺序的唯一角色级编排者。对每个 Task，它按
`Task.Preset -> TaskCreationCore.CopyProcess -> Task.Setup -> pi-lock/runqueue wakeup ->
Task.Enable` 推进类型级 lifecycle；wakeup 必须在同一编排内完成 stack/context setup、runqueue
selection、CPU assignment、enqueue 与初始 Flow binding。Kernel-init 的 CPU pin 与 kthreadd 的
global-ref publication 分别发生在对应 Enable 之后。PID 1、入口、`CLONE_FS`、kthreadd clone flags、
provider 和 schedule-loop 等角色事实由该 Phase 的 ensures/invariants 提供，不下沉为
`KernelInitTask` 或 `KthreaddTask` 的实例级 lifecycle。

## 生命周期

UpMultitaskPhase 与三个直接子阶段均遵循标准阶段生命周期：

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

`Started` 是 Preset 被接受时的事件 checkpoint，不是额外状态。每个叶子的 Linux 对象动作
归入自身 Preset；Setup 和 Enable 只检查 Preset 已建立的事实并逐层发布状态。

父阶段用三次迁移分别拥有三个子阶段：

- UpMultitask.Preset 驱动 BootInitRestInit.Preset；其 Online 后发布 UpMultitask.Prepared。
- UpMultitask.Setup 驱动 BootInitScheduleHandoff.Preset；其 Online 后发布 UpMultitask.Ready。
- UpMultitask.Enable 驱动 BootIdleEntry.Preset；其 Online 后发布 UpMultitask.Online。

`KernelInitTask` 的 `CurrentTaskRef` 标签只是线性调度事实；只有上述真实 task stack handoff 和
KernelInit 入口的实际 SP 验证构成物理跨栈边界。

`BootTask` 始终是静态 `init_task`/PID 0/swapper 的唯一 Task carrier。`RootStream` 与
`BootIdleFlow` 分别保存 handoff 前后的 Flow lifecycle；Task 的调度身份、CPU 归属、preemption
状态和 `TaskThreadContext` 不随 Flow handoff 重建。

## 引用

- [阶段范式](../phase-paradigm.md)
- [UpMultitask model](../../model/phases/up-multitask/phase.spec)
- [UpMultitask coding](../../coding/phases/up-multitask.md)
- [Kernel 系统](../systems/kernel.md)
