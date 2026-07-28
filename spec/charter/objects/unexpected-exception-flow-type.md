# UnexpectedExceptionFlowType

`UnexpectedExceptionFlowType` 是一次兜底异常 occurrence，parent 固定为入口 CPU 的
`UnexpectedExceptionType`。它记录入口 CPU、返回/当前 CPU、TaskRef、TaskFlowRef、root/leaf FlowRef、
栈界限和原因后进入 fail-and-shutdown；不得转为可恢复路径或全局 fallback。

## Mapping

- Model: `spec/model/objects/unexpected_exception_flow_type.spec`
- Coding: `spec/coding/objects/unexpected-exception-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/unexpected_exception_flow_type.rs`
