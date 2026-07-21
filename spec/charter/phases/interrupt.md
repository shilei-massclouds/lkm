# InterruptPhase 中断期阶段

InterruptPhase 是 Kernel 的第三个直接子阶段，负责在 BootPhase 已完成且 boot CPU 仍处于
单任务启动上下文时，建立 IRQ/time 基础、打开 boot CPU 本地中断总入口，并完成进入
`rest_init()` 前的 late core 与进程基础准备。

## 边界与职责

- 入口：`Kernel.Setup` 在 `BootPhase.Online` 后驱动 `InterruptPhase.Preset`。
- 出口：InterruptPhase 到达 Online 后返回 `BootInitFlow.Setup` continuation；由 BootInitFlow 决定
  何时进入 Enable。
- 本阶段只拥有下列四个直接子阶段，不拥有 `BootInitRestInitPhase` 或其它启动叶子阶段。

四个子阶段按固定顺序执行：

1. `IrqTimeInitPhase`：在 `SingleTaskContext` 内建立 IRQ、timer、timekeeping、random、IPI 和
   call-function 基础，保持 boot CPU 本地中断关闭。
2. `LocalIrqEnablePhase`：清除 early IRQ flag 并打开 boot CPU 的 `sstatus.SIE`；该迁移自身改变
   中断上下文，因此不使用覆盖整个迁移的外层 context。
3. `IrqOpenPreparePhase`：在 `SingleTaskInterruptStreamContext` 内完成中断开放后的 late core
   准备。
4. `ProcessPreparePhase`：在同一 context 内准备 PID、task、cred、VMA、namespace、key、
   security 和初始 ramfs rootfs，但不创建 PID 1 或 kthreadd。

子阶段之间不得直接调用 sibling。每个子阶段 Online 后返回 InterruptPhase 持有的 continuation，
由父阶段检查完成事实并启动下一项。

## 生命周期

InterruptPhase 与四个直接子阶段都遵循标准阶段生命周期：

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

`Started` 是 transition 被接受时的事件观察点，不是额外状态。四个子阶段的 Linux 初始化对象动作
都归属于各自 Preset；Setup 和 Enable 只检查并逐层发布已经建立的事实，不重复执行对象动作。

InterruptPhase.Preset 依次驱动四个子阶段到 Online 后提交 Prepared；InterruptPhase.Setup 检查
完整子阶段 Online 集合后提交 Ready；InterruptPhase.Enable 提交 Online 并返回
`BootInitFlow.Setup` continuation。

## 引用

- [阶段范式](../phase-paradigm.md)
- [Interrupt model](../../model/phases/interrupt/phase.spec)
- [Interrupt coding](../../coding/phases/interrupt.md)
- [Kernel 系统](../systems/kernel.md)
