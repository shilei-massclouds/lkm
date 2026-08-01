# BootInitFlow 编码指引

`BootInitFlow` 是 `BootTask.initial_flow` 指向的静态 TaskFlow。model 来源为
[`phase.spec`](../../model/phases/boot-init/phase.spec)，实现落点为
`impl/arceos_ex/src/flows/boot_init_flow/mod.rs`。Boot、interrupt 目录只保留 Setup 叶阶段 namespace；实现不得
恢复 `BootPhase`、`InterruptPhase` 或任何平行 wrapper lifecycle。

实现必须复用统一 `TaskFlow` core 和固定 `TaskFlowRef::BOOT_INIT`。不得另存 Flow state 或 guard
bool；每个 transition 都即时检查 parent `BootTask` 为 OnCpu。

## Transition 映射

| Transition | 直接 drives | completion |
| --- | --- | --- |
| Preset: Base -> Prepared | 直接按入口顺序驱动 `InterruptType` 至 `Soc` 的具体对象 | 完整入口事实成立后提交 `BootInitFlow.Prepared`，再由同对象 `emits BootInitFlow.Setup`；不建立入口 wrapper lifecycle |
| Setup: Prepared -> Ready | 依次驱动 `EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase`、`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase`、`BootInitRestInitPhase` 的 Preset | RestInit Online 后提交 `BootInitFlow.Ready`，再由同对象 `emits BootInitFlow.Enable`；此时两个新 Task 已完成 Preset/Setup/Enable，initial Flow 仍为 Base |
| Enable: Ready -> Online | 只驱动 `BootInitScheduleHandoffPhase.Preset` | 该阶段建立 `BootIdleFlow` owner/active binding 并完成可逆切换预检；随后提交 `BootInitFlow.Online`，再进入真实 schedule |

这些 transition 是 Kernel.Enable 的下层过程细化。Kernel 只同步驱动 Preset；Setup 与 Enable 分别是
Preset 与 Setup 成功提交后的同对象 completion event，不是 Kernel 的第二、第三次 drive。Preset 只能在同一 Enable handler 已依次完成
acceptance、BootCPURef binding 和 PhysicalDirect InitialActivation 后接受；整个 BootInitFlow 生命周期
执行期间 Kernel 保持 Ready。每个叶子 Online 只能返回 BootInitFlow 当前 transition 的具名 continuation；父 continuation 必须检查
刚完成的叶子精确 Online，才能启动下一个叶子。

```text
OpenSBI -> Kernel.Enable accepts -> AssignCpuRef -> PhysicalDirect InitialActivation
  -> Kernel drives BootInitFlow.Preset (BootTask.OnCpu)
  -> entry objects -> BootInitFlow.Prepared -> emits BootInitFlow.Setup
  -> EntrySuccessor -> CorePrepare -> MmCoreInit -> SchedInit
  -> IrqTimeInit -> LocalIrqEnable -> IrqOpenPrepare -> ProcessPrepare
  -> BootInitRestInit -> BootInitFlow.Ready -> emits BootInitFlow.Enable
  -> BootInitScheduleHandoff -> BootInitFlow.Online
  -> active BootIdleFlow emits Cpu0.Scheduler.Schedule (before-send endpoint)
  -> Scheduler may perform the real BootTask-to-KernelInitTask switch
  -> KernelInitFlow leaves while Kernel.Ready
```

## 首次真实调度

Task Enable 只负责 `wake_up_new_task` 等价动作；创建 PID 1 和 kthreadd 时不发送 initial-flow Signal，
不得预执行任何 Flow。

首次真实调度由已是 BootTask active Flow 的 `BootIdleFlow` 于 BootInitFlow.Online 后 emits 无实参
Schedule；Scheduler 从 sender Flow/CpuRef 和 CPU-local current binding 解析 prev。调度先 PreparePrev，再 PickNextTask；
只在 next != prev 时按保存 BootTask 上下文、Suspend、恢复 next 上下文并提交 CPU-local current
Task/context、next-stack finish 的顺序完成物理切换；随后在 `kernel_init_entry()` 中处理 KernelInitTask Continue 并严格向
`KernelInitFlow` 发出 Startup。真正的 KernelInitFlow 叶阶段代码在验证 16 KiB vmalloc
stack 后执行，不能在 BootTask 的 `schedule()` 调用栈上执行。

`BootIdleEntryPhase` 是 `BootIdleFlow` 的直接子阶段，不属于 BootInitFlow。BootTask 将来从首次
schedule 返回时才驱动它并进入不返回的 idle loop。

## Checkpoint

`BootInitFlow.Started/Prepared/Ready/Online` 四个边界保留。`Startup` 只显示 canonical Preset，不是另一个
signal；`Started` 只记录 Preset 已接受，必须位于 acceptance/binding/InitialActivation 之后和首个 child
action 之前，且不推进 Base。`Prepared` 位于全部入口对象动作完成后，
`Ready` 位于 RestInit Online 后，`Online` 位于 ScheduleHandoff Online 后且真实 switch 前。已删除的
BP EntryPrelude、Boot 和 Interrupt wrapper checkpoint 不得作为兼容 marker 保留。

## 实现布局

- `flows/boot_init_flow/mod.rs`：`BootInitFlow` 类型、Preset/Setup/Enable 与全部父 continuation。
- `flows/boot_init_flow/preset.rs`：入口汇编 adoption、具体对象 drive 与 Preset 完成验证。
- `flows/boot_init_flow/rest_init.rs`：`BootInitRestInitPhase`。
- `flows/boot_init_flow/schedule_handoff.rs`：`BootInitScheduleHandoffPhase`。

跨子模块只暴露父 continuation 与完成验证所需的 `pub(super)` 事实；不得保留
`phases::boot_init` 旧路径 re-export。
