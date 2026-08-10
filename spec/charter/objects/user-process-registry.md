# UserProcessRegistry

`UserProcessRegistry` 是全部用户进程 aggregate 的唯一可解析目录。固定容量为 32，其中 PID 1
占用一个终身稳定条目，另外 31 个条目允许在完整 reap 后递增 generation 并复用。每个已发布条目
独占一个 `UserProcessAggregate`；aggregate 物理包含 Task、其固定 TaskFlow、稳定
UserAppRuntime、当前 ApplicationInstance、独立 UserAddressSpace、UserStack、用户 TrapFrame、
files/fs、credentials、signal/wait 与 exec 状态。不同条目不得借用全局 active carrier 作为所有者。

## 引用、lease 与 CPU 所有权

条目只在单个 release publication 后可由 `{TaskRef, generation}` 解析。`UserProcessLease` 必须同时
验证 slot、generation、TaskRef、固定 FlowRef 和 `TaskFlow.cpu_ref`；写 lease 还必须验证调用 CPU 是
该进程终身固定的 owner CPU。stale ref、错误 CPU、已 terminal 或尚未 publish 的候选在暴露 aggregate
地址之前拒绝。lease 存活期间条目不得被 reap 或复用；reap 只在 runqueue、inbox、CurrentTask、trap
overlay 和所有 lease 引用都消失后提交。

PID 1 固定在 `online_cpus[0]`。普通 fork child 的 CPU 在 snapshot prepare 阶段按冻结、按 logical ID
递增的 online CpuRef 序列选择：`online_cpus[child_pid % cpu_count]`。选择后写入 child 固定 TaskFlow，
直到 terminal 都不得改变。vfork 或任何首轮接受的 `CLONE_VM` 形态必须继承 parent CpuRef，并维持
parent-blocking 串行 handoff；本轮不允许同一个 mm 同时在多个 CPU 执行。

vfork child 仍必须获得自己的 registry 条目、Task、TaskFlow、Runtime、kernel stack、TrapFrame 和
files/fs snapshot；不得复用全局 active-child 状态或另一个进程的 vfork snapshot。它在 exec 前以
generation-checked 链接直接引用仍存活的有效 mm owner，而不是占有或复制该 mm。每层 vfork 都保存
自己的 immediate-parent TaskRef 和一次性 completion/wake reservation，因此嵌套 vfork 形成独立的
逐层 handoff：只有 immediate child 的成功 exec 或 exit 可以唤醒对应 parent，不能唤醒、覆盖或读取
兄弟进程及祖先的 handoff 状态。parent 必须通过其固定 CPU Scheduler 失去运行资格，而不是忙等或仅
保存一个全局布尔状态。

## fork、exit 与 wait

普通 fork 在 publish 前完成 PID/slot reservation、generation、独立 mm/COW、kernel stack、TrapFrame、
目标 CPU、目标 inbox reservation 及所有资源准备。第一个 parent PTE 改写之后不得再执行可失败分配；
唯一 commit 使 child `Task/TaskFlow/UserAppRuntime = Online/Online/Online` 并把预留 inbox 消息变为可消费。
失败返回 `EAGAIN`（slot/PID/inbox 容量）或 `ENOMEM`（内存资源），且不得留下 PID、PTE、frame ref、
registry 条目或消息。

vfork 在 publish 前完成 child activation 和 parent completion/wake 两个 inbox reservation，以及除
独立 mm 内容外的全部 child aggregate 资源；两者中任一失败都按 fork 失败规则完整回滚。child publish
并在 parent 固定 CPU 入队后，parent 才可声明 scheduler sleep。child exec 在 point-of-no-return 把
新 image 提交到 child 自己的稳定 `UserAddressSpace` 对象，切断 shared-mm 链接且不 retire parent mm，
然后恰好一次发布 parent wake；child exit 不释放 parent mm，并同样恰好一次发布该 wake。wake 被消费
后 reservation/link 不得再次用于 exec、exit、reap 或 slot reuse。

普通 fork 为 child 建立独立 fd table，但每个继承的 live open-file description、pipe backing 和 endpoint
引用必须继续别名到 parent 的同一共享对象；这不是 `CLONE_FILES`，parent/child 后续 close 或 CLOEXEC
只改各自 fd table。pipe 的读写字节和 endpoint 引用计数由共享对象承载：任一 writer 在至少一个跨进程
reader 存活时可写，空 pipe 只在最后一个 writer 关闭后向 reader 返回 EOF。保存/恢复 parent fd table
snapshot 不得代替普通 fork 的共享 open-file backing，也不得复制 pipe 字节形成私有分叉。

进程 exit 必须在 zombie publication 前恰好一次释放该 aggregate 对 live open-file description、pipe backing
和 endpoint 的全部引用，并在同一个受保护事务中撤销 files 资源所有权。wait4 取得独占 reap 权后只清理
已经 Destroyed 且不再拥有 files 资源的 aggregate/Task 载体，不得再次递减这些共享引用。这与 Linux 6.12
在 `do_exit()` 中执行 `exit_files()`、而 `wait_task_zombie()`/`release_task()` 只回收 task 载体的职责分离一致。

当共享 pipe 为空且仍有 writer 时，阻塞式 read 不得以“不支持”或 EOF 返回。reader 必须先在共享
backing 下登记 generation-checked `{TaskRef, CpuRef}` wait，再通过 owner Scheduler 放弃运行资格；写入
首批可读数据或释放最后一个 writer 时合并一次面向该 Task 固定 CPU 的 wake。登记、条件重检与 sleep/wake
交接必须覆盖“wake 发生在 Task 真正离开 CPU 之前”的窗口：该 wake 要么使本次 PreparePrev 保持 runnable，
要么在 Task 已 blocked 后重新入队，不能丢失、重复入队或在持有 pipe/files 锁时调用 Scheduler。Task 恢复
后重新检查共享 backing，再决定复制数据、继续等待或返回 EOF。

exit 在条目内 release 发布 zombie/completion、wait word 和 SIGCHLD，然后向 parent 固定 CPU 合并一次
wake。该 wake 不得早于 wait4 实际可消费的完整完成条件；若 Task 的 owner-CPU retirement 是独立的
后续条件，则 wake 必须在该条件 release 发布后发出，或者以持久 pending 状态覆盖 parent 重检条件到
实际睡眠的窗口，不能只在较早的 zombie publication 时产生一次可丢失 IPI。vfork completion 是独立的
parent-blocking handoff，仍按其 immediate-parent reservation 发布，不替代稍后的 wait/reap wake。
wait4 的 PID `-1` 匹配当前 parent 的任一 child，正 PID 只匹配当前 parent 的该 PID child；不属于
当前 parent 的同 PID 进程不可成为等待、handoff 或 reap 目标。在匹配集合内，wait4 对一个 zombie 取得
独占 reap 权；复制 status 失败不得消费 zombie，成功后才清除调度引用并回收条目。exec 保持
aggregate、Task、TaskFlow、Runtime、CpuRef 和 UserAddressSpace 对象 identity，
只事务替换 address-space 内容及 ApplicationInstance；terminal 仍严格按 Runtime、TaskFlow、Task
顺序清理。

## 同步边界

registry 元数据、每个 aggregate 和共享 files/VFS/console 分别使用 IRQ-safe acquire/release spinlock；
页分配器与 frame metadata 使用其自己的锁。锁保护的数据只能经 guard-owned `UnsafeCell` 访问；记录
布尔 `locked` 事实而不排斥并发不构成锁。不得以一个全局 syscall mutex 串行化全部 ABI，也不得在
持有任一上述锁时调用 Scheduler。fork 通过 unpublished candidate 和 inbox reservation 避免跨锁半提交。

## 能力边界

首轮不导出 sched affinity ABI，不执行运行期 CPU hotplug、任务迁移、自动负载均衡、共享 mm 跨核、
ASID 或远程 TLB shootdown。online CpuRef 序列在 SMP bringup 完成后冻结。

## Mapping

- Model: `spec/model/objects/user_boot.spec`
- Coding: `spec/coding/objects/user-boot.md`
- Implementation: `impl/arceos_ex/src/objects/user_process_registry.rs`
