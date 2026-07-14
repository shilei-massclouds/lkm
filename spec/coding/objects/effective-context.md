# Effective Context coding

本文件是 Effective Context 实现映射的权威 coding 规格，保留稳定 rule ID、原
`ArceosExEffectiveContextCodingMust` type 分组和 MUST 层级；这些 ID 用于评审和追踪，不是 pyveri predicate。

## Rule catalog

### ArceosExEffectiveContextCodingMust

#### Natural phase-boundary guard

Rule ID: `arceos_ex_must_phase_boundary_guard_lower_to_context_contribution_only` (MUST).

A natural phase-boundary guard records the Effective Context contribution of
the surrounding execution window, such as boot-time single CPU/task
execution with local interrupts and preemption already disabled. It
must lower to no runtime enter/exit code by itself. Its contribution
still participates in nested Effective Context and may influence
lowering decisions for inner context or guard constructs; generated
code must not emit dummy lock, irq or preemption operations for the
phase-boundary guard itself.

#### PreemptionControl guard boundary

Rule ID: `arceos_ex_must_preemption_guard_lower_to_counted_enter_exit` (MUST).

A guard whose entered_by/exited_by use PreemptionControl is a protocol
guard, not a pure boolean proof. It
must lower to the target's counted preemption-disable enter and
matching exit operation, or an equivalent RAII guard that performs
those operations exactly once. Even if an outer Effective Context
already proves preemption: disabled, the nested preemption-control guard must
still preserve its own count/owner/debug protocol. Avoiding those
operations is valid only when the model does not introduce a nested
preemption-control guard and instead relies solely on an outer context
contribution.

#### LocalInterruptControl guard boundary

Rule ID: `arceos_ex_must_local_interrupt_guard_preserve_saved_flags` (MUST).

A guard whose entered_by/exited_by use LocalInterruptControl
irqsave/irqrestore must lower to
operations that save the incoming local interrupt state and restore
exactly that saved state on exit by default. It must not be reduced
to an unconditional disable/enable pair. Current arceos_ex lowering
does not use Effective Context plus `only-once` to elide this
protocol. A future proof-only optimization must be introduced as a
separate rule and must prove that no saved-flags token, count, debug
side effect or later consumer depends on the runtime protocol. A
guard that intentionally models unconditional local IRQ
disable/enable must still be represented as a distinct explicitly
documented action, not inferred from the irqsave form.

#### RawSpinLock irq-save guard boundary

Rule ID: `arceos_ex_must_raw_spin_lock_irqsave_guard_lock_and_restore` (MUST).

A guard whose entered_by/exited_by use RawSpinLock LockIrqSave /
UnlockIrqRestore must lower to the lock irqsave protocol:
save local interrupt state, disable local interrupts as required by
the target primitive, acquire the raw spin lock, and on exit release
the same lock before restoring the saved interrupt state. Its
Effective Context contribution includes the held lock, local
interrupts disabled, preemption disabled and voluntary switching
disabled while the guard is active, but those derived attributes do
not replace the lock/unlock protocol itself.

#### RawSpinLock ordinary guard boundary

Rule ID: `arceos_ex_must_raw_spin_lock_guard_use_plain_lock_unlock` (MUST).

A guard whose entered_by/exited_by use RawSpinLock Action::Acquire /
Action::Release represents plain raw_spin_lock/raw_spin_unlock. It
must lower to acquiring and releasing the same raw spin lock and
must not be silently dropped merely because an outer context already
disables local interrupts or preemption. It also must not be
rewritten into a second irqsave/irqrestore pair; irq/preemption
effects must come only from the explicit outer guard that models
them. init_idle() uses this shape for BootRunQueueLock inside the
outer BootIdlePiLock irqsave context.

#### Effective Context is not a license to erase protocol guards

Rule ID: `arceos_ex_must_effective_context_not_elide_protocol_guards` (MUST).

Effective Context may influence lowering of runtime-code-free context
sources and avoid duplicate attribute-only scaffolding. It must not
by itself erase a guard whose implementation owns state, nesting
counts, saved flags, lock ownership, memory ordering, debug
assertions, wakeups or other resource protocol effects. Such guards
remain real code even when an outer context already provides the same
high-level attribute. Future proof-only lowering must be introduced
as an explicit optimization rule with its own model proof and
protocol-side-effect audit.

#### RCU read-side first slice

Rule IDs (MUST):

- `arceos_ex_must_rcu_read_side_first_slice_remain_marked_incomplete`
- `arceos_ex_must_defer_full_rcu_read_side_lowering`
- `arceos_ex_must_defer_rcu_lowering_until_primitive_exists` (the legacy index declared this ID outside its type invariant; it remains authoritative here)

RCU read-side guards belong to the same guard lowering family. The
current SchedInitPhase mapping may implement BootIdleRcuReadSide
only as the incomplete first slice required by init_idle():
balanced rcu_read_lock()/rcu_read_unlock() accounting around
__set_task_cpu(). It must keep explicit incomplete/deferred facts
and must not claim full RCU reader nesting, preemptible-RCU
accounting, quiescent-state reporting, lockdep/debug checks, or
scheduler/RCU context-switch integration. Broader RCU read-side
lowering requires a later complete primitive-specific rule.
