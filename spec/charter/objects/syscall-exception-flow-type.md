# SyscallExceptionFlowType

`SyscallExceptionFlowType` 是一次 syscall occurrence，parent 固定为入口 CPU 的
`SyscallExceptionType`。它允许调度和跨 CPU 恢复；迁移只改变 TaskFlow 的当前 CpuRef 与目标 CPU
入口上下文，不改变本 occurrence 的 parent。返回值和返回检查点必须在完整 Cleanup 前提交。

## Mapping

- Model: `spec/model/objects/syscall_exception_flow_type.spec`
- Coding: `spec/coding/objects/syscall-exception-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/syscall_exception_flow_type.rs`
