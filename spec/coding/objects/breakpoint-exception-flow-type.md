# BreakpointExceptionFlowType

This occurrence binds to `BreakpointExceptionType`, drives one registered hook or the fatal default and records the
handled instruction width. It neither schedules nor enables nested exceptions unless an explicit source rule says so.

Mapping: charter [`breakpoint-exception-flow-type.md`](../../charter/objects/breakpoint-exception-flow-type.md), model
[`breakpoint_exception_flow_type.spec`](../../model/objects/breakpoint_exception_flow_type.spec), implementation
[`breakpoint_exception_flow_type.rs`](../../../impl/arceos_ex/src/objects/breakpoint_exception_flow_type.rs).
