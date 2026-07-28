# TrapType

`TrapType` is embedded in each `Cpu`; `Context` must not own a second trap resource. Its RISC-V lowering owns the
public `stvec` service, the per-CPU `TrapEntryContext`, stack-capacity admission and the emergency failure path.

- `Preset` publishes only the physical early fatal prelude. That prelude records no Flow occurrence.
- `Setup` installs `formal_trap_entry` in `stvec` after the CPU's interrupt and exception children are ready.
- `Enable` accepts entries only after both children are online.
- `sscratch` addresses the current CPU's `TrapEntryContext`; BP entry, every AP entry and scheduler commit install or
  refresh that context before an entry can occur.
- `TrapEntryContext` contains CPU identity, the active task binding, current kernel-stack bounds, emergency-stack
  bounds/state and an optional generation-checked root `TrapFlowRef`. It is entry lowering, not selector authority.
- The assembly record is `TrapFrame` followed by a naturally aligned `TrapExecutionRecord`. Admission compares the
  prospective complete record against the installed stack bounds. Insufficient capacity switches to the owning
  CPU's emergency stack and fail-stops; emergency reentry fail-stops without another switch.
- `sret` is reachable only after Rust returns a consumed, one-shot `TrapReturnToken` produced after Cleanup.

Mapping: charter [`trap-type.md`](../../charter/objects/trap-type.md), model
[`trap_type.spec`](../../model/objects/trap_type.spec), implementation
[`trap_type.rs`](../../../impl/arceos_ex/src/objects/trap_type.rs).
