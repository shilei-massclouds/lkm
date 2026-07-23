# FirmwareProject coding mapping

`impl/arceos_ex/src/projects/firmware.rs` is metadata-only. It records ownership of SbiSpec, the immutable BootArgs
product and the constructed OpenSBI image. Existing `BootArgs::new(a0, a1)` remains the Kernel entry boundary's
local materialization of the already specified FirmwareProject ABI; this round does not alter assembly or Rust
startup control flow.
