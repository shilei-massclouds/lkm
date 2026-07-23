# OpenSBI system coding mapping

`impl/arceos_ex/src/systems/opensbi.rs` records metadata only. Existing OpenSBI supplies `a0/a1` and transfers
control to the kernel image. The formal handoff equates those BootHartContext registers with FirmwareProject's
BootArgs; no firmware binary, build recipe, or actual control flow changes in this round.
