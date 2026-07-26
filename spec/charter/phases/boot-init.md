# BootInitFlow 启动执行阶段

`BootInitFlow` 是静态 `BootTask.initial_flow` 指向的 TaskFlow 实例。由于 TaskFlow 继承
PhaseObject，它从 `_start` 开始承载并编排启动执行片段，同时拥有独立 FlowRef、lifecycle 与 owner
关系。`BootTask` 从模型入口起已经 OnCpu，并在后续真实调度中通过 Suspend/Continue 与 Online 往返；
它始终是 PID 0 的同一静态 carrier。

## 边界与职责

- OpenSBI 发出 `Kernel.Enable` 后，Kernel 在自己的 Enable 响应中同步驱动严格
  `BootInitFlow.Startup`（canonical `Preset`）；BootInitFlow 必须仍为 Base、Kernel 必须保持 Ready，
  且 parent BootTask 必须为 OnCpu。接受并记录 Started 后直接
  编排入口对象；全部入口事实成立后提交 `BootInitFlow.Prepared`。重复启动或执行权不匹配使根执行失败。
- Setup 直接顺序驱动 `EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、
  `SchedInitPhase`、`IrqTimeInitPhase`、`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、
  `ProcessPreparePhase` 和 `BootInitRestInitPhase`。最后一个叶子 Online 后提交
  `BootInitFlow.Ready`；不建立 `BootPhase` 或 `InterruptPhase` 包装 lifecycle。
- `BootInitRestInitPhase` 完整驱动 `KernelInitTask` 与 `KthreaddTask` 的 Preset/Setup/Enable。
  Task Enable 对应 `wake_up_new_task()` 并只发布 Online；initial Flow 必须等 Task 首次真实获得 CPU、
  接受 Scheduler 发出的 Continue 并提交 OnCpu 后严格启动。
- Enable 只驱动 `BootInitScheduleHandoffPhase`，由该叶子建立首次调度的可逆预检与
  `BootIdleFlow` owner/active binding。
- 不可逆切换前必须完整建立 `BootIdleFlow` 的 owner/active binding 并使其到达 Ready；随后提交
  `BootInitFlow.Online` 与 Prepared switch result，再由 Kernel.Enable 驱动首次 Scheduler 调度并执行
  真实 BootTask→KernelInitTask task-stack switch。此时 Kernel 仍为 Ready。

BootInitFlow 的每个 lifecycle transition 都在执行时重新检查 parent BootTask 必须为 OnCpu；不存在
另一个 dispatch guard 字段、状态或镜像对象。

Setup/Enable 的全部 boot execution 叶子都直接以 `BootInitFlow` 为 parent。叶子 Online 后只返回
`BootInitFlow` 当前 transition 的 continuation，不直接启动 sibling。

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
