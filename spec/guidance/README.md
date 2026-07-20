# Guidance 规格

本目录记录约束 AI 或其它代码生成器行为的上层指导规格。

这些规则不重新定义 model、coding、compose 或 testing 语义，而是规定生成器在使用这些规格时必须遵守的工作流。例如生成测试用例时，应先阅读原则和具体规格要求，再实现，最后检查结果是否符合原则和要求。

正式规格入口是 [`main.spec`](main.spec)。

## 仓库变更工作流

所有 AI 或代码生成器参与的仓库变更必须遵守三条硬约束：

- 不要猜测修复。遇到问题时，必须先通过可复现观测逐步缩小范围，再下结论和改代码。
- 先规格后实现。普通的功能、接口、已承诺对象边界或 Linux 差分语义扩展，必须先更新适用的 model、coding、testing 或 guidance 规格，再改实现。只有下文定义的“无特性重构轮”可以在受控范围内先改实现。
- 每次修改代码后，最终回归必须在仓库根目录运行 `make test`。Focused test 只能用于中间定位，不能替代最终回归。

### 无特性重构轮

无特性重构轮是一个最小验收单位：它只重组当前测试已验收的行为切片，不增加能力、不扩大接口、不引入新的系统语义或对外承诺。“通用语义”由语义是否稳定判断，不以是否已有多个实现实例为前提。

每轮必须遵守以下闭合流程：

1. 开始前阅读现有适用规格，记录基线提交、要保持的现有行为和验收测试，并确认基线根目录 `make test` 通过。
2. 允许先修改实现，但改动必须限于已声明的行为切片。轮内的实现/规格临时不一致只能存在于未提交工作区，不得形成中间提交或跨轮遗留。
3. 实现完成后以实际 diff 和可复现测试为事实输入，审查是否意外增加功能、扩大接口或改变测试覆盖外的承诺；再从系统功能、边界、信号、状态和动作中识别稳定语义。
4. 必须审查所有适用的 charter、formal model、coding 和 testing 层，并按 charter -> formal model -> coding mapping -> testing 的顺序闭合。每层都要审查，但只修改确有语义或映射变化的层。
5. 测试可随等价重构调整；删除、放宽或改写既有断言时，必须说明理由，并保留原场景的可观测验收目的。不得把弱化测试作为规格闭合手段。
6. 如果实现无法与合理规格闭合，必须停止当轮并报告冲突，不得猜测修改实现或弱化测试。
7. 轮末必须运行行为切片的 focused tests、适用的规格专项校验、`git diff --check` 和仓库根目录直接 `make test`。只有跨层一致、无未解释功能扩张且全部回归通过，轮次才完成；提交仍需当轮明确授权。

Linux checkpoint 对齐映射任务属于只读 cross-reference 阶段：代理只能消费已有 arceos_ex checkpoint inventory、读取参考
Linux 源码树并生成可审阅清单，不得把该任务扩展成 Linux 源码插桩、运行时采集、checkpoint handler 修改或行为修改。

checkpoint inventory 和 Linux mapping 的测试模式是只读漂移检测：只能在内存中重新生成 JSON/Markdown，与
`tools/out/checkpoints/` 中已提交的审阅产物比较；发现漂移应作为测试失败报告，不得在测试模式下重写产物。
Linux mapping 可以只读解析 C 函数、`SYSCALL_DEFINE*` macro 和 assembly symbol/label；遇到 arch/config
条件化 syscall ABI wrapper 时，只能映射到共同 helper 或保持 unmapped，并在 confidence/notes 中保守说明。
若参考 Linux tree 已包含由本项目生成的整行 `/* LKM_CHECKPOINT ... */` marker，mapping 解析必须在内存视图中忽略这些
marker 行，使 mapping artifact 对 clean tree 和已标注 tree 保持稳定；这不得替代显式 marker stale/mismatch 校验。
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
若同一个运行时 checkpoint 必须覆盖 Linux 同一语义的多个分支等价 call-site，mapping/plan 仍只能选择一个规范 marker anchor；
其它分支只能添加同一 checkpoint id 的 runtime record call，不得添加第二个不同 fingerprint 的 `LKM_CHECKPOINT` marker。
根目录 `make difftest` 必须先以只读方式运行本仓 artifact drift 检查和 sibling Linux marker 检查；任何 missing、stale 或
fingerprint mismatch 都必须在 paired runner 启动前失败并提示人工同步。同步流程只能由开发者显式执行：先修改并审核
mapping/语义与 Linux instrumentation，再运行 `make checkpoints`、marker check 和 difftest；验证入口不得自动重写源码、
tracked artifacts 或生成/apply marker patch。

根目录 [`../../AGENTS.md`](../../AGENTS.md) 是给支持该机制的代理使用的短入口；本目录是这些约束的正式规格位置。

## 用户态启动代码生成

生成或修改第一个用户态应用启动路径前，必须先阅读：

- [`../model/common/user_boot.spec`](../model/common/user_boot.spec)
- [`../model/payload/phase.spec`](../model/payload/phase.spec)
- [`../coding/arceos_ex.md`](../coding/arceos_ex.md)

`../coding/arceos_ex.md` 是 coding 权威索引；按其中链接进入具体 system、phase 或 object
映射。正式生命周期和对象语义直接读取上方 model 文件。

该路径使用已经确认的对象名和边界：`UserBootPayload`、`ElfObject`、`UserAddressSpace`、`UserStack`、`UserTrapFrame`、`SyscallException` 和 `SyscallTable`。不要重新引入 `SyscallDispatcher`、`ElfLoader`、`ExecCore` 或把 `MmStruct` 作为首轮用户态 hello 的主对象名。syscall 必须走既有 `ExceptionStream -> SyscallException` 分支；当前 whole-disk ext2 rootfs 不生成分区对象。

## CPU/CpuGroup 代码生成

生成或修改 CPU/CpuGroup 相关对象、checkpoint、trace 或测试前，必须先阅读：

- [`../charter/main.md`](../charter/main.md) 中 “CPU / CpuGroup 类型与实例”
- [`../model/SEMANTICS.md`](../model/SEMANTICS.md) 中 `SEM-CURRENT-CPU-MODEL-001`
- [`../model/boot/entry-prelude/phase.spec`](../model/boot/entry-prelude/phase.spec)
- [`../model/boot/entry-successor/phase.spec`](../model/boot/entry-successor/phase.spec)
- [`../model/phases/smp-runtime/smp-bringup/phase.spec`](../model/phases/smp-runtime/smp-bringup/phase.spec)
- [`../coding/arceos_ex.md`](../coding/arceos_ex.md)

`../coding/arceos_ex.md` 是 coding 权威索引；按其中链接进入具体 system、phase 或 object
映射。正式生命周期和对象语义直接读取上方 model 文件。

生成结果必须使用统一 CPU 实例模型：`BootCPU` 是 logical id `0` 的 bootstrap-role CPU 实例，secondary CPU 复用同一类型。`CpuGroup` 维护 `CpuGroup.Cpu[id]` 引用索引和 possible/present/online 集合视图；集合元素是 CPU 引用，不是新的 CPU 本体对象。AP 真实进入 `ApEntryPreludePhase` 前，不得生成 live AP `CurrentCPU` 或 CPU-local 控制链；AP current/task/stack facts 必须来自 `ApEntryPreludePhase`、`ApSmpCallinPhase` 和 `ApOnlineIdlePhase`，不能从 possible/present membership 或 BP HSM request 直接推断。测试生成必须覆盖 index 0、CpuRef target、possible/present/online 集合视图、secondary not-online、per-AP idle task/stack、AP ack 后 online 和 logical-id/hartid 唯一性边界。
