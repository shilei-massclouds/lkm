# PageFaultExceptionType

`PageFaultExceptionType` 是每 CPU `ExceptionType.page_fault` 资源。它拥有 page-fault handler binding、
fixup 能力与上下文分类。用户态或其它可恢复、可睡眠来源可以调度并迁移；atomic/hardirq page fault
只能完成已证明安全的 fixup，否则 fatal。每次处理使用 fresh `PageFaultExceptionFlowType`。

`Preset` 只建立 page fault 的初始兜底，使尚未具备正式 handler 或恢复能力的 fault 具有确定处理
去向；这些更完整的能力由后续 lifecycle 建立，尚未闭合的部分可以保留为 deferred obligation。

## Mapping

- Model: `spec/model/objects/page_fault_exception_type.spec`
- Coding: `spec/coding/objects/page-fault-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/page_fault_exception_type.rs`
