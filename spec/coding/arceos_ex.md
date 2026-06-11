# arceos_ex 对象级实现说明

本文记录 `arceos_ex` 第一轮对象级实现的设计背景、命令、工程边界和专题方案。统一任务优先级和状态以
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) 为准；本文不维护独立计划表。

当前执行路线已经调整为：先在本仓库内直接完成对象级实现实验，源码放在
`impl/arceos_ex/`，使用 `Makefile` 编译和运行；暂时不进入 `tgoskits`、`xtask`、ArceOS crate 兼容和 feature 传递问题。

`tgoskits`/ArceOS 组件兼容属于后续 `Composition Phase`，只有对象级实现闭环后再恢复讨论。此前在
`tgoskits` 中实现过的 `arceos_ex` 仍具有参考价值，尤其是 RISC-V64 入口、链接脚本、checkpoint 字符输出、SBI/FDT
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
- `IrqOpenPreparePhase.Ready`
- `ProcessPreparePhase.Ready`
- `UpMultitaskPhase.Ready`
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
make run LOG=trace
make verify
make verify REPORT=graph
make test
make test-verify
make test-kunit
make test-smoke
make clean
```

`KERNEL ?= arceos_ex` 选择默认内核，`APP ?= smoke` 选择默认 selected payload。`build` 负责编译内核镜像；`run` 使用 QEMU/OpenSBI 运行；`run LOG=trace`
启用 checkpoint 字符输出。`verify` 调用 `pyveri` 对当前启动时间轴规格做推导验证；`verify REPORT=graph`
生成带注释的 trace SVG 报告。`test` 是默认验证闭环，按顺序执行 `test-verify`、`test-kunit`
和 `test-smoke`：第一步运行正式规格 strict derive，第二步用 `impl/arceos_ex/tests/kunit.handlers`
聚合 checkpoint/KUnit handler 在 `APP=hello` 路径下验证局部对象和 action 边界，第三步运行 `APP=smoke`
验证最终 payload 可观测行为。新增 checkpoint KUnit handler 时必须加入该 handler 文件，除非它需要单独的
测试入口并在 coding 规格中记录原因。

当前对象级实现已经能通过 `make run` 和 `make run LOG=trace` 完成 `EntryPreludePhase.Ready`、
`EntrySuccessorPhase.Ready`、`CorePreparePhase.Ready`、`MmCoreInitPhase.Ready`、`SchedInitPhase.Ready` 和
`InterruptPhase.Ready`（其当前展开子阶段包括 `IrqTimeInitPhase.Ready`、`IrqOpenPreparePhase.Ready` 和
`ProcessPreparePhase.Ready`），再完成 `UpMultitaskPhase.Ready`（当前展开 `RestInitPhase.Ready`），
随后完成 `SmpRuntimePhase.Ready`（当前展开 `PreSmpInitPhase.Ready`、`SmpBringupPhase.Ready`、
`RuntimeCorePhase.Ready`、`InitcallPhase.Ready`、`RootfsPhase.Ready` 与 `FinalizePhase.Ready`），再通过
后续的 `PayloadPhase` 进入默认 `smoke` payload，执行 smoke 用例后通过 SBI 关机。`PayloadPhase` 是
`SmpRuntimePhase` 的后续阶段，不是其最后一个子阶段；它直接衔接 `FinalizePhase.Ready` /
`FinalizeBoundary.Ready`。
当前 `MmCoreInitPhase`、`SchedInitPhase` 和 `IrqTimeInitPhase` 都保持最小对象级语义：`PageAllocator`、
`SlubAllocator`、`VmallocAllocator`、`Scheduler`、`Workqueue`、`Softirq`、`RcuCore`、`RiscvTimerProvider` 和
`SmpCallFunction` 只发布状态与必要事实，不提供完整运行期服务。

## ProcessPreparePhase 编码约束

`ProcessPreparePhase` 是 `InterruptPhase` 的第三个子阶段，formal model 路径为
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
keyring 和 security 对象按 formal trace 后续推进。VFS/proc/page-cache/net namespace、`signals_init()` 以及
实际任务创建仍保持 deferred 或 trimmed checkpoint，不应伪装成完整运行期服务。

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
`KthreaddReadyGate.complete()` 必须驱动通用 `Completion.complete()`；释放 PID 1 的场景结果由
`RestInitPhase` 事实承载，不引入 Linux 中不存在的额外 action。

smoke 测试必须覆盖两类路径：一是独立 `Completion` 实例的 setup/enable/complete/token consume/reinit 流程，二是
`rest_init()` 中的 live `kthreadd_done` 实例，验证 `KthreaddReadyGate` 已驱动 `Completion.complete()` 并唤醒 PID 1。

## RestInitPhase 编码约束

`RestInitPhase` 是 `UpMultitaskPhase` 的第一个子阶段，formal model 路径为
`spec/model/up-multitask/rest-init/`，目标实现路径为
`impl/arceos_ex/src/phases/up_multitask/rest_init.rs`。该阶段必须在 `ProcessPreparePhase.Ready` 之后运行，并在
`PayloadPhase` 之前完成；它覆盖 Linux `rest_init()` 的最小对象级边界。

本阶段的主线对象是 `KernelInitTask`、`KthreaddTask`、`SystemState`、`KthreaddReadyGate`
和 `BootIdleRuntime`。实现必须发布 PID 1 已创建并入队、`kthreadd`
provider 已创建并绑定全局引用、`system_state == SYSTEM_SCHEDULING`、`kthreadd_done` 已 complete、
`schedule_preempt_disabled()` 已按 preemption guard 退出、`Scheduler.schedule()` 调度分界、
post-schedule boot idle context 进入三段体提交首次调度交接，boot idle runtime 入口已确认等事实。
这些事实当前仍是对象级模拟边界，不得实现真实任务栈切换、真实调度上下文切换或 idle loop。

PID 1 和 kthreadd 的创建必须通过 `TaskCreationCore` 的 entry contract 表达：`KernelInitTask` 使用
`TaskEntry::KernelInit`，其第一执行线指向 `SmpRuntimePhase`；`KthreaddTask` 使用
`TaskEntry::Kthreadd`，其第一执行线指向 kthreadd 服务循环边界。当前 BP 最小实现只需要把 kthreadd 入口循环建模为“等待工作、无工作时请求 `schedule()` 切出”的 named boundary；真实 kthread 请求消费、park/stop/wait 细节，以及非 idle current 下的完整 scheduler 切换留给后续模型。

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
`Scheduler` action，而应展开为 `BootIdlePreemption.enable_no_resched()`、
`Scheduler.schedule()` 和 post-schedule boot idle context。不得引入
`KernelInitDispatchGate` 生命周期对象。`SmpRuntimePhase` 首个子阶段 `PreSmpInitPhase` 依赖
`KernelInitTask` release/dispatch facts 和 Scheduler 首次调度 fact，不能硬依赖 `RestInitPhase.Ready`。
`RestInitPhase.Ready` 仍必须覆盖
`cpu_startup_entry(CPUHP_ONLINE)` 对应的 boot idle 尾部完成事实。

`BootIdleRuntime` 的代码结构必须和 model action 边界对齐：`setup()` 只建立
`BootIdleRuntime.Ready` 壳并确认首次调度交接已存在，不得一次性写入全部 idle 尾部事实；
phase 代码必须随后显式调用 `prepare_idle_entry()` 和 `run_idle_loop()`。`prepare_idle_entry()` 承载
`current->flags |= PF_IDLE`、`arch_cpu_idle_prepare()` 和 `cpuhp_online_idle(CPUHP_ONLINE)` 的当前抽象事实；
`run_idle_loop()` 只提交进入 idle loop，并驱动一轮代表性的 `do_idle_cycle()`。
`RestInitPhase.setup()` 主线必须直接呈现
`BootIdleRuntime.setup()` -> `BootIdleRuntime.prepare_idle_entry()` -> `BootIdleRuntime.run_idle_loop()` ->
`RestInitPhase.Ready` checkpoint 的顺序。代码可以为每个 named action 保留小 helper，但不得再用单个
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

本阶段可以打开“单核多任务”语义，但仍不得启动 secondary CPU；也不得把完整 workqueue/SMP 拓扑、
真实 Tasks RCU GP kthread 运行、后续 kthread request 消费提前实现。`KernelInitTask` 的下一执行线是
`SmpRuntimePhase`，其首个子阶段是 `PreSmpInitPhase`；该事实来自 `TaskEntry::KernelInit` 的创建入口绑定。`KthreaddTask` 当前只实现入口循环和 schedule 请求边界，完整运行期服务能力留给后续模型。

## PreSmpInitPhase 编码约束

`PreSmpInitPhase` 是 `SmpRuntimePhase` 的第一个子阶段，formal model 路径为
`spec/model/smp-runtime/pre-smp-init/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/pre_smp_init.rs`。该阶段由 `KernelInitTask` 在
`kernel_init_freeable()` 中推进，入口是 `KernelInitTask` 已被 `kthreadd_done` 释放、
`KernelInitTask` dispatch facts 和 Scheduler 首次调度 fact，出口停在 `smp_init()` 调用前。

本阶段必须覆盖 `PageAllocator.open_full_gfp_mask()`、`CpuGroup`/CPU topology 的 pre-SMP present 边界、
`Workqueue.setup()`、`VmstatCore.preset()`、`TasksRcu.setup()`、`PreSmpInitcallTable.run_early()` 和
`PreSmpInitBoundary`。它可以发布阻塞 GFP 分配可用、workqueue worker 创建边界、Tasks RCU GP thread
创建边界和 early initcall 已运行事实，但不得把 secondary CPU 标记为 online，也不得执行 `smp_init()`。
本阶段入口除依赖 `KernelInitTask` release/dispatch facts 和 Scheduler 首次调度 fact 外，还必须消费
`TaskCreationCore` 建立的 entry contract：`KernelInitTask` 的 `TaskEntry::KernelInit` 指向 `SmpRuntimePhase`。

测试应覆盖 full GFP mask 已打开、secondary CPU 只处于 present/not-online、Workqueue Ready 但 SMP topology
仍 deferred、VmstatCore Prepared、TasksRcu Ready、pre-SMP initcall 已运行、`smp_init()` 未执行，以及
`PreSmpInitPhase` 的入口来自 `KernelInitTask` entry/release/dispatch 和 Scheduler 首次调度 facts，而非
`RestInitPhase.Ready`。

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

`SmpRuntimePhase` 当前已经继续串联 `RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase` 和
`FinalizePhase`。各子阶段未展开的完整运行期服务仍保持显式 deferred 边界，不得伪装为已经实现。

测试应覆盖 BP 侧 bringup 主线已经闭合、secondary idle task 已准备、CPU hotplug 同步量已建立并被 AP summary ack
观察、secondary CPU 从 present/not-online 推进到 online、`smp_concurrency_open` 成立，以及 AP 内部路径仍为
deferred summary。

## RuntimeCorePhase 编码约束

`RuntimeCorePhase` 是 `SMP Runtime Phase` 的第二个子阶段，formal model 路径为
`spec/model/smp-runtime/runtime-core/`，目标实现路径为
`impl/arceos_ex/src/phases/smp_runtime/runtime_core.rs`。该阶段必须在 `SmpBringupPhase.Ready` 之后运行，由
`KernelInitTask` 在 boot CPU 上继续推进 `sched_init_smp()` 到 `page_alloc_init_late()` 的 BP 主线。

本阶段必须覆盖 `Scheduler.enable_smp()`、`Workqueue` topology action、`AsyncCore` deferred boundary、
`PadataCore` deferred boundary、`PageAllocator` late action 和 `RuntimeCoreBoundary.setup()`。
`Scheduler.enable_smp()` 必须发布 SMP sched domain ready、PID 1 boot CPU affinity 已解除、
`PF_NO_SETAFFINITY` 已清除、RT/DL SMP 后置状态 ready 和调度 granularity 已刷新事实。由于 `Scheduler`
主对象此前已经 `Online`，该动作不得重新推进 `Scheduler` 主生命周期。

`Workqueue` 和 `PageAllocator` 在当前对象级 prototype 中保持其前序 `Ready` 主状态；RuntimeCore 通过
topology/late action facts 表达 `workqueue_init_topology()` 和 `page_alloc_init_late()` 的完成边界，避免破坏前序
phase 对 `Workqueue.Ready`、`PageAllocator.Ready` 的历史不变式。`async_init()` 和 `padata_init()` 在本轮保留
Linux 时序位置，但必须显式记录为 deferred boundary，不能静默假设可用。

测试应覆盖 `RuntimeCorePhase.Ready`、`Scheduler.smp_initialized`、PID 1 affinity 释放、Workqueue topology
facts、Async/Padata deferred facts、PageAllocator late facts，以及下一入口仍是 `do_basic_setup()`。

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

测试应覆盖 `InitcallPhase.Ready`、Cpuset trimmed、DriverCore/IrqProcView deferred、CtorTable 表位置、
`InitcallTable.preset()` 的静态 range 收集事实、level/entry 摘要、entry operation binding、
`InitcallTable.all_levels_ran`、命令行 scratch 和运行上下文事实，以及下一入口仍是 `kunit_run_all_tests()` /
`RootfsPhase`。具体 entry 的目标副作用，例如 `of_platform_default_populate_init()` 填充 platform bus，应由
对应对象规格和后续 smoke 测试覆盖，不混入 initcall 机制本身。

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

该 action 的遍历规则参考 Linux 6.12.37 `drivers/of/platform.c`：
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
参考 Linux 6.12.37 `kernel/printk/printk.c::register_console()` 的交接边界，真实 serial console 成为
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
不得继续停留在只写 `PrintkBuffer` 的事实层。当前 serial8250 后端必须建模为无中断 polling write：
每个待发送字符按 Linux `uart_console_write()` 语义处理换行 CRLF，发送前观察 LSR/THRE ready 条件，然后写 THR/TX；
寄存器地址必须从 `Uart8250Port.membase` 加 `reg_shift` 派生，并尊重 `reg_io_width`。代码只有在
`VmallocAllocator.map_page_range()` 已把 vmap VA/PA 映射安装进当前 swapper 页表并记录 runtime mapping ready
后，才能对 `membase` 派生出的 LSR/THR 地址做 `read_volatile`/`write_volatile`；不得直接访问 `mapbase`，
也不得用 SBI 路径伪装真实 UART 写。由于 PLIC/IRQ 驱动尚不可用，本阶段不得实现或声明 interrupt-driven console
output ready，只能记录 IRQ 输出路径 deferred。
为避免 handoff 后重复输出，`ConsoleRegistry` 必须区分 printk 记录保存和 legacy boot-console drain cursor。
注册 preferred serial8250 console 时，应先把 boot console pending records 按 boot console 路径 flush 并推进 cursor；
serial8250 route 成功写出的记录必须标记为已交付，不得再被 `earlycon::drain_printk()` 经 SBI 重放。默认
handoff 完成后，EarlyCon 后端必须进入 offline/disabled 状态；后续直接推进 earlycon event/action 或调用
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

本阶段覆盖 `kunit_run_all_tests()`、`wait_for_initramfs()`、`console_on_rootfs()`、
`init_eaccess(ramdisk_execute_command)` 对应 checkpoint、`prepare_namespace()` 和
`integrity_load_keys()` 的时序位置。当前 `CONFIG_KUNIT=n`，`kunit_run_all_tests()` 必须建模为
trimmed/no-op，不得单独升格为 `KUnitPhase`。

`InitramfsSyncDeferred`、`RootfsConsoleDeferred`、`RootFsEnableDeferred` 和 `IntegrityKeysDeferred`
在本轮只保留 deferred/position-preserved 语义。其中 `init_eaccess(ramdisk_execute_command)` 是 required
checkpoint，必须记录当前 Linux-like 路径要求进入 `prepare_namespace()` 分支。`RootFsEnableDeferred`
不得伪造真实 root device 探测、devtmpfs mount、`MS_MOVE` 或 `chroot(".")` 已完成。

测试应覆盖 `RootfsPhase.Ready`、KUnit trimmed、initramfs wait deferred、rootfs console deferred、
ramdisk eaccess 强制进入 prepare_namespace、RootFS enable deferred、integrity keys deferred，以及下一入口仍是
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

`SystemState.enable()` 是本阶段的主要状态动作：它必须从 `SYSTEM_SCHEDULING` 进入
`SYSTEM_FREEING_INITMEM` 窗口，并最终发布 `SYSTEM_RUNNING`，把 `SystemState.state` 推进到 `Online`。
`RcuCore.end_inkernel_boot()` 是本阶段的另一个主线 action，必须记录 `rcu_boot_ended == true`，但不得把完整
RCU GP 服务或 worker 运行伪装成已实现。

测试应覆盖 `FinalizePhase.Ready`、async full sync deferred、init memory cleanup deferred/trimmed 事实、
mapping protection deferred、PTI trimmed、`SystemState.Online`/`SYSTEM_RUNNING`、RCU in-kernel boot ended、
sysctl args deferred，以及下一入口仍是 `PayloadPhase`。

## PayloadPhase 编码约束

`PayloadPhase` 是 `StartupTimeline` 的末尾阶段，formal model 路径为 `spec/model/payload/`。它必须作为
`SmpRuntimePhase.Ready` 之后的后续阶段实现，而不是嵌套为 `SmpRuntimePhase` 的子阶段。入口必须要求
`FinalizePhase.Ready` 和 `FinalizeBoundary.Ready`，并消费 `payload_phase_next_boundary()` 事实。

KernelInitTask 的执行线从 `SmpRuntimePhase` 入口开始：`rest_init()` 通过 `TaskCreationCore` 把
`TaskEntry::KernelInit` 绑定到 `KernelInitTask`，并提交 release/dispatch facts；`SmpRuntimePhase` 的首个子阶段 `PreSmpInitPhase` 消费这些 entry/release/dispatch facts 后进入。后续阶段按 phase 顺序衔接到 `FinalizePhase`，再自然进入 `PayloadPhase`。因此 selected payload 的执行归属应从这条连续执行线推出，而不是由 `PayloadPhase` 单独声明一个调用者事实。`BootIdleTask` 只负责 idle loop、need_resched observation 和 schedule boundary；`KthreaddTask` 当前提供内核线程管理者 ready/provider 事实和最小 schedule-loop 入口边界。

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

`80966ec` 曾暴露 `13 obligation / 2 deferred`。这些条目不能作为实现可忽略的提示；处理原则是：实现某个对象事件前，必须先通过“推导义务门禁”。能由模型、推导工具、链接脚本、ISA、固件规范或已证明前序事实推出的，应优先补齐推导证明；只能由外部交付保证支撑的，应明确作为 source assumption，并在后续对象事件中尽快转化为运行期检查；无法归类的应作为规格缺口或显式 deferred。

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

以下条目来自对照 `linux-6.12.37/default_config` 后的规格缺口复查。它们不要求立即实现，但后续应先补入
`spec/model` 对应对象或 phase 的 `deferred`，避免读者把当前最小模型误解为已经完整覆盖参考 Linux
启动路径。

| 候选项 | 参考配置/路径 | 建议归属 | 说明 |
| --- | --- | --- | --- |
| RISC-V Linux boot image header / boot protocol header | RISC-V 入口协议 | `PreparePhase` 或 `Lds` | 当前由 `BootArgs`、`Lds` 前置事实吸收，但 header 本身没有说明不展开。 |
| EFI stub / PE header 入口细节 | `CONFIG_EFI=y`、`CONFIG_EFI_STUB=y` | `PreparePhase` 或 `Lds` | 现有 deferred 只覆盖 `efi_init()`，未覆盖 EFI stub/header 入口。 |
| SATP mode 探测与页表层级降级 | `CONFIG_PGTABLE_LEVELS=5` | `Config` 或 `Vm.Preset` | 当前 `Config.satp_mode` 是既定事实，未描述 Linux 的运行时探测/降级过程。 |
| `apply_early_boot_alternatives()` | `CONFIG_RISCV_ALTERNATIVE_EARLY=y` | `Vm.Preset` 或 `EarlyVm.Setup` | 早期 alternatives/errata patch 尚未作为对象或 deferred 标记。 |
| `set_task_stack_end_magic()` | `CONFIG_SCHED_STACK_END_CHECK=y` | `BootInitStack` | 可折叠进栈保护语义，但应说明当前不展开 Linux 的具体检查标记。 |
| `init_vmlinux_build_id()` | `start_kernel()` early generic path | `EntrySuccessorPhase` | 当前未建模 build id 初始化，也未标记 deferred。 |
| `page_address_init()` | `start_kernel()` before `setup_arch()` | `EntrySuccessorPhase` | 当前没有 page address 元数据对象。 |
| `setup_command_line()` / saved cmdline | `start_kernel()` after `setup_arch()` | `CommandLine` | `CommandLine` 管理 raw/saved/static 三个文本视图；Param 解析对象由 `Params` 管理并保持原时序。 |
| DT unflatten | `CONFIG_OF_FLATTREE=y` | `EarlyDtb` 或后续 DT 对象 | 当前只覆盖 early scan 所需事实，未标记 unflatten 阶段。 |
| `phys_ram_base` / `kernel_map.va_pa_offset` 建立 | `CONFIG_64BIT=y`、`CONFIG_MMU=y` | `MemBlock.Setup` 或 `SwapperVm.Setup` | 当前折叠进映射正确性谓词，未单独说明。 |
| `ZONE_DMA32` / zone 边界初始化前置事实 | `CONFIG_ZONE_DMA32=y` | `MemBlock.Setup` | 当前未抽象 zone 边界与 DMA32 限制。 |
| hugetlb 早期保留 | `CONFIG_HUGETLB_PAGE=y` | `MemBlock.Setup` | 当前只保留 memblock 高层结果，未展开 hugetlb reserve。 |
| final page table 权限细分 RW/RO/NX | `CONFIG_STRICT_KERNEL_RWX=y` | `SwapperVm.Setup` | 当前 `SwapperVm` 只要求映射 ready，未细化 text/rodata/data 权限域。 |
| `riscv_fill_hwcap()` / ISA 能力发布 | FPU/V/Zicbom 等启用 | 后续 `CpuFeature` / `UserIsa` 对象 | 当前边界没有 CPU feature/hwcap 发布对象。 |
| `apply_boot_alternatives()` | `CONFIG_RISCV_ALTERNATIVE=y` | 后续 `Alternative/Patch` 对象 | boot alternatives 未建模，且不同于 early alternatives。 |
| `riscv_user_isa_enable()` | RISC-V ISA 配置相关 | 后续 `UserIsa` 对象 | 用户态 ISA 暴露语义当前不属于最小闭环。 |

由此得到后续 `EntryPreludePhase` 实现顺序：

1. 持续保持 `Lds`/`KernelImage` 相关符号和布局检查面，确保 `_start`、`kernel_start`、text 起点、ELF entry、head text 范围在实现中可对应。
2. 继续实现 `RawDtb.Preset/Setup` 的最小确认路径，把 OpenSBI handoff 假设转化为 header/magic/totalsize/range 的运行期检查。
3. 在上述事实具备后，推进 `FixMap`、`TrampolineVm`、`EarlyVm` 和 `Vm` 三段切换。

## 应用复用

当前对象级实验不复用现有 ArceOS Unikernel 应用，不依赖 `ax-std`、`ax-api`、`ax-feat` 或 `arceos-rust`。

第一轮保留两个内建 payload：默认 `APP=smoke` 和最小独立 `APP=hello`。对象级初始化完成后，启动链沿 `KernelInitTask` 的 `TaskEntry::KernelInit` 从 `PreSmpInitPhase` 开始的连续执行线进入 `PayloadPhase`，在 `PayloadPhase.Enable` 提交后调用 selected payload 的 `run() -> !`。当前 `smoke` payload 在 `impl/arceos_ex/src/apps/smoke/cases/` 下维护可返回测试用例，首批覆盖输出路径、格式化输出、MemBlock 分配和 FDT 查询。`APP=hello` 仍作为最小独立 payload，只通过 printk 前端输出 `Hello, world!` 后通过 SBI 关机；它不得直接调用 early console 或 real console backend。

所有 payload 的入口约定为 `run() -> !`。这表示控制流不返回启动编排链：Unikernel payload 可以进入服务循环或停机，未来宏内核 payload 可以加载首个用户态程序并完成用户态切换。若某个 payload 意外返回，应视为违反 `PayloadPhase.Enable` 的 no-return handoff 契约。

当前 payload 选择由 Makefile 变量控制，`APP` 会转换为 Rust `--cfg app_<name>`，例如 `APP=smoke` 对应 `app_smoke`。后续新增 payload 时，应在 `impl/arceos_ex/src/apps/` 下新增模块，并在 `apps/mod.rs` 中加入对应静态选择分支。后续新增 smoke 用例时，应放在 `impl/arceos_ex/src/apps/smoke/cases/` 下，并返回 `SmokeResult`，不得使用 payload 级 `run() -> !` 契约。

在未来 Composition Phase 中，再恢复“Unikernel app 引领内核形态”的 ArceOS 设计，并讨论如何接入 `ax-std`、测试 payload 和宏内核 payload。

### smoke 测试边界

smoke payload 用于覆盖 QEMU 运行期可观察行为，以及规格推导不能单独替代的实现效果，例如控制台输出、格式化输出、内存分配动作、FDT/DeviceTree 公开查询接口、资源摘要和 payload 关机路径。若某个性质仅仅是在重复对象状态、生命周期顺序、内部副本一致性或谓词不变量，并且已经能由 `make verify` 的规格推导闭合，则不应为它新增 smoke 用例。

实现也不应为了 smoke 暴露原本不需要公开的内部状态查询接口。若某个对象同时有可验证的不变量和用户可观察行为，smoke 应测试后者；前者保留在模型谓词、推导验证和对象事件推进检查中。例如 `CommandLine` 的 raw/saved/static 文本视图一致性属于规格和实现状态推进约束，不需要单独增加只读取内部状态的 smoke case。

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
`key=value` 调试字段。机器可解析的状态序列应通过 checkpoint trace 或后续结构化报告承载，不应挤进普通启动日志。

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
- `impl/arceos_ex` 的完整 `make build`、`make run`、`make run LOG=trace`。
- QEMU smoke test，检查 smoke 汇总输出、独立 `APP=hello` 输出或 checkpoint 序列。
- 生成 unresolved obligations、deferred items、对象覆盖表、QEMU 日志等报告。

手动触发可用于规格大改后立即刷新展示结果，也可后续增加参数，例如指定 spec、指定 kernel implementation、是否运行 QEMU、是否发布 Pages。

### CurrentCPU 与锁上下文变更后的生成要求

后续若模型正式引入 `CurrentCPU`、`LocalInterruptControl`、`CurrentTaskSlot`、`PreemptionControl` 或 `RawSpinLock`，不能只完成规格验证就停止。规格验证通过后，必须检查对象级实现和代码生成指导是否受影响。

重点检查范围：

- `CurrentCPU` 是否拥有对应 CPU 对象，`CpuGroup` 是否只维护这些 CPU 对象的引用、索引和拓扑组织关系。
- possible secondary CPU 是否仍只是 `CpuGroup`/topology 中的候选或描述；AP 真实进入 secondary entry 前，不得生成 live AP `CurrentCPU`，也不得把 AP 的 local interrupt、current task slot 或 task preemption 控制链视为可操作。
- `BootCPU` 是否仍作为独立实现对象存在，或已经退化为 `CurrentCPU.cpu` 指向 CPU 的 bootstrap role/alias。
- `InterruptStream.Enable/Setup` 是否仍直接维护 boot CPU 本地中断总开关事实，或已经改为只在 lifecycle event 中驱动 CPU-local `LocalInterruptControl`。
- 是否仍有 `InterruptStream` 或其它对象直接改写 `sstatus.SIE` 总开关；接管后只有 `LocalInterruptControl` 可以直接操作本 CPU 中断总开关状态。`InterruptStream` 可以直接管理 `sie/sip` source enable / pending 分开关。
- `BootRunQueue.curr`、`BootCPU` 当前任务事实和 scheduler setup 是否需要收敛到 `CurrentTaskSlot`。
- `preempt_disable()` / `preempt_enable()` 语义是否通过 `CurrentCPU -> cpu -> CurrentTaskSlot.current_task -> PreemptionControl` 表达。
- `KernelInitTask.Enable` 是否通过 `WakeUpNewTaskContext.guard: RawSpinLockIrqSaveGuard` 绑定的 `RawSpinLock.LockIrqSave/UnlockIrqRestore` 进入和退出资源独占上下文，并只在该 guard 保护区内驱动受保护资源对象的 action/event。
- checkpoint、trace 注释、smoke case 和 KUnit case 是否仍引用旧的 `BootCPU` 或裸 `boot_cpu_current_is_idle_task(...)` 事实。

若上述检查表明正式对象、event/action、trace checkpoint 或上下文边界发生变化，应只重新生成受影响部分，不得全局重排无关实现。预计受影响的实现范围包括 CPU/current 相关对象、CPU-local interrupt 控制对象、task preemption 控制对象、raw spinlock wrapper、`rest_init` 中 `KernelInitTask.Enable` 路径、trace 输出以及 smoke/KUnit 测试注册。

有实现变化时，验证至少覆盖：

```text
make build APP=smoke
make test
make verify
```

同时应运行受影响的 KUnit 入口和 smoke case。smoke/KUnit 应覆盖 `CurrentCPU` 绑定、CPU-local interrupt save/restore、current task slot、task preemption disable/enable，以及 `RawSpinLock.LockIrqSave/UnlockIrqRestore` 驱动 `KernelInitTask.Enable` 的最小路径。若某项暂时无法实现，应在规格或 coding 文档中记录明确 deferred 边界，不得用普通 TODO 代替。

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
- `CpuIdMap`
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
- `CorePreparePhase` 编排的最小对象骨架：`DeviceTree`、`Zones`、`ResourceTree`、`CacheBlockInfo`、`CpuCapabilities`、`CommandLine`、`PerCpuStorage`、`CpuHotplugState`、`Params`、`BootParam`、`PayloadParam`、`Randomness`、`ExceptionTable`。这些对象不是 `CorePreparePhase` 的下级对象；phase 只驱动其生命周期事件。`SavedCommandLine` / `StaticCommandLine` 是 `CommandLine` 的子视图对象，不是独立顶级对象；`EarlyParam` / `BootParam` / `PayloadParam` 是 `Params` 的子对象。`PerCpuStaticImage`、`PerCpuFirstChunk` 和 `PerCpuOffsetTable` 是 `PerCpuStorage` 的子对象；first chunk 的 unit 个数和 offset table 边界必须基于 `CpuGroup` 的 possible CPU 集合，而不是 online CPU 集合。

`Randomness.preset()` 对应 `random_init_early(command_line)` 的早期语义。实现应建立 early seed material，并至少混入 `StaticCommandLine`；具体 mix/hash 算法不由 coding 规格限定。当前 RISC-V64 最小实现允许记录 arch entropy 为 0，且 `Randomness.Prepared` 不得暴露正式随机数接口或表示完整 RNG ready。

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

## `mm_core_init()` 编码约束

`MmCoreInitPhase` 已正式落到 `spec/model/boot/mm-core-init/`。实现侧必须保持与模型一致的阶段边界：入口是 `CorePreparePhase.Ready`、`ExceptionStream.Ready`、`MemBlock.Online`、`DmaCachePolicy.Ready`、`StaticBranch.Ready` 和 `SystemExclusive`；出口是 `PageAllocator.Ready`、`MemBlock.Offline`、`SlubAllocator.Ready`、`PageTableCaches.Ready`、`VmallocAllocator.Ready`、`MmStructCache.Ready`。

`PageAllocator.preset()` 只做 `build_all_zonelists(NULL)` 和 `page_alloc_init_cpuhp()` 对应的拓扑与 hook 建立：`BootZonelistSet.Ready`、fallback zoneref 顺序、NULL sentinel、`CPUHP_PAGE_ALLOC` step 注册。它不得释放 MemBlock 页，也不得把自身推进到 `Ready`。

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

`PageRef` 的实现语义应对齐 Linux `struct page *`：它引用 `PageMetadataMap` 中的具体 page metadata 项，而不是直接等同
物理地址或线性映射虚拟地址。`PageMetadataMap` 对应 Linux `mem_map` 或 RISC-V `SPARSEMEM_VMEMMAP` 下的 `vmemmap` 视图，
必须覆盖 PageAllocator 管理的 PFN 范围。实现应提供并内部复用 Linux-like 转换边界：`pfn_to_page(pfn)`、
`page_to_pfn(page_ref)`、`phys_to_page(phys)`、`page_to_phys(page_ref)`、`virt_to_page(linear_addr)` 和
`page_to_virt(page_ref)`/`page_address(page_ref)`。这些转换必须检查或依赖 PFN 有效、地址页对齐、地址属于已建立 direct map
或 vmemmap 覆盖范围；不得把任意整数地址直接伪造成 `PageRef`。

当前 `arceos_ex` 实现采用 flat `mem_map` 形式：`PageMetadataMap.setup()` 在 `MemBlock.Online` 且
`SwapperVm.Online` 后，通过 `memblock.alloc_phys()` 分配一段页对齐的连续 metadata storage，并把它表示为以 PFN
为索引的 `PageMetadata` 数组。`PageRef` 必须绑定到 `mem_map[pfn - start_pfn]` 对应 slot；PFN、物理页地址和 direct-map
线性地址只是从该 slot 的 PFN 关系派生出的转换结果。将来若切换到 sparse/vmemmap，只能替换 metadata storage 布局，
不得改变 `PageRef` 指向 page metadata 项这一契约。

`SlubAllocator.Ready` 之后必须暴露正式的 Linux-like kmalloc API：`kmalloc(size, gfp)`、`kzalloc(size, gfp)` 和
`kfree(alloc_ref)`。model 层对应 `SlubAllocatorType.Action::Kmalloc(size, gfp) -> KmallocAllocRef`、
`SlubAllocatorType.Action::Kzalloc(size, gfp) -> KmallocAllocRef` 与
`SlubAllocatorType.Action::Kfree(alloc_ref)`；coding 层可按 Rust 需要调整参数和返回封装，但必须保留 size、GFP、
caller-owned allocation reference、线性映射读写、kzalloc 返回前清零，以及 kfree 释放后对象回到所属 kmalloc cache 的契约。

当前第一轮 `arceos_ex` SLUB/kmalloc 实现只要求 Linux-like page-backed slab：`KmallocCaches` 使用固定默认 size classes
`8/16/32/64/128/256/512/1024/2048/4096/8192`，请求 size 通过向上取整选择 size class；当某个 cache 没有空闲对象时，必须通过
`PageAllocator.alloc_pages()` 获取 backing page，把 page 切成同尺寸 slots 并挂入该 cache 的 freelist。4KiB page 配置下，
8KiB cache 必须使用 order-1 backing page，模拟 Linux `KMALLOC_MAX_CACHE_SIZE = PAGE_SIZE * 2` 的普通 kmalloc cache 边界。第一轮可先不在
空 slab 时把整页归还给 `PageAllocator.free_pages()`，但所有 backing pages 必须由 `PageAllocator` 拥有并通过 `PageRef`
建立 direct-map slot 地址。freelist 节点可以使用 slot 内存的 intrusive next pointer 或等价的固定元数据表示，但不得依赖
`Vec`、`Box`、全局 heap 或 MemBlock。`kmalloc` 不保证清零；`kzalloc` 必须清零返回对象的 requested size 范围；
`kfree` 必须拒绝不属于任一 kmalloc cache/slab 的引用，并把有效对象放回所属 cache 的 freelist。NUMA、per-CPU partial、
slab debug redzone/poison、freelist random/hardened、memcg kmalloc、reclaim/compaction 和 slab sysfs/FULL 状态均 deferred。

`KernelGlobalAllocator.Setup` 必须发生在 `SlubAllocator.Ready` 之后，它表示 Rust `core::alloc::GlobalAlloc` 边界已经可用，
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
或映射跨出当前已预分配的最小 vmalloc 页表范围，实现必须返回失败，不得只记录 `VmapMapping.installed=true`。
当前实现可以把支持范围限制在
`VMALLOC_START` 起始的首个 L1/L0 页表窗口，但该限制必须通过 ready/failure fact 暴露，后续再泛化到完整
`VMALLOC_START..VMALLOC_END`。
具体实现不能只依赖底层 `vpn0` 越界失败；在调用页表安装前，`VmallocAllocator.map_page_range()` 必须显式确认
`VmapArea` 完全落在当前 runtime mapping window 内。落到后续 L0/PTE window 的 area 当前必须失败并记录为
cross-window deferred，不能复用首个 L0 表错误安装。同时，同一个 busy `VmapArea` 已经存在 installed
`VmapMapping` 时，第二次 `map_page_range()` 必须失败，避免一个 area 产生多个并存页表映射记录。

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

`MmStructCache.setup()` 只建立 `"mm_struct"` cache。`vm_area_struct` cache、`vma_lock_cachep` 和 `mmap_init()` 属于后续 `proc_caches_init()` 或进程地址空间初始化路径，不得为了填满本阶段而提前塞进 `MmStructCache`。

`PageExt`、`KFENCE`、`KMSAN`、`Kmemleak`、`DebugObjectsMemory` 和 `ExecMemory` 当前按 `linux-6.12.37/default_config` 记录为 model `deferred`/trimmed 路径。实现若遇到这些调用位置，应输出 checkpoint 或保留 no-op 分支说明，不得散落 TODO 来替代正式规格记录。

## `SchedInitPhase` 编码约束

`SchedInitPhase` 已正式落到 `spec/model/boot/sched-init/`。实现侧必须保持与模型一致的阶段边界：入口是
`MmCoreInitPhase.Ready`、`PageAllocator.Ready`、`SlubAllocator.Ready`、`KmallocCaches.Ready`、`CpuGroup.Ready`、
`CpuIdMap.Ready`、`PerCpuStorage.Ready`、`CpuHotplugState.Ready`、`StaticBranch.Ready`、`PrintkBuffer.Ready` 和
`SystemExclusive`；出口是 `Scheduler.Online`、`RadixTree.Ready`、`MapleTree.Ready`、`Workqueue.Prepared`、
`Softirq.Prepared`、`RcuCore.Ready` 和 `TasksRcu.Prepared`。

目录、文件和对象命名必须跟阶段名一致：模型目录为 `spec/model/boot/sched-init/`，实现文件为
`impl/arceos_ex/src/phases/boot/sched_init.rs`，阶段对象名为 `SchedInitPhase`。不得混用 `scheduler-init` /
`SchedulerInitPhase`，除非先正式改名并同步所有规格、图示和实现。

`Scheduler.preset()` 对应 `sched_init()` 的全局前置准备：默认 root domain、bit wait queue table 和调度类壳。当前
`SchedClass` 细分仍 deferred，调度类顺序检查只作为实现一致性检查或 checkpoint，不作为独立生命周期对象。

`Scheduler.setup()` 建立 possible CPU 的 runqueue 元数据，并把 boot CPU 的当前 `InitTask/current` 建模为
`BootIdleTask`。它不得分配新的 boot idle task，不得创建第二个 runnable task，也不得把完整 SMP 调度拓扑提前塞进本阶段。
`BootRunQueue.curr`、`BootRunQueue.idle`、`BootCPU.idle_thread_ref` 和 per-cpu idle task 引用必须收敛到同一个
`BootIdleTask` 事实。

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

`RadixTree.setup()` 和 `MapleTree.setup()` 只建立 node cache 与全局分配基础。具体 radix tree、IDR、XArray、maple tree
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
不再属于后续中断开放期的开头，而是本阶段的结尾。阶段出口必须满足 `InterruptStream.Online`、boot CPU
`sstatus.SIE` 已打开、`IrqController.Ready`、`RiscvIntc.Ready`、`IrqDispatchTree.Ready`、`Plic.Prepared`、`Tick.Ready`、
`TimerWheel.Ready`、`HrtimerCore.Ready`、`Timekeeper.Ready`、`RiscvTimerProvider.Ready`、`Softirq.Ready`、`Randomness.Ready`、
`SbiIpi.Ready`、`IpiMux.Ready` 和 `SmpCallFunction.Ready`。

目录、文件和对象命名必须跟阶段树一致：模型目录为 `spec/model/interrupt/irq-time-init/`，实现文件位于
`impl/arceos_ex/src/phases/interrupt/irq_time_init.rs` 等 `interrupt` 阶段子树下；旧
`phases/boot/irq_time_init.rs` 路径不得再作为本阶段实现位置。

当前按 OpenSBI 下的 RISC-V S-mode 路径建模：`RiscvIntc` 表示每 hart 直连 CPU 的 local interrupt controller，
`RiscvTimerProvider` 表示 Linux `timer-riscv` 风格的 time/clockevent provider，timer programming 走 SBI TIME 或后续 SSTC
能力，而不是直接把 CLINT 作为本阶段对象。IPI 路径按 `SbiIpi` 提供 software IRQ mapping 和 send action，`IpiMux` 在其上建立虚拟
IPI range；这对应 Linux `sbi-ipi` + generic `ipi-mux`。`Plic` 在本阶段只推进到 `Prepared` 占位，保留 external interrupt
provider discovery 和 parent 关系，不开放 external IRQ route，也不注册外部中断 handler。

本阶段打开的只是 boot CPU 本地中断总入口。普通任务并发、secondary CPU 并发、周期 tick 服务、workqueue worker
kthread、RCU GP kthread、IPI enable 和完整 softirq 执行路径仍不得提前解释为 Online。

`RiscvTimerProvider.setup()` 可以提供两个最小 action：`read_time()` 和 `schedule_oneshot(delta, callback) -> Option<deadline>`。smoke 必须分开验收
时间功能和时钟中断功能：前者确认 time source 可读且单调推进；后者注册一次性 clockevent callback，使用 SBI timer 在 deadline
到来时触发 supervisor timer interrupt，并确认 handler 返回前调用关联函数。该 smoke 不表示完整周期 tick 或 clockevent 运行期已经启动。

`poking_init()`、`ftrace_init()` 和 `context_tracking_init()` 当前按 RISC-V64/default_config 记录为 trimmed/no-op。
`early_trace_init()`、`trace_init()` 和 `housekeeping_init()` 当前保留为 model `deferred`。实现若遇到这些调用位置，应按模型记录
checkpoint 或 no-op 条件，不得以零散 TODO 代替正式 deferred。

## `IrqOpenPreparePhase` 编码约束

`IrqOpenPreparePhase` 已正式落到 `spec/model/interrupt/irq-open-prepare/`，属于 `InterruptPhase` 的第二个子阶段。它必须接在
`IrqTimeInitPhase.Ready` 之后运行，此时 boot CPU 本地中断总入口已经开放；不得把 `local_irq_enable()` 从
`IrqTimeInitPhase` 末尾移到本阶段开头。

目录、文件和对象命名必须跟阶段树一致：实现文件位于
`impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs` 等 `interrupt` 阶段子树下，不得落回 `boot` 阶段子树。

`kmem_cache_init_late()` 当前只实现为 `SlubAllocator.setup_flush_workqueue()` 内部资源事实：依赖
`SlubAllocator.Ready`、`KmallocCaches.Ready` 和 `Workqueue.Prepared`，记录 `flush_workqueue_ready == true`。
它不得把 `SlubAllocator` 推进到 `Online`，Linux `slab_state = FULL` / `SlubAllocator.enable()` 仍留给后续
`slab_sysfs_init()` 类 late initcall。

`console_init()` 当前只要求 `Console.Prepared`、`TtyLineDisciplineRegistry.Prepared` 和 `ConsoleDriverSet.Prepared`。
正式 console 进入准备边界后，真实 console device probe、boot console unregister 和完整 early/boot console handoff
仍是条件事实或后续设备初始化结果，不作为本阶段固定结束条件。

`sched_clock_init()` 对应 `SchedClock.setup()`，只发布 generic sched clock core 的启动期读数可用事实，不改变
`RiscvTimerProvider` 或 `Timekeeper` 的生命周期状态。`calibrate_delay()` 对应 `DelayLoop.setup()`，消费
`RiscvTimerProvider` 的 timebase/lpj fact，建立 `udelay`/`ndelay`/`mdelay` 等 Ready 后 action 的参数基础。

本阶段仍不得打开普通任务并发或 secondary CPU 并发；周期 tick 服务、完整 softirq 执行、IPI enable、workqueue worker
kthread 和 RCU GP kthread 仍保持 deferred。`setup_per_cpu_pageset()` 当前只作为 `PageAllocator.setup()` 的 per-CPU
pageset 快速路径 deferred 细项记录，不引入新的 lifecycle slot。

smoke 可覆盖两个用户可观察 action：`SchedClock.read()` 至少能返回随 time source 推进的读数，`DelayLoop.udelay(usec)`
能完成一个有界 busy-wait 并保持中断开放状态。smoke 不应为了重复内部状态不变量而暴露更多私有对象字段。

## CacheBlockInfo 编码约束

`CacheBlockInfo.setup()` 对应 Linux 6.12.37 的 `riscv_init_cbo_blocksizes()`。当前实现只发布平台级 `CBOM` 和 `CBOZ` block size 事实；`CBOP` 虽然存在 DeviceTree binding，但不在该 Linux 初始化点发布，暂不进入当前对象状态。

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

第一轮不得把现有 boot page table 代码简单改名为多个模型事件。每次页表切换必须显式处理 RISC-V64 所需的 `sfence.vma` 边界。

## checkpoint

checkpoint 是对象事件和 Phase 边界的可配置观测/探针分发点，不是普通日志函数，也不是状态机推进的一部分。第一轮预留 checkpoint hook 接口，但不要求实现完整状态差分输出。默认 hook 必须为空实现；trace、test、probe、verify、stop-at-checkpoint 或状态差分采集等具体 handler，都必须通过编译期配置启用，不得依赖运行期动态注册来改变 checkpoint 语义。

checkpoint handler 必须遵守以下约束：

- 不得推进对象或 Phase 状态，不得调用 `Lifecycle::transition`、`Lifecycle::adopt_transition`、`phases::state::mark` 或 `phases::state::adopt`。
- 不得把自身行为作为模型事件成功的前置条件；对象事件的 `ensures` 只能来自对象事件实现本身，不能来自可选 handler 的副作用。
- 默认只能观察只读上下文。handler 输入应优先是 `Checkpoint` 加 `&Context` 或更窄的只读 `CheckpointContext<'_>`；不得默认暴露 `&mut Context`。
- 若某个 checkpoint probe 必须调用会改变对象内部数据的功能 API，例如在 `MemBlock.Online` 后、`MemBlock.Disable` 前测试 `alloc_phys()`，必须通过该 checkpoint 专属的显式 probe capability 授权。该 capability 只能覆盖被测试 API 的最小能力，仍不得推进 lifecycle 或 Phase 状态。
- handler 可以失败并停机，也可以主动停机。建议 outcome 至少区分 `Continue`、`FailAndShutdown` 和 `StopAndShutdown`：前者继续启动，第二类表示 probe/verify 失败，第三类表示达到逐级构建或逐级验证目标后主动结束。
- handler 内部不得再次调用 checkpoint；实现应通过 guard 或模块边界防止 checkpoint 重入。若发生重入，应视为实现错误并停机或直接忽略内层 checkpoint，但不得递归执行 handler。

checkpoint trace 独立于 `EarlyCon` 和正式 `Console`。极早期地址空间阶段可以使用 RISC-V64 SBI legacy putchar 输出单个字符，用于定位 `EarlyVm` 切换生效前的最小事件；该路径不得依赖 allocator、锁、字符串地址、FixMap 或线性映射状态。`EarlyVm` 切换生效、完整 `KernelImage` 映射可访问后，trace 后端应输出稳定 checkpoint 名称字符串，而不是继续消耗单字符 id。

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
