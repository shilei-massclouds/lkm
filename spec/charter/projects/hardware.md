# Hardware Project Charter

`HardwareProject` 负责平台规格和硬件构造产物，不负责运行时启动平台。

## Lifecycle and products

- Preset 采纳外部 RISC-V ISA 标准 `Riscv64`，建立 `Riscv64Platform` 系统规格，然后停在 Prepared。
- Setup 构造 `Riscv64Platform`，然后停在 Ready。

`Riscv64` 是只读的外部 ISA 规格，parent 是 `HardwareProject`。它只描述 ISA 能力，不保存某个 hart
启动过程中的 `a0/a1/sp/tp/gp` 或 supervisor CSR。

`BootCpuRegisters` 表示 boot CPU 天然存在的启动相关 GPR/CSR 子集，parent 是 `BootCPU`，初态直接为
Online。HardwareProject 不构造或推进该对象；Online 只保证寄存器属性可访问，不为尚未由固件或内核
确定的寄存器合成值。

## Mapping

- Model: `spec/model/projects/hardware.spec`
- Coding: `spec/coding/projects/hardware.md`
- Implementation: `impl/arceos_ex/src/projects/hardware.rs`
