# TaskFlow coding constraints

## Identity and storage

Each Task physically embeds exactly one lifetime `TaskFlow`. `TaskFlow` stores its immutable owner
`TaskRef`, stable generation and mutable `CpuRef`. Owner and parent are the same Task, and a FlowRef can
resolve to no other Task. Dynamic user tasks allocate one aggregate occurrence; exec never allocates
another Flow. BootInitFlow, KernelInitFlow and KthreaddFlow remain Model instance names and module names,
not Rust wrapper structures.

The static mappings are:

- BootTask -> BootInitFlow;
- KernelInitTask -> KernelInitFlow;
- KthreaddTask -> KthreaddFlow;
- each AP idle Task -> its keyed ApIdleFlow;
- each dynamic user Task -> a fresh UserTaskFlow.

BootInitFlow directly owns idle setup, scheduling return, idle entry and idle-loop actions.

## Lifecycle and execution

KernelInitFlow, KthreaddFlow and UserTaskFlow are moved to `Online` before their owner Task is published
`Online`. Ordinary TaskFlow state remains `Online` across every task switch. Terminal teardown uses
`Online -> Offline -> Destroyed`.

`TaskFlow::enter_contextual` accepts only the private proof produced by its owner Task's Dispatch. It
validates TaskRef, FlowRef/generation, CpuRef, context epoch, dispatch record, CurrentTask/CurrentStack and
effective-flow execution, then records consumption so duplicate Enter fails deterministically. It contains
no fixed-entry branch. It resumes a matching YieldToken; on the first prepared context it consumes the
Setup-bound `Start` coordinate exactly once; otherwise execution continues at the saved coordinate.

`Task` exposes paired internal operations for Setup/Enable, Save/Suspend, Dispatch/Enter and teardown so
the embedded Flow never requires a self-referential pointer or unsafe alias. Concrete Flows do not
override `Enter`; their unique `Start` action owns only the first-entry body.

Trap, interrupt and exception handlers temporarily change the CPU effective-flow stack. A regular
TaskFlow action may execute only when it is the effective Flow. A trap may schedule without changing the
underlying TaskFlow state.

## User runtime

Every user-capable Flow logically owns at most one stable `UserAppRuntime`, stored as companion data in
the containing Task aggregate. The runtime owns a mutable reference to
the current `ApplicationInstance`. Exec commits a fresh ApplicationInstance into that reference without
changing Task, Flow, Runtime, TaskRef or FlowRef identity. Fork creates all three fresh Task/Flow/Runtime
occurrences.
