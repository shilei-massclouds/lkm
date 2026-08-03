# SMP bring-up coding constraints

Each AP idle Task embeds one immutable keyed ApIdleFlow. HSM startup resolves that fixed FlowRef. AP Tasks
begin OnCpu with Reserved authority; startup grants Live authority and the architecture entry calls
the prepared `ApIdleFlow.RunIdle` body coordinate directly without Dispatch/Enter. The first switch out publishes the Task's first
recoverable Online breakpoint; later dispatch uses the common Dispatch/Enter path.

The deterministic per-CPU lane representation removes all Flow-selection and dispatch-kind branches.
Global arbitration, cross-CPU mailbox protocol, migration and complete schedule replay remain P2.
