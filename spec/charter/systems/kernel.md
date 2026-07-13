# 内核系统

内核系统（简称内核）指代计算机中的一次内核运行实例，它负责管理物理资源和建立应用运行环境。

> [model] MUST：内核正式命名是Kernel。

内核是被动模式，本身不主动改变自身状态，不主动对外发送信息，只有受到外部事件触发时，才会产生反应，处理完毕后重回静默状态。由于外部事件可能并发到来，内核对不同事件的响应过程可能重叠，所以内核需要建立同步/互斥机制进行必要的内部协调与状态保护。

## 边界与交互

内核有三条边界：面向OpenSBI的启动边界，面向中断源的中断边界和面向硬件平台的硬件边界。

内核是被动系统，只针对两种外部事件产生反应：OpenSBI发出的启动事件，中断源发出的中断事件。

<img src="D:\doc\内核边界.svg" alt="内核边界" style="zoom:50%;" />

1. 启动边界：OpenSBI在引导末尾向与内核发出启动事件，交接系统控制权，内核启动。启动事件引起内核内部的一系列连锁事件，推动内核的生命周期状态前进，并且触发首个应用启动。
2. 中断边界：中断控制器代表各中断源向内核发出中断事件，内核响应中断。
3. 硬件边界：硬件寄存器/MMIO界面。内核**不主动**访问硬件，内核对硬件的访问来源于启动事件的余波或者来源于中断事件。
4. 应用边界：应用是内核的组成部分，因此应用边界**不是**内核的边界，它只是内核内部的子系统边界。

## 生命周期

### 范式

符合[阶段范式](../phase-paradigm.md)。内核启动事件对应Preset迁移事件。

> [model] MUST：内核启动事件对应Preset迁移事件。

### 状态与迁移

* Base：内核映像驻留在内存/闪存中，尚未启动，等待启动事件。

  > [model] MUST：确保 Riscv64 规范、SBI 规范、OpenSBI、Lds 和 Config 都处于 `Online` 状态。

* Preset：内核通过引导期过程。

  > [model] MUST：驱动BootPhase.Preset，等待BootPhase到达Online状态。

* Prepared：内核完成早期引导；中断尚未开启。

* Setup：内核通过中断期引导过程。

  > [model] MUST: 驱动InterruptPhase.Preset，等待InterruptPhase到达Online状态。

* Ready：中断已经开启，内核具备了支持多任务的能力，但是还没有真正启动多任务。

* Enable：内核通过单核多任务期、多核运行期和应用交接期阶段。

  > [model] MUST: 依次驱动并等待UpMultitaskPhase、SmpRuntimePhase和PayloadPhase阶段完成。

* Online：内核处于正常服务状态，支持应用运行。

## 引用

* [阶段范式](../phase-paradigm.md)
* [charter/phases](../phases)

## 映射目标

* [model/kernel](../../model/systems/kernel.spec)
