# TaskFlow coding constraints

## Identity and storage

Each Task owns exactly one lifetime `TaskFlow`. `TaskFlow` stores its immutable owner `TaskRef`, stable
generation and mutable `CpuRef`. Owner and parent are the same Task, and a FlowRef can resolve to no other
Task. Dynamic user tasks allocate one `UserTaskFlow` storage occurrence; exec never allocates another Flow.

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

`TaskFlow::continue_contextual` validates owner, FlowRef generation, CpuRef, current Task/Stack binding,
effective-flow execution and context epoch. It contains no entry function selection. It resumes a pending
model yield when one matches; otherwise it enters the Flow's ordinary action body selected by the already
restored TaskThreadContext.

Trap, interrupt and exception handlers temporarily change the CPU effective-flow stack. A regular
TaskFlow action may execute only when it is the effective Flow. A trap may schedule without changing the
underlying TaskFlow state.

## User runtime

Every user-capable Flow owns at most one stable `UserAppRuntime`. The runtime owns a mutable reference to
the current `ApplicationInstance`. Exec commits a fresh ApplicationInstance into that reference without
changing Task, Flow, Runtime, TaskRef or FlowRef identity. Fork creates all three fresh Task/Flow/Runtime
occurrences.
