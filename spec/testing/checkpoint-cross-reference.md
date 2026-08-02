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

## Scheduler 差分 checkpoint

Scheduler 差分只增加只读观测，不改变 Linux 调度行为。成对运行至少采集并比较：

- Schedule entry：CPU、schedule mode、prev identity、prev state、prev on-rq；
- PreparePrev exit：pending signal 是否恢复 running、是否 block/deactivate、最终 on-rq；
- PickNext exit：prev、next 与所选 sched class；
- 仅 `next != prev` 时出现的 SwitchTo entry；
- next stack 上的 finish-task-switch 与 schedule return。

每个 marker 必须位于能稳定读取对应事实的 Linux `__schedule()`/context-switch 边界，并以源码顺序证明
PreparePrev 先于 pick、pick 先于 class handoff、非 identity switch 先于 next-stack finish。identity case
不得合成 SwitchTo marker。Boot 首次 `schedule_preempt_disabled()` 必须单独纳入 case-local scope，用于证明
BootTask 在 pick 前未被错误 deactivate。具体函数、符号和 anchor 只记录于 mapping/instrumentation plan；
不得写入 Coding 核心规格。

当前 sibling Linux 的差分记录使用独立于生命周期 checkpoint ID 的定长只读缓冲区，并在既有
`lkm_checkpoints_dump()` 前导区输出 `scheduler_diff:` 记录；关闭 `CONFIG_LKM_CHECKPOINTS` 时 recorder
必须编译为空操作。字段与 anchor 固定如下：

| stage | Linux anchor | 必须采集的字段 |
| --- | --- | --- |
| `entry` | `kernel/sched/core.c::__schedule()` 取得 `cpu_rq(cpu)->curr` 后、`schedule_debug()` 前 | cpu、mode、prev pid、`prev->__state`、`prev->on_rq`、入口 preempt flag |
| `prepare_prev` | signal recovery 或 `block_task()` 完成后、`pick_next_task()` 前；SM_IDLE identity fast path 同样单独记录 | prev 最终 state/on-rq、signal-recovered、blocked、SM_PREEMPT disposition |
| `pick_next` | `pick_next_task()` 返回后；SM_IDLE identity fast path 在 `goto picked` 前 | prev/next pid、prev state/on-rq、blocked、next class |
| `switch_entry` | `likely(prev != next)` 分支入口、更新 `rq->curr` 前 | prev/next pid、blocked、next class；identity 严禁出现 |
| `finish_return` | next stack 的 `finish_task_switch()` 完成 mm/dead-task cleanup 后、return 前 | cpu、已预捕获的 prev pid/state/on-rq、current pid/class |

class 编码只用于差分输出，顺序为 stop、deadline、realtime、fair、idle；参考配置未启用的 ext/SCX
仍可被诊断为 `ext`，但不进入核心规格。缓冲记录必须先写 payload、以 release-order 最后发布 stage；
dead-task 路径必须在引用释放前捕获所有 prev 字段，finish recorder 不得解引用已释放的 prev。

## Coverage 与 paired difftest

coverage 审阅只从已提交 mapping JSON 聚合数量、confidence、Linux 文件和 unmapped family；不读写 Linux 树、
不改变 mapping 分类、不增加插桩或 runtime 采集。检查模式只做内存重建和漂移比较。

paired checkpoint difftest 的 `checkpoint_scope` 是 case-local 硬比较集，不代表所有 exact mapping 已纳入比较。
启用 coverage 的 case 必须把每个必需 checkpoint 归入 scope，或用稳定理由列入 `accounted_outside_scope`；未归类项必须在 dry-run/QEMU 前导致配置失败。
仓库内正式 Linux paired case 的 `checkpoint_scope` 只能包含当前 mapping 中的 `exact` checkpoint；
`range`、`unmapped` 或 mapping 中不存在的 checkpoint 只能保留为 scope 外观察项。mapping 分类变化后，
复合配置单测必须在 QEMU 前拒绝仍把非 exact checkpoint 留在硬比较集的 stale case。

## Marker patch

Linux marker patch 是显式同步动作，只能消费已提交的 `linux_checkpoint_instrumentation_plan.json`，并只写入调用者指定的 unified diff；默认不得直接修改 Linux 树或维护第二份 checkpoint 列表。
同一 anchor 的 marker 按 checkpoint index 排序，已存在的相同 marker 不重复。写出 patch 前必须拒绝指纹不匹配和计划中已不存在的 stale marker。
