# BinaryFormatRegistry coding contract

Model source: [`binary_format_registry.spec`](../../model/objects/binary_format_registry.spec).
Implementation: `impl/arceos_ex/src/objects/binary_format_registry.rs`.

`Context` owns the fixed registry. `InitcallPhase` calls `setup()` once after `InitcallTable` is Ready and
registers exactly one `BinaryFormatHandler::Elf(ElfBinaryFormat)` entry. The registry uses a fixed array/no
heap and becomes immutable after Ready.

`dispatch()` accepts only kernel-owned bytes plus the active transaction staging objects. It never copies a
user pointer, opens an fd, writes SATP, changes current mm or emits a successful commit. Handler rejection is
distinct from allocation/read failures; exhausting the entry table with no accepted handler maps to
`ExecError::NoExecutableFormat` / `ENOEXEC`.

Script, misc, dynamic handler registration and request-module retry remain explicit deferred/trimmed facts.
Do not add a persistent loader or route ELF parsing directly from `SyscallTable`/`UserBootPayload`.
