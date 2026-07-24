# Signal-driven tools2 独立工具链

## 目标与边界

在 `tools2/` 从头建立 Signal 驱动的 `parse -> model -> derive -> check -> view -> render/animate` Python
纵切，并由独立 `pyveri` driver 串联。它不替换 `tools/`，不进入根 `make test`；当前 v4 工具链已经
能够消费完整主模型并生成 text 与独立离线 HTML，仍不实现或复用老静态 SVG。

权威设计见 [`../../spec/charter/system-signal.md`](../../spec/charter/system-signal.md)，formal semantics
见 [`../../spec/model/SEMANTICS.md`](../../spec/model/SEMANTICS.md#sem-signal-tools2-001-first-signal-derivation-is-an-isolated-compatibility-semantics)，
实现协议见 [`../../spec/coding/tools2.md`](../../spec/coding/tools2.md)。本文只记录里程碑和实施证据，
不覆盖上述规格。

## 里程碑

1. 首期最小闭环（已完成）：独立包和 producer/version 隔离；必要 DSL 子集；Transition/Action 隐式 Signal；
   drives 同步、emits post-commit FIFO；严格 Signal；层级预算；scenario/snapshot；结构化
   derive/view JSON 和 text renderer；稳定性及端到端测试。
2. 显式 Signal DSL：在单独的 charter/model-first 决策中引入 `signal` 声明和 `on Signal` 语法，
   停止依赖调用表达式的隐式规范化。首期不得提前接受该语法。
3. handler 命名：评审兼容 handler 从同名 Transition/Action 迁移到 `OnPreset` 等显式响应过程的规则，
   包括歧义、重载和迁移诊断。
4. pending 与 continuation：定义接受后等待未来 Signal、保存/恢复 continuation、队列所有权、超时、
   取消和 snapshot 可续跑语义；首期条件失败必须保持 rejected。
5. 交互 HTML（已完成）：独立 animate 阶段共同消费 tools2 v4 `model.json` 和 `view.json`，生成内嵌
   `lkm.spec.signal-animation` v1 数据的自包含 HTML；按 Signal 前进/后退，不扩展 v4 view schema，
   也不把浏览器变成推导器。完整计划见
   [`interactive-model-animation.md`](interactive-model-animation.md)，老 tools 静态 SVG 保持原责任。
6. 老工具迁移/退役：只有用户另行明确决定后才能规划。不得以 tools2 覆盖率或版本号自动触发。

## 首期验收证据

2026-07-22 首期实现完成，证据如下：

- `make -C tools2 test-focused`：1 项同步/异步纵切通过。
- `make -C tools2 test`：22 项通过，覆盖协议互拒、include/span、隐式 handler、payload/reference 类型、
  严格 Signal、invariant、层级坐标和预算、FIFO、快照续跑、稳定 JSON、view 投影、文本失败链和
  `pyveri2` 短参数入口。
- `tools/pyveri/bin/pyveri spec/model/main.spec -T /tmp/lkm-tools2-legacy.trace.svg
  --trace-annotations state,transition --strict`：老静态 SVG 生成成功；主模型 `obligation=0`、
  `blocked=0`、`contradiction=0`。
- `git diff --check`：通过。
- 仓库根直接 `make test`：最终汇总 `182/182`；native/linux-object KUnit 各 `25/25`，app smoke
  各 `55/55`，LTP close list acceptance 两侧通过。

首期已闭环；主 Roadmap 保持“进行中”只表示里程碑 2–6 仍需未来独立决策和实施，不表示首期缺少
验收责任。

2026-07-23 协议升级到 v4：删除 Signal 的 lossy 字段和 discarded outcome；所有已发送 Signal
都必须被接受并处理，异步 emits 失败同样传播为根执行 failed。

## 交互 HTML 验收证据

2026-07-24 里程碑 5 已闭环。实现位于独立 `tools2/animate/`，生成
`lkm.spec.signal-animation` v1 自包含 HTML；driver/shortcut 的 `--html-out` 与 text、scenario、snapshot
和 work-dir 共存并保留 check 0/1。Python 54 项、Vitest 8 项和 Playwright 5 项通过；浏览器测试覆盖
无网络 `file://` 加载、按钮/键盘、确定往返、父子/兄弟布局、普通/自环/异常箭头、自动滚动、
reduced-motion、截图，以及完整主模型 277 个 Signal 的前后往返。完整证据保存在
[`interactive-model-animation.md`](interactive-model-animation.md)。
