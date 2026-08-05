# Task coding constraints

## Layer closure

- Charter: changed for fixed-CPU user aggregates, registry leases and user-return preemption.
- Model: changed for `UserProcessRegistry<32>`, per-CPU inbox/clockevent and the existing single
  `Online --Dispatch--> OnCpu --Suspend--> Online` protocol.
- Coding: changed here.
- Impl: must use the representation and commit order below.
- Compose and Testing: reviewed separately; both require SMP placement, lease and preemption evidence.

## Representation

`Task` physically embeds exactly one lifetime `TaskFlow` named `flow`; `flow_ref()` is derived from that
instance rather than stored as a second Task field. The association is installed while the Task is
created and remains valid until the aggregate is destroyed. There is no concrete Flow wrapper, parallel
Flow storage, Flow list, predecessor, handoff slot, dispatch-kind field, or replacement operation.

Every ordinary published Task owns one `TaskThreadContext` containing the architectural
`TaskSwitchContext`, breakpoint state, fixed FlowRef, optional root TrapFlowRef, context epoch and private
dispatch record. `ContextCoordinate` is model metadata realized by the existing `ra/sp`, YieldToken and
save point; it adds no Rust field or assembly ABI slot. `Setup` installs the initial `ra/sp`,
FlowRef/generation, epoch and the embedded Flow's `initial_context` body coordinate before Flow Enable
validates and publishes Online and Task Enable publishes a Valid breakpoint. The boot Task starts `OnCpu` with an
Invalid breakpoint and initializes the architectural save area only on its first real switch out.

The private `TaskRef` slot ranges for boot/static, AP-idle, user and dynamic-kernel Tasks are disjoint. The
AP-idle range contains exactly `MAX_CPUS` slots starting at `TASK_SLOT_AP_IDLE_BASE`; its classifier and
diagnostic name range derive their upper bound from `MAX_CPUS`, never from a particular validation provider's
default QEMU CPU count. Thus every supported logical ID in `0..MAX_CPUS` has a classifiable keyed AP-idle ref,
while the following user range begins only after that complete interval.

Fork is the sole exception to the from-scratch lifecycle constructor. A private aggregate slot is initialized
through `materialize_fork_child_snapshot`, which validates the complete Task/Flow/Runtime/Application candidate
and publishes it directly as `Online/Online/Online`. The constructor writes the immutable Task↔Flow association,
fresh TaskRef/FlowRef generations, `None/Valid` execution/breakpoint state and post-fork ContextCoordinate before
publication. It never calls the public Preset/Setup/Enable transition methods and never imports the parent's
active TrapFlowRef, YieldToken, CurrentTask/CurrentStack binding or Live authority.

A UserTask owns a dedicated aligned kernel stack inside its `UserProcessAggregate`; a dummy, zero or shared
carrier range cannot satisfy contextual `Enter`'s CurrentStack check. Trap entry, syscall execution, saved
overlay continuation and later dispatch all use that Task-owned stack. PID 1's stable aggregate adopts the
already-running KernelInitTask stack without making it available to another user Task.

Every AP-idle, dynamic-kernel and user Task stack is at least 32 KiB. At every interrupt-enabled kernel
continuation its remaining lower-address capacity must cover one complete `TRAP_STACK_RECORD_SIZE`; this includes
the continuation that resumes an AP idle Task after switching out a terminal user Task. A smaller private stack
does not satisfy the dedicated-stack representation merely because a user-origin trap starting at its top fits.
The current RISC-V lowering allocates 32 KiB for PID 1's user trap stack and every ordinary user Task stack.
Aggregate initialization writes the embedded stack backing directly in its static slot; it must not materialize
a stack-sized `UserProcessAggregate` or `UserTaskKernelStack` temporary on the creator's kernel stack.

A bounded dynamic kernel-task registry physically embeds each Task, fixed KernelTaskFlow, dedicated aligned stack
and `TaskSwitchContext`. The first slice publishes a slot at most once; any later slot-reuse extension must increment
both TaskRef and FlowRef generations before republishing. Creation initializes the entry,
stack bounds and first context, then binds one explicit online CpuRef before release publication. After publication
the CpuRef is immutable in this slice. Remote activation/wake changes only the target inbox; the target CPU is the
only writer of the Task's scheduler/runtime fields while it is published.

The scheduler sleep declaration and matching pending-wake bit are owner-CPU fields. Inbox consumption may set
the bit only when the target is that CPU's current `OnCpu` Task with sleep declared. `PreparePrev` consumes the
pair once and keeps the Task runnable, or observes no pending bit and completes deactivation/Suspend. For an
already blocked `Online` Task, the owner CPU clears the declaration only as part of runqueue wake enqueue.

## Stack guard representation

`Task` owns the stack-guard installation flag and the lowering of `Action::EnableStackGuard`; `InitStack`
does not own a parallel guard lifecycle. On RISC-V64 the kernel stack grows toward lower addresses, so the
guard occupies exactly one machine word at the recorded stack base. Its `usize` encoding is
`0x0000000057AC6E9D` (`0x57AC6E9D` zero-extended to the 64-bit machine word).

Before the first memory write, the implementation must validate in this order-independent set that both
range endpoints are nonzero, `base < top`, `top - base >= size_of::<usize>()`, both endpoints are aligned
to `align_of::<usize>()`, and the supplied pair exactly matches the Task's recorded kernel-stack bounds.
Any failure returns without reading or writing the supplied address and without setting the installation
flag.

The first valid call performs one volatile machine-word write at `base` and then marks the guard installed.
A repeated call first performs the read-only integrity check: an intact word succeeds without another
write; a different word reports corruption and leaves it unchanged. The public installation query reads
only the flag. The integrity query validates the recorded range before a volatile read and never writes or
changes Task state. Guard-page mappings and randomized stack canaries remain separate representations and
cannot satisfy this flag or integrity query.

## Lifecycle and scheduler ownership

Ordinary Task lifecycle is `Base -> Prepared -> Ready -> Online -> OnCpu -> Online`, with terminal
`OnCpu -> Offline -> Destroyed`. Only Scheduler code may invoke `Dispatch`, `SaveCoreContext`, `Suspend`,
or `RestoreCoreContext`.

The non-identity switch order is fixed:

1. validate TaskRef, fixed FlowRef, generation, CPU and context epoch;
2. save `prev` architectural core context and increment its context epoch;
3. commit `prev` from `OnCpu` to `Online` with a Valid breakpoint;
4. restore `next` architectural core context;
5. commit CPU-local CurrentTask and CurrentStack to `next`;
6. consume the Valid breakpoint and commit `next` from `Online` to `OnCpu` through `Dispatch`, creating a
   private, single-use Enter proof from CPU, TaskRef, FlowRef/generation, context epoch and dispatch ordinal;
7. pass that proof to embedded `next.flow.Action::Enter`.

First dispatch and later dispatch use the same code. `Enter` consumes the current coordinate without
classifying it: the initial coordinate reaches the declared body Action, while saved yield/machine
coordinates resume their continuation. Machine entry is selected only by the restored context. Identity scheduling sends neither Dispatch nor Enter.
Trap entry does not change Task lifecycle; the optional root TrapFlowRef in the context restores the
effective trap leaf before the underlying TaskFlow continuation.

`NextDispatch` copies that optional root only as an identity proof. A nonempty root requires
generation-checked resolution of root, active child and concrete leaf against the fixed Task/Flow, CpuRef and
context epoch before Dispatch and again at Enter. Enter records leaf resume but never writes `ra/sp` or selects a
handler by leaf kind. An empty root produces no leaf-resume observation.

The initial BootTask and AP-idle architecture entries call their Flow actions directly without a false
Dispatch/Enter. After either Task has really switched out, every restoration uses the common proof path.

Once SMP concurrency is open, no runtime user path obtains an `&'static mut Context`. BP and AP code obtain a
narrow owner-CPU lease for exactly one Scheduler/Interrupt pair and a generation-and-CPU-checked
`UserProcessLease` for the addressed Task. The lease guard is the only capability that can reach mutable
aggregate resources. Shared registry/inbox metadata uses acquire/release atomics or an IRQ-safe lock; no hart can
acquire a mutable reference to the whole Context.

## Teardown

The embedded Flow disables and cleans up first. Any logically Flow-owned UserAppRuntime and current
ApplicationInstance are companion storage inside the Task aggregate and are quiesced before Flow cleanup.
Task disable and cleanup follow; the FlowRef is never rebound during this sequence.

A user COW allocation or commit resource failure records `TaskTerminalReason::OutOfMemory` before this terminal
sequence. The child archive stores a fatal-signal wait word with low signal bits equal to 9 rather than the normal
`exit_code << 8` encoding, wakes an eligible parent wait and queues SIGCHLD. The current Task's mm is released
exactly once during terminal handoff; reap releases only the Task record. This reason does not construct a user
signal frame, select an OOM victim or invoke a system-wide panic path.

A fatal illegal user page fault instead records `TaskTerminalReason::SegmentationFault` and one immutable
`TaskSegvInfo`. Its representation contains `TaskSegvCode::{Maperr, Accerr}` and the exact fault address; the signal
number is the constant 11 rather than a mutable field. Recording is accepted only for an OnCpu/Live Task whose
terminal reason is `None`, so duplicate or conflicting terminals fail deterministically. `task_wait_status` encodes
this reason as 11 in the low signal bits. The existing terminal handoff and reap ordering is shared with OOM, but
the two reasons and wait words remain disjoint.
