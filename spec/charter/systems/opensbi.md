# OpenSBI System Charter

`OpenSBI` 是 `Computer` 的直接子系统和固件运行实例。它消费 `FirmwareProject` 已构造的固件、
`SbiSpec` 与只读 `BootArgs`。

OpenSBI 自身按 Preset、Setup、Enable 推进。Enable 精确保证
`BootCpuRegisters.a0 == BootArgs.boot_hartid` 与
`BootCpuRegisters.a1 == BootArgs.dtb_pa`，提交 Online 后异步发送 `Kernel.Startup`。这条边界表达控制权
交接；它不表示其它启动相关寄存器已经具有 Kernel 最终值。OpenSBI 不拥有 Kernel，也不拥有 Kernel
的内部 phase。

## Mapping

- Model: `spec/model/systems/opensbi.spec`
- Coding: `spec/coding/systems/opensbi.md`
- Implementation: `impl/arceos_ex/src/systems/opensbi.rs`
