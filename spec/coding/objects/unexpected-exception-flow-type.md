# UnexpectedExceptionFlowType

This occurrence binds to `UnexpectedExceptionType`, emits the complete failure diagnostic and transitions terminally.
No return token is consumable for this flow and no nesting or scheduling window is opened.

Mapping: charter [`unexpected-exception-flow-type.md`](../../charter/objects/unexpected-exception-flow-type.md), model
[`unexpected_exception_flow_type.spec`](../../model/objects/unexpected_exception_flow_type.spec), implementation
[`unexpected_exception_flow_type.rs`](../../../impl/arceos_ex/src/objects/unexpected_exception_flow_type.rs).
