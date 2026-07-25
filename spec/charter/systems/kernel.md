# 内核系统

> [model] MUST：内核系统模型正式命名是Kernel。

`Kernel` 是系统树根 `Computer` 的直接子系统。它同时承担 Kernel 规格采纳、静态配置发布、kernel
image 构造和运行入口交接；OpenSBI 通过 canonical `Kernel.Enable` 把控制权交给已经 Ready 的 Kernel
实例。

## 系统功能

内核系统（简称内核）的核心功能是运行用户应用，具体包括三层：

1. 基础：对硬件平台资源进行统一管理和抽象；
2. 机制：以任务为核心调度单元和资源分配单元；
3. 目的：为用户应用提供运行环境。

内核是被动响应模式，在没有信号触发时不自行改变状态，也不自发向外发送信号。内核收到信号后
执行相应的迁移或动作，并可以在响应过程中继续同步或异步发送信号；响应完毕后重回静默状态。
由于外部信号可能并发到来，内核对不同信号的响应过程可能重叠，所以内核需要建立同步/互斥机制
进行必要的内部协调与状态保护。

> GAP: 外部信号的并发交付、响应重叠、排序和失败语义尚未在 formal semantics 与当前 model 中闭合。

被动响应和信号触发是内核以及各级子系统的统一模式，主要是降低整个计算机系统的工作能耗，并用
一套因果关系简化系统建模。任务、CPU flow、调度和应用执行也是启动信号或运行时信号引起的后续
响应链，不再引入主动模式作为另一种推进机制。

## 系统边界

内核有两条边界：

1. 引导边界：内核的引导入口。
2. 中断边界：中断信号入口。

> GAP: 当前只列出两个输入边界，尚未覆盖下文硬件访问信号和 SBI-call 穿越的输出边界。

<img src="../pic/内核边界.svg" alt="内核边界" style="zoom:50%;" />

## 关联系统

内核与三个系统协作：

1. OpenSBI：在计算机启动链上，是内核的上一级引导系统，在加载内核映像之后，向内核发出startup信号完成引导；在内核运行过程中，OpenSBI作为服务提供者，接收内核发出的sbi-call信号并响应。相对内核，它既是信号源也是信号目标。
2. 中断控制器Interrupt Controller：中断控制器代表各种中断源，随时可能向内核发出中断信号。中断控制器只作为信号源。
3. 硬件平台：硬件平台是计算机裸机系统，是对CPU、各类设备的集成。硬件平台只作为信号目标。注意：中断控制器也属于硬件平台的构成部分。在配置路径上，内核向中断控制器发出配置信号，中断控制器只是作为硬件平台中的普通设备；在上面讨论的中断路径上，中断控制器向内核发出中断信号。

> GAP: 中断控制器“只作为信号源”与配置路径上作为信号目标的描述冲突，并且它作为硬件平台子系统时的分析层级尚未明确。

注意，应用是内核的内部组成部分，不是与内核平级的关联系统。

## 信号

内核作为信号目标系统可以接收的信号：

1. 引导信号：OpenSBI 向内核发出 Enable，把引导控制权移交到已经构造完成的 Kernel；内核对应处理
   该信号的迁移过程是 `Kernel.Transition::Enable`。
2. 中断信号：如前所述，中断控制器向内核发出中断信号irq，内核对应处理该信号的动作是Kernel.OnIrq。

> GAP: Kernel.Enable 与 Kernel.OnIrq 的 Signal-to-Transition/Action 绑定尚未在正式 Signal DSL 中定义。

内核可能发出的信号：

1. 硬件访问信号：包括针对硬件平台各类设备的一系列信号类型，如针对Cpu寄存器gpr/csr的访问信号，针对各类设备的io/mmio访问信号。
2. Sbi-call服务调用信号：面向OpenSBI服务的调用。

> GAP: 硬件访问信号和 SBI-call 的目标系统、同步方式、附加信息及当前 model 映射尚未定义。

注意：异常、syscall、调度、任务切换等信号的目标是内核内部子系统，所以属于 `Kernel` 内部信号。内部信号
只触发其明确的目标子系统，不沿系统层级自动向上冒泡，因此不能直接触发 `Kernel` 自身的迁移或
动作。

## 状态机

### 生命周期范式

生命周期符合 [SystemObject 四态](../system-signal.md#systemobject-四态)。`Preset` 和 `Setup` 是构建
阶段；OpenSBI 的引导交接触发 `Kernel.Transition::Enable`。`Startup` 仍只规范化为 Preset，因此
`Kernel.Startup` 表示设计期 `Kernel.Preset`，绝不表示真实固件入口。

### 状态与迁移

* Base：Kernel 规格尚未采纳。

* OnPreset：由 `Computer.Preset` 同步驱动。本轮没有额外的 Kernel 规格迁移内容，只提交 Prepared；
  不启动 BootTask 或任何内部 Flow。

* Prepared：Kernel 规格已建立，等待构造。

* OnSetup：由 `Computer.Setup` 同步驱动。先同步发送 `Config.Enable`，再同步发送 `Lds.Enable`；Lds
  必须依赖已经 Online 的 Config。随后验证两者并构造 kernel image，提交 Ready。Config 与 Lds 的
  完整模型初态都是 Ready：Enable 只验证、发布构建输入，不代表运行期初始化。

* Ready：kernel image 已构造，Config/Lds 已 Online，可以接受固件交接。

* OnEnable：只接受 OpenSBI 的真实入口交接。必须验证 `Riscv64Platform`、`OpenSBI`、`Riscv64`、
  `SbiSpec`、`BootArgs`、Config/Lds 和 kernel image，确认静态 `BootTask` 与入口 ABI，并精确检查
  `BootCpuRegisters.a0 == BootArgs.boot_hartid`、
  `BootCpuRegisters.a1 == BootArgs.dtb_pa`；不得把整组寄存器已准备完成作为前置。验证完成后提交
  Kernel.Online，再异步发送 `BootInitFlow.Preset`。

* Online：Kernel 实例自身已经启动并把控制权交给 BootInitFlow。Kernel 在整个内部启动、首次调度、
  PID 1 初始化和 payload 交接期间始终保持 Online；它不等待这些内部边界完成，也不在后续任何阶段
  再次提交 Online。`SystemState.Online`、`BootInitFlow.Online` 和
  `KernelInitFlow.PayloadHandoffCommitted` 保持互相独立。

`BootTask.OnCpu` 是固件/架构入口交接的初态事实，在 `_start` 紧随 Kernel Enable 接受点观察且只观察
一次。`BootInitFlow` 是 `BootTask.initial_flow` 指向的 TaskFlow；它收到 Preset 后自行串联
Preset/Setup/Enable：Preset 直接编排入口对象并提交 Prepared、自发 Setup；Setup 直接顺序驱动
`EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、
`IrqTimeInitPhase`、`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase`、
`BootInitRestInitPhase`，提交 Ready、自发 Enable；Enable 只驱动 `BootInitScheduleHandoffPhase`，提交
Online 后直接异步发送 `Scheduler.Action::Schedule`。

首次调度真实切换先同步驱动 BootTask.Suspend，随后提交 CurrentTaskSlot 与 context-switch prepare
事实并完成物理栈切换；next 栈上的 finish 原子保存/发布 BootTask 断点、消费 KernelInitTask 断点并
提交 CurrentTaskSlot/OnCpu/Live。PID 1 的真实入口直接启动 `KernelInitFlow.Preset`，不再回调 Kernel
的 Setup 或 Enable。Preset body 必须在 `kernel_init_entry()` 验证 PID 1 vmalloc stack 后执行。

`KernelInitFlow.Preset` 直接驱动 `PreSmpInitPhase`、`SmpBringupPhase`；Setup 直接驱动
`RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、`FinalizePhase`、`PayloadPreparePhase`；Enable
只驱动 `PayloadHandoffPreparePhase`。Flow Online 后执行受 parent Task OnCpu 约束的
`CommitPayloadHandoff`：UserBoot 执行 Flow replacement，Hello/Smoke 保持 KernelInitFlow 并进入
内核态 no-return entry；该 action 不触碰 Kernel.Online。

## 动作

* OnIrq：内核响应中断信号。

  > GAP: Kernel.OnIrq 尚未在 spec/model/systems/kernel.spec 定义为 Online 状态内 action，也未映射到现有 RiscvIntc、PLIC 和 IRQ 子系统处理链。

## 功能构成

内核的核心功能是建立和维护应用的运行环境，支持运行环境的基础是`任务子系统`。

内核直接接收的唯一运行时信号是中断信号，代表内核处理中断的边界是`中断子系统`。

`任务子系统`和`中断子系统`目前是 charter 层的候选系统边界，model 映射暂时 deferred。后续需要
先确定它们是新的聚合系统对象，还是由现有 Task、Scheduler、BootInitFlow 直接 interrupt 叶阶段、EventStream、
InterruptStream 和 IRQ 对象改造形成；在决定前，不把任一现有 phase 或 stream 直接等同于这两个
子系统。

## 引用

* [系统与信号模型](../system-signal.md)
* [概念体系迁移](../concept-migration.md)
* [阶段范式](../phase-paradigm.md)
* [charter/phases](../phases)

## 映射目标

* [model/kernel](../../model/systems/kernel.spec)
