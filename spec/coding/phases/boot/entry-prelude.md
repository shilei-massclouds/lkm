# EntryPreludePhase Coding

入口前导阶段是 `BootInitFlow` 的直接子阶段，对应 model `EntryPreludePhase` 的完整
`Preset -> Setup -> Enable` 生命周期，实现落点仍为
`impl/arceos_ex/src/phases/boot/entry_prelude.rs`。不迁移现有 Rust module 或物理目录。

## 入口例外与 adoption

`_start` 在任何对象 drive 前依次输出 `Kernel.Started` 的 `R`、`BootTask.Online` 的 `T`、
`BootInitFlow.Started` 的 `O` 和 `EntryPreludePhase.Started` 的 `A`。BootTask marker 观察镜像内
已存在的 Online carrier；其余 Started checkpoint 是各自 Preset 接受事件。汇编随后完成必须发生
在 Rust 前的 CSR/GPR/BSS 操作。

进入 `entry_prelude_rust_entry()` 后，先 adoption Prepare、Kernel、BootTask、BootInitFlow 和
EntryPrelude 的入口边界。EntryPrelude adoption 必须检查自身精确 Base 以及 model 的
Riscv64、SbiSpec、BootArgs、BootHartContext、OpenSBI、Lds、Config 依赖；不得再次输出早期 checkpoint。

## Preset: Base -> Prepared

Preset 横跨三个物理实现段，但仍是一个 model transition：

| 段 | model drives 与实现 |
| --- | --- |
| `_start` head | `InterruptStream.Preset` 清 `sie/sip`；`KernelImage.Preset` 建立 `gp`；EntryPrelude 私有入口动作禁用 FPU/vector；`KernelImage.Setup` 清 BSS；adopt boot hart；建立 `BootTaskEntryBinding` 物理 binding，并静默验证 BootTask Online；adopt init stack |
| `preset_until_vm_switch()` | adoption head 对象事实；驱动 `BootCurrentCPU.Setup -> CpuGroup.Preset -> BootCurrentCPU.Enable`、`EventStream.Preset`、`ExceptionStream.Preset` 和 `Vm.Preset` |
| `after_vm_setup()` | `Vm.Setup` 地址空间 continuation 返回后驱动 `EventStream.Setup`、`BootTaskEntryBinding.Setup` 虚拟 binding、静默验证 BootTask Online、`BootInitStack.Setup` 和 `Soc.Preset` |

`Vm.Setup` 必须在同一个 Preset 内完成 TrampolineVm 到 EarlyVm 的切换，并通过
`after_vm_setup_continuation()` 回到 EntryPrelude owner。所有 drives 成功后检查 Preset 后置对象
事实，提交 Prepared，读回并发出 `EntryPreludePhase.Prepared`，随后按 emits 调用 Setup。

`BootTaskEntryBinding` 的 Base/Prepared/Ready 状态只保存在本 phase 文件的私有静态状态中，不加入
公共 `Context`，也不新增 crate/module facade。物理 adoption 与虚拟切换 helper 在任何修改前必须
一次性验证 binding、BootTask、VM/KernelImage、`TaskRef::BOOT` 和预期 `tp` 地址；提交后失败沿既有
shutdown 路径终止，不能返回可继续执行的半提交状态。binding 本身不发 checkpoint。

`BootTask.Online` 的 `T` 只在 `_start` 观察一次；物理/虚拟 binding 都不得推进 BootTask lifecycle
或重复该 marker。两次 binding 必须解析到同一 `init_task_storage`/`TaskRef::BOOT` carrier，期间不
建立 TaskFlow ownership。

## Setup: Prepared -> Ready

Setup start 检查精确 Prepared。该 transition 没有 drives；确认 Prepared 后置事实仍成立后提交
Ready，读回并发出 `EntryPreludePhase.Ready`，随后按 emits 调用 Enable。

## Enable: Ready -> Online

Enable start 检查精确 Ready，并验证 model Online invariant：Interrupt/Exception/Event streams、
KernelImage、RawDtb、BootTaskEntryBinding、BootTask、BootInitStack、Vm/TrampolineVm/EarlyVm、BootCurrentCPU/BootCPU/
CpuGroup 和 Soc 必须处于 model 规定状态。成功后提交 Online，发出
`EntryPreludePhase.Online`，再返回 `BootInitFlow.Preset` completion continuation。该 continuation
提交 `BootInitFlow.Prepared` 后返回 Kernel.Preset；本阶段不得直接启动 BootInitFlow.Setup 的后续叶阶段或
EntrySuccessorPhase。

## 状态与 checkpoint

`ENTRY_PRELUDE_PHASE_STATE` 持久记录四状态；`is_online()` 只匹配 Online。

| Checkpoint | owner state | Position |
| --- | --- | --- |
| `EntryPreludePhase.Started` | Base | `_start` 的 `A`，Rust 不重复 |
| `EntryPreludePhase.Prepared` | Prepared | `after_vm_setup()` drives 完成并检查后 |
| `EntryPreludePhase.Ready` | Ready | Setup 提交后 |
| `EntryPreludePhase.Online` | Online | Enable 提交后、返回父 continuation 前 |

## Coding Constraints

- RISC-V early alternatives (`apply_early_boot_alternatives`) 在 `Vm.Preset` 中保持显式 deferred；
  不得假装 absent 或 implemented。
- head 汇编只执行 Rust 前不可延迟的架构动作；每个已完成动作由 Rust adoption 进入对象状态，
  不重复对应 checkpoint。
- `init_task_storage` 的定义和 linker-visible 地址继续归 `objects/boot_task.rs`；汇编只引用该符号。
  `BootTask` wrapper 不再暴露入口 lifecycle-driving 方法，只保留 canonical carrier/identity、调度角色
  和受控 `Task` core 访问。
- TrampolineVm 到 EarlyVm 的 translation synchronization 和 continuation identity 必须保持；地址
  空间切换不能制造第二个 Preset 或绕过中间阶段状态。
- `Started` 不是第五种状态，不新增 `Phase.Base` checkpoint。
