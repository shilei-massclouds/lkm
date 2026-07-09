# LocalIrqEnablePhase coding

本文件承载 `spec/coding/phases/interrupt/local-irq-enable.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/interrupt/local-irq-enable.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`local-irq-enable.spec`](local-irq-enable.spec)。

### ArceosExLocalIrqEnableCodingMust

#### Model path

LocalIrqEnablePhase is InterruptPhase subphase 2. Its formal model
path is spec/model/phases/interrupt/local-irq-enable/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/local_irq_enable.rs.

#### Separate subphase

local_irq_enable() must be represented as a standalone
LocalIrqEnablePhase after IrqTimeInitPhase.Ready. It must not be
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

<!-- formal-predicate-notes:spec/coding/phases/interrupt/local-irq-enable.spec END -->
