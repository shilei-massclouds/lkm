# OpenSBI System Charter

`OpenSBI` 是 `Computer` 的直接子系统和固件运行实例，承担 SBI/OpenSBI 规格采纳、固件构造与 Kernel
交接。只读外部标准 `SbiSpec` 和只读启动 ABI 实参 `BootArgs` 的静态 parent 均为 `OpenSBI`。

OpenSBI 初态为 Base：

- `Preset` 采纳 `SbiSpec`、规定 `BootArgs` 的类型和含义并建立 OpenSBI 系统规格，提交 Prepared；它
  不构造或改变初态 Online 的只读输入实例。
- `Setup` 验证规格并构造 OpenSBI 固件，提交 Ready。
- `Enable` 一次性验证平台 Online、固件、SBI/BootArgs、Config/Lds、kernel image、固件/DTB 与有序
  启动前提。它精确保证
`BootCpuRegisters.a0 == BootArgs.boot_hartid` 与
`BootCpuRegisters.a1 == BootArgs.dtb_pa`，提交 Online 后异步发送 `Kernel.Enable`。这条边界表达控制权
交接；它不表示其它启动相关寄存器已经具有 Kernel 最终值。OpenSBI 不拥有 Kernel，也不拥有 Kernel
的内部 phase。

OpenSBI.Online 只表示固件实例已经启动并提交 Kernel 入口控制权；下游失败不回滚该状态。

## Mapping

- Model: `spec/model/systems/opensbi.spec`
- Coding: `spec/coding/systems/opensbi.md`
- Implementation: `impl/arceos_ex/src/systems/opensbi.rs`
