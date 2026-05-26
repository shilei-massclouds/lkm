# Coding 规格

本目录记录从模型规格生成或指导内核代码实现时需要遵守的补充约束。

主规格和 `spec/model` 仍是对象、状态、事件、依赖、阶段顺序和证明边界的权威来源；本目录不重新定义模型语义，只补充对象级编码阶段必须知道的架构、语言、参考实现和工程组织要求。若 coding 规格需要覆盖某条模型规格，必须明确列出覆盖范围、理由和生效对象。

## 阅读入口

`spec/coding/README.md` 是 coding 规格目录的说明入口，只负责说明范围、文档优先级和阅读顺序。正式规格入口是 [`main.spec`](main.spec)；对象级代码生成的硬性约束从 `main.spec` include 的 [`mapping.spec`](mapping.spec) 进入。实现代码前必须先阅读正式 `.spec` 入口，再阅读对应 `.md` 说明。

当前阅读顺序为：

1. `README.md`：确认 coding 规格范围、外部规格优先级和本目录阅读顺序。
2. `main.spec`：coding 目录正式规格入口。
3. `mapping.spec`：对象、Phase、状态、事件、checkpoint、源码落点和规则强度分层的正式规格。
4. `riscv64.spec`：RISC-V64 架构、链接脚本和入口地址语义相关的正式规格。
5. `rust.spec`：Rust 语言、安全边界和 ABI 相关的正式规格。
6. `mapping.md`：对 `mapping.spec` 的说明、例子和补充解释，不覆盖正式规格。
7. `riscv64.md`：RISC-V64 架构相关补充说明。
8. `rust.md`：Rust 语言、安全边界和 crate 信任边界相关补充说明。
9. `arceos.md`：参考 ArceOS 时的取舍原则。
10. `arceos_ex-plan.md`：当前实验内核的执行计划和任务状态；它不覆盖前述规格，只记录当前阶段如何落实规格。

若后读文档与先读文档发生冲突，不能自行选择更方便的解释。必须回到上级规格确认：模型语义优先于 coding 规格，`mapping.spec` 的 `MUST` 优先于其它 coding 补充文档，`mapping.spec` 的 `SHOULD` 需要默认遵循或显式记录偏离原因，计划文档不得覆盖规格文档。

## 阶段边界

规格指导实现时分为两个连续阶段：

1. `Object Coding Phase`：对象级编码阶段。该阶段只考虑模型对象如何在代码中被承载，包括对象状态、对象属性、对象间依赖、事件推进、状态检查、checkpoint 和必要的架构/语言安全边界。
2. `Composition Phase`：组合封装阶段。该阶段在对象级实现语义已经明确的前提下，决定对象如何被组织进 crate、module、公开接口、构建 profile 和应用接入路径。

`spec/coding/` 只描述 `Object Coding Phase`。对象级编码阶段不以最终 crate/module 边界为优先目标；代码物理上可以暂时放入某个 crate，但该 crate 只作为当前实现载体，不自动成为最终组件边界。

组件、module、crate、公开接口、adapter、构建接入以及与 ArceOS 组件体系兼容相关的规则，记录在 `spec/compose/`。`Composition Phase` 不应改变对象状态、事件迁移和依赖语义，只决定这些对象级实现如何组合和发布。

## 当前实践目标

当前目标是实践主规格文档的第二点用途：基于规格指挥 AI 生成内核代码。

第一轮实践目标暂定为：

- 实现语言：Rust。
- 目标体系结构：RISC-V64；第一轮不做多架构兼容实现。
- 参考实现：ArceOS。
- 目标内核：`tgoskits` 中新增 `os/arceos_ex`，作为 ArceOS 的规格化演化候选。
- 目标平台：新增 RISC-V64 generic SBI/FDT 平台实现，当前以 QEMU 为测试环境，但不以 QEMU virt 硬编码作为平台语义来源。
- 规格来源：优先使用 `spec/model` 下已经规格化的启动阶段模型，必要时由本目录补充编码约束。

第一轮内核形态以 ArceOS 的 Unikernel 方式起步，由 `helloworld` 应用引领构成最小系统。即使应用只是 `helloworld`，也必须完整支撑当前模型中的入口前导期和入口后继期两个子阶段。

## 规格优先级

编码阶段按以下顺序解释规格：

1. `spec/model/SEMANTICS.md`：模型生命周期硬语义，不允许被编码便利性绕过。
2. `spec/model/**/*.spec`：对象、状态、事件、依赖、驱动顺序和阶段完成条件。
3. `spec/组件化内核规格.md`：模型意图、设计背景、对象解释和参考边界。
4. `spec/coding/main.spec` 与其 include 的正式规格：面向对象级代码实现的硬约束。
5. `spec/coding/*.md`：面向对象级代码实现的补充说明。
6. `spec/compose/main.spec` 与其 include 的正式规格：面向 crate/module 组合、公开接口和构建接入的硬约束。
7. `spec/compose/*.md`：面向 crate/module 组合、公开接口和构建接入的补充说明。
8. `tgoskits` 或当前实验内核目录内的本地约定：目录、构建、测试和已有抽象。

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
| `deferred` | 显式 stub、TODO 或 feature gate，不得隐式实现 |

更细的映射规则见 [`mapping.md`](mapping.md)。

## 最小编码原则

- 严格遵循规格指导。若某个模型约束、阶段边界、状态迁移、前置依赖或后置检查无法满足，必须停止实现并报告，不得用隐式假设绕过。
- 编码实现受“推导义务门禁”约束。`make verify` 报告的 `obligation` 不是实现可忽略的提示；实现某个阶段或对象事件前，必须先检查它直接依赖的义务。能够由模型、推导器、链接脚本、ISA、固件规范或已证明前序事实推出的义务，应优先补齐推导证明；只能由外部交付保证支撑的义务，必须明确记录为 source assumption，并在后续最早可行事件中转化为运行期检查；无法归类的义务应视为规格缺口或延期项，不得宣称对应阶段已经闭合实现。
- 先完成对象级语义正确的最小内核路径，再进入组件组合和通用框架整理。
- 对象实现建议优先使用能表达当前语义的高粒度复合对象边界，由该对象封装属性和子对象状态，并驱动子对象事件完成自身事件；若必须展开为较低粒度对象或临时混合承载，应记录原因。
- 对象生命周期函数必须使用模型事件名称，除非 coding 规格明确给出本地别名。
- checkpoint 只能由对应对象事件或阶段边界的映射实现发出。低层阶段、资源对象、入口汇编或 continuation 不得为了让运行期 trace 视觉上贴合推导 trace，而代发父阶段、兄弟阶段、准备期或其它对象的合成 checkpoint。
- linker script 是 model 中 `Lds` 准备期对象的代码生成产物，必须由 `Lds` 属性及其依赖的 `Config` 属性共同驱动。硬编码链接常量只能作为带记录的过渡例外存在，并必须指出对应的 `Lds`/`Config` 来源。
- 每个 `unsafe` 块必须对应清晰的硬件、链接器、启动 ABI 或裸机内存访问边界。
- 参考 ArceOS 时优先参考工程边界和成熟实现经验，不以逐行复刻为目标。
- 对尚未建模的实现需求，应先记录为 coding 约束或模型缺口，再决定是否直接编码。

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
- `main.spec`：coding 目录正式规格入口。
- `mapping.spec`：模型对象、阶段、状态、事件和检查点到代码的正式规则，包括 `MUST`、`SHOULD`、`MAY` 和 `NOTE` 分层。
- `riscv64.spec`：RISC-V64 链接脚本、入口地址事实和地址转换来源的正式规则。
- `rust.spec`：Rust 语言、安全边界和 ABI 使用的正式规则。
- `mapping.md`：模型对象、阶段、状态、事件和检查点到代码的说明性映射文档。
- `arceos_ex-plan.md`：`arceos_ex` 第一轮实现任务清单；不得作为覆盖规格的依据。
