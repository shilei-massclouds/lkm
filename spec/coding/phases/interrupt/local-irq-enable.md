# LocalIrqEnablePhase coding

model 来源为 `spec/model/phases/interrupt/local-irq-enable/phase.spec`，实现落点为
`impl/arceos_ex/src/phases/interrupt/local_irq_enable.rs`。本文件同时承载原 legacy formal rule
的有效实现约束；coding 层不再保留同主题 `.spec`。

## 生命周期映射

| Transition | depends / drives / ensures | checkpoint 与 continuation |
| --- | --- | --- |
| Preset: Base -> Prepared | 精确检查 Base、IrqTimeInit Online、本地中断关闭与 early flag；无外层 context；依次清除 early flag并执行 `InterruptStream.Enable`/SIE 开启；检查只有 boot CPU 总入口开放 | 接受后发出 Started；提交后发出 Prepared，调用 Setup |
| Setup: Prepared -> Ready | 精确检查 Prepared 和完整 `local_irq_enable_phase_ready()` | 提交后发出 Ready，调用 Enable |
| Enable: Ready -> Online | 精确检查 Ready 并重新确认外部 IRQ、softirq、IPI、worker、RCU、task 和 SMP 门仍关闭或 deferred | 提交后发出 Online，返回 `interrupt::preset_after_local_irq_enable()` |

`is_online()` 只接受精确 Online。LocalIrqEnable 不得启动 IrqOpenPrepare。

## 已迁移实现约束

#### Model path

LocalIrqEnablePhase is InterruptPhase subphase 2. Its formal model
path is spec/model/phases/interrupt/local-irq-enable/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/local_irq_enable.rs.

#### Separate subphase

local_irq_enable() must be represented as a standalone
LocalIrqEnablePhase after IrqTimeInitPhase.Online. It must not be
folded into IrqTimeInitPhase or moved to IrqOpenPreparePhase.

#### Scope

The phase may only open the boot CPU local interrupt total gate
(RISC-V sstatus.SIE) and clear early_boot_irqs_disabled. It must not
enable PLIC source gates, root external input gates, periodic tick,
full softirq execution, workqueue workers, RCU GP kthreads, task
concurrency or SMP concurrency.

#### Linux ordering

start_kernel() clears early_boot_irqs_disabled before executing
local_irq_enable(). The implementation must preserve that ordering
so there is no window where SIE is open while the early flag still
claims IRQs are disabled.

#### Context

LocalIrqEnablePhase must not be wrapped in a within context. It is
the standalone boundary that changes the boot CPU local interrupt
context from disabled to enabled.

#### Deferred runtime gates

The ready check must keep the negative facts observable: root
supervisor external input, PLIC UART source enable, full softirq
execution, IPI runtime, workqueue workers, RCU GP threads, task
concurrency and SMP concurrency remain closed or deferred.
