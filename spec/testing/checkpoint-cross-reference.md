# Checkpoint Cross-reference 与差分验证

本文件承载具体实现之间的 checkpoint inventory、静态对照、coverage 审阅、插桩计划和差分验证规则。
这些规则是 Testing/cross-reference 支撑责任，不是 Coding 核心语义，不得反向定义 Charter、Model 或
Model 到代码表示的约束。

## Checkpoint inventory

inventory 只消费 `impl/arceos_ex/src/checkpoint/mod.rs` 已有的 checkpoint 枚举、稳定名称和可选早期宣告元数据，
并在 `tools/out/checkpoints/` 生成机器可读 JSON 与人类可读 Markdown。它不得修改 checkpoint 行为、handler、
KUnit 输出、运行时观测或任何参考实现。检查模式必须在内存中重建预期产物并报告漂移，不得重写仓库输出。

## Linux 静态对照

Linux checkpoint mapping 只是只读 cross-reference。它消费已导出的 `arceos_ex` inventory，读取用户指定或默认
`../linux-6.12` 参考树，并按 inventory 顺序为每个 checkpoint 产生 `exact`、`range` 或 `unmapped` 记录。

- `exact` 必须有稳定的 Linux 函数、符号或 call anchor。
- `range` 只声明可复核的有序源码区间，不暗示 Linux 拥有对应的规格对象。
- 无法稳定证明的对照必须保持 `unmapped`，并记录原因，不得猜测。
- 架构或配置条件化的 ABI wrapper 必须保留范围与 confidence；RISC-V64 `head.S`、`setup_vm()` 和
  return-to-user anchor 不得宣称跨架构等价。
- boot-time `kernel_execve()` 与运行期 `execve/execveat` 即使共享 helper，也必须保持不同的事件 owner。

mapping pass 不得修改 Linux 树、添加 probe、修改 `arceos_ex` 行为或 checkpoint handler，也不得把静态锚点
当作已完成插桩或语义等价的证明。已由本项目生成的 marker/recorder 行在解析锚点时必须从内存视图忽略，但这不替代独立的 stale/mismatch 检查。

## Coverage 与 paired difftest

coverage 审阅只从已提交 mapping JSON 聚合数量、confidence、Linux 文件和 unmapped family；不读写 Linux 树、
不改变 mapping 分类、不增加插桩或 runtime 采集。检查模式只做内存重建和漂移比较。

paired checkpoint difftest 的 `checkpoint_scope` 是 case-local 硬比较集，不代表所有 exact mapping 已纳入比较。
启用 coverage 的 case 必须把每个必需 checkpoint 归入 scope，或用稳定理由列入 `accounted_outside_scope`；未归类项必须在 dry-run/QEMU 前导致配置失败。

## Marker patch

Linux marker patch 是显式同步动作，只能消费已提交的 `linux_checkpoint_instrumentation_plan.json`，并只写入调用者指定的 unified diff；默认不得直接修改 Linux 树或维护第二份 checkpoint 列表。
同一 anchor 的 marker 按 checkpoint index 排序，已存在的相同 marker 不重复。写出 patch 前必须拒绝指纹不匹配和计划中已不存在的 stale marker。
