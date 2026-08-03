# SMP bring-up coding constraints

Each AP idle Task embeds one immutable keyed ApIdleFlow. HSM startup resolves that fixed FlowRef. AP Tasks
begin OnCpu with Reserved authority; startup grants Live authority and the architecture entry calls
the prepared `ApIdleFlow.RunIdle` body coordinate directly without Dispatch/Enter. The first switch out publishes the Task's first
recoverable Online breakpoint; later dispatch uses the common Dispatch/Enter path.

After the three AP bringup actions, RunIdle installs the CPU-local scheduler lease and SSIP gate, publishes
idle-loop readiness, and enters the common mailbox/runqueue/safe-`wfi` loop. A first idle switch saves the real AP
continuation; restoring AP idle and restoring a dynamic kernel task both use the generic contextual Enter path.

The first SMP slice implements only explicit-target activation/wake mailbox delivery. Global arbitration, load
balancing, running-task migration, AP user tasks, timer preemption and complete schedule replay remain deferred.
