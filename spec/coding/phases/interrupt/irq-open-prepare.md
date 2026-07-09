# IrqOpenPreparePhase coding

本文件承载 `spec/coding/phases/interrupt/irq-open-prepare.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/interrupt/irq-open-prepare.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`irq-open-prepare.spec`](irq-open-prepare.spec)。

### ArceosExIrqOpenPrepareCodingMust

#### Model path

IrqOpenPreparePhase is InterruptPhase subphase 3. Its formal model
path is spec/model/phases/interrupt/irq-open-prepare/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs.

#### Ordering

IrqOpenPreparePhase must run after LocalIrqEnablePhase.Ready, with
the boot CPU local interrupt gate already open. It must not contain
another local_irq_enable() boundary.

#### Runtime services

This phase must keep task concurrency and SMP concurrency closed and
must not implicitly start periodic tick service, IPI enable,
workqueue workers, RCU GP kthreads or full softirq execution.

#### SLUB late boundary

kmem_cache_init_late() must be represented as the internal
SlubSubsystem flush workqueue fact. It must not advance
SlubSubsystem to Online/FULL; that belongs to later slab_sysfs_init()
style work.

#### Console boundary

console_init() must prepare the formal Console object, line
discipline registry and early console driver set only. Real device
probe, boot console unregister and full handoff remain conditional
or deferred facts.

#### Trimmed/deferred paths

The panic_later checkpoint, lockdep_init(), locking_selftest(),
initrd bounds check, setup_per_cpu_pageset(), numa_policy_init(),
acpi_early_init(), late_time_init hook and arch_cpu_finalize_init()
positions must be represented by a structured
IrqOpenPrepareTrimmedPaths-style object. setup_per_cpu_pageset()
remains an explicit PageAllocator deferred fact; the others record
their current config/no-op reasons.

#### Sched clock local IRQ guard

sched_clock_init() must record the local_irq_disable()/
local_irq_enable() window around generic_sched_clock_init() through the existing
BootCpuLocalInterrupt LocalInterruptControl. The surrounding phase
context has local interrupts enabled, so this temporary guard must
remain an explicit protocol fact.

#### Smoke actions

SchedClock.setup() and DelayLoop.setup() must expose enough action
surface for smoke checks after IrqOpenPreparePhase.Ready: a sched
clock read that advances and a bounded busy-wait delay action.

<!-- formal-predicate-notes:spec/coding/phases/interrupt/irq-open-prepare.spec END -->
