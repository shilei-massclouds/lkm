# PageFaultExceptionType

`PageFaultExceptionType` is the CPU-local binding for RISC-V instruction/load/store page-fault causes. Source and
atomicity are separate classifications. User recovery is normally schedulable. A kernel-origin fault must match the
sorted `ExceptionTable`; only a non-nested, non-hardirq, entry-irq-enabled current Task context may schedule before
fixup. Nested, hardirq or entry-irq-disabled faults may only apply the validated fixup immediately. Missing/stale
entries and illegal context are terminal.

User-origin instruction/load/store causes build one `UserFaultRequest` from the trap frame and current Task/mm,
then call the `UserAddressSpace` classifier. A successful result returns the original `sepc`; invalid or failed
results remain explicit and cannot fall through to kernel exception-table lookup. `SegvMaperr`/`SegvAccerr` are
lowered only here, for a real user trap, to the current Task's synchronous fatal SIGSEGV record and terminal
handoff; usercopy callers continue to translate the same range failure to `EFAULT`. The kernel branch likewise
must not read user VMA, sparse-backing, or COW diagnostic state.

`Preset` installs only the terminal page-fault fallback consumed by `TrapType.Setup`; handler and fixup bindings
remain later lifecycle work.

Mapping: charter [`page-fault-exception-type.md`](../../charter/objects/page-fault-exception-type.md), model
[`page_fault_exception_type.spec`](../../model/objects/page_fault_exception_type.spec), implementation
[`page_fault_exception_type.rs`](../../../impl/arceos_ex/src/objects/page_fault_exception_type.rs).
