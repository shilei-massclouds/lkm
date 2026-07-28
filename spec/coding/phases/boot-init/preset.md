# BootInitFlow.Preset Coding

`BootInitFlow.Preset` 直接承载 BP 入口前导编排，不再 lowering 为独立 PhaseObject。model 来源为
[`preset.spec`](../../../model/phases/boot-init/preset.spec) 与
[`phase.spec`](../../../model/phases/boot-init/phase.spec)，实现落点为
`impl/arceos_ex/src/flows/boot_init_flow/preset.rs`。

## 入口例外与 adoption

`_start` 在任何对象 drive 前依次输出 `Kernel.Started` 的 `R` 和 `BootTask.OnCpu` 的 `T`。BootTask
marker 观察镜像内已拥有执行权的 OnCpu carrier。Rust 入口完成 Prepare 与 Kernel.Enable adoption 后，
在 Kernel 仍为 Ready 时输出 `BootInitFlow.Started` 的 `O`；后者表示 Kernel 驱动的
`BootInitFlow.Preset` 已接受。不得输出或 adoption 任何 BP EntryPrelude lifecycle。

进入 `boot_init_flow_preset_rust_entry()` 后，先 adoption Prepare 和 canonical Kernel.Enable 发送前
边界，验证 BootTask、BootInitFlow Base、Lds、Config、kernel image、BootArgs 与 `a0/a1` 交接事实，
只记录 Enable 已接受；随后接受 BootInitFlow.Preset 并 adoption head 已完成的入口动作。不得要求整组
寄存器已经具有 Kernel 最终值，也不得重放 Human、Computer、Platform、OpenSBI 或 Kernel 的设计期
构造过程。

## Preset: Base -> Prepared

Preset 横跨三个物理实现段，但仍是 BootInitFlow 的一个 model transition：

| 段 | model drives 与实现 |
| --- | --- |
| `_start` head | `InterruptStream.Preset` 清 `sie/sip`；`KernelImage.Preset` 建立 `gp`；Preset 私有入口动作禁用 FPU/vector；`KernelImage.Setup` 清 BSS；adopt boot hart；建立 `BootTaskEntryBinding` 物理 binding，并静默验证 BootTask OnCpu；adopt init stack |
| `preset_until_vm_switch()` | 读取 Kernel Enable 前已发布的 `CpuGroup.cpus[0]`；通过 BootInitFlow 的 CpuRef 解析 `CurrentCPU.Setup`，并驱动 `EventStream.Preset`、`ExceptionStream.Preset` 和 `Vm.Preset` |
| `after_vm_setup()` | `Vm.Setup` 地址空间 continuation 返回后驱动 `EventStream.Setup`、`BootTaskEntryBinding.Setup` 虚拟 binding、静默验证 BootTask OnCpu、`BootInitStack.Setup` 和 `Soc.Preset` |

`Vm.Setup` 必须在同一个 Preset 内完成 TrampolineVm 到 EarlyVm 的切换，并通过
`after_vm_setup_continuation()` 回到 BootInitFlow owner。全部 drives 成功后直接检查原入口 Phase
Online invariant 的完整对象事实并提交 `BootInitFlow.Prepared`；随后由 BootInitFlow 自身直接启动
Setup 的第一个叶阶段，不回调 Kernel。

`KernelImage.Setup` 的 BSS 清零事实必须在 head 清零循环完成点记录到不属于 BSS 的 handoff storage，
并由 Rust adoption 消费。不得在进入 Rust、发出 BootInitFlow.Started checkpoint 后
重新要求整段 BSS 仍为零：checkpoint handler、诊断缓冲区和其它已启动静态对象可以从这一刻起合法写入
BSS。handoff fact 只证明清零循环已完成，不改变后续 BSS 的正常可写语义。

`BootTaskEntryBinding` 的 Base/Prepared/Ready 状态只保存在本 Flow 子模块的私有静态状态中，不加入
公共 `Context`。物理 adoption 与虚拟切换 helper 在任何修改前必须一次性验证 binding、BootTask、
VM/KernelImage、`TaskRef::BOOT` 和预期 `tp` 地址；提交后失败沿既有 shutdown 路径终止，不能返回可继续
执行的半提交状态。binding 本身不发 checkpoint，只协调入口架构绑定与物理到虚拟身份迁移，不承担
CurrentTask lifecycle、accessor 或权威存储职责；BootInitFlow 生效后 `CurrentTask` 必须直接解析为
BootTask。

`BootTask.OnCpu` 的 `T` 只在 `_start` 观察一次；物理/虚拟 binding 都不得推进 BootTask lifecycle
或重复该 marker。两次 binding 必须解析到同一 `init_task_storage`/`TaskRef::BOOT` carrier，期间不
建立新的 TaskFlow ownership。

## Checkpoint 与完成边界

入口前导不保存独立四态，也不发 Started/Prepared/Ready/Online checkpoint。唯一正式包装边界是：

| Checkpoint | Position |
| --- | --- |
| `BootInitFlow.Started` | Rust 接受 BootInitFlow.Preset 时的 `O`；此时 Kernel 仍为 Ready |
| `BootInitFlow.Prepared` | `after_vm_setup()` 完成全部入口 drives、验证完整入口事实后 |

`Kernel.Online` 不属于入口 Preset：它在后续 `PayloadHandoffPreparePhase.Online` 与
`KernelInitFlow.Online` 之后提交。

`BootInitFlow.Prepared` 的长期 invariant 只保留后续阶段仍稳定的 Flow/BootTask 事实；入口时的一次性
完整条件只在 Preset ensures 与提交前检查中维护。

## Coding Constraints

- RISC-V early alternatives (`apply_early_boot_alternatives`) 在 `Vm.Preset` 中保持显式 deferred。
- head 汇编只执行 Rust 前不可延迟的架构动作；已完成动作由 Rust adoption 进入对象状态，不重复 checkpoint。
- `init_task_storage` 与 linker-visible 地址继续归 `objects/boot_task.rs`；汇编只引用该符号。
- TrampolineVm 到 EarlyVm 的 translation synchronization 和 continuation identity 必须保持。
- `Started` 不是第五种状态，不新增 `Flow.Base` checkpoint。
