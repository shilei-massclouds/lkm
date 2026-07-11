# 引导期阶段

负责内核从引导入口到中断启用之前的启动阶段。

> [model] MUST：引导期阶段正式命名是BootPhase。

涵盖 入口先导、入口后继、内核准备、内存子系统初始化、调度子系统初始化、时间子系统初始化共6个阶段。

## 边界与交互

1. 入口边界：内核引导入口，承接OpenSBI移交的执行控制权。
2. 出口边界：把执行权移交给中断期阶段。

## 生命周期

### 范式

符合阶段范式，初始状态Base，启动事件Preset，自动推进状态迁移，直到Online。

> [model] MUST：阶段启动事件对应Preset迁移事件。

### 状态与迁移

* Base：引导期开始。

* Preset：内核通过最初的入口先导期启动过程。

  > [model] MUST：驱动EntryPreludePhase.Preset并等待其到达Online状态。

* Setup：内核通过入口后继期启动过程。

  > [model] MUST: 驱动EntrySuccessorPhase.Preset并等待其到达Online状态。

* Enable：依次通过内核准备、内存子系统初始化、调度子系统初始化、时间子系统初始化阶段。

  > [model] MUST: 依次驱动并等待CorePreparePhase、MmCoreInitPhase、SchedInitPhase和TimeInitPhase4个子阶段完成。

* Online：引导期完成。

## 引用

* [charter/phases](../phases)

## 映射目标

* [phases/boot](spec/model/phases/boot.spec)

