# BootInitFlow.Preset Coding

`BootInitFlow.Preset` 直接承载 BP 入口前导编排，不再 lowering 为独立 PhaseObject。model 来源为
[`preset.spec`](../../../model/phases/boot-init/preset.spec) 与
[`phase.spec`](../../../model/phases/boot-init/phase.spec)，实现落点为
`impl/arceos_ex/src/flows/boot_init_flow/preset.rs`。

## 入口例外与 adoption

`_start` 先检查 live `satp=0` 并进入 canonical Kernel.Enable handler。该 handler 在任何
BootInitFlow child drive 前依次接受 Enable、把 BootCPURef 绑定到 BootInitFlow、提交 PhysicalDirect
InitialActivation，然后接受 canonical BootInitFlow.Preset 并输出 `BootInitFlow.Started` 的 `O`。
BootTask.OnCpu marker 只观察镜像内已拥有执行权的同一 carrier。`Kernel.Started` 不得替代 Enable
acceptance 或 BootInitFlow.Started；不得输出或 adoption 任何 BP EntryPrelude lifecycle。

进入 `boot_init_flow_preset_rust_entry()` 后，先 adoption Prepare 和 canonical Kernel.Enable 发送前
边界，验证 BootTask、BootInitFlow Base、Lds、Config、kernel image、BootArgs 与 `a0/a1` 交接事实，
并核对已提交的 Enable acceptance、BootCPURef binding、PhysicalDirect InitialActivation 与 Preset
acceptance；随后 adoption head 已完成的入口动作。Rust 函数的物理执行时点不得重排这些语义边界。
不得要求整组
寄存器已经具有 Kernel 最终值，也不得重放 Human、Computer、Platform、OpenSBI 或 Kernel 的设计期
构造过程。

## Preset: Base -> Prepared

Preset 横跨三个物理实现段，但仍是 BootInitFlow 的一个 model transition：

| 段 | model drives 与实现 |
| --- | --- |
| Kernel entry acceptance | `AcceptEnable` → `AssignCpuRef` → `PhysicalDirect.ActivateOnCpu(InitialActivation)` → 接受 Preset 并记录 `BootInitFlow.Started`；任何 Preset child 尚未启动 |
| `_start` Preset head | `InterruptType.Preset` 先以 `csrw sie, zero` 实现全部分路门控关闭，再以 `csrw sip, zero` 实现全部待决信号清空；不得由此推断 `sstatus.SIE` 总门控已关闭；随后把 `__global_pointer$` 装入 `gp/x3`，并用 `.option norelax` 保护该初始化，以建立 `gp_relative_addressing_ready(KernelImage)`；`CurrentCPU.Action::DisableFpuVectorExecution` 再以 `li t0, SR_FS_VS` 和 `csrc CSR_STATUS, t0` 同时关闭 BootCPU 的 FS/VS 执行状态；然后清 BSS 并 adopt boot hart/stack |
| `preset_until_vm_switch()` | 读取 Kernel Enable 前已发布的 `CpuGroup.cpus[0]`；验证 BootInitFlow 的 CpuRef 与已激活的 PhysicalDirect association，调用 `BindBootTaskEntry(TaskRef::BOOT)` 建立物理 `tp`/首次 preempt 事实；通过 BootInitFlow 的 CpuRef 解析 `CurrentCPU.Setup`，驱动 `KernelAddrSpace.Preset`、`TrapType.Preset`、`ExceptionType.Preset` 和 `Vm.Preset` |
| `after_vm_setup()` | `Vm.Setup` 的 Trampoline→Early continuation 返回后调用同一 `BindBootTaskEntry` 建立虚拟 `tp` 且保持 preempt count，再驱动 `TrapType.Setup`、`BootInitStack.Setup` 和 `Soc.Preset` |

`Vm.Setup` 必须在同一个 Preset 内通过 per-CPU `ActivateOnCpu` 完成 TrampolineVm 到 EarlyVm 的 Handoff，并通过
`after_vm_setup_continuation()` 回到 BootInitFlow owner。全部 drives 成功后直接检查原入口 Phase
Online invariant 的完整对象事实并提交 `BootInitFlow.Prepared`；随后由 BootInitFlow 自身直接启动
Setup 的第一个叶阶段，不回调 Kernel。

`KernelImage.Setup` 的 BSS 清零事实必须在 head 清零循环完成点记录到不属于 BSS 的 handoff storage，
并由 Rust adoption 消费。不得在进入 Rust、消费早先的 BootInitFlow.Started checkpoint 后
重新要求整段 BSS 仍为零：checkpoint handler、诊断缓冲区和其它已启动静态对象可以从这一刻起合法写入
BSS。handoff fact 只证明清零循环已完成，不改变后续 BSS 的正常可写语义。

`DisableFpuVectorExecution` 是 CPU action，不是 BootInitFlow 私有 action，也不属于 CpuGroup。Rust
adoption 必须通过 BootInitFlow 的 CpuRef 解析到 `CpuGroup.cpus[0]`，再验证该 CPU 的 FS/VS 已关闭；
不得把 `sstatus` 检查重新解释为 Flow 状态、CPU 能力缺失或用户态永久禁用。

`BindBootTaskEntry` 是本 Flow 的可重复 action，不保存 Base/Prepared/Ready 私有状态，不加入公共
`Context`，也不发 lifecycle checkpoint。每次调用在修改 `tp` 前验证 BootTask、`TaskRef::BOOT`、当前
CPU controller association 与 live SATP；PhysicalDirect 首次调用且仅首次初始化入口 preempt count，EarlyVm/SwapperVm
调用保持该 count 并绑定同一 carrier 的虚拟地址。Trampoline、controller 缺失、SATP 不匹配或 carrier
不一致必须记录稳定诊断并沿既有 shutdown 路径 fail-stop。该 action 不承担 CurrentTask lifecycle、
accessor 或权威存储职责；BootInitFlow 生效后 `CurrentTask` 必须直接解析为 BootTask。

`BootTask.OnCpu` 的 `T` 只在 `_start` 观察一次；物理/虚拟 bind action 都不得推进 BootTask lifecycle
或重复该 marker。两次调用必须解析到同一 `init_task_storage`/`TaskRef::BOOT` carrier，期间不
建立新的 TaskFlow ownership。

## Checkpoint 与完成边界

入口前导不保存独立四态，也不发 Started/Prepared/Ready/Online checkpoint。唯一正式包装边界是：

| Checkpoint | Position |
| --- | --- |
| `BootInitFlow.Started` | 前三项 Kernel child 已提交、Preset 被接受且首个 Preset child 尚未启动时的 `O`；Kernel 仍为 Ready，BootInitFlow 仍为 Base |
| `BootInitFlow.Prepared` | `after_vm_setup()` 完成全部入口 drives、验证完整入口事实后 |

`Kernel.Online` 不属于入口 Preset：它在后续 `PayloadHandoffPreparePhase.Online` 与
`KernelInitFlow.Online` 之后提交。

`BootInitFlow.Prepared` 的长期 invariant 只保留后续阶段仍稳定的 Flow/BootTask 事实；入口时的一次性
完整条件只在 Preset ensures 与提交前检查中维护。

## Coding Constraints

- RISC-V early alternatives (`apply_early_boot_alternatives`) 在 `Vm.Preset` 中保持显式 deferred。
- head 汇编只执行 Rust 前不可延迟的架构动作；已完成动作由 Rust adoption 进入对象状态，不重复 checkpoint。
- `init_task_storage` 与 linker-visible 地址继续归 `objects/boot_task.rs`；汇编只引用该符号。
- PhysicalDirect→TrampolineVm→EarlyVm 的 per-CPU translation controller、同步事实和 continuation identity 必须保持。
- `Startup` 只是 Preset 的显示名；`Started` 不是第五种状态，不新增另一条 Signal 或 `Flow.Base` checkpoint。
