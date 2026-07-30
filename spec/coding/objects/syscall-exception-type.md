# SyscallExceptionType

`SyscallExceptionType` owns the user-ecall binding and its `SyscallTable` relationship. It becomes online only after the
table is ready. Syscall occurrence Flows may schedule and migrate; scheduler commit moves the saved short-term chain
and installs the target CPU entry context without changing the root Flow's entry-CPU parent.

`Preset` installs only the not-yet-ready syscall/ecall fallback consumed by `TrapType.Setup`; it does not publish the
`SyscallTable` or user-return service.

The Linux PLIC foreign ABI may borrow `tp` only inside its assembly shim. Every normal, error and nested return restores
the architectural task `tp` before entering Rust Context, scheduling, or selector resolution.

Mapping: charter [`syscall-exception-type.md`](../../charter/objects/syscall-exception-type.md), model
[`syscall_exception_type.spec`](../../model/objects/syscall_exception_type.spec), implementation
[`syscall_exception_type.rs`](../../../impl/arceos_ex/src/objects/syscall_exception_type.rs).
