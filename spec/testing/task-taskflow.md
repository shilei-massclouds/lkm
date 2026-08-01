# Task / TaskFlow testing contract

This file is the authoritative testing contract for the unified `Task`
carrier and its independently-lived `TaskFlow` instances.

## Carrier and flow boundaries

- Boot and scheduler tests must observe one `BootTask` identity. The
  `BootIdleSetup` scheduling record is an internal projection and must not be
  reported as a second Task or own a second Task lifecycle.
- Entry tests must observe `BootTask.OnCpu` exactly once with the `T` early
  marker, in `Kernel.Started -> BootTask.OnCpu -> PhysicalDirect.ActivatedOnCpu ->
  BootInitFlow.Started -> InterruptType.Prepared` order. PhysicalDirect, Started and
  InterruptType use the stable early bytes `D -> O -> I`, each exactly once.
  Physical and virtual BindTask happen later, each must perform a real `tp` write,
  do not change BootTask lifecycle, and both must resolve to the same
  linker-visible `init_task_storage`/`TaskRef::BOOT` carrier. The first call
  establishes the CPU binding, the second refreshes its address representation
  without changing binding identity, and neither initializes preempt count.
- The complete Kernel.Enable boundary must preserve
  `Kernel.Started < PhysicalDirect.ActivatedOnCpu < BootInitFlow.Started < Scheduler.Schedule <
  PayloadHandoffPreparePhase.Online < Kernel.Online <
  KernelInitFlow.PayloadHandoffCommitted < application entry`. Kernel must
  remain Ready through the handoff-prepare checkpoint, and Hello, Smoke and
  UserBoot acceptance must each observe exactly one `Kernel.Online`.
- BootInitFlow must expose `TaskFlowRef::BOOT_INIT`, owner/parent BootTask and
  `BootTask.initial_flow == BOOT_INIT`; it must not expose a stored guard field.
- BootInitFlow tests must observe the standard Started/Prepared/Ready/Online
  lifecycle and must not observe BP EntryPrelude checkpoints. Preset must drive
  the original entry-object order directly and commit Prepared only after the
  complete entry facts hold. The boot scheduling endpoint is the stable boundary
  after BootInitFlow commits Online and immediately before BootIdleFlow emits
  `Cpu0Scheduler.Schedule`; at that boundary BootTask remains OnCpu and no
  Schedule Signal, PreparePrev or switch occurrence exists. BootIdleEntry may
  start only after a later scheduler call restores BootTask.
- Execution-boundary coverage must prove that a Flow process proceeds only
  when its parent Task is OnCpu/Live. Reserved authority, stale Flow generation or another mismatch must fail without changing Flow
  lifecycle state. CurrentTask must first read the CPU-local binding and resolve
  only when parent/owner, active Flow, same-CPU OnCpu/Live authority and the
  unique TaskRef generation all agree. CurrentCPU must still resolve from
  BootInitFlow.cpu_ref before the first binding.
- CopyProcess coverage must accept an OnCpu/Live source whose validated TaskRef
  is the derived `CurrentTaskRef`, and reject Online, Reserved, non-current and mismatched
  TaskRef sources without changing TaskCreationCore or destination state.
- CurrentTask, CurrentTaskRef and CurrentStack tests must prove that none has
  lifecycle, owned storage, a slot, system state or instance entries. Stack must remain
  a `Task.stack` value attribute rather than an object. Snapshot coverage must retain
  only the CPU-keyed task/stack contextual binding facts.
  Synchronous continuations inherit the updated binding/effective Flow;
  asynchronous emits resolve independently.
- Binding focused coverage must exercise boot-only `BindTaskStack` first binding and
  `RefreshTaskStack` same-pair refresh, pre-bind CurrentTask/CurrentStack rejection,
  post-bind CurrentTask/CurrentTaskRef/CurrentStack resolution, general scheduler-only
  `BindTask(next)`, and independent bindings for two CPUs. Reject wrong parent/owner,
  active-flow, CPU, OnCpu/Live, TaskRef, stack argument/controller and non-switch
  boundaries. Every rejection must preserve the exact before snapshot, proving the
  special Actions never expose a half-bound task/stack pair. No case may introduce a
  public BindStack Signal, slot or Stack object semantics.
- Model-tool coverage must exercise multi-level lifecycle inheritance: cumulative
  conditions/facts, base-to-derived drives, post-commit emits, duplicate-side-effect
  rejection and complete override. The legacy and tools2 pipelines must produce the
  same effective ordering for the shared fixture.
- TaskFlow trace coverage must observe exactly one Setup and one Enable completion
  Signal for each Flow lifecycle, preserve BootInitFlow's entry-object order, and
  continue through rest-init, first dispatch and later Flow startup.
- Every executable TaskFlow test must observe its sole `cpu_ref` association;
  the parent Task must not expose a synonymous CPU field. Entry acceptance and
  scheduler migration commit are the only writers. Flow handlers and their
  synchronous `drives` descendants may read the inherited effective CpuRef,
  while asynchronous `emits` handlers must fail if they attempt to inherit it.
- A Flow leaving OnCpu retains its assigned/last-owner CpuRef. Migration tests
  must change it exactly at scheduler commit. Flow handoff tests must copy the
  old Flow's CpuRef to the successor before the active binding changes or the
  successor can be enabled.
- KernelInitTask, KthreaddTask and clone-child Setup must leave a Prepared context and their initial
  Flow in Base. Enable binds the context to the exact initial FlowRef and publishes Online/Valid without
  sending Startup. The first real switch must validate/consume it and start the initial Flow.
- Repeated switches to an already-started Task must select the active Flow's
  strict Continue handler without resending initial-flow Startup; BootTask must
  resume BootIdleFlow without restarting BootInitFlow.
- PID 1 tests must keep `KernelInitTask` online across exec while observing
  an explicit fresh-Flow declaration that remains in `Base` before Preset, then
  `KernelInitFlow: Online -> Offline -> Destroyed` and
  `UserAppFlow: Base -> Prepared -> Ready -> Online` in that order.
- The handoff is valid only when the old flow is inactive before the active
  binding changes, the new flow is Ready at commit, and at most one owned flow
  is Online afterward. A post-Kernel.Online emitted handoff failure must fail
  the root result without rolling Kernel back or submitting Online again.
- Identity schedule must not emit Task Suspend/Continue, change lifecycle/authority/breakpoint, or increment
  physical context save/restore counts; it emits Continue only to the original active TaskFlow. Terminal switch must take OnCpu directly to Offline and cleanup
  on the next stack without publishing an Online breakpoint.
- BootTask begins OnCpu/Live/Invalid; its first Suspend binds BootIdleFlow and publishes its first Valid
  breakpoint. Each AP begins OnCpu/Reserved/Invalid with Base ApIdleFlow; HSM preserves TaskRef/FlowRef,
  activates Live authority and starts the matching logical-id Flow without Task Enable/Continue.
- RISC-V sentinel coverage must save/restore `ra/sp/s0..s11` and prove formal switch commit writes `tp/x4`
  from next Task identity rather than from `TaskSwitchContext`.
- Linux PLIC foreign-ABI coverage must prove that ordinary completion, an error return through the Rust
  IRQ-domain bridge, and a nested foreign call each restore the exact entry `tp`; Rust Context and
  CurrentTask resolution may run only while the saved canonical Task `tp` is active.
- User entry checkpoints use `UserAppFlow.EnterUserMode`. Old Task,
  persona, and Flow checkpoint names are not compatibility interfaces.

## Fork and bounded storage

- Every successful fork/clone observation must receive a monotonically fresh
  PID and logical allocation occurrence. A released internal task record may
  be recycled only after the prior task's exit/wait cleanup facts are complete;
  recycling storage must never reuse the prior Task identity or lifecycle.
- Tests must distinguish `UserTaskSet.state()` from the active task record's
  lifecycle and must not describe the implementation as a reusable child
  slot. Sequential storage is a bounded implementation constraint, not the
  object model's multiplicity rule.
- Nested fork/vfork smoke coverage must assert that the child identity differs
  from its parent and that the allocation occurrence advances.

## Gates

Focused runtime coverage includes kernel smoke and user smoke. Checkpoint
inventory/mapping/coverage/marker checks must be current after a checkpoint
rename. The final regression gate after implementation changes is the direct
repository-root `make test` command.
