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

## 工程模型与系统模型

内核系统是软件工程的产品，对工程同样基于状态机建模，用规格约束工程的过程。

顶层模型是计算机工程，内核工程作为计算机工程的下级。把计算机工程做为顶层根，可以为内核工程的分析、推导验证提供必要的前置条件，也为将来计算机系统整体的规格化设计预留空间。

内核工程模型涵盖了从规格建立、代码生成、组装构造以及运行评估的全生命周期，最后的阶段运行评估要对内核运行实例建立规格模型，称为内核系统模型。

内核系统模型是本项目的核心模型，其次是内核工程模型。

所有模型都优先采用标准状态和迁移描述，必要时进行扩展。

### 计算机工程模型

计算机工程模型（Computer Project）是构造计算机的工程过程实践的抽象，涵盖从规格标准到运行系统。该模型的最后一个阶段对产出的计算机系统实例进行启动和测试评估，这个计算机系统实例是包含了硬件/固件/内核/应用的完整系统。

1. 建立硬件标准和规范（preset）：从不存在（Base）到“纸上”计算机硬件（Prepared），具体到当前项目，是制订/遵循riscv64 ISA标准和soc硬件规格。
2. 制造硬件和构造固件（setup）：从“纸上”计算机硬件（Prepared）到可运行的计算机硬件/固件（Ready），具体到当前项目，是制造riscv64体系结构的Soc计算机，制订/遵循SBI规范并构造SBI固件，如OpenSBI。
3. 设计构造内核以及运行评估（enable）：从计算机硬件/固件（Ready）到可运行的完整的计算机系统（Online），具体到当前项目，在规格的指导约束下，构造内核，可以与Linux进行差分测试与评估。

<img src=".\pic\计算机和内核建模.svg" alt="计算机和内核建模" style="zoom:50%;" />

计算机工程模型的preset迁移和setup迁移自动在迁移完成时产生迁移完成通知事件，事件随即触发目标状态继续迁移，由此产生连锁反应直至Online状态。

> MUST[model]：计算机工程模型
>
> 1. 外部事件：唯一的工程启动事件PRESET，作用于Base状态，触发preset迁移
> 2. preset和setup迁移在完成时自动产生通知事件，分别触发Prepared和Ready状态迁移

### 内核工程模型

内核工程模型由计算机工程模型在enable时驱动建立，工程生命周期包括从建立规格到运行评估。

1. 建立内核规格（preset）：从不存在（Base）到规格化描述的内核（Prepared），具体到当前项目，是制订多层次的规格定义，并能够通过推导验证。这个多层次的规格定义即**内核系统模型**。
2. 生成代码和组装构造（setup）：规格化描述的内核（Prepared）到内核Image（Ready），具体到当前项目，是在规格指导下由AI生成代码、封装组件和组装内核Image。
3. 启动内核和测试评估（enable）：从内核Image（Ready）到内核运行实例（Online），具体到当前项目，内核系统启动完成进入到运行状态，通过各类测试和评估达到预期目标。

内核工程模型的preset迁移和setup迁移自动在迁移完成时产生迁移完成通知事件，事件随即触发目标状态继续迁移，由此产生连锁反应直至Online状态。

> MUST[model]：内核工程模型
>
> 1. 外部事件：唯一的工程启动事件PRESET，作用于Base状态，触发preset迁移
> 2. preset和setup迁移在完成时自动产生通知事件，分别触发Prepared和Ready状态迁移

### 内核系统模型 - 本项目核心模型

内核系统模型描述了一个内核实例的规格。

<img src=".\pic\内核系统模型.svg" alt="内核系统模型" style="zoom:50%;" />

内核本身是惰性的，只能被动等待外部事件的触发而产生反应。

外部事件只有两类：

1. 启动事件：源于计算机的开机事件，这个事件只发生一次，作用于内核的初始状态Base，之后的每级迁移完成时，自动触发内部的完成事件，驱动状态自动迁移，如此形成连锁反应直到Online状态，即内核启动完成，开始正常提供服务。整个启动过程分为四个状态，Base状态时仅有单任务且未开中断，Prepared状态时中断开启，Ready状态时进入多任务，Online状态时SMP多核已启动。
2. 中断事件：包括时钟中断和外设中断，不同的中断触发内核不同的反应，处理完成后重回触发时的状态，等待下一次中断事件。内核处于不同的状态时，处理中断的能力不同。Base状态时不响应中断事件，从Prepared状态开始逐步具备响应各种中断的能力，直到Online状态时达到响应中断的最大开放能力。注意：中断事件不会导致状态迁移，引起状态迁移的是启动事件以及后续的迁移完成事件。

启动事件具有“惯性”，最初由每一级的迁移完成事件传递，直至Online状态。在Online状态下，内核具有两种策略，一是进入等待状态，等待中断的触发；二是进入轮询状态，不断尝试主动读取外部设备的状态，符合某种条件时采取处理措施。因此，当采取轮询策略时有一个特例，外部设备的状态变化基于轮询机制触发内核的内部反应。
> MUST[model]：内核系统模型
>
> 1. 外部事件：启动事件PRESET，作用于Base状态，触发preset迁移；中断事件，包括时钟中断、外设中断、核间中断等类型，作用于Prepared/Ready/Online状态，触发相应动作。
> 1. preset和setup迁移在完成时自动产生通知事件，分别触发Prepared和Ready状态迁移

# 规格

内核**规格**是内核之所以称之为内核的**特征**，是所有内核实例的最大公约数。

规格建立在状态机模型基础上，是对模型的细化。核心是内核系统模型的规格，另外还包括内核工程模型和前置依赖的计算机工程模型。

## 计算机工程规格

对构建计算机系统的过程约束，参照前述计算机工程模型，包括三个阶段：定义体系结构和硬件平台规范，生产硬件平台和构造固件，构造内核然后进行系统集成测试和评估。

> MUST[model]：计算机工程规格，遵循''标准状态和迁移''和“计算机工程模型”
>
> Preset：建立Riscv64 ISA规范
>
> Setup：建立硬件平台，定义BootArgs标准，建立SbiSpec规范，实现OpenSBI
>
> Enable：驱动内核工程



## 内核工程规格

构造内核系统的工程过程约束，参照前述内核工程模型，包括三个阶段：建立内核系统规格、构造内核映像、造到启动评估。

> MUST[model]：内核工程规格，遵循''标准状态和迁移''和“内核工程模型”
>
> Preset：建立内核规格charter、model和coding
>
> Setup：定义Lds和Config，构造产生内核映像
>
> Enable：驱动内核系统的实例启动，测试和评估



## 内核系统规格

内核系统的规格约束了内核系统实例的启动与运行行为，以及测试和评估过程。

> MUST[model]：内核系统规格，遵循''标准状态和迁移''和“内核系统模型”
>
> Preset：接替固件引导计算机系统，驱动 BootInitFlow 的入口前导部分
>
> Setup：由 BootInitFlow 接续入口前导期，依次推进引导期和中断期
>
> Enable：完成 BootInitFlow、真实切换到 PID 1，推进多核运行期并完成对选中 Payload 的交接

### 启动执行阶段 BootInitFlow

`BootInitFlow` 是静态 `BootTask` 的 PhaseObject 子对象，使用标准
`Base -> Prepared -> Ready -> Online` 生命周期。Preset 驱动 `EntryPreludePhase`，Setup 顺序驱动
`BootPhase` 与 `InterruptPhase`，Enable 顺序驱动 `BootInitRestInitPhase` 与
`BootInitScheduleHandoffPhase`，建立 `BootIdleFlow` 首个 owner/active binding，并在首次真实 PID 1
切换的 commit 边界到达 Online。

### 入口前导期EntryPreludePhase

`EntryPreludePhase` 拥有入口期的 `BootTaskEntryBinding` 协调协议。它按“物理 `tp` binding →
VM setup → 虚拟 `tp` binding”建立调度器运行前的初始抢占关闭条件。binding 不是 Task、TaskRef
或 Flow，不改变 PID 0 identity。`BootTask` 在入口前已经由静态初始化器构造为 Online；本阶段只
验证其稳定 storage、PID 0、`TaskRef::BOOT` 和 canonical identity。

### 引导期BootPhase

从入口前导完成到中断期开始之前的阶段。

#### EntrySuccessorPhase

待补充。

#### CorePreparePhase

待补充。

#### MmCoreInitPhase

待补充。

#### SchedInitPhase

待补充。

### 中断期InterruptPhase

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

同一 `BootTask` 保持 PID 0 与 Task identity；`BootInitRestInitPhase` 创建具有各自 Task identity
与初始 Flow 的 `KernelInitTask` 和 `KthreaddTask`，`BootInitScheduleHandoffPhase` 预检并提交首次
调度事实。`BootIdleFlow` 是 BootTask 的首个 TaskFlow，不是从启动 TaskFlow handoff 得到。

#### BootInitRestInitPhase

待补充。

#### BootInitScheduleHandoffPhase

待补充。

#### BootIdleEntryPhase

它是 `BootIdleFlow` 的子 Phase，只在调度器未来恢复 BootTask 后执行，不属于 BootInitFlow 的完成链。

#### PreSmpInitPhase

待补充。

### 多核运行期SmpRuntimePhase

启用多核，完成内核初始化并引导进入用户态应用或Unikernel应用。

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

### 应用引导期PayloadPhase

从准备应用启动环境到切换到应用。

#### PayloadExecSyncBoundaries

待补充。

#### UserCloneDeferredBoundaries

待补充。

#### UserBootPayloadSetup

待补充。

#### UserBootPayloadEnable

待补充。
