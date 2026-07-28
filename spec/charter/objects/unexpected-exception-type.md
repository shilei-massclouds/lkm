# UnexpectedExceptionType

`UnexpectedExceptionType` 是每 CPU `ExceptionType.unexpected` 的兜底资源。它覆盖所有未被更具体资源
接受的异常并记录完整执行身份后 fail-and-shutdown，不提供默认嵌套或恢复。每次到达兜底边界仍创建
并清理可诊断的 fresh `UnexpectedExceptionFlowType`。

## Mapping

- Model: `spec/model/objects/unexpected_exception_type.spec`
- Coding: `spec/coding/objects/unexpected-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/unexpected_exception_type.rs`
