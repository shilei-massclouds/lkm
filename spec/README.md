# 规格目录

本目录同时描述内核的核心语义、内核从规格到实现的细化过程，以及支撑这一过程的工作流、装配和验证约束。

## 核心语义细化链

项目只有一条核心语义细化链：

```mermaid
flowchart LR
    Charter["Charter<br/>人类可理解的设计意图"] --> Model["Model<br/>形式化与精确化"]
    Model --> Coding["Coding<br/>Model 到代码的映射"]
    Coding --> Impl["Impl<br/>实现共同要求的效果"]
```

- `charter/`：以符合人类直觉的自然语言定义设计意图、目标、概念和长期边界，是最高语义权威。
- `model/`：以偏形式化语言精确化 Charter，定义对象、状态、事件、依赖和证明边界；只能细化 Charter，不得改变其含义。
- `coding/`：约束 Model 到代码的映射，可以具体到数据结构、算法、函数和模块内落点、内存布局、寄存器及架构/语言安全边界；不得覆盖 Charter 或 Model。
- `impl/`：实现 Charter、Model 和 Coding 共同要求的效果，不具有根据现有代码反向解释规格的权力。

Coding 的“映射”仅指 Model 到代码表示的映射，不包括两个具体实现之间的源码、函数、符号或 checkpoint
对照。Linux、ArceOS 或其它参考实现可以提供设计与验证依据，但这些依据只有转化为 Model 到代码表示的约束后才属于
Coding；具体实现之间的 mapping 由独立 cross-reference 或验证责任承载。

每个下层都必须满足所有适用上层约束。只有全部适用上层都未约束的细节，下层才可局部自由选择；若这种选择形成新的可观察行为、接口或对象边界，必须先提升到适当的核心规格层明确，再沿核心链向下闭合。

## 支撑职责

以下目录支撑核心链，但不进入核心语义权威链：

- `guidance/`：约束 AI、代码生成器和变更过程如何读取、修改、审查和验证各层，是工作流约束，不定义内核语义。
- `compose/`：在适用时约束对象级代码如何装配、封装和发布，位于 Coding 到 Impl 的装配路径；它不得改变 Coding 已确定的映射或更高层语义。Compose 与 Coding 冲突时以 Coding 为准。
- `testing/`：约束如何从核心规格选择验证目标、构造场景和执行测试，是验证旁支。测试与核心规格冲突时，应修正测试或不符合规格的实现，不得修改高层规格来迎合现有测试结果。

Guidance、Compose 和 Testing 可以约束各自负责的流程、装配或验收产物，但都不能用这些局部约束反向定义 Charter、Model、Coding 或 Impl 应有的核心语义。

## 文件与目录入口

- `*.md` 是自然语言说明性规格。Charter 首先保证人类可理解；Coding 也以自然语言 `.md` 为唯一权威来源。
- `*.spec` 是需要机器解析和检查的正式规格，供 Model、Guidance、Compose 和 Testing 使用。形式化表达不改变上述语义权威关系。
- `main.spec` 是正式规格的顶层索引；每个包含正式规格的目录也以 `main.spec` 作为本目录入口。
- `charter/main.md` 是总 Charter 入口；专题说明继续放在 `charter/` 下。
- `charter/`、`model/` 和 `coding/` 以 `systems/`、`phases/`、`objects/` 为最低公共层次；各目录仍可保留 `pic/`、正式入口文件、通用说明或其它职责明确的专题目录。Computer 是唯一顶层系统，不再另设工程对象层。

旧的 Excalidraw/SVG 层级图资产继续保留用于历史参考，但本页 Mermaid 图和文字定义是当前顶层层级说明。

## 变更工作流

默认使用 `charter-first`：先在 Charter 确定设计意图，再按 `Charter -> Model -> Coding -> Impl` 审查和闭合核心语义。每轮先确定最高受影响核心层，并对 Charter、Model、Coding、Impl 依次留下“已修改”或“已审查、无需修改”的结论；适用的 Compose 装配约束和 Testing 验收约束作为旁支一并审查。

计划、roadmap、现有实现、Compose 或测试不得反向覆盖较高核心层。若 Charter 含义不清或目标与 Charter 冲突，应停止并回到 Charter 决策；Charter 锁仍须获得对具体文件的显式解锁授权。

只有用户明确声明本轮 `model-first` 并提供 Model 调整方案时，才允许先执行 Model-only 调整和确认。该例外只改变当轮修改与确认顺序，不改变 Charter 的最终最高语义权威；确认后仍须先向上闭合 Charter，再使 Model 符合 Charter，并继续闭合 Coding、Impl 及适用旁支。完整门禁见 [`guidance/README.md`](guidance/README.md) 和 [`guidance/generation.spec`](guidance/generation.spec)。
