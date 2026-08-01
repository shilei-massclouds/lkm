# SMP bring-up coding constraints

Each AP idle Task has one immutable keyed ApIdleFlow. HSM startup resolves that fixed FlowRef. AP Tasks
begin OnCpu with Reserved authority; startup grants Live authority and contextual ApIdleFlow Continue
enters the prepared architectural context. The first switch out publishes the Task's first recoverable
Online breakpoint; later dispatch uses the same Continue path.

The deterministic per-CPU lane representation removes all Flow-selection and dispatch-kind branches.
Global arbitration, cross-CPU mailbox protocol, migration and complete schedule replay remain P2.
