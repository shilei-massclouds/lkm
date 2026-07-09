# Coding Systems

System-level coding constraints live here when they need a dedicated topic file.

- `kernel.spec` is the formal Kernel system coding rule index. It also
  carries the `arceos_ex` startup/payload handoff coding type that belongs
  to the Kernel system boundary.
- `kernel.md` carries the explanatory text for those formal predicates.

`../arceos_ex.spec` includes `systems/kernel.spec` so the historical
`arceos_ex` entry still reaches `ArceosExStartupPhaseCodingMust`.
