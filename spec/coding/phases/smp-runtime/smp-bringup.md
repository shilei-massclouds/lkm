# SmpBringupPhase coding

本文件承载 `spec/coding/phases/smp-runtime/smp-bringup.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/smp-bringup.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`smp-bringup.spec`](smp-bringup.spec)。

### ArceosExSmpBringupCodingMust

#### Model path

SmpBringupPhase is SMP Runtime Phase subphase 2. Its formal model
path is spec/model/phases/smp-runtime/smp-bringup/.

#### Code path

Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.

#### BP/AP phase split

The implementation must keep the BP smp_init()/__cpu_up() line and
the AP secondary_start_sbi -> smp_callin() -> online-idle line
separate. BP code prepares resources, issues HSM hart_start and
waits for completions; AP code owns the AP entry phases and produces
those completions.

#### Per-AP idle task and stack

Each secondary CPU must have its own inactive IdleTask and dedicated
stack/pt_regs pointer prepared before hart_start. BootCPU's idle
task/stack must not be reused for AP boot data.

#### SBI HSM start path

RISC-V cpu_ops_sbi.cpu_start() must be lowered through SBI HSM
hart_start with secondary_start_sbi as entry and per-AP boot data
as opaque data. The current ordered booting path must not add a
spinwait fallback unless the model is extended first.

#### AP subphases

AP startup is not a single BP-side summary. The implementation must
expose minimal ApEntryPreludePhase, ApSmpCallinPhase and
ApOnlineIdlePhase checkpoints/facts, even if full CPU-local object
chains remain deferred.

#### Synchronization

BP/AP synchronization facts must stay explicit: cpu_running,
done_up and done_down placement must be represented even when AP
internals are summarized.

#### CPU hotplug guards

cpuhp_threads_init() must preserve the cpus_read_lock() and
smpboot_threads_lock mutex guards. bringup_nonboot_cpus()/cpu_up()
must preserve cpu_add_remove_lock and cpus_write_lock() writer
guards before publishing secondary online facts.

#### Completion wait locks

cpu_running and done_up are completions. Their wait.lock raw
spinlock irqsave/irqrestore guard must stay observable on the BP
wait side, while AP phases produce the matching completion facts.

#### SBI boot data ordering

RISC-V cpu_ops_sbi.cpu_start() uses smp_mb() before and after
publishing the secondary task/stack boot data. The implementation
must retain an equivalent observable ordering fact.

#### AP local sync summary

riscv_ipi_enable(), AP icache/TLB flush, AP local_irq_enable() and
the cpuhp_thread_fun() should_run memory-barrier pair remain
summary/deferred facts in this phase and must not be treated as
absent.

#### Online boundary

This phase must move secondary CPUs from present/not-online to
online only after the AP online-idle done_up fact is observed, then
publish the opening of SMP concurrency.

#### AP checkpoints

Long-term diagnostics must distinguish BP HSM request/return, AP
secondary entry reached, boot data consumed, AP current/stack
established, smp_callin cpu_running completion and online-idle
done_up completion.

#### Deferred hotplug callbacks

The CPUHP callbacks after CPUHP_AP_ONLINE_IDLE, per-thread callback
bodies and full CPU hotplug offline/rollback remain deferred; the
AP entry/callin/online-idle path itself is in scope.

#### Later runtime

SmpBringupPhase must hand off to RuntimeCorePhase. Later
subphases are expanded through their own formal model and code
steps rather than being silently assumed complete.

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/smp-bringup.spec END -->
