# arceos_ex 对象级实现说明

本文记录 `arceos_ex` 第一轮对象级实现的设计背景、命令、工程边界和专题方案。统一任务优先级和状态以
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) 为准；本文不维护独立计划表。

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
- `RestInitPhase.Ready`
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
`BootInitRestInitPhase.Ready`、`BootInitScheduleHandoffPhase.Ready`、`BootIdleEntryPhase.Ready` 和兼容
wrapper `RestInitPhase.Ready`），
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
`spec/model/interrupt/process-prepare/`，目标实现路径为
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

`Completion` 是 `spec/model/common/main.spec` 中定义的可复用 Type process。当前对象级实现必须把它落到
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

## RestInitPhase 编码约束

`rest_init()` 路径在 formal model 中拆成三个 owner-scoped 子阶段：
`BootInitRestInitPhase`、`BootInitScheduleHandoffPhase` 和
`BootIdleEntryPhase`，路径为 `spec/model/up-multitask/rest-init/`，目标实现路径为
`impl/arceos_ex/src/phases/up_multitask/rest_init.rs`。`RestInitPhase` 只保留为
三个子阶段都 Ready 后的兼容 wrapper，不再作为跨 owner 的最小子阶段。

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
`KernelInitTask` release/dispatch facts 和 Scheduler 首次调度 fact，不能硬依赖 `RestInitPhase.Ready`。
`RestInitPhase.Ready` 只表示三个 UP multitask 子阶段已 Ready。

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
`spec/model/smp-runtime/pre-smp-init/`，目标实现路径为
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
`RestInitPhase.Ready` 或 `BootIdleEntryPhase.Ready`。

## SmpBringupPhase 编码约束

`SmpBringupPhase` 是 `SMP Runtime Phase` 的第二个子阶段，formal model 路径为
`spec/model/smp-runtime/smp-bringup/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/smp_bringup.rs`。该阶段从 `smp_init()` 开始，由
`KernelInitTask` 在 boot CPU 上驱动，当前对象级实现只展开 BP 侧主线和 BP/AP 同步边界。

本阶段必须覆盖 `SecondaryIdleTaskSet.preset()`、`CpuHotplugSyncSet.preset()`、
`CpuStartProvider.setup()`、`SecondaryCpuStartupAck.setup()`、`SecondaryCpuOnlineAck.setup()` 和
`SmpBringupBoundary.setup()`。AP 侧 `secondary_start_sbi`、`smp_callin()`、本地中断打开、AP idle 入口和
AP hotplug callbacks 的内部细节当前保持 deferred；但 AP 对 BP 可见的同步量不得省略，至少要发布
`cpu_running` observed、`done_up` observed、`done_down` reserved/deferred、secondary CPU online 和
`smp_concurrency_open` 事实。

`cpuhp_threads_init()` 的 Linux 路径必须保留 `cpus_read_lock()` 与 `smpboot_threads_lock` mutex guard；
`bringup_nonboot_cpus()` / `cpu_up()` 必须保留 `cpu_add_remove_lock` 与 `cpus_write_lock()` 的 writer
guard。RISC-V `__cpu_up()` / AP `smp_callin()` 的 `cpu_running` completion，以及 generic CPUHP
`done_up` completion，必须保留 wait.lock 的 raw spinlock irqsave/irqrestore 观测。`cpu_ops_sbi.cpu_start()`
发布 secondary boot data 前后的 `smp_mb()` 顺序必须作为可观察 fact 或等价内存顺序边界保留；AP
侧 `riscv_ipi_enable()`、`local_flush_icache_all()`、`local_flush_tlb_all()`、`local_irq_enable()` 和
AP hotplug thread `should_run` memory-barrier 配对当前可作为 summary/deferred fact，但不得默认为
不存在。

`SmpRuntimePhase` 当前已经继续串联 `RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase` 和
`FinalizePhase`。各子阶段未展开的完整运行期服务仍保持显式 deferred 边界，不得伪装为已经实现。

测试应覆盖 BP 侧 bringup 主线已经闭合、secondary idle task 已准备、CPU hotplug 同步量已建立并被 AP summary ack
观察、secondary CPU 从 present/not-online 推进到 online、`smp_concurrency_open` 成立，以及 AP 内部路径仍为
deferred summary。smoke 还应覆盖 hotplug read/write guard、`smpboot_threads_lock` / `cpu_add_remove_lock`
mutex guard、`cpu_running` / `done_up` completion wait-lock irqsave guard、SBI boot-data publish ordering
和 AP local sync summary facts。

## RuntimeCorePhase 编码约束

`RuntimeCorePhase` 是 `SMP Runtime Phase` 的第三个子阶段，formal model 路径为
`spec/model/smp-runtime/runtime-core/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/runtime_core.rs`。该阶段必须在 `SmpBringupPhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `sched_init_smp()` 到 `page_alloc_init_late()` 的 BP 主线。

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
`spec/model/smp-runtime/initcall/`，目标实现路径为
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
`init_fileio.c`、`sh_probe.c` 等子用例函数。内核主 Makefile 只负责读取 overlay 映射文件、选择默认工具链/链接方式、
指定输出目录和执行 rootfs 拷贝，不直接承载用户态编译细节。
默认 `ROOTFS_OVERLAY_MAP` 为 `tests/user/rootfs-overlay.map`；`ROOTFS_OVERLAY=none` 时必须跳过映射文件，
否则逐行读取该 map。每个非注释行声明 rootfs 目标路径、用户态测试程序名或特殊动作，以及可选的 toolchain/link mode；未写 toolchain
或 link mode 时分别使用 `ROOTFS_OVERLAY_TOOLCHAIN` 和 `ROOTFS_OVERLAY_LINK`。特殊动作 `__absent__` 表示从 staging rootfs
删除该目标路径，并且不得编译或复制用户态 fixture；它只用于构造明确的 negative/fallback 测试盘，不能用于默认 rootfs overlay。
用户态测试命名必须体现 libc
约定：`.S` 后缀表示纯汇编 fixture；`*_nolibc.c` 表示 freestanding/no-libc C fixture；省略 `nolibc` 的 `.c`
名称表示 libc-linked 测试。默认 map 当前只保留 `tests/user/rootfs-overlay.map`，并把 `user_smoke`
的 musl dynamic 产物放到 `/sbin/init`；`user_smoke` 是 `tests/user/smoke/smoke.c` 主程序和
`init_fileio`、`sh_probe` 子用例的组合。专门验证 `/bin/sh` fallback 时，显式 absent/失败 candidate
map 应由 harness 或手工命令在临时目录生成，不作为长期 checked-in overlay map。配置仍必须允许用户态测试程序以
static 或 dynamic 方式覆盖 `/sbin/init` 或其它 rootfs 内可执行路径。用户态测试
Makefile 必须预留 GNU GCC / musl GCC 和 static / dynamic 四种组合的选择入口；dynamic libc fixture 必须以真实 `PT_INTERP` 形式进入 rootfs，并由用户启动路径读取解释器 ELF，不能把普通 libc 程序伪装成 no-libc `_start` 程序。`make disk` 默认只在磁盘文件不存在时创建；已有磁盘不得因为 overlay 配置变化而被隐式重建，强制重建必须由
`make disk FORCE=1` 或 `make disk-clean` 后再 `make disk` 明确触发。
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
逐级 lookup 直接子节点，遇到 mount point 时 crossing 到 mounted root，再完成 open/read。相对路径、cwd、symlink、
权限、fd table、rootfs 切换、page cache/folio、间接块、xattr、quota、block allocation、写路径、remount 或错误恢复
必须保持 deferred。ext2 smoke 必须通过 VFS path read 读取稳定测试文件，同时仍不得通过 VirtioBlkDevice 私有入口绕过
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
registry 策略。`TtyFlipBuffer` 是 RX interrupt 到 TTY 层之间的 staging buffer，下一轮首个闭环只验证
insert/push，不展开完整 N_TTY read。`TtyXmitFifo` 表示普通 TTY write 的 TX FIFO，必须与当前 printk console
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
`TtyFlipBuffer.Push(record)`；首轮验收终点是 flip-buffer push，不进入完整 N_TTY read。
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
`spec/model/smp-runtime/rootfs/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/rootfs.rs`。该阶段必须在 `InitcallPhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `kernel_init_freeable()` 的 rootfs 准备边界。

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
`spec/model/smp-runtime/finalize/`，目标实现路径为
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

`SystemState.enable()` 是本阶段的主要状态动作：它必须从 `SYSTEM_SCHEDULING` 进入
`SYSTEM_FREEING_INITMEM` 窗口，并最终发布 `SYSTEM_RUNNING`，把 `SystemState.state` 推进到 `Online`。
`RcuCore.end_inkernel_boot()` 是本阶段的另一个主线 action，必须记录 `rcu_boot_ended == true`，但不得把完整
RCU GP 服务或 worker 运行伪装成已实现。实现还必须暴露 Linux `rcu_end_inkernel_boot()` 中
`rcu_unexpedite_gp()` 的 atomic decrement、`CONFIG_RCU_LAZY` 未启用导致 `rcu_async_relax()` 不改变 lazy
nesting 的裁剪事实、`CONFIG_PREEMPT_RT=n` 下 `rcu_normal_after_boot` 默认不触发 `WRITE_ONCE(rcu_normal, 1)`
的事实，以及 `rcu_boot_ended` publish fact。`numa_default_policy()` 的裁剪 checkpoint 必须位于
`SYSTEM_RUNNING` 之后、`rcu_end_inkernel_boot()` 之前，不能放在 `RestInitPhase`。

测试应覆盖 `FinalizePhase.Ready`、async full sync deferred、init memory cleanup deferred/trimmed 事实、
mapping protection deferred、PTI trimmed、`SystemState.Online`/`SYSTEM_RUNNING`、NUMA default policy
trimmed、RCU in-kernel boot ended 与 atomic/WRITE_ONCE/lazy-trim facts、sysctl args deferred，以及下一入口仍是
`PayloadPhase`。

## PayloadPhase 编码约束

`PayloadPhase` 是 `StartupTimeline` 的末尾阶段，formal model 路径为 `spec/model/payload/`。它必须作为
`SmpRuntimePhase.Ready` 之后的后续阶段实现，而不是嵌套为 `SmpRuntimePhase` 的子阶段。入口必须要求
`FinalizePhase.Ready` 和 `FinalizeBoundary.Ready`，并消费 `payload_phase_next_boundary()` 事实。

KernelInitTask 的执行线从 `SmpRuntimePhase` 入口开始：`rest_init()` 通过 `TaskCreationCore` 把
`TaskEntry::KernelInit` 绑定到 `KernelInitTask`，并提交 release/dispatch facts；`SmpRuntimePhase` 的首个子阶段 `PreSmpInitPhase` 消费这些 entry/release/dispatch facts 后进入。后续阶段按 phase 顺序衔接到 `FinalizePhase`，再自然进入 `PayloadPhase`。因此 selected payload 的执行归属应从这条连续执行线推出，而不是由 `PayloadPhase` 单独声明一个调用者事实。`BootIdleTask` 只负责 idle loop、need_resched observation 和 schedule boundary；`KthreaddTask` 当前提供内核线程管理者 ready/provider 事实和最小 schedule-loop 入口边界。

`UserBootPayload` 是 Linux-like 用户态首进程路径的 selected payload 变种。代码生成或手写实现必须保持以下边界：

- syscall 入口沿用 `ExceptionStream -> SyscallException`，不得为了用户态 hello 新增独立根 `Syscall` 对象或绕过异常分发。
- Linux `kernel_execve()` 完整路径中的同步协议必须保留为显式 `PayloadExecSyncBoundaries` deferred contract，而不能被 `KernelInitTask`、boot-only 或 no-return handoff 事实吞掉。该 contract 至少记录 `binfmt_lock`、`cred_guard_mutex`、`exec_update_lock`、`exec_mmap()` 中的 local IRQ disable/enable、`mmap_lock`、`exec_mmap()` task lock/mm handoff、`sighand->siglock`、`tasklist_lock`、`fs->lock` + RCU read side、membarrier/switch-mm ordering、`sched_mm_cid_*()` 的 rq_lock_irqsave+smp_mb、`bprm_mm_init()` / `finalize_exec()` 的 task_lock rlimit 边界、files unshare/CLOEXEC file_lock、io_uring cancel、POSIX timer siglock、namespace switch、exec 成功后的 rseq/perf/audit/accounting hooks、完整 binfmt/script retry 和 panic terminal 边界。当前 `../linux-6.12/.config` 中 `CONFIG_MODULES=n`，因此 `request_module("binfmt-...")` retry 必须记录为 trimmed/no-op 而不是 deferred。当前 `APP=user-boot` 只实现最小 VFS/ELF/UserAddressSpace/trap-return handoff；上述 Linux exec guard 不得标为已实现，也不得只留在 roadmap 表格。
- `UserBootPayload.Setup` 必须由已经到达 `PayloadPhase` 的 `KernelInitTask` 执行线驱动；实现应记录 payload 归属事实，而不是把用户态入口建模为独立启动根。
- `UserBootPayload` 的默认 init 选择必须对齐 Linux 6.12 `init/main.c::kernel_init()` 中 `try_to_run_init_process()` 的 fallback 顺序：在 `ramdisk_execute_command`、`execute_command` 和空 `CONFIG_DEFAULT_INIT` 的边界按既有 trimmed/deferred 事实处理之后，依次尝试 `/sbin/init`、`/etc/init`、`/bin/init`、`/bin/sh`，直到首个成功 candidate 成为 selected path；所有 candidate 都失败时进入 Linux-like “No working init found” panic terminal boundary。当前首片实现的 candidate 成功口径是：从当前 `FsStruct.root` 经 `VfsCore.ReadPath` 读取到文件，并通过当前 `ElfObject.Preset/Setup` 支持的 ELF 检查；这不等价于完整 `kernel_execve()` 成功。Linux `kernel_execve()` 成功时返回整数 0，但当前 task 已经完成 exec 身份转换，`kernel_init()` 不再继续尝试后续 fallback；实现必须把成功 candidate 记录为停止 fallback chain 和不返回启动编排链的边界。candidate 失败必须分类记录并允许继续尝试后续 candidate，不得在 `/sbin/init` 失败时直接 panic。后续若补 `init=`/`rdinit=`、脚本 binfmt、symlink、权限或完整 errno 语义，必须继续保持 Linux 该段顺序和终端 panic 语义。
- `make test` 中的 `APP=user-boot` 验收不得只依赖 QEMU/SBI shutdown 退出码；host harness 必须解析普通输出中的 `user exit status=N`，且 native 与每个 configured provider 下均只有 `N == 0` 计为通过。非零状态、缺失该输出或 QEMU 命令自身失败都必须判失败。guest 仍由用户态 `exit/exit_group` syscall 路径打印状态并执行 shutdown，harness 只解释输出，不迁移 shutdown 责任。
- 发行版 `/bin/sh` 分阶段验证不得依赖“当前尚未支持 symlink”这类临时缺口来绕过 `/sbin/init`。默认 staged user smoke 应覆盖 `/sbin/init`，保持首个 fallback candidate 成功；默认 checked-in overlay map 只保留 `tests/user/rootfs-overlay.map`，并把综合 `user_smoke` 覆盖到 `/sbin/init`。`user_smoke` 的主文件是 `tests/user/smoke/smoke.c`，它调用由 `init_fileio.c`、`sh_probe.c` 等文件提供的子用例函数；框架层输出必须使用 `user-smoke:` 前缀，打印总入口 `user-smoke: begin`、每个 case 的 `user-smoke: case <name>: begin` 和 `user-smoke: case <name>: end status=N`、以及最终 `user-smoke: end status=N`，并在总入口和每个 case 边界之间用空行分隔。子用例内部只输出具体能力点，例如每个 syscall 路径成功后的 `syscall ... ok` 标记，避免把“用户 ELF 已加载”和“具体 syscall 已支持”混在同一条 `user hello` 观测里；host harness 仍只以 `user exit status=N` 判断通过，不依赖这些人读 marker。专门验证 `/bin/sh` fallback 时，测试 rootfs overlay 必须显式把 `/sbin/init`、`/etc/init` 和 `/bin/init` 构造成 absent/失败 candidate，再把 `/bin/sh` 覆盖为受控 probe；这类 fallback overlay map 应由 harness 或手工命令在临时目录生成，不作为长期 checked-in map。`sh_probe` 或后续 BusyBox 探针需要的 syscall 集合必须先由本地构建产物静态/半静态分析得出，例如源码调用、ELF interpreter/dynamic section、符号和反汇编；guest 运行只用于验证内核实现是否满足这些已知需求，或在静态分析无法收敛时定位条件路径缺口。目录枚举阶段的 `sh_probe` 先输出既有 regular file syscall 成功标记，再用直接 Linux/RISC-V syscall 形态验证 `openat(AT_FDCWD, "/", O_RDONLY|O_DIRECTORY)`、`getdents64(61)`、`linux_dirent64` 记录解析和目录 fd `close`。host harness 必须为 user-boot smoke 使用独立临时 rootfs image，避免已有 `build/virtio-blk.raw` 或其它 overlay case 污染结论。面向发行版 `/bin/ls` 的下一步采用流程化的本地静态/半静态分析，而不是引入自建长期工具：用 `file`、`readelf`、`objdump`、本地 RISC-V Linux syscall headers，以及必要时只读查看 sysroot path/symlink，记录目标 ELF、可选 guest path 解析、`PT_INTERP`、program headers、dynamic section、动态符号、反汇编中可见的 `ecall`/`a7` 证据和 syscall 名称映射；分析结果写入临时记录即可，不作为长期 baseline。该流程不得新增 Makefile target、重建 rootfs staging，或改变默认 `make disk/run` 的 rootfs 构造路径；`make test` 为隔离 overlay 验收而传入 case-local disk image 是允许的 harness 行为。当前 Alpine `/bin/ls` 是 BusyBox applet，BusyBox 全局动态符号只能作为保守候选集，临时记录不得把它们声称为 `ls` applet 的完整运行时 syscall trace；路径敏感缩小或 guest 验证必须作为后续步骤单独记录。每个用户态 syscall/VFS 子项进入 model/coding 规格前，必须以本地 `../linux-6.12` 为参考源码，而不是按泛化 Linux 行为推断；当前 `/bin/ls` 第一批目录/fd 工作的基准入口包括 `arch/riscv/kernel/syscall_table.c`、`include/uapi/asm-generic/unistd.h`、`fs/open.c::do_sys_openat2()/sys_openat()`、`fs/readdir.c::sys_getdents64()/iterate_dir()`、`fs/stat.c::vfs_getattr()/sys_newfstatat()/sys_newfstat()/sys_readlinkat()`、`fs/file.c::fdget()/fdget_pos()/alloc_fd()`，以及后续命名解析需要的 `fs/namei.c`；若首片实现刻意裁剪其中的锁、RCU、权限、mount namespace、LSM 或完整 errno 行为，必须在 model/coding 中记录为 trimmed/deferred，不能只在实现里静默省略。
- 用户可执行文件对象命名为 `ElfObject`，不得生成单独的 `ElfLoader` 资源对象。`ElfObject.Preset` 只检查 ELF 类型支持；`ElfObject.Setup` 解析 ELF header / program headers，并形成 `PT_LOAD` 映射计划、段权限、entry 和 `.bss` 清零计划；真实映射到 `UserAddressSpace` 与 `.bss` 清零事实归 `UserAddressSpace.Setup`。dynamic libc 支持仍然使用 `ElfObject`：主程序的 `PT_INTERP` 只绑定 interpreter 路径事实，并驱动 `UserBootPayload` 把解释器作为 role 为 interpreter 的第二个 `ElfObject` 读取和解析；不得因此引入 `ElfLoader` 资源对象或独立 `Load` 生命周期阶段。`ElfObject.Enable` 只确认 entry、用户栈和 trap frame 已可用于进入用户态。
- `UserAddressSpace` 是多实例用户地址空间对象，低地址用户区独立；高地址内核映射共享或引用 `SwapperVm`。`SwapperVm` 继续是内核共享地址空间实例，不应被改造成普通多实例用户地址空间类型。首个 `UserAddressSpace` 实例必须在 `Preset` 时依赖 `KernelInitTask.Online`，并记录 `KernelInitTask` 绑定该首个用户地址空间的事实，供后续 stack、ELF mapping 和 trap frame setup 消费；该绑定只表示 kernel_init 用户态启动路径已拥有待启用地址空间，不表示已经写入 `satp` 或完成硬件地址空间切换。
- `UserAddressSpace.Setup` 消费 `ElfObject` 的 `PT_LOAD` 映射计划，记录 segment/stack/heap mapping facts、用户页 `U` 权限事实和内核映射 `U=0` 事实，并在切换前分配 backing pages、复制 ELF 文件内容、清零 `.bss`/stack/heap，建立 page-table-shaped view。对存在 `PT_INTERP` 的 dynamic executable，它必须把主 ELF 和 interpreter ELF 的 `PT_LOAD` 段映射到同一个 `UserAddressSpace`，并记录 interpreter mapping facts；interpreter 可为 `ET_DYN`，但首轮使用固定 non-overlap load bias，不引入完整 ASLR/VMA tree。ELF segment backing 和 low-half leaf PTE 按页粒度覆盖 `align_down(p_vaddr)..align_up(p_vaddr + p_memsz)`；因此 `mprotect`/`munmap` 首片对 mapped user range 的判断也必须使用同一页范围，允许动态链接器对 GNU_RELRO 所在整页执行保护变更。dynamic linker 首片还必须提供阶段性的 user heap / anonymous mmap arena，用于承接早期 `brk`/`mmap` 需求；这属于正式运行时语义，不是 smoke/KUnit 专用 API。`UserAddressSpace.Enable` 是真实切换前的 satp-ready 边界：它可以分配真实 Sv39 用户页表页、安装低端用户 leaf PTE、复制或共享 `SwapperVm` 高端 root entries、生成待使用的 satp token，并标记 `runtime_ready` 表示该地址空间可被下一轮 trap return 消费；但它必须同时记录 prepared-but-not-current 事实，不得写入 `satp`、不得执行 `sfence.vma` 作为地址空间切换、不得执行 `sret`、不得设置 `SyscallTable` 或 `UserInitProcess` 状态。
- `UserStack.Setup` 必须为 static libc 用户 init 建立最小 Linux initial stack，而不是只把 `sp` 置为栈顶。当前最小布局至少包含 `argc=1`、`argv[0]=selected_path`、`argv` 终止空指针、`envp` 终止空指针，以及 `auxv` 中的 `AT_PAGESZ` 和 `AT_NULL`，并让 `UserTrapFrame.Setup` 消费该布局起点作为用户 `sp`。默认 overlay fixture 下 selected path 仍通常是 `/sbin/init`；发行版路径中必须由实际成功的 fallback candidate 决定。dynamic libc 首片必须进一步提供动态链接器关键 auxv 字段，至少包括主程序 `AT_PHDR`、`AT_PHENT`、`AT_PHNUM`、`AT_ENTRY`、interpreter base 对应的 `AT_BASE` 和 `AT_PAGESZ`/`AT_NULL`；随机化、guard page、栈扩展、`AT_RANDOM`、`AT_EXECFN`、真实 argv/envp 派生和完整 auxv 后续再展开，但不得把普通 libc fixture 伪装成 no-libc `_start` 程序。
- `UserTrapFrame.Setup` 只准备 `sepc=elf.runtime_entry`、`sp=user_stack.initial_sp`、用户态 `sstatus` 事实和关联的 `UserAddressSpace`，表示下一轮可以由统一 trap return 消费；静态程序的 runtime entry 是主 ELF entry，动态程序的 runtime entry 是 interpreter entry。它本身不得执行 `sret` 或观察用户态已经进入。
- B 阶段的 U-mode entry 必须复用现有 `EventStream` trap return 形状，并在模型中用 `within UserModeTrapReturnContext { ... }` 包住 `UserInitProcess.Action::EnterUserMode` 的最终硬件交接。实现必须写入 `sscratch/sepc/sstatus/satp`、在 `satp` 写入后执行 `sfence.vma`，再通过 `sret` 进入用户态；该 context 的退出是 `Never`，因为正常路径不返回启动编排链。用户态 trap 入口不能把用户 `sp` 当成内核 trap frame 栈使用，必须先切到内核拥有的 trap 栈，例如通过 `sscratch` 暴露的 trap stack top，再保存完整 trap frame。
- 用户态 `ecall` 必须进入既有 `ExceptionStream -> SyscallException` 分支，不得新增独立根 `Syscall` 对象或绕过异常分发。`SyscallException` 承担用户态 syscall 入口、来源检查、参数提取和分发选择；不得再生成独立 `SyscallDispatcher` 对象。`SyscallTable` 是独立对象，承载具体 syscall action 集合；具体 syscall 是 `SyscallTable` 的 actions，不是单独资源对象。
- `FilesStruct` 表示任务拥有的打开文件上下文，和表示 root/pwd 的 `FsStruct` 并列；不得把 fd table 塞进 `FsStruct`。首轮 `FilesStruct.Setup` 必须在同一 `KernelInitTask` 执行线上建立 `FileDescriptorTable`、stdio `OpenFileDescription` 和 console-like `FileBackend::CharDevice`，预安装 fd 0/1/2，并记录 fd table、flags、offset、close-on-exec 和 next-fd 基础事实。当前 read-only 扩展只允许再建立一个 `RegularFile` opened instance：`openat` 从当前 `FsStruct.root` 经 `VfsCore.ReadPath` 读取已存在 regular file，`FileDescriptorTable.Install` 分配固定首个普通 fd，`read` 从该 opened instance 的 offset 读取到用户缓冲区，`close` 清除该 fd，`newfstatat` 返回最小 regular-file metadata。块设备后端、完整 `/dev/console`、TTY、目录 fd、权限、symlink、poll、共享 fd table、普通文件写路径、page cache 和跨任务共享语义可以 deferred，但必须在模型和代码事实中明确标记，不得伪装为已实现。
- `UserInitProcess` 表示 PID 1 的 `KernelInitTask` 经 `kernel_init -> run_init_process()/execve` 后获得的用户态进程身份视图，不表示新创建了第二个 task。`UserInitProcess.Setup` 必须依赖 `KernelInitTask.Online`、`UserAddressSpace.Online`、`ElfObject.Online`、`UserTrapFrame.Ready`、`FsStruct.Ready` 和 `FilesStruct.Ready`，并记录 PID 1 身份延续、同一 task_struct 复用、`KernelInitTask` 未被销毁、`FsStruct`/`FilesStruct` 继承、用户地址空间和 trap frame 绑定等事实。`UserInitProcess.Enable` 是进入 U-mode 前的最后对象边界，必须在 `SyscallException.Online` 和 `SyscallTable.Ready` 后记录 syscall 上下文绑定和 trap return/satp handoff 事实。`UserInitProcess.Action::EnterUserMode` 是消费已就绪 `UserTrapFrame` / `UserAddressSpace` 并执行最终 trap-return handoff 的运行时 action，不负责制造 trap frame；真实 `write`/`exit` 观测只能由进入 U-mode 后的 `SyscallTable.Action::Write` / `Exit` / `ExitGroup` 生产路径 checkpoint 提交，不得在 Enable 中预写。不得在这里引入 fork/wait/signal、完整 scheduler 运行期或新的 task 生命周期对象。
- 当前 rootfs 输入是 whole-disk ext2；`UserBootPayload` 不应要求 `PartitionTable` / `BlockPartition`。若未来磁盘镜像切换为带分区表，再补分区对象建模。
- 当前 `SyscallTable` 覆盖 `write(1/2, user_buf, len)`、`writev(1/2, iov, iovcnt)`、read-only `openat/read/close/newfstatat` 首片、directory-capable `openat(AT_FDCWD, path, O_RDONLY|O_DIRECTORY)` 和 `getdents64(61)` 首片、dynamic loader 所需的 `brk/mmap/mprotect/munmap`、`set_tid_address(clear_child_tid)`，以及 `exit/exit_group(status)`。`write/writev` 通过受限 `UserCopy` 从当前 `UserAddressSpace` 复制用户字节，然后必须经 `FilesStruct.Action::LookupFd -> FileDescriptorTable.Action::Lookup -> OpenFileDescription.Action::Write -> FileBackend.Action::WriteCharDevice` 路径写入现有 printk/serial console；不得继续在 `SyscallTable.Action::Write` 或 `Writev` 内直接用 `fd == 1 || fd == 2` 绕过 fd table。`writev` 首片只支持有限数量的小 iovec，并按 iovec 顺序复用既有 fd 写路径。`openat/read/close/newfstatat/getdents64` 必须经 `FilesStruct`、`FileDescriptorTable`、`OpenFileDescription`、`FileBackend` 和当前 VFS path API，不得在 syscall table 中直接裸读 ext2 或为测试专门构造文件/目录内容。`openat` 当前只接受 `AT_FDCWD` 下的 read-only 普通文件打开，或带 `O_DIRECTORY` 的 read-only 目录打开；可以接受 libc 在 64-bit 路径中自动附带的 `O_LARGEFILE`，但仍必须拒绝写、创建、truncate、append、`O_TMPFILE`、相对 dirfd 等超出首片的 flags。`getdents64` 首片对齐 Linux 6.12 `fs/readdir.c::sys_getdents64()/iterate_dir()` 和 `fs/file.c::fdget_pos()` 的核心语义：从 fd table 取得 directory open file description，使用 fd position 作为目录 byte offset，序列化 `linux_dirent64` 记录，成功返回复制字节数，并把 offset 推进到最后返回的 ext2 dirent 后；用户缓冲不足以放下下一条完整记录时停止在已完成记录边界。首片只覆盖当前 ext2 read-only direct-block 目录、`DT_REG/DT_DIR/DT_UNKNOWN`、单个非 stdio fd slot 和 `USER_COPY_MAX` 内的部分填充；Linux 的 `i_rwsem`、`security_file_permission()`、fsnotify/file_accessed、dead directory 检查、mount namespace、RCU/`f_pos_lock` 竞争和完整 errno 细节必须记录为 deferred/trimmed。`brk/mmap/mprotect/munmap` 必须路由到 `UserAddressSpace` 的阶段性 heap/mmap arena；`mmap` 首轮只要求 anonymous private 映射，`mprotect`/`munmap` 可先作为已映射用户区间的成功边界。`set_tid_address` 必须落到当前 `UserInitProcess` 的 `clear_child_tid` 属性，返回 PID1，不得伪造第二个 task 或引入线程组/futex 完整语义。该路径仍不等价于完整 `/dev/console` 文件、TTY/N_TTY、fork/wait、signal、普通文件 `write`、权限检查、symlink、poll、完整 VMA tree、文件映射或完整 fd 生命周期管理。
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

`80966ec` 曾暴露 `13 obligation / 2 deferred`。这些条目不能作为实现可忽略的提示；处理原则是：实现某个对象 transition前，必须先通过“推导义务门禁”。能由模型、推导工具、链接脚本、ISA、固件规范或已证明前序事实推出的，应优先补齐推导证明；只能由外部交付保证支撑的，应明确作为 source assumption，并在后续对象 transition中尽快转化为运行期检查；无法归类的应作为规格缺口或显式 deferred。

当前已补齐 Lds、OpenSBI DTB handoff 和 BootCPU 前序事实的推导规则，`make verify` 报告为 `0 obligation / 3 deferred`。后续若再次出现 obligation，应先回到本节分类处理，不得直接继续实现。

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
    objects/
    phases/
    trace/
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

`MmCoreInitPhase` 已正式落到 `spec/model/boot/mm-core-init/`。实现侧必须保持与模型一致的阶段边界：入口是 `CorePreparePhase.Ready`、`ExceptionStream.Ready`、`MemBlock.Online`、`PageMetadataMap.Ready`、`DmaCachePolicy.Ready`、`StaticBranch.Ready` 和 `SystemExclusive`；出口是 `PageAllocator.Ready`、`MemBlock.Offline`、`SlubSubsystem.Ready`、`PageTableCaches.Ready`、`VmallocAllocator.Ready`、`MmStructCache.Ready`。本阶段只能消费已经建立的 `PageMetadataMap`；`mem_init()` 开头的 `BUG_ON(!mem_map)` 对应 checkpoint，不在 `mm_core_init()` 内推进 `PageMetadataMap.setup()`。

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

`SchedInitPhase` 已正式落到 `spec/model/boot/sched-init/`。实现侧必须保持与模型一致的阶段边界：入口是
`MmCoreInitPhase.Ready`、`PageAllocator.Ready`、`SlubSubsystem.Ready`、`KmallocCaches.Ready`、`CpuGroup.Ready`、
`PerCpuStorage.Ready`、`CpuHotplugState.Ready`、`StaticBranch.Ready`、`PrintkBuffer.Ready` 和
`SystemExclusive`；出口是 `Scheduler.Online`、`RadixTree.Ready`、`MapleTree.Ready`、`Workqueue.Prepared`、
`Softirq.Prepared`、`RcuCore.Ready` 和 `TasksRcu.Prepared`。

目录、文件和对象命名必须跟阶段名一致：模型目录为 `spec/model/boot/sched-init/`，实现文件为
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
`switch_to` 仍只实现 RISC-V 核心保存/恢复边界和 `CurrentTaskRef` commit；真实 task stack switch、next task
上下文恢复、`finish_task_switch()` 等细节后续展开。若调用路径来自 `schedule_preempt_disabled()`，
调用方继承的 preemption guard 退出和 post-schedule guard 重新进入必须在调用方上下文中显式建模。

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

`IrqTimeInitPhase` 已正式落到 `spec/model/interrupt/irq-time-init/`，属于 `InterruptPhase` 的第一个子阶段。实现侧边界和早期设计草案不同：`local_irq_enable()`
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

目录、文件和对象命名必须跟阶段树一致：模型目录为 `spec/model/interrupt/irq-time-init/`，实现文件位于
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

`LocalIrqEnablePhase` 已正式落到 `spec/model/interrupt/local-irq-enable/`，属于 `InterruptPhase` 的第二个子阶段。它必须接在
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

`IrqOpenPreparePhase` 已正式落到 `spec/model/interrupt/irq-open-prepare/`，属于 `InterruptPhase` 的第三个子阶段。它必须接在
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
