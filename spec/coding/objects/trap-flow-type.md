# TrapFlowType

Each accepted entry declares a fresh, nonzero-generation `TrapFlowType` in the stack-local `TrapExecutionRecord`, then
binds its immutable parent to the entry CPU's `TrapType`. `Preset` validates the entry and saves its return checkpoint;
`Setup` creates and records one child FlowRef; `Enable` creates one return token; `Disable` and `Cleanup` synchronously
destroy child then root. Assembly consumes the token only after Cleanup.

`Task.active_flow` remains the persistent TaskFlow and is paused beneath this chain. `CurrentTask`, `CurrentTaskRef`
and `CurrentCPU` continue to resolve through that TaskFlow. `TaskThreadContext.root_trap_flow_ref` carries the optional
root reference across a legal scheduling point; the root's active-child links locate the leaf.

Entry capture records the effective `TaskRef`, `TaskFlowRef`, CPU and exec-transaction commit count. Return accepts the
same Task/TaskFlow pair, a scheduler-committed terminal Task switch, or a same-Task successful-exec handoff. The exec
case is valid only when the transaction is no longer active or past point-of-no-return, its commit count advanced by
exactly one during this occurrence, and the currently active successor's generation-checked `predecessor` is exactly
the entry `TaskFlowRef`. A historical exec commit or an unrelated Flow replacement fails with the full trap authority
diagnostic and shutdown.

Root-reference cleanup distinguishes a live entry Task from a scheduler-committed terminal switch. A live Task must
still contain the exact root installed by this occurrence, or the generation-checked outer root for a nested
occurrence. If the exact terminal-switch facts above have already retired the entry Task storage, absence of that
storage completes its root-reference teardown; a present-but-missing, replaced or stale reference is never accepted.

Mapping: charter [`trap-flow-type.md`](../../charter/objects/trap-flow-type.md), model
[`trap_flow_type.spec`](../../model/objects/trap_flow_type.spec), implementation
[`trap_flow_type.rs`](../../../impl/arceos_ex/src/objects/trap_flow_type.rs).
