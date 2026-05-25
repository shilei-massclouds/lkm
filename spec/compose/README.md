# Compose 规格

本目录记录 `Composition Phase` 的补充约束。

`Composition Phase` 位于 `Object Coding Phase` 之后。它不重新定义模型对象、状态、事件、依赖和阶段顺序，而是在对象级编码实现已经满足规格语义的前提下，决定这些对象如何被组合、封装和发布。

## 阶段目标

`Composition Phase` 关注以下问题：

- 哪些对象级实现放入哪些 crate。
- crate 内部如何拆分 Rust module。
- crate 和 module 的公开接口如何设计。
- 哪些接口需要 adapter 或 facade。
- 构建工具如何选择目标内核、平台、应用和 feature。
- 如何让新实现尽量接近参考内核的组件形态，便于演化和维护。

该阶段不得改变对象级语义。若组合封装需求与模型事件、状态迁移或依赖关系冲突，应回到 `spec/model` 或 `spec/coding/` 处理，而不是在封装层隐式改变行为。

## 与 Coding 的分工

`spec/coding/` 负责对象级编码规则：

- 对象由什么 Rust 类型、静态单例或启动上下文字段承载。
- 事件如何推进状态。
- 依赖和后置事实如何检查。
- checkpoint/hook 如何对应状态一致点。
- 架构、语言、安全和外部 crate 信任边界。

`spec/compose/` 负责组合封装规则：

- 对象级实现如何组织为 crate/module。
- 哪些 public API 需要保持与参考实现接近。
- 哪些内部实现可以重构，哪些接口需要兼容。
- overlay workspace、构建配置和应用选择如何接入。

代码物理位置不等于最终组件边界。早期实现可以先把对象级代码放入方便运行的 crate 中；后续整理时，应按本目录规则重新审视 crate/module 边界。

## ArceOS 组合原则

`arceos_ex` 的组合封装目标是成为 ArceOS 的规格化演化候选，而不是另起一套完全无关的工程结构。因此在 `Composition Phase` 中应遵循：

- 组件形态尽量沿用 ArceOS 的 crate 边界和命名习惯。
- crate 内部 module 的公开接口尽量接近 ArceOS。
- `ax-std`、`ax-api`、`ax-feat` 这类应用接口层第一轮不主动复制；优先通过构建工具把底层实现切换到 `_ex` 组件。
- 接口兼容是优先目标，但不是百分百硬约束。若规格语义、对象边界或演化路径要求改变接口，应记录原因。
- `os/arceos` 和已有 `components` 实现作为只读参考；`arceos_ex` 通过新增目录、overlay workspace 或新增 `_ex` 组件接入。
- 第一轮只参考并接入 ArceOS 的 `ax-std` Unikernel 路径；不参考、不适配也不调试 ArceOS C API 和 Rust std/Hermit API 路径。

组合封装阶段可以复用 ArceOS 的接口形状、目录习惯和成熟构建路径，但不得因为 ArceOS 现有模块边界而改变模型对象的状态迁移。

## 输出 Facade

对象级输出路径应先由 `PrintkBuffer`、`EarlyCon` 和后续正式 `Console` 承担。`Object Coding Phase` 可以先引入启动期内部 `printk`/`println-like` 前端，用于输出 `arceos_ex` 启动 banner；应用侧 `axstd::println!` 是另一个前端入口，用于 `helloworld` 等 Unikernel payload。二者不是同一个入口，但应汇聚到同一条缓冲路径；`info!/debug!/warn!/error!` 和 panic 输出等更完整前端后续也应汇聚到该路径：

```text
frontend -> PrintkBuffer.write(...) -> ring buffer -> EarlyCon/Console drain
```

`axlog` 属于 ArceOS 风格的上层日志 facade。它可以在 `Composition Phase` 中提供日志级别过滤、格式化、时间戳、CPU/task 标识、颜色和 `LogIf` 适配，但不应反向改变对象级输出语义。对象级编码阶段可以先实现启动期内部输出前端和应用侧 `axstd::println!` 到 `PrintkBuffer -> EarlyCon(SBI)` 的最小闭环；后续再把 `axlog` 接到同一 `PrintkBuffer` 路径。
