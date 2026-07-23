# Computer System Charter

`Computer` 是运行系统树的显式根，由 `Riscv64Platform`、`OpenSBI` 和 `Kernel` 三个直接子系统静态
组装。该 parent 树表达结构，不产生隐式信号传播。

`Computer.Startup` 是 `Computer.Preset` 的外部别名。Preset、Setup、Enable 只推进 Computer 自身；
Enable 提交 Online 后异步发送 `Riscv64Platform.Startup`。因此 Computer Online 表示启动链已交给
平台，并不等待平台、固件或内核完成。

## Mapping

- Model: `spec/model/systems/computer.spec`
- Coding: `spec/coding/systems/computer.md`
- Implementation: `impl/arceos_ex/src/systems/computer.rs`
