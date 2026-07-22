# Signal-driven tools2 独立工具链

## 目标与边界

在 `tools2/` 从头建立 Signal 驱动的 `parse -> model -> derive -> check -> view -> render` Python
纵切，并由独立 `pyveri` driver 串联。首期用于验证目标 Signal 语义，不替换 `tools/`，不进入根
`make test`，不解析完整主模型，也不实现 SVG/HTML。

权威设计见 [`../../spec/charter/system-signal.md`](../../spec/charter/system-signal.md)，formal semantics
见 [`../../spec/model/SEMANTICS.md`](../../spec/model/SEMANTICS.md#sem-signal-tools2-001-first-signal-derivation-is-an-isolated-compatibility-semantics)，
实现协议见 [`../../spec/coding/tools2.md`](../../spec/coding/tools2.md)。本文只记录里程碑和实施证据，
不覆盖上述规格。

## 里程碑

1. 首期最小闭环（已完成）：独立包和 producer/version 隔离；必要 DSL 子集；Transition/Action 隐式 Signal；
   drives 同步、emits post-commit FIFO；strict/lossy 分类；层级预算；scenario/snapshot；结构化
   derive/view JSON 和 text renderer；稳定性及端到端测试。
2. 显式 Signal DSL：在单独的 charter/model-first 决策中引入 `signal` 声明和 `on Signal` 语法，
   停止依赖调用表达式的隐式规范化。首期不得提前接受该语法。
3. handler 命名：评审兼容 handler 从同名 Transition/Action 迁移到 `OnPreset` 等显式响应过程的规则，
   包括歧义、重载和迁移诊断。
4. pending 与 continuation：定义接受后等待未来 Signal、保存/恢复 continuation、队列所有权、超时、
   取消和 snapshot 可续跑语义；首期条件失败必须保持 rejected。
5. 交互 HTML：由独立 JavaScript/TypeScript frontend 消费 view schema，实现确定 trace 的前进/后退
   浏览；不得把浏览器变成推导器。与现有
   [`interactive-model-animation.md`](interactive-model-animation.md) 协调 schema，但不删除老静态 SVG。
6. 老工具迁移/退役：只有用户另行明确决定后才能规划。不得以 tools2 覆盖率或版本号自动触发。

## 首期验收证据

2026-07-22 首期实现完成，证据如下：

- `make -C tools2 test-focused`：1 项同步/异步纵切通过。
- `make -C tools2 test`：22 项通过，覆盖协议互拒、include/span、隐式 handler、payload/reference 类型、
  strict/lossy、invariant、层级坐标和预算、FIFO、快照续跑、稳定 JSON、view 投影、文本失败链和
  `pyveri2` 短参数入口。
- `tools/pyveri/bin/pyveri spec/model/main.spec -T /tmp/lkm-tools2-legacy.trace.svg
  --trace-annotations state,transition --strict`：老静态 SVG 生成成功；主模型 `obligation=0`、
  `blocked=0`、`contradiction=0`。
- `git diff --check`：通过。
- 仓库根直接 `make test`：最终汇总 `182/182`；native/linux-object KUnit 各 `25/25`，app smoke
  各 `55/55`，LTP close list acceptance 两侧通过。

首期已闭环；主 Roadmap 保持“进行中”只表示里程碑 2–6 仍需未来独立决策和实施，不表示首期缺少
验收责任。
