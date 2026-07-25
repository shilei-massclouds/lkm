# RISC-V 64 Platform System Charter

`Riscv64Platform` 是 `Computer` 的直接子系统，承担平台规格采纳、平台构造与运行交接。只读外部 ISA
规格 `Riscv64` 的静态 parent 是 `Riscv64Platform`；平台消费它，但不重新定义 ISA 标准。

平台初态为 Base：

- `Preset` 采纳 `Riscv64` 能力并建立平台系统规格，提交 Prepared。
- `Setup` 验证平台规格并构造平台，提交 Ready。
- `Enable` 验证 `Computer.Online`、静态 ISA、平台规格与构造事实，提交 Online，然后异步发送
  `OpenSBI.Enable`。

Riscv64Platform.Online 只表示平台实例已经启动并把控制权交给固件；异步固件失败不回滚该状态。

启动 CPU 的寄存器不属于平台，也不随平台 Enable 同步推进。`BootCpuRegisters` 是 `BootCPU` 的直接
子对象且初态为 Online；ISA 能力条件仍引用静态 `Riscv64`。

## Mapping

- Model: `spec/model/systems/riscv64-platform.spec`
- Coding: `spec/coding/systems/riscv64-platform.md`
- Implementation: `impl/arceos_ex/src/systems/riscv64_platform.rs`
