# HardwareProject coding mapping

`impl/arceos_ex/src/projects/hardware.rs` is metadata-only. `Riscv64` names the adopted ISA specification;
the boot CPU's naturally present GPR/CSR subset belongs to `BootCPU.BootCpuRegisters`. Project metadata must not
construct that register object, add register state to Riscv64, or drive the runtime platform lifecycle.
