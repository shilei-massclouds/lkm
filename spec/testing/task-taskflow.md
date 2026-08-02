# Task / TaskFlow testing contract

This file derives the testing contract for the lifetime one-to-one Task and
TaskFlow boundary from Charter, Model, and Coding.

## Fixed carrier and flow binding

- Every published Task exposes exactly one immutable `flow: TaskFlowRef`. The
  referenced Flow has that Task as its unique owner/parent, cannot be shared,
  and cannot be replaced during the Task lifetime.
- BootTask is created `OnCpu/Live` and is permanently bound to BootInitFlow.
  BootInitFlow contains the idle setup, scheduling return, and idle-loop
  actions; tests must not observe a second boot Task or a second boot Flow.
- KernelInitTask, KthreaddTask, and every dynamic UserTask are published
  `Online/None/Valid` with their fixed Flow already `Online`. Each AP idle Task
  starts `OnCpu/Reserved/Invalid` with its fixed ApIdleFlow already `Online`;
  HSM changes execution authority without replacing the Task or Flow.
- A user TaskFlow creates at most one stable, private UserAppRuntime. Repeated
  exec replaces only the Runtime's ApplicationInstance. Fork creates a new
  Task, a new lifetime UserTaskFlow, and a new Runtime.
- Runtime identity tests cover repeated exec (same Task/Flow/Runtime, new
  application generation), fork (all three identities distinct), stale
  TaskRef/FlowRef generation rejection, and bounded slot recycling only after
  terminal cleanup.

## Dispatch and context

- All non-identity dispatches use one path: Scheduler restores the selected
  TaskThreadContext, commits `Task Online --Continue--> OnCpu`, and delivers
  contextual `TaskFlow.Action::Continue` to the fixed FlowRef. First entry and
  later entry differ only in the architectural context contents.
- Contextual Continue carries no PC, SP, register, function name, or
  checkpoint. Tests cross-check its YieldToken/dispatch record against the
  TaskThreadContext epoch without copying either representation into the
  other.
- Identity Schedule performs no Task transition, context save/restore,
  CurrentTask/CurrentStack update, or contextual Continue. It consumes the
  source YieldToken through the generic target-completion resume attempt.
- A real switch explicitly orders SaveCoreContext, prev `Suspend` to Online,
  RestoreCoreContext and CPU-local binding commit, next `Continue` to OnCpu,
  and contextual Flow Continue. Blocked and wakeup tests use the same Task
  states; runqueue membership, not a second lifecycle state, distinguishes
  them.
- Terminal paths order Flow `Online -> Offline -> Destroyed` and Task
  `OnCpu/Online -> Offline -> Destroyed`. No terminal path republishes a
  resumable context.

## Stack guard action

- Model derivation keeps `BootTask.Action::EnableStackGuard` as the first
  `BootInitFlow.Setup` child signal, leaves BootTask `OnCpu`, and publishes
  `task_stack_guard_ready(BootTask, BootTask.stack)` without an InitStack
  Online transition.
- Runtime coverage checks the exact machine word at the Task-owned stack base,
  the installed flag, read-only integrity query, and a repeated intact call.
- Empty, reversed, smaller-than-one-word, unaligned, and recorded-range-mismatch
  inputs must all fail before storage changes or the installed flag is set.
- Corrupting an installed word must make the integrity query fail; a repeated
  Enable reports corruption and leaves the corrupted value unchanged.
- The observable completion is `BootTask.StackGuardEnabled`.
  `InitStack.Online` must be absent from the checkpoint inventory; Linux
  mapping remains a separate cross-reference responsibility.

## Effective-flow and trap coverage

- A TaskFlow action is executable only while its owner is the CPU's current
  `OnCpu/Live` Task, its fixed FlowRef and CpuRef match, and that Flow is the
  effective-flow stack leaf/root selected for the action.
- Trap entry does not change Task OnCpu or TaskFlow Online. The CPU effective
  Flow stack pushes Trap/Interrupt/Exception leaves and restores the nested
  leaf before a task-switched trap continuation resumes.
- Ordinary IRQ coverage proves no Task lifecycle delta. Wrong CPU, Flow,
  generation, context epoch, or duplicate continuation fails terminally after
  commit, while rejection before YieldToken commit preserves the exact before
  snapshot.
- RISC-V switch coverage independently proves save/restore of
  `ra/sp/s0..s11`, `tp`/CurrentTask identity publication, stack rebinding, and
  that none of those changes are implicit effects of `yields`.

## Boot and user acceptance

- Boot ordering observes one BootTask and BootInitFlow through idle setup,
  first Schedule, return, and idle loop. BootTask remains bound to
  `TaskFlowRef::BOOT_INIT` after every switch.
- KernelInitTask and KthreaddTask Flow state is Online before either Task is
  eligible for its first dispatch.
- PID 1 multiple-exec smoke preserves KernelInitTask, KernelInitFlow, and
  UserAppRuntime identity. Child smoke proves independent
  Task/UserTaskFlow/UserAppRuntime identity and terminal reclamation.
- User-entry checkpoints name the runtime/application boundary rather than a
  replaceable Flow. Checkpoint inventory and Linux mapping artifacts must be
  regenerated after any rename.

## Gates

Focused coverage includes tools2 YieldToken/snapshot tests, kernel smoke, user
smoke, checkpoint mapping checks, and architecture disassembly checks. The
final regression gate after every implementation change is the direct
repository-root `make test` command.
