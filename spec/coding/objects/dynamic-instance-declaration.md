# Dynamic instance declaration coding constraints

Dynamic declaration allocates serializable identity/generation records, never host-language frames.
Fresh user Task, TaskFlow, UserAppRuntime and ApplicationInstance occurrences are the production users.
Each child receives independent storage and generation; retirement cannot make a stale Ref resolve.

Snapshot materialization uses a private candidate aggregate containing the instance table, state map, reference/fact
sets, PID reservation, page-table updates and frame reference deltas. The implementation publishes that aggregate with
one commit marker only after every binding and final-state invariant succeeds. Failure restores the pre-block tables,
counters and lexical bindings; candidate PID slots, PTEs and frame references must be released without becoming
observable. A materialized instance stores its construction kind, snapshot-site identity, declared Type, generation
and requested committed state. It must not call or synthesize lifecycle transition functions.

Ordinary `declare` keeps the existing Base-state diagnostic-instance behavior and has no snapshot rollback. The
lowering must reject `materialize` outside a snapshot, lifecycle transitions inside a snapshot, state on a stateless
Type and missing/unknown state on a stateful Type.
