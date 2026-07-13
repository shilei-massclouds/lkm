# 引导期阶段

负责内核从 OpenSBI 主 hart 入口到中断期开始之前的启动编排。

> [model] MUST：引导期阶段正式命名是 BootPhase。

BootPhase 固定包含五个直接子阶段：入口先导、入口后继、核心准备、内存核心初始化和调度初始化。
时间与中断控制初始化属于后续 InterruptPhase，不是 BootPhase 子阶段。

## 边界与交互

1. 入口边界：OpenSBI 把主 hart 控制权交给 `_start`。同一条架构入口依次接受
   `Kernel.Preset`、`BootPhase.Preset` 和 `EntryPreludePhase.Preset`；这三个层级仍各自拥有
   独立状态和 checkpoint，不能折叠为一个阶段。
2. 出口边界：SchedInitPhase 达到 Online 后返回 BootPhase.Enable continuation；BootPhase
   提交 Online，再返回 Kernel.Preset continuation。BootPhase 不直接启动 InterruptPhase。

## 生命周期

### 范式

符合[阶段范式](../phase-paradigm.md)，初始状态为 Base，启动事件为 Preset，按
`Preset -> Setup -> Enable` 自动推进到 Online。

> [model] MUST：阶段启动事件对应 Preset 迁移事件。

### 状态与迁移

* Base：BootPhase 已进入模型但尚未接受 Preset。`BootPhase.Started` 只观察 Preset 接受时点，
  发出时状态仍为 Base。

* Preset：驱动 EntryPreludePhase.Preset，并等待 EntryPreludePhase 到达 Online。

  > [model] MUST：驱动 EntryPreludePhase.Preset并等待其到达 Online 状态。

  EntryPreludePhase.Online 必须返回 BootPhase.Preset continuation；该 continuation 提交
  BootPhase.Prepared 并触发 BootPhase.Setup，EntryPreludePhase 不直接启动 sibling。

* Prepared：入口先导完成，BootPhase.Preset 已提交。

* Setup：驱动 EntrySuccessorPhase.Preset，并等待 EntrySuccessorPhase 到达 Online。

  > [model] MUST：驱动 EntrySuccessorPhase.Preset并等待其到达 Online 状态。

  EntrySuccessorPhase.Online 必须返回 BootPhase.Setup continuation；该 continuation 提交
  BootPhase.Ready 并触发 BootPhase.Enable。

* Ready：入口后继完成，BootPhase.Setup 已提交。

* Enable：依次驱动 CorePreparePhase、MmCoreInitPhase 和 SchedInitPhase 的 Preset，并等待每个
  子阶段到达 Online。每个子阶段完成后都返回 BootPhase.Enable 的下一 continuation，不建立
  child-to-sibling 调用边。

  > [model] MUST：依次驱动并等待 CorePreparePhase、MmCoreInitPhase 和 SchedInitPhase 完成。

* Online：五个直接子阶段均为 Online；BootPhase.Enable 已提交，并返回 Kernel.Preset
  continuation。

## 引用

* [阶段范式](../phase-paradigm.md)
* [charter/phases](../phases)

## 映射目标

* [BootPhase model](../../model/phases/boot/phase.spec)
