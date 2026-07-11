# 中断期阶段

负责内核从中断启用到多任务启动之前的启动阶段。

> [model] MUST：中断期阶段正式命名是InterruptPhase。

涵盖 中断启用、中断开放、进程子系统初始化和调度准备4个阶段。

## 边界与交互

1. 入口边界：引导期移交控制权，本阶段入口启用中断。
2. 出口边界：把执行权移交给单核多任务期阶段。

## 生命周期

### 范式

符合阶段范式，初始状态Base，启动事件Preset，自动推进状态迁移，直到Online。

> [model] MUST：阶段启动事件对应Preset迁移事件。

### 状态与迁移

* Base：中断期开始。

* Preset：内核开启中断并完成涉及中断的其它处理。

  > [model] MUST：驱动LocalIrqEnablePhase.Preset并等待其到达Online状态，驱动IrqOpenPreparePhase 并等待其到达Online状态。

* Setup：内核初始化进程子系统。

  > [model] MUST: 驱动ProcessPreparePhase.Preset并等待其到达Online状态。

* Enable：内核创建多任务准备启动。

  > [model] MUST: 驱动BootInitRestInitPhase.Preset并等待其到达Online状态。Online：中断期完成。


## 引用

* [charter/phases](../phases)

## 映射目标

* [phases/interrupt](spec/model/phases/interrupt.spec)

