# Task coding constraints

## Layer closure

- Charter: changed for the lifetime one-to-one Task/TaskFlow boundary.
- Model: changed to the single `Online --Dispatch--> OnCpu --Suspend--> Online` protocol.
- Coding: changed here.
- Impl: must use the representation and commit order below.
- Compose and Testing: reviewed separately; both require updated fixed-Flow evidence.

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

A UserTask binds the actual shared kernel carrier-stack range during `Setup`; a dummy or zero range cannot
satisfy contextual `Enter`'s CurrentStack check. On the real user exception path this is the prepared user
kernel trap-stack range on which the simulated user-to-user handoff and contextual Enter execute, not the
KernelInitTask stack from which user entry was originally prepared. A smoke-only handoff may instead bind
the KernelInitTask range when that is the stack on which the simulated Enter actually executes. This range
is companion state inside the UserTask aggregate, not a new context specification object and not a
parallel TaskFlow owner.

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

The initial BootTask and AP-idle architecture entries call their Flow actions directly without a false
Dispatch/Enter. After either Task has really switched out, every restoration uses the common proof path.

## Teardown

The embedded Flow disables and cleans up first. Any logically Flow-owned UserAppRuntime and current
ApplicationInstance are companion storage inside the Task aggregate and are quiesced before Flow cleanup.
Task disable and cleanup follow; the FlowRef is never rebound during this sequence.
