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

两层范式的权威入口分别是：[`spec/charter/phase-paradigm.md`](../../spec/charter/phase-paradigm.md)
定义 charter -> model，[`spec/coding/phase-paradigm.md`](../../spec/coding/phase-paradigm.md)
定义 model -> impl。通用 [`mapping.md`](../../spec/coding/mapping.md) 不再维护第二套阶段调用规则。

## 审计契约

每个 system、编排阶段和叶子阶段都检查同一组项目：

| 检查项 | 判定标准 |
| --- | --- |
| 职责 | charter、model、coding、impl 对该层负责和不负责的内容一致。 |
| 生命周期 | 标准阶段使用 `Base -> Prepared -> Ready -> Online` 和 `Preset/Setup/Enable`；impl 对每个 source/target state 有精确设置与检查，例外必须在 charter/model/coding 同时说明。 |
| 前置条件 | charter 意图能在 model `depends_on` 中找到，coding 映射为明确检查，impl 不提前消费依赖。 |
| 驱动 | model `drives` 的对象动作或子阶段迁移在 coding 和 impl 中保持顺序与所有权。 |
| 完成条件 | model `ensures`/invariant 在 coding 中有映射，并由 impl 状态、检查或长期测试支撑。 |
| 层级推进 | 父子关系只由父 transition 的 source-ordered `drives` 表达；`emits` 只连接标准阶段自身的 Preset -> Setup -> Enable，子阶段完成后返回父 continuation。 |
| checkpoint | 只能由对应阶段/对象边界发出；每个状态点明确记录 checkpoint 或省略理由，名称、时点和 owner 与已提交状态一致。 |
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
2. **Kernel 根与 coding 权威来源（完成）**：Kernel charter/model/coding/impl 的状态与
   continuation 已完成首轮对齐；coding README/mapping 已改为 `.md` 权威；删除不可解析的
   `coding/main.spec` 和 `coding/arceos_ex.spec`，顶层
   `spec/main.spec` 恢复可解析；当时剩余 24 个 coding `.spec` 已分类。announce 运行以
   `RAI...` 证明 `Kernel.Started` 先于 `EntryPreludePhase.Started`，并最终到达
   `Kernel.Online -> Hello, world!`。
3. **两层阶段范式（完成）**：charter 范式只负责生成 model 的标准生命周期、父 `drives` 和
   同对象 `emits`；coding 范式独立负责四状态设置/检查、父 continuation、执行主体交接和
   checkpoint lowering。旧的 sibling emits、平坦 DFS 和 `handoff() -> next.setup()` 规则已删除。
4. **BootPhase（完成）**：Boot charter 固定五个直接子阶段；model 保持 Kernel -> Boot ->
   EntryPrelude 的父 `drives` 和同对象 `emits`，只补入口 lowering/continuation 注释；coding
   逐项映射父/叶 source、target、depends、drives、ensures、state、checkpoint 和 continuation，
   并删除三个已迁移的 Boot coding `.spec`。实现入口采用 `R -> B -> A` 与四层 Rust adoption，
   Boot 和五个叶子均可观察 Started/Prepared/Ready/Online；所有 sibling 直调已改为 Boot 父
   continuation，查询收敛为精确 Online。announce 实测 `RBAIKOZHTS`，并观察
   EntryPrelude.Online < Boot.Prepared < EntrySuccessor.Started、EntrySuccessor.Online <
   Boot.Ready < CorePrepare.Started、SchedInit.Online < Boot.Online < InterruptPhase.Started。
5. **InterruptPhase（待办）**：审计四个叶子阶段，重点核对中断开关、effective context、
   非标准 transition 入口、父 continuation 和 checkpoint owner。
6. **UpMultitaskPhase（待办）**：审计 rest-init 子阶段链，重点核对 BootIdle、KernelInit、
   KThreadd 的任务所有权、真实栈切换和无限调度循环。
7. **SmpRuntimePhase（待办）**：审计六个叶子阶段，确认整棵运行期初始化链由
   KernelInitTask 驱动。
8. **PayloadPhase（待办）**：先补齐当前 model 缺失的 `Setup` 和同对象 `emits`，再核对
   selected payload、KernelInitTask owner、不返回语义和关机边界。
9. **coding `.spec` 退场（待办）**：每审完一棵子树就迁移并删除对应 phase/system `.spec`；
   阶段树完成后处理 project、mapping、build、riscv64、rust 和 object 类剩余文件，迁移所有
   工具与文档引用，最终确保 `find spec/coding -name '*.spec'` 为空。
10. **全树验收（待办）**：复核一致性矩阵无未分类缺口，运行 `make verify`、必要 focused
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
| 两层阶段范式 | `drives` 父子关系、同对象 `emits` | model 作为唯一执行语义 | 独立状态/continuation/checkpoint lowering | 各子树逐项应用 | complete |
| Kernel | 意图/边界一致 | 补齐三个 Enable ensures | `.md` 权威且 continuation 映射明确 | `RBA...`、Kernel.Online 已验证 | complete |
| BootPhase | 五子阶段、入口/出口已固定 | 既有 drives/emits 保持 | 父 continuation/四状态/checkpoint 已完整映射 | Online 状态和五个父 continuation 已验证 | complete |
| EntryPreludePhase | 入口例外已明确 | 入口 adoption/父返回注释已补 | 汇编、VM continuation、四状态已映射 | `RBA...` 且四 checkpoint 已验证 | complete |
| EntrySuccessorPhase | 边界一致 | 既有 transition 保持 | legacy rules 已迁入 `.md` | depends/四状态/父返回已验证 | complete |
| CorePreparePhase | 边界一致 | 既有 transition 保持 | legacy rules 已迁入 `.md` | depends/四状态/父返回已验证 | complete |
| MmCoreInitPhase | 边界一致 | 既有 transition 保持 | legacy rules 已迁入 `.md` | depends/四状态/父返回已验证 | complete |
| SchedInitPhase | 边界一致 | 既有 transition 保持 | 四状态/父返回已映射 | depends/四状态/Boot.Online 已验证 | complete |
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
- coding 目录已从 26 个 `.spec` 降到 21 个；剩余分类为通用映射/构建/架构/语言 4 个、
  project 1 个、phase 11 个、object 5 个。
- coding 权威入口已经切换为 `.md`；顶层 `spec/main.spec` 不再 include coding formal 入口并
  已通过 pyveri 检查。
- 阶段范式已纠正为两层定义：父子阶段只由父 `drives` 连接，`emits` 只连接同一标准阶段的
  Preset -> Setup -> Enable；impl 通过父 continuation 执行下一条 drive，不建立 sibling 边。
- Boot 子树已消除 model Online 被实现命名/记录为 Ready 的漂移；剩余问题继续按 Interrupt、
  UpMultitask、SmpRuntime 子树批次逐项修正。
- Boot 批次已通过 `make checkpoints`、`make test-checkpoints`、`make verify`、
  `make run APP=hello PROBE=announce` 和根目录 `make test`；最终回归为 167/167。
- 本批开始前根目录 `make test` 为 167/167；默认 ordinary-path stress 三个 case 各 30 次，
  总计 90/90，通过且每个 case 只有一个成功事件序列。
