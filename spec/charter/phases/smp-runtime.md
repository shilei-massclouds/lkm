# 多核运行期阶段

仅从属于内核初始化任务的阶段，从启动多核到准备启动应用之前。

> [model] MUST：多核运行期阶段正式命名是SmpRuntimePhase。

涵盖 启动多核、核心初始化、模块初始化、根文件系统初始化、核心收尾共5个阶段。

## 边界与交互

1. 入口边界：多核启动。(可能要改为KernelInitTask启动)
2. 出口边界：把执行权移交给应用交接期阶段。

## 生命周期

### 范式

符合阶段范式，初始状态Base，启动事件Preset，自动推进状态迁移，直到Online。

> [model] MUST：阶段启动事件对应Preset迁移事件。

### 状态与迁移

* Base：多核运行期开始。

* Preset：空。（可能需要把PreSmpInit挪过来）

* Setup：启动多核并等待它们完成。

  > [model] MUST: 驱动SmpBringupPhase.Preset并等待其到达Online状态。

* Enable：内核执行核心初始化、模块初始化、根文件系统初始化和核心收尾共4个阶段。

  > [model] MUST: 依次驱动并等待RuntimeCorePhase、InitcallPhase、RootfsPhase和FinalizePhase阶段完成。

* Online：多核运行期完成。

## 引用

* [charter/phases](../phases)

## 映射目标

* [phases/boot](spec/model/phases/boot.spec)

