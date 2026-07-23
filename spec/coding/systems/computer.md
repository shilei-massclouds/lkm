# Computer system coding mapping

`impl/arceos_ex/src/systems/computer.rs` records metadata only. The real binary still begins at the existing
firmware/kernel entry path; this module must not add a second startup state machine or checkpoint stream.
