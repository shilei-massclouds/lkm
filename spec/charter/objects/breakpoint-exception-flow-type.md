# BreakpointExceptionFlowType

`BreakpointExceptionFlowType` 是一次 breakpoint occurrence，parent 固定为入口 CPU 的
`BreakpointExceptionType`。它保存 hook 选择、恢复地址与处理结果；嵌套只由来源和上下文的明确规则
允许，不因 breakpoint 类型自动开放。

## Mapping

- Model: `spec/model/objects/breakpoint_exception_flow_type.spec`
- Coding: `spec/coding/objects/breakpoint-exception-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/breakpoint_exception_flow_type.rs`
