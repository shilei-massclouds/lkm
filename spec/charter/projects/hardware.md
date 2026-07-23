# Hardware Project Charter

`HardwareProject` 负责平台规格和硬件构造产物，不负责运行时启动平台。

## Lifecycle and products

- Preset 采纳外部 RISC-V ISA 标准 `Riscv64`，建立 `Riscv64Platform` 系统规格，然后停在 Prepared。
- Setup 构造 `Riscv64Platform` 及其处于 Base 的 `BootHartContext`，然后停在 Ready。

`Riscv64` 是只读的外部 ISA 规格，parent 是 `HardwareProject`。它只描述 ISA 能力，不保存某个 hart
启动过程中的 `a0/a1/sp/tp/gp` 或 supervisor CSR。

`BootHartContext` 保存 boot hart 的可变 GPR/CSR，parent 是运行系统 `Riscv64Platform`；它由
`HardwareProject.Setup` 构造，但只在平台收到启动信号后随平台三个阶段推进。

## Mapping

- Model: `spec/model/projects/hardware.spec`
- Coding: `spec/coding/projects/hardware.md`
- Implementation: `impl/arceos_ex/src/projects/hardware.rs`
