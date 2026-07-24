# Task / TaskFlow testing contract

This file is the authoritative testing contract for the unified `Task`
carrier and its independently-lived `TaskFlow` instances.

## Carrier and flow boundaries

- Boot and scheduler tests must observe one `BootTask` identity. The
  `BootIdleSetup` scheduling record is an internal projection and must not be
  reported as a second Task or own a second Task lifecycle.
- Entry tests must observe `BootTask.OnCpu` exactly once with the `T` early
  marker, in `Kernel.Started -> BootTask.OnCpu -> BootInitFlow.Started` order.
  Physical and virtual binding happen later,
  do not change BootTask lifecycle, and both must resolve to the same
  linker-visible `init_task_storage`/`TaskRef::BOOT` carrier.
- BootInitFlow must expose `TaskFlowRef::BOOT_INIT`, owner/parent BootTask and
  `BootTask.initial_flow == BOOT_INIT`; it must not expose a stored guard field.
- BootInitFlow tests must observe the standard Started/Prepared/Ready/Online
  lifecycle and must not observe BP EntryPrelude checkpoints. Preset must drive
  the original entry-object order directly and commit Prepared only after the
  complete entry facts hold. Online must precede and be adjacent to the first real
  BootTask-to-KernelInitTask switch commit; BootIdleEntry may start only after
  that scheduler call later restores BootTask.
- Execution-boundary coverage must prove that a Flow process proceeds only
  when its parent Task is OnCpu/Live. Reserved authority, stale Flow generation or another mismatch must fail without changing Flow
  lifecycle state, and CurrentTaskSlot must agree with the OnCpu Task once set.
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
  is Online afterward.
- Identity switch must not emit Suspend/Continue, change lifecycle/authority/breakpoint, or increment
  physical context save/restore counts. Terminal switch must take OnCpu directly to Offline and cleanup
  on the next stack without publishing an Online breakpoint.
- BootTask begins OnCpu/Live/Invalid; its first Suspend binds BootIdleFlow and publishes its first Valid
  breakpoint. Each AP begins OnCpu/Reserved/Invalid with Base ApIdleFlow; HSM preserves TaskRef/FlowRef,
  activates Live authority and starts the matching logical-id Flow without Task Enable/Continue.
- RISC-V sentinel coverage must save/restore `ra/sp/s0..s11` and prove `tp` is established from next Task
  identity rather than from `TaskSwitchContext`.
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
