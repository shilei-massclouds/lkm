# ExecSyncBoundaries coding contract

Model source: [`exec_sync_boundaries.spec`](../../model/objects/exec_sync_boundaries.spec). Implementation:
`impl/arceos_ex/src/objects/exec_sync_boundaries.rs`.

This Context-owned object replaces `PayloadExecSyncBoundaries`. `PayloadPhase.Preset` calls `setup()` only
after the registry is Ready and retains the existing Linux-deferred fact accessors required by smoke and
checkpoint handlers. Both boot and runtime transactions must validate it before beginning.

The implemented boundary is one active transaction slot plus a precommit/point-of-no-return split. Runtime
commit writes the new SATP, performs `sfence.vma`, and rewrites the syscall frame before retired backing is
freed. Boot commit installs the prepared current image but leaves the final SATP/sret to the existing
PayloadPhase no-return entry. Bounded CLOEXEC is prechecked before point-of-no-return and mutated only after
the image swap begins.

"CPU no longer references retired backing" includes the bounded saved-parent snapshot: a runtime child exec
must transfer the old mm/stack to the `UserTaskSet`-owned continuation record while that parent continuation is live. The corresponding
child-exit handoff releases the child image and restores the parent ownership before resuming it.

No code may claim the deferred Linux locks/hooks are held or complete. Concurrency, credentials, signals,
LSM, namespace/accounting and fatal-signal recovery remain outside this slice.
