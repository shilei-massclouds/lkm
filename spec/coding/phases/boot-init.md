# BootInitFlow 编码指引

`BootInitFlow` 是 `BootTask.initial_flow` 指向的静态 TaskFlow。model 来源为
[`phase.spec`](../../model/phases/boot-init/phase.spec)，实现落点为
`impl/arceos_ex/src/phases/boot_init/mod.rs`。Boot、interrupt 目录只保留叶阶段 namespace；实现不得
恢复 `BootPhase`、`InterruptPhase` 或任何平行 wrapper lifecycle。

实现必须复用统一 `TaskFlow` core 和固定 `TaskFlowRef::BOOT_INIT`。不得另存 Flow state 或 guard
bool；每个 transition 都即时检查 parent `BootTask` Online 且 `BootDispatchWindow` 当前指向它。

## Transition 映射

| Transition | 直接 drives | completion |
| --- | --- | --- |
| Preset: Base -> Prepared | `EntryPreludePhase.Preset` | EntryPrelude Online 后提交 `BootInitFlow.Prepared` |
| Setup: Prepared -> Ready | 依次驱动 `EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase`、`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase`、`BootInitRestInitPhase` 的 Preset | RestInit Online 后提交 `BootInitFlow.Ready`；此时两个新 Task 已完成 Preset/Setup/Enable，initial-flow signal 均已在 BootTask window 下丢弃 |
| Enable: Ready -> Online | 只驱动 `BootInitScheduleHandoffPhase.Preset` | 该阶段建立 `BootIdleFlow` owner/active binding 并完成可逆切换预检；随后提交 `BootInitFlow.Online`，再进入真实 schedule |

每个叶子 Online 只能返回 BootInitFlow 当前 transition 的具名 continuation；父 continuation 必须检查
刚完成的叶子精确 Online，才能启动下一个叶子。

```text
_start -> BootTask.Online -> BootInitFlow.Preset
  -> EntryPreludePhase -> BootInitFlow.Prepared
  -> EntrySuccessor -> CorePrepare -> MmCoreInit -> SchedInit
  -> IrqTimeInit -> LocalIrqEnable -> IrqOpenPrepare -> ProcessPrepare
  -> BootInitRestInit -> BootInitFlow.Ready
  -> BootInitScheduleHandoff -> BootInitFlow.Online
  -> real BootTask-to-KernelInitTask switch
```

## 首次 dispatch

Task Enable 只负责 `wake_up_new_task` 等价动作并发出 lossy initial-flow Preset。创建 PID 1 和
kthreadd 时窗口仍属于 BootTask，所以两个信号必须返回 `Discarded`，不得排队或预执行 Flow。

首次真实 dispatch 必须在同一 commit boundary 更新 CPU-local current Task 与
`BootDispatchWindow`，然后再次向 `KernelInitFlow` 发出 lossy Preset。runtime lowering 只能接受该
信号；真正的 KernelInitFlow 叶阶段代码由随后进入的 `kernel_init_entry()` 在验证 16 KiB vmalloc
stack 后执行，不能在 BootTask 的 `schedule()` 调用栈上执行。

`BootIdleEntryPhase` 是 `BootIdleFlow` 的直接子阶段，不属于 BootInitFlow。BootTask 将来从首次
schedule 返回时才驱动它并进入不返回的 idle loop。

## Checkpoint

`BootInitFlow.Started/Prepared/Ready/Online` 四个边界保留。`Prepared` 位于 EntryPrelude Online 后，
`Ready` 位于 RestInit Online 后，`Online` 位于 ScheduleHandoff Online 后且真实 switch 前。已删除的
Boot/Interrupt wrapper checkpoint 不得作为兼容 marker 保留。
