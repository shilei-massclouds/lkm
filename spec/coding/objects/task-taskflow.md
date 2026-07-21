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
  不得把存储槽地址或测试字段当作可复用 TaskRef。

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
实现共址，但必须有独立 lifecycle 查询。PID 1 用户资源必须命名并组织为 `KernelInitTask` 的
user state；`Pid1UserAppFlow` 仅持有 application continuation lifecycle，不得把 credentials、files、
signals、PID 或 process-group 身份再封装成 persona carrier。

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

boot PID 1 的首个实例是 `Pid1UserAppFlow`。后续 runtime exec 与 fork continuation 每次使用 fresh
Flow generation；新旧 Flow state、trace/checkpoint facts 不得复用。exit/exit_group 必须先停止并
清理 Task 的 owned Flow，再允许 Task 退出。

## 用户应用黑盒

`UserAppFlow.Enable` 只表示进入对应应用 continuation。应用内部 syscall、trap、files、地址空间、
credentials、signal 与任务身份操作继续路由到 `KernelInitTask`/当前用户 Task及相应资源对象；不得在
Flow 类型内复制这些内核 actions。实现字段名、SyscallTable route fact 与 smoke assertion 也不得使用
已删除 persona 名称。

## 测试与诊断

- checkpoint 使用 `BootTask.Online`、`BootIdleSetup.Ready` 和 `Pid1UserAppFlow.Online`；不保留旧名称
  alias。
- scheduler handoff 日志使用 `BootTask -> KernelInitTask`，恢复日志使用 `BootTask restored`。
- smoke 必须验证 boot idle 前后 Task identity 相同、PID 1 exec 前后 Task identity 相同、Flow
  lifecycle 独立，以及连续 child allocation 的 logical identity/PID 不复用。
- 实现允许当前 DSL 只能形式化一个具名 child witness，但不得把该见证名或内部存储槽变成生产 API。
