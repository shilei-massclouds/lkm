# BootInitFlow

`BootInitFlow` 是静态 `BootTask.initial_flow` 指向的 TaskFlow 实例。由于 TaskFlow 继承
PhaseObject，它从 `_start` 开始承载并编排启动执行片段，同时拥有独立 FlowRef、lifecycle 与 owner
关系。`BootTask` 从模型入口起已经 OnCpu，并在后续真实调度中通过 Suspend/Continue 与 Online 往返；
它始终是 PID 0 的同一静态 carrier。

## 边界与职责

- OpenSBI 发出 `Kernel.Enable` 后，Kernel 在保持 Ready 的同一个 Enable handler 中严格依次完成
  `Kernel.Action::AcceptEnable`、通过 `BootInitFlow.Action::AssignCpuRef(BootCPURef)` 把
  `ref(CpuGroup.cpus[0])` 绑定到 `BootInitFlow.cpu_ref`，以及
  `PhysicalDirect.Action::ActivateOnCpu(BootCPURef)` 的 `InitialActivation`。只有这三项都已完整提交，
  Kernel 才同步 drives canonical `BootInitFlow.Transition::Preset`；`BootInitFlow.Startup` 只是该 Preset
  的显示名，不产生另一条 transition、action、signal identity 或 lifecycle。接受 Preset 时
  BootInitFlow 必须仍为 Base、parent BootTask 必须为 OnCpu，并在第一个直接 child action 之前记录
  `BootInitFlow.Started` checkpoint；全部入口事实成立后才提交 `BootInitFlow.Prepared`。任一前置失败、
  重复启动或执行权不匹配都使根执行失败，且不得启动或部分提交 BootInitFlow.Preset。
- `BootInitFlow.Preset` 的第一个直接 child 是 BootCPU 的 `InterruptType.Preset`。它先关闭 BootCPU 的
  全部中断分路门控，再清空这些门控上的全部待决中断信号；该 child 不改变或判定中断总门控，也不
  提前建立 handler、fallback 或正式分派框架。
- 建立相对 `gp` 寻址基准后，`BootInitFlow.Preset` 驱动 BootCPU 关闭浮点运算和向量运算能力，再为
  内核映像的BSS段清零，让落到该段的全局变量初值为零。相关执行状态属于 BootCPU；BootInitFlow
  只负责编排，不拥有这些状态。
- 把内核启动时的第一个参数作为BootCPU的hartid记录下来，以备后续使用。
- 初始 task/stack binding 建立后，Preset 单独驱动 BootCPU 的 `TrapType.Preset`，为所属 CPU 建立
  临时保护入口，用于处理初始化过程中意外发生的异常或中断，便于测试和定位缺陷。
- 随后 Preset 依次驱动 `Vm.Preset` 与 `Vm.Setup`。前者准备 `KernelAddrSpace`、`RawDtb`、`FixMap`、
  `TrampolineVm` 和 `EarlyVm`，但不改变当前 CPU；后者使 BootCPU 按 PhysicalDirect → TrampolineVm
  → EarlyVm 切换并提交 `Vm.Ready`。`Vm.Enable` 留给后续 SwapperVm 阶段，本入口前导期不触发。
- `Vm.Setup` 完成后，Preset 驱动 BootCPU 的 `TrapType.Setup`，把异常/中断响应入口从临时保护入口
  重置为正式的 `TrapFlowType` 响应流入口。该动作同时驱动 `ExceptionType.Preset`，后者继续驱动
  page-fault、syscall、breakpoint 与 unexpected 四个异常子类型的 `Preset`；成功后 TrapType 为 Ready，
  ExceptionType 及四个子类型为 Prepared，但中断仍未开放。
- 正式响应入口重置完成后，Preset 才调用
  `CurrentTask.RefreshTaskStack(BootTask, BootTask.stack)`，保持 task/stack binding identity 并原子刷新
  EarlyVm 下的地址表示；TrapType.Setup 不承担这一执行绑定刷新。
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

## 直接叶阶段与物理 namespace

`boot` 只允许作为文件组织 namespace，不表示 `Boot`、`BootPhase`、入口前导 wrapper 或任何其它
拥有 lifecycle 的对象。`BootInitFlow.Preset` 先以单个
`CurrentTask.BindTaskStack(BootTask, BootTask.stack)` 原子建立物理 `tp/sp` 与首次 task/stack binding，
再在 EarlyVm 接管后以单个 `CurrentTask.RefreshTaskStack(BootTask, BootTask.stack)` 保持 identity 并
原子刷新虚拟地址表示；不存在公开 `BindStack` Signal。全部事实成立后提交 Prepared。
`BootInitFlow.Setup` 再按以下顺序直接驱动四个 boot 叶阶段：

1. `EntrySuccessorPhase`；
2. `CorePreparePhase`；
3. `MmCoreInitPhase`；
4. `SchedInitPhase`。

四个叶阶段各自遵循标准四态，直接 parent 均为 `BootInitFlow`。每个叶阶段 Online 后只能返回
`BootInitFlow.Setup` 的 continuation；不得直接启动下一 sibling，也不得提交不存在的 wrapper 状态或
checkpoint。叶阶段的物理目录或 namespace 不改变 parent、执行 owner 或 signal 因果关系。

## 生命周期与执行主体边界

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

`Started` 只是 Preset 被接受时的 checkpoint：它位于 Kernel acceptance、CpuRef binding 和
PhysicalDirect InitialActivation 之后、Preset 第一个直接 child action 之前，不是第五种状态，也不是
Startup 的独立 signal identity。记录 Started 时 BootInitFlow 仍为 Base；只有全部 Preset 事实成立后
才原子提交 Prepared。Prepared、Ready、Online 均使用标准 phase checkpoint；
`BootInitFlow.Online` 必须紧邻真实 PID 1 switch commit，且位于不可逆切换之前。

`BootIdleEntryPhase` 不属于 `BootInitFlow`。它是 `BootIdleFlow` 的 PhaseObject 子对象，只在未来
调度恢复 `BootTask`、真实 current/SP 已回到 PID 0 后才开始，随后进入 `cpu_startup_entry()` 和
idle loop。KernelInitTask 的入口不得预执行、假定或等待 BootIdleEntry 完成。

## 引用

- [阶段范式](../phase-paradigm.md)
- [BootInitFlow model](../../model/phases/boot-init/phase.spec)
- [BootInitFlow coding](../../coding/phases/boot-init.md)
- [Kernel 系统](../systems/kernel.md)
