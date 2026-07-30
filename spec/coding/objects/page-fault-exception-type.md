# PageFaultExceptionType

`PageFaultExceptionType` is the CPU-local binding for RISC-V instruction/load/store page-fault causes. User and other
recoverable faults may schedule and migrate while their root TrapFlow remains parented to the entry CPU. A fault in
hardirq/atomic context may only complete an exception-table fixup; otherwise it is terminal.

`Preset` installs only the terminal page-fault fallback consumed by `TrapType.Setup`; handler and fixup bindings
remain later lifecycle work.

Mapping: charter [`page-fault-exception-type.md`](../../charter/objects/page-fault-exception-type.md), model
[`page_fault_exception_type.spec`](../../model/objects/page_fault_exception_type.spec), implementation
[`page_fault_exception_type.rs`](../../../impl/arceos_ex/src/objects/page_fault_exception_type.rs).
