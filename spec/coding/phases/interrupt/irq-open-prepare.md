# IrqOpenPreparePhase coding

model 来源为 `spec/model/phases/interrupt/irq-open-prepare/phase.spec`，实现落点为
`impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs`。本文件同时承载原 legacy formal rule
的有效实现约束；coding 层不再保留同主题 `.spec`。

## 生命周期映射

| Transition | depends / drives / ensures | checkpoint 与 continuation |
| --- | --- | --- |
| Preset: Base -> Prepared | 在 `SingleTaskInterruptStreamContext` 下精确检查 Base、LocalIrqEnable Online 及对象依赖；按 model 顺序执行现有 SLUB flush、Console、trimmed paths、SchedClock 和 DelayLoop 动作 | 接受后发出 Started；提交后发出 Prepared，调用 Setup |
| Setup: Prepared -> Ready | 精确检查 Prepared 和完整 `irq_open_prepare_phase_ready()` | 提交后发出 Ready，调用 Enable |
| Enable: Ready -> Online | 精确检查 Ready 并重新确认 invariant | 提交后发出 Online，返回 `interrupt::preset_after_irq_open_prepare()` |

`is_online()` 只接受精确 Online。IrqOpenPrepare 不得启动 ProcessPrepare。

## 已迁移实现约束

#### Model path

IrqOpenPreparePhase is InterruptPhase subphase 3. Its formal model
path is spec/model/phases/interrupt/irq-open-prepare/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs.

#### Ordering

IrqOpenPreparePhase must run after LocalIrqEnablePhase.Online, with
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
surface for smoke checks after IrqOpenPreparePhase.Online: a sched
clock read that advances and a bounded busy-wait delay action.
