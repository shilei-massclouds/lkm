# Guidance 规格

本目录记录约束 AI 或其它代码生成器行为的上层指导规格。

这些规则不重新定义 model、coding、compose 或 testing 语义，而是规定生成器在使用这些规格时必须遵守的工作流。例如生成测试用例时，应先阅读原则和具体规格要求，再实现，最后检查结果是否符合原则和要求。

正式规格入口是 [`main.spec`](main.spec)。

## 仓库变更工作流

所有 AI 或代码生成器参与的仓库变更必须遵守三条硬约束：

- 不要猜测修复。遇到问题时，必须先通过可复现观测逐步缩小范围，再下结论和改代码。
- 先规格后实现。涉及行为、接口、对象边界或 Linux 差分语义的改动，必须先更新适用的 model、coding、testing 或 guidance 规格，再改实现。
- 每次修改代码后，最终回归必须在仓库根目录运行 `make test`。Focused test 只能用于中间定位，不能替代最终回归。

Linux checkpoint 对齐映射任务属于只读 cross-reference 阶段：代理只能消费已有 arceos_ex checkpoint inventory、读取参考
Linux 源码树并生成可审阅清单，不得把该任务扩展成 Linux 源码插桩、运行时采集、checkpoint handler 修改或行为修改。

checkpoint inventory 和 Linux mapping 的测试模式是只读漂移检测：只能在内存中重新生成 JSON/Markdown，与
`tools/out/checkpoints/` 中已提交的审阅产物比较；发现漂移应作为测试失败报告，不得在测试模式下重写产物。
Linux mapping 可以只读解析 C 函数、`SYSCALL_DEFINE*` macro 和 assembly symbol/label；遇到 arch/config
条件化 syscall ABI wrapper 时，只能映射到共同 helper 或保持 unmapped，并在 confidence/notes 中保守说明。
Linux checkpoint mapping coverage 也是 mapping-only 审阅产物，只能从已提交 mapping JSON 聚合覆盖率视图，不得读取
或修改 Linux tree、不得重解释 mapping 语义、不得新增 runtime 采集或 checkpoint handler；其测试模式同样只能做只读漂移检测。

后续若进入 Linux 侧 checkpoint 插桩同步阶段，插桩清单和 Linux marker 必须从本项目 checkpoint inventory 与已提交
Linux mapping 派生，不得在 Linux tree 内维护独立 checkpoint 列表。同步工具必须能从 mapping 生成或校验
instrumentation plan，并报告三类漂移：mapping 中已有可插桩 anchor 但 Linux marker 缺失、checkpoint 已删除或重命名但
Linux marker 仍残留、Linux anchor/fingerprint 已移动导致 marker 不再对应原 mapping。新增 checkpoint 默认只能先表现为
inventory/mapping/coverage 漂移或 `unmapped`，不得被静默视为 Linux 已插桩；删除 checkpoint 必须触发 stale marker 清理。
Linux marker patch 只能由已提交 `linux_checkpoint_instrumentation_plan.json` 派生，默认不得修改参考 Linux tree；工具只能在显式
`--emit-marker-patch <path>` 模式下输出可审阅 unified diff。marker 插入位置固定为对应 anchor line 的前一行并继承 anchor
缩进；同一 anchor 上的多条 marker 必须按 `checkpoint_index` 排序。已存在完全相同 marker 时不得重复插入；若 Linux tree 中存在
同一 `checkpoint_name + checkpoint_variant` 但 fingerprint 不同的 marker，或存在不属于当前 plan 的 stale marker，patch 生成必须失败。

根目录 [`../../AGENTS.md`](../../AGENTS.md) 是给支持该机制的代理使用的短入口；本目录是这些约束的正式规格位置。

## 用户态启动代码生成

生成或修改第一个用户态应用启动路径前，必须先阅读：

- [`../model/common/user_boot.spec`](../model/common/user_boot.spec)
- [`../model/payload/phase.spec`](../model/payload/phase.spec)
- [`../coding/arceos_ex.spec`](../coding/arceos_ex.spec)
- [`../coding/arceos_ex.md`](../coding/arceos_ex.md)

该路径使用已经确认的对象名和边界：`UserBootPayload`、`ElfObject`、`UserAddressSpace`、`UserStack`、`UserTrapFrame`、`SyscallException` 和 `SyscallTable`。不要重新引入 `SyscallDispatcher`、`ElfLoader`、`ExecCore` 或把 `MmStruct` 作为首轮用户态 hello 的主对象名。syscall 必须走既有 `ExceptionStream -> SyscallException` 分支；当前 whole-disk ext2 rootfs 不生成分区对象。

## CPU/CpuGroup 代码生成

生成或修改 CPU/CpuGroup 相关对象、checkpoint、trace 或测试前，必须先阅读：

- [`../charter/main.md`](../charter/main.md) 中 “CPU / CpuGroup 类型与实例”
- [`../model/SEMANTICS.md`](../model/SEMANTICS.md) 中 `SEM-CURRENT-CPU-MODEL-001`
- [`../model/boot/entry-prelude/phase.spec`](../model/boot/entry-prelude/phase.spec)
- [`../model/boot/entry-successor/phase.spec`](../model/boot/entry-successor/phase.spec)
- [`../coding/arceos_ex.spec`](../coding/arceos_ex.spec)
- [`../coding/arceos_ex.md`](../coding/arceos_ex.md)

生成结果必须使用统一 CPU 实例模型：`BootCPU` 是 logical id `0` 的 bootstrap-role CPU 实例，secondary CPU 复用同一类型。`CpuGroup` 维护 `CpuGroup.Cpu[id]` 引用索引和 possible/present/online 集合视图；集合元素是 CPU 引用，不是新的 CPU 本体对象。AP 真实进入 secondary entry 前，不得生成 live AP `CurrentCPU` 或 CPU-local 控制链。测试生成必须覆盖 index 0、CpuRef target、possible/present/online 集合视图、secondary not-online 和 logical-id/hartid 唯一性边界。
