# PageFaultExceptionFlowType

This occurrence binds to `PageFaultExceptionType`, records fault address/access/source, and is schedulable only for a
recoverable non-atomic fault. Migration preserves the root's entry-CPU parent and updates only return-CPU/context facts.

Mapping: charter [`page-fault-exception-flow-type.md`](../../charter/objects/page-fault-exception-flow-type.md), model
[`page_fault_exception_flow_type.spec`](../../model/objects/page_fault_exception_flow_type.spec), implementation
[`page_fault_exception_flow_type.rs`](../../../impl/arceos_ex/src/objects/page_fault_exception_flow_type.rs).
