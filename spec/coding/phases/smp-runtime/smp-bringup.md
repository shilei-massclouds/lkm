# SMP bring-up coding constraints

Each AP idle Task embeds one immutable keyed ApIdleFlow. HSM startup resolves that fixed FlowRef. AP Tasks
begin OnCpu with Reserved authority; startup grants Live authority and the architecture entry calls
the prepared `ApIdleFlow.RunIdle` body coordinate directly without Dispatch/Enter. The first switch out publishes the Task's first
recoverable Online breakpoint; later dispatch uses the common Dispatch/Enter path.

After the three AP bringup actions, RunIdle installs the CPU-local scheduler lease and SSIP gate, publishes
idle-loop readiness, and enters the common inbox/runqueue/safe-`wfi` loop. Because the one-way bring-up phase chain
never returns, the handoff first re-establishes `RunIdle` at the top of the same AP Task-owned stack; it preserves
Task, Flow, CpuRef and stack-range identity and occurs before opening interrupts. A first idle switch saves this
clean real AP continuation; restoring AP idle and restoring a dynamic kernel or user Task both use the generic contextual Enter path.

When all target CPUs have published Online, the BP freezes an array of online CpuRefs in ascending logical-id
order. It then enables the per-CPU inbox and `SchedulerClockevent` on every element before admitting user fork
publication. PID 1 stays on CPU0. Ordinary fork selects `online_cpus[child_pid % cpu_count]`; vfork/CLONE_VM selects
the parent CPU. Every user Task thereafter resolves its registry lease and SATP only on that CPU.

Each AP opens SSIE and its timer source only after the corresponding inbox, runqueue, CurrentTask binding and
clockevent are initialized. Timer hardirq coalesces `need_resched`; dispatch occurs only at the common user-return
safe point. Global arbitration, load balancing, running-task migration, runtime CPU hotplug, shared-mm cross-CPU
execution, kernel-mode immediate preemption and complete schedule replay remain deferred.
