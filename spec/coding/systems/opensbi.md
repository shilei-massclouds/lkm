# OpenSBI system coding mapping

`impl/arceos_ex/src/systems/opensbi.rs` records metadata only. Existing OpenSBI supplies `a0/a1` and transfers
control to the kernel image. The formal handoff equates only `BootCpuRegisters.a0/a1` with FirmwareProject's
`BootArgs.boot_hartid/dtb_pa`; it does not synthesize values for the remaining register subset. No firmware binary,
build recipe, or actual control flow changes in this round. The metadata maps OpenSBI as initially Ready; its single
Enable transition validates the combined firmware/platform/static-image prerequisites, establishes the DTB,
`a0/a1`, and BootTaskRef handoff facts, then emits canonical `Kernel.Preset` (`Kernel.Startup`).
