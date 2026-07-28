# Charter Objects

Object-level charter notes live here when they need a dedicated topic file.

## Exec core objects

| Object | Authoritative charter |
| --- | --- |
| `CPU` | [`cpu.md`](cpu.md) |
| `CpuGroup` | [`cpu-group.md`](cpu-group.md) |
| `Task` | [`task.md`](task.md) |
| `TaskFlow` | [`task-flow.md`](task-flow.md) |
| Runtime Type instance / `declare` | [`dynamic-instance-declaration.md`](dynamic-instance-declaration.md) |
| `ExecTransaction` | [`exec-transaction.md`](exec-transaction.md) |
| `UserStack` | [`user-stack.md`](user-stack.md) |
| `BinaryFormatRegistry` | [`binary-format-registry.md`](binary-format-registry.md) |
| `ExecSyncBoundaries` | [`exec-sync-boundaries.md`](exec-sync-boundaries.md) |
| `ElfObject` | [`elf-object.md`](elf-object.md) |

The exec entries are lifecycle objects in the shared boot/runtime exec pipeline; the runtime-instance topic
defines the common creation and identity mechanism used by any declared Type. `BinaryFormatHandler` and
`ElfBinaryFormat` are registry entry/type contracts; `ExecArguments`, `ExecError` and `ExecOwner` are value
types. None of them adds another four-layer object family.

Formal object specifications live under [`../../model/objects/`](../../model/objects/). Their complete
file-level implementation mapping is maintained in
[`../../coding/objects/README.md`](../../coding/objects/README.md); charter topics describe intent and do
not replace that coverage table.
