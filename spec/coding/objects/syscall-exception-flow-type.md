# SyscallExceptionFlowType

This occurrence binds to `SyscallExceptionType`, saves the user return checkpoint and drives the selected syscall-table
entry. It may schedule/migrate, but the underlying TaskFlow remains execution authority and resumes only after terminal
Cleanup and one-shot return-token consumption.

Mapping: charter [`syscall-exception-flow-type.md`](../../charter/objects/syscall-exception-flow-type.md), model
[`syscall_exception_flow_type.spec`](../../model/objects/syscall_exception_flow_type.spec), implementation
[`syscall_exception_flow_type.rs`](../../../impl/arceos_ex/src/objects/syscall_exception_flow_type.rs).
