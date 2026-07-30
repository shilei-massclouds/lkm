# Coding 规格

本目录用自然语言记录模型规格到具体代码实现的映射规则。

charter 说明设计意图，`spec/model` 是对象、状态、事件、依赖、阶段顺序和证明边界的正式
来源；coding 不重新定义或覆盖模型语义，只描述模型元素在目标实现中的函数、状态、检查、
checkpoint、架构边界和源码落点。

## 阅读入口

`spec/coding/README.md` 是目录入口，[`mapping.md`](mapping.md) 是通用 model-to-impl 映射
入口，[`phase-paradigm.md`](phase-paradigm.md) 是阶段对象的专用映射规则。实现某个 system、
phase 或 object 前，再按模型树读取对应的 `.md`。coding 层没有独立 formal 入口。

## 权威格式与永久门禁

`.md` 是 coding 层唯一权威来源。不得新增 coding `.spec`、predicate 或 type。原迁移期文件
中属于生命周期、对象关系或依赖的内容已上移到 model，属于实现落点、语言、架构、测试和
工程映射的内容已迁入对应 `.md`；稳定 rule ID 和原 MUST/SHOULD/MAY/NOTE 层级保留在
Markdown 中用于评审追踪，但不再是 pyveri predicate。根目录 `make coding-spec-check` 会列出并
拒绝任何重新引入的 `spec/coding/**/*.spec`，`make test` 在完整 Clippy 矩阵之后执行该门禁。

当前阅读顺序为：

1. `README.md`：确认 coding 规格范围、外部规格优先级和本目录阅读顺序。
2. `mapping.md`：对象、状态、事件、checkpoint 和源码落点的通用映射规则。
3. `phase-paradigm.md`：Phase 的四状态、父子 continuation、`emits` 和 checkpoint 映射。
4. `build.md`、`tools2.md`、`riscv64.md`、`rust.md`：构建、独立 Signal 工具链、架构和语言映射规则。
5. `systems/*.md`：唯一顶层 System 树的规格、构造与启动映射。
6. `phases/**/*.md`、`objects/**/*.md`：按 model 树读取对应主题映射。
7. `arceos_ex.md`：当前目标内核的 coding 索引。
8. `arceos_ex-implementation.md`：当前实现说明、命令和阶段性取舍；它不得覆盖前述映射。

默认 `charter-first` 中，层间冲突按 charter、model、coding、适用的 compose、impl、testing
权威顺序处理。用户显式触发的 `model-first` 例外按 [`spec/guidance`](../guidance/README.md)
执行其 model 确认门禁，再恢复同一权威层级的双向闭合。计划文档不得覆盖 charter、model 或
coding。

## 阶段边界

规格指导实现时分为两个连续阶段：

1. `Object Coding Phase`：对象级编码阶段。该阶段只考虑模型对象如何在代码中被承载，包括对象状态、对象属性、对象间依赖、事件推进、状态检查、checkpoint 和必要的架构/语言安全边界。
2. `Composition Phase`：组合封装阶段。该阶段在对象级实现语义已经明确的前提下，决定对象如何被组织进 crate、module、公开接口、构建 profile 和应用接入路径。

`spec/coding/` 只描述 `Object Coding Phase`。对象级编码阶段不以最终 crate/module 边界为优先目标；代码物理上可以暂时放入某个 crate，但该 crate 只作为当前实现载体，不自动成为最终组件边界。

组件、module、crate、公开接口、adapter、构建接入以及与 ArceOS 组件体系兼容相关的规则，记录在 `spec/compose/`。`Composition Phase` 不应改变对象状态、transition 迁移和依赖语义，只决定这些对象级实现如何组合和发布。

## 目录层次

`spec/coding/` 保留 `systems/`、`phases/`、`objects/` 三类对象层次目录，
与 `spec/model/` 和 `spec/charter/` 的公共层次对齐。现有 `mapping/`、`build/`、`riscv64/`、
`rust/` 等通用编码规格继续由本目录根入口承载；目标内核专题规则按系统、阶段和对象拆入
上述子目录。后续新增或拆分的专题约束，若主要约束系统、阶段或对象之一，应落入对应目录；构建
专题保留在本目录根级文件。

与 Model 对齐所需的独立专题文件即使没有专用 Coding 约束也应保留。空专题文件表示该对象没有
额外 lowering 规则，其全部代码表示直接采用本目录的通用 Coding 规格；空文件不表示缺少审查，
也不允许实现跳过 Model 中已经声明的约束。

## `.spec` 迁移结果

Kernel 根审计开始时 coding 目录有 26 个 `.spec`。Kernel 根批次删除不可解析的 `main.spec`
和 `arceos_ex.spec` 后剩余 24 个；Boot 子树批次迁移并删除 3 个 phase 文件，Interrupt 子树
批次又把 `irq-time-init.spec`、`local-irq-enable.spec`、`irq-open-prepare.spec` 和
`process-prepare.spec` 的有效实现规则迁入对应 `.md` 并删除；现 BootInitFlow 子树迁移删除
`rest-init.spec`，SmpRuntime 子树迁移删除 6 个 phase 文件。最后一批审计并迁移通用
mapping/build/riscv64/rust 4 个、system 1 个和 object 5 个遗留文件后，coding `.spec` 已清零。

## 当前实践目标

当前目标是实践主规格文档的第二点用途：基于规格指挥 AI 生成内核代码。

第一轮实践目标暂定为：

- 实现语言：Rust。
- 目标体系结构：RISC-V64；第一轮不做多架构兼容实现。
- 参考实现：ArceOS。
- 目标内核：`tgoskits` 中新增 `os/arceos_ex`，作为 ArceOS 的规格化演化候选。
- 目标平台：新增 RISC-V64 generic SBI/FDT 平台实现，当前以 QEMU 为测试环境，但不以 QEMU virt 硬编码作为平台语义来源。
- 规格来源：优先使用 `spec/model` 下已经规格化的启动阶段模型，必要时由本目录补充编码约束。

第一轮内核形态以 ArceOS 的 Unikernel 方式起步，由 `helloworld` 应用引领构成最小系统。即使应用只是 `helloworld`，也必须完整支撑当前模型中已经展开的入口、引导、中断时间准备和 payload 交接阶段。

## 规格优先级

以下是 `charter-first` 与 `model-first` 最终闭合后共同遵守的权威优先级，不表示代理可以自行
选择当轮修改顺序：

1. `spec/charter`：设计意图、职责边界和参考范围；若与 model 冲突，先完成审计并修正 model。
2. `spec/model/SEMANTICS.md` 与 `spec/model/**/*.spec`：正式生命周期、对象、迁移和依赖语义。
3. `spec/coding/**/*.md`：model 到对象级代码实现的权威自然语言映射。
4. `spec/compose/main.spec`、`spec/compose/*.md`：crate/module 组合和公开接口约束。
5. 当前实现目录的工程约定：构建、测试和已有抽象；不得反向覆盖前三层。

本目录中的“映射”只表示 Model 到代码表示的约束。Coding 可以吸收 Linux、ArceOS 或其它参考实现提供的机制证据，
但权威正文必须把结论表达为 Model 对数据结构、算法、寄存器、ABI 或其它代码表示的约束。`arceos_ex` 与参考实现之间的
源码、函数、符号或 checkpoint mapping 不是 Coding 规格；它们由独立 cross-reference 或验证产物承载，不得反向定义 Model 或 Coding。

## 模型到代码的默认映射

| 模型元素 | 默认代码落点 |
| --- | --- |
| `object` | Rust 模块、类型、静态单例或启动阶段上下文中的字段 |
| `state` | 生命周期枚举、typestate、debug checkpoint 或启动断言 |
| `event` | 明确命名的 `preset` / `setup` / `enable` / `cleanup` 函数 |
| `depends_on` | 函数前置条件、类型约束、启动断言或构建期检查 |
| `ensures` | 后置状态记录、运行期检查、测试断言或可观测 trace 点 |
| `invariant` | 对象状态保持条件、debug 检查或规格化单元测试 |
| `drives` | 父对象过程中的调用编排顺序 |
| `emits` | owner transition 提交后触发的 completion event；Phase 标准用法只连接同对象迁移 |
| `deferred <id>` | 未闭合的 Transition/Action 操作体默认表示为成功空函数，并在成功边界产生 checkpoint；显式 Model 约束仍照常 lowering，checkpoint 不宣称 Deferred 能力已经实现 |
| `trimmed <id>` | 由当前构建配置、架构、参考输入或编译期 no-op 证明不可达/无操作；不生成运行时 stub |

更细的映射规则见 [`mapping.md`](mapping.md) 和 [`phase-paradigm.md`](phase-paradigm.md)。

## 最小编码原则

- 严格遵循规格指导。若某个模型约束、阶段边界、状态迁移、前置依赖或后置检查无法满足，必须停止实现并报告，不得用隐式假设绕过。
- 编码实现受“推导义务门禁”约束。`make verify` 报告的 `obligation` 不是实现可忽略的提示；实现某个阶段或对象 transition前，必须先检查它直接依赖的义务。能够由模型、推导器、链接脚本、ISA、固件规范或已证明前序事实推出的义务，应优先补齐推导证明；只能由外部交付保证支撑的义务，必须明确记录为 source assumption，并在后续最早可行事件中转化为运行期检查；无法归类的义务应视为规格缺口或延期项，不得宣称对应阶段已经闭合实现。`make verify` 的通过条件包括 `obligation == 0`，工具层不得在仍有 unresolved obligation 时返回成功。
- 先完成对象级语义正确的最小内核路径，再进入组件组合和通用框架整理。
- 对象实现建议优先使用能表达当前语义的高粒度复合对象边界，由该对象封装属性和子对象状态，并驱动子对象 transition完成自身事件；若必须展开为较低粒度对象或临时混合承载，应记录原因。
- 对象生命周期函数必须使用模型 transition名称，除非 coding 规格明确给出本地别名。
- checkpoint 只能由对应对象 transition或阶段边界的映射实现发出。低层阶段、资源对象、入口汇编或 continuation 不得为了让运行期 trace 视觉上贴合推导 trace，而代发父阶段、其它阶段、准备期或其它对象的合成 checkpoint。
- linker script 是 model 中 `Lds` 准备期对象的代码生成产物，必须由 `Lds` 属性及其依赖的 `Config` 属性共同驱动。硬编码链接常量只能作为带记录的过渡例外存在，并必须指出对应的 `Lds`/`Config` 来源。
- 每个 `unsafe` 块必须对应清晰的硬件、链接器、启动 ABI 或裸机内存访问边界。
- 参考 ArceOS 时优先参考工程边界和成熟实现经验，不以逐行复刻为目标。
- 对尚未建模的实现需求，应先记录为 coding 约束或模型缺口，再决定是否直接编码。
- 实现、代码评审和测试必须使用 model boundary ID 引用未闭合责任；不得在 coding、
  TODO 或 roadmap 中复制一段可独立演化的 backlog 文本。一个 ID 只对应一项责任。
- 关闭 deferred boundary 时，必须在同一变更中删除模型记录，用正式事实、实现和测试
  替代，并把原 ID 与证据写入唯一专题归档；ID 不再复用。
- trimmed boundary 的实现分支必须与 `evidence` 一致。当 `revisit_when` 触发时，先重新审计并
  删除或重分类 boundary，不得继续依赖已失效的配置/架构/输入假设。

## tgoskits 落点约束

- 不直接修改 `tgoskits/os/arceos` 下的现有 ArceOS 实现。
- 不直接修改 `tgoskits/components` 下已有组件的行为。
- 新内核代码放在 `tgoskits/os/arceos_ex`。
- 需要新增可复用组件时，直接放入 `tgoskits/components` 的合适层级，并使用 `_ex` 后缀区分现有组件。
- 新 RISC-V64 generic 平台实现不放在 `os/arceos_ex` 下；按 tgoskits 现有组件布局，优先放在 `components/axplat_crates/platforms/` 下，并使用 `_ex` 后缀避免与现有平台 crate 冲突。
- 第一轮应直接接入 tgoskits 的构建/运行工具链，而不是长期依赖临时脚本。
- `cargo xtask` 负责管理 `arceos_ex` 构建所需的 overlay workspace 和 Cargo 依赖映射；不要求开发者手工来回修改顶层 `Cargo.toml`。
- 第一轮应新增 `cargo xtask arceos-ex ...` 子命令，并复用 ArceOS 的测试发现、构建和运行机制来选择 Unikernel 应用；后续可逐步扩展到全量 ArceOS 应用测试。

## 当前文件

- `riscv64.md`：RISC-V64 架构相关编码约束。
- `rust.md`：Rust 语言和安全边界相关编码约束。
- `arceos.md`：参考 ArceOS 时的取舍原则和映射约束。
- `mapping.md`：模型对象、阶段、状态、事件和检查点到代码的权威映射规则。
- `phase-paradigm.md`：阶段 model 到状态、continuation、迁移链和 checkpoint 的专用映射规则。
- `build.md`：构建入口、`make disk`、QEMU 设备、payload 选择、外部工具和脚本失败行为的说明性约束。
- `tools2.md`：首期独立 Signal 工具链的包边界、中间协议、CLI 和实现责任。
- `arceos_ex.md`：`arceos_ex` coding 索引入口，链接 system、phase、object 和实现说明。
- `objects/README.md`：`spec/model/objects/*.spec` 的逐文件 coding 覆盖表；共享专题必须在表中明确归属。
- `rootfs-image.md`：rootfs 镜像构造与 fixture overlay 的权威构建规则。
- `arceos_ex-implementation.md`：`arceos_ex` 第一轮对象级实现证据、源码落点、命令和阶段性观察；不承载 MUST/SHOULD/MAY 规则。统一任务优先级和状态见 [`../../docs/ROADMAP.md`](../../docs/ROADMAP.md)。
- `systems/{computer,riscv64-platform,opensbi,kernel}.md`：四个 System 的规格、构造、启动责任与实现状态。
