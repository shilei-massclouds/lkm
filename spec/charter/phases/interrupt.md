# BootInitFlow 的中断与进程准备叶子阶段

本目录保留 interrupt 命名空间，但不再存在 `InterruptPhase` 包装对象或 lifecycle。
`BootInitFlow.Setup` 在 `SchedInitPhase.Online` 后直接顺序驱动以下四个叶子：

1. `IrqTimeInitPhase`：在 `SingleTaskContext` 内建立 IRQ、timer、timekeeping、random、IPI 和
   call-function 基础，并保持 boot CPU 本地中断关闭。
2. `LocalIrqEnablePhase`：清除 early IRQ flag 并打开 boot CPU 的 `sstatus.SIE`。
3. `IrqOpenPreparePhase`：在允许普通中断进入的单任务上下文内完成中断开放后的 late core 准备。
4. `ProcessPreparePhase`：在同一 context 内准备 PID、task、cred、VMA、namespace、key、security
   和初始 ramfs rootfs，但不创建 PID 1 或 kthreadd。

四个叶子的直接 parent 都是 `BootInitFlow`。每个叶子 Online 后返回父 Flow 的 Setup continuation，
由父 Flow 启动下一 sibling；不存在 child-to-sibling 边。`ProcessPreparePhase.Online` 后父 Flow 继续
驱动 `BootInitRestInitPhase`，后者 Online 才构成 `BootInitFlow.Ready` 的提交条件。

## 引用

- [阶段范式](../phase-paradigm.md)
- [BootInitFlow](../flows/boot_init_flow/README.md)
- [interrupt leaf model](../../model/phases/interrupt/)
