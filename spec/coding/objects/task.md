# Task 编码映射

本文件是 `spec/model/objects/task.spec` 的唯一权威 coding 映射。所有 task_struct-like 实体必须落到
统一 Task carrier。内部 metadata、runqueue 投影或用户资源字段可以拆成多个 Rust 结构，但结构拆分
不得向 checkpoint、Context API 或测试暴露为另一个模型 Task/persona。

TaskFlow 的独立 lifecycle、generation、owner/binding 与 handoff lowering 见
[`task-flow.md`](task-flow.md)。

## 统一 Task carrier

- 静态 `init_task` 只映射为 `BootTask`。该对象从模型入口起就在 boot CPU 上执行，初态为 Task
  专属 `OnCpu`，并只保存稳定的
  storage/PID 0/`TaskRef::BOOT`/canonical identity 事实；Scheduler
  内部可以保存 boot-task scheduling metadata、pi lock、CPU ref 和 switch context，但该结构必须
  命名为 metadata/setup/view，不得拥有第二套 Task identity 或对外生命周期。
- `BootIdleSetup.Ready` 表示 scheduler 已把同一 `BootTask` 绑定为 boot CPU idle/current；它不是
  Task Ready checkpoint。切换日志、`TaskRef` 名称和 current-task 诊断必须继续显示 `BootTask`。
- `KernelInitTask` 与 `KthreaddTask` 各自保存独立 PID、scheduler Task、stack 和
  `TaskThreadContext`。`TaskCreationCore.CopyProcess` 的 destination 必须是这些 Task carrier，并绑定
  相应初始 Flow。
- 用户 child 的实现容器是 `UserTaskSet`。内部固定容量表、active execution record 或 continuation
  snapshot 都只是集合 lowering；每次成功 fork/clone 必须分配新的 logical Task identity 和 PID，
  并发布指向该 identity 的 fresh `TaskRef`；不得把存储槽地址或测试字段当作可复用 TaskRef。

Rust `Task` 必须是 lifecycle、identity、PID、execution authority、`TaskThreadContext`、typed `initial_flow` 和 Flow ownership 的唯一
carrier。Boot/KernelInit/Kthreadd/AP/smoke/user-child 角色结构只能保存角色 metadata 或 continuation
scratch，并委托一个 `Task` core；它们不得另存上述 carrier 字段。linker-visible
`init_task_storage` 的首地址就是 PID 0 canonical `Task` 地址，`tp`、runqueue 和 CurrentTask 解析
BootTask API 必须解析到该同一地址。

CPU assignment 不属于 Task carrier。`Task` 不得保存 `cpu_id`、`CpuRef` 或同义字段；可恢复执行路径
的 CPU 归属只存在于对应 `TaskFlow.cpu_ref`。调度器需要 CPU 时必须先解析即将提交/恢复的 Flow。

## Task 类型级 lifecycle lowering

普通 Task 的 `Base/Prepared/Ready/Online/OnCpu/Offline/Destroyed` 状态、
`TaskExecutionAuthority::{None,Reserved,Live}`、`TaskBreakpointState::{Invalid,Prepared,Valid}` 和
`preset/setup/enable/continue_on_cpu/suspend_from_cpu/disable/cleanup` 必须只实现于统一 Rust `Task` core。静态
`KernelInitTask`、`KthreaddTask` 与 `UserTaskSet` 创建的 child 都通过 core 的同一组方法推进；角色
结构不得定义同名 lifecycle-driving API，也不得保留转发 alias。它们只可暴露角色 metadata、只读
查询、受控的 `Task` core 访问，以及不推进 lifecycle 的 metadata commit helper。

`Task::preset` 负责 identity/TaskRef/initial-flow association/初始 Flow ownership/clone specification；
`Task::setup` 必须在 Phase 或 runtime fork 路径已调用 `TaskCreationCore::copy_process` 后消费其
copy-process 结果，建立 PID、stack/thread context 的初始寄存器字节、Prepared breakpoint、scheduler
entity 和 New/not-enqueued 状态；`Task::enable` 只在调用者已完成 running、runqueue publication 与
初始 Flow structural binding 后把 context 绑定 initial FlowRef 并原子提交 Online/None/Valid；它不启动
Flow。Scheduler prepare 必须验证 prev OnCpu/Live/Invalid/active-flow 和 next Online/None/Valid/FlowRef；
next 栈上的 finish 先验证架构已建立的 next 原始身份，再调用 context save/restore observation，并原子提交 prev
Online/None/Valid、next OnCpu/Live/Invalid、next active Flow 与 CurrentTask 选择结果，然后严格选择 Base initial Flow 的 `Preset` 或 Online active Flow
的 `Continue`。stale generation、无效 context、Reserved authority 或候选歧义必须无状态变化失败。
terminal task 使用 OnCpu 直接 Disable，context 保持 Invalid；identity switch 完全不调用上述方法。
`disable/cleanup` 必须继续检查 owned Flow 的 inactive/Destroyed 顺序。角色专用 flag、入口、
provider、CPU pin 或 global reference publication 不得写入这些通用方法。

`TaskCreationCore::copy_process` 的 inputs 必须包含 source `Task`、经 generation 校验的 source
`TaskRef` 与只读 `CurrentTask` capability。它在写入自身 commit record 或 destination metadata 前，
依次验证 source 为 `OnCpu`、authority 为 `Live`、source ref 解析回同一 carrier，且 capability
解析出的 generation-checked ref 等于该 ref。调用者不得传入 persistent `Online` 状态代替当前执行权；BootTask 不使用名称特判。
上述任一检查失败必须保持 core 和 destination 不变。

`BootTask` 使用 boot-only const initializer 直接构造 OnCpu/Live/Invalid `init_task_storage` 和固定
`TaskRef::BOOT`；不得复用普通 Task lifecycle 方法，也不得暴露 `preset/setup/enable` 或兼容 alias。
早期 `tp` 物理/虚拟地址模式与初始 preemption 事实由 EntryPrelude 私有
`BootTaskEntryBinding` lower；OnCpu 初态只由固件/架构入口执行权事实建立，不依赖尚未建立的 runqueue
或 runqueue current。各阶段只能静默验证该 carrier 仍为 OnCpu/canonical。

## TaskRef 与 storage

`TaskRef` 是 value type，内部恰含 private storage slot 和非零 generation；角色 enum 不是允许的
identity lowering。静态 Task 使用固定 slot/generation。动态 `UserTaskSet` 提供 8 个轻量 Task
slot；每次把已回收 slot 重新声明为 Task 时 generation 单调递增（跳过 0），lookup 同时校验 slot
和 generation。只有所有 owned Flow 都 Destroyed、Task 自身 Destroyed 且 wait/reap 已释放 record
后，slot 才可再分配。stale ref 必须返回失败，不能解析到 slot 的新 occupant。

runqueue entry、scheduler event observation、锁 owner route、completed-child/wait record 可以保存当时的
`TaskRef`，但不得成为当前任务权威副本。`CurrentTask` 是无状态、只读、短生命周期 capability，内部只携带
generation 校验后的 `TaskRef`；禁止 `CurrentTaskRef`、`UserChild` 等按角色枚举身份接口，也不保留 alias。
需要名称或 PID 的诊断必须先通过 owner storage 验证 TaskRef，再读取 Task metadata。

## 与 TaskFlow 的受控协作

Task core 分别保存 immutable initial Flow ref、owned Flow refs 与 active Flow ref；Flow lifecycle、generation 和 handoff predecessor
由独立 `TaskFlow` core 保存。双向协作只能通过 objects 内部的最小 bridge 完成：Flow 注册
ownership，Task 提交 active binding，Flow exit 时清除 binding。不得公开任一 core 的字段、复制
lifecycle state，或用角色 wrapper 绕过 owner/ref 校验。

`BootInitFlow` 是 BootTask 的 initial TaskFlow；BootInitScheduleHandoffPhase 在真实首次切换前直接把
后继 `BootIdleFlow` 建立为 Ready，并提交唯一 owner/active binding，但不得改写 initial Flow。
BootInitFlow lifecycle 与 BootIdleFlow active continuation 是两个独立事实，不得建立第二个 boot-init
Flow 或把 initial ref 当作 active ref。

`BootInitRestInitPhase` 必须对 `KernelInitTask` 与 `KthreaddTask` 分别完整执行
Preset/Setup/Enable：Preset lower 为 `copy_process`，Setup 当前无业务动作，Enable lower 为
`wake_up_new_task`。Enable 后 initial Flow 保持 Base；只有 Scheduler 真实切入相应 Task 后才发送严格
continuation Signal，启动 `KernelInitFlow` 或 `KthreaddFlow`。

fork 的 Task 侧提交顺序固定为 `fresh Task -> fresh fork UserAppFlow -> publish TaskRef ->
owner/active bind`。child exit/exit_group 与 `KernelInitTask` shutdown 只有在当前及 prior owned Flow
均完成 Disable/Cleanup 后，才可 Disable/Cleanup Task；reap 只能消费 Destroyed Task。

## 测试与诊断

- checkpoint 使用 `BootTask.OnCpu`、`BootIdleSetup.Ready` 和既有 Linux marker，不通过改名编码
  TaskRef generation。
- 动态 checkpoint observation 至少包含 TaskRef slot/generation；名称、PID 与角色只能在 ref lookup
  成功后读取。
- scheduler handoff 日志使用 `BootTask -> KernelInitTask`，恢复日志使用 `BootTask restored`。
- smoke 必须验证 boot idle 前后 Task identity 相同、`KernelInitTask` exec 前后 Task identity 相同，
  以及连续 child allocation 的 logical identity/PID 不复用。
- Task 与 TaskFlow 的 KUnit/smoke 专题可继续合并，因为它验证跨对象协议；合并测试不构成合并
  coding 权威。

AP boot-data 的 `task_ptr` 指向统一 `Task` core；`SecondaryIdleTaskSet` 保存已正式建模的按 logical-id
`ApIdleTask`/`ApIdleFlow` family storage 与聚合 observation。smoke scheduler/mutex/rwsem/rwlock 的 test-only role metadata 同样委托统一 Task，
不能拥有平行 lifecycle、CPU 或 switch-context carrier。

AP idle Task 使用专用 const/prepare 构造器直接建立 OnCpu/Reserved/Invalid、CPU/rq idle-current
reservation 与 initial Flow binding；不得经过普通 Task Enable，也不得保留 `adopt_ap_entry_on_cpu()`。
真实 secondary entry 验证 boot-data/`tp` 后只把 authority 从 Reserved 激活为 Live，并通过
`task_ptr.initial_flow` 校验/启动 keyed `ApIdleFlow`；Task lifecycle 保持 OnCpu。后续首次 Suspend 才
发布首个 Online/Valid breakpoint，之后 Suspend/Continue 仍只允许 Scheduler 驱动。

`TaskThreadContext` wrapper 必须位于统一 Task core，内部含 `TaskSwitchContext`、breakpoint state、绑定
FlowRef 与真实 save/restore 计数。scheduler/role wrapper 不得保存测试专用第二套 context carrier。
RISC-V `TaskSwitchContext` 仅含 `ra/sp/s0..s11`；汇编从独立 next Task pointer 参数建立 `tp`。对象层必须
集中提供 `tp` 地址到 canonical Task/TaskRef 的解析，覆盖 Boot、静态内核、动态 smoke/user 与 AP idle
storage，并拒绝未知地址或 stale generation；通用调用方不得取得裸地址或可变 Task 借用。
