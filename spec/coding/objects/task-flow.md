# TaskFlow 编码映射

本文件是 `spec/model/objects/task_flow.spec` 的唯一权威 coding 映射。Flow lifecycle 必须与 Task
lifecycle 分开保存；owner/binding 协作不得产生第二个 Task carrier 或 application persona。

Task carrier、TaskRef、调度状态与 teardown 前置条件见 [`task.md`](task.md)。

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
实现共址，但必须有独立 lifecycle 查询。`KernelInitTask`（PID 1）的用户资源必须命名并组织为该
Task 的 user state；首次 exec 声明的 `UserAppFlow` 仅持有 application continuation lifecycle，
不得把 credentials、files、signals、PID 或 process-group 身份再封装成 persona carrier。

预分配的 Rust field 只是未占用 storage：生产路径必须先执行独立 `declare()`，把 fresh occurrence
物化在声明 Type 的 `initial_state`（当前 Task/TaskFlow 均为 `Base`），随后才能调用 `preset()`；
`declare()` 不得设置 owner、active binding 或推进 lifecycle。

统一 `TaskFlow` core 保存独立 lifecycle、`TaskFlowRef`、owner `TaskRef`、active binding、handoff
predecessor 以及 Disable/Cleanup facts。`TaskFlowRef` 与 `TaskRef` 使用相同的 private slot + nonzero
generation 规则；Destroyed Flow storage 可复用，但新声明必须递增 generation，旧 ref 必须失效。
每个用户 Task 保留两个 live UserAppFlow storage slot，以容纳 current Flow 和 exec staging Flow；
只有旧 Flow Cleanup 到 Destroyed 后才允许该 slot 用于下一 generation。

## Owner 与 active binding bridge

`TaskFlow` 通过 objects 内部最小 bridge 向 owner Task 注册 ownership；Task 提交唯一 active binding；
Flow `Disable`/exit 清除该 binding。bridge 只能使用经过 generation 校验的 `TaskRef` / `TaskFlowRef`，
不得公开 Task/Flow 字段、复制对方 lifecycle，或让 owner role wrapper 直接修改 Flow state。

TaskFlow slot 常量和 `USER_FLOW_SLOTS_PER_TASK` 属于本 module。通用 nonzero generation 递增 helper
可以保留为 `objects` 内部共享实现，但不得成为公开 identity API。

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

`KernelInitTask`（PID 1）首次 exec、每次 runtime exec 与每次 fork continuation 都建立 fresh Flow
generation；新旧 Flow state、trace/checkpoint facts 不得复用。每次 exec 在新 Flow Preset/Setup 后依次
完成旧 Flow Disable、active handoff、新 Flow Enable、旧 Flow Cleanup，并把 retired Flow 记录为
Destroyed。exit/exit_group 必须确认所有 prior owned Flow 与当前 Flow 都已清理，才允许 Task 退出。

所有可恢复的路径、映像和容量 precheck 必须在 Flow declaration 前完成；声明后的
owner/generation/lifecycle 不变量失败属于终止错误，不允许通过隐式 rollback 把同一 identity 恢复为
未声明。runtime exec 与 `KernelInitTask` 首次 exec 使用同一五步 handoff。

## 用户应用黑盒

`UserAppFlow.Enable` 只表示进入对应应用 continuation。应用内部 syscall、trap、files、地址空间、
credentials、signal 与任务身份操作继续路由到 `KernelInitTask`/当前用户 Task 及相应资源对象；不得在
Flow 类型内复制这些内核 actions。实现字段名、SyscallTable route fact 与 smoke assertion 也不得使用
已删除 persona 名称。

## 测试与诊断

- checkpoint 使用包含 runtime instance identity 的 `UserAppFlow.Online`，不保留临时具名 Flow alias。
- 动态 checkpoint observation 至少包含 FlowRef slot/generation；名称和 Linux marker 保持既有接口，
  不通过改 checkpoint 名编码 generation。
- smoke 必须验证 Flow lifecycle 独立、exec 前后 owner Task identity 不变，以及 fork continuation 与
  每次 exec 使用不同 generation。
- model/trace 与测试不得重新引入 `KernelInitTask` 首个用户 Flow、child Task、fork continuation Flow
  或 child exec Flow 的临时具名 compatibility alias。
- Task 与 TaskFlow 的 KUnit/smoke 专题可继续合并，因为它验证 owner/binding/handoff 跨对象协议。

AP idle Flow 使用统一 `TaskFlow` core。smoke scheduler/mutex/rwsem/rwlock 的 test-only TaskFlow 同样
委托统一 core，不能拥有平行 lifecycle 或公开 storage identity。
