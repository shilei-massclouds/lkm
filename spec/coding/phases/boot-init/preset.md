# BootInitFlow.Preset Coding

`BootInitFlow.Preset` 直接承载 BP 入口前导编排，不再 lowering 为独立 PhaseObject。model 来源为
[`preset.spec`](../../model/phases/boot-init/preset.spec) 与
[`phase.spec`](../../model/phases/boot-init/phase.spec)，实现落点为
`impl/arceos_ex/src/flows/boot_init_flow/preset.rs`。

## 入口例外与 adoption

`_start` 在任何对象 drive 前依次输出 `Kernel.Started` 的 `R`、`BootTask.OnCpu` 的 `T` 和
`BootInitFlow.Started` 的 `O`。BootTask marker 观察镜像内已拥有执行权的 OnCpu carrier；
BootInitFlow marker 表示 Preset 已接受。不得输出或 adoption 任何 BP EntryPrelude lifecycle。

进入 `boot_init_flow_preset_rust_entry()` 后，先 adoption Prepare、Kernel、BootTask 和 BootInitFlow
入口边界。adoption 必须检查 BootInitFlow 精确 Base 以及 model 的 Riscv64、SbiSpec、BootArgs、
OpenSBI、Lds、Config 依赖、只表示属性可访问的 `BootCpuRegisters.Online` 和 `a0/a1` 精确交接事实；
不得要求整组寄存器已经具有 Kernel 最终值，也不得再次输出早期 checkpoint。

## Preset: Base -> Prepared

Preset 横跨三个物理实现段，但仍是 BootInitFlow 的一个 model transition：

| 段 | model drives 与实现 |
| --- | --- |
| `_start` head | `InterruptStream.Preset` 清 `sie/sip`；`KernelImage.Preset` 建立 `gp`；Preset 私有入口动作禁用 FPU/vector；`KernelImage.Setup` 清 BSS；adopt boot hart；建立 `BootTaskEntryBinding` 物理 binding，并静默验证 BootTask OnCpu；adopt init stack |
| `preset_until_vm_switch()` | adoption head 对象事实；驱动 `BootCurrentCPU.Setup -> CpuGroup.Preset -> BootCurrentCPU.Enable`、`EventStream.Preset`、`ExceptionStream.Preset` 和 `Vm.Preset` |
| `after_vm_setup()` | `Vm.Setup` 地址空间 continuation 返回后驱动 `EventStream.Setup`、`BootTaskEntryBinding.Setup` 虚拟 binding、静默验证 BootTask OnCpu、`BootInitStack.Setup` 和 `Soc.Preset` |

`Vm.Setup` 必须在同一个 Preset 内完成 TrampolineVm 到 EarlyVm 的切换，并通过
`after_vm_setup_continuation()` 回到 BootInitFlow owner。全部 drives 成功后直接检查原入口 Phase
Online invariant 的完整对象事实并提交 `BootInitFlow.Prepared`；随后返回 Kernel.Preset completion。
Preset 不启动 BootInitFlow.Setup 的后续叶阶段。

`BootTaskEntryBinding` 的 Base/Prepared/Ready 状态只保存在本 Flow 子模块的私有静态状态中，不加入
公共 `Context`。物理 adoption 与虚拟切换 helper 在任何修改前必须一次性验证 binding、BootTask、
VM/KernelImage、`TaskRef::BOOT` 和预期 `tp` 地址；提交后失败沿既有 shutdown 路径终止，不能返回可继续
执行的半提交状态。binding 本身不发 checkpoint。

`BootTask.OnCpu` 的 `T` 只在 `_start` 观察一次；物理/虚拟 binding 都不得推进 BootTask lifecycle
或重复该 marker。两次 binding 必须解析到同一 `init_task_storage`/`TaskRef::BOOT` carrier，期间不
建立新的 TaskFlow ownership。

## Checkpoint 与完成边界

入口前导不保存独立四态，也不发 Started/Prepared/Ready/Online checkpoint。唯一正式包装边界是：

| Checkpoint | Position |
| --- | --- |
| `BootInitFlow.Started` | `_start` 的 `O`，Rust 不重复 |
| `BootInitFlow.Prepared` | `after_vm_setup()` 完成全部入口 drives、验证完整入口事实后 |

`BootInitFlow.Prepared` 的长期 invariant 只保留后续阶段仍稳定的 Flow/BootTask 事实；入口时的一次性
完整条件只在 Preset ensures 与提交前检查中维护。

## Coding Constraints

- RISC-V early alternatives (`apply_early_boot_alternatives`) 在 `Vm.Preset` 中保持显式 deferred。
- head 汇编只执行 Rust 前不可延迟的架构动作；已完成动作由 Rust adoption 进入对象状态，不重复 checkpoint。
- `init_task_storage` 与 linker-visible 地址继续归 `objects/boot_task.rs`；汇编只引用该符号。
- TrampolineVm 到 EarlyVm 的 translation synchronization 和 continuation identity 必须保持。
- `Started` 不是第五种状态，不新增 `Flow.Base` checkpoint。
