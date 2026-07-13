# 阶段范式四层一致性审计

本文是 [`ROADMAP.md`](../ROADMAP.md) 中“Kernel 阶段范式四层一致性审计与 coding
文档化收敛”的执行账本。优先级和任务总状态只在主 roadmap 维护；本文记录逐级审计顺序、
检查矩阵、证据和批次验收结果。

## 目标

从 `systems/kernel` 开始，沿 Kernel 驱动的阶段树自顶向下检查以下四层：

1. `spec/charter`：说明阶段存在的目的、职责边界和阶段范式意图。
2. `spec/model`：正式表达状态、迁移、依赖、`drives`、`ensures`、`emits` 和 deferred。
3. `spec/coding`：用自然语言 `.md` 说明 model 到具体 impl 的映射规则和必要例外。
4. `impl/arceos_ex`：按 coding 映射承载调用链、对象动作、状态检查和 checkpoint。

coding 层不维护第二套模型语义。长期权威来源是 `spec/coding/**/*.md`；现存 `.spec` 中属于
模型语义的约束上移到 model，属于实现映射的约束迁入 `.md`，重复或失效内容删除。所有引用
和工具入口迁移完成后，最终清零 `spec/coding/**/*.spec`。

## 审计契约

每个 system、编排阶段和叶子阶段都检查同一组项目：

| 检查项 | 判定标准 |
| --- | --- |
| 职责 | charter、model、coding、impl 对该层负责和不负责的内容一致。 |
| 生命周期 | 标准阶段使用 `Base -> Prepared -> Ready -> Online` 和 `Preset/Setup/Enable`；例外必须在 charter/model/coding 同时说明。 |
| 前置条件 | charter 意图能在 model `depends_on` 中找到，coding 映射为明确检查，impl 不提前消费依赖。 |
| 驱动 | model `drives` 的对象动作或子阶段迁移在 coding 和 impl 中保持顺序与所有权。 |
| 完成条件 | model `ensures`/invariant 在 coding 中有映射，并由 impl 状态、检查或长期测试支撑。 |
| 串接 | 当前阶段到下一迁移或 sibling 的推进由 `emits` 表达；coding 和 impl 不另造第二条时间线。 |
| checkpoint | 只能由对应阶段/对象边界发出；名称、时点和 owner 与 model 一致。 |
| deferred | 未实现边界在 model/coding/impl 中显式且不被验收误报为完成。 |
| coding 来源 | 实现映射正文位于 `.md`；同主题 `.spec` 完成迁移后删除。 |

发现项按以下类别记录：`missing-layer`、`semantic-conflict`、`phase-paradigm-violation`、
`coding-duplication`、`impl-drift`、`stale-reference`、`unverifiable`。修正顺序固定为 charter ->
model -> coding -> impl；不得先改 impl 再反向解释规格。

## 阶段树

```text
Kernel
├── BootPhase
│   ├── EntryPreludePhase
│   ├── EntrySuccessorPhase
│   ├── CorePreparePhase
│   ├── MmCoreInitPhase
│   └── SchedInitPhase
├── InterruptPhase
│   ├── IrqTimeInitPhase
│   ├── LocalIrqEnablePhase
│   ├── IrqOpenPreparePhase
│   └── ProcessPreparePhase
├── UpMultitaskPhase
│   └── RestInit 子阶段链
├── SmpRuntimePhase
│   ├── PreSmpInitPhase
│   ├── SmpBringupPhase
│   ├── RuntimeCorePhase
│   ├── InitcallPhase
│   ├── RootfsPhase
│   └── FinalizePhase
└── PayloadPhase
```

## 执行批次

1. **计划与基线（完成）**：已建立本文并冻结审计契约，记录了现存文件、失效引用、四层
   检查项和批次门禁；根目录 `make test` 为 167/167。
2. **Kernel 根与 coding 权威来源（完成）**：Kernel charter/model/coding/impl 已对齐；sibling
   串接改为前一阶段 `Enable` 提交 Online 后 `emits` 下一阶段 Preset；coding README/mapping
   已改为 `.md` 权威；删除不可解析的 `coding/main.spec` 和 `coding/arceos_ex.spec`，顶层
   `spec/main.spec` 恢复可解析；剩余 24 个 coding `.spec` 已分类。announce 运行以
   `RAI...` 证明 `Kernel.Started` 先于 `EntryPreludePhase.Started`，并最终到达
   `Kernel.Online -> Hello, world!`。
3. **BootPhase（待办）**：先审计编排层，再按五个叶子阶段顺序执行；EntryPrelude 作为已实现
   样板重新复核，入口汇编例外必须有四层对应说明。
4. **InterruptPhase（待办）**：审计四个叶子阶段，重点核对中断开关、effective context、
   checkpoint owner 和 sibling 串接。
5. **UpMultitaskPhase（待办）**：审计 rest-init 子阶段链，重点核对 BootIdle、KernelInit、
   KThreadd 的任务所有权、真实栈切换和无限调度循环。
6. **SmpRuntimePhase（待办）**：审计六个叶子阶段，确认整棵运行期初始化链由
   KernelInitTask 驱动。
7. **PayloadPhase（待办）**：核对 selected payload、KernelInitTask owner、不返回语义和关机
   边界。
8. **coding `.spec` 退场（待办）**：每审完一棵子树就迁移并删除对应 phase/system `.spec`；
   阶段树完成后处理 project、mapping、build、riscv64、rust 和 object 类剩余文件，迁移所有
   工具与文档引用，最终确保 `find spec/coding -name '*.spec'` 为空。
9. **全树验收（待办）**：复核一致性矩阵无未分类缺口，运行 `make verify`、必要 focused
   `make run`、根目录 `make test`，最后执行 `make test-stress STRESS_RUNS=30`。

## 批次门禁

每个批次都必须：

1. 先记录可复现差异或缺口，再修改对应层。
2. 更新本页矩阵和主 roadmap 当前进展。
3. 运行相关 focused 验证。
4. 直接运行根目录 `make test`，不得用 shell wrapper、管道或重定向替代。
5. 通过 `git diff --check` 后形成独立提交。

## 一致性矩阵

| 节点 | Charter | Model | Coding `.md` | Impl | 结论 |
| --- | --- | --- | --- | --- | --- |
| Kernel | 意图/边界一致 | 补齐三个 Enable ensures | `.md` 权威且 continuation 映射明确 | `RAI...`、Kernel.Online 已验证 | complete |
| BootPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| EntryPreludePhase | 已有首轮 | 已有首轮 | 已有首轮 | 已有首轮 | recheck |
| EntrySuccessorPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| CorePreparePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| MmCoreInitPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| SchedInitPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| InterruptPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| IrqTimeInitPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| LocalIrqEnablePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| IrqOpenPreparePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| ProcessPreparePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| UpMultitaskPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| RestInit 子阶段链 | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| SmpRuntimePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| PreSmpInitPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| SmpBringupPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| RuntimeCorePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| InitcallPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| RootfsPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| FinalizePhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |
| PayloadPhase | 待审计 | 待审计 | 待审计 | 待审计 | pending |

## 当前基线

- `72fa66c`：建立阶段范式并把顶层阶段模型收敛到四状态生命周期。
- `28e9bd9`：建立阶段链式 coding 映射并闭合 EntryPreludePhase 首轮实现。
- `dc6834a`：完成 BootIdle 到 KernelInit 的真实 task stack handoff。
- coding 目录已从 26 个 `.spec` 降到 24 个；剩余分类为通用映射/构建/架构/语言 4 个、
  project 1 个、phase 14 个、object 5 个。
- coding 权威入口已经切换为 `.md`；顶层 `spec/main.spec` 不再 include coding formal 入口并
  已通过 pyveri 检查。
- sibling 串接 Fix 已闭合为“前一阶段 Enable 提交 Online 后 emits 下一阶段 Preset”。
- Kernel 根审计发现各 composite phase 当前 impl 仍把 model Online 完成边界命名/记录为
  `Ready`；该问题按 Boot、Interrupt、UpMultitask、SmpRuntime 子树批次逐项修正。
- 更新本计划前，根目录 `make test` 为 167/167；默认 ordinary-path stress 三个 case 各 30 次，
  总计 90/90，通过且每个 case 只有一个成功事件序列。
