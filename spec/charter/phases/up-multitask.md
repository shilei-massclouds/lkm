# 单核多任务阶段

引导任务身份转化为空闲任务，同时启动两个新任务：内核初始化任务和内核线程守护任务。

内核初始化任务继续承担内核初始化工作，启动多核之前是本阶段的出口边界。

> [model] MUST：单核多任务阶段正式命名是UpMultitaskPhase。

引导任务在本阶段中完成 调度交接、转换空闲任务2个子阶段。

> [model] MUST：引导任务命名BootInitTask，转换身份为空闲任务后，命名为BootIdleTask。

内核初始化任务在本阶段中为多核运行做准备。

> [model] MUST：内核初始化任务命名KernelInitTask。

内核线程守护任务 *暂缺*。

> [model] MUST：内核线程守护任务命名KThreaddTask。

> 待补充。 待补充。 

## 边界与交互

1. 入口边界：中断期阶段把执行权移交当前阶段。
2. 出口边界：把执行权移交给多核运行期阶段，由多核运行期负责启动多核。

## 生命周期

### 范式

符合阶段范式，初始状态Base，启动事件Preset，自动推进状态迁移，直到Online。

> [model] MUST：阶段启动事件对应Preset迁移事件。

### 状态与迁移

#### 引导任务

* Preset：空。

* Setup：内核引导任务进行一次调度，给其它任务让出运行机会。

  > [model] MUST: 驱动SchedHandoffPhase.Preset并等待其到达Online状态。

* Enable：内核引导任务转换身份成为空闲任务。

  > [model] MUST: 驱动BootIdleEntryPhase.Preset并等待其到达Online状态。

#### 内核初始化任务

* Preset：空。

  > [model] MUST：驱动PreSmpInitPhase等待其到达Online状态。

#### 内核线程守护任务

* 暂缺。

## 引用

* [charter/phases](../phases)

## 映射目标

* [phases/boot](spec/model/phases/boot.spec)

