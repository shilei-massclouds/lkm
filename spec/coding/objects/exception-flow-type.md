# ExceptionFlowType

`ExceptionFlowType` binds to the entry CPU's `ExceptionType`. Setup classifies `scause`, declares exactly one concrete
Flow, installs its generation-checked reference in the root chain, and synchronously drives it to completion. Cleanup
invalidates the concrete reference before the parent exception Flow reference.

Mapping: charter [`exception-flow-type.md`](../../charter/objects/exception-flow-type.md), model
[`exception_flow_type.spec`](../../model/objects/exception_flow_type.spec), implementation
[`exception_flow_type.rs`](../../../impl/arceos_ex/src/objects/exception_flow_type.rs).
