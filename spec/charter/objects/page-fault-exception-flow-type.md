# PageFaultExceptionFlowType

`PageFaultExceptionFlowType` 是一次 page fault occurrence，parent 固定为入口 CPU 的
`PageFaultExceptionType`。它保存来源/访问分类和 fixup 或恢复结果；只有可睡眠来源可以通过调度保存
root TrapFlowRef 并在目标 CPU 恢复整条链，atomic/hardirq 来源不得调度。

## Mapping

- Model: `spec/model/objects/page_fault_exception_flow_type.spec`
- Coding: `spec/coding/objects/page-fault-exception-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/page_fault_exception_flow_type.rs`
