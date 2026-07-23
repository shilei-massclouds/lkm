# Riscv64Platform coding mapping

`impl/arceos_ex/src/systems/riscv64_platform.rs` records metadata only. Current assembly and Rust entry code are the
runtime materialization of BootHartContext register updates. This round changes specification ownership, not CSR/GPR
instructions, platform boot order, QEMU wiring, or checkpoint locations.
