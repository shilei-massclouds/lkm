# OpenSBI system coding mapping

`impl/arceos_ex/src/systems/opensbi.rs` is metadata-only. It maps Base/Prepared/Ready/Online, adoption of `SbiSpec`
and immutable `BootArgs`, firmware construction, and the post-commit `Kernel.Enable` handoff. OpenSBI.Setup directly
establishes `opensbi_firmware_constructed()`; it does not need an additional evidence proxy. `BootArgs::new(a0, a1)`
remains the Kernel entry boundary's local materialization of the already-given boot ABI arguments.

OpenSBI.Enable consumes the Kernel-adopted `LinuxRiscv64KernelBootSpec` and the three concrete image-construction
leaf facts. The formal handoff selects the single primary hart under ordered boot, validates the DTB in accessible
RAM, equates `BootCpuRegisters.a0/a1` with `BootArgs.boot_hartid/dtb_pa`, and writes `BootCpuRegisters.satp == 0`.
The stable `opensbi_kernel_entry_satp_zero_handoff()` fact records that handoff after the live register is later
changed by Kernel VM setup.

The OpenSBI mapping must not synthesize or require `sie/sip == 0`; those registers are normalized by the first
Kernel-owned `InterruptStream.Preset` action. The metadata records the DTB, BootTaskRef and entry ABI facts, but adds
no firmware binary, build recipe, or executable OpenSBI control flow.
