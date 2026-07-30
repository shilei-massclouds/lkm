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
| `_start` Preset head | `InterruptType.Preset` 先以 `csrw sie, zero` 实现全部分路门控关闭，再以 `csrw sip, zero` 实现全部待决信号清空；不得由此推断 `sstatus.SIE` 总门控已关闭；随后把 `__global_pointer$` 装入 `gp/x3`，并用 `.option norelax` 保护该初始化，以建立 `gp_relative_addressing_ready(KernelImage)`；`CurrentCPU.Action::DisableFpuVectorExecution` 再以 `li t0, SR_FS_VS` 和 `csrc CSR_STATUS, t0` 同时关闭 BootCPU 的 FS/VS 执行状态；然后清 BSS，把入口 `a0` 的原值保存到持久交接位置 |
| `preset_until_vm_switch()` | 读取 Kernel Enable 前已发布的 `CpuGroup.cpus[0]`；验证 BootInitFlow 的 CpuRef、PhysicalDirect association、BootTask/TaskRef 与 `BootTask.stack` linker range，然后调用单个 `CurrentTask.BindTaskStack(BootTask, BootTask.stack)` 汇编提交块，原子装载物理 `tp/sp` 并发布首次 task/stack pair；通过 BootInitFlow 的 CpuRef 解析 `CurrentCPU.Setup`，驱动 `KernelAddrSpace.Preset`、`TrapType.Preset` 和 `Vm.Preset`；不得在这里提前驱动 `ExceptionType.Preset` |
| `after_vm_setup()` | `Vm.Setup` 的 Trampoline→Early continuation 返回后先驱动 `TrapType.Setup`；该动作驱动 `ExceptionType.Preset` 及四个异常子类型 Preset，把正式响应汇编函数入口写入 `stvec`，并紧接着以 `csrw sscratch, zero` 标记当前处于内核态。随后才调用单个 `CurrentTask.RefreshTaskStack(BootTask, BootTask.stack)` 汇编提交块，保持 binding identity 并原子刷新虚拟 `tp/sp`，最后驱动 `Soc.Preset` |

`Vm.Setup` 必须按 [`Vm Coding`](../../objects/vm.md) 在同一个 Preset 内通过 per-CPU `ActivateOnCpu`
完成 TrampolineVm 到 EarlyVm 的 Handoff，并通过
`after_vm_setup_continuation()` 回到 BootInitFlow owner。全部 drives 成功后直接检查原入口 Phase
Online invariant 的完整对象事实并提交 `BootInitFlow.Prepared`；随后由 BootInitFlow 自身直接启动
Setup 的第一个叶阶段，不回调 Kernel。

`KernelImage.Setup` 在当前 formal 非 XIP 路径中由入口汇编为 BSS 段清零。清零完成后，BSS 按 Model
作为普通可写内存使用。

`RecordBootCpuHartid` 只要把入口第一个参数的值写入后续阶段可读的持久存储位置或变量即可。
该位置是入口汇编与后续 `smp_setup_processor_id()` 对应阶段之间的交接载体，不是另一个 BootCPU
对象或 hartid owner；本阶段不要求汇编寻址并写入 `Cpu.hartid`。

`DisableFpuVectorExecution` 是 CPU action，不是 BootInitFlow 私有 action，也不属于 CpuGroup。Rust
adoption 必须通过 BootInitFlow 的 CpuRef 解析到 `CpuGroup.cpus[0]`，再验证该 CPU 的 FS/VS 已关闭；
不得把 `sstatus` 检查重新解释为 Flow 状态、CPU 能力缺失或用户态永久禁用。

`TrapType.Preset` 必须把 `stvec` 写为一个汇编函数入口。该入口只包含回跳自身的空无限循环；不得加入
`wfi`、Rust/C 调用、checkpoint、日志、关机请求或其它副作用。写入 `stvec` 的必须是该汇编入口本身，
不能是 Rust wrapper 或数据对象。

`TrapType.Setup` 必须先完成 `ExceptionType.Preset` 及四个子类型的 fallback 准备，再把 `stvec` 重置为
正式响应汇编函数入口的当前虚拟地址。该入口是建立 fresh `TrapFlowType` 的架构响应入口；写入值不能
来自 `satp`、页表、Rust wrapper 或数据对象。所有可失败检查及 entry-context 安装必须在寄存器提交前
完成；提交顺序必须是先写正式 `stvec`、再执行 `csrw sscratch, zero`。写入前仍由 Preset 保护入口响应，
写入后不得返回到临时入口、把 `TrapEntryContext` 地址常驻在 `sscratch`，或把异常服务误标为 Online。

`BindTaskStack` 与 `RefreshTaskStack` 是 CPU 执行上下文的两个 boot-only 原子 Action，不保存
Base/Prepared/Ready 私有状态，不加入公共 `Context`，也不发 lifecycle checkpoint。每次调用在修改
寄存器前验证 BootTask、唯一有效 `TaskRef::BOOT`、active Flow/CpuRef、`stack == BootTask.stack`、当前
CPU controller association、live SATP 和目标 `sp` range。首次 Action 还要求 task/stack pair 均未绑定；
刷新 Action 要求现有 pair 精确相同。Trampoline、controller 缺失、SATP 不匹配、carrier/Flow/stack
不一致或跨 CPU 污染必须记录稳定诊断并沿既有 shutdown 路径 fail-stop，且保持调用前 bindings 与
`tp/sp` 不变。

两个 Action 必须由汇编各自提供一个连续提交块。PhysicalDirect 块等价于 Linux 的
`la tp, init_task`、`la sp, init_thread_union + THREAD_SIZE`、`addi sp, sp, -PT_SIZE_ON_STACK`；EarlyVm
块以当前虚拟地址表示重复装载相同符号。所有可失败检查在提交块前完成，`tp/sp` 写入之间不得调用
C/Rust、插入可失败分支或 checkpoint；寄存器写完后才一次性发布两种 contextual binding。不存在公开
`CurrentStack.BindStack` action 或 Signal。BootTask 的初始禁止抢占由静态初始化器建立，两种 Action
均不读写 preempt count。

`BootTask.OnCpu` 的 `T` 只在 `_start` 观察一次；两个 task-stack Action 都不得推进 BootTask lifecycle
或重复该 marker。两次调用必须解析到同一 `init_task_storage`/`TaskRef::BOOT` carrier 和同一
`BootTask.stack` 属性，期间不建立新的 TaskFlow ownership 或 Stack object。

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
