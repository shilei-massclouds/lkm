# Scheduler coding constraints

`Scheduler` is CPU-local and stores TaskRefs in runqueues. Dispatch lookup always obtains the candidate's
single fixed FlowRef and TaskThreadContext; there is no first-versus-resume selector.

`schedule_current()` is the implementation mapping of immediate `yields Scheduler.Action::Schedule`
delivery. The model yield mechanism itself performs no register, Task, CPU binding, runqueue, lock or
interrupt mutation.

Preflight resolves and validates source Task/Flow/CPU, candidate Task/Flow/generation, context epoch,
signal capacity and runqueue eligibility before the yield token is committed. Rejection is side-effect
free.

Identity selection performs class bookkeeping and returns normally. It does not save/restore registers,
change Task state, rebind CurrentTask/CurrentStack, or send Dispatch/Enter; normal target completion
allows the generic yield machinery to resume the source immediately.

Non-identity selection explicitly performs:

```text
PreparePrev -> PickNext -> SaveCoreContext(prev) -> prev.Suspend
            -> RestoreCoreContext(next) -> commit CurrentTask/CurrentStack
            -> next.Dispatch -> next.flow.Enter
```

`NextDispatch` carries the preflight Flow identity, context epoch, optional root TrapFlowRef and exact stack-binding identity. A
physical switch binds the selected Task's own stack. The explicitly simulated user handoff binds the
UserTaskSet's already-prepared shared carrier range; this representation bit distinguishes physical
versus simulated stack commit, never first versus resume. `finish_task_switch` runs on that preflighted
stack and performs both Dispatch and Enter for initial and restored contexts. A post-commit failure,
stale generation, wrong CPU/Flow/epoch/stack or duplicate Enter terminates deterministically without
rollback or retry. Scheduler access resolves only the Task, then reaches its embedded Flow; it never
branches on a concrete Flow wrapper.

For a nonempty root, preflight resolves root→active child→concrete leaf and checks generation, entry Task/Flow,
owner CPU, context epoch and non-cleaned state. Contextual Enter repeats that validation after Dispatch and records
one leaf resume. The Scheduler stores no exception-kind dispatch mode; restored machine state selects continuation.

At SMP runtime each `Scheduler` logically owns one lane in a bounded, CPU-indexed static mailbox companion. Only
that Scheduler's owner CPU consumes the lane. A remote producer writes only immutable
`{TaskRef, generation, target CpuRef, ordinal, kind}` fields, release-publishes the slot, then calls the SBI IPI
extension for the target hart. It never borrows or mutates the target runqueue. The owner CPU acquire-consumes a
slot once, rejects stale generation, wrong target and non-increasing ordinal before mutation, and performs the
enqueue/wake while holding its local runqueue lock. Duplicate IPIs only coalesce `need_resched`.

`schedule_current()` resolves the owner Scheduler from the current fixed Flow's CpuRef; it is not a CPU0 alias.
The same implementation accepts BootTask, keyed AP idle Tasks and dynamic kernel Tasks. AP idle binding is the
owner Scheduler's keyed `idle` TaskRef, and secondary enable prepares the same preemption/runqueue protocol before
the first real switch. Idle first-switch and all later idle restores use the ordinary non-identity sequence above.

The AP idle loop enables SSIP delivery, consumes its mailbox, and schedules only at the post-interrupt idle/Task
safe point. Before `wfi` it closes the total interrupt gate and rechecks both mailbox publication and
`need_resched`; it enters `wfi` only when both remain clear, then reopens the gate. The SSIP handler only clears
pending and sets CPU-local `need_resched`.

Global arbitration, automatic load selection, running-task migration, timer preemption and schedule replay remain
deferred.
