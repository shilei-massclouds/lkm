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
- each dynamic kernel Task -> a fresh KernelTaskFlow;
- each dynamic user Task -> a fresh ordinary TaskFlow.

BootInitFlow directly owns idle setup, scheduling return, idle entry and idle-loop actions.

`ApIdleFlow.RunIdle` remains the one AP execution continuation after bringup and lowers to the common inbox,
need-resched and safe-`wfi` loop. It may dispatch fixed-owner user Tasks as well as kernel Tasks. A dynamic KernelTaskFlow binds its caller-supplied kernel entry as the initial
coordinate and keeps the same identity across identity yield, block/wake and continuation resume.

## Lifecycle and execution

KernelInitFlow, KthreaddFlow and dynamic ordinary TaskFlows are `Online` before their owner Task is published
`Online`. From-scratch construction uses the lifecycle transitions; fork construction uses the checked aggregate
snapshot constructor and directly commits `Task/TaskFlow/UserAppRuntime = Online/Online/Online` without invoking
Preset, Setup, or Enable. Ordinary TaskFlow state remains `Online` across every task switch. Terminal teardown uses
`Online -> Offline -> Destroyed`.

`TaskFlow::enter_contextual` accepts only the private proof produced by its owner Task's Dispatch. It
validates TaskRef, FlowRef/generation, CpuRef, context epoch, dispatch record, CurrentTask/CurrentStack and
effective-flow execution, then consumes the proof so duplicate Enter fails deterministically. It contains
no first/resume or concrete-Flow branch and owns no one-shot entry state. The already restored
`ContextCoordinate` selects a handler body, YieldToken resume coordinate, or machine continuation.

When the dispatch context contains a root TrapFlowRef, the Scheduler bridge supplies a separate validated
root/leaf proof to `Task::enter_flow_contextual`; it is consumed exactly once together with the Dispatch proof.
No-switch trap returns never call this path.

`Task` exposes paired internal operations for Setup/Enable, Save/Suspend, Dispatch/Enter and teardown so
the embedded Flow never requires a self-referential pointer or unsafe alias. Concrete Flows do not
override `Enter`; their `initial_context` declaration names an Online `StateEffect::None` body Action.
There is no `TaskFlow::Exit`: Save/Suspend represents switching out and Disable/Cleanup represents termination.

Trap, interrupt and exception handlers temporarily change the CPU effective-flow stack. A regular
TaskFlow action may execute only when it is the effective Flow. A trap may schedule without changing the
underlying TaskFlow state.

## User runtime

Every user-capable Flow logically owns at most one stable `UserAppRuntime`, stored as companion data in
the containing Task aggregate. The runtime owns a mutable reference to
the current `ApplicationInstance`. Exec commits a fresh ApplicationInstance into that reference without
changing Task, Flow, Runtime, TaskRef or FlowRef identity. Fork creates all three fresh Task/Flow/Runtime
occurrences.

The Runtime is passive: `Online` covers executing, runnable, blocked, and temporarily trap-overlaid
continuations. It is not pushed onto the effective-flow stack and has no Running, Paused, or Trapped storage.
Scheduling restores the Task context and calls only Dispatch and TaskFlow Enter; it never calls a Runtime Enter.
Terminal code explicitly quiesces and cleans Runtime before Flow, and Flow before Task.

At SMP publication, PID 1's Flow is fixed to CPU0. An ordinary fork Flow is fixed to
`online_cpus[child_pid % cpu_count]`; a vfork/CLONE_VM Flow is fixed to the parent CpuRef. No later action mutates a
published user Flow's CpuRef. Timer expiry does not enter a Flow from hardirq: it only records CPU-local
`need_resched`, and a return-to-user safe continuation performs any Dispatch/Enter sequence.
