# PageFaultExceptionType

`PageFaultExceptionType` is the CPU-local binding for RISC-V instruction/load/store page-fault causes. Source and
atomicity are separate classifications. User recovery is normally schedulable. A kernel-origin fault must match the
sorted `ExceptionTable`; only a non-nested, non-hardirq, entry-irq-enabled current Task context may schedule before
fixup. Nested, hardirq or entry-irq-disabled faults may only apply the validated fixup immediately. Missing/stale
entries and illegal context are terminal.

`Preset` installs only the terminal page-fault fallback consumed by `TrapType.Setup`; handler and fixup bindings
remain later lifecycle work.

Mapping: charter [`page-fault-exception-type.md`](../../charter/objects/page-fault-exception-type.md), model
[`page_fault_exception_type.spec`](../../model/objects/page_fault_exception_type.spec), implementation
[`page_fault_exception_type.rs`](../../../impl/arceos_ex/src/objects/page_fault_exception_type.rs).
