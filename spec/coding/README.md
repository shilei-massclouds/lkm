# Coding 规格

本目录记录从模型规格生成或指导内核代码实现时需要遵守的补充约束。

主规格和 `spec/model` 仍是对象、状态、事件、依赖、阶段顺序和证明边界的权威来源；本目录不重新定义模型语义，只补充编码阶段必须知道的架构、语言、参考实现和工程组织要求。若 coding 规格需要覆盖某条模型规格，必须明确列出覆盖范围、理由和生效对象。

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
4. `spec/coding/*.md`：面向具体代码实现的补充约束。
5. `tgoskits` 内目标内核的本地约定：目录、构建、测试和已有抽象。

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
- 先实现能承载当前模型阶段的最小内核路径，再扩展通用框架。
- 对象生命周期函数必须使用模型事件名称，除非 coding 规格明确给出本地别名。
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
- `cargo xtask` 负责管理 `arceos_ex` 构建所需的顶层 Cargo 依赖映射或生成配置；不要求开发者手工来回修改顶层 `Cargo.toml`。

## 当前文件

- `riscv64.md`：RISC-V64 架构相关编码约束。
- `rust.md`：Rust 语言和安全边界相关编码约束。
- `arceos.md`：参考 ArceOS 时的取舍原则和映射约束。
- `mapping.md`：模型对象、阶段、状态、事件和检查点到代码的映射规则。
