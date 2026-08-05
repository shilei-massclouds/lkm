# ExecSyncBoundaries coding contract

Model source: [`exec_sync_boundaries.spec`](../../model/objects/exec_sync_boundaries.spec). Implementation:
`impl/arceos_ex/src/objects/exec_sync_boundaries.rs`.

This Context-owned object replaces `PayloadExecSyncBoundaries`. `PayloadPreparePhase.Preset` calls `setup()` only
after the registry is Ready and retains the existing Linux-deferred fact accessors required by smoke and
checkpoint handlers. Both boot and runtime transactions must validate it before beginning.

The implemented boundary is one active transaction slot plus a precommit/point-of-no-return split. Runtime
commit writes the new SATP, performs `sfence.vma`, and rewrites the syscall frame before retired backing is
freed. Boot commit installs the prepared current image but leaves the final SATP/sret to the existing
selected payload no-return entry. Bounded CLOEXEC is prechecked before point-of-no-return and mutated only after
the image swap begins.

"CPU no longer references retired backing" is checked against the executing aggregate's stable address-space
object and SATP. Runtime exec replaces that object's contents transactionally; it never transfers a whole mm/stack
through a saved-parent continuation. A vfork parent remains blocked on the same CPU and retains its own aggregate.

No code may claim deferred Linux hooks are complete. The current SMP slice nevertheless requires resource-level
IRQ-safe locking for registry, aggregate exec/mm, files/fs, allocator metadata and VFS/console state; it must not
substitute one global syscall mutex. Full Linux credential guards, LSM and namespace/accounting hooks remain
outside this slice.
