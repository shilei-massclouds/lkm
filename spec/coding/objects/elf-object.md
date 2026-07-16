# ElfObject coding contract

Model source: [`elf_object.spec`](../../model/objects/elf_object.spec). Implementation:
`impl/arceos_ex/src/objects/elf_object.rs`.

All ELF constants, header/program-header parsing, `ElfObjectRole`, `ElfError`, `ElfLoadSegment`, and
`ElfObject`'s `struct + impl` live in this module. `user_boot.rs` may consume/re-export the value types for
compatibility but must not contain an ELF parser or format-dispatch pipeline.

`ElfObject` consumes kernel-owned image bytes. `preset_*` validates class/endian/machine/type and role;
`setup` constructs the bounded PT_LOAD/load-bias/entry/interpreter/auxv plan; `bind_runtime_interpreter`
links the main and interpreter artifacts. Mapping allocation remains a `UserAddressSpace` responsibility,
and transaction commit remains an `ExecTransaction` responsibility.

There is no `ElfLoader` lifecycle object. `ElfBinaryFormat` is a stateless registry handler value that creates
or resets these artifacts inside the active transaction.
