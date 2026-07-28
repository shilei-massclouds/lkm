# Guidance 规格

本目录记录约束 AI 或其它代码生成器行为的上层指导规格。

这些规则不重新定义 model、coding、compose 或 testing 语义，而是规定生成器在使用这些规格时必须遵守的工作流。例如生成测试用例时，应先阅读原则和具体规格要求，再实现，最后检查结果是否符合原则和要求。

正式规格入口是 [`main.spec`](main.spec)。

## 仓库变更工作流

所有 AI 或代码生成器参与的仓库变更必须遵守以下共同硬约束：

- 不要猜测修复。遇到问题时，必须先通过可复现观测逐步缩小范围，再下结论和改代码。
- 规格先于实现。功能、接口、已承诺对象边界或 Linux 差分语义变更，必须先按下述某个具名工作流闭合适用规格，再改实现；代理不得自行选择实现切片并从 `impl` 开始。
- 每次修改代码后，最终回归必须在仓库根目录运行 `make test`。Focused test 只能用于中间定位，不能替代最终回归。
- 临时的跨层不一致只能存在于未提交工作区。每个适用层都必须审查，但只修改语义、映射、组合或验收责任实际受影响的层；提交始终需要用户明确授权。

### `charter-first`

`charter-first` 是默认工作流。用户或 charter 先确定设计意图，然后按以下权威顺序闭合：

```text
charter -> model -> coding -> applicable compose -> impl -> testing/tests
```

层间冲突默认按这一顺序处理。计划、roadmap、现有实现或测试结果都不能反向覆盖更高层权威。

### `model-first`

`model-first` 是用户显式触发的当轮修改顺序，不是代理可以自行选择的快捷方式，也不改变最终权威层级。每轮必须遵守：

1. 用户明确声明本轮使用 `model-first`，并提供要评审的 model 调整方案；代理不得自行选择范围或行为切片。
2. 修改前记录基线提交、工作区状态、相关行为和测试，并在仓库根目录直接运行 `make test`，确认基线通过。
3. 第一阶段只修改 model，运行适用的专项校验，然后向用户展示实际 diff 和校验结果。此阶段不得修改 charter、coding、compose、impl 或 testing/tests。
4. 等待用户明确确认 model 调整；确认前不得进入其它层。
5. 确认后，先在不改变已确认 model 含义的前提下向上闭合 charter，再按 coding、适用的 compose、impl、testing/tests 顺序向下闭合。
6. 如果 charter 无法在不改变已确认 model 含义的情况下闭合，必须停止并回到 model 决策阶段，不得静默改写 model 或 charter 意图。
7. 每层都必须审查，但只修改实际受影响的层。临时不一致只能存在于未提交工作区，不得形成中间提交或跨轮遗留。
8. 轮末运行全部适用的专项校验、focused tests、`git diff --check` 和仓库根目录直接 `make test`。全部层级闭合并验证通过后轮次才完成；提交仍需明确授权。

## Charter AI 保护锁

根目录 `charter-locks.json` 是 charter AI 保护锁的正式清单。清单中处于 `locked` 状态的文件优先于
`charter-first` 的修改顺序：层级顺序只决定规格权威，不构成隐式修改授权。普通功能修改、规格闭合或
“按 charter-first 执行”的请求都不解除锁；AI 对锁定文件只能提出建议，不得直接修改。

只有用户明确要求解除具体文件的锁定时，AI 才能通过 `python3 tools/charter_lock.py unlock PATH` 解锁。
授权仅覆盖该次任务；修改完成后，必须在任务结束前通过 `python3 tools/charter_lock.py lock PATH` 刷新
内容哈希并恢复只读锁定。中断遗留的 `unlocked` 状态是门禁失败，不自动授权下一轮 AI 继续修改。

AI 不得自行对目标执行 `chmod`、编辑清单状态或哈希、删除文件首行的可见锁定标注，或绕过
`make charter-lock-check`。清单、锁管理工具、可见标注、本 guidance 和构建门禁都属于保护机制；除非
用户明确要求调整保护机制本身，AI 不得为了绕过锁而削弱或修改它们。`enforce` 只在内容、标注和哈希
仍可信时恢复新 clone 丢失的只读位；`check` 拒绝未重新锁定、内容漂移、非法清单或仍带写位的目标。

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

该路径使用已经确认的对象名和边界：`UserBootPayload`、`ElfObject`、`UserAddressSpace`、`UserStack`、`UserTrapFrame`、`SyscallException` 和 `SyscallTable`。不要重新引入 `SyscallDispatcher`、`ElfLoader`、`ExecCore` 或把 `MmStruct` 作为首轮用户态 hello 的主对象名。syscall 必须走既有 `ExceptionType -> SyscallException` 分支；当前 whole-disk ext2 rootfs 不生成分区对象。

## CPU/CpuGroup 代码生成

生成或修改 CPU/CpuGroup 相关对象、checkpoint、trace 或测试前，必须先阅读：

- [`../charter/main.md`](../charter/main.md) 中 “CPU / CpuGroup 类型与实例”
- [`../model/SEMANTICS.md`](../model/SEMANTICS.md) 中 `SEM-CURRENT-CPU-MODEL-001`
- [`../model/phases/boot-init/preset.spec`](../model/phases/boot-init/preset.spec)
- [`../model/boot/entry-successor/phase.spec`](../model/boot/entry-successor/phase.spec)
- [`../model/phases/smp-runtime/smp-bringup/phase.spec`](../model/phases/smp-runtime/smp-bringup/phase.spec)
- [`../coding/arceos_ex.md`](../coding/arceos_ex.md)

`../coding/arceos_ex.md` 是 coding 权威索引；按其中链接进入具体 system、phase 或 object
映射。正式生命周期和对象语义直接读取上方 model 文件。

生成结果必须使用统一 CPU 实例模型：`BootCPU` 是 logical id `0` 的 bootstrap-role CPU 实例，secondary CPU 复用同一类型。`CpuGroup` 维护 `CpuGroup.cpus[id]` 引用索引和 possible/present/online 集合视图；集合元素是 CPU 引用，不是新的 CPU 本体对象。AP 真实进入 `ApEntryPreludePhase` 前，不得生成 live AP `CurrentCPU` 或 CPU-local 控制链；AP current/task/stack facts 必须来自 `ApEntryPreludePhase`、`ApSmpCallinPhase` 和 `ApOnlineIdlePhase`，不能从 possible/present membership 或 BP HSM request 直接推断。测试生成必须覆盖 index 0、CpuRef target、possible/present/online 集合视图、secondary not-online、per-AP idle task/stack、AP ack 后 online 和 logical-id/hartid 唯一性边界。
