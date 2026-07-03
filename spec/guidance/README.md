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
