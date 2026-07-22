# BootInitFlow 的引导叶子阶段

本目录保留 boot 命名空间，但不再存在 `BootPhase` 包装对象或 lifecycle。引导期的五个阶段按
实际 execution continuation 直接归属于 `BootInitFlow`：

1. `EntryPreludePhase` 由 `BootInitFlow.Preset` 单独驱动；Online 后父 Flow 提交 Prepared。
2. `EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase` 与 `SchedInitPhase` 由
   `BootInitFlow.Setup` 按该顺序驱动。

`EntryPreludePhase` 负责物理/虚拟 `tp` binding 和最早入口事实。其余四个叶子继续覆盖既有 Linux
边界与对象动作。每个叶子都遵循标准四态，且 Online 后只能返回 `BootInitFlow` continuation；
不得调用下一 sibling，也不得提交一个已删除的 wrapper 状态或 checkpoint。

这些叶子的执行 owner 始终是 `BootTask`，对应 DispatchWindow 始终必须指向 BootTask。后续 IRQ/time
和进程准备叶子与它们处在同一个 `BootInitFlow.Setup` continuation 中，直接父链不因目录分组改变。

## 引用

- [阶段范式](../phase-paradigm.md)
- [BootInitFlow](boot-init.md)
- [boot leaf model](../../model/phases/boot/)
