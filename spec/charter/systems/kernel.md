# 内核系统

内核系统（简称内核）指代计算机中的一次内核运行实例，它负责管理物理资源和建立应用运行环境。

> [model] MUST：内核对象在正式规格model中，命名是Kernel。

内核是被动模式，本身不主动改变自身状态，不主动对外发送信息，只有受到外部事件触发时，才会产生反应，处理完毕后重回静默状态。当然，由于外部事件可能并发到来，内核对不同事件的响应过程可能重叠，所以内核需要建立同步/互斥机制进行必要的内部协调与状态保护。

`KernelProject.Enable` 驱动 `OpenSBI.Enable`。OpenSBI 是由内核工程启动链驱动的单一固件/交接对象；它完成控制权交接后发出启动事件，触发内核持续推动生命周期直至运行状态。

> [model] NOTE：`KernelProject.Enable` drives `OpenSBI.Transition::Enable`；`OpenSBI.Enable` emits `Kernel.Transition::Preset` 向内核发出启动事件。

## 边界

1. 启动边界：OpenSBI与内核的系统控制权交接边界，是内核启动入口。
2. 中断边界：中断硬件与内核响应例程之间的分界点，是内核的中断异常向量表入口。
3. 设备边界：设备的寄存器/MMIO界面，内核经由该边界向设备反馈。

## 生命周期

### 状态

* Base：内核映像驻留在内存中，但是内核尚未启动，等待启动事件触发。

  > [model] MUST：确保 Riscv64 规范、SBI 规范、OpenSBI、Lds 和 Config 都处于 `Online` 状态。

* Prepared：内核完成早期引导；中断尚未开启。

  > [model] MUST：引导期BootPhase已经完成。

* Ready：中断已经开启，内核具备了支持多任务的能力，但是还没有真正启动多任务。

* Online：内核处于正常服务状态。

### 迁移

* Preset：从 `Base` 到 `Prepared`，驱动引导期阶段 `BootPhase` 完成。
* Setup：从 `Prepared` 到 `Ready`，驱动中断期阶段 `InterruptPhase` 完成。
* Enable：从 `Ready` 到 `Online`，依次驱动单核多任务期 `UpMultitaskPhase`、多核运行期 `SmpRuntimePhase` 和 payload 应用引导期 `PayloadPhase` 完成。

> [model] NOTE：`BootPhase`、`InterruptPhase`、`UpMultitaskPhase` 和 `SmpRuntimePhase` 都只包含 `Setup` 迁移；`PayloadPhase` 包含 `Setup` 和 `Enable` 两段迁移。

## 触发与事件

事件触发内核的行动、状态变化和反馈。

### 外部事件

外部事件是触发内核产生反应的根源。

1. 启动触发：`KernelProject.Enable` 驱动 `OpenSBI.Transition::Enable`；`OpenSBI.Enable` 完成 OpenSBI 到内核的控制权交接后通过 `emits` 触发 `Kernel.Transition::Preset`。
2. 中断事件：中断控制器代表其它硬件设备向内核发出的中断事件，触发内核通过服务例程进行响应。中断事件包括时钟中断、外设中断和 IPI 中断；后续不引入 `Event::XXX`，而应落到对应的 action 或迁移名称。

### 内部触发

内部触发由内核在执行内部过程中产生，用于描述生命周期迁移链或运行期异步请求。

1. 迁移完成触发：生命周期迁移成功完成后，按模型 `emits` 字段触发下一级迁移。因此 `Kernel.Transition::Preset` 一旦发生，会继续触发 `Kernel.Transition::Setup`；`Kernel.Transition::Setup` 完成后继续触发 `Kernel.Transition::Enable`，直至内核进入 `Online` 状态。
2. 任务启动事件：发出启动新任务的请求；后续落到对应的任务创建 action 或迁移名称。
3. CPU启用事件：发出启动 AP 核的请求；后续落到对应的 CPU bringup action 或迁移名称。

## 引用

* 阶段 charter 后续拆分到 `spec/charter/phases/`。

## 映射目标

- Model：`spec/model/systems/kernel.spec`
