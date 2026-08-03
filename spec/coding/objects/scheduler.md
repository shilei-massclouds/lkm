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

`NextDispatch` carries the preflight Flow identity, context epoch and exact stack-binding identity. A
physical switch binds the selected Task's own stack. The explicitly simulated user handoff binds the
UserTaskSet's already-prepared shared carrier range; this representation bit distinguishes physical
versus simulated stack commit, never first versus resume. `finish_task_switch` runs on that preflighted
stack and performs both Dispatch and Enter for initial and restored contexts. A post-commit failure,
stale generation, wrong CPU/Flow/epoch/stack or duplicate Enter terminates deterministically without
rollback or retry. Scheduler access resolves only the Task, then reaches its embedded Flow; it never
branches on a concrete Flow wrapper.

Global SMP arbitration, cross-CPU mailbox delivery, migration and schedule replay remain P2.
