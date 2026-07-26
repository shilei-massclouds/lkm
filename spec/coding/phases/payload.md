# Payload 直接子阶段与交接 action 编码指引

model 来源为 [`phase.spec`](../../model/phases/payload/phase.spec)。旧 `PayloadPhase` wrapper 已删除；
实现分为两个 KernelInitFlow 直接子阶段和一个 Flow Online action。

## PayloadPreparePhase

`KernelInitFlow.Setup` 在 Finalize Online 后驱动本阶段：

1. Preset 建立公共 exec-sync、exec-transaction 与 user-clone boundary；
2. Setup 选择编译期 payload kind，并运行 selected payload 的可恢复 setup；
3. Enable 只提交 `PayloadPreparePhase.Online`。

Linux `do_sysctl_args()` 等旧 `PayloadPhase.Ready` 的精确语义迁移到
`PayloadPreparePhase.Online`。本阶段不得声明/启用 PID 1 UserAppFlow，也不得替换 active Flow。

## PayloadHandoffPreparePhase

`KernelInitFlow.Enable` 只驱动本阶段。它完成 selected payload entry binding、fresh
`UserAppFlow.Bind/Preset/Setup` 和 replacement precheck。阶段 Online 时：

- `KernelInitFlow` 仍存活且仍是 KernelInitTask active Flow；
- UserBoot 的 fresh UserAppFlow 精确为 Ready，尚未 Enable；
- Hello/Smoke 的内核态 no-return entry 已绑定；
- 所有会失败的路径、容量和交接条件都已预检。

`PayloadHandoffPreparePhase.Online` 是 precommit 边界，默认没有 Linux exact mapping。
它同时是 Kernel.Enable 的应用环境就绪完成边界：checkpoint handler 观察它时 Kernel 必须仍为 Ready，
KernelInitFlow 也仍为 Ready。handler 返回后 KernelInitFlow 与 Kernel 才依次提交 Online。

## KernelInitFlow.CommitPayloadHandoff

只有 Kernel 和 KernelInitFlow 都已 Online 且动态 dispatch guard 成立时才能调用 action。该 action
只能由已提交的 Kernel.Enable 发出；KernelInitFlow.Enable 不得自行发送。UserBoot 必须按固定顺序
提交：

```text
KernelInitFlow.Disable
task active-flow handoff
UserAppFlow.Enable
KernelInitFlow.Cleanup
KernelInitFlow.PayloadHandoffCommitted
```

禁止在 HandoffPrepare 中提前 Disable/Cleanup。成功 exec 后旧 `PayloadPhase.Online` 的 Linux
`run_init_process()` 语义迁移到 `KernelInitFlow.PayloadHandoffCommitted`。

Online 提交和 action 是两个独立原子边界。action 或其 handler 失败时根启动 verdict 为 failed，但
Kernel 保持 Online，不得回滚或重复提交 `KernelOnline`。

Hello 与 Smoke 不替换 Flow：action 保持 KernelInitFlow Online/active，只进入已绑定的内核态
no-return entry。Smoke 的 test Task/Flow 生命周期仍由其自身对象管理。

## Checkpoint

- `PayloadPreparePhase.Started/Prepared/Ready/Online`
- `PayloadHandoffPreparePhase.Started/Prepared/Ready/Online`
- `KernelInitFlow.PayloadHandoffCommitted`

旧 `PayloadPhase.*` checkpoint 全部删除；枚举整体重编号，不保留 tombstone。checkpoint handler 若要
观察 payload 交接后的稳定事实，UserBoot 使用 committed 边界；通用的可恢复准备观察使用
`PayloadPreparePhase.Online`，不得把 precommit checkpoint 当成 replacement 已提交。
