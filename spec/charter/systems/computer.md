# Computer System Charter

`Computer` 是运行系统树的显式根，由 `Riscv64Platform`、`OpenSBI` 和 `Kernel` 三个直接子系统静态
组装。该 parent 树表达结构，不产生隐式信号传播。

`Computer` 在完整模型的初始观察点已经组装、可启动，因此初态为 Ready；Ready 不表示已经运行。
它只接受显式 `Computer.Enable` 启动，提交 Online 后异步发送 `Riscv64Platform.Enable`。因此
Computer Online 表示启动链已交给平台，并不等待平台、固件或内核完成。

全局 `Startup` 仍是 `Preset` 的外部别名，不是 `Enable` 的别名。由于 Computer 不再有 Preset
handler，`Computer.Startup` 不是这条 Ready-to-Online 启动链的入口。

## Mapping

- Model: `spec/model/systems/computer.spec`
- Coding: `spec/coding/systems/computer.md`
- Implementation: `impl/arceos_ex/src/systems/computer.rs`
