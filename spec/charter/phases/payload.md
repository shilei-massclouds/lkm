# KernelInitFlow 的 payload 准备与提交

原 `PayloadPhase` 包装 lifecycle 被拆除，payload 边界由两个直接属于 `KernelInitFlow` 的叶子阶段和
一个 Flow action 表达。构建配置仍必须在 Hello、Smoke、UserBoot 中恰好选择一种 payload。

## PayloadPreparePhase

`KernelInitFlow.Setup` 在 `FinalizePhase.Online` 后驱动本阶段。它准备公共
`ExecSyncBoundaries`、`ExecTransaction`、`UserCloneDeferredBoundaries`，确认唯一 selected payload，
并完成变种 setup。`PayloadPreparePhase.Online` 继承原 `PayloadPhase.Ready` 的 Linux 对齐语义：
`do_sysctl_args()` 返回后的 payload-selection boundary。完成后只返回 KernelInitFlow.Setup。

## PayloadHandoffPreparePhase

`KernelInitFlow.Enable` 只驱动本阶段。该阶段完成 selected payload、no-return entry binding、
UserBoot 映像/地址空间准备、fresh `UserAppFlow.Preset/Setup` 以及 replacement 的完整可逆预检。
阶段 Online 时 `KernelInitFlow` 必须仍存活且未 Disable；`UserAppFlow` 最多为 Ready，active binding
仍指向 KernelInitFlow。该 Online 是 precommit 边界，默认不做 Linux exact mapping。

## CommitPayloadHandoff

`KernelInitFlow` 提交 Online 后才能执行 `CommitPayloadHandoff` action，且 action 必须再次检查 parent
`KernelInitTask.OnCpu`。

- UserBoot 固定执行 `KernelInitFlow.Disable -> KernelInitTask.CommitFlowHandoff ->
  UserAppFlow.Enable -> KernelInitFlow.Cleanup`，再提交用户 payload Online 和用户态 no-return entry。
- Hello/Smoke 不替换 Flow，只进入已经绑定的内核态 no-return entry，KernelInitFlow 保持 Online。

`KernelInitFlow.PayloadHandoffCommitted` 继承原成功 exec 后 `PayloadPhase.Online` 的 Linux 对齐语义。
任何候选、映像或 precheck 失败都不得伪造该 checkpoint；requested/default init 失败继续保持既有
panic terminal 规则。

## 引用

- [阶段范式](../phase-paradigm.md)
- [TaskFlow](../objects/task-flow.md)
- [ExecTransaction](../objects/exec-transaction.md)
- [ExecSyncBoundaries](../objects/exec-sync-boundaries.md)
