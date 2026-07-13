# arceos_ex 对象级实现说明

本文件从 [`arceos_ex.md`](arceos_ex.md) 拆出，保留当前对象级实现说明、命令、工程边界和专题方案；根 `arceos_ex.md` 现在作为兼容索引入口。

本文记录 `arceos_ex` 第一轮对象级实现的设计背景、命令、工程边界和专题方案。统一任务优先级和状态以
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) 为准；本文不维护独立计划表。

正式硬约束位于 [`arceos_ex.spec`](arceos_ex.spec)，并由 [`main.spec`](main.spec) 统一 include。本文承载解释、背景、参考路径和阶段性取舍；formal predicate 说明已拆入根索引链接的 phase/object/system topic 文档。不得用本文覆盖 `.spec` 中的 rule ID 或硬约束层级。

当前执行路线已经调整为：先在本仓库内直接完成对象级实现实验，源码放在
`impl/arceos_ex/`，使用 `Makefile` 编译和运行；暂时不进入 `tgoskits`、`xtask`、ArceOS crate 兼容和 feature 传递问题。

`tgoskits`/ArceOS 组件兼容属于后续 `Composition Phase`，只有对象级实现闭环后再恢复讨论。此前在
`tgoskits` 中实现过的 `arceos_ex` 仍具有参考价值，尤其是 RISC-V64 入口、链接脚本、checkpoint announce 输出、SBI/FDT
平台细节和 overlay 经验；但当前对象级实现不得直接继承其 ArceOS 组件边界、feature 传递、`axlog` 或 `ax-alloc` 接入。

## 目标边界

第一轮目标是让 `arceos_ex` 作为规格驱动的对象级内核原型运行，并完成当前模型中的引导子阶段、中断时间准备期和最终 payload 交接阶段：

- `EntryPreludePhase.Ready`
- `EntrySuccessorPhase.Ready`
- `CorePreparePhase.Ready`
- `MmCoreInitPhase.Ready`
- `SchedInitPhase.Ready`
- `InterruptPhase.Ready`
- `IrqTimeInitPhase.Ready`
- `LocalIrqEnablePhase.Ready`
- `IrqOpenPreparePhase.Ready`
- `ProcessPreparePhase.Ready`
- `UpMultitaskPhase.Ready`
- `BootInitRestInitPhase.Ready`
- `BootInitScheduleHandoffPhase.Ready`
- `BootIdleEntryPhase.Ready`
- `PreSmpInitPhase.Ready`
- `SmpRuntimePhase.Ready`
- `SmpBringupPhase.Ready`
- `RuntimeCorePhase.Ready`
- `InitcallPhase.Ready`
- `RootfsPhase.Ready`
- `FinalizePhase.Ready`
- `PayloadPhase.Online`

最小可见结果是通过独立早期输出路径打印启动 banner，进入默认 smoke payload，执行 smoke 用例并通过 SBI 关机。

实现推进分为两个逻辑阶段：

- `Object Coding Phase`：优先完成对象级语义，包括对象状态、事件推进、依赖检查、checkpoint 和必要的最小运行路径。
- `Composition Phase`：在对象级语义明确之后，再整理 crate/module 边界、公开接口、adapter、overlay workspace 和与 ArceOS 组件体系的兼容关系。

当前只推进 `Object Coding Phase`。不得因为未来 ArceOS 组件封装需要而反向改变模型对象语义。

实现分层上，Phase 对象只作为过程编排存在；非 Phase 对象原则上应在 Rust 中有明确承载，例如 struct、静态单例或启动上下文字段。

## 顶级编码原则

普通对象 API 只能服务正式模型语义、运行时语义或真实观测事实。不得为了 smoke、KUnit、checkpoint 或调试便利，在普通对象上新增
`test_*`、`*_for_smoke`、reset/snapshot/restore、fake completion、mock injection 等测试专用入口；也不得为了测试放宽
正式 API 的签名、可见性、状态前置条件或错误语义。

若测试暴露出缺少可调用边界，应先判断该能力是否属于正式对象 action/API：属于正式语义的，补 model/coding 规格并让生产路径和测试路径共享同一入口；仅用于构造测试场景的，放入 smoke fixture、checkpoint handler 或 test harness，不进入普通对象 API。checkpoint/KUnit handler 默认只读真实路径 facts；需要执行 mutating action 时，必须在 testing/coding 规格中明确 action-level probe capability、可写范围和清理边界。

MUST：处理 `fake_*`、mock、completion injection 这类历史接口时，顶层原则优先于局部章节。若接口只被 smoke/KUnit 用来构造条件，即使它模拟的是现实中会发生的外部事件，也不得留在普通对象 API 或 coding 规格的正式 API 列表中；应下沉到 smoke fixture/test harness，或转化为只读 checkpoint observer。只有生产路径也会调用、并且承载正式运行时语义的入口，才能作为普通对象 API 保留。

MUST：普通 checkpoint handler 的 `HandlerRun` 原型只能保留只读 observer 形态：
`Observe(fn(Checkpoint, &Context, &mut dyn Sink) -> CheckpointOutcome)`。不得重新加入 `Write`
变体、`&mut Context` 参数，或任何可修改 `Context` 普通对象的等价入口。可写能力只能通过受限 `Sink`
暴露。app smoke case 不得作为 checkpoint handler 注册；需要改变对象状态的测试应放在 app smoke 或明确建模的
action-level probe 中。

MUST：当前 checkpoint 机制按“观察点 + consumer”理解。默认构建不得启用重型 consumer；`PROBE=announce`
通过 `checkpoint_handler_announce` 启用 checkpoint 自声明 consumer：早期可以输出稳定单字符，post-VM 后输出
稳定 checkpoint 名称和可用上下文。`PROBE=...` 通过 `checkpoint_handler_*` 启用其它特定 observer/handler。
`PROBE=uart-irq-chain` 属于 UART/PLIC/IRQ 链路的 checkpoint observer；压力测试和并发缺陷定位可以使用它读取
结构化事实，但不得把该 probe 路径等同于普通运行路径的稳定性证明。`LOG=trace` 仅作为兼容入口映射到
`PROBE=announce`，长期应清理；`trace` 名称留给后续 Linux-like trace 机制。

MUST：Linux differential checkpoint 工作的第一个子阶段只导出本项目 checkpoint inventory。唯一事实源是
`impl/arceos_ex/src/checkpoint/mod.rs`：`Checkpoint` enum 顺序定义稳定序号和启动观测顺序基线，
`Checkpoint::name()` 定义稳定对外名称，`early_byte()` 的显式 match arm 定义可选早期单字符 announce 元数据。
导出工具只能写入 `tools/out/checkpoints/` 下的机器可读 JSON 和人工可读 Markdown；JSON 行字段固定为
`index`、`variant`、`name`、`early_byte` 和 `source_file`。本阶段不得改变 checkpoint 行为、handler、KUnit 输出、
运行时观察、内存采集机制或任何 Linux 源码。Linux 插点映射和内存采集设计只能在后续阶段消费该 inventory。

MUST：Linux differential checkpoint 工作的第二子阶段只建立 Linux checkpoint 对齐映射清单。该阶段消费
`tools/out/checkpoints/arceos_ex_checkpoints.json`，只读参考 Linux 源码树（默认 `../linux-6.12`），为每个
arceos_ex checkpoint 输出一个候选映射行，并保持原 checkpoint 顺序。输出仍只能写入
`tools/out/checkpoints/` 下的机器可读 JSON 和人工可读 Markdown；JSON 行字段固定为
`checkpoint_index`、`checkpoint_name`、`checkpoint_variant`、`linux_file`、`linux_symbol`、`linux_anchor`、
`mapping_kind`、`confidence` 和 `notes`。

映射分类只能使用 `exact`、`range` 和 `unmapped`。`exact` 表示已经定位到明确 Linux 函数或调用点，例如
`init/main.c::start_kernel()`、`mm/mm_init.c::mm_core_init()`、`kernel/sched/core.c::sched_init()`、
`init/main.c::rest_init()`、`init/main.c::kernel_init()` 或 `init/do_mounts.c::prepare_namespace()`；
`range` 表示只能定位到 Linux 启动流程中的两个可验证 anchor 之间；`unmapped` 表示当前无法可靠映射，必须记录原因，
不得猜测。该阶段不得修改 Linux tree、不得加入 instrumentation、不得改变 arceos_ex 运行时行为、不得新增
checkpoint handler、不得实现内存采集 runtime，也不得把候选映射当作后续插点已经完成的证明。
该阶段的只读源码解析可以解析 C 函数定义、`SYSCALL_DEFINE*` syscall wrapper macro 定义和 assembly symbol/label。
若参考 Linux tree 已包含本项目生成的整行 `/* LKM_CHECKPOINT ... */` marker 注释，mapping 工具必须在解析符号、anchor
和行号前从内存视图中忽略这些 marker 行，使已提交 mapping 产物不因 marker 同步阶段的注释插入而漂移；这不代表 marker
已通过校验，stale/mismatch 仍只能由显式 `--check-markers` 路径报告。
用户态 syscall 边界可以映射到对应 `SYSCALL_DEFINE*` wrapper 的核心 helper 调用；当 syscall ABI wrapper 受
arch/config 条件化影响而不能稳定代表 RISC-V64 语义时，必须映射到共同实现 helper 或保持 `unmapped`，并用
`medium` 或更低 confidence 在 notes 中说明条件化 ABI 层。用户态 exec/return/wait 边界可以使用 RISC-V64
architecture-scoped anchor，但必须保守使用 `range` 或 `medium` confidence 表达阶段性对应关系。
RISC-V64 `ret_from_exception` 的 return-to-user 类 Linux checkpoint 必须只在 `SR_SPP == 0` 的用户态返回路径记录；
不得在 supervisor/kernel return path 记录 `UserInitProcess.EnterUserMode`、`UserExec.SatpSwitched` 或
`UserExec.ReturnFrameReady`。这些 marker 若通过 assembly recorder 实现，必须放在通用寄存器恢复之前，或使用等价的
寄存器保存协议，不能在恢复 `t4`/`t5`/`t6` 后再调用会改写临时寄存器的 `LKM_RUNTIME_CHECKPOINT`。
`do_basic_setup()` 内对象级 checkpoint 可以复用同一 Linux 函数内的直接 call-site；若多个 checkpoint 共享
`do_initcalls()` 等 anchor，notes 必须分别说明 trimmed/no-op 位置保留、deferred boundary、constructor
table dispatch、initcall dispatcher/table object fact 或 `do_basic_setup()` 结束边界，不能暗示 Linux 暴露了
多个独立对象。
`RootfsPhase` / `FinalizePhase` 后半对象级 checkpoint 也可以复用 `kernel_init_freeable()`、
`prepare_namespace()` 或 `kernel_init()` 内的同一 Linux 函数或直接 call-site；若多个 checkpoint 共享
`integrity_load_keys()`、`rcu_end_inkernel_boot()` 等 anchor，notes 必须分别说明 trimmed/no-op 位置保留、
deferred boundary、路径分类范围、对象 fact 或阶段结束边界，不能暗示 Linux 暴露了多个独立对象。

MUST：Linux differential checkpoint 工作的第三子阶段只汇总已提交 Linux checkpoint mapping 的覆盖率审阅视图。该阶段消费
`tools/out/checkpoints/linux_checkpoint_mapping.json`，不得读取或修改 Linux tree，不得改变任何 checkpoint 的
`mapping_kind`、confidence 或映射语义，不得新增 instrumentation、runtime 采集或 checkpoint handler。输出仍只能写入
`tools/out/checkpoints/` 下的机器可读 JSON 和人工可读 Markdown；JSON 只能保存聚合审阅数据：总 checkpoint 数、
`exact`/`range`/`unmapped` 计数、confidence 计数、mapped Linux file 计数、unmapped checkpoint family 计数和
singleton unmapped family 总数，不得复制逐 checkpoint 明细，不得包含 timestamp。Markdown 保持紧凑，只展示 count >= 2
的 unmapped family，并汇总 singleton family 数。该工具的 `--check` 模式必须只在内存中重新生成 JSON/Markdown 并比较
tracked coverage 产物，发现漂移时报具体文件并非零退出，不得重写 stale tracked 输出。

MUST：paired checkpoint difftest 中的 `checkpoint_scope` 只表示当前 case 的 hard comparison scope，不等同于
`tools/out/checkpoints/linux_checkpoint_mapping.json` 中所有最新 `exact` checkpoint。默认 rc.local paired difftest
必须配置 exact checkpoint 覆盖审计：审计消费 tracked Linux checkpoint mapping，只要求 `required_mapping_kinds`
列出的 mapping kind（当前为 `exact`），并在 dry-run 与正式运行前校验每个 required checkpoint 都已进入
`checkpoint_scope`，或在 explicit outside-scope accounting 表中登记稳定理由。未进入 hard scope 且未登记理由的
required checkpoint 必须让 difftest 配置校验失败；已登记的缺口必须进入 manifest、summary 和 paired diff/report
的 coverage counts。默认 rc.local difftest 结论只能表述为声明的 hard scope 内一致；exact checkpoint 覆盖率由
coverage audit 单独报告，不得把部分 hard scope 误表述为全部 exact mapping 一致。

MUST：后续 Linux checkpoint 插桩同步阶段必须把 `impl/arceos_ex/src/checkpoint/mod.rs` 导出的 checkpoint inventory 作为唯一
checkpoint 源头，把 `tools/out/checkpoints/linux_checkpoint_mapping.json` 作为唯一 Linux anchor 源头。Linux 侧
instrumentation plan 和 marker 只能从这两者派生，不得手写独立 checkpoint list。插桩同步主键必须使用
`checkpoint_name` 和/或 `checkpoint_variant`；`checkpoint_index` 只能用于启动顺序审阅和输出排序，不得作为 Linux marker
身份主键，以免插入 checkpoint 时造成无意义 churn。插桩计划必须携带 Linux file/symbol/anchor 和可复核的 anchor
fingerprint；同步检查必须报告 missing marker、stale marker 和 anchor moved/fingerprint mismatch。默认只有
`exact` 映射可以直接进入插桩计划；`range` 映射必须经过人工确认或降级为区间审阅项；`unmapped` 不得生成 Linux 插点。
checkpoint 删除或重命名必须通过同步检查暴露并清理 stale marker，不能在 Linux tree 中保留孤立插桩。
Linux marker patch 生成是显式同步动作，必须读取已提交的 `linux_checkpoint_instrumentation_plan.json` 与参考 Linux tree，
只输出调用方指定路径的 unified diff，不得默认写回或重写 `../linux-6.12`。patch 只能包含 plan 中的 exact marker；range/unmapped
不会生成插入。marker 插在 anchor line 前一行并继承 anchor 缩进；同一 anchor 的多条 marker 按 `checkpoint_index`
排序。已存在完全相同 marker 时必须跳过，不得重复插入；已存在同一 `checkpoint_name + checkpoint_variant` 但 fingerprint
不同的 marker，或存在不属于当前 plan 的 stale marker 时，patch 生成必须失败且不得写出误导性 patch。`--check-markers`
是显式外部 Linux tree 校验入口，必须继续报告 missing/stale/fingerprint mismatch 并输出摘要计数；它不得进入默认
`make test-checkpoints`，除非参考 Linux tree 已成为仓库内受控产物。

MUST：观察级别至少区分 default、light、failure-only、probe-heavy 和 stress/nightly。default 级别不启用重型
checkpoint handler；light 级别只维护长期低开销 observation facts，例如对象状态、计数器、source-scoped counters
和同步边界事实；failure-only 级别只在失败路径采集结构化 diagnostic；probe-heavy 级别由 `PROBE`、`PROBE_FILE`
或后续等价开关显式开启；stress/nightly 级别负责重复执行、事件序列归档、分类和差分分析。

MUST：观察域应按稳定子系统或对象划分，包括 PLIC/IRQ-domain、UART8250/TTY、virtio-blk/block I/O、VFS/ext2、
scheduler/task、payload 和 phase boundary。对象事实由对应对象或 provider 维护，handler 只能读取和输出；不得为了
某个缺陷在 handler 内新增只对该缺陷有意义的私有事实。新增域、开关、字段或默认启用策略必须先进入 coding 规格。

结构化 failure diagnostic 是 `EventError` 的可选 payload，不是新增 checkpoint，也不得改变成功路径事件序列。
统一输出格式为
`failure_diagnostic phase=<phase> step=<step> object=<object> check=<check> first_failed=<stable-predicate-name>`，
并且必须保留原有 `error=<code> event=<event> actual=<state> expected=<state> target=<state>` 行以兼容既有 stress
分类。`phase`、`step`、`object`、`check` 和 `first_failed` 必须使用长期稳定名称，对应 model/coding 中的阶段、
对象、action 或 predicate；不能使用一次性临时日志文本。默认 `EventError` 可以不带 diagnostic，阶段接入时应按高价值失败路径逐步补齐。
failure diagnostic 的采集阶段和输出阶段必须保持分离：采集发生在失败 check 发现首个失败事实时，输出发生在错误最终报告时。

## 入口命令

当前入口统一使用仓库顶层 `Makefile`，默认内核为 `arceos_ex`。

第一轮优先支持：

```bash
make build
make build APP=smoke
make build APP=hello
make run
make run APP=smoke
make run APP=hello
make run PROBE=announce
make verify
make verify REPORT=graph
make test
make test-verify
make test-kunit
make test-smoke
make clean
```

`KERNEL ?= arceos_ex` 选择默认内核，`APP ?= smoke` 选择默认 selected payload。`build` 负责编译内核镜像；`run` 使用 QEMU/OpenSBI 运行；`run PROBE=announce`
启用 checkpoint 自声明输出。`LOG=trace` 暂时作为兼容入口等价映射到 `PROBE=announce`，但不应作为新用法继续扩展。`verify` 调用 `pyveri` 对当前启动时间轴规格做推导验证；`verify REPORT=graph`
生成带注释的 trace SVG 报告。`test` 是默认验证闭环，按顺序执行 `test-verify`、`test-kunit`
和 `test-smoke`：第一步运行正式规格 strict derive，第二步用 `impl/arceos_ex/tests/kunit.handlers`
聚合 checkpoint/KUnit handler 在 `APP=hello` 路径下验证局部对象和 action 边界，第三步运行 `APP=smoke`
验证最终 payload 可观测行为。新增 checkpoint KUnit handler 时必须加入该 handler 文件，除非它需要单独的
测试入口并在 coding 规格中记录原因。
`impl/arceos_ex/Makefile` 默认通过 `QEMU_DEVICES` 启用一个 `virtio-rng-device`，使 virtio-mmio/platform bus
和 `VirtioBus` checkpoint/KUnit 能观察到真实 `device_id == VIRTIO_ID_RNG` 的 MMIO transport 与 generic
`VirtioDevice`。需要回到裸 QEMU virt placeholder slot 场景时，可显式传入 `QEMU_DEVICES=`。

当前对象级实现已经能通过 `make run` 和 `make run PROBE=announce` 完成 `EntryPreludePhase.Ready`、
`EntrySuccessorPhase.Ready`、`CorePreparePhase.Ready`、`MmCoreInitPhase.Ready`、`SchedInitPhase.Ready` 和
`InterruptPhase.Ready`（其当前展开子阶段包括 `IrqTimeInitPhase.Ready`、`LocalIrqEnablePhase.Ready`、
`IrqOpenPreparePhase.Ready` 和 `ProcessPreparePhase.Ready`），再完成 `UpMultitaskPhase.Ready`（当前展开
`BootInitRestInitPhase.Ready`、`BootInitScheduleHandoffPhase.Ready` 和 `BootIdleEntryPhase.Ready`），
随后完成 `SmpRuntimePhase.Ready`（当前展开 `PreSmpInitPhase.Ready`、`SmpBringupPhase.Ready`、
`RuntimeCorePhase.Ready`、`InitcallPhase.Ready`、`RootfsPhase.Ready` 与 `FinalizePhase.Ready`），再通过
后续的 `PayloadPhase` 进入默认 `smoke` payload，执行 smoke 用例后通过 SBI 关机。`PayloadPhase` 是
`SmpRuntimePhase` 的后续阶段，不是其最后一个子阶段；它直接衔接 `FinalizePhase.Ready` /
`FinalizeBoundary.Ready`。
当前 `MmCoreInitPhase`、`SchedInitPhase` 和 `IrqTimeInitPhase` 都保持最小对象级语义：`PageAllocator`、
`SlubSubsystem`、`VmallocAllocator`、`Scheduler`、`Workqueue`、`Softirq`、`RcuCore`、`RiscvTimerProvider` 和
`SmpCallFunction` 只发布状态与必要事实，不提供完整运行期服务。

## ProcessPreparePhase 编码约束

`ProcessPreparePhase` 是 `InterruptPhase` 的第四个子阶段，formal model 路径为
`spec/model/phases/interrupt/process-prepare/`，目标实现路径为
`impl/arceos_ex/src/phases/interrupt/process_prepare.rs`。该阶段必须在 `IrqOpenPreparePhase.Ready`
之后运行，复用已打开的 boot CPU local IRQ、`Console.Prepared`、`SchedClock.Ready` 和 `DelayLoop.Ready`
事实，为后续 `rest_init()` 创建 `kernel_init`/`kthreadd` 准备对象基础。

本阶段的 Rust 对象承载应覆盖规格中的核心对象：`RootPidNamespace`、`AnonVmaCore`、`TaskCreationCore`、
`CredentialCore`、`VectorContext`、`UprobeCore`、`SignalCore`、`TaskFileContext`、`VmaCore`、`NsProxy`、
`UtsNamespace`、`KeyringCore` 和 `SecurityCore`。这些对象只发布 PID/task/cred/VMA/namespace/key/security
的启动期 ready/prepared 事实，不得创建 PID 1、不得创建 `kthreadd`、不得把系统推进到调度运行状态，也不得启动
workqueue worker、RCU GP kthread、完整 softirq 执行或 SMP 并发。

`TaskCreationCore.Setup` 是本阶段的主要收敛点：它必须依赖 `RootPidNamespace.Ready`、
`CredentialCore.Prepared`、`BootInitTask.Online`、CPU/SLUB 事实和异常分发事实，并在同一事件中驱动
`VectorContext.Preset` 与 `UprobeCore.Setup`。`SignalCore`、`TaskFileContext`、`VmaCore`、namespace、
keyring 和 security 对象按 formal trace 后续推进。`VfsCore.Setup` 只覆盖 Linux `vfs_caches_init()` 的最小
全局结构和 `mnt_init()->init_mount_tree()` 的初始 rootfs mount；当前 rootfs backing 必须是 ramfs。procfs、
page-cache、net namespace、`signals_init()`、真实 mount namespace 切换以及实际任务创建仍保持 deferred 或
trimmed checkpoint，不应伪装成完整运行期服务。

本阶段必须用结构化对象记录 Linux 调用点分类，而不能只散落为 checkpoint：`ProcessPrepareTrimmedPaths`
应覆盖当前 `../linux-6.12/.config` 的 RISC-V 配置下的 x86 EFI runtime switch、SCS、`lockdep_init_task()`、KGDB late init、
cpuset/cgroup/taskstats/delayacct/ACPI/KCSAN 裁剪依据；同时也要记录已启用但本轮不展开的 `net_ns_init()`、
`pagecache_init()`、`seq_file_init()`、`proc_root_init()`、`nsfs_init()`、`pidfs_init()`、以及
`vfs_caches_init()` 内 block/char device cache 初始化位置。`rcu_init_tasks_generic()` 位于 `rest_init()` 之后的
`kernel_init_freeable()`，对 `ProcessPreparePhase` 是 out-of-scope，不能提前建模为本阶段已执行。

`devfs` 不属于 `ProcessPreparePhase` 的初始 rootfs mount 行为。它必须在后续已有设备 registry 可用之后挂载到
初始 rootfs 的 `/dev` 位置；`ProcessPreparePhase` 只提供可被后续挂载消费的 VFS/root dentry/superblock 基础。

`TaskCreationCore` 还必须提供 `copy_process()`/`kernel_clone()` 的通用创建契约。后续 `rest_init()` 创建
`KernelInitTask` 或 `KthreaddTask` 时传入的 `TaskEntry` 要绑定到新任务的启动 context，并决定该任务第一次被调度后的执行入口；它不是创建完成后补写的描述性字段。当前正式规格要求 `KernelInitTask` 绑定 `TaskEntry::KernelInit`，`KthreaddTask` 绑定 `TaskEntry::Kthreadd`。

## Completion 编码约束

`Completion` 是 `spec/model/objects/main.spec` 中定义的可复用 Type process。当前对象级实现必须把它落到
`impl/arceos_ex/src/objects/completion.rs`，由 `Completion` 结构体承载普通生命周期状态、`CompletionExtState`
扩展状态、token 计数和 owned `SimpleWaitQueue`。`SimpleWaitQueue` 对应 Linux simple waitqueue/swait 的核心等待队列语义，
不是 completion 用户传入的外部引用。

`Completion.setup()` 建立 pending 状态、`done == 0` 和 owned wait queue ready；`Completion.enable()` 发布可运行期访问的
completion handle；`complete()`、`complete_all()`、`wait()`、`try_wait()` 和 `reinit()` 只能改变扩展状态和 token/waiter
事实，不能重新推进普通生命周期；`done()` 是只读观察 action。`KthreaddReadyGate` 这类 `object X: Completion`
实例必须包装并驱动该通用对象，而不是把 pending/completed/wake bookkeeping 复制成私有布尔字段。
实例 wrapper 只能承载场景事实：例如 `KthreaddReadyGate.enable()` 发布 completion handle 后，
`KthreaddReadyGate.complete()` 必须驱动通用 `Completion.complete()`；释放 PID 1 的场景结果不能由
BootInitTask 的 complete side 直接写入，必须由 `KernelInitTask` 在 wait side 观察或消费 completion token 后提交。

smoke 测试必须覆盖两类路径：一是独立 `Completion` 实例的 setup/enable/complete/token consume/reinit 流程，二是
`rest_init()` 中的 live `kthreadd_done` 实例，验证 `KthreaddReadyGate` 已驱动 `Completion.complete()`，且
`KernelInitTask` 后续通过 wait side 观察该 gate 后进入 `PreSmpInitPhase`。

## rest_init 路径编码约束

`rest_init()` 路径在 formal model 中拆成三个 owner-scoped 子阶段：
`BootInitRestInitPhase`、`BootInitScheduleHandoffPhase` 和
`BootIdleEntryPhase`，路径为 `spec/model/phases/up-multitask/rest-init/`，目标实现路径为
`impl/arceos_ex/src/phases/up_multitask/rest_init.rs`。规格和实现不得再建立
`RestInitPhase` wrapper 对象、状态或 checkpoint；`rest_init()` 只作为 Linux 控制流名称保留。

本阶段的主线对象是 `KernelInitTask`、`KthreaddTask`、`SystemState`、`KthreaddReadyGate`
和 `BootIdleRuntime`。实现必须发布 PID 1 已创建并入队、`kthreadd`
provider 已创建并绑定全局引用、`system_state == SYSTEM_SCHEDULING`、`kthreadd_done` 已 complete、
`schedule_preempt_disabled()` 已按 `BootInitTask` 视角拆为首次 scheduler handoff，
以及 post-schedule `BootIdleTask` 视角的 boot idle runtime 入口等事实。
这些事实当前仍是对象级模拟边界，不得实现真实任务栈切换、真实调度上下文切换或 idle loop。

PID 1 和 kthreadd 的创建必须通过 `TaskCreationCore` 的 entry contract 表达：`KernelInitTask` 使用
`TaskEntry::KernelInit`，其第一执行线指向 `SmpRuntimePhase`；`KthreaddTask` 使用
`TaskEntry::Kthreadd`，其第一执行线指向 kthreadd 服务循环边界。`BootInitRestInitPhase`
只能发布 kthreadd entry/provider facts，不得驱动 `KthreaddTask` 自己的服务循环子阶段；真实 kthread 请求消费、park/stop/wait 细节，以及非 idle current 下的完整 scheduler 切换留给后续模型。

PID 1 的临时 boot CPU 亲和约束不得实现为独立 `KernelInitAffinity` 对象；它必须作为
`KernelInitTask` 的 `pin_to_boot_cpu()` action 承载。该 action 只提交两类 task 属性：设置
`PF_NO_SETAFFINITY` 等价 flag，以及把 task cpumask 限制到 boot CPU。Linux 源码中的
`find_task_by_pid_ns(pid, &init_pid_ns)` 只作为实现路径说明，不能在对象级实现中变成单独生命周期对象。
包围该查找的 `rcu_read_lock()/unlock()` 读侧上下文当前保留为 deferred 建模问题；代码可以记录注释，
但不得伪造成已经完成的资源独占上下文模型。

`KernelInitTask` 和 `KthreaddTask` 的 wake-up 路径必须参考 Linux `wake_up_new_task()`：
先选择目标 runqueue，再通过 `Task` 级 `set_task_cpu` 边界更新 task 记录的 CPU id，最后进入目标
runqueue 的 enqueue/activate 边界。当前 BP 最小实现可以把 `cpu_of(selected_rq)` 固定为 boot CPU，
但代码和注释必须把这个固定值标为临时特化；未来 SMP 泛化时应从 `RunQueueRef` 解析目标 CPU，而不是继续硬编码
boot CPU。

`schedule_preempt_disabled()` 是 `BootInitTask -> BootIdleTask` 尾部和
`KernelInitTask -> SmpRuntimePhase` 执行线的分叉点。它不得实现为独立对象或单个
`Scheduler` action，而应由 `BootInitScheduleHandoffPhase` 展开
`BootIdlePreemption.enable_no_resched()` 和 `Scheduler.schedule()`，再由
`BootIdleEntryPhase` 进入 post-schedule boot idle context。不得引入
`KernelInitDispatchGate` 生命周期对象。`SmpRuntimePhase` 首个子阶段 `PreSmpInitPhase` 依赖
`KernelInitTask` release/dispatch facts 和 Scheduler 首次调度 fact，不能硬依赖任何
UP multitask 聚合 wrapper。

`Scheduler` 的生命周期不属于 rest_init 三子阶段。实现必须复用
`SchedInitPhase` 已建立的 `Scheduler.Online` 对象；`BootInitScheduleHandoffPhase`
只驱动 `Scheduler.schedule()` action 作为首次 handoff 边界，不得新增 scheduler
Preset/Setup/Ready/Online 迁移，也不得用新的 scheduler lifecycle object 表示 dispatch。
该 action 的同步结构必须对应 formal 的
`SchedulePreemptionContext` -> `ScheduleLocalInterruptContext` ->
`ScheduleRunQueueContext` 三层 `within`：分别表达 schedule-owned
preempt-disabled guard、local-irq-disabled guard 和 rq lock 独占区。

rest_init 路径涉及的锁/同步原语必须保持显式边界：PID 1 和 kthreadd 的
`wake_up_new_task()` 用各自 task pi lock 的 `WakeUp*TaskContext`，并在其中嵌套
`EnqueueSelectedRunQueueContext` 表达 `BootRunQueueLock`；`kthreadd_done` 通过
`Completion` Type process 完成；`BootIdleEntryPhase` 整体由
`BootIdleStartupContext` 覆盖，且该 context 以 `Never` 退出。不得把这些协议改成
裸 `ensures` fact 或只靠外层阶段顺序证明。

`BootIdleRuntime` 的代码结构必须和 model action 边界对齐：`setup()` 只建立
`BootIdleRuntime.Ready` 壳并确认首次调度交接已存在，不得一次性写入全部 idle 尾部事实；
phase 代码必须随后显式调用 `prepare_idle_entry()` 和 `run_idle_loop()`。`prepare_idle_entry()` 承载
`current->flags |= PF_IDLE`、`arch_cpu_idle_prepare()` 和 `cpuhp_online_idle(CPUHP_ONLINE)` 的当前抽象事实；
`run_idle_loop()` 只提交进入 idle loop，并驱动一轮代表性的 `do_idle_cycle()`。
`BootIdleEntryPhase` 主线必须直接呈现进入 `BootIdleStartupContext` ->
`BootIdleRuntime.setup()` -> `BootIdleRuntime.prepare_idle_entry()` -> `BootIdleRuntime.run_idle_loop()` ->
`BootIdleEntryPhase.Ready` checkpoint 的顺序。代码可以为每个 named action 保留小 helper，但不得再用单个
`setup_boot_idle_tail()` 把整条链隐藏起来。

`do_idle_cycle()` 必须进一步暴露 `wait_while_no_need_resched()`、`observe_need_resched()` 和
`schedule_if_need_resched()` 三个命名实现边界，对齐 model 中的
`WaitWhileNoNeedResched`、`ObserveNeedResched` 和 `ScheduleIfNeedResched`。第一段记录 boot idle task
进入抽象 idle wait 且 `need_resched` 尚未设置，polling/nohz/cpuidle/WFI 细节仍 deferred；第二段记录
本 CPU 可观察环境设置 `need_resched`，idle task 离开 wait；第三段记录 idle 专用调度请求、调度返回以及
`need_resched` 被 drain。

`schedule_if_need_resched()` 必须驱动具体的 `Scheduler.schedule_idle()` 实现边界。该 wrapper 要求当前
CPU 的 `CurrentTaskSlot` 仍指向 `BootIdleTask`，并且 `BootIdleRuntime` 已记录 need_resched observation。
它必须复用 `Scheduler.schedule()` 的 pick-next/switch-to 骨架，不能手工提交 `BootIdleTask -> BootIdleTask`
identity switch；当 runqueue 中已有 `KernelInitTask`/`KthreaddTask` 时，`schedule_idle()` 也应通过
`PickNextTask` 选择 runnable task。当前仍只实现对象级代表性一轮，不得引入真实无限
idle loop、真实 timer/IRQ wakeup 源、真实 cpuidle/WFI 路径、Linux
`do { __schedule(SM_IDLE); } while (need_resched())` 循环、`sched_submit_work()` skip 细节、真实任务栈切换
或长期 continuation 控制流。
rest_init smoke 以及 checkpoint KUnit 复用的 smoke case 必须验证 idle schedule 的关系约束：当前 BP
代表性 idle cycle 只记录一次 `schedule_idle()`，idle request/return counters 相互一致，普通
schedule/switch/current-task switch counters 包含这一次 idle pass。当前有 runnable task 时不应要求
idle identity counter 递增；`BootIdleTask -> BootIdleTask` identity 只在没有更合适 runnable task 的未来
策略分支中才可能成立。

`BootInitScheduleHandoffPhase` 可以打开“单核多任务”语义，但仍不得启动 secondary CPU；也不得把完整 workqueue/SMP 拓扑、
真实 Tasks RCU GP kthread 运行、后续 kthread request 消费提前实现。`KernelInitTask` 的下一执行线是
`SmpRuntimePhase`，其首个子阶段是 `PreSmpInitPhase`；该事实来自 `TaskEntry::KernelInit` 的创建入口绑定。`KthreaddTask` 当前只发布 entry/provider 和 deferred facts，不执行 kthreadd 服务循环；完整运行期服务能力留给后续模型。

## PreSmpInitPhase 编码约束

`PreSmpInitPhase` 是 `SmpRuntimePhase` 的第一个子阶段，formal model 路径为
`spec/model/phases/smp-runtime/pre-smp-init/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/pre_smp_init.rs`。该阶段由 `KernelInitTask` 在
`kernel_init_freeable()` 中推进，入口是 `KernelInitTask` dispatch facts 和 Scheduler 首次调度 fact 已成立；
该子阶段第一步必须由 `KernelInitTask` 通过 wait side 观察 `kthreadd_done` / `KthreaddReadyGate` 已释放，
随后才进入 `kernel_init_freeable()` 的 `gfp_allowed_mask = __GFP_BITS_MASK` 起点，出口停在 `smp_init()` 调用前。

本阶段必须覆盖 `PageAllocator.open_full_gfp_mask()`、`CpuGroup`/CPU topology 的 pre-SMP present 边界、
`Workqueue.setup()`、`VmstatCore.preset()`、`TasksRcu.setup()`、`PreSmpInitcallTable.run_early()` 和
`PreSmpInitBoundary`。它可以发布阻塞 GFP 分配可用、workqueue worker 创建边界、Tasks RCU GP thread
创建边界和 early initcall 已运行事实，但不得把 secondary CPU 标记为 online，也不得执行 `smp_init()`。
`Workqueue.setup()` 对应 Linux `workqueue_init()`，必须在 KernelInitTask 执行线上显式经过
`wq_pool_mutex` guard；规格侧用 `within WorkqueuePoolMutexContext { ... }` 表达，`impl/arceos_ex`
必须通过 `WorkqueuePoolMutex` 的 KernelInitTask owner lock/unlock 观测该边界。
本阶段入口除依赖 `KernelInitTask` wait-side release/dispatch facts 和 Scheduler 首次调度 fact 外，还必须消费
`TaskCreationCore` 建立的 entry contract：`KernelInitTask` 的 `TaskEntry::KernelInit` 指向 `SmpRuntimePhase`。

测试应覆盖 full GFP mask 已打开、secondary CPU 只处于 present/not-online、Workqueue Ready 但 SMP topology
仍 deferred、VmstatCore Prepared、TasksRcu Ready、pre-SMP initcall 已运行、`smp_init()` 未执行，以及
`PreSmpInitPhase` 的入口来自 `KernelInitTask` entry/release/dispatch 和 Scheduler 首次调度 facts，而非
`BootIdleEntryPhase.Ready` 或其它 UP multitask 聚合 wrapper。

## SmpBringupPhase 编码约束

`SmpBringupPhase` 是 `SMP Runtime Phase` 的第二个子阶段，formal model 路径为
`spec/model/phases/smp-runtime/smp-bringup/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/smp_bringup.rs`。该阶段从 `smp_init()` 开始，由
`KernelInitTask` 在 boot CPU 上发起，但对象级实现必须把 BP side 和 AP side 分开：BP 准备
secondary idle task、stack/pt_regs、CPU hotplug 同步量和 HSM boot data；AP 从区别于 BP `_start`
的 `secondary_start_sbi` 入口进入自己的启动阶段。

本阶段必须覆盖 `SecondaryIdleTaskSet.preset()`、`CpuHotplugSyncSet.preset()`、
`CpuStartProvider.setup()`、`ApEntryPreludePhase.setup()`、`ApSmpCallinPhase.setup()`、
`ApOnlineIdlePhase.setup()`、`SecondaryCpuStartupAck.setup()`、`SecondaryCpuOnlineAck.setup()` 和
`SmpBringupBoundary.setup()`。每个 secondary CPU 必须有独立的 inactive IdleTask 和 dedicated stack；
`CpuStartProvider` 发布的 Linux-like `sbi_hart_boot_data { task_ptr, stack_ptr }` 必须指向该 AP 的
idle task 与 pt_regs/栈顶边界，不得复用 BootCPU 的 idle task 或栈。`SecondaryCpuStartupAck` 与
`SecondaryCpuOnlineAck` 只是 BP wait side 的观察结果；secondary CPU online 集合只能在 AP 已经到达
online-idle 并产生 `done_up` 之后更新。

`cpuhp_threads_init()` 的 Linux 路径必须保留 `cpus_read_lock()` 与 `smpboot_threads_lock` mutex guard；
`bringup_nonboot_cpus()` / `cpu_up()` 必须保留 `cpu_add_remove_lock` 与 `cpus_write_lock()` 的 writer
guard。RISC-V `__cpu_up()` / AP `smp_callin()` 的 `cpu_running` completion，以及 generic CPUHP
`done_up` completion，必须保留 wait.lock 的 raw spinlock irqsave/irqrestore 观测。`cpu_ops_sbi.cpu_start()`
发布 secondary boot data 前后的 `smp_mb()` 顺序必须作为可观察 fact 或等价内存顺序边界保留，并通过
SBI HSM `hart_start(hartid, secondary_start_sbi, boot_data)` 启动 AP。AP 侧至少要通过 checkpoint/fact
区分 HSM request issued、boot data address/entry selected、HSM return observed、secondary entry reached、
boot data consumed、AP current/stack established、`smp_callin()` cpu_running produced、online-idle
done_up produced。AP hotplug thread `should_run` memory-barrier 配对和 CPUHP_AP_ONLINE_IDLE 之后的
callback 细节当前可作为 deferred fact，但不得把 AP entry/callin/online-idle 本身当作 BP summary。

`SmpRuntimePhase` 当前已经继续串联 `RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase` 和
`FinalizePhase`。各子阶段未展开的完整运行期服务仍保持显式 deferred 边界，不得伪装为已经实现。

测试应覆盖 BP 侧 bringup 主线已经闭合、每个 AP 有自己的 idle task 和 stack、CPU hotplug 同步量已建立、
SBI HSM start request/return 已观测、AP 三阶段 checkpoint 已产生、secondary CPU 只在 AP done_up ack 后
从 present/not-online 推进到 online、`smp_concurrency_open` 成立。smoke 还应覆盖 hotplug read/write
guard、`smpboot_threads_lock` / `cpu_add_remove_lock` mutex guard、`cpu_running` / `done_up` completion
wait-lock irqsave guard、SBI boot-data publish ordering、AP IPI/cache/TLB/local-IRQ 边界和 AP 不运行 BP
payload/syscall 的事实。

## RuntimeCorePhase 编码约束

`RuntimeCorePhase` 是 `SMP Runtime Phase` 的第三个子阶段，formal model 路径为
`spec/model/phases/smp-runtime/runtime-core/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/runtime_core.rs`。该阶段必须在 `SmpBringupPhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `sched_init_smp()` 到 `page_alloc_init_late()` 的 BP 主线。
为匹配 Linux paired diff exact anchor，`Scheduler.enable_smp()` 必须先发布 `Scheduler.SmpReady`；
`RuntimeCorePhase.Started` checkpoint 表示随后的 `workqueue_init_topology()` 区间入口，不能早于
`Scheduler.SmpReady`。

本阶段必须覆盖 `Scheduler.enable_smp()`、`Workqueue` topology action、`AsyncCore` deferred boundary、
`PadataCore` deferred boundary、`PageAllocator` late action 和 `RuntimeCoreBoundary.setup()`。
`Scheduler.enable_smp()` 必须发布 SMP sched domain ready、PID 1 boot CPU affinity 已解除、
`PF_NO_SETAFFINITY` 已清除、RT/DL SMP 后置状态 ready 和调度 granularity 已刷新事实。由于 `Scheduler`
主对象此前已经 `Online`，该动作不得重新推进 `Scheduler` 主生命周期。Linux `sched_init_smp()` 中
`sched_init_domains(cpu_active_mask)` 必须保留 `sched_domains_mutex` guard；实现侧应有可观察的
`sched_domains_mutex` ready、guard-used 和 CPU mask stable facts。

`Workqueue` 和 `PageAllocator` 在当前对象级 prototype 中保持其前序 `Ready` 主状态；RuntimeCore 通过
topology/late action facts 表达 `workqueue_init_topology()` 和 `page_alloc_init_late()` 的完成边界，避免破坏前序
phase 对 `Workqueue.Ready`、`PageAllocator.Ready` 的历史不变式。`workqueue_init_topology()` 必须复用前序
`WorkqueuePoolMutex` 和聚合 `WorkqueueStructMutex` guard：外层 `wq_pool_mutex` 保护全局 workqueues 遍历和 unbound
pool rebinding，内层 `wq->mutex` 保护每个 unbound workqueue 的 max_active 更新。

`async_init()` 和 `padata_init()` 在本轮保留 Linux 时序位置，但必须显式记录为 deferred boundary，不能静默假设可用。
Async deferred facts 应保留专用 `"async"` unbound workqueue 创建和 `min_active` 更新责任；Padata deferred facts
应保留 `CONFIG_PADATA=y` / `CONFIG_HOTPLUG_CPU=y` 下 online/dead CPU hotplug state 注册、possible-CPU work array
和 free list 责任。

`page_alloc_init_late()` 当前按 Linux 6.12 RISC-V default `.config` 记录裁剪依据：
`CONFIG_DEFERRED_STRUCT_PAGE_INIT=n`，因此 deferred init kthread、completion wait 和 `deferred_pages`
static key disable 路径裁剪；`CONFIG_PAGE_EXTENSION=n`，page extension late 裁剪；`CONFIG_SHUFFLE_PAGE_ALLOCATOR=n`，
shuffle late 路径裁剪。

测试应覆盖 `RuntimeCorePhase.Ready`、`Scheduler.smp_initialized`、PID 1 affinity 释放、Workqueue topology
facts 与 mutex guard、Async/Padata deferred facts、PageAllocator late facts 和裁剪依据，以及下一入口仍是
`do_basic_setup()`。

## InitcallPhase 编码约束

`InitcallPhase` 是 `SMP Runtime Phase` 的第三个子阶段，formal model 路径为
`spec/model/phases/smp-runtime/initcall/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/initcall.rs`。该阶段必须在 `RuntimeCorePhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `do_basic_setup()` 的对象级边界。

本阶段必须覆盖 `cpuset_init_smp()` 的 trimmed/no-op、`DriverCore` deferred boundary、
`IrqProcView` deferred boundary、`CtorTable.setup()`、`InitcallTable.preset()`、
`InitcallTable.setup()` 和 `InitcallBoundary.setup()`。当前不得展开完整驱动模型、procfs IRQ
导出或每个 initcall entry 的目标对象；
这些路径必须保留为显式 deferred 或表内属性，而不是静默假设已经可用。

`do_initcalls()` 前的审核边界固定在 `cpuset_init_smp()`、`driver_init()`、`init_irq_proc()` 和
`do_ctors()`，到 `InitcallTable.preset()` 可观察为止。当前 `../linux-6.12/.config` 中
`CONFIG_CGROUPS=n` 且 `CONFIG_CPUSETS` 未选中，所以 `cpuset_init_smp()` 通过
`include/linux/cpuset.h` 折叠为空实现；`CONFIG_CONSTRUCTORS` 未选中，所以 `do_ctors()` 的条件体不执行。
这些裁剪依据必须作为对象事实暴露。

同一区间内，`driver_init()` 不能整体写成 no-op。`devices_init()`、`buses_init()`、
`classes_init()` 和 `firmware_init()` 至少要作为前置 registry/kset/kobject 事实可见；
`platform_bus_init()` 的 `bus_register()` 必须继续暴露 `subsys_private.mutex`、
`klist_devices` 和 `klist_drivers` 初始化事实。当前暂不展开的真实启用路径必须显式 deferred：
`devtmpfs_init()` 在 `CONFIG_DEVTMPFS=y`/`CONFIG_TMPFS=y` 下包含 `req_lock` spinlock、
`setup_done` completion 和 `kdevtmpfs` kthread；`of_core_init()` 在 `CONFIG_OF=y` 下使用
`of_mutex` 建立 OF sysfs/phandle-cache 视图；`init_irq_proc()` 在 `CONFIG_PROC_FS=y`、
`CONFIG_SMP=y`、`CONFIG_GENERIC_IRQ_EFFECTIVE_AFF_MASK=y` 下建立 `/proc/irq` 和 affinity 导出。
这些 deferred 事实不能由 boot-only/system-exclusive 背景替代。

`InitcallTable` 的机制规格和具体 entry 的目标效果必须分开。model 层的
`InitcallTable.Action::Register(level, entry)` 是抽象注册接口，不规定必须依赖 LDS；本 target 的 coding
规格采用 Linux-like LDS/static section 方式实现它。也就是说，owner 对象在自己的 `preset` 中声明某个
`Action` 作为 entry，并把该 entry 注册到指定 level；coding 层将这个注册降为编译/链接期的静态 entry
descriptor，而不是运行期向 growable registry 做 push。

`InitcallEntryPrototype` 必须降为 retained static function pointer，规格 ABI 为
`fn(ContextRef) -> InitcallReturn`。`ContextRef` 是对象图入口；在当前 arceos_ex Rust target 中，它映射为
`&mut crate::context::Context`。`InitcallReturn` 表示单个 entry 的 Linux-like outcome，可由对象 action 的
`EventResult` 在 entry wrapper 内转换得到，并由 `InitcallTable.setup()` 记录。entry 不得依赖 captured
closure、heap object 或带隐藏 payload 参数的动态 callback。静态 entry descriptor 应通过 retained static
declaration 进入对应 level section，例如 Rust 侧可用 `#[used]` 加 `#[link_section]`，或使用 build-generated
table 达到等价效果。链接脚本必须用 LDS/KEEP-style 规则保留这些 section，并导出 start/end ranges 或等价的
range metadata。

`InitcallTable.preset()` 负责消费这些预链接的静态 ranges，建立或校验 level mapping、entry 列表摘要和
entry-to-operation binding；它可以构建表内 metadata，但不得调用 entry。`InitcallTable.setup()` 对应
`do_initcalls()`，只迭代 preset 已收集的 entries，并按 Linux level order 记录 level 数、所有 level
已执行、每 level 命令行 scratch 复用、参数解析、blacklist/filter 处理和 `do_one_initcall()` 运行上下文检查。
sync slot 和 rootfs slot 可以作为静态收集 slot 保留其 Linux 排序语义；是否升格为独立模型 phase 由 model
规格另行决定，coding 机制本身不强制。

`do_initcalls()` 的规格只固定跨 level 的顺序：pure、core、postcore、arch、subsys、fs、device、late。
同一 level 内的 entry 不应被 model 写成强顺序语义；当前 derivation 不证明同级 entry effect
可交换，因此必须把同级顺序无关性标为 proof/nightly deferred。实现仍可按 linker section 内实际顺序执行，
但该顺序只属于本次运行的 descriptor traversal，不是规格承诺。未来验证应通过 nightly permutation 测试扰动
同级 entry 顺序，并比较 canonical final facts；该测试不应放进普通 smoke。

`InitcallTable.setup()` 的 dispatcher 必须保持 entry-agnostic：它只能知道 level range、entry descriptor
和函数指针，不得按 entry name、owner 或具体 operation 做分支。具体副作用必须留在 entry wrapper 和 owner
对象内。每个 entry 的运行记录还必须保留 Linux `do_one_initcall()` 的可见责任：blacklist/filter 已检查、
trace start/finish 边界、返回码、preempt count 快照与失衡修复或确认不存在、disabled IRQ 修复或确认不存在，
以及 latent entropy accounting。当前 boot-only 对象级实现可以把这些责任落成显式 fact，但不得把它们当成
不存在。

测试应覆盖 `InitcallPhase.Ready`、Cpuset trimmed、DriverCore/IrqProcView deferred、CtorTable 表位置、
`InitcallTable.preset()` 的静态 range 收集事实、level/entry 摘要、entry operation binding、
`InitcallTable.all_levels_ran`、命令行 scratch 和运行上下文事实，以及下一入口仍是 `kunit_run_all_tests()` /
`RootfsPhase`。具体 entry 的目标副作用，例如 `of_platform_default_populate_init()` 填充 platform bus，应由
对应对象规格和后续 smoke 测试覆盖，不混入 initcall 机制本身。

`initcall_phase_ready(...)` 是 `InitcallPhase.Ready` 的聚合 ready-check。实现不得只在该聚合谓词失败时输出
压缩的 `EventError` 字段；必须通过统一 `failure_diagnostic` payload 报告
`phase=InitcallPhase step=checkpoint_ready object=InitcallPhase check=<stable-predicate-name>`。
`first_failed` 必须按 `initcall_phase_ready(...)` 的规范顺序报告第一个 false 条件，并使用长期稳定名称；
该名称应对应 model/coding 中的对象状态或谓词事实，而不是一次性临时日志文本。第一批 failure diagnostic 还必须覆盖
`InitcallPhase.setup_objects()` 后半段的关键 setup/action 调用，以及 `InitcallBoundary.setup()` 内部依赖检查；
下游对象若已经携带更具体的 diagnostic，外层不得覆盖它。压力测试与 nightly 流程应把这类诊断视为事件点，用于
failure-vs-success 序列对齐和首个差异点定位。

`of_platform_default_populate_init()` 是 `PlatformBus` 的 entry action。它的源对象是正式
`DeviceTree`，目标对象是 `PlatformBus`；`InitcallTable` 只负责按表执行该 entry，不拥有其目标副作用。
该 action 必须先完成 Linux-like candidate 识别，然后把每个 candidate node 构造成 `PlatformDeviceType`
实例，绑定其内嵌 `DeviceType` 的 node 引用，注册 core device，并把对应 `DeviceRef` 加入
`PlatformBusSubsysPrivate.klist_devices` 的 model 视图。

当前 arceos_ex 实现应由 `PlatformBus` 拥有 OF population 生成的 `PlatformDevice` 生命周期，并使用稳定的
platform device storage；不得把 `DeviceRef` 指向临时栈对象、扫描局部对象或固定容量 action-smoke slot。
如果 `DeviceRef` 表达为指向内嵌 `Device` 的裸指针或长期借用，普通 `Vec<PlatformDevice>` 不适合作为对象本体
backing，因为扩容会移动 inline 元素并使引用失效。第一轮可以把 `DeviceRef` 表达为稳定 index/id handle，
并通过 `PlatformBus.platform_devices` 解析回内嵌 `Device` 和外层 `PlatformDevice`；也可以使用
`Vec<Pin<Box<PlatformDevice>>>` 或 arena-style storage，让 `Vec` 只保存 owning handle 而不移动对象本体。
`PlatformBusSubsysPrivate.klist_devices` 的第一轮 backing 仍可以是 `Vec<DeviceRef>`。无论采用哪种 backing，
都必须先把 `PlatformDevice` 放入 `PlatformBus` 拥有的 storage，再把对应 `DeviceRef` 追加到
`PlatformBusSubsysPrivate.klist_devices`。该实现依赖 `DynamicContainerRuntime.Ready`，不再因为 allocator/Vec
能力缺失而使用固定数组替代正式存储。

Linux-like intrusive list 在 Rust 目标中不得直接作为对象生命周期方案。raw intrusive list 只表达 membership：
它可以链接 `DevicePrivate.knode_bus` 这类内嵌节点，但不拥有外层 `Device`/`PlatformDevice`，也不能独立证明
节点引用总是有效。未来若把 `klist_devices` backing 改为 intrusive list，必须经由
`SafeIntrusiveList` 这类更高层结构封装：内部同时维护稳定对象 storage 和 raw intrusive list，对外只暴露
insert/remove/iterate 等 safe 接口和稳定 `DeviceRef`/handle，不暴露可悬空的 raw node ref。插入顺序必须是
先进入 owner storage，再挂 raw list；删除顺序必须是先从 raw list unlink，再释放 owner storage。当前第一轮
实现不要求完成 `SafeIntrusiveList`，但语义上必须与这个未来 backing 保持一致。

`DeviceNodeId` 是 OF node 的长期稳定身份。Platform device creation 应从 candidate node 获得
`DeviceNodeId`，在 `PlatformDevice`/内嵌 `Device` 中保存该 id，并在需要读取 name/compatible/status 等属性时通过仍然
存在的 `DeviceTree` 解析临时 `DeviceNodeRef<'_>`。`DeviceNodeRef<'_>` 本身不得长期存入 `Context`。
OF platform population smoke 应覆盖 candidate count、`PlatformBus.platform_devices.len()`、
`klist_devices.len()`，并至少验证一个 `DeviceRef -> PlatformDevice -> DeviceNodeId -> DeviceTree node -> compatible`
链路。

model 层的 `DeviceType` 对应 Linux `struct device`，不是 Linux `struct device_type` 描述符；后者如需建模
应另建 `DeviceTypeDescriptor` 或 `DeviceKind`。`PlatformDeviceType` 对应 Linux `struct platform_device`，
它在 `DeviceType` 语义基础上嵌入 core device，并维护 name/id/resource 等 platform-specific 信息。coding 层
必须保留从嵌入的 `DeviceRef` 找回外层 platform device 的 container_of-like 能力，可用 Rust 宏或等价 typed
helper 表达，例如 `to_platform_device!()`。

`DeviceObject` 这类空壳只作为早期模型的对象分类标签保留，不承担 Linux driver-core 语义；可复用类型
`DeviceType`、`BusType` 和 `BusSubsysPrivate` 不应继承它来表达语义。`DeviceType.Action::SetNode` 对应 Linux
`device_set_node()`/`dev.of_node` 绑定：它只把 core device 关联到 `DeviceNodeRef`/firmware node，`compatible`
仍属于 DeviceTree node/property，后续 probe/match 必须经由该 node ref 获取 compatible。model 层的
`DeviceType.of_node: DeviceNodeRef` 表示逻辑关联；coding 层不得把借用型 `DeviceNodeRef<'dt>` 长期存入
`Context`，应存稳定 `DeviceNodeId`/node index/path handle，并在访问时通过仍然存在的 `DeviceTree` 解析出
临时 `DeviceNodeRef<'_>`。

该 action 的遍历规则参考 Linux 6.12 `drivers/of/platform.c`：
`of_platform_default_populate(NULL, ...)` 使用 `of_default_bus_match_table` 调用
`of_platform_populate()`；`root == NULL` 时 root 解析为 `/`；`of_platform_populate()` 遍历 `/`
的直接 child，并以 `strict=true` 调用 `of_platform_bus_create()`。因此每个被考虑的节点必须有
`compatible` 属性；缺失 `compatible` 的节点跳过。可用性判断应按 `of_device_is_available()`：
`status` 缺失、`status = "okay"` 或 `status = "ok"` 视为 available，其它状态跳过。节点被识别为
candidate 后，只有当该节点匹配 Linux 默认 bus 表（`simple-bus`、`simple-mfd`、`isa`，以及未来配置启用时的
`arm,amba-bus`）才递归扫描它的 children。

在当前阶段，识别出的每个 candidate 必须打印节点 name 和 compatible，便于从 KUnit/smoke 输出中核对遍历结果。
随后必须按 Linux-like 顺序执行 `of_platform_device_create_pdata()`/`of_device_alloc()` 边界，建立
`PlatformDeviceType` 实例和其内嵌 `DeviceType`，执行 `DeviceType.Action::SetNode`，绑定
`pdev.dev.bus = &platform_bus_type`，再通过 `platform_device_add()`/`device_add()`/`bus_add_device()` 的建模边界
调用 `BusType.Action::AddDevice(DeviceRef)`。model 层将 `subsys_private.klist_devices` 表达为
`DeviceRefSet`，coding 层应实现为 KList-like 可追加、可遍历的链表/数组池视图；长度不得受早期 action smoke
slot 限制。driver probe/bind 仍然 deferred。

Action checkpoint 命名应使用 `Entry` 表示 action 入口，`Exit` 表示 action 返回边界；必要的中间观测点使用
语义名称，例如 `ScanComplete`、`CandidatesIdentified` 或 `DevicesAdded`。不得用 `Called` 这类模糊名称承载
后置条件或中间完成点。当前 `of_platform_default_populate_init()` 的测试 checkpoint 是
`OfPlatformDefaultPopulate.ScanComplete`：它必须位于 candidate 识别完成并已打印 name/compatible 之后，
KUnit 对 candidate facts 的检查应挂在该点，而不是挂在入口点。
当实现推进到创建设备阶段时，还应提供 `OfPlatformDefaultPopulate.DevicesAdded` 或等价语义 checkpoint，用于在
`Exit` 前检查 `PlatformBus.platform_devices`、`klist_devices` 和 `DeviceRef -> PlatformDevice -> DeviceNodeId`
链路；`Exit` 只表示 action 返回边界。smoke 预期不得假设 initcall table 中只有本测试注册的 entry，必须只断言
本场景关心的 populate entry 已存在且执行结果正确，忽略其它无关 initcall entries。

### virtio-mmio 与 VirtioBus 编码约束

`VirtioBus` 是 `Context` 下的全局 virtio core bus 实例，与 `PlatformBus` 并列存在；它不得作为
`PlatformBus` 的 owned 子对象，也不得由 `PlatformBus` 维护 virtio-specific 状态。`PlatformBus`
的职责保持为通用 driver-core/platform-bus 机制：保存 platform device/driver 集合，执行 OF match、
调用 driver probe，并记录通用 registered/matched/probe/bound 事实。

`VirtioMmioPlatformDriver` 是普通 platform driver。它必须通过 `device_initcall` 调用
`platform_driver_register()`，再由 `PlatformBus` 的 OF match/probe 路径进入 `virtio_mmio_probe()`；
不得绕过 platform bus 直接扫描 FDT。`VirtioMmioTransportDevice` 表示单个 platform/MMIO transport
实例，来源必须是 `PlatformDevice -> Device -> DeviceNodeId -> DeviceTree node`，并通过 `Ioremap`
建立 `membase` 后读取 virtio-mmio header。

`virtio_mmio_probe()` 读到 valid、非 placeholder 的 header 后，才可以调用 `VirtioBus` 的正式
device registration API 构造 `VirtioDevice`。`VirtioDevice` 是 generic virtio core device，拥有
`device_id/vendor_id/status` 等 core 事实，并通过 transport reference 关联回
`VirtioMmioTransportDevice`；`VirtioMmioTransportDevice` 仍只表示 transport，不应和 `VirtioDevice`
合并。`device_id == 0` 的 placeholder slot 必须被拒绝或跳过，不得注册到 `VirtioBus`。

`PlatformBus` 的 public API 不得出现 `VirtioBus` 这样的 virtio-specific 业务参数，也不得把 IRQ/MM
allocator/ioremap 等 Context 字段展开成长参数列表。platform driver probe 应接收 `PlatformProbeContext`
这类由 `Context` 在单次 probe 期间临时打开的窗口：它不拥有对象，不得被 driver 保存，字段必须保持私有，
只能通过正式生产 API 暴露 OF node lookup、platform-device MMIO mapping、IRQ binding、virtio device
registration 等窄能力。普通 driver probe 不得直接接收 `&mut Context`，也不得通过全局 `context()` 反向抓取
可变全局对象。

真实 QEMU `virtio-rng-device` 链路必须通过 checkpoint/KUnit 只读 observer 观测
`ctx.virtio_bus` 中的 `VirtioDevice`、对应的 MMIO transport 和 platform bus 通用 probe 事实。
`VIRTIO_MMIO_PROBE_SUMMARY`、`probe_summary()` 或等价全局 summary 只能作为临时调试手段；建立
`VirtioBus` 后应删除 smoke 对它的依赖。app smoke 可以继续覆盖纯函数/fixture 层的 header classifier
或后续对象级 fixture/integration，但不应读取启动期临时 summary 来替代 `VirtioBus` 事实。

virtio core bus/device 注册完成后，`virtio-rng` 的真实 QEMU 路径必须继续走正式生产链路：
`VirtioBus` 上出现 generic `VirtioDevice` 后，由 `VirtioRngDriver` 匹配 `device_id == VIRTIO_ID_RNG`，
probe 生成 `VirtioRngDevice`，再按 Linux `probe_common()` 语义建立 single input queue、设置 device ready
并提交一次 pending entropy request。不得由 KUnit handler 或 app smoke 触发真实 probe/request/IRQ action；
缺少可观测边界时，应先补 model/coding 规格中的正式对象 API 或只读 checkpoint observer。

`VirtioSplitRing` / `VirtQueue` 首轮应单独放在 `objects::virtio_ring`，作为可复用 ring/queue 对象，而不是塞入
`virtio.rs`。对象级 integration 路径继续支持 single queue、split ring、direct descriptor、单个 input
buffer、`add_inbuf`、`kick` 记录和 `get_buf`；smoke 若要构造 used entry，只能放在 fixture/test harness，
不得作为 `VirtQueue` 普通 API。真实 QEMU 路径必须增加设备可见的
split-ring backing、MMIO queue setup、`QueueNotify` 和 IRQ 后 `get_buf`。当前真实路径可以使用一个页对齐
static coherent backing 作为首轮受限实现，并通过 `KernelImage::runtime_to_phys()` 计算设备可见物理地址；
通用 DMA/coherent allocator、cache maintenance、SWIOTLB/IOMMU、packed ring、indirect descriptor、event idx、
多队列、reset/remove/suspend/resume 仍显式 deferred。实现不得把 smoke 中的小队列容量固化为编译期固定数组；
对象级 ring backing 应按设备给出的 `queue_size` 建立，真实 static backing 的容量必须由 queue setup 检查
`QUEUE_NUM_MAX` 后受控选择。

`virtio-mmio` queue setup 必须按 Linux 6.12 `drivers/virtio/virtio_mmio.c` 的分支建模：version 1 legacy
设备写 `GUEST_PAGE_SIZE`、`QUEUE_SEL`、`QUEUE_NUM`、`QUEUE_ALIGN` 和 `QUEUE_PFN`；version 2 modern 设备写
`QUEUE_SEL`、`QUEUE_NUM`、`QUEUE_DESC/AVAIL/USED` 低高地址和 `QUEUE_READY`。当前 QEMU `virtio-rng-device`
可实际走 legacy 或 modern 任一路径，但代码必须从 MMIO `VERSION` 判定，不得硬编码为单一路径。`QueueNotify`
必须写 queue index；IRQ handler 必须读取 `INTERRUPT_STATUS`，写回 `INTERRUPT_ACK`，仅在 vring interrupt bit
存在时调用 ring/rng completion callback。

`VirtioRngDriver` / `VirtioRngDevice` 对象级 integration 和真实 QEMU 最小路径均放在 `objects::virtio_rng`。driver probe
输入必须是 generic `VirtioDevice`，并通过正式 `device_id == VIRTIO_ID_RNG` 匹配；不得为了 smoke 新增绕过
`VirtioDevice` / `VirtQueue` 的测试专用构造或直接写内部状态的后门。`VirtioRngDevice` 持有 single input
`VirtQueue`，正式 API 只覆盖 `probe`、`request_entropy`、`complete_entropy`、真实 IRQ completion
和 `cleanup/remove` 边界；不得暴露 `transport_complete`、`device_complete_used` 或等价 completion injection
入口作为普通对象 API。smoke 可以通过 fixture 在调用 `complete_entropy()` 前构造 used entry 条件，
但 fixture 不得被生产路径引用，也不是 KUnit/checkpoint handler 行为。首轮只维护 `data_avail`、`data_idx`、request/completion counters
和 pending request 状态。真实路径中 `request_entropy()` 必须提交真实 input buffer、kick/notify queue；
virtio-mmio IRQ handler 调用 `complete_entropy()` 后应触发 `VirtioRng.EntropyReady` checkpoint，由
checkpoint/KUnit 只读 observer 检查 `device_id == 4`、request/notify/irq/get_buf/completion 计数、`len > 0`
和 data buffer 非全零。若启动路径需要等待真实完成，只能实现为生产对象的 bounded wait/poll 边界，且不得由
KUnit handler 修改普通对象。`hwrng_register()` 不得混入 `probe_common()`：Linux-like 边界是 `probe` 建立
`VirtioRngDevice`、single queue、embedded `HwRngDevice` 和 `have_data: Completion` 子实例，并提交第一笔
pending entropy request；随后 `scan` callback 才调用 `HwRngCore::register()` 注册 embedded `HwRngDevice`。
`HwRngCore` 必须维护 registered hwrng 列表和 `current_rng`，首轮允许只有 `virtio_rng.0` 一个设备并在注册后
成为 current；命名应使用 `current`，不要用 `default`。使用者读取时必须经 `HwRngCore::read_current()` /
`HwRngDevice::read()` 转发到 `VirtioRngDevice::read_entropy()`，不得让 smoke 或普通调用直接绕过 hwrng core
访问 virtio-rng 私有 read 路径。`VirtioRngDevice` 的 `have_data` 使用已有 `Completion` Type 作为内部子实例，
不建立新的顶层 `Completion` 对象；首轮 read 只覆盖已完成数据的 nonblocking 消费和耗尽后自动重新提交
entropy request，blocking wait、random pool fill thread、misc `/dev/hwrng`、sysfs `rng_current`/quality、
freeze/restore 和完整 reset/remove 资源回收必须显式 deferred。

`/dev/hwrng` 的完整 miscdevice/file operation 仍保持 deferred；但 devfs 可以在 hwrng core 已有 current rng
之后创建只用于命名空间可发现性的 `hwrng` 设备节点。该节点不得绕过 `HwRngCore::read_current()` 引入新的读取后门。

`VirtioBlkDevice` 的生命周期必须分两层：`Setup` 只建立 matched/probed virtio-blk device、single `VirtQueue`
和 embedded `BlockDevice` shell；`Enable` 才表示真实 transport/config/feature/queue/DRIVER_OK 已完成，并提交
capacity read/nonzero、queue ready 和 driver-ok 事实。`BlockDevice` 也必须分两层：`Setup` 只建立 block device shell、
name/capacity/read callback/provider 绑定；`Enable` 才表示经 `BlockDeviceRegistry::register()` / add-disk 形状发布
到 block core，形成 major/minor、default device 和 registry lookup 可用事实。`Bio`、`BufferHead`、Ext2 和 rootfs 只能依赖
已经 `Online` 的 `BlockDevice`，不得消费仅 `Ready` 的未发布 block shell。

`Bio` / `submit_bio_wait()` / `BufferHead` 是 read-only ext2 前的下一层块 I/O 边界。当前实现必须先建立
Linux-like `Bio`、最小同步 `submit_bio_wait()` / `blk_mq_submit_bio()` 壳，以及 `BufferHead` /
`sb_bread()` / `__bread_gfp()` 路径；不得用 `BlockReadRequest` 或 `BlockIoBuffer` 代替这些 Linux 对应主对象。
`BlockDeviceRegistry::read_default()` / `read_by_devt()` 可以保留为 `submit_bio_wait()` 下面的同步 adapter，
负责 default 或 `devt` lookup 以及 provider dispatch，但高层文件系统面向的读路径不应继续把 registry read 当作公开
块层入口。app smoke 当前读取 ext2 superblock sector 时应经 `sb_bread()` 得到 `BufferHead`，再观察其 uptodate、
提交/完成、数据非零和 ext2 magic 事实；smoke 可以继续检查底层 registry read 事实，但不得绕过 bio/buffer_head
直接调用 registry read API。`BufferHead` 需要承载最多 4KiB 的 ext2 block data，因此 data payload 不得以内联
大数组形式压在调用栈或嵌套返回栈帧中；应由 `BufferHead` 拥有 heap-backed 或等价的独占动态存储，避免 block read
路径破坏 boot/runtime stack 上的 IRQ、scheduler 等全局状态。
当前 `submit_bio_wait()` 的 virtio-blk provider 可以通过两种正式完成路径收束一次同步读：一是外部中断进入
virtio-mmio IRQ handler 后消费 used ring；二是在没有睡眠等待队列/完整调度阻塞语义时，由 bounded wait 轮询
used ring 并消费当前请求的 completion。后者是同步块读运行时语义，不是 smoke/KUnit completion injection；
它不得构造 fake used entry，也不得绕过 `VirtQueue.Action::GetBuf`、status byte 校验或 pending token 校验。
每次 live read 都必须呈现完整的 submit-wait-complete 生命周期：提交一个请求、等待同一个 pending token 完成、
校验 status byte、释放 descriptor chain，然后才能向 `Bio` / `BufferHead` 返回。若进入同步读时已经存在前一轮
pending request，代码必须先主动收束该 inherited request 或返回结构化错误，不能只忙等 pending 布尔位自然消失。
initcall 阶段提交的首个 ext2 superblock probe 也遵守同一规则：它必须在 `VirtioBlkReady` 和后续
`RootfsPhase` 读路径之前完成收束，避免把首个 used-ring completion 留给 rootfs 或 user payload 阶段继承。
IRQ handler 和任务侧 polling 都可以作为 completion source，但同一个 pending token 只能有一个 consumer；实现必须保护
`VirtioBlkDevice`、`VirtQueue` 和静态读缓冲的 completion 临界区，避免 IRQ 流与 `KernelInitTask` 同时消费同一次完成。
virtqueue 发布顺序也必须保持 Linux-like 约束：descriptor/avail ring 写入在 avail idx 和 MMIO notify 之前 release
有序；读取 device used idx 后，在读取 used entry、status byte 和数据缓冲前要有 acquire 观察边界。当前 coherent
static backing 仍可保留 cache maintenance deferred，但不得省略这些顺序边界。

read-only ext2 必须继续走 `BufferHead`。实现边界对应 Linux 6.12 `ext2_fill_super()` /
`ext2_iget()` / `ext2_find_entry()` / direct-block read：读取 superblock、校验 magic/block size、读取 group
descriptor、读取 `EXT2_ROOT_INO`，遍历目录 direct blocks 中的 dirent，lookup 稳定文件，再按
inode direct blocks 把文件内容拷贝给调用者。当前泛化步骤必须支持目录 direct-block 扫描、regular
file 多 direct-block 读取，以及 read-only regular file 的首个 single-indirect block 读取；caller buffer
不足时返回 `ShortBuffer`，遇到 double/triple indirect block 需求时返回 `IndirectBlocksUnsupported`，不得静默截断或绕过 `BufferHead`。`make disk` 在 `FS_TYPE=ext2` 时应从
Alpine minirootfs tarball 构造真实 rootfs，而不是为 smoke 写入专用文件或 filler entries；smoke 应选择该
rootfs 中稳定存在的普通文件，至少覆盖一个跨 ext2 block 的 regular file，保证能观察 multi-direct-block read path。
当前临时用户态 init fixture 必须通过构造期 overlay 注入 rootfs：这里的 overlay 不是运行期 overlayfs，而是在镜像构造时把
用户态测试程序编译产物拷贝到 staging rootfs 的目标路径。目标路径存在时应替换目标路径本身，包括替换已有 symlink；不存在时创建。
用户态测试程序源文件必须放在 `impl/arceos_ex/tests/user/` 下，并通过该目录自己的 Makefile 编译；默认综合
用户态 smoke 位于 `impl/arceos_ex/tests/user/smoke/`，由 `smoke.c` 提供唯一 `main()`，再调用
`fileio.c`、`signal.c`、`stdin.c` 以及按能力拆分的子用例函数。内核主 Makefile 只负责读取 overlay 映射文件、选择默认工具链/链接方式、
指定输出目录和执行 rootfs 拷贝，不直接承载用户态编译细节。
默认 `ROOTFS_OVERLAY_MAP` 为 `tests/user/rootfs-overlay.map`；`ROOTFS_OVERLAY=none` 时必须跳过映射文件，
否则逐行读取该 map。每个非注释行声明 rootfs 目标路径、用户态测试程序名或特殊动作，以及可选的 toolchain/link mode；未写 toolchain
或 link mode 时分别使用 `ROOTFS_OVERLAY_TOOLCHAIN` 和 `ROOTFS_OVERLAY_LINK`。特殊动作 `__absent__` 表示从 staging rootfs
删除该目标路径，并且不得编译或复制用户态 fixture；它只用于构造明确的 negative/fallback 测试盘，不能用于默认 rootfs overlay。
用户态测试命名必须体现 libc
约定：`.S` 后缀表示纯汇编 fixture；`*_nolibc.c` 表示 freestanding/no-libc C fixture；省略 `nolibc` 的 `.c`
名称表示 libc-linked 测试。默认 map 当前只保留 `tests/user/rootfs-overlay.map`，并把 `user_smoke`
的 musl dynamic 产物放到 `/sbin/init`；`user_smoke` 是 `tests/user/smoke/smoke.c` 主程序和
`fileio`、`signal`、`stdin`、`fpu_mmap`、`credentials`、`process_identity`、`tty_termios`、`uts_cwd`
和 `time_random` 子用例的组合。专门验证 `/bin/sh` fallback 时，显式 absent/失败 candidate
map 应由 harness 或手工命令在临时目录生成，不作为长期 checked-in overlay map。配置仍必须允许用户态测试程序以
static 或 dynamic 方式覆盖 `/sbin/init` 或其它 rootfs 内可执行路径。用户态测试
Makefile 必须预留 GNU GCC / musl GCC 和 static / dynamic 四种组合的选择入口；dynamic libc fixture 必须以真实 `PT_INTERP` 形式进入 rootfs，并由用户启动路径读取解释器 ELF，不能把普通 libc 程序伪装成 no-libc `_start` 程序。`make disk` 默认只在磁盘文件不存在时创建；已有磁盘不得因为 overlay 配置变化而被隐式重建，强制重建必须由
`make disk FORCE=1` 或 `make disk-clean` 后再 `make disk` 明确触发。
除编译用户态测试程序的 `ROOTFS_OVERLAY_MAP` 外，构造期还允许显式 `ROOTFS_FILE_OVERLAY_DIR` 静态文件 overlay。该目录默认为空；设置后，
Makefile 必须在 Alpine tarball 解包和现有二进制 fixture overlay 之后，把目录内容按相对路径复制进 staging rootfs。它用于
OpenRC/getty/login 这类需要修改账号文件的发行版验收 fixture，不等价于运行期 overlayfs，也不得隐式启用默认 user-smoke
overlay。`ROOTFS_OVERLAY=none` 只表示跳过编译 fixture map；在没有设置 `ROOTFS_FILE_OVERLAY_DIR` 时，bare Alpine minirootfs
必须保持原始账号状态，例如当前 `root:*` 锁定。OpenRC login shell 验收若需要可登录账号，必须显式传入专用静态 account overlay；
优先使用专用测试用户；若 BusyBox login 拒绝空密码，则使用 checked-in test-only password hash，并由 host harness 在 `Password:`
marker 后发送对应测试密码。不得通过内核绕过认证。当前 focused run 已验证多阶段输入和账号 overlay 能越过认证，旧边界是
BusyBox login 认证后调用 `clone(220)` vfork，而当前单 active child slot 仍被 OpenRC/getty child 占用并返回 `ENOSYS`。
本片已规格化并实现单层 nested vfork/child-slot takeover；修后 focused run 不再出现
`clone_vfork stage=copy_user_process` / `login: vfork: Function not implemented`；后续 fd-local
`fchown(55)` / `fchmod(52)` 也已闭合，复现证据为 BusyBox login 认证后
`fchown(fd=0, uid=1000, gid=100)` 和 `fchmod(fd=0, mode=0600)` 均返回 0。当前推进片是
`socket(198)` 的最小 AF_UNIX stream fd 创建片：前一轮 diagnostic 已确认早期
`socket(AF_UNIX, SOCK_DGRAM|SOCK_CLOEXEC, 0)` 返回 `ENOSYS` 后还能继续到 `sendto(206)`，属于
tolerated/noisy syslog-like path；post-auth `fchmod` 后的直接阻断是
`socket(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC, 0)`，诊断字段为
`domain_raw=1 type_raw=0x80001 protocol_raw=0 type_base=1 sock_cloexec=1 sock_nonblock=0 af_unix_dgram_syslog_like=0`。
该 socket 返回 `ENOSYS` 后未观察到 `setgroups(159)`，login 直接打印
`login: can't set groups: Function not implemented` 并 `exit_group(1)`。该文本不得被解释为首个缺失 syscall 是
`setgroups(159)`；本地 asm-generic 头文件确认 `52=fchmod`、`55=fchown`、`159=setgroups`、`198=socket`、
`203=connect`、`206=sendto`。本片参照 Linux 6.12 `net/socket.c::__sys_socket_create()` /
`__sys_socket()` / `sock_map_fd()` 和 `net/unix/af_unix.c::unix_create()`，只接受
`AF_UNIX + SOCK_STREAM + protocol 0`，去掉 `SOCK_CLOEXEC` / `SOCK_NONBLOCK` 后验证 base type，安装一个
unconnected UnixSocket fd/OFD/backend，并把 `SOCK_CLOEXEC` 作为 fd entry close-on-exec 位、`SOCK_NONBLOCK`
作为 file status flag 保存。focused run 复跑已确认 post-auth stream `socket(198)` 返回 fd 3；新的第一阻断是
`connect(203)`，参数形态为 `fd=3, addrlen=0x18`，返回 `ENOSYS` 后 login 仍打印
`login: can't set groups: Function not implemented` 并 `exit_group(1)`。当前 connect 片已让缺失的
`/var/run/nscd/socket` pathname 返回 `ENOENT`，libc/NSS fallback 后新的直接边界是
`setgroups(159)`，形态为 `gidsetsize=1` 和用户 `gid_t *grouplist`。早期 `AF_UNIX + SOCK_DGRAM` syslog path 暂不改变，仍走 unsupported diagnostic
并返回 `ENOSYS`，避免把 tolerated noise 提前推进到 `sendto(206)` 阻断。当前 `connect(203)` 片把已分类的
AF_UNIX pathname failure 纳入 Linux-like errno 首片：复用现有 user-copy helper，最多按 Linux
`sockaddr_storage` 边界复制用户 sockaddr，只有 `copy=ok`、`AF_UNIX`、满足 Linux 6.12
`unix_validate_addr()` 长度/family 条件、非 abstract pathname、fd 当前指向 `UnixSocket0` 的 pathname
形态才继续走当前 `FsStruct.root` 下的 VFS 只查找路径。VFS 查找必须只返回目标存在性/kind，不分配 `FileRef`、不打开文件、
不修改 fd table；为覆盖 Alpine rootfs 中 `/var/run -> ../run` 的真实路径形态，该只查找路径允许沿用现有 read-only fast
symlink 跟随，并支持 `.` / `..` 目录分量且不得越过当前 `FsStruct.root`。pathname 不存在返回 `ENOENT`，pathname 已存在但当前仍没有 Unix socket/listener 模型时返回
`ECONNREFUSED`。用户 sockaddr copy fault 返回 `EFAULT`；`addrlen == 0`、`addrlen > sockaddr_storage`、
family/length 不合法、非 AF_UNIX、abstract path、非 `UnixSocket0` fd、非 pathname connect 形态和无法可靠映射的 VFS/path
错误继续走 unsupported diagnostic 并返回 `ENOSYS`。该片不得实现 `connect(203)` 成功路径、`sendto(206)`、
socketpair/bind/listen/accept、sockaddr path namespace、Unix peer/listener lookup、sk_buff queue、network namespace、
LSM、完整 credentials、TTY ownership 或 inode ownership/mode 持久化。focused 复跑已分类出早期
`addrlen=110` connect 为 `/run/utmps/.utmpd-socket` / `.wtmpd-socket` pathname AF_UNIX 噪声；认证后
`fchown/fchmod` 之后的直接阻断是 `addrlen=24`、path `/var/run/nscd/socket` 的 libc/NSS nscd pathname connect，
当前 rootfs 中返回 `ENOENT` 并让 libc/NSS fallback 暴露 `setgroups(159)`。`setgroups(159)` 首片以本地
Linux 6.12 `include/uapi/asm-generic/unistd.h::__NR_setgroups=159` 和
`kernel/groups.c::SYSCALL_DEFINE2(setgroups)` 为基准，但只覆盖当前证据：`SyscallTable` 必须路由到
`UserInitProcess` credentials 子状态，当前 effective uid 为 0 时允许 `size==0` 清空 bounded supplementary
group view，允许 `size==1` 从用户指针复制一个 32-bit `gid_t` 并保存到固定容量 1 的数组；用户 copy fault
返回 `EFAULT`，非 root 返回 `EPERM`，`size>1` 保持 unsupported diagnostic/`ENOSYS`，不得伪装为完整
`NGROUPS_MAX`。后续 focused 证据显示登录 shell 在 `setuid(1000)`、`setpgid(0,7)` 和
`TIOCSPGRP` 均成功后，`getgroups(158)` 以 `gidsetsize=32` 和用户 `gid_t *grouplist` 直接阻断；
当前 `getgroups` 首片只读取同一个固定容量 supplementary group view：`size==0` 返回当前数量，
`size < count` 返回 `EINVAL`，`size >= count` 复制已保存的 32-bit `gid_t` 条目并返回数量，用户 copy fault
返回 `EFAULT`。该片不实现完整 `group_info` 分配/排序、capabilities、user namespace、
LSM、task graph credential COW/RCU、文件权限判断、TTY ownership 或 inode ownership/mode 语义。`PROBE=user-syscall-error`
对 supported setgroups error path 应打印低噪声 `setgroups detail size=<n> list=<ptr> errno=<e> copied=<0/1> first_gid=<gid>`，
并不得改变返回值、checkpoint 顺序或默认测试输出。focused 复跑确认 `/var/run/nscd/socket` 仍返回 `ENOENT`，
`setgroups(159)` 返回 0；新的直接边界是 `setgid(144)`，参数为 `gid=100`。当前片只实现 root effective uid
下的最小 `setgid` credential drop：接受 32-bit `gid` 形态并把当前 `UserInitProcess` 的 `gid/egid/sgid/fsgid`
同步为目标 gid；非 root effective uid 仍返回 `EPERM`。focused 复跑确认越过 `setgid` 后的新直接边界是
`setuid(146)`，参数为 `uid=1000`，旧行为返回 `EPERM` 并打印 `login: setuid: Operation not permitted`。
本片按本地 Linux 6.12 `kernel/sys.c::__sys_setuid()` 的 root/CAP_SETUID 分支裁剪为 root effective uid
代理：credentials ready 且当前 `euid == 0` 时接受 32-bit `uid_t` 目标，并把当前 `UserInitProcess` 的
`uid/euid/suid/fsuid` 同步为目标 uid；非 root effective uid、credentials 未 ready 或超出 32-bit uid
仍返回当前 `EPERM` 路径。该片不得顺手实现完整 capabilities、user namespace、LSM、task graph credential
COW/RCU、完整 saved-id 权限矩阵、TTY ownership 或 inode ownership/mode 语义。OpenRC login shell focused case
是通过 `STRESS_CASES` 显式选择的 opt-in 诊断资产，不能进入默认 `make test` 硬门禁或默认 `make test-stress`
suite；越过 `setuid(1000)` 后的新第一边界必须由 focused evidence 记录后再单独规格化。focused 复跑已确认
`setuid(146, uid=1000)` 返回 0，login 继续经过 `chdir`、
`.hushlogin` missing-path 和 MOTD 输出；后续证据把登录 shell `execve(221)` 的 `EFAULT` 定位为
`fail_stage=filename_copy` / `fail_reason=filename_copy`。该 execve filename/argv copy 边界已在后续片
按逐字节 bounded C-string copy 单独规格化；修后 focused 复跑确认登录 shell 进入 `~ $`。当前 focused baseline
使用 account overlay、`PROBE=user-syscall-trace,user-syscall-error` 和 staged `/bin/ls\nexit\n`，确认下一边界为
登录 shell child continuation 内的普通 `clone(220)`：`flags=0x11`、`flags_wo_csignal=0`、
`exit_signal=17`、`newsp=0`、`current_child=1`、`child_pid=7`、`nested_parent_pid=6`、
`active_slot_reusable=0`、`completed_records_total_archived=3`、`completed_records_reaped=3`、
`next_child_pid=8`，随后返回 `ENOSYS` 并打印 `can't fork: Function not implemented`，再暴露既有
job-control `ioctl(TIOCSPGRP)` / `setpgid` 错误。该边界不是 execve filename copy，也不是已支持的
OpenRC nested vfork；本片把该形态收窄规格化为 observed child plain fork：只接受 SIGCHLD-only、
`newsp=0`、当前登录 shell child continuation 且未进入更深 child 的 clone，保存 shell parent 与
grandchild continuation facts，clone 返回 `next_child_pid`，shell 后续 `wait4(-1, status, allowed_options, NULL)`
才切入 grandchild；仍不创建第二个 runnable `UserChild` task ref，不扩大 task graph、通用普通 fork、
job-control 或完整 COW 语义，不属于 setuid 片。
QEMU 启动命令行默认由 `QEMU_APPEND ?= earlycon=sbi` 提供，`run` 目标必须把它原样传给 QEMU
`-append "$(QEMU_APPEND)"`；用户可以通过 make 命令行覆盖，例如
`QEMU_APPEND='earlycon=sbi init=/bin/ls'`。该内核命令行选择的是 Linux-like init 路径，不是 rootfs
overlay 机制。即使支持 `init=`，默认 overlay 仍必须保留为稳定用户态 smoke/probe fixture 注入手段；
`init=` 只决定 `kernel_init()` 的 requested init 分支或默认 fallback 选择哪个 rootfs path 作为 PID1。
Ext2 对象生命周期划分为 `Ext2Driver`、`Ext2Volume` 和 `Ext2FileSystem`：`Ext2Driver` 取代旧的
`Ext2Type`，承载 Linux `file_system_type` 以及当前建模的 super/inode/file operation set；
`Ext2Volume` 表示默认块设备上按 ext2 规范组织的 on-disk volume，由 `Preset` 经 `BufferHead` 检查确认，
不存在或格式不匹配是普通非致命结果，不应 panic 或终止内核；`Ext2FileSystem` 表示一次 mount 后的内存中文件系统实例，
`Preset` 绑定 `Ext2Driver` 和 `Ext2Volume`，`Setup` 解析元信息并建立 root dentry/inode 入口事实，
`Enable` 必须通过 `VfsCore` 把只读 ext2 实例挂接到上级 VFS 目录节点。当前只要求最小 read-only mount/read：
VFS 负责 mount point、dentry、open/read 入口和观测事实，实际 lookup/read 后端仍调度到 `Ext2FileSystem`
的 `BufferHead` + direct-block 路径。
superblock 字段、group descriptor、inode record、dirent 和 file-read result 首轮只是 `Ext2FileSystem`
下的结构化记录/事实，不独立建生命周期；后续若实现 inode cache、refcount、evict 或完整 path walk/page cache，
再讨论是否把 inode 等提升为对象。
当前 4K Buffer 步骤必须把 `BufferHead` 和 virtio-blk read buffer 扩到至少 4KiB，并让 `Ext2Volume` /
`Ext2FileSystem` 支持 ext2 `block_size` 为 1024、2048 和 4096。superblock 仍按 ext2 规则从 byte offset 1024 读取；解析出实际
block size 后，group descriptor、inode table、目录和文件数据读取必须使用真实 filesystem block number 到
sector 的映射。`make disk` 默认不应再强制 `mkfs.ext2 -b 1024`；如需覆盖 block size，应通过显式参数表达。
本轮把 ext2 接入一个显式 VFS mount/read 路径，并支持最小 absolute pathname walk/read：从 current root 出发，
逐级 lookup 直接子节点，遇到 mount point 时 crossing 到 mounted root，再完成 open/read。Symlink follow 的规格必须按
Linux 6.12 `fs/namei.c::link_path_walk()` / `walk_component()` / `pick_link()` 骨架表达：识别 symlink 后取得 target，
绝对 target 从 `FsStruct.root` 重新开始，相对 target 从 symlink 所在目录重新开始，并把 symlink 后剩余路径组件接回
继续解析；follow 次数使用 Linux `include/linux/namei.h::MAXSYMLINKS == 40` 的预算，超限返回 loop 类错误。当前实现首片只承诺
read-only ext2 的 fast symlink：以 inode mode `S_IFLNK` 为准识别 symlink，target 取 ext2 inode raw `i_block` inline
区域并按 inode size 截断。`readlinkat` 是单独的 no-follow final symlink 操作，可复用同一 fast-symlink target
来源，但必须保留 Linux 6.12 的 copy/truncate 语义；slow symlink 的 page/block-backed `page_get_link` 路径、
`O_NOFOLLOW`、`AT_SYMLINK_NOFOLLOW`、magic link、RCU walk、权限/LSM、完整 mount namespace、`..` 语义和完整 errno 仍保持
trimmed/deferred。相对路径、cwd、权限、fd table、rootfs 切换、page cache/folio、间接块、xattr、quota、block allocation、
写路径、remount 或错误恢复必须保持 deferred。ext2 smoke 必须通过 VFS path read 读取稳定测试文件，同时仍不得通过 VirtioBlkDevice 私有入口绕过
`sb_bread()`。

第一批长期观察 checkpoint 必须来自 model/coding 契约，而不是针对单个缺陷临时打印。Payload 读取用户态镜像时必须记录
`PayloadImageReadStart` / `PayloadImageReadComplete`，并保留结构化的 `PayloadImageReadFailed` 分类；VFS path
read 必须记录 `VfsPathReadStart` / `VfsPathReadResolved`，并保留 `VfsPathReadFailed` 分类；Ext2 必须记录
`Ext2LookupStart` / `Ext2LookupFound` / `Ext2LookupFailed` 以及
`Ext2FileReadStart` / `Ext2FileReadBlockRequest` / `Ext2FileReadComplete` / `Ext2FileReadFailed`。
块 I/O 任务侧必须记录 `BlockIoTaskRequestSubmitted`、`BlockIoTaskWaitBegin`、`BlockIoTaskWaitEnd` 和
`BlockIoTaskWaitTimeout`；virtio-blk 完成侧必须记录 `VirtioBlkLiveReadSubmitted`、`VirtioBlkLiveReadCompleted`、
`VirtioBlkLiveReadFailed`、`BlockIoIrqCompletionBegin`、`BlockIoIrqCompletionEnd`、
`BlockIoIrqCompletionFailed`，并在同步轮询完成时记录 `BlockIoTaskPollCompletionObserved`。这些事件需要携带足以关联同一次
block read 的请求身份，并能区分 completion source 是 IRQ 还是 task-side poll，供 nightly/stress 纵向对比和后续
Linux-like 横向对比使用。

`devfs` 的首轮实现属于 `InitcallPhase` 收敛边界：它必须在 `VfsCore` 初始 rootfs mount 已存在、`HwRngCore`
和 `BlockDeviceRegistry` 已 Ready、且 virtio-rng/virtio-blk live driver 已完成注册之后挂载 `/dev`，再创建
当前 hwrng 和默认 block device 对应的设备节点。smoke 验证只能观察 `/dev` 节点、hwrng current 绑定和 block
default/devt 绑定；本步不得为了测试新增对象 API，也不要求通过 VFS file path 读写设备。device file ops、uevent、
sysfs、权限模型、devtmpfs kernel thread 和用户态设备管理仍保持 deferred。

console/earlycon handoff 的实现必须保持 Linux-like `register_console()` 边界，但第一轮仍只要求对象级可观测事实。
正式 `DeviceTree` 应解析 `/chosen/stdout-path`，缺失时兼容 `linux,stdout-path`；属性值中冒号前是节点路径，
冒号后是 console options。节点路径和 options 必须分开保存，options 保留给后续 console setup，不得混入节点路径匹配。
解析结果应保存为稳定 `DeviceNodeId`/path handle，并在需要访问属性时通过仍然存在的 `DeviceTree` 解析临时
`DeviceNodeRef<'_>`。若 `stdout-path` 缺失、解析失败或指向非 ns16550a 节点，当前最小实现不得猜测其它串口为
preferred console。

`ns16550a` probe 必须从 `PlatformDevice -> Device -> DeviceNodeId -> DeviceTree node` 路径解析 UART 资源，
至少记录 MMIO resource、`reg-shift`、`reg-io-width`、`clock-frequency` 或等价默认 clock、line/index 分配和
`serial8250_register_8250_port()` 风格返回事实。MMIO 映射必须继续经由 `Ioremap -> VmallocAllocator`
路径；probe 不得绕过该路径直接把 `mapbase` 当作 `membase`。probe 只有在被绑定设备匹配 `stdout-path`
时，才能把该 port 提升为 `Serial8250Console` 并触发 handoff。非 stdout-path 的 ns16550a 设备只能注册普通 port
或记录 non-console port 事实，不得设置 consdev、不得切换 printk route、不得注销 boot console。

`EarlyCon`、`BootConsole`、`Serial8250Console` 和 `ConsoleRegistry` 不得合并成一个实现对象。`EarlyCon`
只表示早期 SBI 输出 backend 的生命周期和 direct backend access contract；它不是 printk registry entry。
`BootConsole` 表示 printk registry 中包装 EarlyCon 的 `CON_BOOT` console entry，携带 boot/printbuffer 语义；
`BootConsole.Enable` 之后 printk route 必须指向 boot console。真实 `Serial8250Console` 表示由
`Uart8250Port` 注册出来的 real console entry，它只能经 `ConsoleRegistry` 的 `register_console()` 策略成为
active printk route。

`ConsoleRegistry` / `ConsoleHandoff` 的实现应集中承载在 printk console registry 或等价对象中，不得把 handoff
状态散落成 driver 私有布尔值后再伪造 ready 事实。drivers 只能请求注册 console，不能拥有 preferred-console
选择、route 切换、boot pending cursor 移交、legacy earlycon drain 阻断或 `keep_bootcon` 决策。真实
`Serial8250Console` 经 `register_console()` 成为 consdev 后，必须同时提交这些事实：real console registered、
preferred-from-stdout、consdev、write backend ready、printk route 切到 serial8250、handoff complete。默认
`keep_bootcon == false` 时，handoff 必须注销 boot console 或至少把 boot console 标为 offline，并记录 boot
console removed/unregistered 事实；`keep_bootcon == true` 时，boot console 必须保持 registered/online，但
printk route 仍切到 serial8250，handoff 仍视为完成。
参考 Linux 6.12 `kernel/printk/printk.c::register_console()` 的交接边界，真实 serial console 成为
`CON_CONSDEV` 后应显式输出 `Serial8250Console.Online` 或等价 trace；默认非 `keep_bootcon` 分支注销
`CON_BOOT` boot console 时，应显式输出 `BootConsole.Offline` 或等价 trace。trace/SVG 必须能展示
real console 上线和 boot console 下线的顺序；`keep_bootcon` 分支必须展示 serial console online 但 boot console
retained，而不是误报 offline。

`register_serial8250_console(preferred_from_stdout)` 或等价 API 必须模拟 `register_console()` 的策略分支：
`preferred_from_stdout == false` 的 dummy/non-match console 注册请求不得改变 serial console facts、不得设置
consdev、不得完成 handoff，也不得影响 boot console route。dummy/non-match smoke 必须直接在 smoke 文件中定义
临时 console 请求对象，不得为了测试去修改 DeviceTree 或平台设备拓扑。重复注册同一个 preferred serial console
应保持幂等，不得重复分配 port、重复增加 registry entry 或重复执行 boot console unregister。

handoff 后 `printk::write_str()` 或等价输出入口必须经 `ConsoleRegistry` route 分发到 `Serial8250Console`，
不得继续停留在只写 `PrintkBuffer` 的事实层。在 PLIC/UART 外部中断链打开前，serial8250 后端必须建模为无中断 polling write：
每个待发送字符按 Linux `uart_console_write()` 语义处理换行 CRLF，发送前观察 LSR/THRE ready 条件，然后写 THR/TX；
寄存器地址必须从 `Uart8250Port.membase` 加 `reg_shift` 派生，并尊重 `reg_io_width`。代码只有在
`VmallocAllocator.map_page_range()` 已把 vmap VA/PA 映射安装进当前 swapper 页表并记录 runtime mapping ready
后，才能对 `membase` 派生出的 LSR/THR 地址做 `read_volatile`/`write_volatile`；不得直接访问 `mapbase`，
也不得用 SBI 路径伪装真实 UART 写。PLIC source gate 和 root external input gate 尚未打开前，不得声明
interrupt-driven console output ready，只能记录 IRQ 输出路径 deferred。
在 `UartInterruptChainProbe` 已证明 `UART -> PLIC -> IRQ core -> UART handler` 一轮 claim/complete/zero-claim
闭合之后，必须通过独立的 `Serial8250ConsoleIrqTxProbe` 或等价生产边界把 runtime TX 路径切换为 interrupt-driven：
`printk` 前端只提交输出记录，serial8250 后端把 CRLF 后的字节放入 TX queue，在保存并关闭本地中断的临界区内 kick
`UART_IER_THRI`，随后由真实 UART THRE interrupt 经 PLIC claim loop 和 IRQ core dispatch 调用 UART handler；
handler 必须按 Linux `serial8250_handle_irq()` / `serial8250_tx_chars()` 形状在 THRE 条件下 drain TX queue，
队列清空后清掉 THRI，不能由 smoke/KUnit 直接调用 handler 或手动 drain 后端。

当前 kernel `printk` console TX 基线 contract 已收口为：app/payload/smoke 只能通过 `printk` 或等价公开输出前端提交
输出，不得直接 import 或调用 `EarlyCon`、`BootConsole`、`Serial8250Console`、`Serial8250RuntimePort` 等 backend，
也不得新增公开 `printk::flush()` 来推进 drain；`Serial8250Console` 只负责把 CRLF 展开后的 printk bytes 入队并
kick THRI，实际 byte drain 只能发生在真实 `root INTC -> PLIC claim -> IRQ core -> Serial8250RuntimePort.HandleInterrupt ->
TransmitChars` 路径中；KUnit/checkpoint handler 只能读取 fact/counter/trace 并写入受限 sink，不得触发 IRQ、claim/complete、
调用 UART handler、修改 TX queue 或推进 drain。`Serial8250ConsoleTxQuiesceProbe` ready 后，任意后续实现变更都必须保持
queue empty、THRI stopped、bounded idle 期间无新增 PLIC claim、IRQ dispatch、UART handler drain、printk kick/drain 或
ordinary `TtyXmitFifo` mutation；若要改变 queue full、reentrant printk 或 irq-disabled/hardirq 上下文下的行为，必须先补
明确规格和独立 probe，不得借现有 smoke/KUnit 隐式改变这条基线。

下一轮 serial8250 runtime RX/TTY/FIFO 建模必须先固定对象职责和协作关系，再定义 transition/action。对象边界如下：
`Uart8250Port` 继续只表示 platform probe 得到的 8250 资源实例，拥有 MMIO resource、`mapbase/membase`、
`reg_shift/reg_io_width`、clock、line 和 logical IRQ 绑定；它不得承担 console handoff、TTY buffering、PLIC
claim/complete 或用户态 TTY 语义。`Serial8250RuntimePort` 表示覆盖在 `Uart8250Port` 之上的 8250 运行期行为层，
负责 IER/IIR/LSR 状态、RDI/RLSI/THRI enable/disable、port lock/irqsave 以及 RX/TX handler 分发形状；
它不得解析设备树、执行 irqdomain translate、拥有 console registry 策略或直接 claim/complete PLIC。
`Serial8250Console` 继续只表示 printk registry 中的 real console entry，负责把 printk console 输出提交给
serial8250 runtime TX 路径；它不是 TTY runtime，也不拥有 RX。`TtyPort` 表示最小 `uart_state/tty_port`
容器，绑定 UART line 并连接 `TtyFlipBuffer` 与 `TtyXmitFifo`；它不得访问 UART MMIO、分发 IRQ 或拥有 console
registry 策略。`TtyFlipBuffer` 是 RX interrupt 到 TTY 层之间的 staging buffer，当前首片由
`TtyFlipBuffer.Push` 把真实 UART RX 产生的 byte 发布为 bounded raw ready-data，供已有 fd0 char-device
`read/ppoll` 立即路径消费；这仍不等价于完整 N_TTY read。`TtyXmitFifo` 表示普通 TTY write 的 TX FIFO，必须与当前 printk console
TX queue 区分开；普通 TTY write 接入可以后置。

上述对象的协作关系按三条链描述。设备构成链是
`DeviceTree ns16550a node -> PlatformDevice -> Uart8250Port -> Serial8250RuntimePort`。console 输出协作链是
`printk -> ConsoleRegistry -> Serial8250Console -> Serial8250RuntimePort.StartTx -> UART THRI interrupt ->
IrqAction -> Serial8250RuntimePort.HandleInterrupt -> TransmitChars`。RX 输入协作链是
`UART RX byte -> PLIC source -> root INTC -> PLIC claim -> IrqAction.Dispatch -> Serial8250RuntimePort.HandleInterrupt ->
ReceiveChars -> TtyFlipBuffer.Insert/Push`。下一步定义 transition/action 时必须以这些边界为前提；测试对象只能观察这些
生产链路的结果，不能代替链路中的 IRQ、handler 或 backend 调用。

serial8250 runtime RX/TTY/FIFO 的 transition/action 边界应按以下接口收敛。`TtyPort.Transition::Setup` 建立最小
`uart_state/tty_port` 容器，驱动 `TtyFlipBuffer.Transition::Setup` 和 `TtyXmitFifo.Transition::Setup`，并依赖
`TtyLineDisciplineRegistry.Prepared`，但不访问 UART MMIO、不注册 IRQ handler、不改变 console route。
`TtyPort.Transition::Enable` 对应最小 `uart_startup()` 边界，只把 TTY port 标记 initialized，并允许后续
`Serial8250RuntimePort.Transition::Enable` 打开 RX runtime；完整 open/close、termios、hangup 和用户态 file 语义后续展开。
`Serial8250RuntimePort.Transition::Setup` 依赖 `Uart8250Port.Ready`、`IrqAction.Ready` 和 `TtyPort.Ready`，只建立
IER/IIR/LSR、port lock/irqsave、TTY buffer 绑定和 handler shape；`Serial8250RuntimePort.Transition::Enable` 依赖
`UartExternalIrqEnable.Ready` 和 `TtyPort.Online`，打开 RDI/RLSI，并保持 THRI 为 demand-driven。

运行期 action 只能经生产链路调用。`Serial8250RuntimePort.Action::HandleInterrupt(cause)` 必须要求 hardirq
context 和 port lock/irqsave，读取 IIR/LSR，先处理 RX，再检查 modem status，再在 `LSR_THRE && IER_THRI`
条件下处理 TX；它不得 claim/complete PLIC，也不得由 KUnit/smoke 直接调用。`ReceiveChars(byte)` 消费
LSR_DR/BI 代表的 RX byte，使用 bounded drain 策略，驱动 `TtyFlipBuffer.InsertChar(byte)` 和
`TtyFlipBuffer.Push(record)`；`Push` 可以发布 bounded raw ready-data，但不进入 canonical line discipline、
wait queue、job control、hangup 或 signal restart。
`TransmitChars` 使用 tx_loadsz/FIFO 策略并在队列空时停止 THRI；`StartTx`/`StopTx` 分别只负责设置/清除 THRI。
`TtyXmitFifo.Enqueue/DequeueForTx` 描述普通 TTY write FIFO 接口，必须与当前 printk console TX queue 分离；
完整 line discipline/file write 入口可以后置，但受控 ordinary TTY write probe 可以先接入 runtime TX/THRI。

ordinary TTY TX FIFO 的首轮实现只能建立 `TtyXmitFifo` 自身的 enqueue/dequeue 可观测边界。实现必须通过独立
`TtyXmitFifoProbe` 或等价生产边界提交固定探针 byte，并验证 enqueue 后能 dequeue 同一 byte、dequeue 后 FIFO
为空、无 overflow/underflow；该 probe 不得调用 `printk` 前端，不得写 UART THR，不得设置 `UART_IER_THRI`，
不得改变 serial8250 console 的 printk TX queue/kick/drain counters。KUnit 只能读取这些结果并通过受限 sink
输出诊断，不能直接 enqueue/dequeue 或触发 UART handler。

ordinary TTY write 接入 runtime TX/THRI 的首轮必须通过独立 `TtyWriteRuntimeTxProbe` 或等价生产边界执行：
probe 先向 `TtyXmitFifo` enqueue 固定 byte，再经 `Serial8250RuntimePort.StartTx` 设置 `UART_IER_THRI`，随后等待真实
`root INTC -> PLIC claim -> IrqAction -> Serial8250RuntimePort.HandleInterrupt -> TransmitChars -> TtyXmitFifo.DequeueForTx`
链路 drain FIFO 并清掉 THRI。该路径可以复用 8250 THR/MMIO 写和 PLIC/IRQ dispatch，但不得复用 printk console TX queue，
不得增加 printk write call，不得让 KUnit/smoke 直接调用 handler 或手动 drain FIFO；KUnit 只能观察 enqueue/dequeue
计数、THRI kick、handler drain、PLIC claim/complete/zero-claim loop exit、queue empty 和 last byte。

ordinary TTY write 的第二轮批量补强必须通过独立 `TtyWriteBatchRuntimeTxProbe` 或等价生产边界执行，并且只能在
`TtyWriteRuntimeTxProbe` 单 byte 闭环 Ready 之后运行。该 probe 使用固定小批量，批量长度必须小于当前
`TtyXmitFifo` 容量和 runtime TX drain/tx_loadsz 预算；它可以一次 enqueue 多个 ordinary TTY byte，然后只经
`Serial8250RuntimePort.StartTx` 打开 THRI，让真实 PLIC/IRQ/runtime handler 的 `TransmitChars` drain 批量 FIFO。
验收至少要求 enqueue/dequeue/runtime drain 的增量匹配批量长度、queue empty、last byte 匹配、无 overflow/underflow、
local irq guard 可观察、PLIC claim/complete/zero-claim loop exit 闭合，并继续验证 printk console TX queue/write/kick/drain
计数不变。该 probe 仍不引入公开 `printk::flush()`、完整 `tty_write()`/line discipline/file write API，也不得让
KUnit/smoke 直接调用 handler 或手动 drain FIFO。

首轮 RX 验证应采用 8250 loopback probe，而不是修改设备树或让 KUnit 人工制造中断。`Serial8250RxLoopbackProbe`
或等价生产边界负责保存 MCR、设置 loopback、由 smoke/probe 路径提供一个 TX 字符、写入 UART TX，让硬件回送成
RX 并触发真实 RDI/RLSI interrupt；随后必须走真实 `UART -> PLIC -> root INTC -> IrqAction dispatch ->
Serial8250RuntimePort.HandleInterrupt -> ReceiveChars -> TtyFlipBuffer.Push` 链路，probe 结束后恢复 MCR。
首轮单字符闭环已经完成；有限批量补强必须通过独立 `Serial8250RxBatchLoopbackProbe` 或等价生产边界执行，
使用固定小批量且批量长度必须小于 `Serial8250RuntimePort` 的 RX drain limit。该 probe 可以写入 UART TX 产生
loopback RX，但必须仍走真实 PLIC/IRQ/runtime handler 路径，并观察一次 bounded `ReceiveChars` 把批量字符 insert 后
push 到 `TtyFlipBuffer`；验收至少要求 batch insert 数、last pushed len、last byte、无 overflow、claim/complete 和
zero-claim loop exit 闭合。KUnit 在这条路径中只能作为 observer/checker：它可以读取 counters/facts/trace 并向
受限 sink 输出诊断，但不得写 TX 字符、不得设置 loopback、不得调用 handler、不得 claim/complete PLIC，也不得改
pending/enable 状态。smoke 可以作为生产侧 stimulus，但也不能直接调用 backend handler；它只能通过公开/受控的
前端或 runtime probe 入口提交字符。
这里的 zero-claim loop exit 闭合要求表示同一轮真实 PLIC claim loop 已经观察到 zero claim 和随后 loop exit 两个边界；
如果实现把 zero claim 与 loop exit 记录为两个独立计数，probe 不得把某个并发采样点上的两个计数不相等直接判定为失败。
`Serial8250RxLoopbackProbe.setup` 和 `Serial8250RxBatchLoopbackProbe.setup` 失败时必须通过统一
`failure_diagnostic` payload 报告内部首个失败事实。这些字段是长期 RX/PLIC/IRQ/flip-buffer 观察事实，不是为某个
缺陷临时添加的日志。单字符 probe 的输出形态为
`phase=InitcallPhase step=setup_objects.serial8250_rx_loopback_probe.setup object=Serial8250RxLoopbackProbe check=serial8250_rx_loopback_probe.setup first_failed=<stable-predicate-name>`；
批量 probe 的输出形态为
`phase=InitcallPhase step=setup_objects.serial8250_rx_batch_loopback_probe.setup object=Serial8250RxBatchLoopbackProbe check=serial8250_rx_batch_loopback_probe.setup first_failed=<stable-predicate-name>`。
`first_failed` 必须按实现判定顺序使用长期稳定名称。第一批至少覆盖通用前置事实：probe lifecycle base、前序 RX
probe ready、RX runtime enabled/deferred、PLIC/domain/registry ready、logical IRQ valid、UART source mapping
present/matches/source gate open、registered handler present、flip buffer empty、fixed batch nonempty、batch length within
drain limit；刺激事实：enable runtime RX、trigger single loopback byte、trigger bounded loopback batch；等待事实：
single/batch RX request observed、PLIC claim observed、PLIC IRQ-domain dispatch observed、IRQ handler registry dispatch
observed、runtime handler observed/RX handled、PLIC complete observed、zero-claim observed、claim-loop exit observed、
source-scoped PLIC claim/dispatch/complete delta，以及 single/batch RX loopback wait closed；观测事实：stimulus committed、
PLIC claim observed、IRQ dispatch observed、runtime handler received RX、flip buffer pushed、PLIC complete observed、
zero-claim loop exit observed、source claim/complete delta matched、IRQ cycle closed、bounded drain observed、
batch count matched、last byte matched 和 no overflow observed。若等待失败涉及中断链路，diagnostic 必须保留 PLIC/IRQ-domain/IRQ-registry 事实，不得把问题
预先归因到 UART 侧。
loopback probe 在 initcall 阶段产生的 RX byte 只属于链路诊断事实；`UserBootPayload` 进入用户态前必须通过
`FilesStruct.Action::ClearStdinReadyData -> TtyFlipBuffer.Action::ClearReadyData` 丢弃这些 ready-data，随后只有选中
受控 `user_smoke` fixture 时才能准备固定 stdin ready-data。发行版 `init=/bin/sh`、`init=/bin/ls` 或默认 fallback
路径不得继承 loopback probe byte，也不得注入 fixture。
为避免 handoff 后重复输出，`ConsoleRegistry` 必须区分 printk 记录保存和 legacy boot-console drain cursor。
注册 preferred serial8250 console 时，应先把 boot console pending records 按 boot console 路径 flush 并推进 cursor；
serial8250 route 成功写出的记录必须标记为已交付，不得再被 `earlycon::drain_printk()` 经 SBI 重放。默认
handoff 完成后，EarlyCon 后端必须进入 offline/disabled 状态；后续直接推进 earlycon transition/action 或调用
`earlycon::drain_printk()` 属于非法 backend 访问，必须触发 panic 或等价 contract violation，不能静默 no-op。
EarlyCon 下线应输出 `EarlyCon.Offline` 或等价 trace，和 `Serial8250Console.Online`、`BootConsole.Offline`
一起展示完整交接。
即使 `keep_bootcon` 保留 boot console，本阶段 active printk route 仍是 serial8250，legacy earlycon drain 也不得重放
handoff 后的正常 printk 记录。

payload、hello app 和 app-level smoke 只能通过 `printk` 前端或等价公开输出 API 产生输出，不得 import 或直接调用
`EarlyCon`、`BootConsole`、`Serial8250Console` 等 console backend，也不得为了收尾输出新增公开 `printk::flush()`
这类刷新 API。flush/drain、record cursor 推进、panic/shutdown 前的必要输出处理都属于 printk/console 子系统内部策略。
若需要直接验证 earlycon 或 console backend，只能放在 checkpoint KUnit 或等价的启动阶段单元测试中，并且测试挂点必须保证
被测 backend 处于允许访问的生命周期阶段。

smoke/KUnit 应覆盖：`stdout-path` 命中后发生 `Uart8250Port` 与 `Serial8250Console` 注册、非 stdout-path 设备不抢占
console、dummy/non-match console 不改变 registry、默认策略下 boot console 注销、`keep_bootcon` 保留 boot console、
handoff 后 printk route facts 符合预期、serial console 的 `membase` 来自 ioremap/vmalloc 映射而不是 direct map，
serial8250 write 后端记录 polling、LSR/THR、membase、non-SBI、IRQ deferred 事实，以及 handoff 后正常 printk
不会被 legacy earlycon/SBI drain 重放。后续应把当前仍偏对象内部的 console registry 行为 smoke 逐步迁移到
checkpoint KUnit/action-level 测试，app-level smoke 保留对公开 printk 输出和用户可见启动结果的验证。

## RootfsPhase 编码约束

`RootfsPhase` 是 `SMP Runtime Phase` 的第四个子阶段，formal model 路径为
`spec/model/phases/smp-runtime/rootfs/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/rootfs.rs`。该阶段必须在 `InitcallPhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `kernel_init_freeable()` 的 rootfs 准备边界。
为匹配 Linux paired diff exact anchor，`RootfsPhase.Started` checkpoint 不得在 `rootfs::setup()` 函数入口发布；
它表示 `init_eaccess(ramdisk_execute_command)` 之后即将进入 `prepare_namespace()` 的分支入口。因此顺序必须是
`KUnitRuntime.TrimmedReady`、`InitramfsSync.DeferredReady`、`RootfsConsole.DeferredReady`、
`RamdiskExecuteCommand.EaccessCheckpoint`、`RootfsPhase.Started`，随后才推进 prepare_namespace path classification
和 `RootFS.Online`。

初始 rootfs mount 不属于本阶段首次创建：它已经在 `ProcessPreparePhase` 的 `VfsCore.Setup` /
`RamFsType.Setup` / `VfsCore.MountInitialRamFsRoot` 中对应 Linux `vfs_caches_init()->mnt_init()->init_mount_tree()` 完成。
本阶段的 `RootFS.Transition::Enable` 表示 `prepare_namespace()` 中从初始 ramfs/rootfs backing 走向真实 root device 的
enable 位置；`RootFS` 是顶层根文件系统视图对象，不得把 enable 位置再建成独立对象。本轮必须记录
prepare_namespace 的输入条件：初始 ramfs rootfs 已存在，`FsStruct.root/pwd` 仍指向初始 root，`DevFs` 已挂载，
`BlockDeviceRegistry` 已有默认块设备作为 root device candidate；随后基于该默认块设备建立
`Ext2Driver` / `Ext2Volume` / `Ext2FileSystem` 链，并按 Linux `do_mount_root()` 的形态把真实 ext2 文件系统挂载到临时
`/root` 目录。随后必须把该 ext2 mount 通过 `VfsCore.Action::MoveMountToRoot` 移到 `/`，再通过
`FsStruct.Action::ChrootDot` 把任务可见的 `root/pwd` 切到 ext2 root；这两个动作在规格上必须分开表达。

本阶段覆盖 `kunit_run_all_tests()`、`wait_for_initramfs()`、`console_on_rootfs()`、
`init_eaccess(ramdisk_execute_command)` 对应 checkpoint、`prepare_namespace()` 和
`integrity_load_keys()` 的时序位置。当前 `CONFIG_KUNIT=n`，`kunit_run_all_tests()` 必须建模为
trimmed/no-op，不得单独升格为 `KUnitPhase`。

`InitramfsSyncDeferred`、`RootfsConsoleDeferred` 和 `IntegrityKeysDeferred` 在本轮仍只保留
deferred/position-preserved 语义。其中 `init_eaccess(ramdisk_execute_command)` 是 required checkpoint，必须记录当前
Linux-like 路径要求进入 `prepare_namespace()` 分支。`RootFS.Transition::Enable` 必须真实更新 VFS mount 与 `FsStruct.root/pwd`
事实，不得用单独测试 API 或 summary 伪造 `MS_MOVE` / `chroot(".")` 已完成；
root device candidate 只能来自已有 `BlockDeviceRegistry.default_device`，`/dev` 条件只能来自已有 `DevFs`，不得为了
rootfs smoke 新增测试专用设备 API。当前不把 `DevFs` 偷偷 remount 到新的 ext2 root 下；相关行为留给后续 mount
namespace/devtmpfs 轮次。

`prepare_namespace()` 内部非主线必须结构化记录。`wait_for_device_probe()` 在 Linux 中涉及
`deferred_probe_work`、`probe_count` atomic 和 `probe_waitqueue`，本轮保持 deferred，不能用 “boot-time 单任务”
吞掉 waitqueue/atomic 责任。当前命令行未启用 `rootdelay=`、`rootwait` 或 `rootwait=`，这些分支记录为
trimmed/no-op，同时保留 `wait_for_root()` 的轮询/睡眠路径为 deferred。当前 `.config` 下
`CONFIG_BLK_DEV_INITRD=n`，`initrd_load()` 为 trimmed；`CONFIG_MD=y`，`md_run_setup()` 为 deferred；
`CONFIG_ROOT_NFS=y` 但当前根不是 `/dev/nfs`，NFS root 为 deferred；`CONFIG_CIFS_ROOT=n`，CIFS root 为
trimmed；nodev root、`saved_root_name`/`ROOT_DEV` 解析和 `parse_root_device()` 变体仍 deferred。Linux 参考配置为
`CONFIG_EXT2_FS=n`、`CONFIG_EXT4_USE_FOR_EXT2=y`，因此要显式记录 arceos_ex 当前 `Ext2Driver` 是阶段性替代实现。
`CONFIG_DEVTMPFS=y` / `CONFIG_DEVTMPFS_MOUNT=y` 下 `devtmpfs_mount()` 是真实 Linux 路径；本轮只记录
deferred，并明确当前 `DevFs` 不会在 root switch 后重新挂到新的 ext2 root 下。

`IntegrityKeysDeferred` 必须同时记录 `CONFIG_INTEGRITY=y` 的调用位置，以及当前 `CONFIG_IMA=n`、`CONFIG_EVM=n`
导致 IMA/EVM x509 key loading 不展开的裁剪依据。

测试应覆盖 `RootfsPhase.Ready`、KUnit trimmed、initramfs wait deferred、rootfs console deferred、
ramdisk eaccess 强制进入 prepare_namespace、prepare_namespace 输入条件已具备、真实 ext2 root staging mount 曾建立于
`/root`、`MS_MOVE` 挂载移动完成、`FsStruct.ChrootDot` 完成、当前 root 为 ext2、可直接读取 `/etc/alpine-release`、
device-probe/rootwait/initrd/md/NFS/CIFS/devtmpfs 路径分类、integrity keys deferred/trimmed 细分事实，以及下一入口仍是
`FinalizePhase`。

## FinalizePhase 编码约束

`FinalizePhase` 是 `SMP Runtime Phase` 的第六个子阶段，formal model 路径为
`spec/model/phases/smp-runtime/finalize/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/finalize.rs`。该阶段必须在 `RootfsPhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `kernel_init()` 中 `kernel_init_freeable()` 返回后的收尾边界。

本阶段覆盖 `async_synchronize_full()`、`SYSTEM_FREEING_INITMEM`、init-only memory cleanup、
`mark_readonly()`、`pti_finalize()`、`SYSTEM_RUNNING`、`numa_default_policy()`、`rcu_end_inkernel_boot()` 和
`do_sysctl_args()` 的时序位置。当前 `AsyncCore`、ftrace/free_initmem、mapping protection 和 sysctl 参数路径
仍保留 deferred/position-preserved 语义；Kprobes/KGDB/BootConfig/PTI/NUMA 按当前配置记录为 trimmed/no-op。
`async_synchronize_full()` 不得被写成普通 no-op：Linux 通过
`wait_event(async_done, lowest_in_progress(NULL) >= ASYNC_COOKIE_MAX)` 等待全局 async work drain，
`lowest_in_progress()` 使用 `async_lock` 的 `spin_lock_irqsave()`，async worker 完成时更新 `entry_count`
atomic 并 `wake_up(&async_done)`。当前实现仍不展开 AsyncCore runtime，但必须把 waitqueue、irqsave
spinlock、atomic 计数和全局 cookie 边界记录为 deferred facts。

`SystemState.enter_freeing_initmem()` 必须紧跟 `AsyncFullSyncDeferred.Ready`，对应 Linux
`async_synchronize_full()` 之后、init-only memory cleanup 之前的
`system_state = SYSTEM_FREEING_INITMEM`。`InitMemoryCleanupDeferred`、`KernelMappingProtectionDeferred`
和 `PtiFinalizeTrimmed` 必须在该窗口之后运行。`SystemState.enable()` 只负责在 PTI finalize 之后发布
`SYSTEM_RUNNING`，并把 `SystemState.state` 推进到 `Online`。
`RcuCore.end_inkernel_boot()` 是本阶段的另一个主线 action，必须记录 `rcu_boot_ended == true`，但不得把完整
RCU GP 服务或 worker 运行伪装成已实现。实现还必须暴露 Linux `rcu_end_inkernel_boot()` 中
`rcu_unexpedite_gp()` 的 atomic decrement、`CONFIG_RCU_LAZY` 未启用导致 `rcu_async_relax()` 不改变 lazy
nesting 的裁剪事实、`CONFIG_PREEMPT_RT=n` 下 `rcu_normal_after_boot` 默认不触发 `WRITE_ONCE(rcu_normal, 1)`
的事实，以及 `rcu_boot_ended` publish fact。`numa_default_policy()` 的裁剪 checkpoint 必须位于
`SYSTEM_RUNNING` 之后、`rcu_end_inkernel_boot()` 之前，不能放在 `rest_init()` 路径。

测试应覆盖 `FinalizePhase.Ready`、async full sync deferred、init memory cleanup deferred/trimmed 事实、
mapping protection deferred、PTI trimmed、`SystemState.Online`/`SYSTEM_RUNNING`、NUMA default policy
trimmed、RCU in-kernel boot ended 与 atomic/WRITE_ONCE/lazy-trim facts、sysctl args deferred，以及下一入口仍是
`PayloadPhase`。

## PayloadPhase 编码约束

`PayloadPhase` 是 `Kernel` 的末尾阶段，formal model 路径为 `spec/model/phases/payload/`。它必须作为
`SmpRuntimePhase.Ready` 之后的后续阶段实现，而不是嵌套为 `SmpRuntimePhase` 的子阶段。入口必须要求
`FinalizePhase.Ready` 和 `FinalizeBoundary.Ready`，并消费 `payload_phase_next_boundary()` 事实。

KernelInitTask 的执行线从 `SmpRuntimePhase` 入口开始：`rest_init()` 通过 `TaskCreationCore` 把
`TaskEntry::KernelInit` 绑定到 `KernelInitTask`，并提交 release/dispatch facts；`SmpRuntimePhase` 的首个子阶段 `PreSmpInitPhase` 消费这些 entry/release/dispatch facts 后进入。后续阶段按 phase 顺序衔接到 `FinalizePhase`，再自然进入 `PayloadPhase`。因此 selected payload 的执行归属应从这条连续执行线推出，而不是由 `PayloadPhase` 单独声明一个调用者事实。`BootIdleTask` 只负责 idle loop、need_resched observation 和 schedule boundary；`KthreaddTask` 当前提供内核线程管理者 ready/provider 事实和最小 schedule-loop 入口边界。

`UserBootPayload` 是 Linux-like 用户态首进程路径的 selected payload 变种。代码生成或手写实现必须保持以下边界：

- syscall 入口沿用 `ExceptionStream -> SyscallException`，不得为了用户态 hello 新增独立根 `Syscall` 对象或绕过异常分发。
- Linux `kernel_execve()` 完整路径中的同步协议必须保留为显式 `PayloadExecSyncBoundaries` deferred contract，而不能被 `KernelInitTask`、boot-only 或 no-return handoff 事实吞掉。该 contract 至少记录 `binfmt_lock`、`cred_guard_mutex`、`exec_update_lock`、`exec_mmap()` 中的 local IRQ disable/enable、`mmap_lock`、`exec_mmap()` task lock/mm handoff、`sighand->siglock`、`tasklist_lock`、`fs->lock` + RCU read side、membarrier/switch-mm ordering、`sched_mm_cid_*()` 的 rq_lock_irqsave+smp_mb、`bprm_mm_init()` / `finalize_exec()` 的 task_lock rlimit 边界、files unshare/CLOEXEC file_lock、io_uring cancel、POSIX timer siglock、namespace switch、exec 成功后的 rseq/perf/audit/accounting hooks、完整 binfmt/script retry 和 panic terminal 边界。当前 `../linux-6.12/.config` 中 `CONFIG_MODULES=n`，因此 `request_module("binfmt-...")` retry 必须记录为 trimmed/no-op 而不是 deferred。当前 `APP=user-boot` 只实现最小 VFS/ELF/UserAddressSpace/trap-return handoff；上述 Linux exec guard 不得标为已实现，也不得只留在 roadmap 表格。
- `UserBootPayload.Setup` 必须由已经到达 `PayloadPhase` 的 `KernelInitTask` 执行线驱动；实现应记录 payload 归属事实，而不是把用户态入口建模为独立启动根。
- Linux `PayloadPhase.Online` 表示 selected payload handoff attempt。Linux runtime 插桩必须覆盖
  `init=` requested-init 成功路径和默认 fallback 路径：requested-init 分支在调用
  `run_init_process(execute_command)` 前记录同一 checkpoint id；默认 fallback 分支继续在
  `try_to_run_init_process("/sbin/init")` block 前记录。当前 Linux marker plan 只支持单一
  `checkpoint_name + checkpoint_variant` fingerprint，因此 `LKM_CHECKPOINT` marker 注释仍只保留在
  规范 mapping anchor 上；分支等价的额外 runtime record call 不得添加第二个不同 fingerprint 的 marker 注释。
- `UserBootPayload` 的 init 选择必须对齐 Linux 6.12 `init/main.c::kernel_init()` 中 `run_init_process()`、`execute_command` 和 `try_to_run_init_process()` 的顺序。`BootParam` 必须从 `StaticCommandLine` 解析 `init=` value 并保留最后一次出现的值；当前首片只支持非空、绝对路径、长度不超过固定 selected-path buffer 的 requested init。存在 `init=` 时，必须先尝试 requested path；成功后停止启动编排链，失败时进入 Linux-like “Requested init ... failed” panic terminal boundary，不得继续 fallback 到 `/sbin/init`、`/etc/init`、`/bin/init` 或 `/bin/sh`。没有 `init=` 时，在 `ramdisk_execute_command` trimmed、空 `CONFIG_DEFAULT_INIT` trimmed 之后，依次尝试 `/sbin/init`、`/etc/init`、`/bin/init`、`/bin/sh`，直到首个成功 candidate 成为 selected path；所有 candidate 都失败时进入 Linux-like “No working init found” panic terminal boundary。当前首片实现的 candidate 成功口径是：从当前 `FsStruct.root` 经 `VfsCore.ReadPath` 读取到文件，并通过当前 `ElfObject.Preset/Setup` 支持的 ELF 检查；这不等价于完整 `kernel_execve()` 成功。Linux `kernel_execve()` 成功时返回整数 0，但当前 task 已经完成 exec 身份转换，`kernel_init()` 不再继续尝试后续路径；实现必须把成功 candidate 记录为停止 fallback chain 和不返回启动编排链的边界。candidate 失败必须分类记录并允许继续尝试后续 default candidate，不得在默认 `/sbin/init` 失败时直接 panic。每一次失败必须先记录到 `UserBootPayload` 的长期 failure observation 中，再触发通用 `UserBoot.InitAttemptFailed` checkpoint；该 observation 至少包含 path kind、实际 path 长度、stable stage、stable reason、requested-terminal/default-nonfatal 标志，stage/reason 名称必须来自模型枚举而不是一次性日志文本。该 checkpoint 是长期 trace 边界，KUnit 只是可选只读 consumer；failure KUnit 必须使用显式 failure-only probe（当前为 `PROBE=user-boot-failure`），不得并入默认成功路径 `user-boot` KUnit plan；它不得改变成功路径 checkpoint 序列，也不得引入测试专用对象 API。`argv[0]` 必须来自实际 selected path bytes：默认 overlay 下通常是 `/sbin/init`，`QEMU_APPEND='earlycon=sbi init=/bin/ls'` 下必须是 `/bin/ls`，不能只用 fallback enum 的静态 path。后续若补 `rdinit=`、脚本 binfmt、权限或完整 errno 语义，必须继续保持 Linux 该段顺序和终端 panic 语义。
- `make test` 中的 `APP=user-boot` 验收不得只依赖 QEMU/SBI shutdown 退出码；host harness 必须解析普通输出中的 `user exit status=N`，且 native 与每个 configured provider 下均只有 `N == 0` 计为通过。非零状态、缺失该输出或 QEMU 命令自身失败都必须判失败。guest 仍由用户态 `exit/exit_group` syscall 路径打印状态并执行 shutdown，harness 只解释输出，不迁移 shutdown 责任。手工 `make run APP=user-boot` 的默认内核命令行必须进入 `init=/bin/sh`，而 `make test` 的 user-boot smoke case 必须显式传入 `QEMU_APPEND=earlycon=sbi`，继续验证 overlay-installed `/sbin/init` 的 `user_smoke` fixture。`make test` 还必须包含独立发行版 rootfs smoke：`ROOTFS_OVERLAY=none QEMU_APPEND='earlycon=sbi init=/bin/ls'` 验证真实 Alpine `/bin/ls -> /bin/busybox`，以及 `ROOTFS_OVERLAY=none QEMU_APPEND='earlycon=sbi init=/bin/sh'` 配合 host harness 在可见 shell-ready marker 之后喂入固定短脚本 `/bin/ls\nexit\n`，验证真实 `/bin/sh` 派生 child 执行 `/bin/ls` 的 fork/wait4/execve 路径；后者的输入属于 host harness，不是 kernel-side stdin ready-data fixture，也不得在早期串口诊断和 initcall 完成前预先注入。host delayed-input timeout 默认与 DF-0003 外部命令 stress 预算对齐为 120s 量级，输出必须先 drain 到 log 再回放，避免当前终端逐字转发与 BusyBox prompt/terminal query 交织污染非 PTY 结论；成功必须检查 rootfs 目录 marker（当前为 `lost+found`）和 `user exit status=0`，fork/wait4/execve 边界由 checkpoint/KUnit facts 回归，不能依赖默认打印的 `wait4 child handoff` 诊断行。
- `rc.local` 第一阶段是 opt-in focused diagnostic，而不是默认 `make test` 门禁。该 case 必须使用 `ROOTFS_OVERLAY=none` 加 checked-in `ROOTFS_FILE_OVERLAY_DIR=tests/rootfs-overlays/rc-local`，只替换 `/etc/inittab` 与 `/etc/rc.local`；`/etc/inittab` 直接让 BusyBox PID1 以 `::sysinit:/bin/sh /etc/rc.local` 执行脚本，host harness 在观察到 `lkm-rc-local: begin`、rootfs 列表 marker（当前 `lost+found`）和 `lkm-rc-local: end status=0` 后可以按 marker-only 成功终止 QEMU。`make difftest` 的默认 paired checkpoint baseline 必须切到这个 rc.local direct-inittab case；原发行版 `init=/bin/sh` delayed `/bin/ls\nexit\n` baseline 只能通过显式 `DIFFTEST_CASE=impl/arceos_ex/tests/stress/cases/linux-exact-baseline-difftest.toml` 运行，且 difftest 入口可以用 `DIFFTEST_CASE` 列出多条 case path。rc.local difftest 为了让 Linux 侧 dump checkpoint buffer，可以在成功 marker 后触发 `/sbin/poweroff -f`；marker-only 成功口径仍停在脚本完成 marker，不要求 arceos_ex 完整 shutdown。若 Linux dump 需要额外 poweroff exec，paired runner 只能通过显式 checkpoint count limit 排除 marker 之后的 dump-only exec，不得把该额外 exec 伪装成 rc.local 脚本语义一致性。该默认 case 还必须带 exact checkpoint coverage audit；`checkpoint_scope` 是 hard comparison scope，不是全部 exact mapping 覆盖声明，任何 exact checkpoint 必须进入 hard scope 或在 outside-scope accounting 表中登记理由。该片不宣称真实 OpenRC `local.d`、`/sbin/openrc` service graph、daemon supervision、runlevel completion、脚本 binfmt、完整 PID1 lifecycle 或全部 exact checkpoint 已比较完成。
- 发行版 `/bin/sh` 分阶段验证不得依赖“当前尚未支持 symlink”这类临时缺口来绕过 `/sbin/init`。默认 staged user smoke 应覆盖 `/sbin/init`，保持首个 fallback candidate 成功；默认 checked-in overlay map 只保留 `tests/user/rootfs-overlay.map`，并把综合 `user_smoke` 覆盖到 `/sbin/init`。`user_smoke` 的主文件是 `tests/user/smoke/smoke.c`，它调用按能力拆分的子用例文件；框架层输出必须使用 `user-smoke:` 前缀，打印总入口 `user-smoke: begin`、每个 case 的 `user-smoke: case <name>: begin` 和 `user-smoke: case <name>: end status=N`、以及最终 `user-smoke: end status=N`，并在总入口和每个 case 边界之间用空行分隔。子用例内部只输出具体能力点，例如每个 syscall 路径成功后的 `syscall ... ok` 标记，避免把“用户 ELF 已加载”和“具体 syscall 已支持”混在同一条 `user hello` 观测里；host harness 仍只以 `user exit status=N` 判断通过，不依赖这些人读 marker。`fileio` 子用例承载当前 regular file 与 directory file/VFS syscall smoke，包括 `open/read/close/newfstatat/fstat/faccessat/fcntl/lseek/ioctl/getdents64/readlinkat` 和 `linux_dirent64` 解析；原 `sh_probe` 综合 case 不再作为长期 case 名保留，必须拆为独立 `fpu_mmap`、`credentials`、`process_identity`、`tty_termios`、`uts_cwd`、`time_random` 等能力 case，使失败码和 case 名直接指向对应能力边界。若后续需要 BusyBox/动态 libc 启动路径分解出的非 VFS 轻量 syscall 探针，也必须按能力归类加入对应 case 或新增明确 case，不得重新聚合为含义混杂的 `sh_probe`。host harness 必须为 user-boot smoke 使用独立临时 rootfs image、默认 overlay map、`FORCE=1` 和显式 `QEMU_APPEND=earlycon=sbi`，避免已有 `build/virtio-blk.raw`、手工 `/bin/sh` 默认启动或其它 overlay case 污染结论。面向发行版 `/bin/ls` 或 `/bin/sh` 缺口分析仍采用流程化的本地静态/半静态分析和 guest diagnostic 结合；分析流程不得新增长期工具、Makefile target 或 rootfs staging 改动。外部命令执行与原生 `/sbin/init`/OpenRC 不得在未定位和规格化前加入强制通过门禁；必须先通过真实 guest 证据定位 `clone/vfork/fork`、子进程 `execve`、`wait4`、pipe/dup 继承、CLOEXEC propagation、job-control/signal 等缺口，再逐项对照本地 `../linux-6.12` 规格化实现。每个用户态 syscall/VFS 子项进入 model/coding 规格前，必须以本地 `../linux-6.12` 为参考源码，而不是按泛化 Linux 行为推断；当前 `/bin/ls` 第一批目录/fd 工作的基准入口包括 `arch/riscv/kernel/syscall_table.c`、`include/uapi/asm-generic/unistd.h`、`fs/open.c::do_sys_openat2()/sys_openat()`、`fs/readdir.c::sys_getdents64()/iterate_dir()`、`fs/stat.c::vfs_getattr()/sys_newfstatat()/sys_newfstat()/sys_readlinkat()`、`fs/file.c::fdget()/fdget_pos()/alloc_fd()`，以及后续命名解析需要的 `fs/namei.c`；若首片实现刻意裁剪其中的锁、RCU、权限、mount namespace、LSM 或完整 errno 行为，必须在 model/coding 中记录为 trimmed/deferred，不能只在实现里静默省略。
- 用户可执行文件对象命名为 `ElfObject`，不得生成单独的 `ElfLoader` 资源对象。`ElfObject.Preset` 只检查 ELF 类型支持；`ElfObject.Setup` 解析 ELF header / program headers，并形成 `PT_LOAD` 映射计划、段权限、entry 和 `.bss` 清零计划；真实映射到 `UserAddressSpace` 与 `.bss` 清零事实归 `UserAddressSpace.Setup`。dynamic libc 支持仍然使用 `ElfObject`：主程序的 `PT_INTERP` 只绑定 interpreter 路径事实，并驱动 `UserBootPayload` 把解释器作为 role 为 interpreter 的第二个 `ElfObject` 读取和解析；不得因此引入 `ElfLoader` 资源对象或独立 `Load` 生命周期阶段。`ElfObject.Enable` 只确认 entry、用户栈和 trap frame 已可用于进入用户态。ELF 类型语义必须按 Linux 6.12 `fs/binfmt_elf.c::load_elf_binary()` 分类，而不是把 `ET_DYN` 直接等同于动态链接、把 `ET_EXEC` 直接等同于静态链接：`PT_INTERP` 是当前 dynamic/interpreter 路径的判定依据。主程序 role 必须继续支持 `ET_EXEC`；同时支持 `ET_DYN + PT_INTERP` 作为 PIE main，并为其使用固定 non-overlap main load bias。首片不实现 Linux 的 `ELF_ET_DYN_BASE + ASLR + mmap` 搜索，只记录为 trimmed；`ET_DYN` 且没有 `PT_INTERP` 的 direct-loader 形态暂不作为 init main 接受，必须记录为 deferred/unsupported，而不是静默按 PIE 装载。
- `UserAddressSpace` 是多实例用户地址空间对象，低地址用户区独立；高地址内核映射共享或引用 `SwapperVm`。`SwapperVm` 继续是内核共享地址空间实例，不应被改造成普通多实例用户地址空间类型。首个 `UserAddressSpace` 实例必须在 `Preset` 时依赖 `KernelInitTask.Online`，并记录 `KernelInitTask` 绑定该首个用户地址空间的事实，供后续 stack、ELF mapping 和 trap frame setup 消费；该绑定只表示 kernel_init 用户态启动路径已拥有待启用地址空间，不表示已经写入 `satp` 或完成硬件地址空间切换。
- `UserAddressSpace.Setup` 消费 `ElfObject` 的 `PT_LOAD` 映射计划，记录 segment/stack/heap mapping facts、用户页 `U` 权限事实和内核映射 `U=0` 事实，并在切换前分配 backing pages、复制 ELF 文件内容、清零 `.bss`/stack/heap，建立 page-table-shaped view。对存在 `PT_INTERP` 的 dynamic executable，它必须把主 ELF 和 interpreter ELF 的 `PT_LOAD` 段映射到同一个 `UserAddressSpace`，并记录 interpreter mapping facts；interpreter 可为 `ET_DYN`，但首轮使用固定 non-overlap load bias，不引入完整 ASLR/VMA tree。主程序若是 `ET_DYN + PT_INTERP` PIE，也必须在 `ElfObject.Setup` 前绑定固定 non-overlap main load bias，使主 PIE、interpreter、heap 与 stack 窗口互不重叠；后续若实现 Linux 式 ASLR/VMA selection，必须替换该固定 bias 并保留差分说明。ELF segment backing 和 low-half leaf PTE 按页粒度覆盖 `align_down(p_vaddr)..align_up(p_vaddr + p_memsz)`；因此 `mprotect`/`munmap` 首片对 mapped user range 的判断也必须使用同一页范围，允许动态链接器对 GNU_RELRO 所在整页执行保护变更。dynamic linker 首片还必须提供阶段性的 user heap / anonymous mmap arena，用于承接早期 `brk`/`mmap` 需求；这属于正式运行时语义，不是 smoke/KUnit 专用 API。`UserAddressSpace.Enable` 是真实切换前的 satp-ready 边界：它可以分配真实 Sv39 用户页表页、安装低端用户 leaf PTE、复制或共享 `SwapperVm` 高端 root entries、生成待使用的 satp token，并标记 `runtime_ready` 表示该地址空间可被下一轮 trap return 消费；但它必须同时记录 prepared-but-not-current 事实，不得写入 `satp`、不得执行 `sfence.vma` 作为地址空间切换、不得执行 `sret`、不得设置 `SyscallTable` 或 `UserInitProcess` 状态。
- `UserAddressSpace` 的阶段性 heap / anonymous mmap arena 是当前裁剪边界，前半供 `brk`，后半供 anonymous `mmap`，用于覆盖已观察到的 BusyBox/musl 早期请求；它不是 Linux 的完整 VMA/ASLR/overcommit 策略，也没有对应的 Linux 固定配置尺寸。若原生 OpenRC 多次 child exec 暴露页压力或旧 mm 回收缺口，必须先用 execve/address-space 诊断定位失败子原因，再按 Linux `exec_mmap()`/VMA/page-fault 差分修正，不能通过调小 arena 尺寸规避。
- `UserStack.Setup` 必须为 static libc 用户 init 建立最小 Linux initial stack，而不是只把 `sp` 置为栈顶。当前最小布局至少包含 `argc=1`、`argv[0]=selected_path`、`argv` 终止空指针、`envp` 终止空指针，以及 `auxv` 中的 `AT_PAGESZ` 和 `AT_NULL`，并让 `UserTrapFrame.Setup` 消费该布局起点作为用户 `sp`。默认 overlay fixture 下 selected path 仍通常是 `/sbin/init`；发行版路径中必须由实际成功的 fallback candidate 决定。dynamic libc 首片必须进一步提供动态链接器关键 auxv 字段，至少包括主程序 `AT_PHDR`、`AT_PHENT`、`AT_PHNUM`、`AT_ENTRY`、interpreter base 对应的 `AT_BASE` 和 `AT_PAGESZ`/`AT_NULL`；随机化、guard page、栈扩展、`AT_RANDOM`、`AT_EXECFN`、真实 argv/envp 派生和完整 auxv 后续再展开，但不得把普通 libc fixture 伪装成 no-libc `_start` 程序。
- `APP=user-boot` 的运行期 checkpoint/KUnit 观测必须按 `UserBootPayload.selected_path` 校验用户栈中的 `argv[0]`，不得把 `/sbin/init` 写死为唯一成功路径。默认 overlay 仍应观测 selected path 为 `/sbin/init`；专门的 fallback 临时 overlay 必须能观测 selected path 和 `argv[0]` 同时变为 `/bin/sh`。
- 用户启动定位应优先复用长期 checkpoint。`UserBoot.InitAttemptFailed` 用于 pre-U-mode candidate 失败；`UserBoot.MainElfReady` / `UserBoot.InterpreterReady` / `UserAddressSpace.Ready` 应携带主 ELF type、load bias、entry、phdr 和 mapping 诊断，支撑后续与 Linux 6.12 差分验证。只有当真实发行版路径推进到现有稳定边界之外、且没有足够内部可见性定位时，才增加新的长期 checkpoint；不得为了单次试探增加无法复用的一次性 checkpoint。
- `UserTrapFrame.Setup` 只准备 `sepc=elf.runtime_entry`、`sp=user_stack.initial_sp`、用户态 `sstatus` 事实和关联的 `UserAddressSpace`，表示下一轮可以由统一 trap return 消费；静态程序的 runtime entry 是主 ELF entry，动态程序的 runtime entry 是 interpreter entry。它本身不得执行 `sret` 或观察用户态已经进入。
- RISC-V 用户态 FPU 首片以 Linux 6.12 `arch/riscv/kernel/process.c::start_thread()`、`arch/riscv/include/asm/csr.h` 和 `arch/riscv/include/asm/switch_to.h` 为基准。`UserTrapFrame.Setup` 在 CPU F/D capability 已存在的当前目标上必须把用户态 `sstatus` 设置为 `SR_PIE | SR_FS_INITIAL` 且 `SPP=0`，使 hard-float Alpine/musl 程序中的普通用户态 FPU 指令可执行。当前只服务单个 PID1 用户任务和 musl/BusyBox 启动路径；Linux 的 `thread.fstate` 保存恢复、`fstate_save()/fstate_restore()`、fork/exec fstate 继承、signal/ptrace fpstate、lazy/clean/dirty 优化、多任务 FPU context switch 和 vector state 管理继续 deferred，不得在首片中伪装为已实现。
- B 阶段的 U-mode entry 必须复用现有 `EventStream` trap return 形状，并在模型中用 `within UserModeTrapReturnContext { ... }` 包住 `UserInitProcess.Action::EnterUserMode` 的最终硬件交接。实现必须写入 `sscratch/sepc/sstatus/satp`、在 `satp` 写入后执行 `sfence.vma`，再通过 `sret` 进入用户态；该 context 的退出是 `Never`，因为正常路径不返回启动编排链。用户态 trap 入口不能把用户 `sp` 当成内核 trap frame 栈使用，必须先切到内核拥有的 trap 栈，例如通过 `sscratch` 暴露的 trap stack top，再保存完整 trap frame。trap 入口保存用户 `sstatus` 到 trap frame 后，必须按 Linux 6.12 `arch/riscv/kernel/entry.S::handle_exception` 的形状清除 live `sstatus` 中的 FS/VS（以及当前不应泄漏到通用异常处理的 SUM），以保持内核态 FPU/vector disabled；返回用户态时再从 trap frame 恢复用户 `sstatus`。
- 用户态 trap/syscall kernel stack 的配置必须对齐本地 Linux 6.12 RISC-V `.config`，当前基准为 `CONFIG_THREAD_INFO_IN_TASK=y`、`CONFIG_IRQ_STACKS=y`、`CONFIG_HAVE_ARCH_VMAP_STACK=y`、`CONFIG_VMAP_STACK=y`、`CONFIG_THREAD_SIZE_ORDER=2`。因此 `THREAD_SIZE = PAGE_SIZE << THREAD_SIZE_ORDER = 16KiB`，`THREAD_ALIGN = 2 * THREAD_SIZE = 32KiB`，`OVERFLOW_STACK_SIZE = 4KiB`，`IRQ_STACK_SIZE = THREAD_SIZE`；实现必须把 `sscratch` 暴露的用户 trap 栈切到 VMALLOC VA，而不是静态 `.bss` 直接映射栈。首片应通过 `VmallocAllocator` 取得 32KiB 对齐的 stack area，并在低侧保留至少 4KiB unmapped guard gap；只把 16KiB usable stack backing pages 映射为 kernel RW，stack top 位于 `base + THREAD_SIZE`，使 Linux `entry.S` 的 bit-test 几何成立。`UserInitProcess.EnterUserMode` checkpoint 必须只读记录 stack base/top/size/alignment、guard base/size/unmapped、backing phys/order、Linux config facts、overflow stack base/top/size，以及尚未接入的 entry scratch/IRQ-stack-switch deferred facts，便于后续定位真实越界。`arch/riscv/kernel/entry.S` 中的 stack-overflow bit test 与 `handle_kernel_stack_overflow` handoff 不得在缺少 scratch slots 时直接使用 `t0/t1` 等通用寄存器试探，因为这会在正常返回路径破坏用户寄存器；本项目接入该 handoff 前必须先建立类似 Linux `thread_info.a0/a1/a2` 的入口 scratch storage 或等价保存机制。`kernel/fork.c::alloc_thread_stack_node()` 的 per-task vmalloc stack 泛化和 hardirq `call_on_irq_stack()` 仍继续 deferred；guard/overflow 只是保护和诊断，不能替代把 `UserAddressSpace`、ELF mapping staging 等大对象移出 syscall/trap 局部栈。
- 用户态 `ecall` 必须进入既有 `ExceptionStream -> SyscallException` 分支，不得新增独立根 `Syscall` 对象或绕过异常分发。`SyscallException` 承担用户态 syscall 入口、来源检查、参数提取和分发选择；不得再生成独立 `SyscallDispatcher` 对象。`SyscallTable` 是独立对象，承载具体 syscall action 集合；具体 syscall 是 `SyscallTable` 的 actions，不是单独资源对象。
- `FilesStruct` 表示任务拥有的打开文件上下文，和表示 root/pwd 的 `FsStruct` 并列；不得把 fd table 塞进 `FsStruct`。首轮 `FilesStruct.Setup` 必须在同一 `KernelInitTask` 执行线上建立 `FileDescriptorTable`、stdio `OpenFileDescription` 和 console-like `FileBackend::CharDevice`，预安装 fd 0/1/2，并记录 fd table、flags、offset、close-on-exec 和 next-fd 基础事实。当前 read-only 扩展允许再建立一个 filesystem opened instance：`openat` 从当前 `FsStruct.root` 经 VFS 读取已存在 regular file 或 directory，`FileDescriptorTable.Install` 分配固定首个 filesystem fd，`read/getdents64/close/newfstatat/fstat` 从该 opened instance 操作，regular/directory 不获得泛化多 open。`/dev/tty` 和 `/dev/tty[0-9]+` 首片允许在固定容量 fd table 中选择最低空闲 fd；stdio 仍打开时自然落在 3..15，若 Linux `close(0/1/2)` 已清空对应 entry，则后续 TTY open 可复用该低号空槽。每个 TTY fd entry 指向当前 console-like `Tty0` char-device backend，用于 BusyBox/OpenRC 的 tty 探测；每个 fd entry 保留自己的 status flags 和 close-on-exec bit，关闭一个 TTY fd 只清除对应 fd entry，不影响其它 TTY fd。这些路径当前仍只是同一个 `tty0` 后端的阶段性别名，该实例和 fd0/1/2 共享当前最小 char-device 后端事实，但仍通过 `FileDescriptorTable -> OpenFileDescription -> FileBackend::CharDevice` 路由；fd table 满时返回 `EMFILE`，不得继续把第二个 TTY alias 误分类为单槽 `AlreadyOpen`。`close(fd)` 首片以 Linux 6.12 `fs/open.c::sys_close()` / `fs/file.c::close_fd()` 为基准，对任意当前已打开 fd 清空对应 entry；无效或越界 fd 返回 `EBADF`，不再映射为 `ENOSYS`。关闭 fd0/1/2 只移除该 fd entry，不销毁共享 console-like 后端，也不实现完整 stdio 重新绑定、`filp_flush()`、`fput()` 或跨任务 files 复制语义。fd table 可以有固定小容量 dup 槽，`F_DUPFD/F_DUPFD_CLOEXEC` 复制 fd entry 指向同一 OFD，不创建新后端；`dup3(oldfd,newfd,flags)` 是定点 fd entry 替换语义，不复用 `F_DUPFD` 的最低空闲槽搜索。`TIOCGPGRP/TIOCSPGRP` 首片仍由 fd table/OFD/backend 先验证 char-device fd，具体 foreground process group 读取/更新事实由当前 `UserInitProcess` 的 controlling tty 子状态提供，不落入 fd table。`ppoll` 首片同样只能通过 fd table/OFD/backend 观察已有 fd 的立即 readiness，不得在 syscall table 中绕过 fd table 直接按 fd 号猜测。VFS path walk 会按当前 fast symlink 首片 follow symlink 后再得到 regular file 或 directory；fd table 不持有 symlink 本身。`readlinkat` 不打开 fd，必须经 `FilesStruct -> VfsCore` 的 no-follow final symlink 路径读取 symlink body。块设备后端、完整 devtmpfs/`/dev/console`、真实 TTY/N_TTY、多个 TTY 实例、TTY driver registry、权限、slow symlink/nofollow 控制、完整 poll waitqueue、共享 fd table、普通文件写路径、page cache、完整 fdtable 扩容/refcount/锁和跨任务共享语义可以 deferred，但必须在模型和代码事实中明确标记，不得伪装为已实现。
- `fchown(55)` / `fchmod(52)` 首片以本地 RISC-V `include/uapi/asm-generic/unistd.h` 号表和 Linux 6.12 fd lookup 形态为基准：syscall table 只做参数提取和 errno 映射，实际行为必须进入 `FilesStruct`，再由 `FileDescriptorTable` 验证 fd 当前已打开。有效 fd 成功返回 0，并在 fd entry 记录 fd-local owner uid/gid 与 chmod permission mode override；无效或越界 fd 返回 `EBADF`。这些 metadata 必须随 parent fd snapshot/restore 一起保存和恢复，避免 child continuation 污染 parent；`dup3` 复制 fd entry 时也复制当前 fd-local metadata。`fstat(fd)` 对同一个 fd 至少保留文件类型 bits 并反映 `fchmod` 设置的 permission bits；当前用户态 stat 写出结构只强制 size/mode，uid/gid 若未纳入 ABI 写出，可先作为对象事实保留。该首片不得绕过 fd table 特判 fd0，也不得实现或声称 path chmod/chown、inode 持久化、ext2 writeback、权限/capability 检查、LSM/security、TTY ownership、mount/idmapped mount、user namespace 或完整 supplementary group 权限语义。
- `/dev/null` 首片是内建 char-device alias，用于解除当前 OpenRC/getty stdio 重定向边界；它不得绕过 fd table 或在 syscall table 中伪造 fd。`openat(AT_FDCWD, "/dev/null", O_RDONLY/O_WRONLY/O_RDWR | O_LARGEFILE/O_CLOEXEC/O_NONBLOCK)` 必须在普通 filesystem/open 权限拒绝前由 `FilesStruct.OpenNullPath` 识别，通过最低空闲 fd slot 安装 `OpenFileDescriptionRef::Null`，并保留 file status flags 与 close-on-exec bit；`O_DIRECTORY` 打开 `/dev/null` 返回 `ENOTDIR`。Null fd 的 `read` 返回 0，`write` 丢弃数据并返回请求长度，`fstat` 返回 device node，TTY ioctl helper 对它返回 `ENOTTY`，不得把它误当 TTY。`ppoll` 可按该 fd entry 的 readable/writable 位立即报告 `POLLIN|POLLRDNORM` 或 `POLLOUT|POLLWRNORM`。本片仍不实现完整 devtmpfs、设备号、权限模型、`/dev/console`、多 char-device registry、LSM/security、mount namespace 或真实 null driver 生命周期。
- `openat` 的目录目标语义以 Linux 6.12 `include/uapi/asm-generic/fcntl.h::O_DIRECTORY`、`fs/open.c::build_open_flags()` 和 `fs/namei.c` lookup flags 为基准：`O_DIRECTORY` 的含义是 resolved target 必须为目录，它不是打开目录 fd 的必要条件。因此 `openat(AT_FDCWD, "/", O_RDONLY|O_LARGEFILE)` 必须经 `FilesStruct -> VfsCore` 解析为目录并安装 directory opened instance；`openat(..., O_DIRECTORY)` 解析到非目录时必须返回 `ENOTDIR`，不得把 `VfsError::NotDirectory` 兜底映射为 `EIO`。当前仍只支持 read-only regular/directory 打开；write/create/truncate/append、`O_PATH`、`O_NOFOLLOW`、权限/LSM、mount namespace、完整 lookup retry 和完整 errno 继续 deferred/trimmed。
- stdin `read(63)` ready-data 首片以 Linux 6.12 `fs/read_write.c::ksys_read()/vfs_read()`、`fs/file.c::fdget_pos()`、`drivers/tty/tty_io.c::tty_read()`、`drivers/tty/n_tty.c::n_tty_read()` 和 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_read=63` 为基准。`SyscallTable.Action::Read` 必须经 `FilesStruct -> FileDescriptorTable -> OpenFileDescription -> FileBackend::CharDevice -> NTtyLineDiscipline -> TtyFlipBuffer` 复制 stdin/tty 数据，不得在 syscall table 中直接写测试字符串。ready-data 有两类来源：真实 UART RX 按 Linux 6.12 `drivers/tty/serial/8250/8250_port.c::serial8250_rx_chars()` / `drivers/tty/tty_buffer.c::tty_flip_buffer_push()` 形状进入 `Serial8250RuntimePort.HandleInterrupt -> ReceiveChars -> TtyFlipBuffer.Push` 后发布 bounded raw slice；受控 `user_smoke` fixture 则只能在 `UserBootPayload` 清理 initcall loopback probe 残留之后，由 `FilesStruct.Action::PrepareDefaultStdinReadyData` 在进入用户态前准备。真实发行版 init/sh/ls ELF 不得注入该 fixture，也不得继承 loopback probe byte。当前 N_TTY 首片只消费 36 字节 old termios 中 `c_lflag` 的 `ICANON` 位：canonical 模式下，fd0/`/dev/tty` 只有当 bounded ready slice 从当前 read offset 起包含 `'\n'` 时才可读，`read` 最多复制到该 newline（若用户 buffer 更小，则复制 buffer 前缀并保留剩余行数据）；noncanonical 模式保持已有 byte-ready 行为。对照 Linux `n_tty_read()`，没有 N_TTY-readable data 且用户请求长度非零时，blocking tty read 会等待输入、hangup、signal 或其它终端条件；当前非 fixture 发行版路径只引入 `TtyInputWait` 首片：syscall 侧打开可被 supervisor external interrupt 打断的等待窗口，反复观察 `FilesStruct -> FileDescriptorTable -> OpenFileDescription -> FileBackend::CharDevice -> NTtyLineDiscipline -> TtyFlipBuffer` readiness，等待真实 `UART -> PLIC -> IRQ core -> Serial8250RuntimePort.HandleInterrupt -> TtyFlipBuffer.Push` 发布足以满足当前 termios 模式的 data 后再返回；它不得直接轮询 UART RX/LSR、不得调用 UART handler、不得 claim/complete PLIC。受控 `user_smoke` fixture 必须 opt out 该等待窗口，继续用 no-ready `ENOSYS` 回归避免默认 `make test` 无输入卡死。`read(fd, buf, 0)` 仍按 Linux 返回 0。echo/erase、`VEOL/VEOF`、CR/NL 转换、`ISIG` 信号生成、真实 wait_queue_head_t/poll_table、scheduler sleep、job control、controlling tty 完整语义、hangup/EOF、signal interruption/restart、nonblocking flags 和完整 N_TTY wakeup 继续 deferred/trimmed，不得把当前 IRQ-ready 首片声称为完整 Linux blocking read。`PROBE=user-read-trace` 是显式低噪声 read 结果诊断；它必须在不改变 read 返回值、errno 映射、checkpoint 序列、user-smoke 判定和默认 `make test` 输出的前提下，打印每次 supported `read(63)` 的 fd、requested length、`USER_COPY_MAX` capped length、成功返回值或失败原因、trap mode、`a0..a2`、`sepc` 和 `stval`，必要时打印内部 `FileError` 或 copy-to-user 失败原因。该 probe 只能作为定位证据。
- `ppoll(73)` 首片以 Linux 6.12 `fs/select.c::sys_ppoll()/do_sys_poll()/do_pollfd()`、RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_ppoll=73` 和 `include/uapi/asm-generic/poll.h` 的 `struct pollfd { int fd; short events; short revents; }` / `POLL*` 位为基准。实现必须先复制用户 `pollfd` 数组，按 Linux 规则处理每个 entry：`fd < 0` 写回 `revents=0`；无效 fd 不让 syscall 整体失败，而是在该 entry 写回 `POLLNVAL` 并计入 ready 数；有效 fd 通过 `FilesStruct -> FileDescriptorTable -> OpenFileDescription -> FileBackend` 取得当前可立即观察的 read/write readiness，再按 `events | POLLERR | POLLHUP` 过滤后写回 `revents`；返回值为非零 `revents` entry 数。可读 regular file/directory 可以报告 `POLLIN|POLLRDNORM`；char-device fd 的 read readiness 必须通过 `NTtyLineDiscipline` 观察当前 termios：canonical 模式只有 newline-terminated bounded slice 报告 `POLLIN|POLLRDNORM`，noncanonical 模式保持 byte-ready；可写 char-device fd 可以报告 `POLLOUT|POLLWRNORM`。pidfd 首片以 Linux 6.12 `fs/pidfs.c::pidfd_poll()` 为基准，只在绑定 child 已 exit 后对 read interest 报告 `POLLIN|POLLRDNORM`；未 exit pidfd 无 ready 时不得进入 TTY input wait，自身 waitqueue/poll_table 仍 deferred。可选 timeout 指针必须按 riscv64 `struct __kernel_timespec { s64 tv_sec; s64 tv_nsec; }` 复制和校验，非法 `tv_sec < 0` 或 `tv_nsec < 0 || tv_nsec >= 1e9` 返回 `EINVAL`。对照 Linux `do_poll()`，无 ready entry 时只有 `timeout={0,0}` 可以作为立即 timeout 返回 0；非 fixture 发行版路径上 `timeout=NULL` 且存在 TTY read interest 时可进入当前 `TtyInputWait` 首片，等待真实 UART RX interrupt 发布足以满足 `NTtyLineDiscipline` 的 ready-data 后重新计算 revents 并返回 ready count。受控 `user_smoke` fixture opt out 该等待窗口，仍把 no-ready `timeout=NULL` 返回 `ENOSYS` 作为 out-of-slice 回归；正数 timeout 仍 out-of-slice，不能伪装成成功 timeout。`sigmask != NULL`、临时 signal mask 安装/恢复、真实 waitqueue/poll_table、scheduler sleep、echo/erase、signal interruption/restart、remaining timeout 更新、`poll()`/`pselect6()`、大 `nfds` 分配和完整 fd refcount/locking 继续 deferred；超出当前固定小数组的 `nfds` 返回 `EINVAL` 而不是伪装成完整动态分配。
- `TtyInputWait` 的当前实现事实必须区分 Linux-like 等待对象生命周期和真实调度睡眠：`read(63)` no-ready 等待路径记录 N_TTY `tty->read_wait` entry/finish_wait 边界，`ppoll(73)` no-ready 等待路径记录 `poll_initwait/__pollwait` 注册和 `poll_freewait` 边界；二者仍只打开 supervisor interruptible 窗口并等待真实 UART RX 使 N_TTY readiness 成立。真实 task state、`wait_woken()`、`poll_schedule_timeout()`、signal interruption/restart 和完整 waitqueue 锁/refcount 继续 deferred，不得把当前自旋重试实现描述成 Linux 的 scheduler sleep。
- `PROBE=user-syscall-trace` 可输出稳定的 `tty wait trace` 行来观察 `TtyInputWait` 生命周期，字段至少区分 `kind=read|ppoll`、`event=enter|finish|freewait`、fd 和 ready/error 结果；该 trace 只能读取已记录对象事实，不得改变 wait 返回值、errno、checkpoint 顺序、user-smoke 判定或默认 `make test` 输出。
- `UserInitProcess` 表示 PID 1 的 `KernelInitTask` 经 `kernel_init -> run_init_process()/execve` 后获得的用户态进程身份视图，不表示新创建了第二个 task。`UserInitProcess.Setup` 必须依赖 `KernelInitTask.Online`、`UserAddressSpace.Online`、`ElfObject.Online`、`UserTrapFrame.Ready`、`FsStruct.Ready` 和 `FilesStruct.Ready`，并记录 PID 1 身份延续、同一 task_struct 复用、`KernelInitTask` 未被销毁、`FsStruct`/`FilesStruct` 继承、用户地址空间和 trap frame 绑定等事实。对照 Linux 6.12 `kernel/fork.c::copy_process()` / `copy_creds()` / `copy_sighand()` / `copy_signal()`，当前用户态 PID1 不是 fork 出来的新 task，因此第一片只在 `UserInitProcess` 上承载从当前 init task 继承出的 root credentials 子状态和单线程 `SignalRuntime` 折叠状态；规格边界仍必须区分 `ProcessSignalState`、`ThreadSignalState` 和 `SignalActionTable`，其中当前 blocked mask 属于 `ThreadSignalState.blocked`，`rt_sigaction` 读写 `SignalActionTable.action[sig - 1]`。为服务 Linux 6.12 `TIOCGPGRP/TIOCSPGRP`、`setpgid`、`getsid` 和 `setsid` 首片，`UserInitProcess` 还可以承载当前 PID1 作为 session leader、process-group leader、拥有 console-like controlling tty，且该 tty foreground process group 初始等于 PID1；plain fork/vfork 后还可以承载一个可见 child pid/pgrp/sid 的阶段性 job-control 视图，使 parent `setpgid(3,3)`、child continuation 中 `setpgid(0,3)` 和 `TIOCSPGRP(3)` 在同 session 内成功，并允许 observed getty child 在尚非 session leader 且不存在同 child pid 的 pgrp 时 `setsid(157)` 成功，把 child SID/PGID 改为 child pid。PID1 `setsid()` 仍因已是 session/process-group leader 返回 `EPERM` 且不得改写 PID1 SID/PGID；child 已是 session leader 或已有同 pid pgrp 时按 Linux `ksys_setsid()` 形状返回 `EPERM`；child 不可见或 child pid 为 0 保持明确 unsupported 边界。`getsid(156)` 首片只读取当前有界身份：`getsid(0)` 返回当前 syscall 身份的 SID，`getsid(1)` 返回 PID1 SID，当前 visible child pid 返回 child SID，未知 pid 返回 `ESRCH`。该视图只覆盖 PID1 与本轮 observed child，不实现完整 tasklist/RCU、security hooks、pid namespace、successful PID1 `setsid`、orphan pgrp、pty、controlling tty 解绑或 signal job-control。完整 `struct cred` COW/RCU、user namespace、capability、ucounts、LSM hooks、shared sighand/signal_struct、pending queues、signal delivery 和 siglock 并发协议必须保留为 deferred。`UserInitProcess.Enable` 是进入 U-mode 前的最后对象边界，必须在 `SyscallException.Online` 和 `SyscallTable.Ready` 后记录 syscall 上下文绑定和 trap return/satp handoff 事实。`UserInitProcess.Action::EnterUserMode` 是消费已就绪 `UserTrapFrame` / `UserAddressSpace` 并执行最终 trap-return handoff 的运行时 action，不负责制造 trap frame；真实 `write`/`exit` 观测只能由进入 U-mode 后的 `SyscallTable.Action::Write` / `Exit` / `ExitGroup` 生产路径 checkpoint 提交，不得在 Enable 中预写。不得在这里引入 fork/wait、完整 signal delivery、完整 scheduler 运行期或新的 task 生命周期对象。
- `rt_sigtimedwait(137)` 首片以 Linux 6.12 `kernel/signal.c::sys_rt_sigtimedwait()`、RISC-V `include/uapi/asm-generic/unistd.h::__NR_rt_sigtimedwait=137` 和 riscv64 `sizeof(sigset_t)==8` 为基准。实现必须校验 `sigsetsize == 8`，复制 `uthese` 中的 signal mask，并按 Linux sigset 规则用 `1 << (sig - 1)` 判断信号位；本片只支持 `SIGCHLD=17` 的 pending/dequeue/wake。`UserInitProcess` 必须记录 last wait mask、`uinfo` 是否为 `NULL`、`uts` 是否为 `NULL`、`pending_sigchld`、当前 mask 是否 match、是否进入 infinite wait、waiter 是否 enqueued、sleep reason、wake signal 和 dequeued signal。当前 OpenRC 观察为 `rt_sigtimedwait(uthese, NULL, NULL, 8)`；没有真实 child exit 时该形态不得返回成功、不得返回 `EAGAIN`、不得继续 `ENOSYS`，而必须记录 `SyscallTable.RtSigtimedwait` 与 `UserSignalWait.Sleep` checkpoint 和稳定诊断，显示 `uthese_copy=ok`、mask、`uinfo=NULL`、`uts=NULL`、`pending_sigchld=0`、`pending_match=0`、`waiter_enqueued=1`、`sleep_reason=rt_sigtimedwait/SIGCHLD/infinite`，然后停在可解释 waitqueue sleep 边界，不再推进到后续 `wait4(-1,NULL,WNOHANG,NULL)`/`nanosleep(1s)` 轮询。真实 `UserChildProcess` exit/exit_group 首片完成且 parent 是 PID1 时，才允许设置 parent pending SIGCHLD；若 PID1 已在该 waitqueue 中等待且 mask 匹配，必须记录 `UserSignalWait.WakeSigchld`，恢复 saved syscall frame，dequeue SIGCHLD，并通过 `SyscallTable.RtSigtimedwaitReturnSignal` 让 syscall 返回 17。若 PID1 尚未等待，只保留 pending SIGCHLD，下一次 matching `rt_sigtimedwait` 立即消费并返回 17。failed fork、unsupported `clone(220)` 和其它诊断边界不得伪造 SIGCHLD；已支持的 OpenRC `clone_flags=0x4111` 只有在 bounded child exit 后才产生 SIGCHLD。`uinfo != NULL` 的 siginfo copyout、`uts != NULL` 的 timeout/remaining timeout、restart、fatal signal、handler delivery、shared pending queues、多线程、多等待者和完整 signal delivery 继续 deferred；bad `sigsetsize` 按 Linux 返回 `EINVAL`，其它超出本片的 copyout/timeout 组合仍可走 unsupported diagnostic，但已支持的 OpenRC 形态不得打印 unsupported syscall。
- OpenRC 后续实测已经越过第一次 `clone(0x4111)` 的 bounded vfork completion，并在第二次及后续同形态 clone 反复停在 `clone_vfork stage=copy_user_process`。诊断显示旧 active child slot 已处于 `Ready`/parent resumed 之后的完成形态，`parent_frame_saved=1`、`child_handoff=1`、`current_child=0`、`child_pid=3`、`child_exit_status=-1`、`pending_sigchld=1`、`parent_clone_return=3`，而 `TaskCreationCore.CopyUserProcess` 只接受 `UserChildProcess.Prepared`。当前片已把 OpenRC vfork 推进为“单 active child execution slot + bounded completed-child records”：同一时刻仍最多一个 active child continuation，scheduler 内部仍只复用 `UserChild` task ref；每次 bounded vfork child exit/exit_group 完成 parent clone resume 后，必须把 user-visible child pid、raw exit status、Linux wait status、pidfd facts 和 reaped=false 归档到固定容量 records，释放 active slot 和 runqueue `UserChild` fact，把 `UserChildProcess` 复位到可再次 `CopyUserProcess` 的 Prepared-like 状态，并记录 `UserChildRecord.Archived`、`UserChildSlot.Reusable`。新的可复现边界是 OpenRC 连续顺序 vfork 的 completed records 达到固定容量后停在 `clone_vfork stage=child_records_full`，诊断为 `completed_records=8 record_capacity=8 active_slot_reusable=1 next_child_pid=11 first_unreaped_pid=0`；因此 record 容量必须解释为当前占用的未收割或诊断相关 record slot 数，而不是从启动以来的单调历史总数。`wait4` 成功收割 completed record 且 status copyout 成功后，必须在保留 `last_archived_*`、`last_reaped_*` 和 total archived/reaped/released 诊断事实的同时释放该 record slot，使后续顺序 vfork 可复用空 slot；若全部 slot 都被未收割 records 占用，仍必须停在 records-full 诊断边界且不得覆盖未收割 record。下一次顺序 `CLONE_VM|CLONE_VFORK|SIGCHLD` clone 必须分配递增 user-visible pid，复用同一个 internal `UserChild` execution slot，记录 `UserClone.VforkNextChildAccepted`，并用 total archived 或 slot reuse generation 证明不是第一 child；不得把 scheduler 单 task ref 伪装成完整 task graph。OpenRC login focused run 的旧实证边界是认证后 BusyBox `login` 在现有 getty/login child continuation 内调用同形态 `clone(0x4111)`，诊断为 `current_child=1 child_pid=6 active_slot_state=Ready active_slot_reusable=0 completed_records=0 completed_records_total_archived=3 completed_records_reaped=3 next_child_pid=7`，随后打印 `login: vfork: Function not implemented`。该 nested-vfork 片只允许这个单层 takeover：当前 UserChild 可作为 vfork parent 保存 clone frame，新 child 取得递增 user-visible pid、`a0=0` 和 `newsp`，复用同一 internal `UserChild` execution slot，不新增第二个 runnable task ref，不支持 pidfd；若已处于 nested child、当前 occupied completed-child records 已满、需要 parent/child 并发、需要多个 runnable user task refs、pidfd nested vfork 或更深嵌套，必须停在明确 unsupported/diagnostic 边界。后续 focused run 已越过该旧 `copy_user_process` 阻断，也确认登录后 fd-local `fchown(55)` / `fchmod(52)` 返回 0；新的 post-auth `socket(198)` 已分类为直接阻断。完整 socket/network、setgroups/credential 切换和 TTY ownership 语义仍不属于 nested-vfork 或 fd-local metadata 片。
- 当前 setuid 更新：OpenRC login 已越过 post-auth `socket(198)`、`/var/run/nscd/socket` `connect(203)` missing-path `ENOENT`、root-only `setgroups(159)` 首片和 `setgid(144, gid=100)` root-euid gid drop；本片前 focused 复跑的直接边界是 `setuid(146)`，参数为 `uid=1000`，旧行为返回 `EPERM`。本片只在当前 effective uid 为 0 时同步 `UserInitProcess.uid/euid/suid/fsuid` 到 32-bit 目标 uid，并保留完整 capabilities、user namespace、LSM、credential COW/RCU、权限检查、TTY ownership 和 inode ownership/mode 语义为 deferred。实现后 focused 复跑确认 `setuid(146, uid=1000)` 返回 0，login 继续经过 `chdir`、`.hushlogin` missing-path 和 MOTD 输出；后续登录 shell `execve(221)` 的 `EFAULT` 已由 `PROBE=user-syscall-error` 定位为 `filename_copy`，并在 execve C-string copy 片单独闭合。后续 staged `/bin/ls` 的普通 `clone(220)` child-continuation 边界已在 observed child plain-fork 片规格化；登录 shell foreground pgrp restore 的剩余错误由 bounded same-session child pgrp 片单独处理，不并入 setuid/execve 片。
- `clone(220)` / fork/vfork 首片以本地 Linux 6.12 `include/uapi/asm-generic/unistd.h::__NR_clone=220`、`include/uapi/linux/sched.h`、`kernel/fork.c::SYSCALL_DEFINE5(clone)` / `kernel_clone()` / `copy_process()` 和 `arch/riscv/kernel/process.c::copy_thread()` 为基准。真实 guest 观察之一来自 `ROOTFS_OVERLAY=none QEMU_APPEND='earlycon=sbi init=/bin/sh'` 后在 BusyBox prompt 输入 `ls`：早期 unsupported diagnostic 打印 `nr=220 a0=0x11 a1=0 a2=0 a3=0x8 a4=0x20096580 a5=1`。按 Linux legacy clone ABI，`a0 & CSIGNAL` 是 `SIGCHLD=17`，`a0 & ~CSIGNAL` 没有额外 `CLONE_*` flags；`newsp=0` 表示 child 继承 parent 用户栈指针；没有 `CLONE_SETTLS` 时不得使用 `a4` 改写 child `tp`，child 继承 parent TLS。
  OpenRC login shell 与 rc.local direct-inittab shell focused baselines 都观察到 child continuation 内 `/bin/ls` 发起同形态 plain fork：`clone_flags=0x11`、`newsp=0`、`current_child=1`、active slot 不可复用；rc.local 证据中该 shell 是 non-nested vfork child（`nested_vfork=0`、`child_pid=3`）。该形态是单-slot observed child plain fork：不进入 `TaskCreationCore.CopyUserProcess`，不在同一个 internal `UserChild` task ref 上再 enqueue 第二个 runnable child；clone 保存 shell parent pid、parent-of-parent pid、fork-time stack、fd snapshot 和 grandchild trap frame facts，并向 shell 返回 `next_child_pid`。只有 shell 后续 `wait4(-1, status, allowed_options, NULL)` 才 handoff 到 grandchild；grandchild exit 恢复 shell address-space、writable pages、fd snapshot 和 visible pid，再让 shell wait4 返回 grandchild pid。
  `PROBE=user-syscall-error` 的 clone diagnostic 必须输出稳定 `clone_kind=plain_fork|vfork_vm|vfork_pidfd|unsupported_shape`；非 SIGCHLD-only、`newsp != 0`、deeper nested child、pidfd、parent/child 并发或多个 runnable user task refs 等 unsupported child-context shape 必须输出稳定 `clone_plain stage=*`，并保留 flags、`flags_wo_csignal`、`exit_signal`、`newsp`、`current_child_continuation`、active slot state/reusable、current child pid、nested parent pid、next child pid 和 completed-record counters。unsupported diagnostic 只读现有 facts，不改变 checkpoint 顺序、trap frame 或用户内存。
  OpenRC `/sbin/init` 观察为 `clone_flags=0x4111`，实测诊断给出 `flags_without_csignal=0x4100`，即 `SIGCHLD | CLONE_VM(0x100) | CLONE_VFORK(0x4000)`，不是 `CLONE_PIDFD`；`newsp` 非零并设置 child sp。Linux 6.12 legacy clone 中只有真正包含 `CLONE_PIDFD(0x1000)` 时才使用 `parent_tidptr` 返回 pidfd；`kernel_clone()` 在 `CLONE_VFORK` 下先 `wake_up_new_task()`，再等待 child vfork completion 后才让 parent 返回。
  `SyscallTable.Action::Clone` 必须只做 ABI 解码、参数分类和 `kernel_clone_args` 形状事实。PID1 plain fork/vfork 和非 nested vfork 仍驱动 `TaskCreationCore.Action::CopyUserProcess` 建立正式 child task/process 边界，分配 PID；普通 PID1 fork/vfork 记录 parent=PID1，observed nested vfork 记录 parent=当前 UserChild pid；两者都记录 tgid=pid、exit_signal=SIGCHLD、files/fs/credentials/signal 继承、用户地址空间 snapshot 或等价首片事实、child trap frame 复制且 `a0=0`、TLS 继承。非 nested fork/vfork 按 `SelectRunQueue -> SetTaskCpu -> EnqueueTask` 的 wake_up_new_task 形状入队；nested vfork 只能复用已经 active 的 internal `UserChild` execution slot，不重复 enqueue 第二个 task ref。
  当前 plain fork 首片必须在 clone 时复制 bounded 用户栈 snapshot，并在 wait4 handoff 给 child continuation 前恢复该 snapshot，避免 parent 返回 child pid 并执行 wait4 期间复用同一用户栈页破坏 child 的返回链；后续 wait4 handoff 还必须保存 parent writable user pages 的页面内容，并在 observed child exit 后恢复，以覆盖缺少 Linux `dup_mm()`/COW 的父页污染。这只是单 observed-child 的阶段性 parent page rollback，不等价于完整 `dup_mm()`/COW mm。vfork+vm 首片不实现完整 vfork scheduler、共享 mm 并发或 task graph：clone 保存 parent clone frame/address-space/writable-page snapshot，然后直接切到 child clone-return continuation；bounded child `exit/exit_group` 后恢复 parent clone frame，使 parent `clone()` 返回 child pid，并产生真实 SIGCHLD pending/wake。真正含 `CLONE_PIDFD` 的 vfork shape 额外安装 pidfd-like fd table entry、将 fd 写回 `parent_tidptr` 并在 child exit 后标记 pidfd readable。除上述单层 nested takeover 和单 observed child plain fork 外，若 child 长驻或需要 parent/child 并发，必须停在新的诊断边界。
  parent syscall 返回 child pid；child 第一次运行时从 copied trap frame 的 syscall return continuation 继续。同一 guest 输入 `ls` 越过 `clone(220)` 后的 parent `wait4(260)` 观察为 `a0=-1 a1=0x3ffff77c a2=2 a3=0 a4=0 a5=0`。`wait4(260)` 首片必须以本地 Linux 6.12 `include/uapi/asm-generic/unistd.h::__NR_wait4=260`、`kernel/exit.c::SYSCALL_DEFINE4(wait4)` / `kernel_wait4()` / `do_wait()` 为基准：该参数形状表示等待任意 child、status 指针非空、`options=WUNTRACED`、`rusage=NULL`，Linux 会在 `kernel_wait4()` 内部追加 `WEXITED`。因为 clone 后 child 已存在但尚未产生 exit/stop/continue wait event，`do_wait()` 会到达 `wait_chldexit` interruptible wait 边界并调度其他可运行任务。当前首片第一段记录 parent wait 边界，在恢复 child stack snapshot 前保存 parent wait frame 和 parent address-space snapshot，再切换到 clone 已创建的 child trap-frame continuation，以暴露真实 child-side 下一边界；不得在 wait4 入口合成 child exit、不得声称完整 scheduler sleep/wakeup 已实现。wait4 handoff 必须保留长期诊断面：`SyscallTable.Wait4` checkpoint/KUnit 只读观察 parent wait saved frame、child frame 和 writable-page snapshot facts；若需要打印 copied child frame 的 `sepc`、`sp`、`gp`、`tp`、`a7` 和 `sstatus`，必须放在显式 `PROBE=user-syscall-trace` 或专门 wait4 trace probe 下，默认 `make run APP=user-boot` 和默认 shell smoke 不得输出 `wait4 child handoff` 详细诊断行。
  当前 wait4 首片实现后，同一 guest 不再停在 unsupported wait4，下一条观察为 child continuation 中 `a7=135` 附近的 instruction page fault；增强诊断显示 `satp` 匹配，`sepc/stval` 落在用户 ELF 的 writable non-executable 页，说明直接共享 parent/child 用户栈会污染 child continuation。后续 `PROBE=user-syscall-trace` 进一步定位到手工只输入 `ls` 时目录输出后紧接 child `/bin/ls` 的 `exit_group(94) status=0`，并且没有 parent wait4 返回或新一轮 shell `ppoll/read`；因此 child exit 首片必须按 Linux 6.12 `exit_group -> do_group_exit -> do_exit` 与 `wait_task_zombie()` 形状完成 observed-child wait 或 bounded vfork completion：wait4 child `exit/exit_group(status)` 不得触发全系统 shutdown，而应保存 wait status `(status & 0xff) << 8`、恢复 wait4 handoff 前保存的 parent address-space snapshot、向 parent status 指针写入 32-bit wait status、让 parent wait4 返回 child pid；vfork child `exit/exit_group(status)` 应保存 child exit status、恢复 clone 保存的 parent address-space snapshot、恢复 parent clone frame 并让 clone 返回 child pid；若该 clone 真实包含 pidfd，还必须标记 pidfd readable。wait status copyout 失败时 parent wait4 返回 `EFAULT`；vfork clone 的 pidfd copyout 在 clone 入口失败时返回 `EFAULT` 且不得创建 child。该首片只覆盖已由当前 wait4/vfork handoff 运行的 observed child，可把该 child 标记为已完成；PID1 自身 `exit/exit_group` 仍打印 `user exit status=N` 并 shutdown。新的真实 PTY 证据显示该路径已让 parent `wait4` 返回 3，但父 BusyBox shell 随后在 `sepc=0x10045318 stval=0x1` 用户态 load fault；`/bin/sh` 为 `/bin/busybox` symlink，BusyBox 是 `ET_DYN`，按 `USER_MAIN_PIE_LOAD_BIAS=0x10000000` 反查 objdump 得到 vaddr `0x45318` 是 stack canary 检查 `ld a5,0(s4)`。因此 wait4/exit 首片必须保留长期 checkpoint：`SyscallTable.Wait4` 记录 parent wait saved frame 的 `sepc/sp/s2/s4/a7/satp/status_ptr/child_pid`，`UserChild.ParentWaitResumed` 记录 child exit 后即将返回父用户态的 `sepc/sp/s2/s4/a0/a7/satp/status_ptr/wait_status/status_copied`；vfork 路径必须记录 `SyscallTable.CloneVforkVm` 或 `SyscallTable.CloneVforkPidfd`、`UserClone.VforkChildHandoff`、`UserClone.VforkParentResumed` 等稳定事实，真 pidfd shape 还记录 `FilesStruct.PidfdInstall` 和 `UserPidfd.Ready`。handler 只能只读这些 facts 并输出 KUnit diag，不得改变 syscall 返回值、TrapFrame、Context 或用户内存。当前 register checkpoint 已显示 saved/resumed 的 `s4` 均为 `0x1`，因此下一层诊断必须在同一 wait4 checkpoint 记录 parent wait 用户栈窗口：handoff 前保存 parent `sp..sp+512` 的 bounded bytes，child exit 恢复 parent address-space 后、wait status copyout 前比较同一窗口，输出 window start/len、diff count、first diff address 和 before/after byte；该比较只能用于定位是否为缺少 Linux `dup_mm()`/COW 隔离导致父栈页被 child continuation 污染，不得改变用户内存或 syscall 结果。若 prompt 返回后 shell 的 last status 仍异常，必须继续在同一 wait4 checkpoint 保存 parent writable user page checksum summary；当前可复现证据为 checked=525、truncated=0、dirty=4、stack_dirty=1、non_stack_dirty=3，首个非栈 dirty 页为 ElfSegment mapping index 1 / page 4 / vaddr 0x100c9000，证明 child fork/exec 路径污染了父进程非栈 writable 页。若这些 checkpoint 显示 `s4` 在 saved frame 已错误，应回溯 parent wait 入口；若 saved 正确但 resumed 错误，才允许修改 wait4/child-exit resume 路径；若栈窗口或其它 writable 页发生 child 写入导致的差异，修正必须按 Linux fork/mm 差分分类后再进入首片父页恢复或更正式的 mm 隔离。当前首片替代完整 Linux COW/mm 的实现顺序是：wait4 或 vfork clone handoff 前为每个 parent writable user page 分配 snapshot page 并复制完整页面内容，同时保存 checksum；child exit 恢复 parent address-space 后先比较/保留必要差异事实，再在恢复 parent syscall frame 前恢复全部 saved writable pages。该实现只覆盖当前单 observed child，不实现共享 PTE write-protect、page fault COW、mm refcount、VMA tree 或 task graph reaping。prompt-return 后 BusyBox 会再次调用 child 已收割后的 follow-up wait4；Linux 6.12 `kernel_wait4()` 对 wait4 options 追加 `WEXITED`，`__do_wait()` 无 eligible child 时以 `notask_error=-ECHILD` 返回。当前已观察到 `wait4(-1, status, options=0, NULL)` 和 `wait4(-1, status, options=3, NULL)`，因此当前首片在 observed child 已收割后，对 `upid=-1/rusage=NULL` 且 options 属于 Linux valid mask 的形状返回 `ECHILD`，不得返回 `ENOSYS` 污染 shell 命令状态；这仍不实现完整 wait 队列、其它 pid/type 或完整 reaping lifecycle。`clone3`、线程组、完整 CLONE_VM/vfork completion scheduler、COW mm、完整 pidfd file ops、ptrace/seccomp/cgroup/audit、namespace、robust futex、clear-child futex wake、完整 wait 睡眠/唤醒、完整 task graph/zombie lifecycle/release_task/资源累计、完整地址空间复制/COW、完整 copy_* 失败回滚和未观察到的 flags/options 组合必须进入 `UserCloneDeferredBoundaries` 或 `SyscallTable` wait4 facts，以 deferred、trimmed 或 unsupported-first-slice 分类记录，不能静默忽略。
- `wait4(-1, status_or_null, WNOHANG/valid_options, NULL)` 对 OpenRC vfork completed records 的优先级高于旧 active-slot 状态：若 bounded completed-child records 中存在未 reaped record，必须先完成 32-bit wait status copyout（`status != NULL` 时），copyout 成功后返回对应 user-visible child pid，记录 `UserChildRecord.Reaped`，并释放该 completed record slot、减少 occupied count、增加 released count；`status` copyout 失败时返回 `EFAULT`，不得标记 reaped，也不得释放 slot。`rt_sigtimedwait(SIGCHLD)` 消费 pending signal 不等价于 reaping，不能自动把 record 标记为已收割或释放 slot。若 active child 仍存在但未 exit，`WNOHANG` 返回 0；若没有未 reaped completed record 且没有 eligible active child，返回 `ECHILD`。completed records 容量固定，但只限制当前 occupied slots；容量耗尽时下一次 clone 停在 records-full 诊断边界，不覆盖未收割 record，也不返回伪成功。完整 zombie list、release_task、pid hash、资源累计、wait queues、多 child 并发和完整 task graph 继续 deferred。
- 加入 bounded stack snapshot/restore 后，同一 `/bin/sh -> ls` guest 已越过上述 instruction page fault；`PROBE=user-syscall-error` 定位新的直接失败源为 parent `setpgid(3,3)` 返回 `ESRCH`、child continuation `setpgid(0,3)` 返回 `EPERM`，随后 `/dev/tty` fd 上 `ioctl(TIOCSPGRP, 3)` 返回 `ESRCH`，BusyBox 因此报 `can't set tty process group: No such process` 并以 `user exit status=2` 退出。后续继续推进前，必须先对照 Linux 6.12 的 `setpgid/getpgid`、`TIOCGPGRP/TIOCSPGRP`、session/controlling tty 和 foreground process group 路径定位直接 errno 来源，不能从用户态错误文本直接猜修。
- `execve(221)` 首片以本地 Linux 6.12 `include/uapi/asm-generic/unistd.h::__NR_execve=221`、`fs/exec.c::do_execveat_common()/bprm_execve()/begin_new_exec()`、`fs/binfmt_elf.c::load_elf_binary()`、`fs/file.c::do_close_on_exec()` 和 RISC-V `arch/riscv/kernel/process.c::start_thread()` 为基准。Linux `include/linux/binfmts.h::struct linux_binprm` 持有 `struct mm_struct *mm`；`fs/exec.c::alloc_bprm()` 用 `kzalloc(sizeof(*bprm), GFP_KERNEL)` 分配 `linux_binprm`，`bprm_mm_init()` 通过 `mm_alloc()` 创建 nascent `bprm->mm` 和临时 argument stack，`begin_new_exec()` 在 point-of-no-return 后先按 close-on-exec bitmap 关闭 fd，再调用 `exec_mmap(bprm->mm)` 把新 mm 安装到 `current->mm/current->active_mm`，随后 RISC-V `start_thread()` 只更新 `pt_regs` 的 `epc/sp/status`。真实 `/bin/sh -> ls` child continuation 的稳定诊断为先调用 `execve("/bin/ls", argv={"ls", NULL}, envp={"SHLVL=1", "PWD=/", NULL})`，若返回 `ENOSYS` 再尝试 `"/usr/bin/ls"`；原生 OpenRC child continuation 的新稳定诊断为 `execve("/sbin/getty", argv0="/sbin/getty", argv1="38400", argv2="ttyN", envp includes TERM/PATH/SHELL/USER)`，其中 `/sbin/getty` 是 rootfs 绝对 symlink，必须经当前 `VfsCore.ReadPath` follow 到 `/bin/busybox`，并按 BusyBox `ET_DYN + PT_INTERP` PIE/dynamic ELF 进入同一 main/interpreter setup 路径，不得把它当作独立 getty ELF。当前首片只接受已经由 `wait4`/vfork handoff 进入的 child continuation、绝对路径、小 filename 和小 bounded argv vector，容量至少覆盖 OpenRC getty 的 `argv[0]="/sbin/getty"`、`argv[1]="38400"`、`argv[2]="ttyN"`、`argv[3]=NULL`，通过当前 `FsStruct.root`/`VfsCore.ReadPath` 读取 executable，复用 `ElfObject` 的 main/interpreter 解析、`UserStack` 初始栈、`UserAddressSpace` ELF/interpreter/stack/heap mapping 和真实页表安装逻辑构造替换地址空间；filename 和每个 argv 字符串的用户复制是 execve 专用 bounded C-string 语义：`user_ptr != 0`，逐字节用当前 `UserAddressSpace` 的 load-accessibility 事实检查 `user_ptr + index` 的单字节可读性，再读取该字节，读到 NUL 时返回实际长度；空字符串、地址加法溢出、任一已读取字节不可读或读满 `USER_PATH_MAX` 仍按当前 copy failure 处理。filename 指针/字符串复制失败必须记录稳定 `filename_copy` failure stage/reason 并返回 `EFAULT`，argv 指针或字符串复制失败必须记录稳定 `argv_copy` failure stage/reason 并返回 `EFAULT`，argv 超出当前 bounded 容量返回 `ENOSYS` 并打印稳定诊断；本片不引入完整 Linux `ENAMETOOLONG`、`MAX_ARG_STRINGS` 或 `ARG_MAX` 语义。该逐字节 C-string 修正只用于 `execve` filename/argv copy 和同源 execve 诊断前缀打印；共享 `copy_cstr_from_user()` 以及 `openat`、`chdir`、`newfstatat`、`readlinkat` 等非 execve path syscall 的现有 path-copy 行为不在本片改动范围内。不实现 `execveat`、PATH 搜索、脚本 binfmt、credentials、signals、files unshare 或完整 Linux stack randomization。长期 checkpoint 曾定位到 `/bin/ls` fault 只越过 `UserExec.MainElfReady`，尚未到 interpreter/address-space/satp，且 kernel `sp=0xffffffff808878c0/0xffffffff8088d8c0` 低于 `USER_KERNEL_TRAP_STACK=0xffffffff808f4010..0xffffffff808f5010`，因此 replacement `UserAddressSpace` 等大对象不得作为 syscall/trap 局部变量占用 4KB trap 栈；实现必须用 `Context` 拥有的 exec staging address space 对应 Linux 的 nascent `bprm->mm`，先构造 ELF/interpreter/stack/address-space 并允许为 `UserAddressSpace.Enable` 准备临时 `UserTrapFrame`，但运行期 checkpoint 提交顺序必须对齐 Linux 首片：`begin_new_exec()` point-of-no-return/context handoff 对应 `UserExec.ContextReplaced`，`exec_mmap()` mm/satp ready 对应 `UserExec.SatpReady`，RISC-V `start_thread()` 安装返回用 `pt_regs` 对应 `UserExec.TrapFrameReady`。成功路径必须在返回用户态前切换到新 `satp` 并执行 `sfence.vma`，同时把当前 trap frame改为 start_thread 形状：新 entry、新 sp、用户态 sstatus/FPU initial，且不再返回旧 syscall continuation。`argv[0]` 使用用户 argv[0] 字节而不是 filename；新用户栈按实际 bounded `argc` 写入 `argc`、每个 `argv[]` 指针、argv NULL、envp NULL 和现有 auxv。envp 首片只做 bounded 观测或 deferred，不把完整环境复制进新栈。当前只实现固定 16 槽 fd table 的最小 `do_close_on_exec()`：在 exec 成功替换地址空间时扫描所有 fd entry，清除 close-on-exec bit 为 true 的 entries，记录 scanned count、closed count、first closed fd 和 remaining open count；不实现 exec files unshare、文件 refcount、锁、`filp_close()`、延迟 fput 或失败回滚。成功 commit replacement address space 后必须把 `Context.user_exec_staging_address_space` 复位到可再次 `Preset` 的空 staging 对象；该复位只清除 staging 对象身份，不释放已经复制给 current user address space 的页。child exit 恢复 parent wait/vfork snapshot 前，若当前 child `satp` 与保存的 parent snapshot `satp` 不同，必须释放该已 exec 替换的 child `UserAddressSpace` 用户 mapping backing pages；这表达 Linux `exec_mmap()`/`mmput()` 方向的首片回收，同时避免释放仍由 vfork parent 引用的共享旧 mm。child exec 页表页、完整 `mm_struct`/VMA refcount 和 TLB shootdown 仍 deferred。pre-point-of-no-return 失败必须丢弃 nascent staging address space 并释放其尚未提交的 backing/page-table pages，避免失败后的 `Preset` 状态污染下一次 child exec 诊断。完整旧 mm 回收、point-of-no-return 失败回滚、credentials/LSM/security、signal handler reset、thread group de-thread、setuid/dumpability、task comm、rseq/perf/audit/accounting、robust futex、interpreter/script retry、execfd 和完整 binfmt 语义均 deferred；不得把本首片声称为完整 `do_execveat_common()`。
- `execve(221)` 首片必须具备长期阶段 checkpoint，而不是只依赖一次性日志定位返回路径问题。`SyscallTable.ExecveArgsReady`、`UserExec.MainElfReady`、`UserExec.InterpreterReady`、`UserExec.AddressSpaceReady`、`UserExec.ContextReplaced`、`UserExec.SatpReady`、`UserExec.TrapFrameReady`、`UserExec.SatpSwitched` 和 `UserExec.ReturnFrameReady` 分别观察 argv/path copy、主 ELF、interpreter、replacement address space、Context 对象替换、satp token、start_thread trap frame、live satp switch 和最终 trap frame 改写。`PROBE=user-boot` handler 可以只读这些 observation facts并输出 stage、filename 长度、argc、argv 总字节数、argv bounded 容量是否超出、主 ELF/interpreter entry/load-bias/segment 数、mapping 数、old/new/current satp、trap entry/sp/sstatus、trap frame 改写前后 `sepc/sp/ra/sstatus` 和当前 kernel `sp`；handler 不得修改 `Context`，不得分配或释放 page，不得改变 syscall 返回值、checkpoint 顺序或默认 `make test` 输出。`PROBE=user-syscall-error` 的 `execve` 诊断必须在 supported `EFAULT` 和 unsupported `ENOSYS` 路径上输出稳定 failure stage/reason/detail，至少区分 filename copy、argv copy、path read、argv capacity、main ELF preset/setup、interpreter read/setup、runtime interpreter bind、staging address-space preset、stack setup、address-space setup/enable、trap frame setup、close-on-exec、satp/frame return；detail 对 `ElfError` 失败输出稳定名称，并同时输出失败时 buddy free/total page 数、filename、argc/argv 总字节数、argv/envp 前缀、已记录的 image length、ELF type、PT_INTERP/解释器长度和 child continuation 事实；该诊断只能读取已经记录的 execve observation 或 best-effort 用户指针副本，不改变返回语义或默认非 probe 输出。若这些 checkpoint/diagnostic 显示 fault 发生在某阶段之后，后续实现修正必须基于该证据更新 roadmap/spec，而不能从症状直接猜补丁。
  Linux paired checkpoint 插桩必须区分 boot-time `kernel_init() -> run_init_process() -> kernel_execve()` 归属和用户态 syscall `execve()/execveat()` 归属，即使二者共享 `load_elf_binary()`、`begin_new_exec()`、`exec_mmap()`、`start_thread()` 或 `ret_from_exception` anchor。boot-time init exec 只能记录 `UserBoot.*`、`UserAddressSpace.Ready` 和单次 `UserInitProcess.EnterUserMode`；不得同时记录 `UserExec.*`。`UserExec.*` 只属于已经经由 `do_execveat_common()` 并越过 `SyscallTable.ExecveArgsReady` 的运行期用户 exec，例如 `/bin/sh -> /bin/ls` child continuation。`UserExec.SatpSwitched`、`UserExec.ReturnFrameReady` 和 `UserInitProcess.EnterUserMode` 还不得在每次普通 syscall 返回用户态时刷屏；若 Linux 实跑插桩暂时无法带有这种 ownership guard，对应事件必须留在 active paired case 的 hard scope 外，只作为 observed-but-not-compared 诊断证据。
  Linux paired checkpoint 插桩中，RISC-V `ret_from_exception` 对应的 `UserExec.SatpSwitched`、`UserInitProcess.EnterUserMode`
  和 `UserExec.ReturnFrameReady` 只能在 `PT_STATUS.SPP == 0` 的返回用户态路径记录。若这些 marker 位于 `ret_from_exception`
  的 kernel/user 汇合标签之后，会把内核态 trap return 误报为用户态事件；若位于 `restore_from_x6_to_x31` 或 `REG_L x2`
  之后再调用当前 assembly recorder，还会破坏即将返回用户态的 `t4`/`t5`/`t6` 或用户栈指针。因此 Linux 侧实跑插桩必须在
  通用寄存器恢复前完成记录，并以 `SR_SPP` guard 跳过 kernel return path。
- 发行版 `/bin/sh` delayed-input smoke 升级为真实外部命令闭包必须基于 reproducible observation，而不是从外部截断症状猜修。手工 PTY 路径已经证明 `ls` 可列出 Alpine rootfs 并返回 `user exit status=0`，非 PTY delayed-input 使用 `ls\nexit\n` 已两次观察到 rootfs 目录和 `user exit status=0`，DF-0003 使用显式 `/bin/ls\nexit\n` 完成 50/50 stress 成功。基于该证据，`make test` 的 distro shell case 固定输入 `/bin/ls\nexit\n`，覆盖 `/bin/sh` 派生 child 执行 `/bin/ls` 的路径，并同时断言 `lost+found` 和 `user exit status=0`；fork/wait4/execve 边界由 `SyscallTable.Clone`、`SyscallTable.Wait4`、`UserChild.ParentWaitResumed` 和 execve checkpoints/KUnit facts 覆盖。若该路径后续失败，必须使用成功/失败序列、现有 `PROBE=user-syscall-trace`、`PROBE=user-read-trace` 或新增长期 checkpoint 区分 harness EOF、TTY/N_TTY stdin、QEMU stdio、child stdout 和 child exit/wait4 边界；不得通过更换为其它输入命令形态、伪造 ANSI cursor-status response 或新增测试专用 kernel API 来试修。harness 仍不得在 shell-ready marker 前通过 pipe 或 stdin 预注入输入，以免污染 serial8250 initcall 诊断。
- `getpgid(155)` / `getsid(156)` / `setpgid(154)` / `setsid(157)` 进程身份首片以 Linux 6.12 `kernel/sys.c::do_getpgid()/sys_getpgid()/SYSCALL_DEFINE1(getsid)/sys_setpgid()/ksys_setsid()` 和 RISC-V `include/uapi/asm-generic/unistd.h::__NR_getpgid=155` / `__NR_getsid=156` / `__NR_setpgid=154` / `__NR_setsid=157` 为基准。`getpgid(0)` 必须读取当前 syscall 身份的 process group；当前 PID1 返回 1，wait4 handoff 后的 child continuation 返回 child pgrp。`getpgid(1)` 可作为 PID1 查找成功返回 1；observed child pid 已由 clone/vfork 创建且尚未释放身份视图时，`getpgid(child_pid)` 返回当前 child pgrp。其它正数 pid 或用户态传入的负数 pid 在当前无完整 task graph 的首片中按 Linux 查找失败形状返回 `ESRCH`。`getsid(0)` 必须读取当前 syscall 身份的 session id；PID1 current 返回 1，getty child continuation 在 successful `setsid()` 后返回 child pid。`getsid(1)` 返回 PID1 SID；当前 visible child pid 返回 child SID；未知 pid 或负 pid 返回 `ESRCH`。`setpgid` 必须保留 Linux 的 pid/pgid 零值归一化：`pid == 0` 表示当前 syscall 身份，`pgid == 0` 表示归一化后的 pid；当前首片接受 PID1 的 `setpgid(0,0)`、`setpgid(0,1)`、`setpgid(1,0)` 和 `setpgid(1,1)`，也接受 observed child 的 `setpgid(child_pid, child_pid)` 以及 child continuation 中的 `setpgid(0, child_pid)`，把 child pgrp 置为 child pid 并返回 0。负 `pgid` 返回 `EINVAL`，未知 `pid` 返回 `ESRCH`，未知目标 pgrp 返回 `EPERM`。`setsid` 必须保留 Linux `ksys_setsid()` 的首个保守失败规则：当前调用者 PID1 已经是 session leader/process-group leader，因此返回 `EPERM`；observed getty child 若不是 session leader，且不存在与 child pid 相同的现有 pgrp，则返回 child pid，并把 child SID/PGID 设为 child pid、标记 child session leader；重复 child `setsid()` 或 child 已有同 pid pgrp 返回 `EPERM`。当前首片不改变既有 PID1 session/pgrp 初始化事实，不实现 successful PID1 `setsid`，也不清除 controlling tty。该 syscall 读取/更新 `UserInitProcess` 的 process-group/session 首片视图，不落入 `FilesStruct` 或 TTY ioctl；完整 tasklist/RCU、`security_task_getpgid()` / `security_task_setpgid()` / `security_task_getsid()`、pid namespace、`PF_FORKNOEXEC/EACCES` 生命周期、多进程 process group、orphan pgrp、pty、controlling tty 解绑和 job-control signal delivery 继续 deferred。
- observed getty child 的 controlling-tty 首片以 Linux 6.12 `drivers/tty/tty_jobctrl.c::tiocsctty()/tiocgsid()` 为基准细化上一条边界：child 成功 `setsid()` 后只清除 child 继承的 controlling-tty fact，不改变 PID1 的 console-like controlling tty；随后 child 作为 session leader 且尚无 controlling tty 时，`ioctl(TIOCSCTTY, arg=1)` 可把当前 console-like TTY fd 绑定为 child controlling tty，并把 tty session id 设为 child SID、foreground pgrp 设为 child PGID。`ioctl(TIOCGSID, pid_t *)` 在当前 syscall 身份拥有该 console-like controlling tty 时写回对应 tty session id，否则返回 `ENOTTY`。本片不实现完整 tty steal/CAP_SYS_ADMIN、pty master/slave、`TIOCNOTTY`、真实 tty refcount/locks、session_clear_tty、多 session 竞争或完整 job-control signal。
- OpenRC login shell `/bin/ls\nexit\n` focused baseline 已越过 `/bin/ls` exec/wait：rootfs marker `lost+found` 输出，`wait4` 返回 observed grandchild pid 8，随后 shell 恢复 foreground process group 时 `ioctl(fd=10, TIOCSPGRP, &pgrp)` 返回 `ESRCH`，再调用 `setpgid(0, 6)` 返回 `EPERM`，最终 `user exit status=2`。本片只支持这个 bounded shell job-control 形态：当前 login shell/observed child 可保留或加入 inherited same-session child pgrp，`setpgid(pid=0, pgid=<inherited child pgrp>)` 是 same-session join/no-op；`TIOCSPGRP` 可把 foreground tty pgrp 切回 PID1 pgrp、当前 visible child pgrp 或该 shell inherited child pgrp。未知 pgrp 仍返回 `ESRCH`，跨 session pgrp 仍返回 `EPERM`。Observed grandchild handoff/restore 只切换 visible child pid；shell 的 inherited pgrp/session/controlling-tty facts 不得被 grandchild pid 覆盖。本片不实现任意多 pgrp、pgrp lifetime、orphan pgrp、后台 signal、pty、完整 tasklist lookup 或通用 job-control。
- 当前 rootfs 输入是 whole-disk ext2；`UserBootPayload` 不应要求 `PartitionTable` / `BlockPartition`。若未来磁盘镜像切换为带分区表，再补分区对象建模。
- 当前 `SyscallTable` 覆盖 `write(1/2, user_buf, len)`、`writev(1/2, iov, iovcnt)`、read-only `openat/read/close/newfstatat` 首片、directory-capable `openat(AT_FDCWD, path, O_RDONLY|O_DIRECTORY)` 和 `getdents64(61)` 首片、`openat(AT_FDCWD, "/dev/tty" 或 "/dev/tty[0-9]+", O_RDWR|O_NONBLOCK|O_LARGEFILE)` console-like char-device 多 fd entry 首片、`ppoll(73)` 立即 readiness 首片、`readlinkat(78)` fast-symlink 首片、fd-local `fchmod(52)` / `fchown(55)` 首片、`getrandom(278)` 小 buffer 首片、`getuid(174)` / `getgid(176)` / `setuid(146)` root-euid bounded uid drop 首片、`setgid(144)` root-euid bounded gid drop 首片、`setgroups(159)` root bounded supplementary groups 首片、`getgroups(158)` bounded supplementary groups readback 首片、`getpid(172)` / `getppid(173)` / `getpgid(155)` / `getsid(156)` / `setpgid(154)` / `setsid(157)` PID1/observed child pid/pgrp/sid 身份首片、`geteuid(175)` / `getegid(177)` / `getresuid(148)` / `getresgid(150)` credentials 读取首片、`uname(160)` static new_utsname 首片、`getcwd(17)` root cwd 首片、`rt_sigprocmask(135)` blocked-mask 首片、`rt_sigaction(134)` signal action table 首片、`rt_sigtimedwait(137)` OpenRC infinite wait boundary 首片、`ioctl(TIOCGWINSZ/TCGETS/TCSETS/TIOCGPGRP/TIOCSPGRP)` TTY 读取/termios 更新/foreground-pgrp 更新首片、`clock_gettime(113)` / `gettimeofday(169)` 时间读取首片、`nanosleep(101)` bounded short relative sleep 首片、dynamic loader 所需的 `brk/mmap/mprotect/munmap`、`set_tid_address(clear_child_tid)`、`clone(220)` plain-fork 首片、`wait4(260)` parent wait/child continuation 和 `wait4(-1,NULL,WNOHANG,NULL)` 非阻塞 no-waitable/no-child 首片、`execve(221)` child continuation ELF reload 首片，以及 `exit/exit_group(status)`。`write/writev` 通过受限 `UserCopy` 从当前 `UserAddressSpace` 复制用户字节，然后必须经 `FilesStruct.Action::LookupFd -> FileDescriptorTable.Action::Lookup -> OpenFileDescription.Action::Write -> FileBackend.Action::WriteCharDevice` 路径写入现有 printk/serial console；不得继续在 `SyscallTable.Action::Write` 或 `Writev` 内直接用 `fd == 1 || fd == 2` 绕过 fd table。`writev` 首片只支持有限数量的小 iovec，并按 iovec 顺序复用既有 fd 写路径。`openat/read/close/newfstatat/getdents64/readlinkat/ppoll/fchmod/fchown` 必须经 `FilesStruct`、`FileDescriptorTable`、`OpenFileDescription`、`FileBackend` 和当前 VFS path API 中适用的对象边界；`readlinkat` 不经过 fd install/read 路径，但仍不得在 syscall table 中直接裸读 ext2 或为测试专门构造文件/目录内容；`fchmod/fchown` 不经过 path walk 或 inode writeback，必须只记录当前 fd entry 的阶段性 metadata。`getrandom` 不得经过 `FilesStruct`、`VfsCore`、`DevFs` 或 `/dev/random`/`/dev/urandom` 路径；首片必须经现有 Linux-like `HwRngCore.read_current()` 当前设备边界读取，当前 provider 是 virtio-rng。`getpid` 必须返回当前 syscall 身份的 thread-group id：PID1 current 返回 1，visible child continuation 返回 child pid，缺少可见 child 身份时保持明确 unsupported 边界；`getppid` 必须按 Linux 6.12 `rest_init()` / `copy_process()` 中 PID1 的 boot idle/init_task 可见父进程语义返回 0，而不是临时返回 PID1 或 kthreadd；`getsid` 必须读取同一 PID1/observed child session 身份视图；`setsid` 当前只允许 PID1 已是 session/process-group leader 的 `EPERM` 和 observed getty child 成功成为 session/pgrp leader 的首片，不实现 PID1 成功 setsid 或 controlling tty 解绑。`geteuid/getegid/getresuid/getresgid` 必须从同一 credentials 子状态读取，`getres*` 按 riscv64 `uid_t/gid_t` 用户布局依次写 real/effective/saved id；`getgroups` 只回读 `setgroups` 首片保存的固定容量 supplementary group view。`uname` 必须按 Linux 6.12 `sys_newuname()` 和 `struct new_utsname` 六个 65 字节字段复制本地 `../linux-6.12` 生成的 static init UTS 值；`getcwd` 必须经继承的 `FsStruct.root/pwd` 判断当前 root cwd，当前首片只返回 `"/\0"` 且返回值包含结尾 NUL。`openat` 参数校验以 Linux 6.12 `fs/open.c::build_open_flags()`、`include/uapi/asm-generic/fcntl.h`、`drivers/tty/tty_io.c::tty_open()` 和 `drivers/tty/tty_port.c::tty_port_block_til_ready()` 为基准：`O_RDONLY/O_WRONLY/O_RDWR` 是 access mode，不得在复制 path 之前当作非法 flags；`O_NONBLOCK=00004000` 是 file status flag，可被 TTY open 路径消费并保留到 opened file flags。当前普通 regular/directory 路径仍只支持 read-only 打开，写模式返回超出首片的权限/能力错误，`O_NONBLOCK` 只允许 TTY char-device 路径消费；`/dev/tty` 与 `/dev/tty[0-9]+` 特例支持 `O_RDWR|O_NONBLOCK|O_LARGEFILE`，在固定 fd table 中分配最低空闲 fd entry（stdio 未关闭时为 3..15），并让这些 fd entry 共享同一个 console-like `Tty0` 后端。可以接受 libc 在 64-bit 路径中自动附带的 `O_LARGEFILE` 和 `O_CLOEXEC`，但仍必须拒绝创建、truncate、append、`O_TMPFILE`、相对 dirfd 等超出首片的 flags；普通 follow 模式下遇到 fast symlink 必须由 VFS path walk 解析后再打开目标。`AT_FDCWD` 下的 path walk 当前除绝对路径外，还必须支持从 `FsStruct.pwd` 出发解析 `"."` 和不含 `/` 的相对单组件；这是 Linux 6.12 `fs/namei.c` path walk 在 dirfd 为 `AT_FDCWD` 时使用当前工作目录的首片，用于支撑 BusyBox `/bin/ls` 的 `newfstatat(AT_FDCWD, ".", ...)`。`getdents64` 首片对齐 Linux 6.12 `fs/readdir.c::sys_getdents64()/iterate_dir()` 和 `fs/file.c::fdget_pos()` 的核心语义：从 fd table 取得 directory open file description，使用 fd position 作为目录 byte offset，序列化 `linux_dirent64` 记录，成功返回复制字节数，并把 offset 推进到最后返回的 ext2 dirent 后；用户缓冲不足以放下下一条完整记录时停止在已完成记录边界。首片只覆盖当前 ext2 read-only direct-block 目录、`DT_REG/DT_DIR/DT_LNK/DT_UNKNOWN`、单个 filesystem fd slot 和 `USER_COPY_MAX` 内的部分填充；Linux 的 `i_rwsem`、`security_file_permission()`、fsnotify/file_accessed、dead directory 检查、mount namespace、RCU/`f_pos_lock` 竞争和完整 errno 细节必须记录为 deferred/trimmed。`brk/mmap/mprotect/munmap` 必须路由到 `UserAddressSpace` 的阶段性 heap/mmap arena；`mmap` 首轮只要求 anonymous private 映射，`mprotect`/`munmap` 可先作为已映射用户区间的成功边界。`set_tid_address` 必须落到当前 `UserInitProcess` 的 `clear_child_tid` 属性，返回 PID1，不得伪造第二个 task 或引入线程组/futex 完整语义。该路径仍不等价于完整 devtmpfs、`/dev/console` 文件、真实 TTY/N_TTY、多个 TTY 实例、TTY driver registry、controlling tty 分配、N_TTY 非阻塞读完整语义、job-control 完整语义、完整 fork/wait/exec、完整 signal delivery、普通文件 `write`、权限检查、slow symlink/nofollow、完整 poll waitqueue、完整 VMA tree、文件映射、完整 fd 生命周期管理（除本片明确的 close、close-on-exec 扫描和 fd-local chmod/chown metadata）、相对 dirfd、`..` 或多组件相对 path。
- `mmap(222)` 首片以 Linux 6.12 RISC-V `arch/riscv/kernel/sys_riscv.c::SYSCALL_DEFINE6(mmap)`、`mm/mmap.c::ksys_mmap_pgoff()/do_mmap()/mmap_region()` 和 `include/uapi/asm-generic/mman-common.h` / `include/uapi/linux/mman.h` 为基准：64-bit `offset` 必须按页对齐校验，`len == 0` 返回 `EINVAL`，`MAP_ANONYMOUS` 路径不使用 fd，`MAP_PRIVATE|MAP_ANONYMOUS` 仍是当前普通分配首片。真实 BusyBox `/bin/sh` 证据显示 musl 会请求 `addr=USER_HEAP_BASE, len=4096, prot=PROT_NONE, flags=MAP_PRIVATE|MAP_FIXED|MAP_ANONYMOUS, fd=-1, offset=0`；当前兼容首片不消费 fd，只在请求完全落入已预映射 anonymous arena、页对齐且为该 PROT_NONE fixed anonymous private 形状时返回固定地址成功，用于表达 Linux `MAP_FIXED` 可在目标地址建立 VMA 的用户可见结果。该首片不得声称已实现完整 `MAP_FIXED` 覆盖 unmap、PROT_NONE PTE、VMA tree 合并/拆分、ASLR area selection、file-backed mmap、overcommit/accounting、`MAP_FIXED_NOREPLACE`、`MAP_SHARED`、`MAP_HUGETLB`、`MAP_GROWSDOWN/GROWSUP`、populate、mmap locks、LSM/security、`mmap_write_lock` 或完整 errno；这些必须保持 deferred/trimmed。
- `openat` 的 `O_CLOEXEC`、fd dup 和最小 close-on-exec 首片以 Linux 6.12 `include/uapi/asm-generic/fcntl.h`、`include/uapi/linux/fcntl.h`、`include/uapi/asm-generic/unistd.h`、`fs/open.c::build_open_flags()`、`fs/file.c::alloc_fd()/ksys_dup3()/do_dup2()/do_close_on_exec()` 和 `fs/fcntl.c::do_fcntl()/f_dupfd()` 为基准。当前必须接受 `O_CLOEXEC=02000000`，并把它作为 fd table close-on-exec bit，而不是 `struct file` status flag；`fcntl(F_GETFL)` 不得返回 `O_CLOEXEC`。BusyBox `/bin/ls` 当前会调用 `openat(AT_FDCWD, ".", O_DIRECTORY|O_CLOEXEC|O_LARGEFILE)`，该路径必须仍走 directory-capable `openat` 和 `FsStruct.pwd` 相对 path walk 首片。`fcntl(F_GETFD/F_SETFD/F_DUPFD/F_DUPFD_CLOEXEC)` 首片必须经 fd table：`F_GETFD` 返回当前 close-on-exec bit，`F_SETFD` 只消费 `arg & FD_CLOEXEC` 设置或清除该 bit并返回 0；`F_DUPFD` 从 `arg` 指定的最低 fd 起查找空槽，复制原 fd entry 指向同一 `OpenFileDescription`，new fd 的 close-on-exec 为 false；`F_DUPFD_CLOEXEC` 同样复制 entry，但 new fd 的 close-on-exec 为 true。若随后关闭原 fd，而 duplicate 仍指向同一 staged `Regular0` OFD，close 不得清除共享的 regular 文件长度、offset、kind 或路径元数据；这些单槽元数据只能在最后一个指向该 OFD 的 fd alias 被关闭时释放。无效 fd 返回 `EBADF`，`arg` 超出当前固定 fdtable 容量返回 `EINVAL`，容量内无空槽返回 `EMFILE`，未知 cmd 仍返回 `EINVAL`。`dup3(24)` 首片支持 `dup3(oldfd,newfd,flags)`：`flags` 只允许 `0` 或 `O_CLOEXEC`，其它 flags 返回 `EINVAL`；`oldfd == newfd` 返回 `EINVAL`；无效 `oldfd` 或 `newfd >= FILE_FD_COUNT` 返回 `EBADF`；成功时 `newfd` 指向与 `oldfd` 相同的 OFD/backend，继承 readable/writable/status flags，close-on-exec bit 由 `flags & O_CLOEXEC` 决定；若 `newfd` 已打开，当前只替换该 fd table entry。该首片不实现 Linux 的 `expand_files()`、rlimit、`EBUSY` larval fd 检测、`get_file()`、`filp_close()`、`fput()`、refcount 或并发 fdtable 锁。runtime `execve` 成功时扫描当前固定 fd table 并关闭 close-on-exec bit 为 true 的 entries。新的可复现失败证据为 child `/bin/ls` execve close-on-exec 关闭 fd10 后，parent `/bin/sh` 的 `ioctl(fd=10,TIOCSPGRP)` 返回 `EBADF`；根因是当前单 runtime `FilesStruct` 被 child close-on-exec 污染。首片必须在 plain-fork/vfork child handoff 前保存 parent fd table 与单 regular/pidfd 槽元数据快照，在 child exit 返回 parent 前恢复该快照，使 child close-on-exec 不清掉 parent fd entry。这是 observed single-child 的 files rollback，不是完整 `copy_files()`、`CLONE_FILES`、fdtable refcount 或 OFD 生命周期模型；动态扩容 fdtable、file refcount、`dup(23)`、`dup2`、并发 fdtable 锁和完整 files unshare 继续 deferred。
- Credentials 首片以 Linux 6.12 `kernel/sys.c::__sys_setuid()/__sys_setgid()/sys_getuid()/sys_getgid()`、`kernel/cred.c::prepare_creds()/commit_creds()` 和 RISC-V `include/uapi/asm-generic/unistd.h` 为基准。`getuid()` 和 `getgid()` 必须从当前 `UserInitProcess` credentials 子状态返回当前 real uid/gid，当前 root init 初值为 0。`setuid(uid)` 和 `setgid(gid)` 必须路由到同一子状态；`setuid` 当前片在 credentials ready 且当前 effective uid 为 0 时接受 32-bit uid 目标，并同步 real/effective/saved/fs uid；`setgid` 当前片在 credentials ready 且当前 effective uid 为 0 时接受 observed 32-bit gid 目标，并同步 real/effective/saved/fs gid。当前以 `euid == 0` 作为阶段性 `CAP_SETUID` / `CAP_SETGID` 代理，不实现 capabilities、user namespace、LSM、`prepare_creds()` 分配失败、`set_user()`/ucounts、RCU/cred refcount、完整 saved-id 权限矩阵或其它权限检查；非 root effective uid 的 `setuid`/`setgid` 返回 `EPERM`，无效 id 后续再补完整 `EINVAL` 范围规则。`setuid(1000)` 成功后当前进程有意失去 root，不支持通过 saved uid/capability regain 恢复 root。
- `rt_sigprocmask` 首片以 Linux 6.12 `kernel/signal.c::sys_rt_sigprocmask()/sigprocmask()/set_current_blocked()` 和 RISC-V `__NR_rt_sigprocmask=135` 为基准。实现顺序必须保持：先校验 `sigsetsize == sizeof(sigset_t)`，当前 riscv64/uapi 为 8 字节；读取当前 `UserInitProcess.blocked_signal_mask` 作为 old mask；若 `nset != NULL`，先从用户态复制 8 字节 mask，清除 `SIGKILL` 与 `SIGSTOP`，再按 `SIG_BLOCK=0`、`SIG_UNBLOCK=1`、`SIG_SETMASK=2` 更新当前 blocked mask，未知 `how` 返回 `EINVAL`；若 `oset != NULL`，向用户态写回 old mask。用户指针复制失败返回 `EFAULT`。完整 shared sighand、`spin_lock_irq(&sighand->siglock)`、pending/recalc、restartable syscall、signal delivery、thread group、compat sigset 和更大 sigset 变体继续 deferred/trimmed。
- `rt_sigaction` 首片以 Linux 6.12 `kernel/signal.c::sys_rt_sigaction()/do_sigaction()`、`include/uapi/asm-generic/signal.h`、`arch/riscv/include/uapi/asm/signal.h` 和 RISC-V `__NR_rt_sigaction=134` 为基准。实现顺序必须保持：先校验 `sigsetsize == sizeof(sigset_t)`，当前 riscv64/uapi 为 8 字节；若 `act != NULL`，按 riscv64 kernel ABI 布局 `{ handler: usize, flags: usize, mask: usize }` 从用户态复制；拒绝无效 signal number 和对 `SIGKILL`/`SIGSTOP` 安装新 action；读取旧 `SignalActionTable.action[sig - 1]`；若 `act != NULL`，清除未支持 userspace flags 并从 action mask 中清除 `SIGKILL`/`SIGSTOP` 后存入表；若 `oact != NULL`，按同一布局写回旧 action。用户指针复制失败返回 `EFAULT`，无效 signal/sigsetsize/kernel-only set 尝试返回 `EINVAL`。当前单 PID1 实现可把 `SignalRuntime`/`SignalActionTable` 折叠进 `UserInitProcess`，但必须保留对象事实；完整 sighand 共享、siglock/RCU、ignored-signal pending flush、handler delivery、signal frame 和 `rt_sigreturn` 继续 deferred/trimmed。
- `rt_sigtimedwait(137)` 的历史 unsupported 诊断已经定位 OpenRC 参数形态为 `uthese=a0=0x100c9350`、`uinfo=a1=NULL`、`uts=a2=NULL`、`sigsetsize=a3=8`，并确认其后才进入 `wait4(-1, NULL, WNOHANG, NULL)` 与 `nanosleep(1s)` 轮询。本片推进后，该 OpenRC 形态必须进入 supported 分发表并记录 `SyscallTable.RtSigtimedwait` 与 `UserSignalWait.Sleep` wait boundary；unsupported diagnostic 只保留给 `uinfo != NULL`、`uts != NULL` 或其它尚未规格化组合。当前 OpenRC 若只有 unsupported clone 而没有真实 child exit，验收是 `pending_sigchld=0`、`waiter_enqueued=1` 的真实 sleep 边界，而不是强制继续运行；不得把 failed clone 当成 SIGCHLD source。不得把 OpenRC 当前触达的 `137` 误标为 `rt_sigsuspend(133)`，也不得再对该形态返回 `ENOSYS`。
- OpenRC child record 回收、`F_SETFL` 首片和 TTY alias 多 fd entry 首片后，`make run APP=user-boot ROOTFS_OVERLAY=none QEMU_APPEND=earlycon=sbi PROBE=user-syscall-trace,user-syscall-error FORCE=1` 的可复现边界已越过 `/dev/ttyN` `openat(AT_FDCWD, "/dev/ttyN", O_RDWR|O_NONBLOCK|O_LARGEFILE)` 旧的 `access_mode+unsupported_flags` 拒绝、首个 TTY fd 上的 `fcntl(F_SETFL, O_NONBLOCK)`，以及后续 `/dev/tty2`、`/dev/tty3` 等 alias open 的 fd 3 单槽 `FileError::AlreadyOpen`/`EMFILE` 阻断。上一片开始前的直接证据是 OpenRC 子路径在 `openat("/dev/ttyN", O_RDWR|O_CLOEXEC|O_LARGEFILE)` 前调用 `close(0)`，当时实现把该 supported syscall 子路径错误退成 `ENOSYS`，随后多个 O_CLOEXEC TTY alias entry 留在固定 fd table 中，最终返回 `FileError::TooManyOpenFiles`/`EMFILE` 并报 `can't open /dev/ttyN: No file descriptors available`。上一片行为只解除该 fd 生命周期边界：允许关闭 fd0/1/2 和其它当前已打开 fd，TTY open 可复用被关闭的低号空槽，并在 runtime `execve(221)` 成功时执行固定表最小 close-on-exec 扫描；上一片不扩容 fd table，不实现完整 files unshare/fork 文件表复制、devtmpfs、`/dev/null`、真实 `/dev/console`、多个 TTY 实例、TTY driver registry、controlling tty 分配、N_TTY 非阻塞读完整语义或 job-control 完整语义。若之后第一阻断转到 `/dev/null`、stdio 重定向、`ioctl`、`read`、`ppoll`、`nanosleep`、`wait` 或其它 syscall 新边界，必须保留精确诊断并在下一片基于证据规格化。
- `user-smoke` 应包含独立的 signal syscall case，覆盖 `rt_sigprocmask` 和 `rt_sigaction` 的首片 ABI、old-value writeback、SIGKILL/SIGSTOP mask 清理和错误码；发行版 `/bin/sh` 前置探针可以保留最小“syscall rt_sigaction ok”标记，但不应承载全部 signal 回归。真实 signal handler delivery、用户栈 signal frame 和 `rt_sigreturn` 未规格化前，不得把 handler 被调用作为通过条件。
- `user-smoke` 应包含独立的 stdin syscall case，用于验证 fd0 `read(63)` ready-data 首片，并在同一固定 ready data 被消费前验证 `ppoll(73)` 的立即 readiness 首片。该 case 只能消费默认 overlay 进入用户态前准备的固定 ready data，并输出清晰的 `syscall ppoll stdin ok` 与 `syscall read stdin ok` marker；消费该 fixture 后还必须验证 fd0 非零长度 no-ready `read(63)` 和 blocking `ppoll(73)` 都显式返回 `ENOSYS` out-of-slice，避免回归为 EOF/timeout 成功。它不得依赖人工键盘输入、第二个用户任务或真实 blocking read 前进，也不得把这个 case 移入 `fileio` 的 regular/directory VFS 回归中。
- Time syscall 首片以 Linux 6.12 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_clock_gettime=113`、`__NR_gettimeofday=169`，`kernel/time/posix-stubs.c::sys_clock_gettime()` 和 `kernel/time/time.c::sys_gettimeofday()` 为基准。`clock_gettime(clockid, tp)` 必须先按 Linux `do_clock_gettime()` 语义处理 clock id：当前首片支持 `CLOCK_REALTIME=0` 和 `CLOCK_MONOTONIC=1`，超出首片的 id 返回 `EINVAL`；成功后向用户态写出 riscv64/uapi `struct __kernel_timespec { s64 tv_sec; s64 tv_nsec; }`，用户指针不可写返回 `EFAULT`。`gettimeofday(tv, tz)` 必须允许 `tv == NULL` 或 `tz == NULL`；`tv != NULL` 时写出 riscv64/uapi `struct __kernel_old_timeval { long tv_sec; long tv_usec; }`，`tz != NULL` 时写出 `struct timezone { int tz_minuteswest; int tz_dsttime; }` 的当前零值首片；任一用户写失败返回 `EFAULT`。当前时间值复用已建模的 `Timekeeper` wall/monotonic ready 事实和 `RiscvTimerProvider.read_time()` / timebase，将 timer tick 转换为秒、纳秒或微秒；`CLOCK_REALTIME` 暂用同一启动后基准，不实现 RTC/NTP/wall-clock 校准。完整 POSIX timer/vDSO、`CLOCK_BOOTTIME`、coarse/raw/process/thread clocks、time namespace、seqcount retry、timezone 修改、settimeofday、leap second、clocksource 质量与高精度 timekeeping 继续 deferred/trimmed。
- `nanosleep(101)` 首片以 Linux 6.12 `include/uapi/asm-generic/unistd.h::__NR_nanosleep=101`、`kernel/time/hrtimer.c::SYSCALL_DEFINE2(nanosleep)`、`kernel/time/time.c::get_timespec64()` 和 `include/linux/time64.h::timespec64_valid()` 为基准。Linux 64-bit 顺序是先从 `rqtp` 复制 16 字节 `struct __kernel_timespec { s64 tv_sec; long long tv_nsec; }`，复制失败返回 `EFAULT`；再校验 `tv_sec >= 0` 且 `0 <= tv_nsec < 1_000_000_000`，失败返回 `EINVAL`；成功后设置 restart block，并以 `CLOCK_MONOTONIC` 相对时间进入 `hrtimer_nanosleep()`。真实 `/bin/sh` unsupported 诊断已定位请求为 `req_sec=0, req_nsec=20_000_000`，`rmtp` 与 `rqtp` 相同，因此当前首片支持 `0 <= duration <= 100ms` 的相对短睡眠：用 `Timekeeper`/`RiscvTimerProvider.read_time()` 和 timebase 计算目标 tick 并忙等到期，成功返回 0 且不写 `rmtp`，这对应 Linux 完成睡眠路径。负秒、负纳秒或 `nsec >= 1e9` 返回 `EINVAL`，`rqtp` 不可读返回 `EFAULT`；超过 100ms、timer provider 不可用或 tick 计算溢出返回 `ENOSYS` 作为 out-of-slice，而不是伪装成 Linux-invalid。unsupported diagnostic 中的 `nr=101` 参数打印保留用于未来定位更大需求，但一旦 nanosleep 进入 supported 分发表，真实错误应走 supported-syscall error 诊断。`clock_nanosleep`、signal interruption/restart、remaining-time copyout、timer slack、真实 waitqueue/scheduler 集成和多任务 sleep 继续 deferred。
- Metadata/fd 首片以 Linux 6.12 `fs/stat.c::vfs_fstatat()/newfstatat()`、`fs/stat.c::vfs_fstat()/newfstat()`、`fs/namei.c` path walk 和 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_newfstatat=79`、`__NR_fstat=80` 为基准。`newfstatat(AT_FDCWD, absolute_path, statbuf, 0)` 必须通过当前 `FsStruct.root`、VFS path walk 和 ext2 inode metadata 返回 regular file 或 directory 的最小 `struct stat`；`newfstatat(AT_FDCWD, ".", statbuf, 0)` 必须从当前 `FsStruct.pwd` 返回当前工作目录 metadata；不含 `/` 的相对单组件从 `FsStruct.pwd` 查找；BusyBox `/bin/ls` 当前需要的 `"./component"` 必须按 Linux path walk 从 `FsStruct.pwd` 处理 `"."` 后继续查找 final component。`AT_SYMLINK_NOFOLLOW=0x100` 必须按 Linux 6.12 `vfs_fstatat()` / statx lookup flags 只禁止 final component follow：final 是 fast symlink 时返回 symlink 自身的最小 metadata `S_IFLNK|0777` 和 symlink target size；没有 nofollow 时仍按普通 path walk follow fast symlink 后返回目标 metadata。`newfstatat` 当前只接受 flags `0` 或 `AT_SYMLINK_NOFOLLOW`，其它 flags 返回 `EINVAL`。metadata 中 mode 至少区分 `S_IFREG|0444`、`S_IFDIR|0555` 与 `S_IFLNK|0777`，size 取 VFS/ext2 inode size；不再只通过 read file 长度推导 regular file metadata。`fstat(fd, statbuf)` 必须通过 `FilesStruct -> FileDescriptorTable -> OpenFileDescription` 找到当前 fd，对已打开 regular file 和 directory fd 返回同一类最小 metadata，并按 Linux 无效 fd 返回 `EBADF`。本首片仍裁剪 device id、inode number 精确值、uid/gid、时间戳、link count 精确值、statx attributes、`AT_EMPTY_PATH`、automount、mount id、LSM/security hooks、RCU/refcount、permission checks、`..`、任意多组件相对 path 和完整 errno；这些必须作为 deferred/trimmed 语义保留，不能在实现里静默假装完整。
- Fd API 兼容首片以 Linux 6.12 `include/uapi/asm-generic/fcntl.h` 的 `F_GETFL=3`、`F_SETFL=4`、`O_NONBLOCK=00004000`、`O_LARGEFILE=00100000`、`O_CLOEXEC=02000000`，`include/linux/fcntl.h` 的 valid open flags，`fs/fcntl.c::sys_fcntl()/do_fcntl()/setfl()`、`fs/read_write.c::ksys_lseek()` 和 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_fcntl=25`、`__NR_lseek=62` 为基准。当前支持 `fcntl(fd, F_GETFL, 0)`、TTY char-device fd 上的 `F_SETFL` `O_NONBLOCK` 状态位更新、`F_GETFD` 和 `F_SETFD`。`F_GETFL` 必须通过 fd table 找到 open file description，并返回当前记录的 Linux open/status flags：stdio 返回其访问模式，regular/directory fd 返回 `O_RDONLY` 以及打开时接受的 `O_DIRECTORY` 等持久 flags，TTY fd 返回 accepted open 中的 `O_RDWR|O_LARGEFILE|O_NONBLOCK` 等持久 status flags，且不得返回 `O_CLOEXEC`。`F_SETFL` 对齐 Linux `setfl()` 的用户可见形状：它更新 file status flags，不修改 access mode；当前首片只接受当前 console-like TTY char-device fd，并且只把 `arg & O_NONBLOCK` 写回该 fd entry 的 `O_NONBLOCK` 位，保留原 access mode、`O_LARGEFILE`、`O_DIRECTORY` 等已持久 flags，不接收也不返回 `O_CLOEXEC`。多个 `/dev/ttyN` alias fd 共享同一个 `Tty0` 后端，但 `F_SETFL/F_GETFL` 观察和修改的是各自 fd entry 的 flags；在第二个或第三个 TTY fd 上设置/清除 `O_NONBLOCK` 不应改变其它 TTY fd entry。因此 OpenRC 传入 `O_RDWR|O_NONBLOCK|O_LARGEFILE` 时成功后 access mode 仍来自原 open，`F_GETFL` 可观察 `O_NONBLOCK`；后续 `F_SETFL` 传入不含 `O_NONBLOCK` 的 flags 必须清除此位。普通 regular/directory/stdin/stdout/stderr fd 的 `F_SETFL` 当前仍 out-of-slice，返回明确错误且保留 fcntl 诊断；无效 fd 返回 `EBADF`。`F_GETFD` 返回 `FD_CLOEXEC` 或 0；`F_SETFD` 只取 `arg & FD_CLOEXEC` 修改同一 fd entry 的 close-on-exec bit。未知 cmd 返回 `EINVAL`。`lseek(fd, offset, SEEK_SET/SEEK_CUR/SEEK_END)` 必须通过 `FilesStruct -> FileDescriptorTable -> OpenFileDescription` 消费和更新当前 file position；regular file 的 `SEEK_END` 基于文件 size，directory fd 的 offset 是当前 `getdents64` byte offset，stdio/不可定位 fd 返回 `ESPIPE`。负目标 offset、未知 whence 返回 `EINVAL`。本首片仍裁剪普通文件/目录完整 `setfl()`、`O_APPEND` 写路径、`O_DIRECT/O_NOATIME/FASYNC`、file locks、owner/signals、leases、pipe/memfd fcntl、TTY/N_TTY 非阻塞读完整语义、完整 `vfs_llseek()` per-file op、64-bit overflow、concurrent `fdget_pos()` locking/refcount 和权限/LSM；这些必须作为 deferred/trimmed 语义保留。
- TTY ioctl 首片以 Linux 6.12 `fs/ioctl.c::sys_ioctl()/do_vfs_ioctl()/vfs_ioctl()`、`drivers/tty/tty_io.c::tty_ioctl()/tiocgwinsz()`、`drivers/tty/tty_ioctl.c::tty_mode_ioctl()/set_termios()`、`drivers/tty/tty_jobctrl.c::tty_jobctrl_ioctl()/tiocgpgrp()/tiocspgrp()/tiocgsid()/tiocsctty()`、RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_ioctl=29`、`include/uapi/asm-generic/ioctls.h` 的 `TIOCGWINSZ=0x5413` / `TCGETS=0x5401` / `TCSETS=0x5402` / `TIOCSCTTY=0x540e` / `TIOCGPGRP=0x540f` / `TIOCSPGRP=0x5410` / `TIOCGSID=0x5429`、`include/uapi/asm-generic/termios.h::struct winsize` 和 `include/uapi/asm-generic/termbits.h::struct termios` 为基准。当前支持 char-device fd 上的 `ioctl(fd, TIOCGWINSZ, struct winsize *)`、`ioctl(fd, TCGETS, struct termios *)`、`ioctl(fd, TCSETS, struct termios *)`、`ioctl(fd, TIOCGPGRP, pid_t *)`、`ioctl(fd, TIOCSPGRP, pid_t *)`、`ioctl(fd, TIOCGSID, pid_t *)` 和 observed `ioctl(fd, TIOCSCTTY, 1)`，必须先按 `sys_ioctl()` 的 fdget-like 顺序通过 fd table 验证 fd，再做 cmd 分派；这些 ioctl 都必须继续通过 `FilesStruct -> FileDescriptorTable -> OpenFileDescription -> FileBackend::CharDevice` 判断 fd 后端。`TIOCGWINSZ` 向用户指针写出 8 字节 winsize `{ ws_row, ws_col, ws_xpixel, ws_ypixel }`。`TCGETS/TCSETS` 使用 riscv64/generic 36 字节 `struct termios { tcflag_t c_iflag, c_oflag, c_cflag, c_lflag; cc_t c_line; cc_t c_cc[19]; }`；当前 console-like termios 初值采用 Linux 6.12 `tty_std_termios` 风格快照：`c_iflag = ICRNL|IXON`、`c_oflag = OPOST|ONLCR`、`c_cflag = B38400|CS8|CREAD|HUPCL`、`c_lflag = ISIG|ICANON|ECHO|ECHOE|ECHOK|ECHOCTL|ECHOKE|IEXTEN`，`c_line = 0`，控制字符可先返回 Linux-like 默认最小集合或 0。`TCSETS` 对齐 `tty_mode_ioctl(TCSETS)` 调用 `set_termios(..., TERMIOS_OLD)` 的用户可见首片：从用户指针复制一个 36 字节 old `struct termios`，更新当前 console-like termios 状态并返回 0；用户读失败返回 `EFAULT`，后续 `TCGETS` 必须读回更新值。`TIOCGPGRP` 对齐 Linux `tiocgpgrp()`：非 pty 主端首片要求当前任务的 controlling tty 等于该 char-device，否则返回 `ENOTTY`；当前 PID1 首片把 `/dev/tty` 和 fd0/1/2 的 console-like char-device 视为同一个 controlling tty，foreground process group 初始为 PID1，因此向用户指针写出 riscv64 `pid_t`/`int` 值 1；如果未来没有 foreground pgrp，按 Linux `pid_vnr(NULL)` 形状写 0 而不是臆造错误。`TIOCSPGRP` 对齐 Linux `tiocspgrp()`：同样要求当前 controlling tty，先从用户指针读出 riscv64 `pid_t`/`int`，负 pgrp 返回 `EINVAL`，未知 pgrp 返回 `ESRCH`，跨 session pgrp 返回 `EPERM`；当前首片接受 pgrp 1 和 observed child pgrp，并把 foreground pgrp 设置为对应值。`TIOCGSID` 对齐 Linux `tiocgsid()`：非 pty 主端首片要求当前任务的 controlling tty 等于该 char-device；若当前 PID1 或已通过 `TIOCSCTTY` 绑定的 observed child 拥有该 tty，则向用户 `pid_t *` 写出对应 tty session id；无当前 controlling tty/session 返回 `ENOTTY`，用户写失败返回 `EFAULT`。`TIOCSCTTY` 对齐 Linux `tiocsctty()` 的 observed getty 首片：只支持 current child 在 `setsid()` 成功后作为 session leader、尚无 child controlling tty 时，对 console-like TTY fd 绑定成功；成功后 child tty session id 设为 child SID，foreground pgrp 设为 child PGID。非 session leader、已有 child controlling tty、非 tty fd 或不兼容 session 竞争返回 `EPERM/ENOTTY`。`arg == 1` 只作为 observed 参数接受和记录，不实现完整 tty steal/CAP_SYS_ADMIN。无效 fd 返回 `EBADF`，非 char-device fd、未知 ioctl cmd 或当前 char-device 不支持的 ioctl 返回 `ENOTTY`，用户指针不可读写返回 `EFAULT`。固定长度用户态读写拷贝在执行 load/store 前必须先用当前 `UserAddressSpace` 的映射与权限事实预检，未映射、越界或权限不满足必须返回 `EFAULT`，不得在 supervisor copy loop 中触发 page fault。固定 winsize/console-like termios/pgrp/session 是阶段性的 console 事实，不得伪装为完整 TTY/N_TTY；`TCSETSW/TCSETSF`、输出 drain/wait、flush、driver/ldisc `set_termios` 回调、locked termios、`isatty` 完整语义、pty、resize/SIGWINCH、`TIOCSWINSZ`、`TIOCNOTTY`、完整 tty steal/CAP_SYS_ADMIN、真实 tty refcount/locks、session_clear_tty、orphan pgrp、job-control signal delivery、`FIONREAD/FIONBIO`、LSM/security hooks、compat ioctl、fd refcount/locking 和完整 `file_operations::unlocked_ioctl` 分派继续 deferred。
- Path access 首片以 Linux 6.12 `fs/open.c::sys_faccessat()/do_faccessat()` 和 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_faccessat=48` 为基准。`faccessat(dfd, path, mode)` 必须先按 Linux 顺序校验 `mode & ~(R_OK|W_OK|X_OK) == 0`，再从用户态复制 path；当前只支持 `dfd == AT_FDCWD`、非空 absolute path 和隐含 `flags=0`。实现必须通过当前 `FsStruct.root`、VFS path walk 和 ext2 inode metadata 判断路径存在，不得在 syscall table 中直接查 ext2；flags=0 下 fast symlink follow 与普通 path walk 一致。`F_OK` 对存在路径成功，`R_OK/W_OK/X_OK` 使用当前最小 `FileStat.mode()` 的 owner/group/other 位近似检查，其中 regular file 目前是 `0444`、directory 目前是 `0555`，所以只读 rootfs 上 regular `W_OK` 返回 `EACCES`，regular `X_OK` 返回 `EACCES`，directory `R_OK|X_OK` 可成功。无效 mode 或 unsupported dirfd 返回 `EINVAL`，路径不可读返回 `EFAULT`，路径不存在返回 `ENOENT`，权限不满足返回 `EACCES`，symlink follow 超过 40 次返回 `ELOOP`。本首片仍裁剪 `faccessat2` flags、`AT_EACCESS` credential override、`AT_SYMLINK_NOFOLLOW`、`AT_EMPTY_PATH`、相对 dirfd、真实 uid/gid/capability、ACL、idmapped mount、readonly mount 的 `EROFS` 区分、LSM/security hooks、`inode_permission()` 完整语义、RCU/retry_estale、slow symlink 和完整 errno；这些必须作为 deferred/trimmed 语义保留。
- Readlink 首片以 Linux 6.12 `fs/stat.c::do_readlinkat()/sys_readlinkat()`、`fs/namei.c::vfs_readlink()/readlink_copy()` 和 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_readlinkat=78` 为基准。`readlinkat(dfd, path, buf, bufsiz)` 必须先按 Linux 顺序拒绝 `bufsiz <= 0` 为 `EINVAL`，再从用户态复制 path；当前只支持 `dfd == AT_FDCWD` 和非空 absolute path。实现必须通过当前 `FsStruct.root` 与 VFS no-follow final symlink lookup，读取 final symlink 本身的 target，而不是普通 path walk follow 后的目标文件；非 symlink final dentry 返回 `EINVAL`，路径不存在返回 `ENOENT`，用户 path/buffer 不可读写返回 `EFAULT`，symlink follow parent 路径超过 40 次返回 `ELOOP`。成功时复制 `min(strlen(target), bufsiz)` 字节到用户 buffer，不追加 NUL，并返回复制字节数；target 大于 buffer 时按 Linux 截断成功，不返回 `ENAMETOOLONG`。本首片只覆盖 read-only ext2 fast symlink inline target；`AT_EMPTY_PATH`、相对 dirfd、slow symlink page/block target、AFS-style non-symlink readlink、atime、LSM/security hooks、RCU/retry_estale、mount namespace、权限和完整 errno 继续 deferred/trimmed。
- Getrandom 首片以 Linux 6.12 `drivers/char/random.c::sys_getrandom()`、`include/uapi/linux/random.h` 和 RISC-V `include/uapi/asm-generic/unistd.h` 的 `__NR_getrandom=278` 为基准。Linux 顺序是先校验 flags 只包含 `GRND_NONBLOCK|GRND_RANDOM|GRND_INSECURE`，拒绝 `GRND_INSECURE|GRND_RANDOM` 组合，再按 CRNG ready 和 flags 处理等待或 `EAGAIN`，随后 import 用户 buffer 并通过 `get_random_bytes_user()` 复制随机字节。当前首片只支持 `flags == 0`、`len == 0` 返回 0、`0 < len <= USER_COPY_MAX` 的小 buffer 成功路径，使用 `HwRngCore.read_current()` 从当前 hwrng provider 读取并复制给用户；provider 不可用或暂时无数据返回 `EAGAIN`，用户 buffer 不可写返回 `EFAULT`，未知 flags 或超出首片的大 buffer 返回 `EINVAL`。该 syscall 是 kernel random core 接口，不依赖 `/dev` 文件系统；完整 CRNG 池、blocking wait queue、`GRND_NONBLOCK`、`GRND_RANDOM`、`GRND_INSECURE`、signal interruption、iov iteration、大 buffer 循环和随机性质量策略继续 deferred/trimmed。
- Unsupported syscall 首片必须参照 Linux 6.12 RISC-V 路径：`arch/riscv/kernel/traps.c::do_trap_ecall_u()` 在 syscall 分发表前预置 `a0=-ENOSYS`，`arch/riscv/kernel/syscall_table.c` 将未填表项指向 `__riscv_sys_ni_syscall`，`kernel/sys_ni.c::sys_ni_syscall()` 返回 `-ENOSYS`。当前实现对未知 syscall number 或本项目尚未支持的 syscall 子路径必须返回 `-ENOSYS`，并打印稳定诊断行，至少包含 `nr`、trap mode、`a0..a5`、`sepc` 和 `stval`；该诊断用于定位下一批 syscall 缺口，不表示该 syscall 已建模支持，不得新增 checkpoint 或改变成功路径事件序列。OpenRC/login 早期证据中的 `socket(AF_UNIX, SOCK_DGRAM|SOCK_CLOEXEC, 0)` / `sendto(206)` 返回 `ENOSYS` 后用户态仍能推进到 getty/login，因此该 syslog-like DGRAM 形态继续记录为 tolerated/noisy unsupported syscall；fd-local `fchown(55)` / `fchmod(52)` 闭合后的 focused trace 又记录 post-auth 第一阻断为 `socket(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC, 0)`，而不是 guest 文本中的 `setgroups(159)`。当前片把该 stream socket 纳入 supported 分发表并安装 fd；unsupported socket detail 仍用于 DGRAM 和其它尚未规格化子路径，必须打印 raw domain/type/protocol、`type` 去掉 `SOCK_CLOEXEC`/`SOCK_NONBLOCK` 后的 base type、两个 flag 是否存在，以及是否为 `AF_UNIX + SOCK_DGRAM` syslog-like 形态。`connect(203)` 已进入分发表，但仅支持 AF_UNIX pathname failure errno 首片；unsupported connect 子路径仍必须显示 `name=connect`、fd、sockaddr 指针、addrlen、fd 是否指向 `UnixSocket0`、sockaddr copy/family/validate/path 分类，并返回 `ENOSYS`，不得安装 fd、改变用户内存、改变 checkpoint 顺序或提前实现 `sendto(206)`、成功 `connect(203)`、完整 credentials 或 TTY ownership。若真实发行版路径到达 `nanosleep(101)`，诊断可以在同一行或紧邻行增加 `rqtp`、`rmtp`、`req_sec`、`req_nsec` 或 `req_copy=failed`，但该 best-effort 用户内存读取只用于定位，不得改变 `ENOSYS`、用户内存、checkpoint 顺序或后续 syscall 顺序。若真实 `/bin/sh -> ls` child continuation 到达 `execve(221)`，unsupported 诊断必须在仍返回 `ENOSYS` 的前提下增加 `name=execve`、`filename_ptr`、`argv_ptr`、`envp_ptr`、是否已经进入 `UserChildProcess` continuation，以及受 `UserAddressSpace` 映射范围保护的 best-effort filename、bounded argv/envp 前缀指针和值复制；复制失败、空指针或跳过必须以稳定 token 表示，不得触发新的 page fault、改变用户内存、改变 checkpoint 顺序、改变 `ENOSYS` 返回或把该 syscall 计为已支持。
- 当前 setgroups/getgroups 更新：`setgroups(159)` 已进入 supported syscall table，但只覆盖 root effective uid、`size==0` 和 `size==1` 的 bounded supplementary group view；`size>1` 及完整 credentials 子路径仍使用 unsupported `ENOSYS` 诊断。supported error path 的 `EPERM`/`EFAULT` 由 `PROBE=user-syscall-error` 打印 `setgroups detail`。`getgroups(158)` 只回读该 bounded view；`gidsetsize=0` 返回数量，缓冲区不足返回 `EINVAL`，copyout fault 返回 `EFAULT`。
- OpenRC foreground pgrp 诊断必须留在长期低噪声 probe 下。`PROBE=user-syscall-error` 的 `TIOCSPGRP` detail 必须在不改变 errno 的前提下打印 decoded `pgrp_value`、当前 foreground pgrp、current visible child pid、child pgrp、child session 和 controlling-tty bound facts；`setpgid` error detail 必须打印 raw/normalized pid、raw/normalized pgid、current child pid/current child pgrp/current child session 和拒绝原因。若这些字段显示 pgrp/session facts 不符合 bounded same-session 形态，应先重新分类，不得顺手扩大到完整 job-control。
- Supported syscall error diagnostic 必须是显式低噪声 probe，而不是默认日志。当前用于定位发行版 `/bin/ls` 和 `/bin/sh` 的 `PROBE=user-syscall-error` 只观察已经进入 `SyscallTable` 的 supported syscall 错误返回，输出稳定行并至少包含 `nr`、可读 syscall name、`errno`、trap mode、`a0..a5`、`sepc` 和 `stval`；path syscall 在已经完成用户 path copy 后失败时，还应输出内部 `FileError` 名称和可打印 path 字节，便于把用户态 `I/O error` 缩小到相对路径、cwd、path walk 或后端错误分类。若 supported syscall 在复制 path 之前就按当前 Linux-like 参数顺序决定失败，例如 `openat` 因 dirfd、access mode 或超出首片的 flags 返回 `EINVAL`，probe 可以额外输出拒绝原因、`dirfd`、`flags`、不支持的 flag mask、access mode 和受 `UserAddressSpace` 已映射范围保护的 best-effort path copy 结果；该 copy 只能在 probe 中、且只能在返回值已经决定之后用于诊断，失败或跳过时必须打印 `path_copy=failed/skipped`，不得改变 errno、用户可见副作用或后续 syscall 顺序。`openat`、`close`、`dup3` 和 close-on-exec 相关错误/边界诊断还应打印 fd table occupancy、capacity、每个 occupied fd 的 backend/OFD kind、status flags、close-on-exec bit 和失败原因；`execve` EFAULT 诊断应打印 filename/argv/envp 指针、child continuation、failure stage/reason/detail、filename/argv/envp 前缀复制结果和已记录的 execve observation，execve 前缀 C-string copy 使用与 execve filename/argv 相同的逐字节 bounded 可读性检查，不再要求整段 `USER_PATH_MAX` 已映射，失败或读满仍打印稳定 failed/skipped 类 token；成功路径的 close-on-exec 诊断应打印 scanned count、closed count、first closed fd 和 remaining open count。`fcntl`、`ioctl` 和 `ppoll` 错误诊断应按本地 Linux 6.12 `include/uapi/asm-generic/fcntl.h`、`include/uapi/linux/fcntl.h`、`include/uapi/asm-generic/ioctls.h` 和 `include/uapi/asm-generic/poll.h` 解码常见命令名或 readiness 位，至少覆盖 `F_DUPFD`、`F_GETFD`、`F_SETFD`、`F_GETFL`、`F_SETFL`、`F_DUPFD_CLOEXEC`、`TCGETS`、`TIOCGWINSZ`、`TIOCGPGRP/TIOCSPGRP` 和 `POLLIN/POLLOUT/POLLNVAL`；syscall name 解码必须覆盖 `dup3`，使 `PROBE=user-syscall-trace,user-syscall-error` 能区分 nr 24 与 unknown syscall。该 probe 不改变 syscall 返回值、errno 映射、checkpoint 序列或 user-smoke 判定，默认 `make run APP=user-boot`、默认 overlay smoke 和 `make test` 不启用。该诊断用于把 `ls: .: I/O error`、`/bin/sh: can't access tty`、`No file descriptors available` 或 OpenRC login `Bad address` 之类用户态症状缩小到直接 kernel errno 来源；如果字段不足，应先扩展长期有意义的诊断字段或对象事实，再改功能代码。若症状没有对应错误返回，例如 `read(63)` 成功返回 0 导致 shell 认为输入结束，则必须使用专门的 success-path/read trace probe 或补长期有意义的对象事实，不能靠错误 probe 盲猜。
- `PROBE=user-syscall-trace` 是显式的 syscall return/exit 顺序诊断，用于定位发行版 `/bin/sh` 这类没有错误返回但行为仍未解释的 success-path。它必须在不改变 syscall 返回值、errno 映射、checkpoint 序列、user-smoke 判定和默认 `make test` 输出的前提下，打印每次 syscall return 的 `nr`、可读 name、return value、trap mode、`a0..a5`、`ra`、`s2/s3/s4`、`sepc` 和 `stval`；`ra` 用于把 libc syscall wrapper 返回点映射回 BusyBox/用户程序调用点，尤其是在 `sepc` 固定落于 musl ecall wrapper 时定位上层控制流。`exit(93)` / `exit_group(94)` 因成功后不会回到 syscall return 流程，必须在 shutdown 前单独打印 status 和同一组寄存器上下文。该 trace 可以覆盖 unsupported/error return，但不得替代 `user-syscall-error` 的细粒度错误分类，也不得为了 trace 额外读取或修改用户内存。`ppoll(73)` 这类 syscall 如果已经在正常语义中复制了参数对象，可以在同一 probe 下打印已复制的 `nfds`、timeout 是否为 NULL、timeout 值、sigmask 是否为 NULL、ready count 以及每个 bounded pollfd 的 `fd/events/revents`；这些 detail 只能来自 syscall 正常处理已经取得的局部事实。若字段不足以解释 prompt/terminal query/exit 顺序，应先扩展长期有意义的 trace 字段或对象事实，再改功能代码。
- 用户态 page fault、unexpected exception、syscall-disabled 等 trap 终止路径必须输出稳定 trap diagnostic，而不是只输出临时 panic 文本。诊断字段至少包含 trap mode（由 `sstatus.SPP` 判断 user/supervisor）、`scause`、`sepc`、`stval`、`sstatus`、`a7`、`a0..a5`、`gp` 和 `tp`。用户态 page fault 还必须输出 `UserAddressSpace` 诊断：当前 `satp`、该地址空间的 expected `satp` token、二者是否匹配、`sepc` 与 `stval` 各自命中的 mapping kind、mapping 起止页范围、R/W/X/U 权限，以及 fault 类型需要的权限是否满足；未命中任何 mapping 时必须稳定打印 `unmapped`。mapping kind 至少区分 main/interpreter ELF segment、stack、heap、anonymous mmap/arena 和 empty/unmapped；当前实现若尚不能区分 main 与 interpreter，必须先用 `elf` 作为保守分类并保留后续细分任务。该诊断用于区分未映射、缺少执行权限、错误 `satp`/当前地址空间和 trap frame/sstatus 错误，不得改变 syscall 返回值、checkpoint 序列、用户内存或后续 trap 处理。字段对应 Linux 6.12 RISC-V `handle_exception` 保存 `pt_regs` 后分发到 `do_trap_*` 的定位面；当前只作为失败路径可见性，不实现完整 signal/page-fault recovery。
- `APP=user-boot` 的运行期 syscall checkpoint/KUnit handler 观测的是 syscall/event 类事实，不是用户程序每次 syscall 的 trace 计数。`write/openat/read/close/newfstatat/exit` 等 checkpoint 若在同一次用户态运行中重复出现，handler 必须保持固定 KUnit plan，只对该事件类的首个代表性 checkpoint 产出 case；后续重复 checkpoint 可跳过。`write` 路径观测不得要求固定 payload 文本或固定长度，只能校验 `SyscallTable -> FilesStruct -> FileDescriptorTable -> OpenFileDescription -> FileBackend::CharDevice` 路径、非零写入长度和 OFD/backend 写入长度一致。
- 针对当前临时 `/sbin/init` overlay ELF 的 smoke/KUnit 验证只能读取 `ElfObject` 正常模型事实，例如 entry、`PT_LOAD` 数量、段权限、entry 是否落在可执行段、以及 fixture 内容字节是否位于 loadable 文件内容中；不得为了测试给普通对象增加 `test_only_*` 或等价专用 API。

## Pre-VM lifecycle 代码生成约束

`Lifecycle` 是对象模型贯穿内核生命周期的核心保障机制：它既覆盖入口前导期，也覆盖后续核心初始化和 payload handoff。其实现应按内核基础设施对待，优先保证确定性、可审计性和 pre-VM 安全性；必要时可以用手写汇编实现关键路径，而不是完全依赖编译器对普通 Rust 控制流的 lowering。

`Lifecycle::{transition, adopt_transition}` 在 `EarlyVm` 启用前可由入口前导路径调用。该路径运行时 `satp == 0`，只能执行 PC-relative
直线代码，不得依赖 high-half jump table、`.rodata` 分发表、间接跳转目标、格式化或 panic 路径。`is_allowed_lifecycle_transition`
是这两个方法共享的转移合法性检查，必须保持为 pre-VM safe 实现。

RISC-V64 实现中，`State` 与 `LifecycleEvent` 必须使用稳定 `#[repr(u8)]` 编码；`is_allowed_lifecycle_transition`
必须通过手写汇编 `arceos_ex_is_allowed_lifecycle_transition` 执行整数 key 比较。该汇编函数只允许：

- 使用 `a0/a1/a2` 中的 `source/event/target` 编码构造 key。
- 使用 `li/beq/ret` 等直线比较和直接条件分支。
- 返回 `a0 = 0/1`。

该函数不得访问内存，不得调用其它函数，不得使用跳转表，不得依赖 `.rodata`，不得改回普通 Rust `match`、嵌套 enum 分支或其它可能由编译器 lowering
成 jump table 的写法。若新增 lifecycle 状态或事件，必须同步更新：

- `State` / `LifecycleEvent` 的 `#[repr(u8)]` 枚举顺序或显式编码。
- 汇编中的 allowed transition key 列表。
- 非 RISC-V fallback 的整数 key 列表。

修改后应通过反汇编审计确认 `arceos_ex_is_allowed_lifecycle_transition` 不含 `jr/jalr`、不含 load 指令、也不引用 `.rodata`。
将来若出于性能、确定性或 pre-VM 约束考虑，把 `Lifecycle::transition`、`Lifecycle::adopt_transition` 或其它 lifecycle
核心函数也改为汇编实现，应延续同样约束：固定 ABI 编码、无表驱动控制流、无隐式内存依赖，并在规格中同步记录审计要求。

## `make verify` obligation 分类

`80966ec` 曾暴露 `13 obligation / 2 deferred`。这些条目不能作为实现可忽略的提示；处理原则是：实现某个对象 transition前，必须先通过“推导义务门禁”。能由模型、推导工具、链接脚本、ISA、固件规范或已证明前序事实推出的，应优先补齐推导证明；只能由外部交付保证支撑的，应明确作为 source assumption，并在后续对象 transition中尽快转化为运行期检查；无法归类的应作为规格缺口或显式 deferred。`make verify` 的通过口径必须包含 `obligation == 0`；`check: passed` 或进程退出码为 0 不能在仍有 unresolved obligation 时被解释为规格验证通过。

当前已补齐 Lds、OpenSBI DTB handoff 和 BootCPU 前序事实的推导规则，`make verify` 报告为 `0 obligation / 3 deferred`。后续若再次出现 obligation，应先回到本节分类处理，不得直接继续实现；工具层必须让 `make verify` 在 `obligation > 0` 时返回失败。

| 分类 | 条目 | 影响范围 | 当前处理策略 |
| --- | --- | --- | --- |
| 固件交付假设，运行期逐步确认 | `firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)`；`firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)` | `OpenSbiFirmware.Online`、`RawDtb.Preset`、`RawDtb.Setup` | 已作为 OpenSBI handoff source fact 进入推导；`RawDtb` 仍必须逐步读取 header、检查 magic、读取 totalsize 并确定完整范围。若确认失败，事件返回 `Failed`，不得继续推进。 |
| 链接/入口布局硬约束 | `text_start == kernel_start`；`elf_entry == kernel_start`；`entry_head_text_layout_ready(Lds)`；`pre_mmu_access_discipline_ready(Lds)`；`trampoline_access_discipline_ready(Lds)` | `Lds.Online`、`KernelImage.Preset/Setup`、`TrampolineVm`、`EarlyVm` | 已作为 linker script source fact 进入推导；实现必须继续由 linker script、入口段布局和构建期/启动期检查维持这些事实。推进真实页表前，还要复查 head/trampoline 安全范围和访问纪律。 |
| 对象前序事实 | `boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid)`；`boot_cpu_present(BootCPU)`；`boot_cpu_active(BootCPU)` | `BootCPU.Preset`、`BootCPU.Setup`、`BootCPU.Enable` | 已由 `BootCPU` 事件 `ensures` 和前序事实推导消化；实现方面，`BootCPU.Preset` 已记录启动 hartid，`Setup/Enable` 仍需在入口后继期基于 `PlatformCpuInfo` 和 FDT `/cpus` 完成。 |
| 显式 deferred | `Soc` 早期平台状态点；`efi_init()` | `Soc.Preset` 和后续非最小启动路径 | 保持 deferred，不在第一轮对象级最小闭环中隐式实现；`StaticBranch` 已进入 formal model，后续 EFI 对象扩展时再展开 EFI 路径。 |

### 待迁移到 model 的 deferred 标记

`deferred` 的归属首先是 `spec/model`。本节只是实现说明中的审计清单，用来记录哪些参考 Linux
启动流程尚未迁移到模型文件中的显式 `deferred` 块。完成迁移后，模型才是这些 deferred 的事实源；
`impl/arceos_ex` 是否实现、何时实现，是后续单独的展开任务。

以下条目来自对照 `../linux-6.12/.config` 后的规格缺口复查。它们不要求立即实现，但后续应先补入
`spec/model` 对应对象或 phase 的 `deferred`，避免读者把当前最小模型误解为已经完整覆盖参考 Linux
启动路径。

| 候选项 | 参考配置/路径 | 建议归属 | 说明 |
| --- | --- | --- | --- |
| RISC-V Linux boot image header / boot protocol header | RISC-V 入口协议 | `PreparePhase` 或 `Lds` | 当前由 `BootArgs`、`Lds` 前置事实吸收，但 header 本身没有说明不展开。 |
| EFI stub / PE header 入口细节 | `CONFIG_EFI=y`、`CONFIG_EFI_STUB=y` | `PreparePhase` 或 `Lds` | 现有 deferred 只覆盖 `efi_init()`，未覆盖 EFI stub/header 入口。 |
| SATP mode 探测与页表层级降级 | `CONFIG_PGTABLE_LEVELS=5` | `Config` 或 `Vm.Preset` | 当前 `Config.satp_mode` 是既定事实，未描述 Linux 的运行时探测/降级过程。 |
| `apply_early_boot_alternatives()` | `CONFIG_RISCV_ALTERNATIVE_EARLY=y` | `Vm.Preset` | 已迁移到 `EntryPreludePhase`/`Vm.Preset` 的显式 deferred；早期 alternatives/errata patch 仍未展开为对象。 |
| `set_task_stack_end_magic()` | `CONFIG_SCHED_STACK_END_CHECK=y` | `BootInitStack` | 可折叠进栈保护语义，但应说明当前不展开 Linux 的具体检查标记。 |
| `init_vmlinux_build_id()` | `start_kernel()` early generic path | `EntrySuccessorPhase` | 已迁移到 `EntrySuccessorPhase.Setup` 的显式 deferred；当前不展开 build-id 元数据对象，但实现必须保留调用位置事实。 |
| `page_address_init()` | `start_kernel()` before `setup_arch()` | `EntrySuccessorPhase` | 已迁移到 `EntrySuccessorPhase.Setup` 的显式 deferred；当前没有 page address freelist/hash 元数据对象，但实现必须保留调用位置事实。 |
| `setup_command_line()` / saved cmdline | `start_kernel()` after `setup_arch()` | `CommandLine` | `CommandLine` 管理 raw/saved/static 三个文本视图；Param 解析对象由 `Params` 管理并保持原时序。 |
| DT unflatten | `CONFIG_OF_FLATTREE=y` | `EarlyDtb` 或后续 DT 对象 | 当前只覆盖 early scan 所需事实，未标记 unflatten 阶段。 |
| `phys_ram_base` / `kernel_map.va_pa_offset` 建立 | `CONFIG_64BIT=y`、`CONFIG_MMU=y` | `MemBlock.Setup` | 已迁移为 `memblock_phys_ram_base_ready(MemBlock)` 和 `memblock_kernel_va_pa_offset_ready(MemBlock, Vm)`；实现由 `MemBlock` setup facts 暴露。 |
| `ZONE_DMA32` / zone 边界初始化前置事实 | `CONFIG_ZONE_DMA32=y` | `MemBlock.Setup` | 已迁移为 `memblock_dma32_limit_ready(MemBlock)` / `memblock_dma32_zone_input_ready(MemBlock)`；完整 `Zones` 层级仍由后续 `CorePreparePhase` 建模。 |
| hugetlb 早期保留 | `CONFIG_HUGETLB_PAGE=y` | `MemBlock.Setup` | 已迁移为 `memblock_hugetlb_early_reserve_deferred(MemBlock)`；当前不展开 Hugetlb/CMA 对象或运行期 hugetlb 锁。 |
| final page table 权限细分 RW/RO/NX | `CONFIG_STRICT_KERNEL_RWX=y` | `SwapperVm.Setup` | 已迁移为 `swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm)` 和 `swapper_vm_final_permissions_not_split_yet(SwapperVm)`；完整 mapping protection 后续展开。 |
| `riscv_fill_hwcap()` / ISA 能力发布 | FPU/V/Zicbom 等启用 | 后续 `CpuFeature` / `UserIsa` 对象 | 当前边界没有 CPU feature/hwcap 发布对象。 |
| `apply_boot_alternatives()` | `CONFIG_RISCV_ALTERNATIVE=y` | 后续 `Alternative/Patch` 对象 | boot alternatives 未建模，且不同于 early alternatives。 |
| `riscv_user_isa_enable()` | RISC-V ISA 配置相关 | 后续 `UserIsa` 对象 | 用户态 ISA 暴露语义当前不属于最小闭环。 |

由此得到后续 `EntryPreludePhase` 实现顺序：

1. 持续保持 `Lds`/`KernelImage` 相关符号和布局检查面，确保 `_start`、`kernel_start`、text 起点、ELF entry、head text 范围在实现中可对应。
2. 继续实现 `RawDtb.Preset/Setup` 的最小确认路径，把 OpenSBI handoff 假设转化为 header/magic/totalsize/range 的运行期检查。
3. 在上述事实具备后，推进 `FixMap`、`TrampolineVm`、`EarlyVm` 和 `Vm` 三段切换。

## 应用复用

当前对象级实验不复用现有 ArceOS Unikernel 应用，不依赖 `ax-std`、`ax-api`、`ax-feat` 或 `arceos-rust`。

第一轮保留两个内建 payload：默认 `APP=smoke` 和最小独立 `APP=hello`。后续 `APP=user-boot` / `UserBootPayload` 作为第三类 selected payload 接入同一选择机制，用于从当前 rootfs 读取 `/sbin/init` 并进入第一个用户态 ELF。对象级初始化完成后，启动链沿 `KernelInitTask` 的 `TaskEntry::KernelInit` 从 `PreSmpInitPhase` 开始的连续执行线进入 `PayloadPhase`，在 `PayloadPhase.Enable` 提交后调用 selected payload 的 `run() -> !`。当前 `smoke` payload 在 `impl/arceos_ex/src/apps/smoke/cases/` 下维护可返回测试用例，首批覆盖输出路径、格式化输出、MemBlock 分配和 FDT 查询。`APP=hello` 仍作为最小独立 payload，只通过 printk 前端输出 `Hello, world!` 后通过 SBI 关机；它不得直接调用 early console 或 real console backend。

所有 payload 的入口约定为 `run() -> !`。这表示控制流不返回启动编排链：Unikernel payload 可以进入服务循环或停机，未来宏内核 payload 可以加载首个用户态程序并完成用户态切换。若某个 payload 意外返回，应视为违反 `PayloadPhase.Enable` 的 no-return handoff 契约。

当前 payload 选择由 Makefile 变量控制，`APP` 会转换为 Rust `--cfg app_<name>`，例如 `APP=smoke` 对应 `app_smoke`。后续新增 payload 时，应在 `impl/arceos_ex/src/apps/` 下新增模块，并在 `apps/mod.rs` 中加入对应静态选择分支。后续新增 smoke 用例时，应放在 `impl/arceos_ex/src/apps/smoke/cases/` 下，并返回 `SmokeResult`，不得使用 payload 级 `run() -> !` 契约。

在未来 Composition Phase 中，再恢复“Unikernel app 引领内核形态”的 ArceOS 设计，并讨论如何接入 `ax-std`、测试 payload 和宏内核 payload。

### smoke 测试边界

smoke payload 用于覆盖 QEMU 运行期可观察行为，以及规格推导不能单独替代的实现效果，例如控制台输出、格式化输出、内存分配动作、FDT/DeviceTree 公开查询接口、资源摘要和 payload 关机路径。若某个性质仅仅是在重复对象状态、生命周期顺序、内部副本一致性或谓词不变量，并且已经能由 `make verify` 的规格推导闭合，则不应为它新增 smoke 用例。

实现也不应为了 smoke 暴露原本不需要公开的内部状态查询接口。若某个对象同时有可验证的不变量和用户可观察行为，smoke 应测试后者；前者保留在模型谓词、推导验证和对象 transition推进检查中。例如 `CommandLine` 的 raw/saved/static 文本视图一致性属于规格和实现状态推进约束，不需要单独增加只读取内部状态的 smoke case。

`TaskCreationCore.copy_process()` 与 `CurrentRunQueueRef`/`RunQueue.EnqueueTask`/`RunQueue.PickNextTask` 属于 `ObjectApiBehavior` smoke：它们验证正式对象 API/action 契约，可以构造局部 subject 对象，并只读 live context 满足依赖前置条件。这类用例默认只注册到 app smoke，不加入 checkpoint KUnit smoke 列表。实现不得为它们增加 `test_*` 被测入口；若缺少可测边界，应补正式对象 API，并让生产路径与 smoke 路径共享同一入口。

`Scheduler.schedule()` 的专门 smoke 验收是 payload 阶段的 app-smoke-only 用例，
不是 checkpoint KUnit。它测试的是启动完成后 `Scheduler.schedule()` 作为正式 API
在真实运行环境中的协作式切换闭环，而不是 `rest_init` 首次 schedule checkpoint。
正常场景从 `KernelInitTask` 执行线开始：创建并入队一个 smoke scheduler task，
随后 `KernelInitTask` 在有限次数内主动调用 `schedule()`，必须真实进入该 smoke
task 的入口；smoke task 记录已经运行，再主动调用 `schedule()`/yield，最终返回
`KernelInitTask` 的调用点并由 smoke 断言成功。

这一路径要求 `Scheduler.schedule()` 在 payload 阶段支持非 idle current，
`CurrentTaskRef` 至少能表示 `KernelInitTask` 和该 smoke scheduler task，runqueue
能接收并选择该 smoke task，`switch_to` 至少对该 smoke task 执行真实的协作式
栈/上下文转移。完整抢占、时间片、睡眠唤醒、SMP 调度和 Linux
`finish_task_switch()` 仍 deferred。实现不得把这个 smoke 用例注册到 checkpoint
KUnit，也不得放宽 `scheduler_action` KUnit 对 `rest_init` 首次 schedule 的断言。
若为该闭环新增边界，必须是正式 scheduler/task API，不得新增 `test_*` 被测入口。

PageAllocator API smoke 是动态容器前置链路的第一个运行期用例：它应通过正式 `alloc_pages(order, gfp)` 分配至少一个
order-0 页引用，并至少覆盖一个高 order 分配，通过该 `PageRef` 的受限映射视图做有界读写校验，再用
`free_pages(page_ref, order)` 释放。该用例不得绕过正式对象 API，也不得读取 buddy free list、zone 统计等私有内部状态作为断言条件。
SLUB/kmalloc smoke 必须在这个 PageAllocator smoke 之后推进。

SLUB/kmalloc API smoke 是动态容器前置链路的第二个运行期用例：它应通过正式 `kmalloc(size, gfp)`、`kzalloc(size, gfp)`
和 `kfree(alloc_ref)` API 覆盖小对象分配、写读、零初始化、同尺寸多对象互不覆盖，以及释放后复用。该用例不得直接操作
SLUB freelist、slab slot metadata 或 PageAllocator 内部状态；断言应通过分配引用的线性映射访问边界完成。`GlobalAlloc`/`Vec`
smoke 必须在这个 SLUB/kmalloc smoke 之后推进。

GlobalAlloc/Vec smoke 是动态容器前置链路的第三个运行期用例：它应在 `KernelGlobalAllocator.Ready` 和
`DynamicContainerRuntime.Ready` 之后，通过普通 `Vec` API push 足够元素以触发至少一次 grow，校验元素读回，并让 `Vec`
正常 drop。该用例不得调用 SLUB/PageAllocator 私有接口，也不得通过测试专用 allocator hook 绕过正式 `GlobalAlloc` 路径。

### 启动与 smoke 输出风格

启动日志和 smoke 用例输出主要服务人工审阅，SHOULD 优先采用接近 Linux 启动日志的清晰文本格式，而不是大量
`key=value` 调试字段。机器可解析的状态序列应通过 checkpoint announce、Linux-like trace 或后续结构化报告承载，不应挤进普通启动日志。

建议格式如下：

- 用简短标题标明当前对象或测试主题，例如 `Resource tree:`。
- 多项事实分行输出，左侧使用稳定的人类可读标签，冒号对齐，右侧放结果值。
- 物理地址范围采用 `[mem start-end]` 风格，输出为闭区间；内部实现仍可继续使用半开区间。
- 容量优先用 `KiB`、`MiB` 等可读单位，避免只输出裸字节数。
- 汇总行应说明事实类别和数量，例如 `1 region`、`5 regions`、`4 segments`；单复数可读性优先于完全机器化。
- 测试失败时可以直接输出一行具体失败原因，不要求套用对齐格式。

示例：

```text
Resource tree:
  Entries      : 12
  Root         : I/O memory
  System RAM   : 1 region, 131072 KiB
  Reserved     : 5 regions
  Kernel image : [mem 0x80200000-0x80221fff], 4 segments
```

### per-cpu 链接段约束

`PerCpuStaticImage` 由链接脚本中的 `.data..percpu` 输出段提供。链接脚本必须先收集显式 per-cpu 输入段，再收集普通
`.data.*` 尾部段；不得让 `.data .data.*` 的普通 wildcard 提前吞掉 `.data..percpu`、`.data..percpu.*` 等输入段。运行期
`Lds.entry_layout_ready()` 会检查 `__per_cpu_start < __per_cpu_end`、起点页对齐，以及 `__per_cpu_load` 到模板大小的加载范围仍处于内核镜像内。

### 静态 per-cpu 访问接口

当前实现 SHOULD 提供两类接近 Linux 静态 percpu 使用方式的接口：

- 定义接口：通过宏封装 `#[link_section = ".data..percpu"]` 和 `#[used]`，只声明静态 percpu 模板变量。
- 访问接口：通过 `PerCpuStorage` 的 offset table 按 logical CPU 计算实例地址，并提供 `per_cpu_ptr` 风格地址计算以及类型化读/写 action；调用者不得直接依赖 first chunk 的 dynamic/reserved 内部布局。

从规格语义看，静态 percpu 访问是 action：它锚定在 `PerCpuStorage.Ready` / `PerCpuOffsetTable.Ready` 状态上执行。若对象尚未处于这些状态，action 应失败或返回空结果；若 action 成功，也仍停留在原 Ready 状态，不推进任何对象生命周期状态，也不得伪造成新的 `Setup/Enable` 事件。

smoke 可以覆盖静态 percpu 访问的真实运行期行为，例如对每个 possible logical CPU 的实例分别写入并读回，确认实例互不影响。完整动态
percpu allocator 暂缓，在规格中只有 `dynamic reserve ready` 布局事实时，不应新增动态分配或动态区裸写 smoke。

## 新增核心 crate

当前不新增 crate。源码先集中在 `impl/arceos_ex/src/`，可按对象和架构分目录组织：

```text
impl/arceos_ex/
  Makefile
  README.md
  linker/riscv64.lds
  src/
    arch/riscv64/
    projects/
    systems/
    objects/
    phases/
    checkpoint/
```

目录结构服务于对象级实现清晰性，不承担最终组件边界。

对象级实现优先采用“一个模型对象对应一个 `.rs` 文件”的组织方式。该要求可以逐步落实，不要求一次性重排既有文件。随着文件数量增加，可再按对象类别建立目录层级；架构强相关对象可以在类别目录下进一步区分架构。目录层级只用于源码管理，不等同于后续 Composition Phase 的 crate/module 公开边界。

## Cargo/xtask 策略

当前不使用 Cargo workspace、overlay workspace 或 `xtask`。如果需要 Rust 编译，Makefile 直接调用 `rustc` 或一个局部最小
`Cargo.toml`，但不得引入 ArceOS feature 传递链。

Cargo/xtask 策略整体延期到 Composition Phase。

## CI 与项目主页

GitHub workflow 分为测试和展示两类，但展示内容应主要来自测试流水线产物，不建立另一套独立生成来源。主页展示不要求高即时性，优先展示最近一次 nightly 或手动 workflow 成功生成的结果。

### 快速 CI

快速 CI 用于 pull request 和 push，目标是在较短时间内发现关键问题。第一轮应覆盖：

- 推导工具本身的格式检查、lint、单元测试和关键边界测试。
- 核心规格推导验证，例如 `spec/model/main.spec --derive --strict`。
- trace 生成 smoke test：输出到临时目录，确认命令成功，不要求把生成图提交回仓库。
- 顶层 `make verify`。
- 当对象级内核骨架具备可编译状态后，加入顶层 `make build`。

快速 CI 不发布 GitHub Pages，不运行耗时长或依赖模拟器稳定性的全量任务。

### Nightly 与手动触发

Nightly workflow 用于定时日构建，也支持 `workflow_dispatch` 手动触发。它可以执行耗时更长的任务：

- 推导工具全量测试、系统测试和 fixture 回归。
- 全规格批量推导验证。
- trace SVG 全量生成，并作为 artifact 保存。
- `impl/arceos_ex` 的完整 `make build`、`make run`、`make run PROBE=announce`。
- QEMU smoke test，检查 smoke 汇总输出、独立 `APP=hello` 输出或 checkpoint 序列。
- 生成 unresolved obligations、deferred items、对象覆盖表、QEMU 日志等报告。

手动触发可用于规格大改后立即刷新展示结果，也可后续增加参数，例如指定 spec、指定 kernel implementation、是否运行 QEMU、是否发布 Pages。

### CurrentCPU 与锁上下文变更后的生成要求

后续若模型正式引入 `CurrentCPU`、`LocalInterruptControl`、`CurrentTaskSlot`、`PreemptionControl` 或 `RawSpinLock`，不能只完成规格验证就停止。规格验证通过后，必须检查对象级实现和代码生成指导是否受影响。

CPU/CpuGroup 相关代码生成必须先服从统一 CPU 实例模型：

- 每个 logical CPU 对应一个统一 CPU 类型的对象实例；`BootCPU` 是 logical id `0` 的 bootstrap-role 实例，不是单独类型。
- `hartid`、`logical_id`、`possible`、`present`、`active` 和 `online` 属于 CPU 实例事实；`CpuGroup` 只维护 `CpuGroup.Cpu[id] -> CpuRef -> CPU` 索引和 possible/present/online 集合视图。
- 不生成拥有 CPU 本体的 `PossibleCpu`、`PossibleRunQueue` 或独立 possible 集合对象。若实现需要 mask/table/storage，必须标注为 `CpuGroup` 的索引/集合视图承载。
- 当前 `impl/arceos_ex` 可以暂时保留 `boot_cpu` 与 `secondary_cpus` 分开的存储结构，但这只是 lowering 细节。对外 checkpoint、trace、测试和后续生成注释都应呈现为统一 CPU 实例和 logical-id 索引模型。
- `SecondaryCpuStore` 只允许作为 `impl/arceos_ex` 的 secondary CPU 实例承载，用来保存 AP 真正 online 前已经发现的 CPU 本体事实；它不是 formal 顶级对象，也不是新的 CPU group。所有对外可见的 CPU 成员关系仍必须通过 `CpuGroup.Cpu[logical_id] -> CpuRef -> CPU instance`、`CpuGroup.possible_cpus`、`CpuGroup.present_cpus` 和 `CpuGroup.online_cpus` 表达。`CpuGroup` 可以从 `SecondaryCpuStore` 刷新 `CpuView`，但不得把 store 暴露为调度、per-cpu、hotplug 或 root-domain 的身份来源。
- 不生成或保留独立 `CpuIdMap` 对象。logical-id 检查、hartid 唯一性、possible 集合边界和 `CpuGroup.Cpu[logical_id] -> CpuRef -> CPU instance` 解析都必须由 `CpuGroup` 的索引/集合视图直接承载。
- AP 真实进入 secondary entry 前，不得为 possible secondary CPU 生成 live AP `CurrentCPU`、`LocalInterruptControl`、`CurrentTaskSlot` 或 `PreemptionControl` 链。

重点检查范围：

- `CurrentCPU` 是否拥有对应 CPU 对象，`CpuGroup` 是否只维护这些 CPU 对象的引用、logical-id 索引、集合视图和拓扑组织关系。
- possible secondary CPU 是否已经作为 CPU 实例引用进入 `CpuGroup` 的 possible/present 视图；AP 真实进入 secondary entry 前，不得生成 live AP `CurrentCPU`，也不得把 AP 的 local interrupt、current task slot 或 task preemption 控制链视为可操作。
- `BootCPU` 是否仍作为独立实现对象存在，或已经退化为 `CurrentCPU.cpu` 指向 CPU 的 bootstrap role/alias。
- `InterruptStream.Enable/Setup` 是否仍直接维护 boot CPU 本地中断总开关事实，或已经改为只在 lifecycle event 中驱动 CPU-local `LocalInterruptControl`。
- 是否仍有 `InterruptStream` 或其它对象直接改写 `sstatus.SIE` 总开关；接管后只有 `LocalInterruptControl` 可以直接操作本 CPU 中断总开关状态。`InterruptStream` 可以直接管理 `sie/sip` source enable / pending 分开关。
- `BootRunQueue.curr`、`BootCPU` 当前任务事实和 scheduler setup 是否需要收敛到 `CurrentTaskSlot`。
- `preempt_disable()` / `preempt_enable()` 语义是否通过 `CurrentCPU -> cpu -> CurrentTaskSlot.current_task -> PreemptionControl` 表达。
- `KernelInitTask.Enable` 是否通过 `WakeUpNewTaskContext.guard` 绑定的 `RawSpinLock.LockIrqSave/UnlockIrqRestore` 进入和退出资源独占上下文，并只在该 guard 保护区内驱动受保护资源对象的 action/event。
- checkpoint、trace 注释、smoke case 和 KUnit case 是否仍引用旧的 `BootCPU` 或裸 `boot_cpu_current_is_idle_task(...)` 事实。

若上述检查表明正式对象、transition/action、trace checkpoint 或上下文边界发生变化，应只重新生成受影响部分，不得全局重排无关实现。预计受影响的实现范围包括 CPU/current 相关对象、`CpuGroup` logical-id 索引和集合视图、CPU-local interrupt 控制对象、task preemption 控制对象、raw spinlock wrapper、`rest_init` 中 `KernelInitTask.Enable` 路径、trace 输出以及 smoke/KUnit 测试注册。

有实现变化时，验证至少覆盖：

```text
make build APP=smoke
make test
make verify
```

同时应运行受影响的 KUnit 入口和 smoke case。smoke/KUnit 应覆盖 `CpuGroup.Cpu[0] -> BootCPURef -> BootCPU`、boot CPU possible/present/online facts、secondary CPU possible/present/not-online facts、unique logical-id/hartid boundaries、`CurrentCPU` 绑定、CPU-local interrupt save/restore、current task slot、task preemption disable/enable，以及 `RawSpinLock.LockIrqSave/UnlockIrqRestore` 驱动 `KernelInitTask.Enable` 的最小路径。若某项暂时无法实现，应在规格或 coding 文档中记录明确 deferred 边界，不得用普通 TODO 代替。

### 项目主页展示

项目主页应作为测试流水线结果的发布视图，而不是独立测试来源。第一阶段采用轻量 GitHub Pages 方案即可，例如从 `docs/` 或 workflow artifact 发布静态页面。

主页展示内容优先包括：

- 项目目标和当前阶段。
- 主规格、`spec/model`、`spec/coding`、`spec/compose` 的入口。
- 最新成功 nightly/manual 生成的 startup trace SVG。
- 推导摘要、unresolved obligations 和 deferred items。
- `impl/arceos_ex` 对象级实现进展、Makefile 命令和 smoke 测试结果。
- 生成时间、commit id 和 workflow run id。

自动生成内容应放在清晰的 generated 区域，例如 `docs/generated/` 或 Pages artifact。普通 PR/push 只检查生成脚本可运行，不直接发布主页；nightly 或手动触发成功后再发布 GitHub Pages。

## 外部 crate 整改

`arceos_ex` 必须遵守 Rust coding 规格中的 crate 信任边界。当前实现中已发现的直接外部 crate 使用需要整改：

- `fdt-parser`：不得作为黑盒依赖保留。后续应改为本项目维护的最小 FDT 解析实现，或先把可参考源码引入
  `components/` 后审查、裁剪和改造。
- `sbi-rt`：不得作为 `SBI.setup()` 的实现依赖扩大使用范围。后续 SBI 能力视图优先由本项目维护的最小 SBI ecall
  wrapper 建立；现有 checkpoint SBI 字符输出和平台关机路径也应逐步收口到本项目维护的 SBI 封装。
- 对上述 crate 的传递依赖也必须按同一规则处理，不能留下未审查的黑盒依赖。

## 第一轮最小对象覆盖

实现必须覆盖当前模型中已展开的 `EntryPreludePhase`、`EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase` 和 `PayloadPhase` 所需对象。Phase 对象可以是编排过程；非 Phase 对象原则上应有 Rust struct、静态单例或启动上下文字段承载。

重点对象包括：

- `BootArgs`
- `BootCPU`
- `RootStream`
- `InterruptStream`
- `KernelImage`
- `RawDtb`
- `PhysicalMemory`
- `PlatformCpuInfo`
- `BootInitTask`
- `BootInitStack`
- `Vm`
- `TrampolineVm`
- `EarlyVm`
- `SwapperVm`
- `FixMap`
- `EarlyDtb`
- `CommandLine` / `KernelCmdline`
- `Params` / `EarlyParam`
- `SBI`
- `PrintkBuffer`
- `EarlyCon`
- `MemBlock`
- `InitMM`
- `EarlyIoremap`
- `CorePreparePhase` 编排的最小对象骨架：`DeviceTree`、`Zones`、`ResourceTree`、`CacheBlockInfo`、`CpuCapabilities`、`CommandLine`、`PerCpuStorage`、`CpuHotplugState`、`Params`、`BootParam`、`PayloadParam`、`Randomness`、`ExceptionTable`。这些对象不是 `CorePreparePhase` 的下级对象；phase 只驱动其生命周期 transition。`SavedCommandLine` / `StaticCommandLine` 是 `CommandLine` 的子视图对象，不是独立顶级对象；`EarlyParam` / `BootParam` / `PayloadParam` 是 `Params` 的子对象。`PerCpuStaticImage`、`PerCpuFirstChunk` 和 `PerCpuOffsetTable` 是 `PerCpuStorage` 的子对象；first chunk 的 unit 个数和 offset table 边界必须基于 `CpuGroup` 的 possible CPU 集合，而不是 online CPU 集合。

`Randomness.preset()` 对应 `random_init_early(command_line)` 的早期语义。实现应建立 early seed material，并至少混入 `StaticCommandLine`；具体 mix/hash 算法不由 coding 规格限定。Linux 主线直接调用内部 `_mix_pool_bytes()`，不经过带 `input_pool.lock` 的 `mix_pool_bytes()`，因此实现不得为 `Randomness.preset()` 无条件生成 input-pool spinlock guard。当前 RISC-V64 最小实现允许记录 arch entropy 为 0，且 `Randomness.Prepared` 不得暴露正式随机数接口或表示完整 RNG ready。`crng_ready()` / `trust_cpu` 条件路径可能进入 `crng_reseed()` 或 `_credit_init_bits()`，并使用 `base_crng.lock` 的 `spin_lock_irqsave()` / `spin_unlock_irqrestore()`；当前最小实现可保持该条件路径 deferred，若后续实现则必须通过既有 `RawSpinLock` irqsave guard 协议表达。

`EntrySuccessorPhase.setup()` 对应 Linux `start_kernel()` early generic path 到 RISC-V `setup_arch()->paging_init()` 的入口后继切片。实现必须保留 `init_vmlinux_build_id()` 和 `page_address_init()` 的 deferred/position-preserved 事实，不能因当前没有 BuildId/PageAddress 对象而静默省略。`MemBlock.setup()` 必须记录 RISC-V `setup_bootmem()` 中的 `phys_ram_base`、`kernel_map.va_pa_offset`、DMA32 limit/zone input 和 hugetlb CMA reserve deferred facts；完整 `Zones` 仍由 `CorePreparePhase` 建模。`SwapperVm` 必须记录 `setup_vm_final()` 后的 SATP/TLB 同步事实，同时显式说明 `CONFIG_STRICT_KERNEL_RWX` 的最终 RW/RO/NX 权限细分仍未展开。

`PrintkBuffer.setup()` 对应 `setup_log_buf()` 的运行期准备语义。它不重新建立早期输出能力，而是在 `MemBlock`、`PerCpuStorage` 和 `BootParam` 已就绪后完成运行期日志缓冲准备；当前最小实现可以不扩容、不切换动态缓冲，继续使用静态 ring buffer，但必须标记运行期 printk 数据可用，且不得清空或丢弃 setup 时仍未 drain 的记录。

`ExceptionTable.setup()` 对应 `sort_main_extable()`。链接脚本必须提供 `__start___ex_table` / `__stop___ex_table` 边界，运行时必须基于该范围完成主异常表排序并提供按 faulting instruction address 查询的路径。当前表为空时也必须显式确认空表范围有效、排序事实成立且 lookup 返回 miss，不得仅以 checkpoint 代替该语义。

`EventStream.setup()` 安装的 formal trap entry 只负责读取 cause 并做第一层分流：interrupt cause 必须进入
`InterruptStream`，exception cause 必须进入 `ExceptionStream`。不得让 `ExceptionStream` 直接承担中断过滤职责。

`ExceptionStream` 和 `InterruptStream` 应维护 cause 到 handler policy 的静态分发表。当前最小实现可以使用固定大小数组或等价的静态表；每个表项初始必须绑定到 fallback panic/halt handler，不能留下空项、未初始化项或落到未定义行为。具体异常或中断对象完成 `Setup` / `Enable` 时，只能替换它所覆盖 cause 的表项。

异常 cause 映射至少应满足：

1. instruction/load/store page fault 绑定到 `PageFaultException`。
2. breakpoint 绑定到 `BreakpointException`。
3. user/supervisor ecall 绑定到 `SyscallException`，在 syscall 机制未启用前仍必须落到受控 panic/halt 或 disabled policy。
4. 其它同步异常绑定到 `UnexpectedException`。

`formal_event_entry` 必须先建立共享 trap context，再进入 `EventStream` 的第一层分流。该 context 至少应保存
`scause`、`sepc`、`sstatus`、`stval`，以及可透明返回所需的通用寄存器；当前最小实现 SHOULD 保存全部整数寄存器。
具体异常 handler 不应直接执行 `sret`，而应通过统一的 trap outcome 或等价约定表达 `Resume` / `Panic`：
`Resume` 由统一出口写回 `sepc`、恢复寄存器并 `sret`，`Panic` 仍走受控 panic/halt。

`BreakpointException.Setup` 安装的是 breakpoint trap 的 hook 分发入口，而不是“任意 #BR 均跳过返回”的终点
handler。其处理顺序 SHOULD 参考 Linux/RISC-V 的 `handle_break()`：先给 single-step/probe 类 hook 机会，再给
breakpoint hit hook 机会，后续可扩展 KGDB、BUG、CFI 等 hook。hook 输入为共享 trap context，输出应能表达：
已接管并 resume、已接管并 panic/halt、未接管。只有明确接管并要求 resume 的 hook 可以修改 `sepc` 并返回；
未知 #BR 或无人接管的 #BR 必须落入受控 panic/halt，不得默认跳过。

`Preset` 阶段仍只安装受控 fallback panic/halt；`Setup` 阶段才把 breakpoint cause 切换到 hook 分发入口；
`Enable` 保留给后续完整调试机制，例如断点管理、kprobe/kgdb 或等价设施。当前 smoke 若需要验证可返回路径，
应通过一个明确的 smoke/probe hook 接管测试断点，而不是依赖未知 #BR 的默认跳过行为。该 hook 若需要跳过
`ebreak` 或 `c.ebreak`，指令长度判断参考 Linux/RISC-V 的 `GET_INSN_LENGTH` 规则：读取 `sepc` 处半字，
若低两位为 `0b11` 则跳过 4 字节，否则跳过 2 字节。

其它 handler 当前仍可以返回 `!` 并执行 panic/halt；后续支持 syscall、timer interrupt 或 page fault recovery 时，
应复用同一套 trap context、handler outcome 和统一出口路径。

## DeviceTree unflatten 编码约束

正式 `DeviceTree` 对应模型中的 `DeviceTree.setup()`，参考 Linux 的 `unflatten_device_tree()` /
`unflatten_and_copy_device_tree()` 路径。它不同于早期 `EarlyDtb` 扫描：`EarlyDtb` 只提取启动早期事实，
`DeviceTree` 要建立后续运行期可遍历、可查询的树结构。

`DeviceTree.setup()` 必须把 `MemBlock.Online` 作为可分配早期物理内存的能力来使用，而不是只检查状态。
展开树的节点和属性元数据必须来自 `MemBlock` 早期分配；不得使用普通 heap、`Vec`/`Box`，也不得用固定静态数组作为正式展开存储。属性原始值可以引用生命周期受保护的 `RawDtb` 或内建 DTB 拷贝，但这种引用关系必须在对象状态中可解释，不能依赖已经销毁的 `EarlyDtb` 临时结构。

实现应采用两次遍历 `RawDtb` 的流程：

1. 第一遍校验 FDT 结构，并计算展开后节点、属性和必要元数据所需空间；若遇到格式错误、深度越界、大小溢出或无法映射的地址范围，事件必须失败，不能提交 `Ready`。
2. 通过 `MemBlock.alloc_phys(size, align)` 或等价的 `MemBlock` 事件接口申请物理存储，并通过 `SwapperVm`/`Config` 已建立的线性映射取得可写地址。
3. 第二遍填充 `DeviceNode`、property、root、parent/children 和查询索引或等价关系。

`DeviceTree.Ready` checkpoint 只能在第二遍完成，并且 root 唯一、非 root 节点 parent 唯一、parent/children 一致、路径查询和 property 查询均可用之后发出。涉及裸指针写入 `MemBlock` 分配存储的代码应封装在小的内部 unsafe 边界内，对外优先暴露安全的状态推进和查询接口。

## `PageMetadataMap` 编码约束

`PageMetadataMap.setup()` 属于 `CorePreparePhase`，必须在 `Zones.Ready` 之后、`trap_init()` 边界之前完成。当前参考配置为
`CONFIG_FLATMEM=y`、`CONFIG_SPARSEMEM=n`，因此它对应 Linux/RISC-V
`misc_mem_init() -> zone_sizes_init() -> free_area_init()` 中的 `alloc_node_mem_map()` 和 `memmap_init()`；
`sparse_init()` 在该配置下为空操作，不能作为本对象建立的主线证据。

当前 `arceos_ex` 实现采用 flat `mem_map` 形式：`PageMetadataMap.setup()` 在 `MemBlock.Online`、`Zones.Ready` 且
`SwapperVm.Online` 后，通过 `memblock.alloc_phys()` 分配一段页对齐的连续 metadata storage，并把它表示为以 PFN
为索引的 `PageMetadata` 数组。`PageRef` 必须绑定到 `mem_map[pfn - start_pfn]` 对应 slot；PFN、物理页地址和 direct-map
线性地址只是从该 slot 的 PFN 关系派生出的转换结果。将来若切换到 sparse/vmemmap，只能替换 metadata storage 布局，
不得改变 `PageRef` 指向 page metadata 项这一契约。

## `mm_core_init()` 编码约束

`MmCoreInitPhase` 已正式落到 `spec/model/phases/boot/mm-core-init/`。实现侧必须保持与模型一致的阶段边界：入口是 `CorePreparePhase.Ready`、`ExceptionStream.Ready`、`MemBlock.Online`、`PageMetadataMap.Ready`、`DmaCachePolicy.Ready`、`StaticBranch.Ready` 和 `SystemExclusive`；出口是 `PageAllocator.Ready`、`MemBlock.Offline`、`SlubSubsystem.Ready`、`PageTableCaches.Ready`、`VmallocAllocator.Ready`、`MmStructCache.Ready`。本阶段只能消费已经建立的 `PageMetadataMap`；`mem_init()` 开头的 `BUG_ON(!mem_map)` 对应 checkpoint，不在 `mm_core_init()` 内推进 `PageMetadataMap.setup()`。

`SystemExclusive` / `SingleTaskContext` 只提供当前 boot 调用点的上下文事实。它不能作为规格省略锁、irqsave、preempt、RCU、per-cpu 或 TLB/cache 同步语义的理由，也不再作为默认擦除 protocol guard 的依据。`mm_core_init()` 中每个对象/API 都必须明确属于三类之一：本阶段实际执行同步协议、纯上下文事实且无运行时协议、或后续 runtime consumer 触发时再展开。凡是 `Ready` 后暴露 runtime API 的对象，若完整并发协议尚未展开，必须保留显式 deferred contract，而不是用顶层 context 抵消。

`MemoryTopology.setup()` 只把 CorePrepare 已经 `Ready` 的 `Zones` 投影为 allocator-visible 的 `MemoryNode`/`ZoneSet` 视图；它不得重新划分 zone、不得从 `MemBlock` 分配或创建 `mem_map`/page metadata，也不得把 `ZoneSet` 表达为对真实 Linux `node_zones` 的新所有权。`PageAllocator.preset()` 只做 `build_all_zonelists(NULL)` 和 `page_alloc_init_cpuhp()` 对应的拓扑与 hook 建立：`ZonelistSet.Ready`、fallback zoneref 顺序、NULL sentinel、`CPUHP_PAGE_ALLOC` step 注册。Linux boot path 中 `__build_all_zonelists(NULL)` 在 `zonelist_update_seq` 的 `write_seqlock_irqsave()` 区间内执行，并包在 `printk_deferred_enter()` / `printk_deferred_exit()` 之间；当前实现必须执行并记录这两个 setup-time protocol 的 enter/exit 配平，且 `write_seqlock_irqsave()` 必须复用 `BootCpuLocalInterrupt.SaveAndDisable/Restore`。完整 runtime seqlock reader/retry 机制不在本阶段展开。`build_all_zonelists_init()` 随后为 possible CPU 初始化 boot pageset；实现必须把该 checkpoint 绑定到已 `Ready` 的 `PerCpuStorage` first chunk CPU 数。它不得释放 MemBlock 页，也不得把自身推进到 `Ready`。

`MemoryDebugHardening.setup()` 对应 `mem_debugging_and_hardening_init()` 的默认策略收敛。它必须依赖并记录已扫描的 `EarlyParam.Ready`，并使用既有 `StaticBranch` registry 写入 `InitOnAlloc`、`InitOnFree`、`DebugPageAlloc`、`DebugGuardPage` 和 `CheckPages` static keys；当前实现暂不支持 Linux `init_on_alloc`、`init_on_free`、page poisoning、`debug_pagealloc`、guard page 等 early-param 开关，必须把这些策略记录为 trimmed/default-policy facts，而不得让规格暗示已经完整消费参数语义。

`StackDepot.setup()` 对应 `stack_depot_early_init()` 的 `mm_init()` 调用点。当前参照配置中 `CONFIG_STACKDEPOT=y`，但 `CONFIG_STACKDEPOT_ALWAYS_INIT` 未选中，且 `PAGE_OWNER`、`DEBUG_KMEMLEAK`、`KASAN`、`KFENCE` 等 early 消费者未启用；在没有 `stack_depot_request_early_init()` 的条件下，Linux 该调用只记录 early init passed 并返回，不通过 memblock 分配 hash table。实现必须结构化记录 `config_enabled`、`early_init_passed`、`early_init_requested=false`、`early_table_allocated=false` 和 `late_init_deferred` 等事实；不得把 `StackDepot.Ready` 解释为 stack trace save/fetch API 已经 Online，也不得通过 smoke 暗示已有完整 stack depot storage。

`Swiotlb.setup()` 对应释放 MemBlock 前的 early SWIOTLB 决策和 static pool 结构初始化。若当前 policy 需要 early pool，必须记录 static pool area 已建立，且每个 area 的 lock 初始化完成；当前最小实现可把 early pool 抽象为单个 static area 和单个 lock。dynamic SWIOTLB 的 RCU/list/spinlock 路径、runtime bounce slot 分配、DMA map/unmap 和 sync 行为仍为 trimmed/deferred，不得通过 smoke 暗示已有完整 SWIOTLB API。

`PageAllocator.setup()` 才对应 `memblock_free_all()`。该事件必须先要求 `Swiotlb.Ready` 和 `MemoryDebugHardening.Ready`，再把 MemBlock free ranges 交给 buddy/free page sets，并通过 `MemBlock.Disable` 使 `MemBlock.state == Offline`。本阶段不得执行 `memblock_discard()`，不得把 `MemBlock` 推进到 `Destroyed`。

当前 minimal buddy 实现必须把 buddy free lists 建在 `PageAllocator` 内部，而不是作为依赖 heap 的外部容器。结构按
Linux-like `free_area[MAX_ORDER]` 建模：每个 zone 拥有按 order 索引的 free area，第一轮只保留单一 migratetype，
不展开 `MIGRATE_*` 分类、pageblock 迁移类型、compaction、reclaim 或 NUMA fallback。`PageAllocator.setup()` 遍历
MemBlock 可用区段并排除 reserved 区段，把剩余页切成 order 对齐的最大 buddy block；每个 free block 以 block head
对应的 `PageMetadata` 作为 free-list 节点，并在 metadata 中记录 buddy order、allocated/free 状态和必要链接。实现不得用
`Vec`、`Box`、普通 heap 或 SLUB/kmalloc 来承载 buddy 自身的 free-list 节点；这些 allocator 都必须建立在 buddy 之后。
free-list 必须采用 intrusive list：`BuddyFreeArea` 只持有 list head 和 `nr_free` 计数，链表节点来自 free block 的 head
`PageMetadata` 内嵌 link，语义对应 Linux `struct free_area.free_list[...]` 链接 `struct page.buddy_list`。
除 block head 外，同一 buddy block 内的其它页 metadata 不得作为该 block 的 free-list 节点。

`PageAllocator.Ready` 之后必须暴露正式的 Linux-like buddy API：`alloc_pages(order, gfp)`、order-0 convenience
`alloc_page(gfp)` 和 `free_pages(page_ref, order)`。model 层对应 `PageAllocatorType.Action::AllocPages(order, gfp) -> PageRef`
与 `PageAllocatorType.Action::FreePages(page_ref, order)`；coding 层可按 Rust 需要调整参数顺序或封装形式，但必须保留
order、GFP 约束、返回 `PageRef`、caller-owned 语义和释放时 order 必须匹配的契约。返回的 `PageRef` 表示 2^order 个连续
buddy pages，并且必须可通过已经建立的线性映射进行受限读写；它不得暴露 buddy free list 或 zone 内部结构。
完整 Linux runtime buddy 并发协议仍未在本阶段展开：zone lock、PCP lock、irqsave、preempt 规则和 reclaim/compaction 相关同步必须保留为 runtime deferred contract。当前 `alloc_pages/free_pages` smoke 只能证明正式 API 的最小对象语义，不证明通用多 CPU/中断/抢占环境下的完整锁模型。

`PageRef` 的实现语义应对齐 Linux `struct page *`：它引用 `PageMetadataMap` 中的具体 page metadata 项，而不是直接等同
物理地址或线性映射虚拟地址。`PageMetadataMap` 对应 Linux `mem_map` 或 RISC-V `SPARSEMEM_VMEMMAP` 下的 `vmemmap` 视图，
必须覆盖 PageAllocator 管理的 PFN 范围。实现应提供并内部复用 Linux-like 转换边界：`pfn_to_page(pfn)`、
`page_to_pfn(page_ref)`、`phys_to_page(phys)`、`page_to_phys(page_ref)`、`virt_to_page(linear_addr)` 和
`page_to_virt(page_ref)`/`page_address(page_ref)`。这些转换必须检查或依赖 PFN 有效、地址页对齐、地址属于已建立 direct map
或 vmemmap 覆盖范围；不得把任意整数地址直接伪造成 `PageRef`。

`SlubSubsystem.Ready` 之后必须暴露正式的 Linux-like kmalloc API：`kmalloc(size, gfp)`、`kzalloc(size, gfp)` 和
`kfree(alloc_ref)`。model 层对应唯一对象 `SlubSubsystem.Action::Kmalloc(size, gfp) -> KmallocAllocRef`、
`SlubSubsystem.Action::Kzalloc(size, gfp) -> KmallocAllocRef` 与
`SlubSubsystem.Action::Kfree(alloc_ref)`；coding 层可按 Rust 需要调整参数和返回封装，但必须保留 size、GFP、
caller-owned allocation reference、线性映射读写、kzalloc 返回前清零，以及 kfree 释放后对象回到所属 kmalloc cache 的契约。
`SlubSubsystem` 是 SLUB 子系统 facade，不是某个 cache 实例；规格不再引入 `SlubSubsystemType` 作为可复用类型。
单个 Linux `struct kmem_cache` 实例的正式类型名是 `SlubCache`，不是 `SlubCacheType`。
`SlubCacheRegistry` 是所有 `SlubCache` 实例的注册、查找和枚举集合；`SlubSubsystem` 通过 registry 间接管理
`boot_kmem_cache_node`、`boot_kmem_cache`、正式 `kmem_cache_node`/`kmem_cache`、kmalloc size-class caches
和后续 named caches，不能再建立与 registry 并列的第二套 cache 所有权。
`KmallocCaches` 只能实现为 size/size-class 到已注册 kmalloc `SlubCache` 实例的引用或索引视图，不得拥有这些 cache
实例，也不得维护一套与 `SlubCacheRegistry` 并列的实例生命周期。
凡是规格中命名为某个 Linux `kmem_cache` 的阶段对象，都只能作为该 named `SlubCache` 实例的阶段 owner/引用持有者；
实例所有权仍归 `SlubCacheRegistry`。当前实现必须至少把 `"page->ptl"`、`"vmap_area"`、`"mm_struct"`、
radix tree node cache 和 maple node cache 注册为 `SlubCacheRegistry` 中的 named cache，并在各自阶段 ready invariant
中确认 registry 可枚举该 kind。
实现中的 `SlubState` 必须保留 `Down`、`Partial`、`Up`、`Full` 四个边界，分别对应 Linux `slab_state` 到规格生命周期的
`Base`、`Prepared`、`Ready`、`Online` 映射。当前 `mm_core_init()` 只能推进到 `Up/Ready`；
`kmem_cache_init_late()` 只能通过 `SlubSubsystem.setup_flush_workqueue()` 记录 flush workqueue 事实，不得写入 `Full`；
`SlubSubsystem.enable()` / `Full/Online` 留给后续 `slab_sysfs_init()` 类 late initcall。
Linux `kmem_cache_init()` 在 bootstrap 阶段明确尚不需要 `slab_mutex`，因此当前实现不得伪造 slab_mutex lock/unlock；但这不表示 SLUB 没有锁模型。SLUB runtime 的 per-cpu/node/global locks、slab list mutation、slab mutex、sysfs/FULL 生命周期、CPU hotplug callback 和 debug/freelist hardening 同步都必须作为显式 deferred contract 保留，直到具体 runtime consumer 需要展开。

当前第一轮 `arceos_ex` SLUB/kmalloc 实现只要求 Linux-like page-backed slab：`KmallocCaches` 使用固定默认 size classes
`8/16/32/64/128/256/512/1024/2048/4096/8192`，请求 size 通过向上取整选择 size class；当某个 cache 没有空闲对象时，必须通过
`PageAllocator.alloc_pages()` 获取 backing page，把 page 切成同尺寸 slots 并挂入该 cache 的 freelist。4KiB page 配置下，
8KiB cache 必须使用 order-1 backing page，模拟 Linux `KMALLOC_MAX_CACHE_SIZE = PAGE_SIZE * 2` 的普通 kmalloc cache 边界。第一轮可先不在
空 slab 时把整页归还给 `PageAllocator.free_pages()`，但所有 backing pages 必须由 `PageAllocator` 拥有并通过 `PageRef`
建立 direct-map slot 地址。freelist 节点可以使用 slot 内存的 intrusive next pointer 或等价的固定元数据表示，但不得依赖
`Vec`、`Box`、全局 heap 或 MemBlock。`kmalloc` 不保证清零；`kzalloc` 必须清零返回对象的 requested size 范围；
`kfree` 必须拒绝不属于任一 kmalloc cache/slab 的引用，并把有效对象放回所属 cache 的 freelist。NUMA、per-CPU partial、
slab debug redzone/poison、freelist random/hardened、memcg kmalloc、reclaim/compaction 和 slab sysfs/FULL 状态均 deferred。

`KernelGlobalAllocator.Setup` 必须发生在 `SlubSubsystem.Ready` 之后，它表示 Rust `core::alloc::GlobalAlloc` 边界已经可用，
而不是新的底层分配器。`GlobalAlloc::alloc(Layout)` 必须通过 SLUB `kmalloc` 获得 storage；
`GlobalAlloc::alloc_zeroed(Layout)` 必须通过 `kzalloc` 或等价的 alloc 后清零实现；`GlobalAlloc::dealloc(ptr, Layout)`
必须能从裸指针和 layout 找回所属 kmalloc slab/cache，再把对象交回 SLUB。实现不得在这一层直接调用 MemBlock、直接操作 buddy
free list，或建立测试专用 heap。

第一轮 `arceos_ex` GlobalAlloc layout 支持范围应显式受限：`size > 0`，`size <= 8192`，alignment 不超过当前 kmalloc slot
天然能满足的范围。若实现通过 size class/page 对齐能够满足更大 alignment，可在 coding 注释和 smoke 中说明；否则必须对超出范围的
layout 返回 null/失败，而不是返回未满足 alignment 的地址。后续 large allocation、realloc、OOM policy、per-CPU cache 和特殊
alignment fallback 均 deferred。

`DynamicContainerRuntime.Ready` 表示普通 `Vec`、List、Set 等动态容器可以通过 `KernelGlobalAllocator` 获取 storage。它不得
让动态容器绕过 `GlobalAlloc` 直接拿 `KmallocAllocRef` 或 `PageRef`。后续平台总线 populate 若需要保存变长 `PlatformDevice`
集合，应依赖该 runtime，而不是恢复固定容量数组。当前 `DynamicContainerRuntime.Ready` 只承诺 documented layout subset
内的动态容器能力：`Vec` 增长导致的单次 `Layout` 若超过 `KernelGlobalAllocator` 第一轮 `size <= 8192` 边界，应以
allocation failure 处理，而不能被解释为 `Vec` 语义本身可用性失效。涉及 initcall 批量对象创建的 smoke 应覆盖这种边界，
避免普通 `push` 触发 `alloc_error_handler` 后只留下不可定位的关机日志。
`KernelGlobalAllocator` 和 `DynamicContainerRuntime` 的 runtime 同步能力继承自 SLUB/kmalloc；它们本身不消除底层 SLUB runtime 锁模型的 deferred 状态。

`VmallocAllocator` 的边界是 vmalloc/vmap 虚拟地址区间管理和 vmap 映射执行，不是物理资源策略层，也不是
MMIO 属性策略层。`PageTableCaches.setup()` 承担 RISC-V 当前主线的 `VMALLOC_START..VMALLOC_END`
页表范围预分配事实；`VmallocAllocator.setup()` 负责 `VmapAreaCache`、`VmapAddressSpace`、
`VmapNodeSet`、`VmapBlockQueues` 和 `VfreeDeferredSet`，并导入已有 `vmlist` 作为 busy areas、
建立 free vmap space。`PageTableCaches` / `SwapperVm` 提供“可以在 vmalloc 范围安装页表项”的能力；
`VmallocAllocator.map_page_range()` 是每次 vmap VA/PA 映射的执行者，不能只把这个能力表示为一次性的 ready bit。

`VmallocAllocator` 必须暴露与 model `VmallocAllocatorType` 对齐的最小运行期接口：申请/保留
`VmapArea`，基于调用方给出的物理区间或 pages/PFN 信息和 `PageProtection` 执行 page range 映射，
以及维护 `vm_struct`/`vmap_area` 元数据。当前第一轮实现可以只支持 runtime `ioremap` 需要的
`VM_IOREMAP` area 和 IO memory protection，但 API 命名和状态事实必须保持通用，不能把接口写成
ns16550a 或 console 专用路径。

`VmallocAllocator.map_page_range()` 的 installed fact 必须对应当前 swapper 页表里的真实 vmap VA/PA
映射安装。每次成功调用都必须产生一个独立的 `VmapMapping` 记录，绑定本次 `VmapArea`、调用方给出的
物理页/PFN/range、`PageProtection` 和已安装页表项；这不是重新推进 `VmallocAllocator` 生命周期，也不是把
全局 `runtime_page_table_mapping_ready` 位重复置真。若 `PageTableCaches` 没有提供 vmalloc install range，
或映射跨出当前已安装且可扩展的 vmalloc 页表范围，实现必须先通过 `PageTableCaches`/`PageAllocator`
按需补充 L0/PTE 页表页；补充失败时必须返回失败，不得只记录 `VmapMapping.installed=true`。
当前实现支持完整 `VMALLOC_START..VMALLOC_END` runtime vmalloc 地址空间：启动时只安装首批静态 L1/L0 页表，
后续当目标 area 触及尚未安装的 L1/L0/PTE window 时，`VmallocAllocator.map_page_range()` 必须请求
`PageTableCaches` 分配并清零新的页表页，并按需补充稀疏 slot metadata，再安装映射。该能力必须通过
ready/failure fact 暴露；超出 `VMALLOC_END` 的 area 必须被拒绝。`VmapArea` / `VmapMapping` 记录存储必须使用
动态容器 backing，不能在旧的固定测试 slot 数量处失败；但完整 Linux `vm_struct/vmap_area` 行为，如空洞复用、
增强树查找、lazy purge 批处理和并发/RCU 细节仍是后续边界。
具体实现不能只依赖底层 `vpn0` 越界失败；在调用页表安装前，`VmallocAllocator.map_page_range()` 必须显式确认
`VmapArea` 完全落在 runtime mapping window 能力内，必要时先扩展该能力。落到 `VMALLOC_END` 之外的 area
当前必须失败并记录为 range boundary，不能复用错误的 L0 表安装。同时，同一个 busy `VmapArea` 已经存在 installed
`VmapMapping` 时，第二次 `map_page_range()` 必须失败，避免一个 area 产生多个并存页表映射记录。
`VmallocAllocator.Ready` 不只表示 setup 数据结构完成，还表示后续 `get_vm_area()`、`map_page_range()`、
`unmap_page_range()` 和 `free_vm_area()` 的最小运行期契约已经发布：调用者进入这些 API 时必须满足 guard
条件；成功安装 PTE 后必须执行 RISC-V 当前实现可见的 kernel mapping sync/TLB 边界；失败路径不得留下 active
`VmapMapping`。若调用方已经保留了临时 `VmapArea`，则必须通过 `unmap_page_range()+free_vm_area()` 或
`free_vm_area()` 明确回滚该 area。当前实现使用同步 `sfence.vma` 表示本轮本地 kernel mapping
可见性边界；Linux RISC-V `new_vmalloc[]` 式跨 CPU lazy fault 修正、完整远端 shootdown batching、lazy
purge 和 RCU/free ordering 仍记录为 deferred，不得在本轮假装已完整覆盖。
`vmalloc_init()` 建立的 per-cpu `vmap_block_queue`、`vfree_deferred`、vmap node busy/lazy/pool locks 和 reclaim hook 都属于正式同步面。当前 setup 可在 `SystemExclusive` 下发布这些结构和 guard contracts；但 runtime vmap locks、per-cpu queue locks、vfree lazy purge、RCU/free ordering 以及跨 CPU vmalloc shootdown 不能因 boot context 省略，必须继续作为 deferred contract 或后续 consumer gap 处理。

`VmallocAllocator.unmap_page_range()` / `free_vm_area()` 必须保持 Linux-like teardown 顺序：先清除
对应 `VmapMapping` 的页表映射并记录 removed fact，再释放 `VmapArea` 的 busy/`vm_struct`/`vmap_area`
metadata。重复 unmap、重复 free、未 unmap 就 free 都必须失败。当前第一轮实现只做同步拆映射和元数据释放；
完整 `vunmap/vfree` 的 cache/TLB batching、lazy purge、per-CPU deferred free 和 VA 区间复用仍 deferred。
`Ioremap.iounmap()` 必须只请求这条 vmalloc/vmap teardown 路径：先由 `VmallocAllocator.unmap_page_range()`
清除绑定的 `VmapMapping` 页表项，再由 `VmallocAllocator.free_vm_area()` 释放对应 vmap area metadata，
最后把 ioremap cookie 标记为 retired/unmapped。`Ioremap` 不得自行清页表，也不得维护 vmap free-list。
重复 `iounmap()` 或对未知 cookie 执行 `iounmap()` 必须失败。

`Ioremap` 是 MMIO 策略调用方：它负责从设备资源得到物理 MMIO range、选择 `VM_IOREMAP` flag、
选择 IO memory protection、返回 `membase`/`__iomem` 语义并记录 not-linear-direct-map 事实。它不得维护
自己的 vmap bump allocator、不得持有 `next_vaddr` 这类 area 分配 cursor，也不得直接越过
`VmallocAllocator` 安装 vmap page range。`Ioremap.map_device_mmio()` 必须驱动
`VmallocAllocator.get_vm_area(...)` 和 `VmallocAllocator.map_page_range(...)`，然后只把返回的
`VmapArea`/`VmapMapping` 绑定进 `IoMemoryMapping`。
如果 `get_vm_area()` 已经成功但后续 `map_page_range()`、membase 计算或 ioremap mapping facts 校验失败，
`Ioremap` 必须通过 `VmallocAllocator.free_vm_area()` 或 `unmap_page_range()+free_vm_area()` 回滚；
不能把 `VM_IOREMAP` area 泄漏成 busy，也不能留下已安装但没有 `IoMemoryMapping` owner 的页表映射。
`Ioremap` 的 mapping/unmapping 同步继承 `VmallocAllocator` 的 guard、mapping sync 和 flush contract。当前 RISC-V plain-device path 可用本地 `sfence.vma` 证明本轮映射可见性；WC/NC/normal alias policy、远端 shootdown 和完整 teardown 仍不得标成已完成。

MMIO 属性必须显式建模，不能把“能通过 PTE 访问”偷换成“属性完整正确”。参照 Linux RISC-V
`_PAGE_IOREMAP`/`PAGE_KERNEL_IO`、`pgprot_noncached()` 和 `pgprot_writecombine()` 的分工，
`Ioremap` 必须区分 plain device、non-cache、write-combine 和 normal-memory alias 策略。当前实现只支持
plain device ioremap，并记录 RISC-V `_PAGE_IOREMAP` 风格的 IO memory attribute fact；non-cache、
write-combine、normal memory alias 先记录为 deferred/unsupported，不得返回成功 mapping 或让
`VmallocAllocator` 代替 `Ioremap` 选择属性。

测试必须覆盖这条分工：`VmallocAllocator` ready 后公开 area/mapping API；一次 ns16550a probe 后，
`VmallocAllocator` 至少记录一个 `VM_IOREMAP` area 和对应 IO memory page-range mapping；`Ioremap`
记录的 mapping 必须引用该 area/mapping，且 `membase != mapbase`、page-aligned、使用 IO protection。
属性测试必须覆盖 plain device 成功、WC/NC/normal 策略不被误标为已支持。

`MmStructCache.setup()` 只建立 `"mm_struct"` cache，并且该 cache 必须作为 `SlubSubsystem`/`SlubCacheRegistry` 管理下的具名 SLUB cache 实例注册；`MmStructCache` 本身不得成为新的 allocator 类型。`vm_area_struct` cache、`vma_lock_cachep` 和 `mmap_init()` 属于后续 `proc_caches_init()` 或进程地址空间初始化路径，不得为了填满本阶段而提前塞进 `MmStructCache`。

`PageExt`、`KFENCE`、`KMSAN`、`Kmemleak`、`DebugObjectsMemory` 和 `ExecMemory` 当前按 `../linux-6.12/.config` 记录为 model trimmed/no-op 路径。实现必须通过 `MmCoreTrimmedPaths` 这类结构化对象记录这些调用位置和裁剪原因，不得只散落 TODO 或只依赖文字 deferred；`execmem_init()` 的 no-op 依据是当前未选择 `CONFIG_EXECMEM`，因此走 `include/linux/execmem.h` 的 inline no-op，而不是把 `MODULES=n`、`BPF_JIT=n`、`KPROBES=n` 单独当作函数体裁剪依据。

## `SchedInitPhase` 编码约束

`SchedInitPhase` 已正式落到 `spec/model/phases/boot/sched-init/`。实现侧必须保持与模型一致的阶段边界：入口是
`MmCoreInitPhase.Ready`、`PageAllocator.Ready`、`SlubSubsystem.Ready`、`KmallocCaches.Ready`、`CpuGroup.Ready`、
`PerCpuStorage.Ready`、`CpuHotplugState.Ready`、`StaticBranch.Ready`、`PrintkBuffer.Ready` 和
`SystemExclusive`；出口是 `Scheduler.Online`、`RadixTree.Ready`、`MapleTree.Ready`、`Workqueue.Prepared`、
`Softirq.Prepared`、`RcuCore.Ready` 和 `TasksRcu.Prepared`。

目录、文件和对象命名必须跟阶段名一致：模型目录为 `spec/model/phases/boot/sched-init/`，实现文件为
`impl/arceos_ex/src/phases/boot/sched_init.rs`，阶段对象名为 `SchedInitPhase`。不得混用 `scheduler-init` /
`SchedulerInitPhase`，除非先正式改名并同步所有规格、图示和实现。

`Scheduler.preset()` 对应 `sched_init()` 的全局前置准备：默认 root domain、bit wait queue table 和调度类壳。当前
`SchedClass` 细分仍 deferred，调度类顺序检查只作为实现一致性检查或 checkpoint，不作为独立生命周期对象。

`DefaultSchedRootDomain` 必须按调度覆盖视图实现，而不是新的 CPU 身份表。它的 CPU 覆盖集合必须来自
`CpuGroup.possible_cpus`，并且集合元素必须是 `CpuRef` 或等价 CPU 引用；实现不得在 root domain 内重新拥有 CPU 本体，
也不得把 `possible_cpu_count` 这类裸计数当作完整规格事实。若实现使用 mask、数组或 compact table 承载覆盖集合，
必须能从每个 covered entry 解析回 `CpuGroup.Cpu[logical_id]` 的 CPU 引用，并满足
`DefaultSchedRootDomain.covered_cpus == CpuGroup.possible_cpus`。当前 `sched_init()` 仍不建立完整 SMP 调度拓扑，
RT/DL/EAS/load-balance 等共享调度状态可以保留字段或 deferred fact，但不能削弱 root domain 对 possible CPU 引用集合的覆盖约束。

`Scheduler.setup()` 编排 possible CPU 的 CPU-owned runqueue 元数据，并把 boot CPU 的当前 `InitTask/current`
建模为 `BootCPU.IdleTask`。它不得拥有每个 CPU 的 runqueue/idle task 本体，不得分配新的 boot idle task，不得创建第二个
runnable task，也不得把完整 SMP 调度拓扑提前塞进本阶段。`BootCPU.RunQueue.curr`、`BootCPU.RunQueue.idle`、
`BootCPU.idle_thread_ref` 和 per-cpu idle task 引用必须收敛到同一个 `BootCPU.IdleTask` / `BootIdleTask` 事实。
若当前 Rust lowering 仍把 `BootRunQueue`、`BootIdleTask` 存放在 `Scheduler` 字段中，代码注释、公开 API 和 smoke
必须把它们解释为 `BootCPU.RunQueue` 与 `BootCPU.IdleTask` 的物化视图，而不是 Scheduler 拥有的子对象。

每个 possible CPU 的 runqueue setup 必须以 `DefaultSchedRootDomain` 作为 attach target。当前实现可只完整物化
`BootCPU.RunQueue` / `BootRunQueue`，但聚合事实必须表达 Linux `for_each_possible_cpu()` 已为 possible CPU 建立
runqueue 元数据；`BootRunQueue` 必须保存或可解析自己的 `CpuRef == BootCPURef`，并在 attach 时验证该 CPU 引用属于
`DefaultSchedRootDomain.covered_cpus`。secondary CPU 在 online 前可以拥有准备好的 runqueue 元数据和 root-domain
覆盖事实；secondary CPU 的 idle task 身份跟随 idle thread / SMP bringup 路径推进，这不表示 AP 已有 live
`CurrentCPU`、current-task slot 或可执行任务流。

`Scheduler.enable()` 只表示 boot CPU 调度基础和主动调度入口可用，并设置 `scheduler_running` 等价事实。它不表示 timer tick、
中断调度、kthread 调度、secondary CPU 调度或 SMP domain 已经可用。

`Scheduler.schedule()` 是 `Scheduler.Online` 后的调度分界 event。当前阶段只能由实现或 smoke 主动调用；
不得依赖中断、tick、softirq 或 workqueue 触发。该 event 自己负责 `schedule()`/`__schedule()` 内部边界：
进入 schedule-owned preemption guard，关闭 boot CPU 本地中断，进入 runqueue lock context，先从当前 CPU 的
current-task 视图取得 `CurrentTaskRef`，再从 `CurrentRunQueueRef` 执行 `pick_next_task` 得到 `next`，
并经过 `Scheduler.switch_to(CurrentTaskRef, next)` 框架后再退出这些边界。调用方不得为了满足 `Scheduler.schedule()` 前置条件而直接裸写
`sstatus.SIE`；本地中断总开关必须通过 `LocalInterruptControl` 操作。当前 RISC-V `switch_to` 框架只模拟 Linux
`__switch_to` 的核心保存/恢复边界：每个 `Task` 拥有一个 `TaskThreadContext`，其寄存器组严格对应
`thread.ra`、`thread.sp` 和 `thread.s[0..11]`。保存/恢复必须通过 `TaskRef` receiver 对目标 task 的
`TaskThreadContext` 生效，`switch_to` 完成后必须通过本 CPU 的 `CurrentTaskSlot` 提交 next 已成为本 CPU
`CurrentTaskRef` 目标的事实，不能只依赖 `Scheduler` 计数或 `BootRunQueue.curr` 间接表示。当前 `rest_init`
首次调度从 `BootIdleTask` 选择已入队的 `KernelInitTask` 或 `KthreaddTask`，实现暂时固定优先
`KernelInitTask`，以支撑后续 `PreSmpInitPhase -> ... -> PayloadPhase` 的 KernelInit 执行线。当前
`switch_to` 的通用 action 仍负责 RISC-V 核心保存/恢复边界和 `CurrentTaskRef` commit；在 owner-split 对象事实
线性提交完成、`UpMultitaskPhase` 离开前，impl MUST 通过每个 `Task` 拥有的 `TaskSwitchContext` 保存
`BootIdleTask` 的真实 `ra/sp/tp/s0..s11` 并恢复 `KernelInitTask` 的 vmalloc stack context，再由
`kernel_init_entry()` 驱动 `PreSmpInitPhase -> ... -> PayloadPhase`。不得从 BootIdle 的调用栈直接调用
`smp_runtime::setup()` 或 selected payload。若 KernelInitTask 后续切回使 BootIdle continuation 恢复，
该 continuation 必须停留在无限 `schedule_idle()` 循环，不得继续执行 payload。KthreaddTask 当前入口可在
请求消费语义尚未展开时使用无限 `schedule()` 循环主动让出 CPU；循环次数和任务切换次数不得成为默认
hello 验收条件。MM/FPU/vector 切换、通用 `finish_task_switch()` 和完整 kthreadd 请求处理仍 deferred。
若调用路径来自 `schedule_preempt_disabled()`，
调用方继承的 preemption guard 退出和 post-schedule guard 重新进入必须在调用方上下文中显式建模。
真实 owner handoff 和 KernelInit entry 栈范围检查闭合后，生成的 `.boot.stack` 使用 codegen profile 的
16KiB 配置；该缩减必须由完整 `make test` 同时覆盖 native/linux-object 与 hello/user/smoke 矩阵。
默认 `make run` 的功能验收只要求 KernelInitTask 最终输出 `Hello, world!` 并关机，不绑定 BootIdle/Kthreadd
是否完成过一轮、具体切换次数或 switch 诊断输出顺序。

在 coding/codegen 层，模型中带显式参数和返回值的 action 可以 lowering 为统一入口形态：`Action(ContextRef, MutPacketRef)`。`ContextRef`
提供生产对象图入口，`MutPacketRef` 是该 action chain 的强类型、局部、schema 明确的临时 packet，用于承载同级 actions 之间传递的临时值，例如
`prev_ref`、`current_rq_ref` 和 `next_ref`。正式 model 层仍必须保留显式 action 参数、返回值和 `let` 绑定，不得把规格写成万能 packet 黑板。packet
只能保存临时值，不能保存长期对象事实；每个 action 可读写的 packet 字段必须由模型 lowering 或 coding 规格明确约束。采用统一 action 入口后，每个 action
的入口和出口都是潜在 checkpoint，action 内部的关键边界也可以通过 packet schema 暴露给 checkpoint/KUnit；对象方法不得为此反向抓取全局 `Context`。

`Scheduler.schedule()` 的 checkpoint/KUnit 应先从 action chain 前段向后覆盖：第一步检查 `PickNextTask` 退出点已经得到
`next_ref`，且 `rest_init` 首次 schedule 的 `prev_ref == BootIdleTask`、`next_ref` 是 `KernelInitTask` 或
`KthreaddTask`；当前实现固定优先 `KernelInitTask`。第二步检查 `SwitchTo` 进入点的
`prev_ref` 和 `next_ref` 与 pick result 一致，并且该进入点发生在本次 `CurrentTaskRef` switch commit 之前。
第三步检查 `SwitchTo` 退出点：第一次 `rest_init` schedule 返回时，`CurrentTaskRef` 必须指向
`PickNextTask` 选出的同一个 runnable task，即 `KernelInitTask` 或 `KthreaddTask`。更粗的
`Scheduler.Schedule.Exit` 后置 checkpoint 位于 `local_irq_restore()` 之后；第一次 `rest_init` schedule
返回时，`CurrentTaskRef` 必须仍指向所选 runnable task，`schedule_passes` 必须已经提交，并且本次 local interrupt
save/restore 计数配平。

`CurrentTaskRef` 在 `arceos_ex` 中必须按模型定义实现为 CPU 视角私有引用。当前 BP 路径只存在 `BootCurrentCPU` 的
`CurrentTaskRef`；它在 sched-init 后指向 `BootIdleTask`，在 `rest_init` 首次调度后更新为所选 runnable task。
实现不得新增 `CurrentTask` 描述性对象，也不得把 BP 的 `CurrentTaskRef`
当作所有 CPU 共享的全局 current task。RISC-V64 代码可以并且 SHOULD 参考 Linux 用 `tp` 寄存器实现 current-task
视图；`CurrentTaskSlot` 是对象级实现边界，在 RISC-V64 后端可以收敛到以 `tp` 承载或快速访问 current-task 引用，但通用规格和通用代码不把
`CurrentTaskSlot` 定义成 `tp` 本身。per-cpu 存储只作为其它 CPU-local 数据的实现方式，不应替代规格中的 CPU 视角定义。

`CurrentRunQueueRef` 同样必须按模型定义实现为 CPU 视角私有引用。它不是 `BootRunQueue` 的别名，也不得实现为
描述性 current-runqueue 对象或全局 singleton。实现应参考 Linux 的间接路径：先通过 `CurrentTaskRef` 得到当前 task，
读取 task 记录的 CPU id，再通过 CPUGroup/runqueue topology 解析该 CPU 的 runqueue。当前 BP 最小实现可以把这个解析固定到
boot runqueue，但代码和注释必须把该绑定标成 UP 临时特化；未来 SMP 泛化时应替换为基于 selected/current runqueue ref 的
`cpu_of(...)` 解析。

`RadixTree.setup()` 和 `MapleTree.setup()` 只建立 node cache 与全局分配基础；这两个 node cache 必须作为
`SlubCacheRegistry` 拥有的 named `SlubCache` 实例注册。具体 radix tree、IDR、XArray、maple tree
实例由后续使用者对象拥有，不在本阶段创建。

`Workqueue.preset()` 只覆盖 `workqueue_init_early()`：system workqueue、worker pool 壳、unbound cpumask、BH pool 和属性缓存。
它使 `Workqueue.state == Prepared`，表示可以创建 workqueue 和排队/取消 work item；不得解释为 worker kthread 已创建或可执行。

`Softirq.preset()` 是模型显式补充动作，用于在 `RcuCore.setup()` 前建立 `SoftirqActionTable`、slot 和 per-CPU pending bit
承载壳。它不得执行 softirq，不得建立 tasklet 队列；`softirq_init()` 对应的 `Softirq.setup()` 留给下一子阶段。

`RcuCore.setup()` 覆盖 `rcu_init()` 的共同核心设施：boot CPU online 事实、RCU softirq 注册、RCU workqueue 基础和
Tasks RCU callback-list 壳。`TasksRcu` 在本阶段只允许推进到 `Prepared`；GP kthread 创建和 `TasksRcu.Ready` 属于后续
`rcu_init_tasks_generic()` 路径。

## `IrqTimeInitPhase` 编码约束

`IrqTimeInitPhase` 已正式落到 `spec/model/phases/interrupt/irq-time-init/`，属于 `InterruptPhase` 的第一个子阶段。实现侧边界和早期设计草案不同：`local_irq_enable()`
不属于本阶段，而是独立的 `LocalIrqEnablePhase`。阶段出口必须满足 `InterruptStream.Ready`、boot CPU
`sstatus.SIE` 仍关闭、`early_boot_irqs_disabled == true`、`IrqController.Ready`、`RiscvIntc.Ready`、
`IrqDispatchTree.Ready`、`Plic.Ready`、`Tick.Ready`、`TimerWheel.Ready`、`SrcuCore.Ready`、
`HrtimerCore.Ready`、`Timekeeper.Ready`、`RiscvTimerProvider.Ready`、`Softirq.Ready`、`Randomness.Ready`、
`BootStackCanary.Ready`、`PerfEventCore.Ready`、`ProfileCore.Ready`、`SbiIpi.Ready`、`IpiMux.Ready` 和
`SmpCallFunction.Ready`。

`init_IRQ()` 的 RISC-V arch 边界必须显式保留。当前 `../linux-6.12/.config` 为
`CONFIG_IRQ_STACKS=y`、`CONFIG_VMAP_STACK=y`、`CONFIG_SHADOW_CALL_STACK=n`，因此实现必须用
`RiscvIrqStackSet` 或等价对象记录 `init_irq_stacks()` 为每个 possible CPU 建立 IRQ stack pointer 的事实，并记录
`init_irq_scs()` 因 shadow call stack 未启用而 trimmed/no-op。真实 hardirq entry 的 `call_on_irq_stack()` 栈切换仍属于运行期
中断入口语义，本阶段只能记录 deferred contract，不能声明完整 hardirq stack switching 已 Online。

`timekeeping_init()` 的锁协议不能被外层 boot-exclusive context 吞掉。实现必须同时记录 `timekeeper_lock` 的
`raw_spin_lock_irqsave()`/`raw_spin_unlock_irqrestore()` 事实和 `tk_core.seq` 的 `write_seqcount_begin/end` writer 事实；
这两类事实都应进入 `Timekeeper.Ready` 或相邻 ready check。reader retry、NTP 运行期更新和完整 clocksource watchdog
仍按后续运行期语义处理。

`rcu_init_nohz()` 和 `kfence_init()` 是本阶段 Linux 调用序列中的真实调用点。当前配置下 `CONFIG_RCU_NOCB_CPU=n` 使
`rcu_init_nohz()` 走 `include/linux/rcupdate.h` 的 inline no-op，`CONFIG_KFENCE=n` 使 `kfence_init()` 走
`include/linux/kfence.h` 的 inline no-op；实现必须通过 `IrqTimeTrimmedPaths` 这类结构化对象记录调用位置和裁剪依据，不能只依赖
缺省未实现或 markdown 说明。

目录、文件和对象命名必须跟阶段树一致：模型目录为 `spec/model/phases/interrupt/irq-time-init/`，实现文件位于
`impl/arceos_ex/src/phases/interrupt/irq_time_init.rs` 等 `interrupt` 阶段子树下；旧
`phases/boot/irq_time_init.rs` 路径不得再作为本阶段实现位置。

当前按 OpenSBI 下的 RISC-V S-mode 路径建模：`RiscvIntc` 表示每 hart 直连 CPU 的 local interrupt controller，
`RiscvTimerProvider` 表示 Linux `timer-riscv` 风格的 time/clockevent provider，timer programming 走 SBI TIME 或后续 SSTC
能力，而不是直接把 CLINT 作为本阶段对象。IPI 路径按 `SbiIpi` 提供 software IRQ mapping 和 send action，`IpiMux` 在其上建立虚拟
IPI range；这对应 Linux `sbi-ipi` + generic `ipi-mux`。

PLIC 是系统 irqchip，不是普通 `PlatformBus` driver probe。实现必须采用 Linux-like
`init_IRQ() -> irqchip_init() -> of_irq_init()` 链路：`PlicDriver` 的 compatible/init callback entry
通过 retained LDS section 静态注册，`IrqChipInitTable.preset()` 只建立该 section 的 entry view，
`IrqChipInitTable.setup()` 才遍历 section、匹配 DeviceTree 中带 `interrupt-controller` 的 PLIC node，并调用匹配 entry
的 PLIC init callback。代码不得在 `IrqTimeInitPhase` 中直接调用 PLIC 专用 setup 来绕过 section 遍历，也不得用运行时
growable registry 作为主注册路径。当前必须至少支持 QEMU virt 常见的 `sifive,plic-1.0.0`，可同时支持
`riscv,plic0`。

`Plic` 在本阶段推进到最小 `Ready`：它确认 provider discovery、DeviceTree interrupt-controller node、compatible
match、callback 调用事实、`reg` MMIO resource、`riscv,ndev` source count、`interrupts-extended` 中的 RISC-V external
interrupt parent input，以及 PLIC 输出连接到上级 `RiscvIntc` external interrupt input 的父链关系。PLIC MMIO 必须通过
runtime `ioremap`/`vmalloc` 映射执行路径建立，但 owner 是 system irqchip，而不是伪造的 `PlatformDevice`。PLIC 的
context 选择必须反查 boot hart 的 `riscv,cpu-intc` phandle，并在 PLIC `interrupts-extended` 中匹配同一 phandle 的
`SUPERVISOR_EXTERNAL_IRQ` 项；不得因为 QEMU boot hart 变化而启用其它 hart 的 PLIC context。

UART 外部中断建模必须分清两条方向相反的链。物理传播链是 `UART -> PLIC -> RiscvIntc -> CPU`：UART 作为 PLIC
source，PLIC output 接到 hart local INTC 的 supervisor external input，最后 CPU 被 interrupt。该链要真正导通，需要两个
独立 enable gate：PLIC 上 UART source 的 enable gate，以及 root INTC/`InterruptStream` 上 supervisor external input 的
enable gate；二者都不同于 `sstatus.SIE` 总开关。当前必须定义这两个 gate 的身份和 closed 状态：root gate 在
`RiscvIntc`/`InterruptStream` 安装 supervisor external route 时定义，UART source gate 在 `PlicIrqMapping` 绑定
`HwirqRef::PlicUart0` 到 UART logical IRQ 时定义。两个 gate 的显式 Enable action 必须由单独的
`UartExternalIrqEnable` 边界执行：PLIC source gate 对应 PLIC context enable bitmap / source priority，root gate 对应
RISC-V INTC `sie.SEIE` unmask。`UartExternalIrqEnable` 仍不得制造 UART interrupt；真实一次性 UART THRE trigger 必须由后续
`UartInterruptChainProbe` 生产边界执行。serial8250 console 仍保持 polling，不得声明 interrupt-driven ready。

软件处理/溯源链是 `CPU -> RiscvIntc -> PLIC -> UART`。运行期 dispatch contract 必须仿照 Linux：RISC-V root INTC
`EXT_IRQ`/SEI entry 只转交给 PLIC chained handler，PLIC handler 先从 claim 寄存器读取 source，经
`PlicIrqDomain`/generic IRQ core dispatch 到已注册 action，handler 返回后再把同一个 source 写回 claim register complete；
claim 返回 0 表示没有 pending source，不得调用 UART handler。PLIC chained handler 必须按 Linux
`while ((hwirq = readl(claim)))` 形状循环 claim，直到 claim 返回 0 才退出；每个非零 claim 都必须完成一次
domain dispatch 尝试，并对同一个 claimed source 执行 complete。缺失 mapping 或缺失 action 可以记录失败并继续保证
complete，但不得把该次 dispatch 计为成功 UART handler。

所有中断类型必须用命名 cause 表达，不得在规格或实现控制流中用裸数字描述。RISC-V supervisor external interrupt 在规格中
称为 `InterruptCauseRef::SupervisorExternalIrq`，代码中应使用 `SUPERVISOR_EXTERNAL_IRQ`、`EXT_IRQ` 或同等架构命名常量；
timer/software/external 等其它 cause 也适用同一规则。

IRQ domain/source mapping 必须分清类型和实例：`IrqDomain` 是 IRQ core 的通用 mapping contract，
`PlicIrqDomain` 是由 PLIC provider 创建的具体实例。`PlicIrqDomain` 只负责把一格 PLIC interrupt specifier
翻译成 PLIC source，并把合法 source 映射成 logical IRQ；source 0 必须视为 reserved/invalid，source 必须落在
`1..=riscv,ndev` 范围内，重复映射同一个 source 必须返回已有 logical IRQ 而不是新增记录。该层不得 enable PLIC
source、不得注册 handler、不得执行 claim/complete 或 dispatch；建立 UART source mapping 时只允许把对应 source gate
定义为 closed，并记录 source enable deferred。

`UartExternalIrqEnable` 只能在 UART IRQ resource、PLIC source mapping 和 `IrqHandlerRegistry` 中的 UART handler action
都 ready 之后执行。它可以打开 PLIC UART source gate 和 root INTC supervisor external input gate，但仍不得直接触发 UART
中断、调用 handler、执行 PLIC claim/complete，或把 serial8250 console 标记为 interrupt-driven。

`UartInterruptChainProbe` 是生产侧一次性验证边界，不是 KUnit handler。它只能在 `UartExternalIrqEnable` ready 后执行，
按照 Linux 8250 startup/THRI 语义打开 UART interrupt-output 前置条件：至少设置 MCR.OUT2，启用 `UART_IER_THRI`，
并通过一次真实 TX empty 转换产生 THRE edge；不得只写 `UART_IER_THRI` 后假设硬件必然立即产生中断。随后等待真实
trap/root INTC/PLIC/IRQ core 路径推进到 `PLIC claim -> logical IRQ dispatch -> UART handler -> PLIC complete`。
该 probe 不得只观察第一次 handler 调用；它必须从触发前 snapshot 观察一轮完整 IRQ cycle：UART THRE request、
PLIC 非零 claim、IRQ dispatch、UART handler、PLIC complete、零 claim loop exit 都发生，并确认本轮非零 claim 与
complete 在当前 UART source 维度成对。全局 claim/complete counter 只能作为辅助观测，不能替代 source-scoped delta，
也不能因其它 source 或并发采样窗口导致的全局 delta 不相等而判定本轮 UART source cycle 失败。零 claim 与 loop exit
的闭合要求是二者都在同一轮真实 claim loop 边界内被观察到；若 provider 以独立 counter 记录 zero claim 和 loop exit，
等待和诊断不得把瞬时 counter 不相等作为失败事实。UART handler 必须清掉 THRI，
避免中断风暴；该 probe 仍不得把 console 输出切换为 interrupt-driven。
KUnit/smoke 只能读取该 probe 和计数结果，不能直接调用 trigger、root intc entry、PLIC claim/complete、IRQ dispatch 或
UART handler。

`UartInterruptChainProbe.setup` 失败时必须通过统一 `failure_diagnostic` payload 细分内部首个失败事实：
`phase=InitcallPhase step=setup_objects.uart_interrupt_chain_probe.setup object=UartInterruptChainProbe check=uart_interrupt_chain_probe.setup first_failed=<stable-predicate-name>`。
PLIC provider 必须提供按 source 维度的长期观察计数，至少包括 `claim_count_for_source(source)`、
`dispatch_count_for_source(source)` 和 `complete_count_for_source(source)`。这些计数是 PLIC/IRQ-domain/IRQ-registry
通用观察事实，不属于 UART 或 DF-0002 专用日志；native provider 与 Linux-object provider 都必须通过同一薄
provider contract 暴露。`last_claimed_source` / `last_completed_source` 只能作为辅助诊断线索，不能作为判断某个
source 是否已经发生 claim/complete 的主判据，因为后续其它 source 可以合法覆盖全局 last source。
`UartInterruptChainProbe`、serial8250 RX/TX probe 和 KUnit observer 判断一轮 UART source IRQ cycle 时，必须优先比较
对应 source 的 claim/dispatch/complete delta；只有 source-scoped delta 不足时，才报告
`plic.source_claim_observed`、`plic.source_dispatch_observed` 或 `plic.source_complete_observed` 等稳定失败事实。
`first_failed` 必须按实现判定顺序使用长期稳定名称，第一批至少覆盖：
`uart_interrupt_chain_probe.lifecycle_base`、`uart_external_irq_enable.state_ready`、
`uart_external_irq_enable.plic_source_gate_open`、`uart_external_irq_enable.root_external_input_gate_open`、
`plic.state_ready`、`plic_irq_domain.state_ready`、`irq_handler_registry.state_ready`、
`uart8250_port.logical_irq_ready`、`uart8250_irq_handler.registered`、`uart8250_port.logical_irq_valid`、
`plic_irq_domain.uart_mapping_present`、`plic_irq_domain.uart_mapping_logical_irq_matches`、
`plic_irq_domain.uart_source_gate_open`、`irq_handler_registry.uart_handler_present`、
`uart_interrupt_chain_probe.trigger_uart_thre_once`、`uart_interrupt_chain_probe.irq_cycle_wait_closed`、
`uart_interrupt_chain_probe.uart_trigger_committed`、`uart_interrupt_chain_probe.plic_claim_observed`、
`uart_interrupt_chain_probe.irq_dispatch_observed`、`uart_interrupt_chain_probe.uart_handler_observed`、
`uart_interrupt_chain_probe.plic_complete_observed`、`uart_interrupt_chain_probe.plic_loop_exit_observed`、
`uart_interrupt_chain_probe.irq_cycle_closed` 和 `uart_interrupt_chain_probe.console_polling_preserved`。
这类 diagnostic 不是 checkpoint，不得在成功路径输出，也不得替代后续规格化的 UART/PLIC/IRQ 长期对象事实。

`Serial8250ConsoleIrqTxProbe` 必须在 `UartInterruptChainProbe` ready 之后执行。它可以把 serial8250 console 从
handoff 后的 polling/backend-deferred 状态推进到 runtime interrupt-driven TX 状态，但必须通过公开 `printk` 前端提交
一条探针输出，由真实 THRI 中断驱动 UART handler drain TX queue，并观察 TX queue kick、UART handler drain、PLIC
claim/complete、zero-claim loop exit、queue empty 和本地中断保存/恢复 guard facts。该 probe 是生产边界，不是
KUnit handler；KUnit 只能读取它留下的 fact/counter/sink 诊断，不能参与 TX drain 流程。
普通 `make run`/app smoke 路径不得默认执行会产生用户可见 payload 的 TX probe，例如固定 printk burst、long burst 或
ordinary TTY write 的 `W`/`tx04` 测试字节；这些 payload probe 只能在 checkpoint/KUnit 测试构建中由生产路径按顺序触发，
然后由只读 KUnit observer 校验事实。KUnit handler 本身仍不得写字符、调用 printk、触发 IRQ、claim/complete 或直接推进
handler/drain。

kernel printk 是当前 serial8250 runtime TX 的主线。`Serial8250ConsoleBurstIrqTxProbe` 或等价生产边界必须在
`Serial8250ConsoleIrqTxProbe` ready 后执行，通过公开 `printk` 前端连续提交多条固定探针输出，随后仍由真实
THRI/PLIC/IRQ/runtime handler 路径 drain serial8250 console TX queue。验收至少要求 write call 增量匹配记录数、
CRLF 后的 drain 增量匹配预期字节数、TX queue empty、last byte 匹配、无 queue overflow、本地中断 guard 可观察、
PLIC claim/complete/zero-claim loop exit 闭合，并确认 ordinary `TtyXmitFifo` 的 enqueue/dequeue/runtime drain
计数不被 printk probe 修改。该轮不得引入 file/stdout/stdin、line discipline 或用户态 write/read 语义；这些只列为后续缓行。

serial8250 TX drain 必须显式建模和实现 `tx_loadsz` 预算。`Serial8250RuntimePort::TransmitChars` 或等价 handler 路径
每次 THRI handler 最多提交 `tx_loadsz` 个 console TX byte；如果 console TX queue 仍非空，必须保持 THRI enabled，
等待下一次真实 THRE/PLIC/IRQ handler round 继续 drain；只有 queue 为空时才执行 `StopTx`/clear THRI。`tx_loadsz`
属于 8250 runtime/port 策略，不属于 `printk` frontend、PLIC、IRQ domain 或 KUnit observer。`Serial8250ConsoleLongIrqTxProbe`
或等价生产边界必须在 burst printk probe ready 后执行，通过公开 `printk` 提交一条 CRLF 展开后超过单轮 `tx_loadsz`
的固定消息，并观察至少两个真实 PLIC claim/IRQ dispatch/UART handler/complete 回合、budget hit、drain byte 增量精确匹配、
queue empty、last byte 匹配和 ordinary `TtyXmitFifo` 未变化。KUnit/smoke 仍只能读取这些事实和诊断，不能手动推进 drain。
`Serial8250ConsoleLongBurstIrqTxProbe` 或等价生产边界必须随后通过公开 `printk` 连续提交多条固定长记录；每条记录
CRLF 展开后都必须超过单轮 `tx_loadsz`，真实 PLIC claim/IRQ dispatch/UART handler/complete 回合至少按每条记录
各自 `ceil(record_bytes / tx_loadsz)` 后累加计算，并要求 budget hit、write call 数、CRLF 插入数、
总 drain byte、last byte、queue empty、无 overflow 和 ordinary `TtyXmitFifo` 未变化全部匹配。该 probe 仍不得直接调用
serial8250 console/backend、不得暴露 printk flush API，也不得引入 file/stdout/stdin 或用户态 write/read 语义。
`Serial8250ConsoleTxQuiesceProbe` 或等价生产边界必须在 long-burst probe ready 后执行一个不提交新 printk 的静止性检查：
它只能读取 TX/IRQ/PLIC/TTY 计数快照并做 bounded idle observation，不得调用 `printk` 前端、不得 kick THRI、不得调用
root INTC/PLIC/IRQ/UART handler，也不得修改普通 `TtyXmitFifo`。验收至少要求 console TX queue 仍为空、THRI 已由
handler stop/clear、printk write/kick/drain 计数不变、PLIC claim/IRQ dispatch/UART handler 计数不增加、ordinary
TTY xmit FIFO enqueue/dequeue/runtime drain 计数不变、无 console TX overflow。该 probe 用于防止长 printk burst 后出现
空转 TX interrupt 或重复 drain；它不是公开 `printk::flush()`，也不推进 file/stdout/stdin 或用户态 I/O 语义。

`ns16550a` 的 platform probe 在解析 MMIO、寄存器宽度和 clock 之外，还必须从自己的 DeviceTree node 解析 UART IRQ
resource：读取 `interrupts` specifier，解析直接或继承的 `interrupt-parent`，确认父节点是当前 PLIC irqchip，然后经
`PlicIrqDomain` 建立 UART source 到 logical IRQ 的记录，并把 logical IRQ 保存在 `Uart8250Port`。这一步只说明外部中断链的
资源和 logical IRQ 绑定已经建立；platform probe 当场仍保持 polling，UART interrupt output 继续记录为 deferred，直到
`UartInterruptChainProbe` 已证明一轮外部中断链闭合后，initcall 生产路径显式切换到 interrupt-driven console TX；
可见 TX load/pressure probe 是否执行由 checkpoint/KUnit 测试配置决定，不属于普通启动输出。

handler registry 必须作为 IRQ core 侧对象建模和实现。`IrqHandlerRegistry` / `IrqAction` 记录 `request_irq`
风格的 logical IRQ -> handler 绑定，输入 logical IRQ 必须已经由 `PlicIrqDomain` 映射；未映射 logical IRQ 注册必须失败，
重复注册同一个 logical IRQ/device 必须按显式 duplicate policy 拒绝或保持幂等。`ns16550a` probe 可以在 UART logical IRQ
ready 后请求注册最小 UART handler 记录，但不得把 handler 表藏在 UART driver 私有状态里，也不得因此 enable PLIC source、
安装私有 claim/complete route，或把 serial8250 console 标记为 interrupt-driven。IRQ core dispatch 必须使用
`PlicIrqDomain` 返回的 logical IRQ，只能调用 `IrqHandlerRegistry` 中已经注册的 action；缺失 mapping 或缺失 action 不得视为
UART interrupt 成功。handler action 必须携带 hardirq context requirement，dispatch 时不得打开 sleep/process-only 路径。

checkpoint KUnit handler 默认必须接收只读 `Context`。允许写入的能力必须通过显式 sink capability 传入，例如 KTAP 输出、
tracer 或 auditor；sink 只能记录、审计或输出诊断，不得暴露对 `Context`、lifecycle 状态、IRQ 状态、设备状态或 scheduler
状态的可写访问。现有 `&mut Context` handler 属于兼容迁移路径，新增 observer 应优先使用 `(&Context, &mut sink)` 形式。

UART 外部中断链的 checkpoint KUnit 必须是旁路观察者，而不是流程参与者。它只能读取 trace、counter 和对象事实 snapshot，
并且只能通过受限 sink 写诊断；不得直接调用 UART handler、root intc entry、PLIC claim/complete、`request_irq` 或
source-enable API，也不得手动修改 pending/claimed/enable 状态来伪造链路推进。真实链路必须由实现路径推进：UART 发出中断、
hart 收到 external interrupt、root intc 分派到 PLIC、PLIC claim、IRQ core dispatch、UART handler、PLIC complete；
KUnit 只能断言这些步骤前后的可观测事实。

本阶段打开的只是 boot CPU 本地中断总入口。普通任务并发、secondary CPU 并发、周期 tick 服务、workqueue worker
kthread、RCU GP kthread、IPI enable 和完整 softirq 执行路径仍不得提前解释为 Online。

## `LocalIrqEnablePhase` 编码约束

`LocalIrqEnablePhase` 已正式落到 `spec/model/phases/interrupt/local-irq-enable/`，属于 `InterruptPhase` 的第二个子阶段。它必须接在
`IrqTimeInitPhase.Ready` 之后运行，且只能执行 boot CPU 的 `local_irq_enable()` 边界：通过
`InterruptStream.enable()` 先把 `early_boot_irqs_disabled` 清为 false，再打开 RISC-V `sstatus.SIE` 本地中断总入口；该顺序必须匹配
Linux `start_kernel()` 中 `early_boot_irqs_disabled = false; local_irq_enable();`，不得留下 SIE 已开但 early flag 仍为 true 的窗口。
本阶段不得打开 PLIC UART source gate、root INTC supervisor external input gate、周期 tick、完整 softirq、IPI runtime、
workqueue worker、RCU GP kthread、task concurrency 或 SMP concurrency。

`LocalIrqEnablePhase` 不得包在 `within` 上下文中。它本身就是 boot CPU local interrupt context 从 disabled 切到 enabled
的独立边界，因此不存在能覆盖整个 transition 的词法 local-interrupt context；`IrqOpenPreparePhase` 之后才进入
`SingleTaskInterruptStreamContext`。

阶段 ready check 必须保留上述负向事实的可观察性：root supervisor external input gate 仍 closed/deferred，PLIC UART
source enable 仍 deferred，softirq execution 仍 closed，IPI runtime 仍 deferred，workqueue workers 和 RCU GP kthreads
仍未运行，task/SMP concurrency 仍 closed。

`RiscvTimerProvider.setup()` 可以提供两个最小 action：`read_time()` 和 `schedule_oneshot(delta, callback) -> Option<deadline>`。smoke 必须分开验收
时间功能和时钟中断功能：前者确认 time source 可读且单调推进；后者注册一次性 clockevent callback，使用 SBI timer 在 deadline
到来时触发 supervisor timer interrupt，并确认 handler 返回前调用关联函数。该 smoke 不表示完整周期 tick 或 clockevent 运行期已经启动。

`poking_init()`、`ftrace_init()` 和 `context_tracking_init()` 当前按 `../linux-6.12/.config` 的 RISC-V64 配置记录为 trimmed/no-op。
`early_trace_init()`、`trace_init()` 和 `housekeeping_init()` 当前保留为 model `deferred`。实现若遇到这些调用位置，应按模型记录
checkpoint 或 no-op 条件，不得以零散 TODO 代替正式 deferred。

## `IrqOpenPreparePhase` 编码约束

`IrqOpenPreparePhase` 已正式落到 `spec/model/phases/interrupt/irq-open-prepare/`，属于 `InterruptPhase` 的第三个子阶段。它必须接在
`LocalIrqEnablePhase.Ready` 之后运行，此时 boot CPU 本地中断总入口已经开放；本阶段不得再执行
`local_irq_enable()`。

目录、文件和对象命名必须跟阶段树一致：实现文件位于
`impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs` 等 `interrupt` 阶段子树下，不得落回 `boot` 阶段子树。

`kmem_cache_init_late()` 当前只实现为 `SlubSubsystem.setup_flush_workqueue()` 内部资源事实：依赖
`SlubSubsystem.Ready`、`KmallocCaches.Ready` 和 `Workqueue.Prepared`，记录 `flush_workqueue_ready == true`。
它不得把 `SlubSubsystem` 推进到 `Online`，Linux `slab_state = FULL` / `SlubSubsystem.enable()` 仍留给后续
`slab_sysfs_init()` 类 late initcall。

`console_init()` 当前只要求 `Console.Prepared`、`TtyLineDisciplineRegistry.Prepared` 和 `ConsoleDriverSet.Prepared`。
正式 console 进入准备边界后，真实 console device probe、boot console unregister 和完整 early/boot console handoff
仍是条件事实或后续设备初始化结果，不作为本阶段固定结束条件。

`panic_later` 检查、`lockdep_init()`、`locking_selftest()`、initrd bounds 检查、`setup_per_cpu_pageset()`、
`numa_policy_init()`、`acpi_early_init()`、`late_time_init` hook 和 `arch_cpu_finalize_init()` 必须通过
`IrqOpenPrepareTrimmedPaths` 这类结构化对象记录调用位置、当前配置裁剪依据和 deferred 边界，不得只依赖 checkpoint 或
markdown 说明。当前 `setup_per_cpu_pageset()` 是 `PageAllocator` per-CPU pageset 快速路径的 deferred 事实；
`lockdep_init()` 的 no-op 依据是 `CONFIG_DEBUG_LOCK_ALLOC=n`，`locking_selftest()` 的 no-op 依据是
`CONFIG_DEBUG_LOCKING_API_SELFTESTS=n`，initrd/NUMA/ACPI 分别依据 `CONFIG_BLK_DEV_INITRD=n`、`CONFIG_NUMA=n`、
`CONFIG_ACPI=n`，`late_time_init` 在 RISC-V 当前路径未设置，`arch_cpu_finalize_init()` 因
`CONFIG_ARCH_HAS_CPU_FINALIZE_INIT` 未启用而为空 inline。

`sched_clock_init()` 对应 `SchedClock.setup()`，发布 generic sched clock core 的启动期读数可用事实，不改变
`RiscvTimerProvider` 或 `Timekeeper` 的生命周期状态。Linux 在 `generic_sched_clock_init()` 周围使用
`local_irq_disable()`/`local_irq_enable()`；当前实现必须通过既有 `BootCpuLocalInterrupt` / `LocalInterruptControl`
记录这个临时本地中断 guard，不能因为外层阶段语义已经是中断开放后单任务上下文而省略该协议事实。`calibrate_delay()` 对应 `DelayLoop.setup()`，消费
`RiscvTimerProvider` 的 timebase/lpj fact，建立 `udelay`/`ndelay`/`mdelay` 等 Ready 后 action 的参数基础。

本阶段仍不得打开普通任务并发或 secondary CPU 并发；周期 tick 服务、完整 softirq 执行、IPI enable、workqueue worker
kthread 和 RCU GP kthread 仍保持 deferred。`setup_per_cpu_pageset()` 当前只作为 `PageAllocator.setup()` 的 per-CPU
pageset 快速路径 deferred 细项记录，不引入新的 lifecycle slot。

smoke 可覆盖两个用户可观察 action：`SchedClock.read()` 至少能返回随 time source 推进的读数，`DelayLoop.udelay(usec)`
能完成一个有界 busy-wait 并保持中断开放状态。smoke 不应为了重复内部状态不变量而暴露更多私有对象字段。

## CacheBlockInfo 编码约束

`CacheBlockInfo.setup()` 对应 Linux 6.12 的 `riscv_init_cbo_blocksizes()`。当前实现只发布平台级 `CBOM` 和 `CBOZ` block size 事实；`CBOP` 虽然存在 DeviceTree binding，但不在该 Linux 初始化点发布，暂不进入当前对象状态。

实现必须从正式 `DeviceTree` 的 `/cpus` CPU nodes 读取 `riscv,cbom-block-size` 和 `riscv,cboz-block-size`，并结合 `CpuGroup` 只收集当前拓扑中的 hart。缺失属性表示 unavailable，不应导致启动失败。多个 hart 值不一致时只记录诊断事实，保持 first value wins 的收敛策略，不 panic，也不阻止 `CacheBlockInfo` 进入 `Ready`。

## RISC-V64 generic 平台任务

`ax-plat-riscv64-generic` 应以 SBI/FDT 为主要事实来源。

第一轮必须解析或建立：

- boot hart id：来自启动 ABI `a0`
- DTB 物理地址：来自启动 ABI `a1`
- CPU 描述：来自 FDT `/cpus`
- 物理内存：来自 FDT `/memory`
- bootargs：来自 FDT `/chosen`
- reserved-memory：必须至少处理 FDT header `/memreserve/` 与 `/reserved-memory` 中当前启动闭环必要的保留范围，
  供 `MemBlock.setup()` 在 allocator 可用前排除；不得把 OpenSBI 或 QEMU virt 固定物理范围作为最终硬编码规则
- SBI 能力视图：至少覆盖 early console、timer、HSM/shutdown 相关能力边界
- 多 hart 平台按 UMA/SMP 处理：`/cpus` 描述 SMP CPU 拓扑，`/memory` 描述共享物理内存地址空间；第一轮不引入 NUMA 语义

第一轮不要求支持：

- initrd
- memory limit
- 多个 memory bank 的完整策略
- NUMA 节点、内存距离和 per-node allocator
- 非 QEMU 的板级差异处理

若 FDT 解析能力不足，应停止并报告缺口，不得静默回退到 QEMU virt 固定内存范围。

## 页表任务

必须严格遵循规格，实现并保持以下边界：

- `TrampolineVm`
- `EarlyVm`
- `SwapperVm`

第一轮不得把现有 boot page table 代码简单改名为多个模型 transition。每次页表切换必须显式处理 RISC-V64 所需的 `sfence.vma` 边界。

## checkpoint

checkpoint 是对象 transition和 Phase 边界的可配置观测/探针分发点，不是普通日志函数，也不是状态机推进的一部分。第一轮预留 checkpoint hook 接口，但不要求实现完整状态差分输出。默认 hook 必须为空实现；trace、test、probe、verify、stop-at-checkpoint 或状态差分采集等具体 handler，都必须通过编译期配置启用，不得依赖运行期动态注册来改变 checkpoint 语义。

checkpoint handler 必须遵守以下约束：

- 不得推进对象或 Phase 状态，不得调用 `Lifecycle::transition`、`Lifecycle::adopt_transition`、`phases::state::mark` 或 `phases::state::adopt`。
- 不得把自身行为作为模型 transition成功的前置条件；对象 transition的 `ensures` 只能来自对象 transition实现本身，不能来自可选 handler 的副作用。
- 默认只能观察只读上下文。handler 输入应优先是 `Checkpoint` 加 `&Context` 或更窄的只读 `CheckpointContext<'_>`；不得默认暴露 `&mut Context`。
- 若某个 checkpoint probe 必须调用会改变对象内部数据的功能 API，例如在 `MemBlock.Online` 后、`MemBlock.Disable` 前测试 `alloc_phys()`，必须通过该 checkpoint 专属的显式 probe capability 授权。该 capability 只能覆盖被测试 API 的最小能力，仍不得推进 lifecycle 或 Phase 状态。
- handler 可以失败并停机，也可以主动停机。建议 outcome 至少区分 `Continue`、`FailAndShutdown` 和 `StopAndShutdown`：前者继续启动，第二类表示 probe/verify 失败，第三类表示达到逐级构建或逐级验证目标后主动结束。
- handler 内部不得再次调用 checkpoint；实现应通过 guard 或模块边界防止 checkpoint 重入。若发生重入，应视为实现错误并停机或直接忽略内层 checkpoint，但不得递归执行 handler。

checkpoint announce 独立于 `EarlyCon` 和正式 `Console`。极早期地址空间阶段可以使用 RISC-V64 SBI legacy putchar 输出单个字符，用于定位 `EarlyVm` 切换生效前的最小事件；该路径不得依赖 allocator、锁、字符串地址、FixMap 或线性映射状态。`EarlyVm` 切换生效、完整 `KernelImage` 映射可访问后，announce 后端应输出稳定 checkpoint 名称字符串，而不是继续消耗单字符 id。

单字符 checkpoint id 只服务 `EarlyVm` 切换前的最低层观测，必须在该极早期后端中保持一一对应，避免运行期 trace 解码歧义。`EarlyVm` 切换后的 checkpoint 新增时只需提供稳定名称，不应再分配单字符 id。

checkpoint 命名应沿用模型对象和状态名称，例如：

- `BootCPU.Prepared`
- `EarlyVm.Ready`
- `Vm.Online`
- `EntrySuccessorPhase.Ready`

## 待确认

- `ax-hal-ex` 与现有 `ax-hal` 的第一轮 public API 对照表。
- `ax-runtime-ex` 与现有 `ax-runtime` 的第一轮 public API 对照表。
- `xtask arceos-ex` 的具体参数和快照格式。
- overlay workspace 生成内容的最小成员集合和依赖替换表。
