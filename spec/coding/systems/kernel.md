# Kernel 系统编码映射

本文件把 [`Kernel` model](../../model/systems/kernel.spec) 映射到
`impl/arceos_ex/src/systems/kernel.rs`。Kernel 同时拥有规格采纳、Config/Lds 发布、kernel image 构造、
真实入口和自身四态 lifecycle；叶阶段状态与 checkpoint 留在所属 module。Computer、
Riscv64Platform、OpenSBI 的 module 只记录 metadata。

## 构造规则

Rule IDs (MUST):

- `kernel_system_coding_maps_to_arceos_ex_system_module`
- `kernel_system_coding_records_spec_chain`
- `kernel_system_coding_uses_model_lds_and_config_inputs`
- `kernel_system_coding_keeps_runtime_phases_internal`
- `kernel_system_coding_does_not_synthesize_checkpoints`

`LinuxRiscv64KernelBootSpec` 是来自 Linux 6.12 RISC-V boot contract 的初态 Online、只读外部规格对象；
Kernel.Preset 只采纳 a0/a1、入口 `satp=0` 与 RV64 物理 PMD 装载对齐要求，不提前宣称当前产物已经
满足要求。`Config` 与 `Lds` 是初态 Ready 的静态构建输入。完整模型中 Kernel.Setup 必须先接受
`Config.Enable`，再接受依赖 Config.Online 的 `Lds.Enable`，然后分别验证并建立：Config/Lds 生成
kernel ELF、ELF 生成 kernel boot artifact。两项叶事实共同推导
`kernel_boot_artifact_constructed()`，随后才提交 Kernel.Ready；Ready 不表示 artifact 字节已经装载
到待启动物理地址。`KernelImageFile` 是未来文件对象的保留名称，本轮不创建其类型、实例或生命周期；
运行时 `KernelImage` 只表示固件已加载到内存中的内核映像。

真实 Rust 不重放这一完整构造过程；build、链接产物及 OpenSBI.Enable 交接域保证的物理装载结果是
进入 canonical Kernel.Enable 发送前边界的外部事实。`systems/kernel.rs` 记录四层规格链，不保存外部规格对象、leaf 或 wrapper lifecycle，
也不伪造设计期 checkpoint。

## Enable 与入口 adoption

架构 `_start` 是 canonical `Kernel.Enable` handler 的接受边界；acceptance 是 Enable 响应内的稳定
执行事实，不由 `Kernel.Started` 或其它后继 checkpoint 代替。Rust 从真实上游
`Kernel.Enable` 发送前边界采用以下状态：Kernel Ready，Config/Lds Online，Computer、
Riscv64Platform、OpenSBI Online。`BootArgs::new(a0, a1)` 物化只读启动 ABI 输入；入口验证 a0/a1、
实时 `satp=0`、OpenSBI 选择的 `kernel_load_pa` 与入口位置一致且物理 PMD 对齐、Config/Lds 与
BootTask 后只接受 Enable，不提交
Kernel.Online。入口 `_start` 必须在 Enable acceptance 和任何 BootInitFlow child action 之前读取 live
`satp` 并对非零值 fail-stop；Rust adoption 在 EarlyVm 切换前再次复核。不得用 OpenSBI 的历史 handoff
记录替代任一次实时检查。随后 BootInitFlow、首次调度和 KernelInitFlow 都在 Kernel Ready/Enable
执行上下文中运行。

Enable handler 必须依次完成 `AcceptEnable`、把解析到的 BootCPURef 绑定到 BootInitFlow，以及由
`PhysicalDirect.ActivateOnCpu(BootCPURef)` 从 absent association 原子提交 InitialActivation。只有三项
均成功后才能接受 BootInitFlow.Preset，并在第一个 child action 前输出 `BootInitFlow.Started`；Rust
事后 adoption 可以核对早期 handoff storage，但不得改变这条语义和 checkpoint 顺序。`KernelAddrSpace`、`Vm`、`Soc`、
`CpuGroup` 是 Kernel 的平级直接子对象；Soc/CpuGroup 之间的启动依赖不表达所有权。

这些已在 MMU-off head 完成的接受事实通过入口私有 `.head.handoff` receipt 跨越 BSS/Rust 边界；Rust
只能在整体校验 receipt 后按 `AcceptEnable -> AssignCpuRef -> PhysicalDirect -> Preset` adoption。
`AcceptEnable` 不得反向依赖 BootInitFlow 已有 CpuRef；receipt 缺失、重复或乱序必须在 Flow CpuRef、
controller 或 Preset 状态发生部分提交前 fail-stop。

OpenSBI 不负责保证 `sie/sip` 已清零。`BootInitFlow.Preset` 的第一个被驱动叶迁移是
`InterruptType.Preset`；入口汇编必须先执行 `csrw sie, zero` 映射全部中断分路门控关闭，再执行
`csrw sip, zero` 映射一次待决清除写完成；硬件驱动的 live `sip` 可以随后再次置位，Rust adoption 不得
读取、重写或要求它为零。该边界不得读写或推断 `sstatus.SIE`，也不得建立
handler、fallback、正式分派框架或 `interrupt_concurrency_closed()`。这发生在 Kernel.Enable 已接受
之后、其余入口前导动作之前。

`PayloadHandoffPreparePhase.Online` 表示应用环境的全部可逆准备已经完成。KernelInitFlow 随后先提交
Online，再由 `systems::kernel` 验证完整下层闭包并原子提交 Kernel.Online；`KernelOnline` 必须严格位于
`PayloadHandoffPreparePhase.Online` 之后、selected payload commit 之前。Kernel.Online 表示应用运行环境
已经准备就绪并等待应用启动，而不是刚进入内核。Online 提交后的 payload handoff 失败不得回滚
Kernel；根启动结果仍按失败处理。

## BootInit 与跨栈 continuation

```text
Kernel.Enable accepts
  -> BootInitFlow.AssignCpuRef
  -> PhysicalDirect.ActivateOnCpu(InitialActivation)
  -> BootInitFlow.Preset -> BootInitFlow.Setup -> BootInitFlow.Enable
  -> BootInitScheduleHandoff -> BootInitFlow.Online
  -> Scheduler.schedule(): BootTask.Suspend, save, current/context commit
  -> physical switch to KernelInitTask
  -> KernelInitTask.Continue
  -> kernel_init_entry() verifies actual SP and directly starts KernelInitFlow.Preset
  -> KernelInitFlow.Preset: PreSmpInit, SmpBringup
  -> KernelInitFlow.Setup: RuntimeCore, Initcall, Rootfs, Finalize, PayloadPrepare
  -> KernelInitFlow.Enable: PayloadHandoffPrepare
  -> KernelInitFlow.Online
  -> Kernel.Online
  -> Kernel emits KernelInitFlow.CommitPayloadHandoff
```

物理 Rust continuation 可以由 BootInitFlow 串联其叶阶段并在 Flow Online 后直接进入 Scheduler，
但这些边界在逻辑上都由同一个 Kernel.Enable 驱动，不得另建 pending/continuation lifecycle 状态。
Scheduler 是 Task Suspend/Continue 的唯一发送者，不得在 BootTask 栈上提前处理 next
Continue 或运行 KernelInitFlow 叶阶段。`kernel_init_entry()` 验证 PID 1 实际 SP 后调用具名
continuation，后者验证 Kernel Ready 且 Enable 已接受、BootInitFlow Online、KernelInitTask OnCpu、CurrentTask
identity 和 entry count，再执行 KernelInitFlow 的第一个叶阶段。

UserBoot 的 commit action 完成旧 Flow Disable、active handoff、新 Flow Enable 和旧 Flow Cleanup；
Hello/Smoke 保持 KernelInitFlow，不进行 Flow replacement。准备期断言必须要求 Kernel Ready 且 Enable
已接受；commit action 必须要求 Kernel 已经 Online，且不得修改 Kernel 状态。唯一 action sender 是
已经完成 Online 提交的 Kernel.Enable，KernelInitFlow.Enable 不得自行发送。

## 约束

- KernelInitFlow 每个 transition/action 都即时检查 parent KernelInitTask 为 OnCpu。
- guard 任一条件失败必须在状态和 checkpoint 修改前 fail-stop。
- KthreaddFlow 只承载服务循环；BootIdleFlow 只直接拥有 BootIdleEntryPhase；UserAppFlow 只承载应用
  continuation/黑盒。
- AP Entry/Callin/OnlineIdle 不得伪装成 KernelInitFlow 子阶段。
