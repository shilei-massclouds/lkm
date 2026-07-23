# Riscv64Platform coding mapping

`impl/arceos_ex/src/systems/riscv64_platform.rs` records metadata only. Current assembly and Rust entry code are the
runtime materialization of updates to the BootCPU-owned `BootCpuRegisters`; they are not platform lifecycle effects.
The platform metadata maps an initially Ready system and its single Enable transition, which hands off to
`OpenSBI.Enable`. CSR/GPR instructions, platform boot order, QEMU wiring, and checkpoint locations remain unchanged;
no Preset/Setup platform checkpoint or runtime function is synthesized.
