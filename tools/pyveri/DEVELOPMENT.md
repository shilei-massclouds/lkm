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

当前诊断行号仍不够精确：推导报告显示的是 `depends_on` block 的起始行，而不是 block 内具体表达式所在行。例如此前报告显示 `line 824`，实际问题表达式 `Vm.state == State::Ready;` 在 `spec/entry-prelude-object-model.spec` 的第 833 行。后续应让 `Block.entries` 或等价结构携带 entry 级行号，使 `depends_on`、`invariant` 和 `drives` 的诊断能指向具体条目。

#### Step B.1: 统一 WSL2/Linux 换行策略

当前主开发环境已迁移到 WSL2/Linux，后续应清理文本文件中的 CRLF `^M`。这对 `pyveri` 解析和推导语义通常没有影响：Python 文本读取会处理通用换行，当前 parser 的行号统计也主要依赖 `\n`。但换行清理会造成大面积 diff，因此必须作为独立维护提交处理，不应混入规格语义或工具逻辑修改。

建议先添加或更新 `.gitattributes`：

```gitattributes
* text=auto eol=lf
*.cmd text eol=crlf
```

随后再统一规范化源码、文档和 `.spec` 文件的换行。Windows 批处理脚本 `*.cmd` 保持 CRLF。

完成标准：

```bash
PYTHONPATH=tools/pyveri/src python -m unittest discover -s tools/pyveri/tests
PYTHONPATH=tools/pyveri/src python -m pyveri spec/entry-prelude-object-model.spec --derive
```

两条命令行为与换行清理前一致。

#### Step C: 改进推导报告可读性

在严格推导通过后，再优化报告输出，避免过早打磨未稳定的结果格式。

候选改进：

- 按 `blocked`、`deferred`、`obligation` 分组时进一步按对象/事件/状态分组。
- 对 `blocked` 输出根因链，而不是只输出逐层传播的 blocked。
- 在摘要中区分“目标已达但存在 obligation”和“目标未达”。
- 增加 `--format json`，为后续工具链中间产物做准备。

#### Step D: 工具链拆分

当严格推导和报告格式稳定后，再进入工具链拆分：

- `parse` 输出 AST。
- `model` 输出静态对象模型。
- `derive` 输出推导轨迹和证明义务。
- `check` 将结果解释为通过、阻塞或矛盾。
- `view` 从模型或推导结果生成视图模型。
- `render` 输出 text、DOT、SVG 或 JSON。

第一版中间产物可以先落在：

```text
tools/pyveri/build/
tools/pyveri/out/
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
