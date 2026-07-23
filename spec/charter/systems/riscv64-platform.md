# RISC-V 64 Platform System Charter

`Riscv64Platform` 是 `Computer` 的直接子系统。它消费 `HardwareProject` 已建立的平台规格和构造事实，
不重新定义 ISA 标准。

平台的 Preset、Setup、Enable 只推进自身生命周期，并通过自发出的 completion Signal 形成独立
`Preset -> Setup -> Enable` 连锁。平台 Enable 提交 Online 后异步发送 `OpenSBI.Startup`。

启动 CPU 的寄存器不属于平台，也不随平台三阶段同步推进。`BootCpuRegisters` 是 `BootCPU` 的直接
子对象且初态为 Online；ISA 能力条件仍引用静态 `Riscv64`。

## Mapping

- Model: `spec/model/systems/riscv64-platform.spec`
- Coding: `spec/coding/systems/riscv64-platform.md`
- Implementation: `impl/arceos_ex/src/systems/riscv64_platform.rs`
