# 内核对世界的认识

内核寄生于计算机并对其进行控制。

内核：即操作系统内核，正式英文名称是Kernel

计算机：正式英文名称是Computer

内核是一个独立的系统，所驱使的计算机是更高一级的系统。内核与硬件、固件以及应用共同构成计算机系统，而它是计算机系统的核心。

<img src=".\pic\内核与计算机.svg" alt="内核与计算机" style="zoom: 50%;" />

内核从两个层面划分自己与外部环境：

1. 内核边界：形式上是syscalls之类的公开接口。如此，下层的硬件和固件，上层应用都属于外部环境。
2. 计算机边界：形式上是计算机硬件的中断接口和I/O接口。人和自然因素都属于外部环境。

**当前**，内核边界是关键，计算机系统边界仅作为理解和分析的辅助。

内核是一种被动系统，总是处于静默等待的状态，外部环境发出的事件如果被内核接受，将会引发内核从边界到内部的连锁反应，内核状态可能会改变，可能完成某些动作后再通过边界向外部环境反馈事件，然后重新回归静默。复杂之处在于，外部环境的事件可能不是串行发生的，内核的上述响应过程可能经常处于相互叠加状态，从边界到内部必须建立一系列同步/协调机制防止内核系统的运行状态被破坏。

计算机同样是被动系统，但是仅在必要时才需要去分析它与内核相关的方面。

# 模型

状态机作为内核相关对象建模的通用模型。相关对象包括内核自身，更高层级的计算机，构成内核的各级子系统，构成子系统的各类资源与执行流。状态机是所有这些对象的共同的行为抽象。

<img src=".\pic\状态机概念.svg" alt="状态机概念" style="zoom:50%;" />

状态机包括四个要素：

1. 状态 - state：对象的“稳态”。对象总是趋向于停留在稳态上，迁移过程中只有暂态，不作为状态，不作为分析推导的依据。
2. 事件 - event：触发对象状态发生变化或产生反应的信号，来源只能是两类，一是来自于对象之外，二是来自状态迁移完成时的触发通知。事件可能触发状态迁移，也可能仅仅触发动作。
3. 迁移 - transition：对象从一个状态到另一个状态的过程，有事件触发且符合条件时才会执行过程，过程完成时改变状态，过程中可以驱动本对象或其它对象的迁移或动作。
4. 动作 - action：对象响应处理事件的过程，不改变对象状态。

## Task、TaskFlow 与稳定引用

`Task` 是内核中唯一的 task_struct-like carrier。每个 Task 独立保存生命周期、稳定 identity、
PID、CPU 归属、调度状态、线程切换上下文以及它拥有的 Flow 引用。`BootTask`、
`KernelInitTask`（PID 1）、`KthreaddTask`、普通用户 child、AP idle 和由 smpboot 模板创建的
内核线程都使用同一 Task 类型；idle、init、
kthread 或测试角色只是 Task metadata，不能成为另一套 lifecycle、PID 或 switch-context carrier。

`TaskFlow` 是 Task 上一段 execution continuation 的独立生命周期载体。每个 Flow 恰有一个 owner
Task；一个 Task 可以按执行历史拥有多个 Flow，但任一时刻最多只能有一个 active Flow。exec 和
boot-idle handoff 保持 Task identity，只替换 active Flow；fork/clone 才创建 fresh Task，并为它
创建 fresh Flow。syscall、files、credentials、signal、地址空间等资源归属于实际当前 Task，不能
因为启动主线最初由 `KernelInitTask`（PID 1）执行就一律归入它。

每个 Task 以 typed `initial_flow` association 固定记录创建时 Flow；BootTask、KernelInitTask 和
KthreaddTask 分别绑定 `BootInitFlow`、`KernelInitFlow` 和 `KthreaddFlow`。TaskFlow 继承
PhaseObject 并以 `parent: Task` 约束 owner，但不拥有 guard 字段、guard 状态或 `process_guard`
类型块。每个 Flow lifecycle/执行 action 在尝试推进时即时检查 parent Task 为 OnCpu。

`TaskRef` / `TaskFlowRef` 是分别引用 Task / TaskFlow runtime identity 的稳定句柄。引用必须同时
携带私有 storage slot 和非零 generation；storage 可在对象完整 Cleanup 后回收，但旧 generation
永远不能重新变为有效。current-task slot、runqueue、scheduler、wait/reap record 和 checkpoint
诊断都传递这种引用，不能用 BootTask/KernelInitTask/UserChild 等角色枚举代替 identity。

本项目采用相对简单的状态机模型，简化未来的建模、推导。

唯一复杂之处在于，事件不仅可以来自外部，也可以是来自迁移完成时触发的通知，是否触发可以根据情况配置。通过此机制可以让部分状态无须由外部事件触发而自动迁移，进而达到一个外部事件引起多个状态顺序推进的连锁反应效果。

> MUST[model]: 状态机规格
>
> 1. 状态机是惰性的，只有事件可以触发状态机产生反应，可以推动状态迁移的是迁移事件，不引起状态变化的是动作事件
> 2. 按照事件产生源头，分为外部事件和内部事件两种，外部事件由其他对象发出；内部事件仅支持迁移完成事件，可以要求迁移完成时向目标状态发出通知
> 3. 事件与迁移/动作一一对应，命名相同，只是事件名称全大写，以方便区分
> 4. 迁移完成事件执行顺序：驱动迁移或动作 -> 更新状态 -> （可选）向目标状态发出迁移完成事件

## 惰性状态机

状态机模型描述了内核系统以及相关构成对象的特点，总是被动等待外部事件的驱使才会做出反应，不会主动产生变化，不会主动寻求与外部环境沟通。即本质上，外部事件是触发状态机做出反应的**唯一**因素，状态机可能迁移一个状态，也可能顺序连锁迁移多个状态，但最终重回被动等待。

必须**澄清**的一点：迁移完成时触发的事件不是系统内部主动产生的，它的源头也必然是来自于外部的某个事件，它只是外部事件的“余波”。

## 标准状态和迁移

所有对象的生命周期都可以分成启动、运行和释放三个阶段，每个阶段又可以进一步划分子阶段。

其中，启动阶段**推荐**基于如下的标准定义状态（4种）和迁移（3种）：

<img src=".\pic\状态机模型.svg" alt="状态机模型" style="zoom:50%;" />

Base代表尚未建立对象的初始状态，Online代表运行状态，其余状态和迁移按照字面意思和每个对象的具体情况进行定义。注意：这些状态和迁移只是**推荐**使用，对象可以在此基础上增减状态和迁移，特殊情况下也可以更名。

## 唯一顶层系统模型

完整模型只有一棵顶层系统树，以 `Computer` 为根，直接包含 `Riscv64Platform`、`OpenSBI` 和
`Kernel`。Kernel 下的启动 CPU 局部层级是 `BootCurrentCPU -> BootCPU -> BootCpuRegisters`；寄存器
对象不属于平台。

System 同时承担本实例的规格建立、构造和运行交接。`SystemObject` 四态统一表示当前实例的局部进度：
Base 尚未完成规格采纳，Prepared 已建立规格，Ready 已构造且可接受本层 Enable，Online 已完整履行
本 System charter 定义的 Enable 服务契约。Online 不自动等于异步下游或根请求全部成功，也不自动
排除内部初始化、应用环境或 payload 准备；这些过程是否属于本层 Enable 由具体 System charter 定义。

所有模型都优先采用标准状态和迁移描述，必要时进行扩展。

### Computer 模型

1. `Preset` 按固定顺序驱动 `Riscv64Platform`、`OpenSBI` 和 `Kernel` 的 `Preset`，分别建立三份规格并
   提交 Prepared；不发送自身 Setup。
2. `Setup` 按相同顺序驱动三个子 System 的构造；三者均 Ready 后建立 Computer assembly fact，提交
   Ready；不发送自身 Enable。
3. `Enable` 提交 Computer.Online 后异步发送 `Riscv64Platform.Enable`；不等待异步下游全部 Online。

<img src=".\pic\计算机和内核建模.svg" alt="计算机和内核建模" style="zoom:50%;" />

Human 在模型外部依次同步 drives Computer.Preset、Computer.Setup，二者成功后异步 emits
Computer.Enable；Computer 的三个 handler 不相互触发。子 System 规格和构造使用同步 `drives`。运行交接按
`Computer.Enable -> Riscv64Platform.Enable -> OpenSBI.Enable -> Kernel.Enable` 异步推进；
`BootInitFlow`、首次 Scheduler 调度与 `KernelInitFlow` 是 Kernel.Enable 的同步逻辑下层过程。

> MUST[model]：Computer 模型
>
> 1. 唯一无需预制条件的完整模型入口是 Human 外部编排，其第一个真实 Signal 是 `Human -> Computer.Preset`
> 2. Preset/Setup 必须按声明顺序驱动 Riscv64Platform/OpenSBI/Kernel
> 3. Enable 先提交 Computer.Online，再异步驱动 Riscv64Platform.Enable

### Kernel 构造与启动模型

`Config` 与 `Lds` 是 Kernel 的构建时静态输入，完整模型初态为 Ready。`Kernel.Preset` 建立 Kernel
系统规格并提交 Prepared；Setup 先驱动 Config.Enable，再驱动依赖 Config.Online 的 Lds.Enable，随后
只建立 ELF 与 boot Image 文件构造事实并提交 Ready。OpenSBI.Enable 选择 `kernel_load_pa`，保证 Image
已装载待交接且地址满足 PMD 对齐，再通过 Kernel.Enable 交接真实入口；Kernel 验证上游和入口 ABI，
在 Ready 内顺序驱动 BootInitFlow、首次调度和 KernelInitFlow 直至
`PayloadHandoffPreparePhase.Online`，再提交 Online 并异步发送 `CommitPayloadHandoff`。

> MUST[model]：Kernel 构造与启动模型
>
> 1. Kernel.Preset/Setup 只由 Computer 的同名迁移同步驱动
> 2. Kernel.Setup 按 Config.Enable、Lds.Enable 顺序发布输入，只完成 image 文件构造后为 Ready
> 3. Kernel.Enable 只接受真实 OpenSBI 交接，应用环境准备完成后提交 Online，并且只由该提交发送 payload handoff

### 内核系统模型 - 本项目核心模型

内核系统模型描述了一个内核实例的规格。

<img src=".\pic\内核系统模型.svg" alt="内核系统模型" style="zoom:50%;" />

内核本身是惰性的，只能被动等待外部事件的触发而产生反应。

外部事件只有两类：

1. 构造与启动事件：Computer 的 Preset/Setup 把 Kernel 从 Base 推进到 Ready；OpenSBI 的 Enable 交接
   在 Ready 内完成 BootInitFlow、首次调度、KernelInitFlow 和 payload 可逆预提交，把 Kernel 推进到
   Online。此 Online 表示应用运行环境已经准备就绪、等待应用启动。
2. 中断事件：包括时钟中断和外设中断，不同的中断触发内核不同的反应，处理完成后重回触发时的状态，等待下一次中断事件。内核处于不同的状态时，处理中断的能力不同。Base状态时不响应中断事件，从Prepared状态开始逐步具备响应各种中断的能力，直到Online状态时达到响应中断的最大开放能力。注意：中断事件不会导致状态迁移，引起状态迁移的是启动事件以及后续的迁移完成事件。

启动事件具有“惯性”，最初由每一级的迁移完成事件传递，直至Online状态。在Online状态下，内核具有两种策略，一是进入等待状态，等待中断的触发；二是进入轮询状态，不断尝试主动读取外部设备的状态，符合某种条件时采取处理措施。因此，当采取轮询策略时有一个特例，外部设备的状态变化基于轮询机制触发内核的内部反应。
> MUST[model]：内核系统模型
>
> 1. 外部构造事件 Preset/Setup 由 Computer 驱动；真实引导交接是 OpenSBI 发出的 Enable
> 2. Kernel 在内部启动期保持 Ready，`PayloadHandoffPreparePhase.Online` 后只提交一次 Online

# 规格

内核**规格**是内核之所以称之为内核的**特征**，是所有内核实例的最大公约数。

规格建立在状态机模型基础上，是对模型的细化。核心是内核系统模型及其前置 Computer、平台与固件
系统模型。

## Computer 规格

对完整计算机系统的规格、构造和启动过程作顶层约束，依次驱动三个直接子 System。

> MUST[model]：Computer 规格，遵循''标准状态和迁移''和“Computer 模型”
>
> Preset：顺序驱动 Riscv64Platform、OpenSBI、Kernel 建立三份系统规格
>
> Setup：按同序构造 Riscv64Platform、OpenSBI 固件与 kernel image，并建立 Computer assembly fact
>
> Enable：提交 Computer.Online 后异步发送 Riscv64Platform.Enable



## 内核系统规格

内核系统的规格约束了内核系统实例的启动与运行行为，以及测试和评估过程。

> MUST[model]：内核系统规格，遵循''标准状态和迁移''和“内核系统模型”
>
> Preset：建立 Kernel 规格并提交 Prepared
>
> Setup：依次发布 Config/Lds，构造 kernel image 并提交 Ready
>
> Enable：接替固件控制权，驱动内部启动与应用环境准备，提交 Kernel.Online 后异步发送 payload handoff

### 启动执行阶段 BootInitFlow

`BootInitFlow` 是静态 `BootTask.initial_flow` 指向的 TaskFlow；TaskFlow 继承 PhaseObject，因此它使用标准
`Base -> Prepared -> Ready -> Online` 生命周期。Preset 直接执行入口前导对象编排；Setup 直接顺序驱动
`EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase`、
`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase`、`BootInitRestInitPhase`；Enable 只驱动
`BootInitScheduleHandoffPhase`。三段 lifecycle 由 Kernel.Enable 依次驱动，Enable 提交 Online 后由
Kernel.Enable 继续驱动 `Scheduler.Action::Schedule`。不存在 `BootPhase` 或 `InterruptPhase` 包装
lifecycle，也不产生第二次 Kernel Enable 接受。

### BootInitFlow.Preset 的入口前导步骤

`BootInitFlow.Preset` 拥有入口期的 `BootTaskEntryBinding` 协调协议。它按“物理 `tp` binding →
VM setup → 虚拟 `tp` binding”建立调度器运行前的初始抢占关闭条件。binding 不是 Task、TaskRef
或 Flow，不改变 PID 0 identity。`BootTask` 在入口前已经由静态初始化器构造为 OnCpu；本步骤只
验证其稳定 storage、PID 0、`TaskRef::BOOT` 和 canonical identity。

### BootInitFlow 的引导叶子

从入口前导完成到中断期开始之前的阶段。

#### EntrySuccessorPhase

待补充。

#### CorePreparePhase

待补充。

#### MmCoreInitPhase

待补充。

#### SchedInitPhase

待补充。

### BootInitFlow 的中断/进程准备叶子

从中断启动到多任务启动之前。

#### IrqTimeInitPhase

待补充。

#### LocalIrqEnablePhase

待补充。

#### IrqOpenPreparePhase

待补充。

#### ProcessPreparePhase

待补充。

### BootInitFlow Enable

同一 `BootTask` 保持 PID 0 与 Task identity；`BootInitRestInitPhase` 完整驱动具有各自 Task identity
与初始 Flow 的 `KernelInitTask` 和 `KthreaddTask` Preset/Setup/Enable；Task Enable 只发布 Online，
不启动 Flow。`BootInitScheduleHandoffPhase` 预检并提交首次调度事实。`BootInitFlow` 是 BootTask 的
initial Flow；`BootIdleFlow` 是后继 active continuation，
且不会改写 `BootTask.initial_flow`。

#### BootInitRestInitPhase

待补充。

#### BootInitScheduleHandoffPhase

待补充。

#### BootIdleEntryPhase

它是 `BootIdleFlow` 的子 Phase，只在调度器未来恢复 BootTask 后执行，不属于 BootInitFlow 的完成链。

### KernelInitFlow 直接叶子

首次 dispatch 必须先同步提交 `BootTask.Suspend`，在真实栈切换后才由 `KernelInitTask.Continue`
提交 OnCpu；PID 1 的真实入口直接启动 `KernelInitFlow.Preset`。实际 body 必须在 `kernel_init_entry()` 验证 PID 1
vmalloc stack 后执行。Preset 驱动 `PreSmpInitPhase` 和
`SmpBringupPhase`，Setup 驱动 runtime/rootfs/finalize 与 `PayloadPreparePhase`，Enable 只驱动
`PayloadHandoffPreparePhase`。

#### PreSmpInitPhase

待补充。

#### SmpBringupPhase

待补充。

#### RuntimeCorePhase

待补充。

#### InitcallPhase

待补充。

#### RootfsPhase

待补充。

#### FinalizePhase

待补充。

### Payload 准备与 commit

`PayloadPreparePhase.Online` 是原 payload Ready 的 selection boundary；
`PayloadHandoffPreparePhase.Online` 是可逆 precommit。`KernelInitFlow` Online 后的
Kernel.Online commit 作为唯一发送者发出 `CommitPayloadHandoff`，才执行 UserBoot Flow replacement；
Hello/Smoke 不替换 Flow。

#### PayloadExecSyncBoundaries

待补充。

#### UserCloneDeferredBoundaries

待补充。

#### UserBootPayloadSetup

待补充。

#### UserBootPayloadEnable

待补充。
