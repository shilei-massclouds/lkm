# UpMultitask rest_init 子阶段编码

本文件是 `BootInitRestInitPhase`、`BootInitScheduleHandoffPhase` 和 `BootIdleEntryPhase` 的
coding 权威来源。原 `rest-init.spec` 的有效实现规则已全部归并到本文，不再维护平行 predicate
索引。

## 生命周期与 continuation

三个子阶段都使用 `Base -> Prepared -> Ready -> Online`。对象动作只在 Preset 执行；Setup 和
Enable 复查同一组长期事实并发布状态。`Started` 只记录 Preset start，不是状态。

| 子阶段 | Preset 对象动作与 context | checkpoints | Online continuation |
| --- | --- | --- | --- |
| BootInitRestInit | RCU start、PID 1/kthreadd 创建、SystemState、completion；`KthreaddReadyGate.Complete` 保持在 `KthreaddReadyGateWaitLockContext` | Started -> Prepared -> 既有 Ready -> Online | `up_multitask::preset_after_boot_init_rest_init()` |
| BootInitScheduleHandoff | `BootIdlePreemption.EnableNoResched` 与 Scheduler 既有 action context 中的首次 `Schedule` | Started -> Prepared -> 既有 Ready -> Online | `up_multitask::setup_after_boot_init_schedule_handoff()` |
| BootIdleEntry | `BootIdleStartupContext` 中进入 runtime、prepare entry 与代表性 idle loop | Started -> Prepared -> 既有 Ready -> Online | `up_multitask::enable_after_boot_idle_entry()` |

每个 Online continuation 只返回父 transition，不启动 sibling。`dispatch_ready()` 保留为调度事实
查询：BootInitScheduleHandoffPhase 提交 Online 时必须仍满足既有 first-schedule、
pick/switch/current-task 与 completion dispatch invariant，并在同一边界锁存长期 dispatch fact；
后续查询同时要求该阶段精确 Online 和锁存事实，不用已经发生后续调度的 current-task 瞬时值重算历史。

BootIdleEntry Online 返回父 Enable continuation；父提交 UpMultitask Online 后才调用
`handoff_boot_idle_to_kernel_init()`。`kernel_init_entry()` 验证真实 `sp` 后调用具名
`kernel::enable_after_up_multitask()`，该 continuation 检查 Kernel Ready、UpMultitask Online、
KernelInitTask Online、唯一 entry count 与 SP verification，再启动 SmpRuntime。线性模型中的
`CurrentTaskRef` 标签不构成物理 Rust stack 归属证据。

## 已迁移实现规则

#### Model path

The rest_init path is split into BootInitRestInitPhase,
BootInitScheduleHandoffPhase and BootIdleEntryPhase under
spec/model/phases/up-multitask/rest-init/. No RestInitPhase wrapper object,
state or checkpoint may be modeled; rest_init() remains only the
Linux control-flow name for the owner-split path.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the up-multitask phase
subtree, for example impl/arceos_ex/src/phases/up_multitask/rest_init.rs.

#### Ordering

BootInitRestInitPhase must run after ProcessPreparePhase.Online;
BootInitScheduleHandoffPhase then opens task-concurrency through the
first scheduler handoff; BootIdleEntryPhase records the boot idle
continuation. UpMultitaskPhase.Online is reached only after those
three concrete subphases each reach Online; no extra RestInitPhase
wrapper reports their readiness.

#### Task creation facts

The implementation must publish explicit object facts for PID 1
KernelInitTask and KthreaddTask creation, scheduling eligibility and
kthreadd provider binding.

#### Explicit task entries

KernelInitTask must be created through TaskCreationCore with
TaskEntry::KernelInit, and KthreaddTask through TaskEntry::Kthreadd.
The selected entry determines the task's first execution line:
KernelInitTask enters the PreSmpInitPhase chain, while KthreaddTask
enters the kthreadd service loop boundary.

#### Kthreadd entry loop

BootInitRestInitPhase must publish KthreaddTask entry/provider facts
but must not execute the kthreadd loop on the BootIdle owner line. When
KthreaddTask is selected, its temporary entry loop repeatedly calls the
ordinary scheduler boundary to yield the CPU. The number of completed loop
iterations is not a boot acceptance condition. Consuming kthread creation
requests and the full wait/park/stop protocol remain deferred until a
KthreaddTask-owned runtime phase exists.

#### System state

rest_init() must publish SystemState.value == SYSTEM_SCHEDULING and
the opening of task-concurrency semantics, without implying SMP.

#### Completion

complete(&kthreadd_done) must drive the reusable Completion object
carried by KthreaddReadyGate and publish the visible gate fact. It
must not directly release KernelInitTask; the release is observed by
KernelInitTask's wait side.

#### Scheduler dispatch facts

schedule_preempt_disabled() must be split across the owner boundary:
BootInitScheduleHandoffPhase performs BootIdlePreemption
enable_no_resched() and Scheduler.schedule(); BootIdleEntryPhase
enters the post-schedule BootIdleStartupContext. It must not be
implemented as a single Scheduler action and must not introduce a
KernelInitDispatchGate lifecycle object; the branch point is the
combination of Scheduler first-schedule and KernelInitTask dispatch
facts. Scheduler.schedule() must derive the
current task reference from the current CPU current-task view,
pick next from CurrentRunQueueRef, then switch through TaskRef-based core
context save/restore and publish the updated CPU-local current task
fact. The implementation boundary must pass through the current
CPU's CurrentTaskSlot; it must not infer or publish the current task
only from Scheduler counters or BootRunQueue.curr.

Scheduler lifecycle belongs to SchedInitPhase. RestInit must consume
Scheduler.Online and drive Scheduler.Action::Schedule only; it must
not create a new Scheduler lifecycle boundary for dispatch. The
schedule action must remain covered by the nested within sequence
SchedulePreemptionContext -> ScheduleLocalInterruptContext ->
ScheduleRunQueueContext, and the wake-up path must keep task pi lock
and runqueue lock coverage in WakeUp*TaskContext /
EnqueueSelectedRunQueueContext. kthreadd_done remains a Completion
Type process, and BootIdleEntryPhase remains covered by
BootIdleStartupContext.

#### Boot idle runtime actions

BootIdleRuntime.setup() must only establish the Ready object shell
after scheduler dispatch facts exist. It must not collapse
PrepareIdleEntry, RunIdleLoop and DoIdleCycle into one setup-time
fact update. The phase code must explicitly drive
BootIdleRuntime.prepare_idle_entry(), then
BootIdleRuntime.run_idle_loop(), with run_idle_loop() committing one
representative do_idle_cycle() boundary for finite model observation. The
runtime continuation itself must remain an unbounded loop that can repeatedly
enter schedule_idle(); no exact loop or switch count is part of boot acceptance.

#### Representative need_resched idle cycle

BootIdleRuntime (the `BootIdleFlow` lowering) must expose the three model action
hooks WaitWhileNoNeedResched, ObserveNeedResched and
ScheduleIfNeedResched as named implementation boundaries. The first
boundary records that the boot idle task enters an abstract
no-need-resched wait state with polling/nohz details deferred; the
second records that the CPU-visible environment sets need_resched and
the idle task leaves the wait state; the third records a
schedule_idle request/return and drains the need_resched fact. This
remains an object-level representative cycle. The implementation repeats this
scheduler boundary, while real timer/IRQ wakeup sources and the cpuidle/WFI
path remain deferred.

#### schedule_idle wrapper

BootIdleRuntime.schedule_if_need_resched() must now drive a concrete
Scheduler.schedule_idle() implementation boundary. The wrapper must
require the current CPU's CurrentTaskSlot to still target BootTask
and the need_resched observation to have been recorded by
BootIdleRuntime. It must reuse the existing Scheduler.schedule()
pick-next/switch-to skeleton and must not hand-commit a new Task identity or a second idle carrier when runnable tasks are
present. It must publish idle-specific counters/facts separately
from ordinary schedule() calls so smoke/KUnit coverage can distinguish the
idle path. The finite object trace need not expand the full Linux do {
__schedule(SM_IDLE); } while (need_resched()) loop, tick/nohz detail, or
sched_submit_work() skip details; the Rust continuation must nevertheless
remain schedulable for an unbounded number of iterations.

#### Action lowering ABI

Coding/codegen may lower model actions with explicit parameters and
return bindings to a uniform Action(ContextRef, MutPacketRef)
implementation ABI. ContextRef is the object graph entry; MutPacketRef
is a strongly typed, local, schema-explicit packet for temporary
values passed between peer actions. The formal model must still keep
explicit action parameters, return values and let bindings. Packets
must not store persistent object facts. With this ABI, every action
entry and exit is a potential checkpoint, and internal action
boundaries may expose packet fields to checkpoint/KUnit. Object
methods must not fetch the global Context themselves.

#### Scheduler action checkpoints

Scheduler.schedule() checkpoint/KUnit coverage must proceed from
the front of the action chain. First check PickNextTask exit: next_ref
has been produced and, for the first rest_init schedule, prev_ref
targets BootTask while next_ref targets KernelInitTask or
KthreaddTask; the current implementation deterministically prefers
KernelInitTask. Then check SwitchTo entry: the recorded
prev_ref/next_ref match the pick result and the checkpoint observes
the boundary before the current-task switch commit for this
invocation. Then check SwitchTo exit: for the first rest_init
schedule return, CurrentTaskRef must target the same runnable task
selected by PickNextTask, namely KernelInitTask or KthreaddTask. The
coarser Scheduler.Schedule.Exit postcondition checkpoint is checked
after local_irq_restore(): for the first rest_init schedule return,
CurrentTaskRef must still target the selected runnable task,
schedule_passes must be committed, and local interrupt
save/restore counts must be balanced for this invocation.

#### BootIdleEntryPhase boot-idle chain

The phase implementation must present the boot-idle tail chain
directly in phase order: enter BootIdleStartupContext, then
BootIdleRuntime.setup(), BootIdleRuntime.prepare_idle_entry(),
BootIdleRuntime.run_idle_loop(), then the BootIdleEntryPhase Prepared,
Ready and Online checkpoints. It may use one small helper for each named action, but
it must not hide the whole chain behind a single setup_boot_idle_tail()
helper or collapse the model action order into one opaque phase call.

#### Smoke/KUnit coverage

The rest_init smoke case and the checkpoint KUnit smoke reuse must validate
the boot idle schedule relation and the KernelInit payload owner, but must not
require an exact number of BootIdle, Kthreadd or context-switch iterations.
Counter observations may prove that a boundary was reached; they are not
default `make run` acceptance criteria.

#### CPU instance model

The implementation must realize the model as one reusable CPU object
type with one instance per logical CPU. BootCPU is the logical-id-0
CPU instance with a bootstrap role, not a separate CPU type.

#### CpuGroup indexing

CpuGroup must expose a logical-id indexed CPU reference view:
CpuGroup.Cpu[0] targets BootCPU and later entries target secondary
CPU instances. Generated code must not create a separate CpuIdMap
object; CpuGroup itself carries the index and possible CPU boundary.

#### CpuGroup ownership

CpuGroup organizes CpuRef indexes, topology and possible/present/
online set views. It must not own CPU bodies, and generated code
must not model PossibleCpu/PossibleRunQueue as separate owning CPU
objects. A set element is a CPU reference.

#### CPU state facts

hartid, logical_id, possible, present, active and online belong to
the CPU instance. CpuGroup maintains set views over CPU references
for possible/present/online membership.

#### AP CurrentCPU boundary

A secondary CPU may be present in CpuGroup's possible/present views
before bringup, but generated code must not create a live AP
CurrentCPU, LocalInterruptControl, CurrentTaskSlot or
PreemptionControl chain before that AP enters secondary entry.
Once generated, those live AP facts must be tied to
ApEntryPreludePhase/ApSmpCallinPhase/ApOnlineIdlePhase, not to
possible/present membership or BP HSM request issuance alone.

#### DefaultSchedRootDomain coverage

DefaultSchedRootDomain must be generated as the scheduler's default
root-domain coverage view. It must derive covered_cpus from
CpuGroup.possible_cpus, store/resolve entries as CpuRef values, and
must not own CPU bodies or define a separate CPU identity table.
Naked possible CPU counts are insufficient as the formal model fact.

#### RunQueue root-domain attach

RunQueue setup for each possible CPU must attach to
DefaultSchedRootDomain only after the runqueue CPU reference is known
to be covered by DefaultSchedRootDomain.covered_cpus. The boot
runqueue must expose or resolve BootCPURef as its CPU reference.
Secondary runqueue metadata may be prepared before AP online, but
that must not create a live AP CurrentCPU or runnable AP flow.

#### sched_init wait-bit/radix/maple synchronization facts

BitWaitQueueTable lowering must expose wait_bit_init() as a scoped
boot initialization of every bit_wait_table bucket's wait_queue_head:
the bucket count matches WAIT_TABLE_SIZE, bucket wait queues are
ready, their internal spinlocks are initialized, and their lists are
empty. These facts must be guarded in the model with a
within BitWaitQueueTableInitContext block rather than represented as
loose, unscoped ensures. RadixTree and MapleTree setup must keep
their runtime call_rcu() node-free callbacks explicitly deferred;
node_api_ready must not be read as meaning those callbacks executed
during radix_tree_init()/maple_tree_init().

#### sched_init workqueue early synchronization facts

workqueue_init_early() must be represented as the first workqueue
stage only: system workqueues and queue/cancel data structures are
prepared, but workers do not run. The lowering must register
KMEM_CACHE(pool_workqueue, SLAB_PANIC) as a PoolWorkqueue named
cache in SlubCacheRegistry. It must model wq_pool_mutex and
workqueue_struct->mutex as explicit guard scopes using
within WorkqueuePoolMutexContext { ... } and nested
within WorkqueueStructMutexContext { ... }. Worker attach/detach,
mayday/rescuer locking and manager_wait behavior remain deferred to
the later worker-runtime phases.

#### sched_init softirq/RCU/tracing boundaries

Softirq.Preset in SchedInitPhase must only create the action-table
and per-CPU pending-bit shell needed by rcu_init(); Linux
softirq_init(), tasklet queues, TIMER_SOFTIRQ and HRTIMER_SOFTIRQ
registration occur after early_irq_init() and are driven by
IrqTimeInitPhase. RcuCore.setup() must explicitly register
RCU_SOFTIRQ against Softirq instead of treating action_table_ready
as sufficient. It must expose rcu_init()'s TREE_RCU node tree
locks/waitqueues/work, per-CPU rcu_data binding, kfree_rcu batch
workqueue/shrinker setup, PM notifier registration, and
tasks_cblist_init_generic() per-flavor/per-CPU callback-list,
lock, work, and barrier-head facts. RCU GP kthreads, callback
execution and full RCU read-side/context-tracking semantics remain
deferred. The active .config enables FTRACE/TRACING, CPU_ISOLATION
and CONTEXT_TRACKING/CONTEXT_TRACKING_IDLE, but not
CONFIG_FTRACE_MCOUNT_RECORD or CONFIG_CONTEXT_TRACKING_USER_FORCE.
Therefore ftrace_init() and context_tracking_init() are
trimmed/no-op call points for the current RISC-V64 target, while
early_trace_init()/trace_init() and housekeeping_init() remain
explicit deferred boundaries whose Linux responsibilities must not
be collapsed into the project checkpoint announce or ignored as
permanently absent. The implementation must preserve the Linux call
order inside the existing SchedInitPhase: poking_init()/ftrace_init()
are recorded before Scheduler setup via SchedInitPreludeTrimmedPaths,
while trace_init()/context_tracking_init() are recorded after
rcu_init() via SchedInitTraceContextBoundaries. These are boundary
objects, not formal subphases.

#### CPU-owned RunQueue/IdleTask

Generated model and code comments must present RunQueue and IdleTask
as objects owned by the corresponding CPU instance:
CpuGroup.Cpu[id].RunQueue and CpuGroup.Cpu[id].IdleTask. Scheduler
may orchestrate setup and policy, but must not be treated as owning
every CPU's runqueue or idle task body. Current Rust lowering may
temporarily store BootRunQueue and boot-task scheduler metadata inside Scheduler fields
only if public facts and smoke checks expose them as BootCPU views of the same `BootTask`.

#### Boot scheduler lock ownership

BootRunQueueLock must be lowered as the lock owned by the
BootCPU-owned BootRunQueue object, and BootIdlePiLock must be
lowered as the pi_lock owned by the BootCPU-owned BootTask
object. Scheduler.setup() may orchestrate init_idle() ordering, but
must not become the semantic owner of those locks. Public readiness
checks may expose transitional Scheduler accessors only as
projections back to BootRunQueue.lock and BootTask.pi_lock.

#### CPU-owned scheduler view lowering

While BootRunQueue and boot-task scheduler metadata are still stored inside the
Scheduler object, generated Rust must expose a formal boot CPU view
of that storage. CpuOwnedSchedulerView is the public implementation
surface for CpuGroup.Cpu[0].RunQueue and CpuGroup.Cpu[0].IdleTask;
CpuIdleTaskView is the public idle-task half of that view. These
views must be derived from CpuGroup.Cpu[0], BootRunQueue and
BootTask facts, must confirm BootRunQueue.curr/idle both point
at BootTask, and must reject mismatched CPU refs or hart ids.
They are not test-only wrappers, and smoke must check them directly.
Core object implementations that only need the boot CPU-owned
RunQueue/IdleTask facts must consume CpuOwnedSchedulerView instead
of directly treating Scheduler.boot_runqueue() or
Scheduler.boot_task_metadata() as the formal ownership source. Direct
accessors may remain as transitional storage/debug observation
surfaces and for BootRunQueue/BootTask-local APIs, but not as the
primary readiness predicate in rest_init task setup/enable paths.
RestInit phase predicates and checkpoint/KUnit handlers that verify
boot CPU runqueue membership or task count must consume read-only
membership/count facts projected by CpuOwnedSchedulerView, not
re-read Scheduler.boot_runqueue() as the formal observation source.
RestInit task-creation helpers, including TaskCreationCore.copy_process(),
must receive enough CpuGroup context to validate the same formal
boot CPU-owned scheduler view instead of using BootRunQueue state as
an implicit scheduler-ready shortcut.
Smoke tests that assert CPU-owned RunQueue/IdleTask functional facts
must prefer CpuOwnedSchedulerView/CpuIdleTaskView observations. A
smoke test may compare against Scheduler.boot_runqueue() or
Scheduler.boot_task_metadata() only when the comparison is explicitly a
transitional storage parity check.

#### Transitional lowering

The current Rust storage may temporarily keep boot_cpu and
secondary_cpus fields for implementation convenience, but such a
split is a lowering detail. Public object facts, checkpoints and
code-generation comments must present the unified CPU instance model
and logical-id indexed CpuGroup view.

#### CPU/CpuGroup coverage

Smoke/checkpoint coverage must observe CpuGroup.Cpu[0] -> BootCPU,
boot CPU possible/present/online facts, secondary possible/present
but not-online facts, and unique logical-id/hartid boundaries.
Scheduler smoke must also observe the formal CpuOwnedSchedulerView
and CpuIdleTaskView rather than only comparing private
Scheduler.boot_runqueue()/boot_task_metadata() fields.

#### CurrentTaskRef scope

CurrentTaskRef must be realized as a private object in the current
CPU view. The BP path owns the BootCurrentCPU CurrentTaskRef and must
not introduce a descriptive CurrentTask object or a global current
task singleton. On task switch, next must become the target of this
CPU-local CurrentTaskRef. RISC-V64 code should follow the Linux-style
tp register implementation reference through the object-level
CurrentTaskSlot boundary, but per-cpu storage remains an
implementation term, not the model definition.

#### CurrentRunQueueRef scope

CurrentRunQueueRef must be realized as a private reference in the
current CPU view. It must not be implemented as a descriptive
current-runqueue object or as a global current-runqueue singleton. Code
should follow the Linux-style path: derive the current task through
CurrentTaskRef, read the task's recorded CPU id, then resolve that
CPU's runqueue through CPUGroup/runqueue topology. The current BP
implementation may collapse this to the boot runqueue while marking
that binding as a temporary UP specialization.

#### CurrentRunQueueRef topology lowering

Current Rust lowering must carry the resolved CPU id inside
CurrentRunQueueRef even while the only concrete target is
BootRunQueue. Scheduler.schedule() lowering must derive that CPU id
from the CurrentTaskRef target task's recorded CPU id, then validate
it against CpuGroup.Cpu[id], Scheduler.cpu_runqueue(id) metadata and
DefaultSchedRootDomain coverage. CpuGroup.boot_cpu() may be used only
as a boot CPU consistency check after the current task CPU id is
known; it must not be the primary source for resolving the current
runqueue. Scheduler.Action::SelectRunQueue lowering is a wake-up
selection path: it may currently select the boot runqueue, but
RestInit task enable paths must consume the selected_rq result by
setting task CPU from selected_rq.cpu_id() and passing selected_rq
into the enqueue boundary. BootRunQueue may remain the UP selected
target, but BootRunQueue enqueue/pick/dequeue APIs must reject a
CurrentRunQueueRef with a mismatched CPU id.

#### RunQueueRef / CurrentRunQueueRef type split

Generated Rust must keep selected runqueue references separate from
current-CPU runqueue references. Scheduler.Action::SelectRunQueue
lowering must return a RunQueueRef value, not CurrentRunQueueRef.
RestInit enqueue paths and smoke task enqueue/dequeue helpers must
pass RunQueueRef into BootRunQueue enqueue/dequeue APIs. Only the
schedule()/pick-next path may use CurrentRunQueueRef, after deriving
it from CurrentTaskRef -> task CPU id -> CpuGroup.Cpu[id].RunQueue.
Both reference types may currently carry the same boot CPU id in the
UP path, but sharing the enum/type is not allowed because the object
capabilities differ.

#### BootRunQueueRef transitional lowering

The model may still name BootRunQueueRef as the current UP
SelectRunQueue result, but Rust reference checks must present the
capability as a CPU-owned runqueue match: the ref's CPU id must match
the target CpuGroup.Cpu[id].RunQueue / BootRunQueue metadata. Public
implementation constructors and predicates should not expose
`targets_boot_runqueue` or `boot(...)` as the formal semantic API
for selected or current runqueue refs; use neutral CPU-owned
constructors and matching helpers instead. The boot-backed enum
variant may remain as a storage/lowering detail until SMP runqueue
variants exist.

#### CurrentRunQueueRef API smoke

CurrentRunQueueRef/RunQueue ObjectApiBehavior smoke must exercise
formal runqueue enqueue and pick-next boundaries. The implementation
must not expose test_* scheduler wrappers for these checks; if the
boundary is needed by tests, expose it as a formal RunQueue API and
route production enqueue/pick behavior through the same API. This
smoke case remains app-smoke-only by default, not checkpoint KUnit.

#### Scheduler.schedule() payload smoke

A Scheduler.schedule() smoke case that targets the API itself must be
app-smoke-only by default and run from the payload phase, where the
caller is KernelInitTask, not the rest_init boot-idle checkpoint. The
normal scenario must use a minimal cooperative switch loop:
KernelInitTask enqueues a smoke scheduler task, calls schedule() a
bounded number of times until that task's entry runs, the smoke task
records that it executed and calls schedule()/yield, and control
returns to KernelInitTask. This requires schedule() to support
non-idle CurrentTaskRef in the payload path and requires switch_to to
perform a real cooperative stack/context transfer for the smoke task.
It must not reuse or weaken the checkpoint/KUnit expectations for
the first rest_init schedule, and it must not add test_* subject APIs;
any needed boundary must be a formal scheduler/task API.

#### Wake-up task CPU action

KernelInitTask and KthreaddTask wake-up paths must follow the
Linux ordering: select the target runqueue, update the task's
recorded CPU through a Task-level set_task_cpu boundary, then
enqueue the task on that runqueue. The current BP implementation may
bind the selected runqueue CPU to BootCPU/BootCPURef, but this is a
temporary specialization; future SMP code must resolve cpu_of from
the selected RunQueueRef.

#### KernelInitTask affinity action

PID 1 boot CPU pinning must be implemented as a KernelInitTask
action that sets the PF_NO_SETAFFINITY-equivalent flag and cpumask
facts. It must not be used as the wake-up set_task_cpu action and
must not be represented by an independent
KernelInitAffinity lifecycle object. The RCU read-side boundary
around the Linux pid lookup remains a deferred context-modeling
question, not a completed resource-exclusive context.

#### Fork dependency

PreSmpInitPhase must depend on UpMultitaskPhase.Online in addition to
the KernelInitTask release/dispatch facts and Scheduler first-schedule
fact. It must not infer readiness from BootIdleEntryPhase.Ready or a
nonexistent RestInitPhase wrapper.

#### Real BootIdle to KernelInit stack handoff

The implementation may linearize the owner-split rest_init object and
checkpoint facts, but leaving UpMultitaskPhase must perform one real
cooperative context transfer from BootTask to KernelInitTask. The
handoff saves BootTask's `ra/sp/tp/s0..s11`, restores the initialized
KernelInitTask context on its vmalloc stack, and enters `kernel_init_entry()`.
That entry must call the named Kernel.Enable continuation, which validates
the owner and stack facts before driving SmpRuntimePhase and PayloadPhase. If a later
schedule restores the BootIdle continuation, it remains in its active idle
schedule loop. If KthreaddTask is selected, it remains in its temporary active
schedule loop. Neither continuation may execute the selected payload.

#### Boot stack size after handoff

Once the real BootTask to KernelInitTask handoff is enabled and the
KernelInit entry verifies that it is running on its own 16 KiB vmalloc stack,
the generated boot stack must use the codegen profile's 16 KiB size. This
reduction is valid only while SmpRuntimePhase and PayloadPhase remain owned by
KernelInitTask; moving either path back to the boot stack requires re-auditing
the boot stack bound before changing the linker profile.

#### Deferred runtime

Secondary CPU bringup, workqueue workers, Tasks RCU GP kthreads,
KernelInitTask.kernel_init_freeable() and kthreadd request
consumption remain later-phase work.
