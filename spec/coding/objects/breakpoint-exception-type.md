# BreakpointExceptionType

`BreakpointExceptionType` owns the breakpoint cause binding and hook registry. Its default handling is non-nesting and
non-schedulable; a registered hook may advance `sepc` only after it reports the exact instruction width handled.

Mapping: charter [`breakpoint-exception-type.md`](../../charter/objects/breakpoint-exception-type.md), model
[`breakpoint_exception_type.spec`](../../model/objects/breakpoint_exception_type.spec), implementation
[`breakpoint_exception_type.rs`](../../../impl/arceos_ex/src/objects/breakpoint_exception_type.rs).
