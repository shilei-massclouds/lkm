# Task 与 TaskFlow 编码映射

本文件是 `spec/model/objects/task.spec` 与 `task_flow.spec` 的权威 coding 映射。所有 task_struct-like
实体必须落到统一 Task carrier；Flow lifecycle 必须与 Task lifecycle 分开保存。内部 metadata、
runqueue 投影或用户资源字段可以拆成多个 Rust 结构，但结构拆分不得向 checkpoint、Context API 或
测试暴露为另一个模型 Task/persona。

## 统一 Task carrier

- 静态 `init_task` 只映射为 `BootTask`。入口期对象保存它的 lifecycle/storage 事实；Scheduler
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

Rust `Task` 必须是 lifecycle、identity、PID、CPU、thread/switch context 和 Flow ownership 的唯一
carrier。Boot/KernelInit/Kthreadd/AP/smoke/user-child 角色结构只能保存角色 metadata 或 continuation
scratch，并委托一个 `Task` core；它们不得另存上述 carrier 字段。linker-visible
`init_task_storage` 的首地址就是 PID 0 canonical `Task` 地址，`tp`、runqueue、current slot 和
BootTask API 必须解析到该同一地址。

## TaskRef 与 storage

`TaskRef` 是 value type，内部恰含 private storage slot 和非零 generation；角色 enum 不是允许的
identity lowering。静态 Task 使用固定 slot/generation。动态 `UserTaskSet` 提供 8 个轻量 Task
slot；每次把已回收 slot 重新声明为 Task 时 generation 单调递增（跳过 0），lookup 同时校验 slot
和 generation。只有所有 owned Flow 都 Destroyed、Task 自身 Destroyed 且 wait/reap 已释放 record
后，slot 才可再分配。stale ref 必须返回失败，不能解析到 slot 的新 occupant。

`CurrentTaskSlot`、runqueue entry、scheduler observation、锁 owner route、completed-child/wait record
统一保存 `TaskRef`。禁止 `CurrentTaskRef`、`UserChild` 等按角色枚举身份接口，也不保留 alias。
需要名称或 PID 的诊断必须先通过 owner storage 验证 TaskRef，再读取 Task metadata。

## TaskFlow lifecycle

Flow 的 `Base/Prepared/Ready/Online/Offline/Destroyed` 必须独立于 owner Task state。实现至少显式保存：

- owner Task identity；
- entry source；
- 当前 lifecycle state；
- fresh instance/generation identity；
- active binding 与 handoff predecessor；
- Disable/Cleanup 完成事实。

`RootStream` 继续作为 boot init Flow 的临时既有实现。`BootIdleFlow` metadata 归
`BootIdleRuntime`/idle continuation；`KernelInitFlow` 和 `KthreaddFlow` 可以和相应 kernel Task
实现共址，但必须有独立 lifecycle 查询。`KernelInitTask`（PID 1）的用户资源必须命名并组织为该 Task 的
user state；首次 exec 声明的 `UserAppFlow` 仅持有 application continuation lifecycle，不得把
credentials、files、signals、PID 或 process-group 身份再封装成 persona carrier。
预分配的 Rust field 只是未占用 storage：生产路径必须先执行独立 `declare()`，把 fresh occurrence
物化在 `Base`，随后才能调用 `preset()`；`declare()` 不得设置 owner、active binding 或推进 lifecycle。

统一 `TaskFlow` core 保存独立 lifecycle、`TaskFlowRef`、owner `TaskRef`、active binding、handoff
predecessor 以及 Disable/Cleanup facts。`TaskFlowRef` 与 `TaskRef` 使用相同的 private slot + nonzero
generation 规则；Destroyed Flow storage 可复用，但新声明必须递增 generation，旧 ref 必须失效。
每个用户 Task 保留两个 live UserAppFlow storage slot，以容纳 current Flow 和 exec staging Flow；
只有旧 Flow Cleanup 到 Destroyed 后才允许该 slot 用于下一 generation。

## Handoff lowering

boot idle handoff 保持 `BootTask` identity。实现顺序必须可观测为旧 `RootStream` inactive、
`BootIdleFlow` ready/active、current/runqueue 仍指向同一 `BootTask`；不得再发出旧 idle-task family
checkpoint。

successful exec 保持 owner Task identity，并按以下顺序提交：

```text
new flow preset/setup
old flow disable
task active-flow handoff
new flow enable
old flow cleanup
```

`KernelInitTask`（PID 1）首次 exec、每次 runtime exec 与每次 fork continuation 都建立 fresh Flow generation；
新旧 Flow state、trace/checkpoint facts 不得复用。每次 exec 在新 Flow Preset/Setup 后依次完成旧
Flow Disable、active handoff、新 Flow Enable、旧 Flow Cleanup，并把 retired Flow 记录为 Destroyed。
exit/exit_group 必须确认所有 prior owned Flow 与当前 Flow 都已清理，才允许 Task 退出。

fork 的提交顺序固定为 `fresh Task -> fresh fork UserAppFlow -> publish TaskRef -> owner/active bind`。
runtime exec 与 `KernelInitTask` 首次 exec 使用同一五步 handoff。所有可恢复的路径、映像和容量 precheck 必须
在 Flow declaration 前完成；声明后的 owner/generation/lifecycle 不变量失败属于终止错误，不允许
通过隐式 rollback 把同一 identity 恢复为未声明。child exit/exit_group 与 `KernelInitTask` shutdown 先
Disable/Cleanup 当前及所有 live owned Flow，再 Disable/Cleanup Task；reap 只能消费 Destroyed Task。

## 用户应用黑盒

`UserAppFlow.Enable` 只表示进入对应应用 continuation。应用内部 syscall、trap、files、地址空间、
credentials、signal 与任务身份操作继续路由到 `KernelInitTask`/当前用户 Task及相应资源对象；不得在
Flow 类型内复制这些内核 actions。实现字段名、SyscallTable route fact 与 smoke assertion 也不得使用
已删除 persona 名称。

## 测试与诊断

- checkpoint 使用 `BootTask.Online`、`BootIdleSetup.Ready` 和包含 runtime instance identity 的
  `UserAppFlow.Online`；不保留临时具名 Flow alias。
- 动态 checkpoint observation 至少包含 TaskRef slot/generation 和 FlowRef slot/generation；名称和
  Linux marker 保持既有接口，不通过改 checkpoint 名编码 generation。
- scheduler handoff 日志使用 `BootTask -> KernelInitTask`，恢复日志使用 `BootTask restored`。
- smoke 必须验证 boot idle 前后 Task identity 相同、`KernelInitTask` exec 前后 Task identity 相同、Flow
  lifecycle 独立，以及连续 child allocation 的 logical identity/PID 不复用。
- model/trace 与测试不得重新引入 `KernelInitTask` 首个用户 Flow、child Task、fork continuation Flow 或
  child exec Flow 的临时具名 compatibility alias。

AP boot-data 的 `task_ptr` 指向统一 `Task` core，AP idle Flow 使用统一 `TaskFlow` core；
`SecondaryIdleTaskSet` 只保存 model-deferred AP family 的聚合 metadata。smoke scheduler/mutex/rwsem/
rwlock 同样由 test-only role metadata 委托统一 Task 和 test-only TaskFlow，不能拥有平行 lifecycle、
CPU 或 switch-context carrier。
