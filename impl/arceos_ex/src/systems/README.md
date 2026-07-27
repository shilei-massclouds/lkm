# arceos_ex Systems

Metadata modules mirror Computer and its Riscv64Platform/OpenSBI/Kernel direct children. Only `kernel.rs` retains a
real runtime lifecycle in this round; the other three modules record mapping metadata without changing startup flow.
The external `LinuxRiscv64KernelBootSpec` remains read-only specification metadata owned by Kernel rather than a
runtime module. Its physical PMD placement and live entry `satp == 0` requirements are checked at the existing
Kernel.Enable adoption boundary; interrupt CSR normalization remains the first BootInitFlow action.
