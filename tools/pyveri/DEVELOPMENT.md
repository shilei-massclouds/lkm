# pyveri Development Plan

本文档记录 `pyveri` 第一版推导验证器的实现计划。当前目标是验证：

```text
../../spec/entry-prelude-object-model.spec
```

## 定位

`pyveri` 是面向组件化内核规格的静态推导验证器。它不读取运行时轨迹，也不依赖模拟器、硬件采集或参考内核差分数据。

第一版验证器只做基于 `.spec` 文件本身的纸上推导验证：从规格声明的顶层启动时间轴出发，检查对象、阶段、状态和事件能否按模型规则逐级推出目标状态。

当前推导入口是：

```text
StartupTimeline.Event::Setup
```

当前推导目标是：

```text
StartupTimeline.state == State::Ready
```

## 工具链架构目标

长期目标是把验证工具链做成类似 `gcc` 的 driver/tool 结构，而不是只做成单一程序的子命令集合。`pyveri` 应作为当前验证器 driver，负责调度一组独立工具；每个阶段工具都是可以单独执行的程序，阶段之间通过明确的中间文件衔接。

仓库内部目录使用短名称，便于开发和组合：

```text
tools/
  common/   # 公共库，不直接作为工具执行
  parse/    # .spec -> AST 中间文件
  model/    # AST -> 静态对象模型中间文件
  derive/   # 模型 -> 推导结果中间文件或报告
  check/    # 推导结果 -> 通过/阻塞/矛盾判断
  view/     # 模型或推导结果 -> 视图中间文件
  render/   # 视图中间文件 -> text/DOT/SVG/JSON
  pyveri/   # 当前 driver，负责调度上述工具
```

发布或安装时可以把这些短名称映射为带项目前缀的长命令名，避免和系统命令或其它项目冲突；仓库内部不需要过早承担外部命名约束。

`common` 是公共库，不是用户直接执行的工具。它应承载工具链共享的数据契约和基础设施，例如 AST/model/derivation/view 的中间文件 schema、JSON 读写、诊断格式、退出码约定和公共测试 fixture。具体阶段逻辑仍归属各自工具：解析在 `parse`，建模在 `model`，推导在 `derive`，检查在 `check`，视图构建在 `view`，渲染在 `render`。

当前 `tools/pyveri` 内部已经先做了 CLI 子命令拆分，这只是过渡步骤，用于明确阶段边界和测试现有行为；它还没有达到最终的独立工具链结构。后续拆分应以独立目录、独立入口和中间文件协议为目标。

## 阶段工具职责

第一版独立工具链包含 6 个阶段工具：`parse`、`model`、`derive`、`check`、`view` 和 `render`。`common` 是公共库，`pyveri` 是 driver，不计入阶段工具。

中间文件第一版优先采用 JSON。JSON 便于人工检查、测试断言和早期 schema 演进；每类中间文件都应包含 `schema`、`version` 和必要的来源信息。

### parse

`parse` 只负责把 `.spec` 源文件转换成语法级 AST 中间文件：

```text
*.spec -> ast.json
```

它应读取 `.spec`、去除注释并保留源码行号，解析顶层声明、对象结构、状态结构、事件结构和 block 条目。复杂表达式第一版保留为原始字符串。AST 节点、block 和 block entry 都应保留 `span`，用于后续诊断指回 `.spec` 行号。

`parse` 不检查对象引用、状态引用或事件引用是否存在，不执行推导，不生成视图，不渲染输出。它只回答：源文件能否被解析成结构化语法树。

### model

`model` 负责把语法级 AST 转换成静态对象模型中间文件：

```text
ast.json -> model.json
```

它应建立对象表、类型/枚举/函数/谓词索引、父子关系、状态表、事件表，以及事件的源状态、目标状态、依赖、驱动队列和延期义务。它负责检查重复声明、未知父对象、未知事件目标状态、未知事件引用和未知状态引用，并保留源码 span。

`model` 不执行事件，不判断 `depends_on` 当前是否满足，不递归执行 `drives`，不判断目标是否可达，不生成视图或图形。它只回答：AST 能否形成引用一致、结构可索引的对象模型。

### derive

`derive` 负责从静态对象模型执行规格内的状态推导，生成推导结果中间文件：

```text
model.json -> derive.json
```

它应根据 `initial_state` 初始化状态表，从目标事件开始按事件源状态、`depends_on`、`drives` 声明顺序和目标状态推进推导。进入某个对象状态后，`derive` 负责处理该状态的 `invariant`：能直接确认的记录为 `proved`，无法求解的复杂谓词记录为 `obligation`，不满足的状态条件记录为 `blocked` 或 `contradiction`。它还负责收集 `deferred`、最终状态表、记录、统计，以及后续 trace 输出所需的结构化数据。

`derive` 只做推导执行和事实收集，不做最终验证裁决。它不决定整体是否通过、不定义退出码策略、不判断 `obligation` 或 `deferred` 在某种策略下是否允许。当前实现中的 `ok` 字段可作为过渡摘要，但未来最终通过/失败应由 `check` 解释。

### check

`check` 负责读取推导结果，根据验证策略给出最终判定和退出码：

```text
derive.json -> check.json
```

它只针对 `derive` 已经产出的最终结果做策略解释和后续处理，不重新执行推导，也不重新验证每个对象状态。第一版默认策略可以是：`target_reached == true`，且没有 `blocked` 和 `contradiction` 即通过；`obligation` 和 `deferred` 允许存在。

后续可扩展不同策略，例如不允许 `obligation`、不允许 `deferred`、CI 严格策略等。同一个 `derive.json` 可以被不同 `check` 策略解释。

### view

`view` 负责把模型或推导结果转换成视图模型中间文件：

```text
model.json/derive.json -> view.json
```

它决定“要展示什么”，而不是“怎么输出”。第一版视图类型包括 `object`、`drives`、`timeline` 和 `trace`。`object` 和 `drives` 可来自 `model.json`；`timeline` 可以来自模型或推导结果；`trace` 应来自 `derive.json`。视图模型应组织节点、边、层级、顺序、状态、分组和 trace 行等渲染前数据。

`view` 不解析 `.spec`，不构建模型，不执行推导，不判断验证是否通过，不直接生成 DOT/SVG/text。

### render

`render` 负责把 `view.json` 渲染成最终输出：

```text
view.json -> text/DOT/SVG/animated SVG
```

它决定“怎么输出”。第一版至少支持 `text`、`dot` 和 `svg`，后续可增加 `animated-svg`、HTML、Markdown、PNG 或视频导出。`render` 不直接读取 `model.json` 或 `derive.json`，除非后续明确增加快捷路径；它的稳定输入应是 `view.json`。

长期看，`render` 可以演进为 renderer host：自身负责读取 `view.json`、根据 `--format` 选择 renderer、传递数据、处理输出路径和错误码；不同格式由独立 renderer 模块或插件处理，例如 text renderer、DOT renderer、SVG renderer、animated SVG renderer。

动画输出遵循简单直观、开源免费、面向工程人员理解使用的原则。优先采用基于开放标准和浏览器原生能力的 `animated SVG`，第一阶段不引入外部动画框架。动画只用于表达推导顺序、状态推进、事件进入/退出、对象出现和 blocked 位置，不做装饰性特效。

## 规格语义

- `object` 是推导中的实体单位。
- `parent` 建立对象之间的静态父子层级。
- `initial_state` 给出对象进入推导模型时的初始状态。
- `state` 定义对象可处于的状态。
- `on Event::X -> State::Y` 定义状态迁移规则。
- `depends_on` 是事件执行前必须满足的前提。
- `drives` 是事件触发的子推导队列，必须按声明顺序处理。
- `invariant` 是对象进入某状态后必须验证的状态约束。
- `deferred` 是规格明确保留的延期证明义务，不等同于注释。
- 注释只供人阅读，解析时忽略。

## 实现阶段

### 1. Parser

输入 `.spec` 文件，输出语法级 AST。

第一版需要支持：

- 去除 `/* ... */` 和 `// ...` 注释。
- 解析 `enum`、`function`、`predicate`、`type`。
- 解析 `object`、`initial_state`、`parent`、`attrs`。
- 解析 `state`。
- 解析 `events` 和 `on Event::... -> State::...`。
- 解析 `depends_on`、`drives`、`may_change`、`invariant`、`deferred`。

第一版可以把复杂表达式保留为原始字符串，不急于完整求值。

### 2. Model Builder

把 AST 装配成可推导模型。

需要建立：

- 对象表。
- 类型表。
- 父子层级。
- 属性表。
- 每个对象的状态表。
- 每个对象的事件表。
- 事件的目标状态、依赖、驱动队列、延期义务。
- 状态的不变量集合。

这一阶段负责把解析得到的扁平声明反扁平化为对象层级和事件层级；它仍然不执行正式推导。

需要检查：

- 对象名重复。
- 状态名重复。
- 事件名重复。
- `parent` 指向的对象存在。
- 事件目标状态存在。
- `Object.Event` 引用存在。
- `Object.state == State::X` 引用存在。

语法错误和模型结构错误应作为 error 报告并停止；不影响建模但可能影响后续推导质量的问题可以作为 warning 报告。

当前进展：

- 已能从当前 `.spec` 建立对象、状态、事件和静态父子层级。
- 已检查重复声明、未知父对象、未知事件目标状态、未知事件引用和未知状态引用。
- 已提供对象视图、驱动关系视图和时间轴视图。
- 图形输出中，`object` 和 `drives` 仍使用 Graphviz DOT；`timeline` 直接生成 SVG。
- 已加入最小推导器，可从 `StartupTimeline.Event::Setup` 按事件源状态、`depends_on`、`drives` 顺序和目标状态执行静态推导，并收集 `proved`、`assumed`、`obligation`、`deferred`、`blocked` 和 `contradiction` 结果。
- 已修正当前真实规格中的严格推导阻塞点：`KernelImage.Event::Enable` 不再依赖聚合完成态 `Vm.state == State::Ready`，而是依赖更局部的 `EarlyVm.state == State::Online`。当前 `StartupTimeline.Event::Setup` 可严格推出 `StartupTimeline.state == State::Ready`，复杂谓词仍作为 `obligation` 保留。

### 2.1 Timeline View

`timeline` 视图用于展示当前 `.spec` 能推出的启动时间线，不等同于完整推导证明结果。

当前布局采用单元格模型：

- 从左到右依次划分为阶段列、子阶段列、状态列和对象列。
- 纵向时间自底向上。
- 先生成状态单元，再由状态单元合并出上层阶段单元和子阶段单元。
- 阶段列当前显示 `PreparePhase` 和 `BootPhase`。
- 子阶段列当前在 `BootPhase` 内显示 `EntryPreludePhase`；`PreparePhase` 当前没有子阶段，子阶段列保持为空。
- 子阶段单元必须嵌在所属阶段单元内部，并与其覆盖的状态单元边界对齐；由于时间向上，`EntryPreludePhase` 当前对齐 `BootPhase` 的下边界。
- 状态列当前包括 `PreparePhase.Ready`、`PreparePhase.Online`、`EntryPreludePhase.Ready` 和 `BootPhase.Ready`。
- 对象列只填入达到某个阶段或子阶段状态时形成的对象状态结果。
- 每个对象在同一张时间轴图中只出现一次，显示其在当前时间轴视图中的最终状态。
- 对象单元按固定列数换行；排不开时在同一状态单元内向上堆叠。

当前对象填充规则：

- 准备期对象来自 `PreparePhase.Online` 的状态不变量，例如 `Riscv64`、`Lds`、`StaticObjects`、`Config`、`PhysicalMemory`。
- 入口前导期对象来自 `EntryPreludePhase.Setup` 递归驱动出的对象状态推进结果，例如 `RootStream`、`InterruptStream`、`Vm`、`InitTask`、`InitStack`、`EventStream` 等。
- `StartupTimeline` 是顶层时间轴对象，不在图中显示。
- 阶段对象和子阶段对象只通过左侧单元体现，不在对象列重复显示。

待确认或后续改进：

- 当前 `EntryPreludePhase` 到 `BootPhase` 的归属仍由工具内的阶段关系规则确定；后续应尽量从 `.spec` 的阶段父子关系和推导路径中通用推导。
- 当前对象列显示的是视图级状态结果，不代表完整证明引擎已经验证所有 `depends_on` 和 `invariant`。
- 单元宽度、对象宽度、对象列数、行高和内边距已经集中在 SVG 渲染逻辑中，后续可暴露为 CLI 参数或配置。

### 2.2 Toolchain Direction

后续 `pyveri` 应从单体命令逐步调整为工具链。

整体方向：

- 将解析、建模、推导、检查、视图生成和结果渲染拆成若干单独的小工具。
- `pyveri` 保留为总入口和集成者。
- `pyveri` 根据命令行参数选择并组织小工具，形成对应的处理链。
- 每个小工具的输入和输出应尽量明确，便于单独调试、复用和缓存。

可能的工具链阶段：

- `parse`：`.spec` 源文件到语法 AST。
- `model`：AST 到静态对象模型。
- `derive`：静态对象模型到推导结果。
- `check`：对模型或推导结果执行验证。
- `view`：从模型或推导结果生成文本、图形或其它视图模型。
- `render`：把视图模型渲染为 text、DOT、SVG 等最终格式。

工程下可以设置一个默认输出目录，用于保存中间构建文件和最终结果文件。该目录至少包含两个子目录：

- `build`：保存中间构建文件，例如解析后的 AST、静态对象模型、推导轨迹、视图模型等。
- `out`：保存最终结果文件，例如文本报告、DOT、SVG、JSON 报告等。

缓存和依赖关系：

- 中间文件可以避免每次都从 `.spec` 源文件完整重建。
- 每个中间文件应记录生成它所依赖的源文件、工具版本、参数和上游中间文件。
- 当源文件、工具版本、参数或上游中间文件变化时，应触发对应阶段及其下游阶段重建。
- 依赖触发机制可以参考 Makefile 的思路，但不一定直接使用 Makefile。
- 第一版可以先使用时间戳和内容摘要；后续再考虑更完整的构建数据库。

### 3. Derivation Engine

从推导入口开始执行静态推导。

基本规则：

1. 每个对象从 `initial_state` 开始。
2. 推进事件前，当前对象必须处于包含该事件的源状态。
3. 推进事件前，`depends_on` 必须可证明或形成明确证明义务。
4. `drives` 中的子事件按声明顺序递归推进。
5. 子事件全部完成后，当前对象进入目标状态。
6. 进入目标状态后验证该状态的 `invariant`。

第一版不做复杂谓词求值；未定义或不可计算的谓词进入证明义务列表。

### 4. Diagnostics

推导结果至少分为：

- `proved`：由模型规则成功推出。
- `assumed`：作为当前规格范围的输入事实接受。
- `obligation`：无法在第一版中求值，但必须保留的证明义务。
- `deferred`：规格明确标记的延期证明义务。
- `blocked`：缺少必要前提，导致事件无法推进。
- `contradiction`：模型内部出现状态或约束矛盾。
- `syntax_error`：语法错误，停止处理。

### 5. CLI

第一版命令形式：

```bash
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive --strict
```

默认目标：

```text
StartupTimeline.Event::Setup
```

后续可以增加参数：

```text
--target StartupTimeline.Event::Setup
--format text|json
--strict
```

当前 CLI 已支持 `--derive`、`--target` 和 `--strict`。默认推导摘要即使为 `blocked` 也返回 0，`--strict` 用于把未达目标的推导结果作为命令失败处理。

后续 CLI 改进：

- 将 `--derive` 行为改为默认行为。当前默认命令已经输出推导摘要，但完整推导报告仍需要显式 `--derive`；后续应让 `python -m pyveri <spec>` 默认执行完整推导，并用参数选择只解析、只建模或只输出视图。
- 保留显式子命令或参数用于工具链阶段，例如 `parse`、`model`、`derive`、`view`、`render`，避免默认行为变复杂后难以单独调试。

当前主开发环境已经迁移到 WSL2/Linux；文档和日常命令优先使用 `/` 路径分隔符、`PYTHONPATH=... command` 环境变量形式和 `sh tools/pyveri/bin/pyveri ...` 本地脚本。Windows PowerShell 命令作为兼容旧环境保留。

### 5.1 Next Execution Plan

后续工作按“小步可验证”的方式推进。当前优先级是先把已经发现的规格阻塞点、测试和开发环境问题逐步收口，再继续扩展工具链。

#### Step A: 修正当前规格阻塞点

已完成。此前严格推导阻塞在：

```text
Vm.Event::Setup
  drives KernelImage.Event::Enable

KernelImage.Event::Enable
  depends_on Vm.state == State::Ready
```

但 `Vm.Event::Setup` 自身完成前，`Vm.state` 仍是 `State::Prepared`。因此这是规格模型中的阶段依赖自锁，不是工具运行错误。当前修正为让 `KernelImage.Event::Enable` 依赖 `EarlyVm.state == State::Online`，表达其真实局部需求：EarlyVm 对应的早期虚拟地址空间已经可用。

完成标准：

```bash
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive --strict
```

当前结果不再因为该状态依赖产生 `blocked`。

#### Step B: 将当前真实规格纳入严格推导测试

已完成。`tools/pyveri/tests/test_derive.py` 已从“报告 blocked”改为“目标可达”：

- 目标为 `StartupTimeline.Event::Setup`。
- 期望最终 `StartupTimeline.state == State::Ready`。
- 复杂谓词仍允许作为 `obligation` 保留。

完成标准：

```bash
PYTHONPATH=tools/pyveri/src python -m unittest discover -s tools/pyveri/tests
```

全部通过，且严格推导命令返回 0。

#### Step B.0: 改进诊断行号精度

已完成。`Block` 现在记录 block body 的真实起始行，并提供 entry 级 `SourceSpan`。推导报告中的 `depends_on`、`invariant`、`drives` 和 `deferred` 诊断可以指向具体条目行号。例如把当前规格临时回退为旧依赖后，报告会指向 `line 833: depends_on requires Vm.state == State::Ready`，而不是此前的 block 起始行。

#### Step B.1: 统一 WSL2/Linux 换行策略

已完成。当前主开发环境已迁移到 WSL2/Linux，仓库已添加 `.gitattributes` 换行策略，并确认 tracked 文本文件当前没有 CRLF `^M` 残留。这对 `pyveri` 解析和推导语义通常没有影响：Python 文本读取会处理通用换行，当前 parser 的行号统计也主要依赖 `\n`。

当前策略：

```gitattributes
* text=auto eol=lf
*.cmd text eol=crlf
*.png binary
```

源码、文档和 `.spec` 文件统一按 LF 管理。Windows 批处理脚本 `*.cmd` 保留 CRLF 策略，避免影响旧 Windows 环境。

完成标准：

```bash
PYTHONPATH=tools/pyveri/src python -m unittest discover -s tools/pyveri/tests
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive
```

两条命令行为与换行清理前一致。

#### Step C: 改进推导报告可读性

在严格推导通过后，再优化报告输出，避免过早打磨未稳定的结果格式。

候选改进：

- 当前完整推导报告过于平铺，信息噪音较大。后续应重新设计输出层次，默认只显示摘要、目标状态、根因 blocked、deferred 和 obligation 统计；详细 transitions 和全部 obligation 应通过 verbose/detail 参数打开。
- 将默认推导过程显示为进入/退出式 trace，而不是平铺的 `transitions` 列表。每个事件输出成对记录：进入行使用 `>`，退出行使用 `<`；被 `drives` 的子事件缩进两格嵌套在中间。成功退出行不额外标注 `ok`，失败退出行标注 `blocked:` 或 `contradiction:` 并附带原因。
- 按 `blocked`、`deferred`、`obligation` 分组时进一步按对象/事件/状态分组。
- 对 `blocked` 输出根因链，而不是只输出逐层传播的 blocked。
- 在摘要中区分“目标已达但存在 obligation”和“目标未达”。
- 增加 `--format json`，为后续工具链中间产物做准备。

#### Step D: 工具链拆分

目标是拆成 `gcc` 风格的 driver/tool 工具链。`pyveri` 作为 driver，独立阶段工具并列放在 `tools/` 下，通过中间文件衔接。

当前已完成过渡步骤：先在 `tools/pyveri` 内部拆出 CLI 阶段边界，并保留旧参数形式兼容：

- `parse` 输出解析摘要。
- `model` 输出静态对象模型摘要。
- `derive` 输出推导报告和证明义务。
- `check` 将推导结果解释为通过、阻塞或矛盾，并用退出码表达。
- `view` 从模型生成文本视图。
- `render` 从模型生成 DOT 或 SVG 输出。

独立工具链第一步已开始落地：

- 已建立 `tools/common` 公共库骨架，包含 AST/model/derive/check/view schema 标识、JSON 读写辅助、共享 AST 类型、共享模型类型、共享推导结果类型、默认 target 和共享的 `model.json` 反序列化逻辑。`common.model_json` 已脱离 `pyveri` 包依赖。
- 已建立 `tools/parse` 独立阶段工具，当前已迁出 parser 逻辑并脱离 `pyveri` 包依赖，可把 `.spec` 输出为 `ast.json`。
- `ast.json` 已包含 `schema`、`version`、`source`、语法树节点、block entry 和源码行号 span。
- 已建立 `tools/model` 独立阶段工具，当前已迁出 model builder 逻辑并脱离 `pyveri` 包依赖，可读取 `ast.json` 并输出索引化的 `model.json`。
- `model.json` 已包含 `schema`、`version`、`source`、summary、diagnostics、对象/状态/事件索引、children 和源码行号 span。
- 已建立 `tools/derive` 独立阶段工具，当前已迁出推导引擎逻辑并脱离 `pyveri` 包依赖，可读取 `model.json` 并输出结构化 `derive.json`。
- `derive.json` 已包含 `schema`、`version`、`source`、target、summary、最终状态表、records 和 transitions。
- 已建立 `tools/check` 独立阶段工具，当前可读取 `derive.json`，按默认策略输出 `check.json` 并返回通过/失败退出码。
- `check.json` 已包含 `schema`、`version`、policy、target、verdict、exit_code、summary、allowed 和 reasons。
- 已建立 `tools/view` 独立阶段工具，当前可读取 `model.json` 并输出 `object`、`drives` 或 `timeline` 的 `view.json`。
- `view.json` 已包含 `schema`、`version`、source、view、rankdir、graph_format、nodes、edges 和 metadata。
- 已建立 `tools/render` 独立阶段工具，当前可读取 `view.json` 并输出 text、DOT 或 SVG。
- `render` 当前复用 `pyveri.view` 中的文本、DOT 和 SVG 渲染逻辑，后续可继续拆分为 renderer host 和格式插件。
- `pyveri` 已收敛为 driver，当前通过子进程调度 `parse`、`model`、`derive`、`check`、`view`、`render` 独立工具，并保留旧 CLI 输出兼容。默认使用临时目录保存中间文件；传入 `--work-dir` 时会把本次流水线生成的中间文件保留在指定目录。

后续需要继续推进到独立工具形态：

- 明确 `common` 公共库的边界和中间文件 schema。
- 按上述阶段工具职责细化中间文件 schema 和退出码。
- 将当前 `tools/pyveri/src/pyveri/` 中的阶段逻辑逐步迁移到并列工具目录。
- 在已支持 `--work-dir` 持久中间文件目录的基础上，继续补缓存策略和增量重建判断。

第一版中间产物可以先落在：

```text
tools/build/
tools/out/
```

缓存策略先采用内容摘要和参数摘要，不急于引入完整构建数据库。

### 6. Tests

第一版测试应覆盖：

- 注释剥离。
- 基础语法解析。
- 对象父子层级。
- 状态和事件引用检查。
- `drives` 顺序保持。
- `deferred` 收集。
- 从 `StartupTimeline.Event::Setup` 推导到目标状态的最小路径。

## 第一版非目标

- 不解析或执行真实内核代码。
- 不读取运行时日志、插桩、模拟器或硬件采集数据。
- 不做参考内核差分。
- 不完整求解所有谓词语义。
- 不引入 SMT/SAT 求解器。
