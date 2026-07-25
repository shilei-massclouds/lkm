# OpenSBI system coding mapping

`impl/arceos_ex/src/systems/opensbi.rs` is metadata-only. It maps Base/Prepared/Ready/Online, adoption of `SbiSpec`
and immutable `BootArgs`, firmware construction, and the post-commit `Kernel.Enable` handoff. `BootArgs::new(a0,
a1)` remains the Kernel entry boundary's local materialization of the already-given boot ABI arguments.

Existing OpenSBI supplies `a0/a1` and transfers control to the kernel image. The formal handoff equates only
`BootCpuRegisters.a0/a1` with `BootArgs.boot_hartid/dtb_pa`; it must not synthesize the remaining register subset.
The metadata validates platform/firmware/static-image prerequisites and records the DTB, BootTaskRef and entry ABI
facts. No firmware binary, build recipe, or executable OpenSBI control flow is added here.
