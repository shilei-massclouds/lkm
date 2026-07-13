# FinalizePhase coding

FinalizePhase 是 SmpRuntimePhase 的第 6 个直接子阶段，由 KernelInitTask 执行。model 路径为
`spec/model/phases/smp-runtime/finalize/`，实现落点为
`impl/arceos_ex/src/phases/smp_runtime/finalize.rs`。

## 生命周期映射

`preset()` 依赖 RootfsPhase 精确 Online，发出 Started，按 model 顺序驱动收尾对象并提交
Prepared。Setup/Enable 只检查 `finalize_phase_ready()` 与 FinalizeBoundary，分别提交
Ready/Online。Online 后只能返回 `smp_runtime::enable_after_finalize()`；由父阶段提交
SmpRuntimePhase.Online 后，才进入 `kernel::enable_after_smp_runtime()` 和 PayloadPhase。

#### Entry gate

FinalizePhase must run after RootfsPhase.Online and preserve the
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
