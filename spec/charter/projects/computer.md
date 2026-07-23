# Computer Project Charter

`ComputerProject` 是工程树的唯一根。它编排三个彼此独立的子工程，建立静态组装事实，并把运行启动
交给 `Computer`；它不是运行中的计算机系统。

## Lifecycle

- `Preset -> Prepared` 按 `HardwareProject`、`FirmwareProject`、`KernelProject` 的固定顺序同步发送
  `Preset`。三个子工程均到达 Prepared 后，`ComputerProject` 提交 Prepared 并异步发送自身 `Setup`。
- `Setup -> Ready` 按相同顺序同步发送三个子工程的 `Setup`。三个子工程均到达 Ready 后，建立
  `Computer` 已由 `Riscv64Platform`、`OpenSBI`、`Kernel` 三个运行系统静态组装的事实，并异步发送
  自身 `Enable`。
- `Enable -> Online` 只同步发送 `Computer.Enable`。`Computer` 在模型初始观察点已经组装并处于
  Ready；该 Enable 才实际启动运行系统并把控制权继续交给平台。Online 仅表示工程已把启动交给
  运行系统，不表示异步下游已全部 Online。

三个子工程不执行 Enable；它们的终态是 Ready。

## Boundary

静态 parent/assembly 表达构成关系；运行时启动使用显式 `drives`/`emits`。本轮不引入动态 `owned`
关系或新的 Signal DSL。全局 `Startup` 继续只是 `Preset` 的外部别名；它不改名、不重绑定为
`Enable`，而工程到已经 Ready 的运行系统使用 canonical `Enable` 信号。

## Mapping

- Model: `spec/model/projects/computer.spec`
- Coding: `spec/coding/projects/computer.md`
- Implementation: `impl/arceos_ex/src/projects/computer.rs`
