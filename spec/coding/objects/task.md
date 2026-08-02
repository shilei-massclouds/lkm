# Task coding constraints

## Layer closure

- Charter: changed for the lifetime one-to-one Task/TaskFlow boundary.
- Model: changed to the single `Online --Continue--> OnCpu --Suspend--> Online` protocol.
- Coding: changed here.
- Impl: must use the representation and commit order below.
- Compose and Testing: reviewed separately; both require updated fixed-Flow evidence.

## Representation

`Task` stores exactly one immutable `TaskFlowRef` named `flow`. The association is installed while the
Task is created and remains valid until the Task and its Flow are destroyed. There is no Flow list,
predecessor, handoff slot, dispatch-kind field, or replacement operation in `Task`.

Every ordinary published Task owns one `TaskThreadContext` containing the architectural
`TaskSwitchContext`, breakpoint state, fixed FlowRef, optional root TrapFlowRef, context epoch and dispatch
record. A newly published ordinary Task already has an initialized architectural context and a Valid
breakpoint bound to `flow`. The boot Task starts `OnCpu` with an Invalid breakpoint and initializes the
architectural save area only on its first real switch out.

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
`OnCpu -> Offline -> Destroyed`. Only Scheduler code may invoke `Continue`, `SaveCoreContext`, `Suspend`,
or `RestoreCoreContext`.

The non-identity switch order is fixed:

1. validate TaskRef, fixed FlowRef, generation, CPU and context epoch;
2. save `prev` architectural core context and increment its context epoch;
3. commit `prev` from `OnCpu` to `Online` with a Valid breakpoint;
4. restore `next` architectural core context;
5. commit CPU-local CurrentTask and CurrentStack to `next`;
6. commit `next` from `Online` to `OnCpu` through `Continue`;
7. deliver contextual `next.flow.Action::Continue`.

First dispatch and later dispatch use the same code. Machine entry is selected only by the restored
context. Trap entry does not change Task lifecycle; the optional root TrapFlowRef in the context restores
the effective trap leaf before the underlying TaskFlow continuation.

## Teardown

The Flow disables and cleans up first. Any Flow-owned UserAppRuntime and current ApplicationInstance are
quiesced before Flow cleanup. Task disable and cleanup follow; the fixed FlowRef is never rebound during
this sequence.
