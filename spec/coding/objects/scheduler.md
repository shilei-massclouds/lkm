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

The owner CPU's outer local-interrupt save remains pending across `SaveCoreContext(prev)`, `prev.Suspend`, the
architecture stack/TP switch, SATP and `rq->curr` publication, `next.Dispatch`, `next.flow.Enter`, and trap-entry
owner refresh. Only the continuation executing on the selected Task's stack restores that exact saved interrupt
state and reenables scheduler preemption. An identity selection restores on its original stack. Error cleanup
before a physical switch restores locally; no interrupt-enabled instruction may expose a `tp` Task that is no
longer `OnCpu`, or a selected Task whose CurrentTask/trap-entry bindings are not yet committed.

`NextDispatch` carries the preflight Flow identity, context epoch, optional root TrapFlowRef and exact stack-binding identity. Every
switch, including a user switch, binds the selected Task's own kernel stack. `finish_task_switch` runs on that preflighted
stack and performs both Dispatch and Enter for initial and restored contexts. A post-commit failure,
stale generation, wrong CPU/Flow/epoch/stack or duplicate Enter terminates deterministically without
rollback or retry. Scheduler access resolves only the Task, then reaches its embedded Flow; it never
branches on a concrete Flow wrapper.

For a nonempty root, preflight resolves root→active child→concrete leaf and checks generation, entry Task/Flow,
owner CPU, context epoch and non-cleaned state. Contextual Enter repeats that validation after Dispatch and records
one leaf resume. The Scheduler stores no exception-kind dispatch mode; restored machine state selects continuation.

At SMP runtime each `Scheduler` owns a bounded CPU-indexed inbox whose capacity covers all deliverable user and
kernel Tasks. Each Task can hold at most one reserved or published activation/wake record; duplicate notices
coalesce. A remote producer reserves capacity before publishing a fork candidate, writes only immutable
`{TaskRef, generation, target CpuRef, ordinal, kind}` fields, release-publishes the record, then calls the SBI IPI
extension for the target hart. It never borrows or mutates the target runqueue. The owner CPU acquire-consumes a
record once, rejects stale generation, wrong target and an ordinal not uniquely claimed by that reservation before
mutation, and performs the enqueue/wake while holding its local runqueue lock. Independently reserved records may
be release-published and consumed in an order different from ordinal claim order; a global consumed-watermark must
not discard such a still-published per-Task record. The slot state supplies exact-once consumption, while a maximum
consumed ordinal is diagnostic only. Reservation cancellation is side-effect free. Duplicate IPIs only coalesce
`need_resched`.

Each scheduler-class queue is also statically sized to cover every TaskRef that can simultaneously target one
CPU; fixed placement cannot rely on other CPUs having spare queue entries. In the current representation the
shared fair-queue bound is `31 dynamic user Tasks + PID 1 + kthreadd + 8 dynamic kernel Tasks = 41`. Consequently
an activation reserved before fork publication cannot later fail merely because more than eight valid Tasks map
to the same CPU. Duplicate identity and invalid-generation rejection remain separate from this capacity bound.

Wake consumption is an owner-CPU two-sided handoff. If the referenced Task is still `OnCpu`, is the local current
Task and has declared this scheduler sleep, inbox consumption records its single matching pending-wake flag;
`PreparePrev` consumes the flag and retains runnable eligibility. If the Task is `Online`, off CPU, sleep-declared
and absent from the runqueue, consumption clears the sleep state and enqueues it under the local runqueue lock.
If it is already runnable/on-rq, the wake is a coalesced no-op. The user-return safe point drains visible inbox
records before deciding whether to schedule, so a wake published between sleep declaration and Suspend reaches
one of these two commits without a producer-side Task mutation.

`schedule_current()` resolves the owner Scheduler from the current fixed Flow's CpuRef; it is not a CPU0 alias.
The same implementation accepts BootTask, keyed AP idle Tasks and dynamic kernel Tasks. AP idle binding is the
owner Scheduler's keyed `idle` TaskRef, and secondary enable prepares the same preemption/runqueue protocol before
the first real switch. Idle first-switch and all later idle restores use the ordinary non-identity sequence above.

The AP idle loop enables SSIP delivery, consumes its inbox, and schedules only at the post-interrupt idle/Task
safe point. Before `wfi` it closes the total interrupt gate and rechecks both inbox publication and
`need_resched`; it enters `wfi` only when both remain clear, then reopens the gate. The SSIP handler only clears
pending and sets CPU-local `need_resched`.

User TaskRefs participate in the owner CPU's fair round-robin queue. Dispatch acquires the selected
`UserProcessLease`, writes that aggregate's SATP, performs local `sfence.vma`, then arms a 10 ms slice. The
per-CPU `SchedulerClockevent` multiplexes this deadline with CPU0's existing one-shot callback. A timer hardirq
only acknowledges the event, rearms the earliest deadline and coalesces CPU-local `need_resched`; it never calls
the scheduler. SPP=U permits scheduling at the post-handler, post-leaf-Disable, pre-leaf/root-Cleanup safe
continuation. That continuation drains all acquire-visible CPU-local inbox records before testing the combined
inbox/SSIP/timer work request. SPP=S retains the request until the next return-to-user boundary. If there is no
competitor, the safe point consumes the bit, advances the deadline from its prior value while skipping missed
periods, and returns without a context switch. A non-identity round trip must revalidate the unreleased concrete
leaf before its lifecycle cleanup continues.

A user-origin blocking syscall wait loop invokes the same safe-point implementation before `wfi` whenever any
CPU-local inbox, SSIP `need_resched`, or scheduler-clockevent `need_resched` is visible. The decision is independent
of whether the pending Task is the waiter's own child. The helper opens the installed CPU-local trap runtime lease,
preserves the Ready/unreleased syscall leaf across a non-identity switch, and applies the same Task/Flow/CPU, root,
context-epoch and active-leaf A→B→A validation before returning to the wait loop. A nested SPP=S hardirq only marks
pending work and never invokes this helper itself.

Global arbitration, automatic load selection, running-task migration, kernel-mode immediate preemption, shared
mm cross-CPU execution, ASIDs, remote TLB shootdown and schedule replay remain deferred.
