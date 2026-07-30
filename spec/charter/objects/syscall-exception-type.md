# SyscallExceptionType

`SyscallExceptionType` 是每 CPU `ExceptionType.syscall` 资源。它拥有 syscall handler binding 与合法返回
策略。syscall occurrence 可调度并迁移，但长期 TaskFlow 仍是任务执行权来源；每次调用使用 fresh
`SyscallExceptionFlowType`，并在完整清理后才允许返回用户 continuation。

`Preset` 只为尚未建立正式 syscall 服务的 syscall/ecall 建立确定的初始兜底；handler、系统调用表与
合法返回能力由后续 lifecycle 建立，尚未闭合的部分可以保留为 deferred obligation。

## Mapping

- Model: `spec/model/objects/syscall_exception_type.spec`
- Coding: `spec/coding/objects/syscall-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/syscall_exception_type.rs`
