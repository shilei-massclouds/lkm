# ComputerProject coding mapping

`impl/arceos_ex/src/projects/computer.rs` is metadata-only. It records the charter/model/coding chain and must not
implement runtime startup, emit checkpoints, or synthesize child-project state. The model is authoritative for the
ordered Hardware/Firmware/Kernel project orchestration and the Computer assembly fact.
The runtime handoff maps specifically to `ComputerProject.Enable` driving the already Ready
`Computer.Enable`; it must not be lowered as `Computer.Preset` or as a synthetic project checkpoint.
