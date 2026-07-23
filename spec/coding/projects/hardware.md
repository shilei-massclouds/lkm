# HardwareProject coding mapping

`impl/arceos_ex/src/projects/hardware.rs` is metadata-only. `Riscv64` names the adopted ISA specification;
boot-hart GPR/CSR storage belongs to the `BootHartContext` system mapping. Project metadata must not add register
state to Riscv64 or drive the runtime platform lifecycle.
