# Riscv64Platform coding mapping

`impl/arceos_ex/src/systems/riscv64_platform.rs` is metadata-only. It maps Base/Prepared/Ready/Online, adoption of
the `Riscv64` external ISA specification, platform construction, and post-commit `OpenSBI.Enable` handoff.

Current assembly and Rust entry code materialize updates to BootCPU-owned `BootCpuRegisters`; they are not platform
lifecycle effects. CSR/GPR instructions, platform boot order and QEMU wiring remain at their existing boundaries.
No synthetic platform checkpoint or runtime Preset/Setup function is introduced.
