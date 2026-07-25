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

`Config` 与 `Lds` 是初态 Ready 的静态构建输入。完整模型中 Kernel.Setup 必须先接受
`Config.Enable`，再接受依赖 Config.Online 的 `Lds.Enable`，验证二者后建立 kernel-image fact 并提交
Kernel.Ready。真实 Rust 不重放这一完整构造过程；build 和链接产物是进入 canonical Kernel.Enable
发送前边界的外部事实。`systems/kernel.rs` 记录四层规格链，不保存 leaf 或 wrapper lifecycle，也不
伪造设计期 checkpoint。

## Enable 与入口 adoption

架构入口的 `KernelStarted` checkpoint 是 canonical `Kernel.Enable` 接受点。Rust 从真实上游
`Kernel.Enable` 发送前边界采用以下状态：Kernel Ready，Config/Lds Online，Computer、
Riscv64Platform、OpenSBI Online。`BootArgs::new(a0, a1)` 物化只读启动 ABI 输入；入口验证 kernel image、
BootTask 和 a0/a1 后立即提交 Kernel.Online，再启动 BootInitFlow。KernelOnline 必须位于入口验证完成
之后、BootInitFlowStarted 之前。

Kernel.Online 只表示当前 Kernel 实例已经启动并提交内部控制权。Kernel 在 BootInit、首次调度、PID 1
初始化和 payload 交接期间始终 Online；任何后续 module 都不得再次提交或回滚它。

## BootInit 与跨栈 continuation

```text
Kernel.Enable accepts
  -> Kernel.Online
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
  -> KernelInitFlow.CommitPayloadHandoff
```

BootInitFlow 自行串联 Preset/Setup/Enable，Enable 提交 Online 后直接调用 Scheduler；不得回调 Kernel 的
Setup/Enable。Scheduler 是 Task Suspend/Continue 的唯一发送者，不得在 BootTask 栈上提前处理 next
Continue 或运行 KernelInitFlow 叶阶段。`kernel_init_entry()` 验证 PID 1 实际 SP 后调用具名
continuation，后者验证 Kernel Online、BootInitFlow Online、KernelInitTask OnCpu、CurrentTaskSlot
identity 和 entry count，再执行 KernelInitFlow 的第一个叶阶段。

UserBoot 的 commit action 完成旧 Flow Disable、active handoff、新 Flow Enable 和旧 Flow Cleanup；
Hello/Smoke 保持 KernelInitFlow，不进行 Flow replacement。payload 断言必须要求 Kernel 已经 Online，
但 payload commit 不修改 Kernel 状态。

## 约束

- KernelInitFlow 每个 transition/action 都即时检查 parent KernelInitTask 为 OnCpu。
- guard 任一条件失败必须在状态和 checkpoint 修改前 fail-stop。
- KthreaddFlow 只承载服务循环；BootIdleFlow 只直接拥有 BootIdleEntryPhase；UserAppFlow 只承载应用
  continuation/黑盒。
- AP Entry/Callin/OnlineIdle 不得伪装成 KernelInitFlow 子阶段。
