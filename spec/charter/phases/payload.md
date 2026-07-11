# 应用交接期阶段

内核初始化任务从准备应用启动环境到切换到应用的阶段。

> [model] MUST：应用交接期阶段正式命名是PayloadPhase。

涵盖 PayloadExecSyncBoundaries、UserCloneDeferredBoundaries、UserBootPayloadSetup和 UserBootPayloadEnable共4个阶段。

## 边界与交互

1. 入口边界：内核初始化任务开始准备应用环境。
2. 出口边界：应用启动。

## 生命周期

### 范式

符合阶段范式，初始状态Base，启动事件Preset，自动推进状态迁移，直到Online。

> [model] MUST：阶段启动事件对应Preset迁移事件。

### 状态与迁移

* Base：引导期开始。

* Preset：准备首个应用的启动环境。

  > [model] MUST：驱动PayloadExecSyncBoundaries和UserCloneDeferredBoundaries并等待其到达Online状态。

* Setup：以当前任务为模板建立承载应用的任务。

  > [model] MUST: 驱动UserBootPayloadSetup.Preset并等待其到达Online状态。

* Enable：准备切换到应用任务。

  > [model] MUST: 驱动UserBootPayloadEnable并等待其到达Online状态。

* Online：引导期完成。

## 引用

* [charter/phases](../phases)

## 映射目标

* [phases/payl](spec/model/phases/payload.spec)

