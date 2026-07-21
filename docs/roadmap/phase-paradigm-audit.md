# 阶段范式四层一致性审计

> 本文保存已完成审计的执行计划、矩阵和证据。当前优先级、状态和下一步顺序只见
> [`../ROADMAP.md`](../ROADMAP.md)；下文计划/状态措辞均为历史快照。

本文是 [`ROADMAP.md`](../ROADMAP.md) 中“Kernel 阶段范式四层一致性审计与 coding
文档化收敛”的执行账本。优先级和任务总状态只在主 roadmap 维护；本文记录逐级审计顺序、
检查矩阵、证据和批次验收结果。

## 目标

从 `systems/kernel` 开始，沿 Kernel 驱动的阶段树自顶向下检查以下四层：

1. `spec/charter`：说明阶段存在的目的、职责边界和阶段范式意图。
2. `spec/model`：正式表达状态、迁移、依赖、`drives`、`ensures`、`emits` 和 deferred。
3. `spec/coding`：用自然语言 `.md` 说明 model 到具体 impl 的映射规则和必要例外。
4. `impl/arceos_ex`：按 coding 映射承载调用链、对象动作、状态检查和 checkpoint。

coding 层不维护第二套模型语义。长期权威来源是 `spec/coding/**/*.md`；旧 `.spec` 中属于
模型语义的约束已上移到 model，属于实现映射的约束已迁入 `.md`，重复或失效内容已删除。
`spec/coding/**/*.spec` 已清零，并由根目录 `make coding-spec-check` 永久禁止重新引入。

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
├── EntryPreludePhase
├── BootPhase
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
│   ├── BootInitRestInitPhase
│   ├── BootInitScheduleHandoffPhase
│   └── BootIdleEntryPhase
├── SmpRuntimePhase
│   ├── PreSmpInitPhase
│   ├── SmpBringupPhase
│   │   ├── ApEntryPreludePhase
│   │   ├── ApSmpCallinPhase
│   │   └── ApOnlineIdlePhase
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
4. **BootPhase（完成）**：Boot charter 固定四个直接子阶段；model 将 EntryPrelude 提升为
   Kernel 直接子阶段，Kernel.Preset 驱动 EntryPrelude，Kernel.Setup 再按 Boot -> Interrupt
   推进；coding
   逐项映射父/叶 source、target、depends、drives、ensures、state、checkpoint 和 continuation，
   并删除三个已迁移的 Boot coding `.spec`。实现入口采用 `R -> A` 与三层 Rust adoption，
   EntryPrelude.Online 后由普通 Rust `Boot.preset()` 发出 Boot.Started；Boot 和四个叶子均可观察
   Started/Prepared/Ready/Online；所有 sibling 直调已改为 Boot 父 continuation，查询收敛为精确
   Online。announce 实测 `RAIKOZHTS`，并观察 EntryPrelude.Online < Boot.Started <
   Boot.Prepared < EntrySuccessor.Started、EntrySuccessor.Online <
   Boot.Ready < CorePrepare.Started、SchedInit.Online < Boot.Online < InterruptPhase.Started。
5. **InterruptPhase（完成）**：charter 固定四个直接子阶段与 Kernel.Setup 入口/continuation
   出口；model、coding 和 impl 统一四态生命周期，IrqTime 使用 SingleTaskContext，LocalIrqEnable
   不使用外层 context，IrqOpenPrepare/ProcessPrepare 使用 SingleTaskInterruptStreamContext。
   叶子对象动作归入 Preset，Setup/Enable 只检查和发布；四个 sibling 直调改为 Interrupt 父
   continuation，查询收敛为精确 Online。四个 legacy coding `.spec` 已迁移删除。五个阶段均完整
   观察 Started/Prepared/Ready/Online，旧 ID 保持，新 Prepared/Online 默认 unmapped。announce
   实测四条 Online -> sibling Started 顺序、ProcessPrepare.Online -> Interrupt.Prepared/Ready/Online
   -> UpMultitask.Started，并最终到达 Kernel.Online 与 Hello；默认 stress 一轮 3/3 通过。
6. **UpMultitaskPhase（完成）**：charter 固定三个直接子阶段、Interrupt.Online 入口和
   Kernel.Enable 跨栈出口；父 Preset/Setup/Enable 分别驱动 RestInit/ScheduleHandoff/BootIdleEntry。
   父与三个叶子统一四态，叶子对象动作归 Preset，Setup/Enable 只检查和发布；Online 只返回
   三个具名父 continuation。离开 UpMultitask.Online 后执行真实 BootIdle -> KernelInit stack
   handoff，KernelInit entry 验证实际 SP、唯一 switch/entry count 后经
   `kernel::enable_after_up_multitask()` 启动 SmpRuntime。legacy `rest-init.spec` 已迁移删除，
   查询收敛为精确 Online；dispatch 历史事实在 ScheduleHandoff.Online invariant 成立时锁存，
   不受后续 scheduler smoke 改写 current-task 瞬时值影响。11 个新增 checkpoint 追加为
   450–460，旧 268–272 不变且 Linux exact mapping 仍只保留 RestInit.Ready。announce 实测
   三组 child Online -> parent state -> next child Started、UpMultitask.Online -> real switch ->
   KernelInit entry -> SmpRuntime.Started，并最终到达 Kernel.Online 与 Hello；stress 一轮 3/3 通过。
7. **SmpRuntimePhase BP 主线（完成）**：charter 固定六个由 KernelInitTask 执行的直接子阶段，
   父 Preset/Setup/Enable 按 1/1/4 分别驱动 PreSmpInit、SmpBringup 和其余四阶段；父与六个
   叶子统一四态，叶子 Online 只返回六个具名父 continuation，下游只消费精确 Online。
   SmpRuntime 入口及父 continuation 均验证当前 task、唯一切栈/入口事实和实际 SP 位于
   KernelInitTask 栈。六个 legacy coding `.spec` 已迁移删除；14 个新 checkpoint 只追加为
   461–474 且默认 unmapped。announce 验证完整父子顺序及所有 BP 主线 checkpoint owner，
   smoke 54/54、stress 一轮 3/3、根目录 `make test` 167/167 通过。
8. **SmpBringup AP 子树（完成）**：三个 AP phase 已固定为以 secondary `logical_id` 为 key 的
   replicated family；每个 AP 独立四态，同一 AP 严格 Entry -> Callin -> OnlineIdle，不同 AP
   允许交错。实现使用三组 `[AtomicU8; MAX_CPUS]`，AP AcqRel 单写、BP Acquire 聚合观察；旧
   Context 聚合 Lifecycle 已删除。AP 入口 adoption 验证 boot-data slot/logical-id/stack pointer、
   真实 SP 位于 16 KiB 目标栈、TP 等于目标 idle-task pointer，三阶段 Online 只返回具名 AP
   continuation，最后进入 WFI park loop。BP 只负责 HSM、all-online/completion wait、ack 和 online
   publish。旧 checkpoint 321–330 不变且均由 `ApIdleTask[n]` 发出；新增 475–480 默认 unmapped。
   SMP2/SMP8 announce 已验证 per-AP owner/pointwise 顺序、跨 AP 交错和全部 Online 后的 BP ack。
9. **PayloadPhase（完成）**：charter/model/coding/impl 已统一四态和同对象 emits。Preset 只准备
   公共 exec/clone deferred 边界；Setup 绑定 build-time 唯一 Hello/Smoke/UserBoot handoff，且
   只有 user-boot 推进 UserBootPayload；Enable 在任何 Online 前完成 selected variant prepare，
   再按 selected handoff Online -> Payload Online/handlers -> Kernel Online -> no-return entry 推进。
   user-boot prepare/enter 已拆分，requested/default init 失败均停在 Payload Ready。旧 ID 432/433
   稳定，新增 481/482 默认 unmapped；inventory 483，mapping exact 103 / range 14 / unmapped 366。
10. **coding `.spec` 退场（完成）**：阶段树文件退场后，已逐项审计 project、mapping、build、
   riscv64、rust 和 5 个 object 遗留文件的 629 行内容；142 个唯一 rule ID、原 type 分组及
   MUST/SHOULD/MAY/NOTE 层级均保留在对应权威 `.md`。charter/model/coding 索引和入口引用已
   切换到 Markdown，10 个遗留 `.spec` 已删除；独立 `coding-spec-check` 会列出并拒绝任何回归，
   `make test` 在完整 Clippy 矩阵之后执行该门禁。
11. **全树验收（完成）**：`make coding-spec-check` 通过；根目录直接 `make test` 为 169/169；
   `make test-stress STRESS_RUNS=30` 的三个 ordinary-path case 各 30/30，总计 90/90；默认
   `make difftest` 为 1/1，并确认 103 个 Linux markers 为 0 missing / 0 stale / 0 mismatch。

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
| 两层阶段范式 | `drives` 父子关系、同对象 `emits`、replicated family | model 作为唯一执行语义 | 独立状态/continuation/checkpoint lowering | 普通与 per-target 阶段逐项应用 | complete |
| Kernel | 意图/边界一致 | Preset 驱动 EntryPrelude；Setup 驱动 Boot、Interrupt | `.md` 权威且 continuation 映射明确 | `RA...`、Kernel.Online 已验证 | complete |
| BootPhase | 四子阶段、入口/出口已固定 | Preset 依赖 EntryPrelude Online；Setup/Enable drives 保持 | 父 continuation/四状态/checkpoint 已完整映射 | Online 状态和四个父 continuation 已验证 | complete |
| EntryPreludePhase | Kernel 直接子阶段入口例外已明确 | 入口 adoption/Kernel 父返回注释已补 | 汇编、VM continuation、四状态已映射 | `RA...` 且四 checkpoint 已验证 | complete |
| EntrySuccessorPhase | 边界一致 | 既有 transition 保持 | legacy rules 已迁入 `.md` | depends/四状态/父返回已验证 | complete |
| CorePreparePhase | 边界一致 | 既有 transition 保持 | legacy rules 已迁入 `.md` | depends/四状态/父返回已验证 | complete |
| MmCoreInitPhase | 边界一致 | 既有 transition 保持 | legacy rules 已迁入 `.md` | depends/四状态/父返回已验证 | complete |
| SchedInitPhase | 边界一致 | 既有 transition 保持 | 四状态/父返回已映射 | depends/四状态/Boot.Online 已验证 | complete |
| InterruptPhase | 四子阶段与 Kernel 边界固定 | 父 Preset 顺序和四态完整 | context/continuation/checkpoint 完整映射 | Prepared/Ready/Online 与 Kernel continuation 已验证 | complete |
| IrqTimeInitPhase | IRQ/time 且 SIE 关闭 | 标准四态、SingleTaskContext | 对象动作归 Preset、父返回已映射 | 四 checkpoint、精确 Online 已验证 | complete |
| LocalIrqEnablePhase | 只开放 boot CPU 总入口 | 标准四态、无外层 context | early flag/SIE 顺序与负向 gates 已映射 | 四 checkpoint、父返回已验证 | complete |
| IrqOpenPreparePhase | late core 准备边界一致 | 标准四态、InterruptStream context | depends/drives/invariant 已映射 | 四 checkpoint、父返回已验证 | complete |
| ProcessPreparePhase | rest_init 前准备边界一致 | 标准四态、下游依赖 Online | 对象覆盖/deferred/父返回已映射 | 四 checkpoint、Interrupt.Prepared 已验证 | complete |
| UpMultitaskPhase | 三子阶段、Interrupt/Kernel 边界固定 | 三迁移分担 drives、四态完整 | continuation/checkpoint/跨栈完整映射 | 精确 Online、三个父 continuation、Kernel continuation 已验证 | complete |
| BootInitRestInitPhase | BootTask 的 rest_init 前半段 | 标准四态、wait-lock context 保持 | 对象动作归 Preset、父返回已映射 | 四 checkpoint、RestInit.Online 已验证 | complete |
| BootInitScheduleHandoffPhase | BootTask 首次调度边界 | 标准四态、Scheduler action context 保持 | dispatch 锁存与父返回已映射 | 四 checkpoint、长期 dispatch query 已验证 | complete |
| BootIdleEntryPhase | BootTask idle 入口边界 | 标准四态、BootIdleStartupContext 保持 | idle chain、父 continuation、真实 handoff 已映射 | 四 checkpoint、Up.Online 后真切栈已验证 | complete |
| SmpRuntimePhase | 六个直接子阶段、Up.Online 入口和 Payload 出口固定 | 1/1/4 drives、四态和精确 Online 完整 | KernelInitTask owner、栈检查与六个 continuation 已映射 | 四 checkpoint、实际 SP 和父 continuation 已验证 | complete |
| PreSmpInitPhase | BP 预备边界一致 | 标准四态、下游依赖 Online | 对象动作归 Preset、父返回已映射 | 四 checkpoint、精确 Online 已验证 | complete |
| SmpBringupPhase | BP 协调 replicated AP family | pointwise AP drives、all-online 后 ack | HSM/Acquire wait/completion/hotplug guard 已映射 | BP 不代写 AP 状态，全部 Online 后才 ack | complete |
| ApEntryPreludePhase | per-logical-id AP entry family | 标准四态、真实 entry adoption facts | AcqRel 状态、SP/TP/boot-data 检查、父返回已映射 | SMP2/8 四 checkpoint 与 AP owner 已验证 | complete |
| ApSmpCallinPhase | per-logical-id callin family | 标准四态、依赖 Entry Online | cpu_running summary、pointwise continuation 已映射 | SMP2/8 四 checkpoint 与 AP owner 已验证 | complete |
| ApOnlineIdlePhase | per-logical-id online-idle family | 标准四态、依赖 Callin Online | done_up/park continuation 与 Acquire 查询已映射 | SMP2/8 四 checkpoint、park/all-online 已验证 | complete |
| RuntimeCorePhase | runtime core 边界一致 | 标准四态、下游依赖 Online | Started 位于 sched_init_smp 动作前 | 四 checkpoint、精确 Online 已验证 | complete |
| InitcallPhase | initcall 边界一致 | 标准四态、下游依赖 Online | 首失败诊断与父返回已映射 | 四 checkpoint、精确 Online 已验证 | complete |
| RootfsPhase | rootfs 边界一致 | 标准四态、下游依赖 Online | Started 位于 Preset/KUnit 入口 | 四 checkpoint、namespace 前边界保持独立 | complete |
| FinalizePhase | running-state 边界一致 | 标准四态、父 Online 出口 | 对象动作归 Preset、父返回已映射 | 四 checkpoint、SmpRuntime.Online 已验证 | complete |
| PayloadPhase | 公共准备、变种 setup/prepare 与三层 handoff 边界固定 | 四态、同对象 emits、SelectedPayloadHandoff 完整 | adapter、owner/SP、checkpoint/失败顺序已映射 | 三变种、四 checkpoint、失败不伪造 Online 已验证 | complete |

## 当前基线

- `72fa66c`：建立阶段范式并把顶层阶段模型收敛到四状态生命周期。
- `28e9bd9`：建立阶段链式 coding 映射并闭合 EntryPreludePhase 首轮实现。
- `dc6834a`：完成 BootIdle 到 KernelInit 的真实 task stack handoff。
- coding 目录已从 26 个 `.spec` 清零；最后 10 个文件的 629 行规则已迁入 Markdown，142 个
  唯一 rule ID、原 type 分组和层级均已保留。
- coding 权威入口已经切换为 `.md`；顶层 `spec/main.spec` 不再 include coding formal 入口并
  已通过 pyveri 检查。
- 阶段范式已纠正为两层定义：父子阶段只由父 `drives` 连接，`emits` 只连接同一标准阶段的
  Preset -> Setup -> Enable；impl 通过父 continuation 执行下一条 drive，不建立 sibling 边。
- Kernel 根和完整阶段树已消除 model Online 被实现命名/记录为 Ready 的漂移；coding `.spec`
  已全部退场并由 `coding-spec-check` 永久门禁保护。
- Boot 批次已通过 `make checkpoints`、`make test-checkpoints`、`make verify`、
  `make run APP=hello PROBE=announce` 和根目录 `make test`；最终回归为 167/167。
- Interrupt 批次专项通过 `make checkpoints`、`make test-checkpoints`、`make verify`、
  `make run APP=hello PROBE=announce` 和 `make test-stress STRESS_RUNS=1`；checkpoint inventory 为
  450，Linux mapping 为 exact 103 / range 14 / unmapped 333，instrumentation plan 仍为 103。
- UpMultitask 批次专项通过 `make checkpoints`、`make test-checkpoints`、`make verify`、
  `make build APP=hello PROBE=announce`、`make run APP=hello PROBE=announce`、
  `make run APP=smoke` 和 `make test-stress STRESS_RUNS=1`；checkpoint inventory 为 461，Linux
  mapping 为 exact 103 / range 14 / unmapped 344，instrumentation plan 仍为 103，stress 为 3/3。
- SmpRuntime BP 主线批次专项通过 `make checkpoints`、`make test-checkpoints`、`make verify`、
  `make build APP=hello PROBE=announce`、`make run APP=hello PROBE=announce`、
  `make run APP=smoke` 和 `make test-stress STRESS_RUNS=1`；checkpoint inventory 为 475，Linux
  mapping 为 exact 103 / range 14 / unmapped 358，instrumentation plan 仍为 103。announce 观察到
  六个直接子阶段完整 Online -> parent continuation 顺序，全部 BP 主线 checkpoint 位于
  KernelInitTask 执行线，实际 SP 栈范围检查与唯一切栈/入口计数均成立；AP checkpoint 保持 AP
  task owner，不套用 BP 断言。
- SmpBringup AP 子树已通过 `make checkpoints`、`make test-checkpoints`、`make verify`、
  `QEMU_SMP=2/8` hello announce；checkpoint inventory 为 481，Linux mapping 为 exact 103 /
  range 14 / unmapped 364，instrumentation plan 仍为 103。SMP8 实测不同 AP 交错，且每个
  logical ID 内四态和三个 sibling 严格有序；全部 AP Online 后 BP 才发布 completion ack、
  SmpBringup Prepared/Ready/Online。
- PayloadPhase 批次已通过 `make checkpoints`、`make test-checkpoints`、`make verify`、hello/user-boot
  announce、smoke 54/54 和 requested/default init 失败 focused run；checkpoint inventory 为 483，
  Linux mapping 为 exact 103 / range 14 / unmapped 366，instrumentation plan 仍为 103。hello 四个
  Payload checkpoint 均由 KernelInitTask 发出；user-boot 实测 UserAddressSpace.Ready <
  Payload.Online < Kernel.Online < UserModeEntry，两个失败分支都没有后三个成功事件。
- coding `.spec` 退场批次通过 `make coding-spec-check`；根目录直接 `make test` 为 169/169；
  默认 ordinary-path stress 三个 case 各 30/30，总计 90/90；默认 rc-local difftest 为 1/1，
  103 个 Linux markers 为 0 missing / 0 stale / 0 mismatch。
