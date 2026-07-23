# RISC-V 64 Platform System Charter

`Riscv64Platform` 是 `Computer` 的直接子系统。它消费 `HardwareProject` 已建立的平台规格和构造事实，
不重新定义 ISA 标准。

平台的 Preset、Setup、Enable 分别同步推进 `BootHartContext` 的同名阶段。平台 Enable 提交 Online 后
异步发送 `OpenSBI.Startup`。`BootHartContext` 是平台的直接子系统，保存本次 boot hart 的可变
`a0/a1/sp/tp/gp` 与 supervisor CSR；ISA 能力条件仍引用静态 `Riscv64`。

## Mapping

- Model: `spec/model/systems/riscv64-platform.spec`
- Coding: `spec/coding/systems/riscv64-platform.md`
- Implementation: `impl/arceos_ex/src/systems/riscv64_platform.rs`
