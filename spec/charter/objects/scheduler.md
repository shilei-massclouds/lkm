# Scheduler

`Scheduler` 负责当前任务与候选任务的交换。每个 CPU 都拥有自己的 Scheduler，Scheduler 维护属于
该 CPU 的逻辑候选任务队列。一次任务交换严格在该 Scheduler 所属 CPU 上完成；跨 CPU 移动由独立的
迁移或协调机制处理。

## 三个基本概念

**当前任务**是正在该 CPU 上执行的唯一任务。Scheduler 每次作出选择时，都以这个任务为当前执行者，
并以它当时的运行资格和调度请求作为选择的起点。

**候选任务队列**是等待在该 CPU 上运行的任务组成的逻辑队列。它描述“哪些任务可以在这里被选中”，
并不要求下层只使用一条物理队列；Model、Coding 和实现可以按照调度类或其它策略，把它组织为多个
内部队列，只要它们共同表现为该 CPU 的候选集合。blocked 任务在被唤醒并重新取得运行资格前，不能
成为候选任务。

**任务交换**是暂停当前任务，并让 Scheduler 选中的另一个候选任务成为新的当前任务。若 Scheduler
最终仍选择当前任务，就只是一次重新选择，不发生任务交换。

## 每 CPU 所有权与可用状态

每个 possible CPU 恰好拥有一个 Scheduler；Scheduler 不被多个 CPU 共享，也不在系统中另建一个全局
Scheduler 来代替各 CPU 的本地选择。`sched_init()` 使 possible CPU 的 Scheduler 到达 Ready。CPU0 的
Scheduler 随 boot CPU 的可调度交接进入 Online；其它 CPU 的 Scheduler 只在各自 CPU online 交接时
进入 Online。

Scheduler 的当前任务和候选队列都只属于 owner CPU。跨 CPU 的 root/sched domain、负载协调和其它
全局资源是独立的共享对象，不属于某一个 Scheduler 的私有队列，也不复制 CPU 或 Scheduler 本体。

## 调度与交换

当前任务通过自己的固定 TaskFlow，请求 owner CPU 的 Scheduler 重新选择任务。Scheduler 结合以下事实
作出选择：当前任务是否仍可运行、队列中各候选任务是否具备运行资格，以及适用的调度策略。

若当前任务仍可运行，并且没有更合适的候选任务，当前任务继续运行。这个结果不保存或恢复任务的执行
位置，不改变当前任务身份，也不产生一次部分完成的交换。

若 Scheduler 选择了另一个任务，它先确认交换所需的当前任务、候选任务和本地执行关系都有效，然后：

1. 保存当前任务离开 CPU 时的执行位置，使当前任务暂停；
2. 恢复候选任务上次保存或首次准备好的执行位置；
3. 完成当前任务身份的交接，使候选任务成为该 CPU 唯一的当前任务并继续执行。

被换出的任务如果仍具备运行资格，可以继续留在或重新进入该 CPU 的候选集合；若它已经 blocked，则在
被唤醒前不能再次被选中。它未来再次成为当前任务时，会从原来的调度请求之后继续，而不是重新执行该
请求之前已经完成的工作。

首次运行与恢复运行使用同一个任务交换概念。候选任务的执行位置本身决定它是进入首次准备好的位置，
还是回到先前暂停的位置；Scheduler 不保存两种派发类型，也不据此选择两套交换协议。

## `yields` 与交换边界

`yields Scheduler.Schedule` 表示当前执行过程把重新选择请求交给 Scheduler，并等待 Scheduler 返回。
`yields` 只表达这段等待关系；它本身不保存机器执行现场，也不改变任务状态、候选队列或 CPU-local
binding。若需要任务交换，这些效果由 Scheduler 的交换过程明确完成。

Scheduler 接受请求前必须完成所有可能拒绝该请求的检查。交换开始前的拒绝不改变当前任务、候选任务
或它们的执行位置，因而不会留下部分交换。一旦新的当前任务身份已经提交，随后发现的严重不一致属于
终止失败；Scheduler 不回滚已经发生的交换，也不把同一交换自动重试为另一条路径。

## 能力边界

一个 Scheduler 只直接选择和交换 owner CPU 上的任务。它不能通过一次本地交换直接修改另一个 CPU 的
当前任务或候选队列。任务迁移、跨 CPU 协调、共享调度域和全局仲裁由独立机制决定；这些机制可以改变
任务之后属于哪个 CPU 的候选集合，但不改变每次任务交换必须在目标 Scheduler 所属 CPU 上完成这一
边界。

## Mapping

- Model: `spec/model/objects/scheduler.spec`
- Coding: `spec/coding/objects/scheduler.md`
- Implementation: `impl/arceos_ex/src/objects/scheduler.rs`
