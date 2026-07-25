# Computer System Charter

`Computer` 是完整模型唯一的顶层系统和显式根，由 `Riscv64Platform`、`OpenSBI` 和 `Kernel` 三个
直接子系统静态组装。该 parent 树表达结构，不产生隐式信号传播。唯一无需预制条件的完整模型入口是
`Human -> Computer.Preset`。

`Computer` 初态为 Base，并承担原先分散的顶层规格编排、构造编排和运行交接：

- `Preset` 按 `Riscv64Platform`、`OpenSBI`、`Kernel` 的固定顺序同步发送 Preset。三者均为 Prepared
  后提交自身 Prepared，并异步发送自身 Setup。
- `Setup` 按相同顺序同步发送三个子系统的 Setup。三者均为 Ready 后建立 Computer 由三者组装的
  assembly fact，提交自身 Ready，并异步发送自身 Enable。
- `Enable` 验证 assembly fact，提交自身 Online，然后异步发送 `Riscv64Platform.Enable`。

Computer.Online 只表示 Computer 实例自身已经启动并把控制权交给平台；它不等待平台、固件、内核
或内部启动闭包完成。提交后的异步失败不回滚 Computer.Online，但必须使根请求失败。

全局 `Startup` 仍是 `Preset` 的外部别名，不是 `Enable` 的别名，因此 `Computer.Startup` 正是
`Computer.Preset` 的外部写法。

## Mapping

- Model: `spec/model/systems/computer.spec`
- Coding: `spec/coding/systems/computer.md`
- Implementation: `impl/arceos_ex/src/systems/computer.rs`
