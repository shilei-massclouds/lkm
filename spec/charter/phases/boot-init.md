# BootInitFlow 启动执行阶段

`BootInitFlow` 是静态 `BootTask` 的 PhaseObject 子对象，从 `_start` 开始承载启动执行片段。它不是
TaskFlow，不建立 FlowRef 或 active Flow binding。`BootTask` 在其整个生命周期中始终是 Online、
PID 0 的同一静态 carrier。

## 边界与职责

- Preset 在 `Kernel.Started -> BootTask.Online -> BootInitFlow.Started` 之后驱动
  `EntryPreludePhase`；完成后提交 `BootInitFlow.Prepared`。
- Setup 顺序驱动 `BootPhase` 和 `InterruptPhase`；二者 Online 后提交 `BootInitFlow.Ready`。
- Enable 顺序驱动 `BootInitRestInitPhase` 和 `BootInitScheduleHandoffPhase`。前者创建并唤醒
  KernelInitTask/KthreaddTask，后者完成首次调度的可逆预检与 dispatch 事实。
- 不可逆切换前必须完整建立 `BootIdleFlow` 的 owner/active binding 并使其到达 Ready；随后提交
  `BootInitFlow.Online` 与 Kernel 的 Prepared switch result，再执行真实 BootTask→KernelInitTask
  task-stack switch。

`EntryPreludePhase`、`BootPhase`、`InterruptPhase`、`BootInitRestInitPhase` 和
`BootInitScheduleHandoffPhase` 都以 `BootInitFlow` 为 parent。叶子 Online 后只返回
`BootInitFlow` continuation，不直接启动 sibling。

## 生命周期与执行主体边界

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

`Started` 是 Preset 被接受时的 checkpoint。Prepared、Ready、Online 均使用标准 phase
checkpoint；`BootInitFlow.Online` 必须紧邻真实 PID 1 switch commit，且位于不可逆切换之前。

`BootIdleEntryPhase` 不属于 `BootInitFlow`。它是 `BootIdleFlow` 的 PhaseObject 子对象，只在未来
调度恢复 `BootTask`、真实 current/SP 已回到 PID 0 后才开始，随后进入 `cpu_startup_entry()` 和
idle loop。KernelInitTask 的入口不得预执行、假定或等待 BootIdleEntry 完成。

## 引用

- [阶段范式](../phase-paradigm.md)
- [BootInitFlow model](../../model/phases/boot-init/phase.spec)
- [BootInitFlow coding](../../coding/phases/boot-init.md)
- [Kernel 系统](../systems/kernel.md)
