# Computer System Charter

`Computer` 是完整模型唯一的顶层系统和显式根，由 `Riscv64Platform`、`OpenSBI` 和 `Kernel` 三个
直接子系统静态组装。该 parent 树表达结构，不产生隐式信号传播。模型外部的 `Human` 是唯一启动
编排者：它先同步 drives `Computer.Preset`，成功后同步 drives `Computer.Setup`，两者都成功后才
异步 emits `Computer.Enable`。

`Computer` 初态为 Base，并承担原先分散的顶层规格编排、构造编排和运行交接：

- `Preset` 按 `Riscv64Platform`、`OpenSBI`、`Kernel` 的固定顺序同步发送 Preset。三者均为 Prepared
  后提交自身 Prepared；它不发送 Computer 的其它 lifecycle Signal。
- `Setup` 按相同顺序同步发送三个子系统的 Setup。三者均为 Ready 后建立 Computer 由三者组装的
  assembly fact，提交自身 Ready；它不发送 Computer 的其它 lifecycle Signal。
- `Enable` 验证 assembly fact，提交自身 Online，然后异步发送 `Riscv64Platform.Enable`。

三个 Computer handler 互不触发。Human 的两次 `drives` 严格按声明顺序完成，任一步失败立即短路，
不得创建后续 Computer Signal；`Computer.Enable` 只有在前两步成功后才进入全局 FIFO。Human 不属于
模型对象树，不具有 lifecycle、parent 或伪造的 `Human.Startup` handler。

Computer.Online 只表示 Computer 实例自身已经启动并把控制权交给平台；它不等待平台、固件、内核
或内部启动闭包完成。提交后的异步失败不回滚 Computer.Online，但必须使根请求失败。

全局 `Startup` 仍是 `Preset` 的外部别名，不是 `Enable` 的别名，因此 `Computer.Startup` 正是
`Computer.Preset` 的外部写法。

## Mapping

- Model: `spec/model/systems/computer.spec`
- Coding: `spec/coding/systems/computer.md`
- Implementation: `impl/arceos_ex/src/systems/computer.rs`
