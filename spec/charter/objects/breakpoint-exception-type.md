# BreakpointExceptionType

`BreakpointExceptionType` 是每 CPU `ExceptionType.breakpoint` 资源。它拥有 hook/handler binding 与恢复
策略。breakpoint 不默认开放异常或中断嵌套；来源与上下文规则必须明确允许。每次处理使用 fresh
`BreakpointExceptionFlowType`。

## Mapping

- Model: `spec/model/objects/breakpoint_exception_type.spec`
- Coding: `spec/coding/objects/breakpoint-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/breakpoint_exception_type.rs`
