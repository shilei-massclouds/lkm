# BootInitFlow rest_init 子阶段编码

本文件是 `BootInitRestInitPhase`、`BootInitScheduleHandoffPhase` 和恢复路径
`BootIdleEntryPhase` 的 coding 权威来源。前两个阶段是 `BootInitFlow` 的直接子阶段；
`BootIdleEntryPhase` 是 `BootIdleFlow` 的子阶段，不属于 BootInitFlow 或 KernelInitTask 执行链。

## 生命周期与 continuation

三个阶段都使用 `Base -> Prepared -> Ready -> Online`。对象动作只在 Preset 执行；Setup 和
Enable 复查同一组长期事实并发布状态。`Started` 只记录 Preset start，不是状态。

| 子阶段 | Preset 对象动作与 context | checkpoints | Online continuation |
| --- | --- | --- | --- |
| BootInitRestInit | RCU start、PID 1/kthreadd 创建、SystemState、completion；`KthreaddReadyGate.Complete` 保持在 `KthreaddReadyGateWaitLockContext` | Started -> Prepared -> 既有 Ready -> Online | `boot_init_flow::setup_after_boot_init_rest_init()` |
| BootInitScheduleHandoff | `BootIdlePreemption.EnableNoResched`、`BootIdleFlow.Setup` 和完整首次切换预检；不执行 `Scheduler.schedule()` | Started -> Prepared -> 既有 Ready -> Online | `boot_init_flow::enable_after_boot_init_schedule_handoff()` |
| BootIdleEntry | 仅在 scheduler 将来恢复 BootTask 后，于 `BootIdleStartupContext` 中进入 runtime、prepare entry 与代表性 idle loop | Started -> Prepared -> 既有 Ready -> Online | 进入 BootTask 的不返回 idle loop，不回调 BootInitFlow |

每个 Online continuation 只返回所属父 transition，不启动 sibling。BootInitScheduleHandoffPhase
提交 Online 时只允许查询 pre-commit readiness：PID 1/kthreadd runnable、completion、BootIdleFlow
Ready/owner/active binding 和 scheduler 输入均已闭合；first-schedule、pick/switch/current-task 与
stack-switch committed 只能由随后真实 `Scheduler.schedule()` 发布。

BootInitFlow 提交 Online 后物理实现直接调用真实 scheduler handoff；逻辑上该 action 仍由
Kernel.Enable 驱动，Kernel 保持 Ready。
`kernel_init_entry()` 验证真实 `sp` 后直接调用 `phases::smp_runtime::start_kernel_init_flow()`；该入口
检查 Kernel Ready/Enable accepted、BootInitFlow Online、KernelInitTask OnCpu、唯一 entry count 与 SP
verification，再启动 SmpRuntime。真实 schedule 调用将来返回到 BootTask 时才启动
BootIdleEntryPhase；线性模型中的 current TaskRef 标签不构成物理 Rust stack 归属证据。

## 已迁移实现规则

#### Model path

The rest_init path is split into BootInitRestInitPhase,
BootInitScheduleHandoffPhase and BootIdleEntryPhase under
spec/model/phases/boot-init/rest-init/. No RestInitPhase wrapper object,
state or checkpoint may be modeled; rest_init() remains only the
Linux control-flow name for the owner-split path.

#### Code path

Phase source layout follows each parent Flow. `BootInitRestInitPhase` is in
`impl/arceos_ex/src/flows/boot_init_flow/rest_init.rs`,
`BootInitScheduleHandoffPhase` is in
`impl/arceos_ex/src/flows/boot_init_flow/schedule_handoff.rs`, and
`BootIdleEntryPhase` is in `impl/arceos_ex/src/flows/boot_idle_flow/entry.rs`.
`BootIdleFlow` core lives in `flows/boot_idle_flow/mod.rs`.

#### Ordering

BootInitRestInitPhase must run after ProcessPreparePhase.Online;
BootInitScheduleHandoffPhase then opens task-concurrency and closes the
pre-commit boundary. BootInitFlow.Online is reached after those two direct
subphases reach Online and before the first scheduler handoff. BootIdleEntryPhase
records only the later restored boot-idle continuation; no extra RestInitPhase
wrapper reports their readiness.

#### Task creation facts

The implementation must publish explicit object facts for PID 1
KernelInitTask and KthreaddTask creation, scheduling eligibility and
kthreadd provider binding.

BootInitRestInitPhase owns the lifecycle-driving sequence for both roles. Its
private phase flow must call the shared Task core in this order; the structural
`Task::preset`/flow binding and `copy_process` together lower the Task Preset
operation, while `Task::setup` has no current business action:

```text
Task::preset
TaskFlow::bind (structural initial-flow association)
TaskCreationCore::copy_process
Task::setup
pi-lock -> select runqueue -> Task::set_task_cpu -> enqueue
Task::enable (initial Flow remains Base until the Task first runs)
```

KernelInitTask CPU pinning follows its successful enable; KthreaddTask global
reference publication follows its successful enable. The phase must preserve
the existing checkpoint IDs, names, Linux markers and fail-stop behavior at
these boundaries. `KernelInitTask` and `KthreaddTask` Rust role structures must
not expose `preset`, `setup` or `enable`, including forwarding aliases. They may
retain role metadata/query APIs, controlled access to the shared `Task` core,
and metadata-only commit helpers for entry/provider/clone facts.

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
enable_no_resched() and BootIdleFlow successor binding, BootInitFlow commits Online,
and Kernel.Enable then calls Scheduler.schedule(); a later restored BootTask
enters BootIdleEntryPhase's post-schedule BootIdleStartupContext. It must not be
implemented as a single Scheduler action and must not introduce a
KernelInitDispatchGate lifecycle object; the branch point is the
combination of Scheduler first-schedule and KernelInitTask dispatch
facts. Scheduler.schedule() must resolve the `CurrentTask` capability from the
effective Flow, validate that Flow's `CpuRef`, and then pick next from
`CurrentRunQueueRef`. The prepare boundary confirms that the resolved
`CurrentTaskRef` is `prev_ref`; it must not infer current identity from
Scheduler counters, `BootRunQueue.curr`, or another stored copy.

SwitchTo must synchronously send `prev.Suspend`, save the old context, commit
the architecture switch with next's canonical `tp`, and perform the physical
stack switch. On the new stack, finish first validates the raw implementation
identity and then atomically commits next's `OnCpu/Live` state, active Flow,
Flow CPU assignment and the resulting `CurrentTask` selection. Only the
new Task's entry/resume point may handle `next.Continue`: a Base initial Flow
accepts strict Startup, otherwise the Online active Flow accepts strict
Continue. Exactly one handler must accept; rejection or handler failure fails
the root execution. Resolving `CurrentTask` after finish must yield next; the
previous Task cannot be reclaimed before that result is established. Identity
switches preserve the same selector result.

Scheduler lifecycle belongs to SchedInitPhase. RestInit must consume
Scheduler.Online; the real Scheduler.Action::Schedule is driven by Kernel.Enable only after
BootInitFlow.Online. It must
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
resolve `CurrentTask` and require it to still target BootTask
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

#### CPU and CpuGroup ownership

Generated Rust must implement one `CpuGroup` with `[Option<Cpu>; MAX_CPUS]` as the sole authoritative CPU instance store. `CpuGroup.cpus[logic_id]` is the canonical identity; `BootCPU` and AP names are role aliases only. `CpuView`, parallel `cpu_refs`, `SecondaryCpuStore`, and an independently owned boot CPU are forbidden. Logical ID is derived from the array index, hartid is stored in `Cpu`, and possible/present/active/online masks are derived caches rather than identity stores. Insertion validates key type, bounds, duplicate keys and hartid uniqueness before publishing the child and parent update atomically.

`CpuRef` lowers to a compact logical ID and dereference must validate that the indexed element exists. `TaskFlow` is the sole owner of CPU assignment; `Task` has no synonymous CPU field. Only entry and scheduler commit boundaries write `TaskFlow.cpu_ref`. A Flow retains its assigned/last CPU when it is not OnCpu; migration changes the ref at commit, and handoff copies it before the successor Flow becomes active.

`CurrentCpu` is a stateless capability created only after resolving `CurrentTask`, reading its validated active Flow and dereferencing that Flow's `cpu_ref` against `CpuGroup`. It is not stored as an object and has no lifecycle. Synchronous helper/drives calls may borrow it; asynchronous emits receive no inherited capability. Trace output must name both canonical target and source Flow/CpuRef. `Context` owns `CpuGroup` only; CPU-local interrupt control, registers and scheduler-local state are reached through the selected `Cpu`.

AP entries may exist as possible/present before bringup, but no AP `CurrentCpu` capability exists until an AP Flow has execution authority and a valid CpuRef. AP entry must establish the idle Task's canonical `tp` before either `CurrentTask` or `CurrentCpu` is resolved. Smoke and checkpoint coverage must validate CPU0 ownership, AP identity, CpuRef dereference, no parallel identity stores, derived masks, logical-ID/hartid bijection, migration and Flow handoff.

#### Current TaskRef scope

`CurrentTask` is a short-lived, read-only capability whose only payload is a
generation-checked `TaskRef`; `CurrentTaskRef` is the typed reference derived
from that selector. Neither name is a role enum, object, lifecycle, writable
slot, or snapshot state, and no compatibility alias or global current-task
singleton is permitted. Resolution starts from the stable RISC-V64 `tp`
implementation identity, maps it through one centralized Task carrier
resolver, then validates that the target is the unique `OnCpu/Live` Task, its
active Flow is the effective Flow, and the Flow parent/owner points back to the
same Task. The resolver covers BootTask, kernel tasks, smoke/user dynamic tasks
and AP idle tasks, rejects unknown addresses and stale generations explicitly,
and exposes neither raw addresses nor mutable Task borrows to ordinary callers.
For BootTask the physical and virtual `tp` resolve to linker-visible
`init_task_storage`. No fallback to BootTask, runqueue state, or Scheduler
caches is allowed.

#### CurrentRunQueueRef scope

CurrentRunQueueRef must be realized as a private reference in the
current CPU view. It must not be implemented as a descriptive
current-runqueue object or as a global current-runqueue singleton. Code
should follow the Linux-style path: resolve the current `TaskRef`, read
the validated Task's recorded CPU id, then resolve that
CPU's runqueue through CPUGroup/runqueue topology. The current BP
implementation may collapse this to the boot runqueue while marking
that binding as a temporary UP specialization.

#### CurrentRunQueueRef topology lowering

Current Rust lowering must carry the resolved CPU id inside
CurrentRunQueueRef even while the only concrete target is
BootRunQueue. Scheduler.schedule() lowering must derive that CPU id
from the TaskRef target Task's recorded CPU id, then validate
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
it from TaskRef -> validated Task -> CPU id -> CpuGroup.Cpu[id].RunQueue.
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
non-idle TaskRef in the payload path and requires switch_to to
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

PID 1 boot CPU pinning must be implemented by BootInitRestInitPhase invoking
the shared Task-level `pin_to_boot_cpu` action on KernelInitTask's core, then
committing any role observation as metadata only. The action sets the
PF_NO_SETAFFINITY-equivalent flag and cpumask facts. It must not be used as the
wake-up set_task_cpu action and must not be represented by an independent
KernelInitAffinity lifecycle object. The RCU read-side boundary
around the Linux pid lookup remains a deferred context-modeling
question, not a completed resource-exclusive context.

#### Fork dependency

PreSmpInitPhase must depend on BootInitFlow.Online in addition to
the KernelInitTask release/dispatch facts and Scheduler first-schedule
fact. It must not infer readiness from BootIdleEntryPhase.Ready or a
nonexistent RestInitPhase wrapper.

#### Real BootIdle to KernelInit stack handoff

The implementation may linearize the owner-split rest_init object and
checkpoint facts, but leaving BootInitFlow must perform one real
cooperative context transfer from BootTask to KernelInitTask. The
handoff saves BootTask's `ra/sp/tp/s0..s11`, restores the initialized
KernelInitTask context on its vmalloc stack, and enters `kernel_init_entry()`.
That entry must call the named Kernel.Enable continuation, which validates
the owner and stack facts before driving KernelInitFlow's direct execution phases. If a later
schedule restores the BootIdle continuation, it remains in its active idle
schedule loop. If KthreaddTask is selected, it remains in its temporary active
schedule loop. Neither continuation may execute the selected payload.

#### Boot stack size after handoff

Once the real BootTask to KernelInitTask handoff is enabled and the
KernelInit entry verifies that it is running on its own 16 KiB vmalloc stack,
the generated boot stack must use the codegen profile's 16 KiB size. This
reduction is valid only while KernelInitFlow's execution phases remain owned by
KernelInitTask; moving that path back to the boot stack requires re-auditing
the boot stack bound before changing the linker profile.

#### Deferred runtime

Secondary CPU bringup, workqueue workers, Tasks RCU GP kthreads,
KernelInitTask.kernel_init_freeable() and kthreadd request
consumption remain later-phase work.
