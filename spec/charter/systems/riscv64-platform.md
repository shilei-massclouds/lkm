# RISC-V 64 Platform System Charter

`Riscv64Platform` 是 `Computer` 的直接子系统。它消费 `HardwareProject` 已建立的平台规格和构造事实，
不重新定义 ISA 标准。

平台在完整模型的初始观察点已经组装、可启动，因此初态为 Ready；Ready 不表示已经运行。它只在
收到显式 Enable 后验证 HardwareProject、静态 ISA、平台规格与构造事实，提交 Online，并异步发送
`OpenSBI.Enable`。已无入口意义的 Base/Prepared 与 Preset/Setup 自连锁不属于该运行系统。

启动 CPU 的寄存器不属于平台，也不随平台 Enable 同步推进。`BootCpuRegisters` 是 `BootCPU` 的直接
子对象且初态为 Online；ISA 能力条件仍引用静态 `Riscv64`。

## Mapping

- Model: `spec/model/systems/riscv64-platform.spec`
- Coding: `spec/coding/systems/riscv64-platform.md`
- Implementation: `impl/arceos_ex/src/systems/riscv64_platform.rs`
