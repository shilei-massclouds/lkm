# UnexpectedExceptionType

`UnexpectedExceptionType` 是每 CPU `ExceptionType.unexpected` 的兜底资源。它覆盖所有未被更具体资源
接受的异常并记录完整执行身份后 fail-and-shutdown，不提供默认嵌套或恢复。每次到达兜底边界仍创建
并清理可诊断的 fresh `UnexpectedExceptionFlowType`。

`Preset` 建立这一终止性初始兜底，使正式响应入口发布后任何尚未被其它具体资源接受的异常都有确定
去向；尚未闭合的诊断细节可以保留为 deferred obligation，但不得形成静默返回路径。

## Mapping

- Model: `spec/model/objects/unexpected_exception_type.spec`
- Coding: `spec/coding/objects/unexpected-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/unexpected_exception_type.rs`
