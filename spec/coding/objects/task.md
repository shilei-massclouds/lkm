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
