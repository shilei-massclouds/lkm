# Kernel 系统编码映射

本文件把 [`Kernel` model](../../model/systems/kernel.spec) 映射到
`impl/arceos_ex/src/systems/kernel.rs`。Kernel 只保存自己的四态 lifecycle 和顶层 completion；叶
阶段状态与 checkpoint 留在所属 module。

## Preset 与 Setup

架构入口发出 `Kernel.Started`、`BootTask.Online`、`BootInitFlow.Started` 和
`EntryPreludePhase.Started` 后进入 Rust adoption。Kernel.Preset 驱动 BootInitFlow.Preset；
EntryPrelude Online 后 BootInitFlow Prepared，Kernel 随后 Prepared。

Kernel.Setup 驱动 BootInitFlow.Setup。BootInitFlow 直接完成全部 boot、interrupt 与 RestInit 叶阶段
后提交 Ready，Kernel 随后 Ready。系统层不得调用已删除的 Boot/Interrupt wrapper。

## Enable 与跨栈 continuation

```text
Kernel.Enable
  -> BootInitFlow.Enable -> BootInitScheduleHandoff -> BootInitFlow.Online
  -> Scheduler.schedule() real switch commit
  -> CurrentTaskSlot + BootDispatchWindow := KernelInitTask
  -> lossy KernelInitFlow.Preset accepted
  -> kernel_init_entry() verifies actual SP
  -> KernelInitFlow.Preset: PreSmpInit, SmpBringup
  -> KernelInitFlow.Setup: RuntimeCore, Initcall, Rootfs, Finalize, PayloadPrepare
  -> KernelInitFlow.Enable: PayloadHandoffPrepare
  -> KernelInitFlow.Online
  -> KernelInitFlow.CommitPayloadHandoff
```

Scheduler 只接受首次 Flow signal，不得在 BootTask 栈上同步运行 KernelInitFlow 叶阶段。
`kernel_init_entry()` 验证 PID 1 实际 SP 后调用具名 continuation，后者再次验证 Kernel Ready、
BootInitFlow Online、KernelInitTask Online、current/window identity 和 entry count，然后才执行
KernelInitFlow.Preset 的第一个叶阶段。

UserBoot 的 commit action 完成旧 Flow Disable、active handoff、新 Flow Enable 和旧 Flow Cleanup；
Hello/Smoke 保持 KernelInitFlow，不进行 Flow replacement。Kernel.Online 只在 selected payload commit
边界完成后提交。

## 约束

- `systems/kernel.rs` 不保存 leaf 或 wrapper lifecycle。
- KernelInitFlow 每个 transition/action 都即时检查 dispatch guard。
- guard 任一条件失败必须在状态和 checkpoint 修改前 fail-stop。
- KthreaddFlow 只承载服务循环；BootIdleFlow 只直接拥有 BootIdleEntryPhase；UserAppFlow 只承载应用
  continuation/黑盒。
- AP Entry/Callin/OnlineIdle 不得伪装成 KernelInitFlow 子阶段。
