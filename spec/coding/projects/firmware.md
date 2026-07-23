# FirmwareProject coding mapping

`impl/arceos_ex/src/projects/firmware.rs` is metadata-only. It records ownership of SbiSpec, the immutable,
initially Online BootArgs ABI argument object and the constructed OpenSBI image. FirmwareProject specifies or
adopts the argument types and meaning but does not construct the instance through Setup. Existing
`BootArgs::new(a0, a1)` remains the Kernel entry boundary's local materialization of the already-given boot ABI
arguments; this round does not alter assembly or Rust startup control flow.
