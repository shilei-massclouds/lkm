# OpenSBI System Charter

`OpenSBI` 是 `Computer` 的直接子系统和固件运行实例。它消费 `FirmwareProject` 已构造的固件、
`SbiSpec` 与只读 `BootArgs`。

OpenSBI 自身按 Preset、Setup、Enable 推进。Enable 使 `BootHartContext.a0/a1` 与
`BootArgs.boot_hartid/dtb_pa` 一致，提交 Online 后异步发送 `Kernel.Startup`。这条边界表达控制权交接；
OpenSBI 不拥有 Kernel，也不拥有 Kernel 的内部 phase。

## Mapping

- Model: `spec/model/systems/opensbi.spec`
- Coding: `spec/coding/systems/opensbi.md`
- Implementation: `impl/arceos_ex/src/systems/opensbi.rs`
