# CorePreparePhase coding

本文件承载 `spec/coding/phases/boot/core-prepare.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/boot/core-prepare.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`core-prepare.spec`](core-prepare.spec)。

### ArceosExCorePrepareCodingMust

#### CorePrepare concurrency boundary

CorePreparePhase runs after paging_init() and before trap_init() /
mm_core_init() while Linux still has early_boot_irqs_disabled == true
and secondary CPUs have not been brought online. The implementation
must not open local IRQs, task concurrency, or SMP concurrency in
this phase; its ready check must still observe the model facts
early_boot_irqs_disabled_true(), interrupt_concurrency_closed(),
task_concurrency_closed(), and smp_concurrency_closed().

#### StaticBranch.setup guard facts

StaticBranch.setup() models Linux jump_label_init(). Even though the
current prototype runs before SMP/task concurrency opens, generated
code must make the Linux guard semantics visible as facts: the CPU
hotplug read guard corresponding to cpus_read_lock() and the
jump_label_mutex guard corresponding to jump_label_lock().

#### JumpLabelMutex object mapping

The Linux jump_label_mutex used by jump_label_lock() must be
represented as an independent Context object, not as a private bool
hidden inside StaticBranch. CorePrepare setup must drive its static
initializer/Preset and Ready setup before StaticBranch.setup()
consumes it.

#### CpuHotplugLock object mapping

The Linux cpu_hotplug_lock used by cpus_read_lock() must be
represented as an independent Context object of type
PerCpuRwSemaphore. The object may depend on PerCpuStorage.Prepared
for early boot-CPU static per-cpu storage, but PerCpuRwSemaphore as a
generic type must not be globally tied to PerCpuStorage. CorePrepare
setup must drive PerCpuStorage.Preset, CpuHotplugLock.Preset and
CpuHotplugLock.Setup before StaticBranch.setup() consumes it.

#### StaticBranch jump-label mutex guard lowering

StaticBranch.setup() must preserve the StaticBranchJumpLabelContext
source boundary and execute the modeled JumpLabelMutex.Lock/Unlock
protocol, or an equivalent implementation that preserves owner,
nesting/debug and wakeup-observable effects. SingleTaskContext facts
are not enough to erase the mutex protocol in the current lowering
strategy. The CorePrepare ready check must observe the independent
JumpLabelMutex ready object and a completed lock/unlock guard fact.

#### StaticBranch CPU hotplug read guard lowering

StaticBranch.setup() must preserve the CpuHotplugReadContext source
boundary for cpus_read_lock()/cpus_read_unlock() and execute the
modeled CpuHotplugLock.ReadLock/ReadUnlock pair. SingleTaskContext
facts are not enough to erase this read-side protocol in the current
lowering strategy. The generic PerCpuRwSemaphore implementation must
still provide real read/write behavior for later call sites and
smoke tests.

#### PerCpuRwSemaphore observable behavior

PerCpuRwSemaphore must model static/runtime initialization, ready
setup, read-side fast path, writer block flag, reader drain, reader
slow-path/blocked observations, write unlock wakeup, and the local
RcuSync child object. Lockdep, tracing, exact scheduler waitqueue
mechanics and a true asynchronous RCU grace-period service may be
internal or deferred, but the visible counters and outcomes must be
testable from the implementation.

#### Text patch synchronization boundary

Runtime static-key code patching uses separate text patch guards and
instruction-cache synchronization on RISC-V. CorePrepare
StaticBranch.setup() must not silently claim those runtime sync
effects; they remain deferred to later StaticBranch action modeling.

#### ResourceTree setup guard

Linux init_resources() inserts resources through insert_resource(),
whose kernel/resource.c path takes resource_lock with write_lock().
ResourceLock must be represented as an independent Context object of
type RwLock, not as a bool hidden inside ResourceTree. CorePrepare
setup must drive ResourceLock.Preset and ResourceLock.Setup before
ResourceTree.setup() consumes it.

#### ResourceTree resource_lock guard lowering

ResourceTree.setup() must preserve the ResourceTreeWriteContext
source boundary and execute the modeled
ResourceLock.WriteLock/WriteUnlock pair. SingleTaskContext facts are
not enough to erase this write-side protocol in the current lowering
strategy. The generic RwLock implementation must still provide real
read/write behavior for later call sites and smoke tests.

#### RwLock observable behavior

RwLock must model static/runtime initialization, Ready/unlocked
setup, read-side sharing, write-side exclusion, trylock outcomes,
read/write unlock conditions and the ordinary read_lock()/write_lock()
boundary that does not save IRQ flags. Lockdep/debug owner,
PREEMPT_RT rwbase_rt, exact architecture raw lock details, exact
reader count internals and irqsave/bh/nested API variants may remain
internal or deferred until a concrete object needs them.

#### PrintkBuffer setup IRQ guard

Linux setup_log_buf() switches the active printk ring buffer under
local_irq_save()/local_irq_restore(). PrintkBuffer.setup() must
record the local IRQ save/restore guard before reporting Ready, and
must bind that guard to the existing BootCpuLocalInterrupt
LocalInterruptControl object rather than hiding it as a PrintkBuffer
internal bool.

In the current lowering strategy arceos_ex must execute or preserve
the save/restore protocol instead of relying on SingleTaskContext to
erase it. A future proof-only optimization may be reintroduced only
after the model marks the lexical guarded block with a verified
single-entry proof and confirms that no saved-flags/debug side
effects are consumed.

#### Randomness preset conditional lock boundary

Randomness.preset() maps to Linux random_init_early(command_line).
The main early-mix path calls the internal _mix_pool_bytes() helper
directly, not mix_pool_bytes(), so it does not take input_pool.lock
and generated code must not add an unconditional input-pool spinlock
guard merely because later random paths use that lock.

#### Randomness conditional reseed/credit lock boundary

Linux random_init_early() may enter crng_reseed() when crng_ready()
is already true, or _credit_init_bits() when trust_cpu is enabled.
Those conditional paths may update base_crng under
spin_lock_irqsave(&base_crng.lock, flags) /
spin_unlock_irqrestore(&base_crng.lock, flags). The current
arceos_ex minimal path may keep this condition deferred, but if the
path is implemented it must lower through the RawSpinLock irqsave
guard protocol, not through an unguarded update or unconditional
early-mix lock.

<!-- formal-predicate-notes:spec/coding/phases/boot/core-prepare.spec END -->
