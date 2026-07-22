# SmpBringupPhase coding

SmpBringupPhase 是 KernelInitFlow.Preset 的第 2 个直接子阶段。父级 transition 与 BP 协调动作由
KernelInitTask 执行；AP Entry/Callin/OnlineIdle 由目标 AP 执行，不归 KernelInitFlow execution
ownership，逐 AP TaskFlow 仍为 deferred。model 路径为
`spec/model/phases/smp-runtime/smp-bringup/`，实现落点为
`impl/arceos_ex/src/phases/smp_runtime/smp_bringup.rs`。

## 生命周期映射

`preset()` 必须依赖 PreSmpInitPhase 精确 Online，发出 Started，然后在 BP 线上驱动准备和
HSM 发起。HSM 前由 BP 把三个 replicated family 的 secondary 状态重置为 Base；HSM 后 AP
entry 自行执行三阶段四态。BP 以 Acquire 观察全部目标 Online，再执行 completion wait、ack、
CpuGroup online publish 和 boundary。全部结果事实成立后提交 Prepared。`setup()`/`enable()`
只检查结果并分别提交 Ready/Online；Online 只返回
`KernelInitFlow.preset_after_smp_bringup()`。

| Checkpoint | owner state | owner |
| --- | --- | --- |
| `SmpBringupPhase.Started` | Base | KernelInitTask/BP |
| `SmpBringupPhase.Prepared` | Prepared | KernelInitTask/BP |
| `SmpBringupPhase.Ready` | Ready | KernelInitTask/BP |
| `SmpBringupPhase.Online` | Online | KernelInitTask/BP，父 continuation 前 |

`ApEntryPreludePhase`、`ApSmpCallinPhase`、`ApOnlineIdlePhase` 各自实现为独立过程 module，
使用 `[AtomicU8; MAX_CPUS]` 存储 per-logical-id 四态。AP 是单写者，transition 使用 AcqRel；
BP 的 `state_for()`/`all_online()` 使用 Acquire。它们不出现在 `Context` 中，也不使用普通资源
对象式聚合 `Lifecycle`。

| AP checkpoint | owner state | owner |
| --- | --- | --- |
| `ApEntryPreludePhase.Started/Prepared/Ready/Online` | Base/Prepared/Ready/Online | `ApIdleTask[n]` |
| `ApSmpCallinPhase.Started/Prepared/Ready/Online` | Base/Prepared/Ready/Online | `ApIdleTask[n]` |
| `ApOnlineIdlePhase.Started/Prepared/Ready/Online` | Base/Prepared/Ready/Online | `ApIdleTask[n]` |

旧 checkpoint ID 321–330 保持原序和语义位置；Prepared/Online 只追加，不改变既有 Linux exact
mapping。三个 phase 的 Online 依次返回具名 SmpBringup continuation：
`ap_after_entry_prelude(logical_id)`、`ap_after_smp_callin(logical_id)`、
`ap_after_online_idle(logical_id)`。最后一个 continuation 记录 park-loop entry 后进入既有 WFI
loop，不返回 BP 执行线。

ApEntryPrelude Preset adoption 必须在 BootDataConsumed 之后的任何成功 checkpoint 前验证：
boot-data 指针/`logical_id` 对应目标 slot，boot-data stack pointer 等于目标 16 KiB AP stack
栈顶，真实 `sp` 位于该 stack，真实 `tp` 和 boot-data task pointer 都等于目标 idle-task pointer。
失败必须输出稳定 phase/logical-id/state/check 诊断并 fail-stop。

`CpuStartProvider` 当前仍是 BP 聚合对象：SMP8 下某个 target 的 HSM return/AP entry 可以早于
全部 target 请求结束后的 `CpuStartProvider.Ready` checkpoint。HSM 前 BP 必须先验证并 release
发布 AP 全局 prerequisite gate；AP Preset acquire 该 gate，并以自己的 target boot data 完成
adoption。该时序不允许 AP 把 provider 聚合 Ready 当成本地状态，也不改变 BP 最终仍需观察
provider Ready 和全部 AP Online 的要求。

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

The inactive IdleTask storage is the same unified `Task` core used by
all other task families, and its initial idle continuation uses the
unified `TaskFlow` core. `task_ptr` in HSM boot data points to that
Task carrier. Concrete AP TaskRef/TaskFlowRef slots are internal
lowering under `SecondaryIdleTaskSet`; they do not complete the model-
deferred AP object topology.

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

BP all-online wait 复用现有 spin budget。超时时输出稳定 `phase`、首个未完成 `logical_id` 和
Acquire 读取的当前 `state`；不得在超时路径补写 AP 状态。SecondaryCpuStartupAck 只消费
ApSmpCallin family Online，SecondaryCpuOnlineAck 只消费 ApOnlineIdle family Online 和 park-loop
entry fact，所有下游 runtime-ready 查询也只消费精确 Online。

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

family Online 不是跨 AP 阶段屏障：AP1 可以进入 SmpCallin 时 AP2 仍在 EntryPrelude。只有 BP
在全部 AP 的三个 phase 都到达 Online 后建立 all-online barrier 并继续 ack/boundary。

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

SmpBringupPhase must return to its parent continuation after Online. The parent then starts
RuntimeCorePhase; the leaf must not call the sibling directly.
