# Computer system coding mapping

`impl/arceos_ex/src/systems/computer.rs` is metadata-only and records the charter/model/coding chain for the unique
top-level System. It must map Base/Prepared/Ready/Online plus the ordered child Preset/Setup and assembly fact; it
must not add a second executable startup state machine or synthesize checkpoints. Human remains outside the model:
it synchronously drives Computer.Preset and Computer.Setup in that order, then asynchronously emits
Computer.Enable. None of the three Computer handlers triggers another Computer handler.

The real binary begins at the existing firmware/kernel entry path. That runtime boundary adopts the canonical
sending snapshot: Computer Online, Riscv64Platform/OpenSBI Online, Kernel Ready, and Config/Lds Online. Rust does
not replay Human, Computer, platform, firmware, Config, or Lds construction. Metadata must still preserve the model
order and the post-commit asynchronous `Riscv64Platform.Enable` handoff.
