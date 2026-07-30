# Coding Systems

System-level coding constraints mirror the unique top-level system tree.

- [`computer.md`](computer.md): metadata-only Computer specification, construction, and root mapping.
- [`riscv64-platform.md`](riscv64-platform.md): metadata-only platform specification/context mapping.
- [`opensbi.md`](opensbi.md): metadata-only firmware construction and handoff mapping.
- [`kernel.md`](kernel.md): the implemented Kernel system guidance. It describes
  the orchestration chain that sequences the top-level phase tree.
- [`soc.md`](soc.md): empty by design; Soc has no specialized lowering constraints and inherits the default Coding rules.
