# PageFaultExceptionType

`PageFaultExceptionType` 是每 CPU `ExceptionType.page_fault` 资源。它拥有 page-fault handler binding、
fixup 能力与上下文分类。用户态或其它可恢复、可睡眠来源可以调度并迁移；atomic/hardirq page fault
只能完成已证明安全的 fixup，否则 fatal。每次处理使用 fresh `PageFaultExceptionFlowType`。

## Mapping

- Model: `spec/model/objects/page_fault_exception_type.spec`
- Coding: `spec/coding/objects/page-fault-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/page_fault_exception_type.rs`
