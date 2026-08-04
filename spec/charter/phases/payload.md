# KernelInitFlow 的 payload 准备与提交

payload 边界由直接属于固定 `KernelInitFlow` 的叶子阶段、Flow-owned `UserAppRuntime` 和 exec action
表达。Hello、Smoke、UserBoot 构建配置仍必须恰好选择一种 payload。

## PayloadPreparePhase

KernelInitFlow 的 Online initialization Action 在 `FinalizePhase.Online` 后驱动本阶段。它准备公共
`ExecSyncBoundaries`、`ExecTransaction`、`UserCloneDeferredBoundaries`，确认唯一 selected payload 并
完成变种 setup。阶段 Online 对齐 `do_sysctl_args()` 返回后的 payload-selection boundary。

## PayloadHandoffPreparePhase

本阶段准备 selected payload、no-return entry、UserBoot image/address-space 与应用实例候选，并完成
exec 的可逆预检。若所属 Flow 尚无 `UserAppRuntime`，它可以创建唯一、终身稳定的 Runtime owned child；
若已存在则复用同一 Runtime。阶段 Online 时 KernelInitFlow 必须仍 Online，Runtime identity 不变，
旧 ApplicationInstance 尚未被替换。

## CommitPayloadHandoff

KernelInitFlow 和 Kernel 都 Online 后才能执行 `CommitPayloadHandoff`，并再次检查 parent
KernelInitTask.OnCpu、effective-flow guard 和 exec transaction epoch。唯一发送者是已提交 Online 的
`Kernel.Enable`。

- UserBoot 原子提交 Runtime 内部 old ApplicationInstance → new ApplicationInstance，更新用户地址空间/
  trap-frame 所属的 exec facts，然后进入用户态 no-return entry。Task、KernelInitFlow、FlowRef、
  UserAppRuntime、CpuRef 和 TaskThreadContext binding 均不替换。
- Hello/Smoke 不创建用户 ApplicationInstance，只进入固定 KernelInitFlow 下已经绑定的内核态
  no-return entry。

`KernelInitFlow.PayloadHandoffCommitted` 是 successful exec 的 commit checkpoint。任何候选、映像或
precheck 失败都不得伪造它；requested/default init 失败保持既有 terminal 规则。Kernel.Online 后 action
失败不回滚 Kernel，也不通过声明新 Flow 重试。

fork/clone 创建 fresh Task、fresh 普通 TaskFlow 和 fresh UserAppRuntime；child 后续 exec 只替换自身
Runtime 内部 ApplicationInstance。不同 Task/Flow 不共享 Runtime。

## 引用

- [阶段范式](../phase-paradigm.md)
- [TaskFlow](../objects/task-flow.md)
- [ExecTransaction](../objects/exec-transaction.md)
- [ExecSyncBoundaries](../objects/exec-sync-boundaries.md)
