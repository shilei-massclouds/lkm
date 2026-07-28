# ExceptionFlowType

`ExceptionFlowType` 是一次异常分类 occurrence，正式 parent 始终是入口 CPU 的 `ExceptionType`。
Preset 验证 root 与入口上下文，Setup 必须恰好选择并同步驱动一个具体异常 Flow，Enable 提交分类处理
完成，Disable/Cleanup 先回收具体 child 再回收自身。它不成为 CurrentTask 或 CurrentCPU 的解析来源。

## Mapping

- Model: `spec/model/objects/exception_flow_type.spec`
- Coding: `spec/coding/objects/exception-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/exception_flow_type.rs`
