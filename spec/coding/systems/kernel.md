# Kernel 系统编码映射

本文件把 [`Kernel` model](../../model/systems/kernel.spec) 映射到
`impl/arceos_ex/src/systems/kernel.rs` 及其驱动的阶段调用链。生命周期和阶段顺序由 model
定义；具体 lowering 遵循[阶段范式代码映射](../phase-paradigm.md)，本文只规定 Kernel 的实现
落点。

## Composite 映射

Kernel 是 composite system，仍完整保留 `Base -> Prepared -> Ready -> Online` 状态和
`Preset/Setup/Enable` transition 边界。实现不使用一个同步函数包裹整棵子阶段树，而采用
start/completion continuation：

1. transition start 检查进入条件并调用首个 `drives` 子阶段；
2. 子阶段通过自己的 `emits` 链运行；
3. 最后一个子阶段回到 Kernel completion continuation；
4. continuation 检查 `ensures`、提交 Kernel 目标状态，再执行 Kernel `emits`。

因此 `systems/kernel.rs` 可以有轻量状态和 continuation 函数，但不得吸收叶子阶段对象动作、
检查或 checkpoint。

## 生命周期调用链

### Preset

OpenSBI 进入 `_start` 后，架构入口先保存固件参数，并按 `R -> T -> O -> A` 输出
`Kernel.Started`、`BootTask.Online`、`BootInitFlow.Started` 和 `EntryPreludePhase.Started` 的稳定
早期编码。`T` 只观察镜像中已存在的 PID 0 carrier；另外三条分别属于 Kernel.Preset、
BootInitFlow.Preset 和 EntryPreludePhase.Preset 的架构 lowering。进入 Rust 后依次 adoption
Prepare、Kernel、BootTask、BootInitFlow 和 EntryPrelude；
`systems::kernel::adopt_head_preset_start()` 校验 Kernel 仍为 Base 和准备条件，不重复输出
checkpoint。Kernel.Preset 的直接 `drives` 是 `BootInitFlow.Preset`，后者驱动
`EntryPreludePhase.Preset`。

EntryPreludePhase 达到 model `Online` 后先进入 BootInitFlow.Preset completion，提交
`BootInitFlow.Prepared`；随后 Kernel.Preset completion 精确检查 Kernel 仍为 Base 和
BootInitFlow Prepared，提交 Kernel `Base -> Prepared`，并启动 Kernel.Setup。

### Setup

Kernel.Setup 驱动 `BootInitFlow.Setup`。BootPhase 达到 Online 后由 BootInitFlow 的具名
continuation 启动 InterruptPhase；InterruptPhase 到达 Online 后提交 `BootInitFlow.Ready`，再由
Kernel.Setup completion 检查 EntryPrelude、Boot、Interrupt 与 BootInitFlow 状态，提交 Kernel
`Prepared -> Ready`，随后启动 Kernel.Enable。

### Enable

Kernel.Enable 的四个 `drives` 必须保持连续 owner 和顺序：

```text
BootInitFlow.Enable
  -> BootInitRestInitPhase -> BootInitScheduleHandoffPhase
  -> BootInitFlow.Online
  -> real BootTask-to-KernelInitTask stack handoff
  -> enable_after_boot_init() on KernelInitTask
  -> SmpRuntimePhase on KernelInitTask
  -> PayloadPhase on KernelInitTask
```

`kernel_init_entry()` 验证实际 SP 位于 KernelInitTask 的 vmalloc stack 后调用
`systems::kernel::enable_after_boot_init()`。该具名 Kernel.Enable continuation 精确检查 Kernel
仍为 Ready、BootInitFlow Online、KernelInitTask Online、entry count 为 1 且 SP 验证成功，然后启动
SmpRuntimePhase。SmpRuntimePhase 完成后进入 `systems::kernel::enable_after_smp_runtime()`，检查前两棵子树已
完成并调用 Payload.Preset。PayloadPhase 完成 selected variant prepare、SelectedPayloadHandoff.Online
和 PayloadPhase.Online 后，先运行 Payload Online checkpoint handlers，再返回
`systems::kernel::mark_online()`；该函数检查三个 `drives` 阶段均已达到 model `Online`，提交 Kernel
`Ready -> Online` 并记录 `Kernel.Online`。随后 adapter 进入 selected payload 的 no-return entry。
PayloadPhase.Online、Kernel.Online 和实际 entry 是三个独立边界。

## 状态与 checkpoint

- `KERNEL_STATE` 只记录 Kernel 四个 model 状态，不驱动子对象生命周期。
- `Kernel.Started` 属于 Kernel.Preset 开始边界；早期入口必须在任何子阶段 marker 前发出其
  稳定编码，Rust system adoption 不得重复发出。
- `BootTask.Online` 和 `BootInitFlow.Started` 必须紧随 Kernel.Started，且分别只观察/发出一次。
- `Kernel.Online` 属于 Kernel.Enable 完成边界，只能在 PayloadPhase.Online 已提交后发出。
- 子阶段 checkpoint 保留在对应 phase module，不得由 `systems/kernel.rs` 代发。

BootInitFlow、EntryPrelude、Boot、Interrupt 和 SmpRuntime 子树均使用精确 `is_online()` 查询和
完整四状态 checkpoint；Kernel.Enable 与 Payload 只消费 SmpRuntimePhase.Online。

## 所有权与范围

Kernel 只拥有顶层生命周期和阶段顺序。ELF、地址空间、syscall、文件系统、TTY、凭据等由
对应 phase/object coding 文件描述。Payload 是 Kernel.Enable 的直接 `drives` 目标，不是
RuntimeCore 的隐式副作用；`UserBootPayload` 是 selected payload variant，不是第二条启动链。

BootIdle continuation 若恢复，只能进入无限 idle 调度循环。SmpRuntime 和 Payload 必须由
`kernel_init_entry()` 在 KernelInitTask task stack 上执行并经具名 Kernel continuation 进入后续
drive；KThreaddTask 不得沿启动栈执行 payload。
