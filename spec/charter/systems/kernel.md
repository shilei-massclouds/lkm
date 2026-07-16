# 内核系统

内核系统（简称内核）指代计算机中的一次内核运行实例，它负责管理物理资源并在此之上建立应用运行环境。

> [model] MUST：内核正式命名是Kernel。

内核采用被动响应模式，在没有信号触发时不自行改变状态，也不自发向外发送信号。内核收到信号后
执行相应的迁移或动作，并可以在响应过程中继续同步或异步发送信号；响应完毕后重回静默状态。
由于外部信号可能并发到来，内核对不同信号的响应过程可能重叠，所以内核需要建立同步/互斥机制
进行必要的内部协调与状态保护。

被动响应和信号触发是内核以及各级子系统的统一模式，主要是降低整个计算机系统的工作能耗，并用
一套因果关系简化系统建模。任务、CPU flow、调度和应用执行也是启动信号或运行时信号引起的后续
响应链，不再引入主动模式作为另一种推进机制。

## 边界与交互

内核有三条边界：面向OpenSBI的启动边界，面向中断源的中断边界和面向硬件平台的硬件边界。

内核是被动系统，只直接响应两种以 `Kernel` 为目标的外部信号：OpenSBI发出的启动信号，中断源
发出的中断信号。

<img src="../pic/内核边界.svg" alt="内核边界" style="zoom:50%;" />

1. 启动边界：OpenSBI在引导末尾向内核发出启动信号，交接系统控制权，内核启动。启动信号引起内核内部的一系列连锁信号，推动内核的生命周期状态前进，并且触发首个应用启动。
2. 中断边界：中断控制器代表各中断源向内核发出中断信号，内核响应中断。
3. 硬件边界：硬件寄存器/MMIO界面。内核**不主动**访问硬件，内核对硬件的访问来源于启动信号的余波或者来源于中断信号。
4. 应用边界：应用是内核的组成部分，因此应用边界**不是**内核的边界，它只是内核内部的子系统边界。

异常、syscall、调度、任务切换等信号的目标是内核内部子系统，属于 `Kernel` 内部信号。内部信号
只触发其明确的目标子系统，不沿系统层级自动向上冒泡，因此不能直接触发 `Kernel` 自身的迁移或
动作。上图只描述 `Kernel` 层面的外部信号，没有展开这些内部信号和子系统边界。

## 功能构成

内核的核心功能是建立和维护应用的运行环境，支持运行环境的基础是`任务子系统`。

内核直接接收的唯一运行时信号是中断信号，代表内核处理中断的边界是`中断子系统`。

`任务子系统`和`中断子系统`目前是 charter 层的候选系统边界，model 映射暂时 deferred。后续需要
先确定它们是新的聚合系统对象，还是由现有 Task、Scheduler、InterruptPhase、EventStream、
InterruptStream 和 IRQ 对象改造形成；在决定前，不把任一现有 phase 或 stream 直接等同于这两个
子系统。

## 生命周期

### 范式

符合[阶段范式](../phase-paradigm.md)。内核启动信号触发 `Kernel.Transition::Preset` 响应过程。

> [model] MUST：以 `Kernel` 为目标的启动信号触发 `Kernel.Transition::Preset`；`Signal` 与
> `Transition` 是不同概念。

### 状态与迁移

* Base：内核映像驻留在内存/闪存中，尚未启动，等待启动信号。

  > [model] MUST：确保 Riscv64 规范、SBI 规范、OpenSBI、Lds 和 Config 都处于 `Online` 状态。

* Preset：内核通过入口前导期过程。

  > [model] MUST：在 `SingleTaskContext` 中向 `EntryPreludePhase` 同步发送 Preset 启动信号，
  > 等待 `EntryPreludePhase` 到达 `Online`；当前 model 兼容写法为驱动
  > `EntryPreludePhase.Transition::Preset`。

* Prepared：入口前导期已经完成；BootPhase 尚未启动，中断尚未开启。

* Setup：内核按顺序通过引导期和中断期过程。

  > [model] MUST：先向 `BootPhase` 同步发送 Preset 启动信号并等待其到达 `Online`，再向
  > `InterruptPhase` 同步发送 Preset 启动信号并等待其到达 `Online`；当前 model 兼容写法为
  > 按此顺序驱动两个阶段的 `Transition::Preset`。

* Ready：入口前导期、引导期和中断期均已完成，中断已经开启，内核具备了支持多任务的能力，
  但是还没有真正启动多任务。

* Enable：内核通过单核多任务期、多核运行期和应用交接期阶段。

  > [model] MUST：依次向 `UpMultitaskPhase`、`SmpRuntimePhase` 和 `PayloadPhase` 同步发送 Preset
  > 启动信号并等待各阶段完成；当前 model 兼容写法为依次驱动各阶段的 `Transition::Preset`。

* Online：内核处于正常服务状态，支持应用运行。

## 引用

* [系统与信号模型](../system-signal.md)
* [概念体系迁移](../concept-migration.md)
* [阶段范式](../phase-paradigm.md)
* [charter/phases](../phases)

## 映射目标

* [model/kernel](../../model/systems/kernel.spec)
