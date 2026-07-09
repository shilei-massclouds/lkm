# FinalizePhase coding

本文件承载 `spec/coding/phases/smp-runtime/finalize.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/finalize.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`finalize.spec`](finalize.spec)。

### ArceosExFinalizeCodingMust

#### Model path

FinalizePhase is SMP Runtime Phase subphase 5. Its formal model
path is spec/model/phases/smp-runtime/finalize/.

#### Code path

Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.

#### Entry gate

FinalizePhase must run after RootfsPhase.Ready and preserve the
kernel_init_freeable() return boundary before PayloadPhase.

#### Deferred cleanup details

async_synchronize_full(), ftrace/free_initmem, mark_readonly() and
do_sysctl_args() must preserve Linux order but remain deferred in
this round. async_synchronize_full() must still expose the
async_done waitqueue, async_lock irqsave spinlock, entry_count
atomic and ASYNC_COOKIE_MAX ordering responsibilities as deferred
facts.

#### Trimmed current-config paths

kprobe_free_init_mem(), kgdb_free_init_mem(), exit_boot_config(),
pti_finalize() and numa_default_policy() must be recorded as
trimmed/no-op under the current RISC-V/default configuration.
The numa_default_policy() checkpoint belongs after SYSTEM_RUNNING
in FinalizePhase, not in the rest_init() path.

#### System state

SystemState.enter_freeing_initmem() must run immediately after
AsyncFullSyncDeferred.Ready and before init-only memory cleanup.
SystemState.enable() must run after PTI finalize and end with
SystemState.state == Online and value == SYSTEM_RUNNING.
RcuCore.end_inkernel_boot() must expose rcu_unexpedite_gp() atomic
decrement, CONFIG_RCU_LAZY related rcu_async_relax() trimming,
rcu_normal_after_boot WRITE_ONCE handling and rcu_boot_ended publish
as observable facts.

#### RCU boot end

RcuCore.end_inkernel_boot() must record rcu_boot_ended == true
without claiming full runtime RCU GP service implementation.

#### Boundary

FinalizeBoundary must mark the next boundary as PayloadPhase.

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/finalize.spec END -->
