# tools2 启动语义校准与旧工具退役章程

## 目标与边界

tools2 的完整主模型闭包成功，只能证明当前工具能够按当前形式规格把根 Signal 推导到结束，且途中
没有已被工具识别的拒绝、失败或预算截断。它不能单独证明模型已经覆盖 Linux 启动的全部因果关系，
也不能证明 Context、Lock、执行主体迁移、失败传播和运行期实例身份等语义已经表达正确。工具只会
检查已经进入其输入和解释规则的义务；遗漏的对象、guard、child Signal、提交边界或失败分支不会因为
闭包成功而自动成为正确。

因此，完整闭包是启动语义校准的必要回归门槛，不是语义完整性的结论。校准必须从 Kernel 启动流程
的真实前序开始，按执行顺序从前到后推进，并在每个大边界内从 Signal 传播的粗粒度轮廓逐步下钻到
handler、guard、状态提交和事实变化。本章程只确定这条校准与退役路线，不在本轮修改 model、coding、
compose、tools2 实现、旧工具实现或测试基线。

本章程中的六组阶段只是审查和交付批次，不新增 wrapper、PhaseObject、lifecycle 或 checkpoint，也不
改变现有启动拓扑。后续每一组都必须单独按 charter-first 闭合：先确认或修订 charter，再依次审查并
关闭 model、coding、适用的 compose、实现和 testing；不得以本章程直接代替各对象和阶段的正式规格。

## 三种验收载体

动画、文本和结构化 JSON 共同验收同一次确定推导，但承担不同责任；任一种通过都不能替代另外两种。

- 动画负责让人沿全局 Signal 顺序观察系统出现、父子结构、source/target 移动、状态变化和失败位置。
  它适合发现顺序、层级、执行主体和结构直觉上的断裂，但播放器不重新 derive，也不从画面推断语义。
- 文本负责提供紧凑、稳定、可逐行评审和 diff 的因果路径。它用于核对启动主干、相邻 Signal、实际
  Transition 前后状态以及异常 outcome；详细文本还承担快速追踪 guard、payload、snapshot delta 和
  failure chain 的责任。
- 结构化 JSON 是精确审计载体。model/derive/view/snapshot 中的 canonical identity、handler、delivery、
  cause、before/after snapshot、事实、引用、事件、outcome 和 reason 必须由它核对。文本或动画中的
  简化、隐藏和布局不得反向成为语义依据。

三者必须来自相同 model fingerprint 和同一次推导身份。发现差异时先回到结构化 JSON 定位是输入、
derive、view 还是展示问题，再用文本复核因果链、用动画复核全局顺序和结构；不得凭单一截图、单一
摘要行或“看起来完整”的动画判定语义成立。

## 总体校准方法

校准按 Kernel 启动实际发生顺序执行，而不是按文件目录、对象名称或工具实现模块倒推：

1. 从默认 Human 外部编排和模型初态开始，保留 Human 对 Computer 的 Preset/Setup/Enable 编排、
   Computer 对三个直接子 System 的 Preset/Setup 编排以及
   `Computer -> Riscv64Platform -> OpenSBI -> Kernel` 的真实上游链。
2. 在待校准组首个 canonical Signal 的发送动作之前停止，导出带 boundary provenance 的稳定
   snapshot。snapshot 必须由真实上游到达，不能由旧工具、手写 state/fact、隐式 emitter 回溯或
   目标 handler 的预执行合成。
3. 以该 snapshot 作为本组唯一入口基线，先审查本组 Signal 主干和 canonical boundary，再逐个展开
   handler 的因果账本。需要继续下钻时，在本组内部下一个真实 Signal 的发送前再次截断，而不是跳过
   上游条件直接调用深层对象。
4. 将每个边界与 charter 意图、Linux 6.12 路径及真实 checkpoint 观察对齐；没有足够观察时先增加
   长期有用的 checkpoint 或诊断，再判定问题归属。
5. 修订必须遵守 authority 顺序。一个校准批次完成 charter、model、coding、适用 compose、实现、
   focused validation 和根回归后，才进入下一批次；临时跨层不一致不得提交。

canonical boundary snapshot 表示“该 canonical Signal 发送之前的最后稳定状态”。它必须记录规范化
Signal identity、source、target、model fingerprint 和 boundary provenance，并可重复从真实上游生成
相同 canonical bytes。`Startup` 只可作为外部显示别名，snapshot 和结构化产物仍使用 `Preset`。阶段
入口 snapshot 不是可恢复 continuation，也不允许把未提交 ancestor 或尚未处理的 FIFO Signal 当作
已经完成。

便利入口以显式 `-t` 自动选择该 Signal 的已提交 canonical snapshot 时，新根 Signal 的默认
sender 必须恢复为 snapshot `provenance.boundary.source`；不得回退到 `Human`，也不得回溯
或重放产生该 snapshot 的上游 Signal。调用者显式给出 `--source` 时才覆盖该 provenance；
省略 `-t` 的默认外部编排和显式 `-s` 非自动场景仍使用原有 sender 规则。canonical
snapshot 缺失、损坏或其 boundary source 不可用时必须在 derive 前拒绝，不能猜测 sender。

## 六组启动校准批次

下表中的“入口 snapshot”是每组必须建立和复核的 canonical boundary snapshot。名称用于标识边界，
不要求本轮新增同名仓库文件；是否把某个 snapshot 提交为 scenario，必须在该组后续 charter-first
工作中另行决定。当前提交的 `Kernel.Enable.snapshot.json` 也必须由真实上游重建和复核，不能仅因
它已经存在而免验。

| 组 | 启动范围 | canonical 入口 snapshot | 本组结束边界 |
| --- | --- | --- | --- |
| 1. 上游构造与交接 | Computer 顺序驱动三个直接子 System 的 Preset/Setup、建立 assembly，以及平台和 OpenSBI 的 FIFO 启动交接 | `Kernel.Enable` 发送前 | `Kernel.Enable` 被接受，Kernel 的启动前提和 a0/a1 交接得到核对 |
| 2. Kernel 与 BootInit 入口 | `Kernel.Enable` 在 Ready 内同步驱动 `BootInitFlow.Preset`，以及 BootInit 最早入口对象建立 | `Kernel.Enable` 发送前 | `BootInitFlow.Setup` 发送前，Kernel 保持 Ready 且 BootInitFlow 已到 Prepared |
| 3. BootInit 引导、中断与进程准备 | `EntrySuccessorPhase`、`CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase`、`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase` | `BootInitFlow.Setup` 发送前 | `BootInitRestInitPhase.Preset` 发送前，前述叶子均已按顺序完成 |
| 4. rest-init 与首次调度切换 | PID 1/kthreadd 建立、completion、BootIdleFlow 预检与 active binding、BootTask 到 KernelInitTask 的真实 switch | `BootInitRestInitPhase.Preset` 发送前 | `KernelInitFlow.Preset` 发送前，PID 1 已真实 OnCpu 且执行权完成迁移 |
| 5. PID 1 内核初始化 | `PreSmpInitPhase`、`SmpBringupPhase`、`RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、`FinalizePhase` 和 `PayloadPreparePhase` | `KernelInitFlow.Preset` 发送前 | `KernelInitFlow.Enable` 发送前，KernelInitFlow 已到 Ready |
| 6. payload 预提交与交接 | `PayloadHandoffPreparePhase`、KernelInitFlow Online、Kernel Online、由 Kernel 发出的 `CommitPayloadHandoff`，以及 Hello/Smoke/UserBoot 各自的 no-return 或 replacement 边界 | `KernelInitFlow.Enable` 发送前 | `KernelInitFlow.PayloadHandoffCommitted` 或对应的确定失败边界；handoff 失败时 Kernel 保持 Online、根结果 failed |

每组先验收粗粒度主干，再按 canonical Signal 顺序细分；表中的结束边界不是允许忽略组内子阶段的
聚合断言。尤其第 5 组列出的各叶子都必须拥有自己的因果账本和 Linux/checkpoint 对照，只是共享同一
PID 1 入口基线。若组内发现问题，只缩小到第一个不一致的边界，不从症状直接修改后续 handler。

## 第 1 组确定语义与因果账本

第 1 组只覆盖 Human/Computer 构造和 `Riscv64Platform -> OpenSBI -> Kernel` 交接，不展开
`BootInitFlow.Preset` 的任何内部动作。canonical `Kernel.Enable` 发送前推导必须从模型初态和唯一
`external Human` 编排到达；不能以显式 `-t Kernel.Enable` 的 Human 根 source 代替真实发送者证据。
本组确定的前 15 个 Signal 如下，编号是该 canonical 推导的稳定 identity：

| ID | source -> target.Signal | delivery / cause | handler 与提交边界 |
| --- | --- | --- | --- |
| `sig-0001` | `Human -> Computer.Preset` | drives / none | `Computer.Transition::Preset@Base`；等待 0002–0004 后提交 Computer Prepared |
| `sig-0002` | `Computer -> Riscv64Platform.Preset` | drives / 0001 | 平台 Base -> Prepared，建立 ISA/平台规格事实 |
| `sig-0003` | `Computer -> OpenSBI.Preset` | drives / 0001 | OpenSBI Base -> Prepared，建立固件系统规格事实 |
| `sig-0004` | `Computer -> Kernel.Preset` | drives / 0001 | Kernel Base -> Prepared，采纳 Linux/RV64 boot 规格 |
| `sig-0005` | `Human -> Computer.Setup` | drives / none | `Computer.Transition::Setup@Prepared`；等待 0006–0010 后建立 assembly 并提交 Computer Ready |
| `sig-0006` | `Computer -> Riscv64Platform.Setup` | drives / 0005 | 平台 Prepared -> Ready，提交平台构造事实 |
| `sig-0007` | `Computer -> OpenSBI.Setup` | drives / 0005 | OpenSBI Prepared -> Ready，提交固件构造事实 |
| `sig-0008` | `Computer -> Kernel.Setup` | drives / 0005 | 等待 0009–0010 后建立 ELF/Image 构造与 Enable 可接受事实，Kernel Prepared -> Ready |
| `sig-0009` | `Kernel -> Config.Enable` | drives / 0008 | Config Ready -> Online，先于 Lds 发布 |
| `sig-0010` | `Kernel -> Lds.Enable` | drives / 0008 | 验证 Config Online 后 Lds Ready -> Online |
| `sig-0011` | `Human -> Computer.Enable` | emits / none | FIFO 接收后 Computer Ready -> Online，再 emits 0012 |
| `sig-0012` | `Computer -> Riscv64Platform.Enable` | emits / 0011 | FIFO 接收后平台 Ready -> Online，再 emits 0013 |
| `sig-0013` | `Riscv64Platform -> OpenSBI.Enable` | emits / 0012 | FIFO 接收后进入 OpenSBI Enable；等待 0014–0015 后提交装载、ABI 与 ordered-boot 事实并使 OpenSBI Ready -> Online |
| `sig-0014` | `OpenSBI -> CpuGroup.Preset` | drives / 0013 | `CpuGroup.Transition::Preset@Base`；原子声明 CPU0，等待 0015 后将父对象提交 Prepared |
| `sig-0015` | `CpuGroup -> CpuGroup.cpus[0].Preset` | drives / 0014 | `CpuGroup.cpus[0].Transition::Preset@Base`；采纳 index-derived logical ID 与入口 hartid并提交 Prepared |

0001–0010 与 0014–0015 是十二个同步 request/feedback；0011–0013 是三个异步 request/settle，共
15 个 Signal、30 个 animation moment。同步父状态只在全部同步子响应成功后提交；三个异步 sender
先提交自身 Online，再把后继放入全局 FIFO，后继失败不回滚已经提交的 sender。0009 必须先于 0010；
0011–0013 的每次 enqueue position 都是 1，且各自 dequeue 后 remaining 都是 0。`Kernel.Enable` 在
这个发送前边界没有 Signal identity、receive 或 handler 事件。

本组 guards 和事实提交分为四层：Computer Preset/Setup 保证三个直接子系统依次 Prepared/Ready，
Setup 才发布 `computer_assembled_from`；Kernel Setup 依次发布 Config/Lds 并建立 ELF、boot Image 和
`kernel_enable_accept_available`；Computer 与平台 Enable 分别验证 assembly 和已构造的平台；
OpenSBI Enable 验证平台、固件、只读 BootArgs、Linux boot 规格、Config/Lds 与 Image 文件构造，
然后同步驱动 CpuGroup 原子发布唯一 owned element `CpuGroup.cpus[0]` 及父对象 Prepared，最后提交
`kernel_load_pa` 非零且物理 PMD 对齐、Image 已装载、a0/a1、`satp=0`、DTB 可访问且完整、ordered
boot、primary hart、BootTaskRef 和 task execution facts。本组恰有一个 `indexed_instance_declared`
事件；子对象或父响应任一步失败都回滚该发布，不留下 element、index 或稳定状态。本范围没有适用的
Context/Lock 进入退出，也没有其它 fresh instance；不能为了填充账本合成它们。

本组结束用第二条真实上游截断证明。从同一模型初态执行 `-u BootInitFlow.Preset` 时，0013 完成后
必须先完成 0014–0015，再实际创建 `sig-0016 OpenSBI -> Kernel.Enable`，delivery 为 emits、cause 为
0013。0016 被 FIFO dequeue 和 Kernel receive，全部入口 guards 成立并进入
`Kernel.Transition::Enable@Ready`；它随后同步驱动 `sig-0017 Kernel -> Kernel.AcceptEnable`、
`sig-0018 Kernel -> BootInitFlow.AssignCpuRef` 和
`sig-0019 Kernel -> PhysicalDirect.ActivateOnCpu`。0017 必须保持 Kernel Ready 并提交
`kernel_enable_accepted(Kernel)`；0018 必须把 `ref(CpuGroup.cpus[0])` 绑定到唯一
`BootInitFlow.cpu_ref`；0019 必须验证入口 `satp=0` 与 absent association，并以 InitialActivation 原子
建立 PhysicalDirect association。紧接着在创建 `BootInitFlow.Preset` 之前 reached：boundary snapshot
等于 0019 的 after snapshot，Kernel 仍为 Ready、BootInitFlow 仍为 Base、CPU0 与 CpuGroup 均为
Prepared、BootCPURef 已绑定且 PhysicalDirect 已在 CPU0 激活，并且
不存在 BootInitFlow lifecycle Signal identity、send/receive 或 handler 事件。因为同步父响应尚未走到
最终 Online commit，0016 在该有界
推导中以 `stopped: until_signal_reached` 结束；这表示 handler 已接受且 ancestor 未提交，不是 rejected
或 failed，也不能改写成 Kernel Ready -> Online。

Linux 6.12 `Documentation/arch/riscv/boot.rst` 为这一交接固定 a0=hartid、a1=DTB physical address、
入口 `satp=0`、RV64 Image 物理 2 MiB/PMD 对齐和 ordered boot 约束。真实 `_start` 在任何
BootInitFlow child checkpoint 前读取 live `satp` 并 fail-stop，保存 a0/a1；announce checkpoint 中
`Kernel.Started` 的 `R` 必须先于首个 `InterruptType.Prepared` 的 `I`。Rust 入口随后以同一 a0/a1
物化 BootArgs 并执行 `accept_enable_at_entry`。设计期 tools2 snapshot、Linux ABI 和这些真实入口观察
共同确认交接；任一方不能单独替代另外两方。

失败账本保持严格：0001–0010、0014–0015 的同步子失败短路后续 drives/emits 且不提交未完成
ancestor；0011–0013 的异步失败传播到根结果但保留已提交 sender；缺失 assembly、Image/ABI guard
时不得进入对应 handler；0014/0015 失败不得留下 CPU element 或创建 Kernel.Enable，0017/0018 失败
时 0016 失败且不得创建 BootInit lifecycle Signal；重复 Kernel.Enable 必须拒绝。所有失败都保留完整
cause chain 和最后稳定 snapshot。

## 第 2 组确定语义与因果账本

第 2 组继续使用第 1 组已经核准的 `Kernel.Enable` 发送前 snapshot，但验收必须重新从模型初态和
唯一 `external Human` 编排到达 `BootInitFlow.Setup` 发送前，不能把提交的 scenario 当作真实 sender
证据。完整路径恰有 53 个 Signal；`sig-0016 OpenSBI -> Kernel.Enable` 保持已接受但 ancestor 尚未
提交的 `stopped: until_signal_reached`，其后的同步 drives 全部完成。本组确定的
`sig-0016` 至 `sig-0053` 如下：

| ID | source -> target.Signal | delivery / cause | handler 与提交边界 |
| --- | --- | --- | --- |
| `sig-0016` | `OpenSBI -> Kernel.Enable` | emits / 0013 | `Kernel.Transition::Enable@Ready`；等待 0017–0052，边界截断时 Kernel 保持 Ready、outcome 为 stopped |
| `sig-0017` | `Kernel -> Kernel.AcceptEnable` | drives / 0016 | `Kernel.Action::AcceptEnable@Ready`；保持 Ready 并提交 Enable acceptance |
| `sig-0018` | `Kernel -> BootInitFlow.AssignCpuRef` | drives / 0016 | `BootInitFlow.Action::AssignCpuRef@process`；唯一写入 `BootCPURef -> CpuGroup.cpus[0]` |
| `sig-0019` | `Kernel -> PhysicalDirect.ActivateOnCpu` | drives / 0016 | Action；验证入口 `satp=0` 与 absent association，以 InitialActivation 把 CPU0 原子关联到 PhysicalDirect |
| `sig-0020` | `Kernel -> BootInitFlow.Preset` | drives / 0016 | `BootInitFlow.Transition::Preset@Base`；在一个 `SingleTaskContext` 内等待 0021–0052 后提交 Prepared |
| `sig-0021` | `BootInitFlow -> InterruptType.Preset` | drives / 0020 | Base -> Prepared；先关闭 CPU0 的全部中断分路门控，再清空这些门控上的全部待决中断信号；不改变总门控或建立 fallback |
| `sig-0022` | `BootInitFlow -> KernelImage.Preset` | drives / 0020 | Base -> Prepared，建立相对 `gp` 寻址基准 |
| `sig-0023` | `BootInitFlow -> CpuGroup.cpus[0].DisableFpuVectorExecution` | drives / 0020 | Action；CurrentCPU 解析到 CPU0，关闭浮点与向量执行状态并建立受控使用策略 |
| `sig-0024` | `BootInitFlow -> KernelImage.Setup` | drives / 0020 | Prepared -> Ready，提交 BSS 清零完成、普通可写与映像虚拟范围事实 |
| `sig-0025` | `BootInitFlow -> BootInitFlow.RecordBootCpuHartid` | drives / 0020 | Action；保存入口 a0 原值供后续 hartid 建立使用 |
| `sig-0026` | `BootInitFlow -> KernelAddrSpace.Preset` | drives / 0020 | 等待 0027–0028 后 Base -> Prepared，建立地址区域布局 |
| `sig-0027` | `KernelAddrSpace -> LinearMap.Preset` | drives / 0026 | Base -> Ready，固定从 `PAGE_OFFSET` 开始的物理线性映射区域 |
| `sig-0028` | `KernelAddrSpace -> UserSpaceReserve.Preset` | drives / 0026 | Base -> Ready，保留 canonical 用户地址范围 |
| `sig-0029` | `BootInitFlow -> CpuGroup.cpus[0].Setup` | drives / 0020 | CurrentCPU 解析到 CPU0；Prepared -> Ready |
| `sig-0030` | `BootInitFlow -> BootCpuLocalInterrupt.Setup` | drives / 0020 | Prepared -> Ready，建立 CPU0 本地中断关闭事实 |
| `sig-0031` | `BootInitFlow -> CurrentTask.BindTaskStack(BootTask, BootTask.stack)` | drives / 0020 | 单个 boot-only 上下文 Action；PhysicalDirect 下原子建立 CPU0 task/stack pair 并写物理 `tp/sp`，不初始化 preempt count |
| `sig-0032` | `BootInitFlow -> TrapType.Preset` | drives / 0020 | Base -> Prepared，为所属 CPU 建立临时保护入口，用于处理意外事件并支持测试和缺陷定位 |
| `sig-0033` | `BootInitFlow -> ExceptionType.Preset` | drives / 0020 | 等待 0034–0037 后 Base -> Prepared |
| `sig-0034` | `ExceptionType -> PageFaultException.Preset` | drives / 0033 | Base -> Prepared |
| `sig-0035` | `ExceptionType -> SyscallException.Preset` | drives / 0033 | Base -> Prepared |
| `sig-0036` | `ExceptionType -> BreakpointException.Preset` | drives / 0033 | Base -> Prepared |
| `sig-0037` | `ExceptionType -> UnexpectedException.Preset` | drives / 0033 | Base -> Prepared |
| `sig-0038` | `BootInitFlow -> Vm.Preset` | drives / 0020 | 等待 0039–0045 后 Base -> Prepared |
| `sig-0039` | `Vm -> TrampolineVm.Setup` | drives / 0038 | Base -> Ready，建立共享 trampoline controller |
| `sig-0040` | `Vm -> EarlyVm.Preset` | drives / 0038 | 等待 0041–0043 后 Base -> Prepared |
| `sig-0041` | `EarlyVm -> RawDtb.Preset` | drives / 0040 | Base -> Prepared，验证入口/header/magic |
| `sig-0042` | `EarlyVm -> RawDtb.Setup` | drives / 0040 | Prepared -> Ready，检查 size/overflow/slot capacity，不解析节点 |
| `sig-0043` | `EarlyVm -> FixMap.Preset` | drives / 0040 | Base -> Ready，将 RawDtb 安排到 FDT slot |
| `sig-0044` | `Vm -> KernelAddrSpace.Setup` | drives / 0038 | Prepared -> Ready，提交四区域 disjoint/canonical reserve |
| `sig-0045` | `Vm -> EarlyVm.Setup` | drives / 0038 | Prepared -> Ready，建立 Image 与 fixmap 的 early page table |
| `sig-0046` | `BootInitFlow -> Vm.Setup` | drives / 0020 | 等待 0047–0049 后 Prepared -> Ready |
| `sig-0047` | `Vm -> TrampolineVm.ActivateOnCpu` | drives / 0046 | Action；CPU0 PhysicalDirect -> TrampolineVm Handoff |
| `sig-0048` | `Vm -> EarlyVm.ActivateOnCpu` | drives / 0046 | Action；CPU0 TrampolineVm -> EarlyVm Handoff，trampoline 仅对该 CPU 退役 |
| `sig-0049` | `Vm -> KernelImage.Enable` | drives / 0046 | Ready -> Online，确认当前执行环境中的相对 `gp` 寻址机制可用 |
| `sig-0050` | `BootInitFlow -> TrapType.Setup` | drives / 0020 | Prepared -> Ready，安装正式 event entry |
| `sig-0051` | `BootInitFlow -> CurrentTask.RefreshTaskStack(BootTask, BootTask.stack)` | drives / 0020 | 单个 boot-only 上下文 Action；EarlyVm 下保持同一 pair identity 并原子刷新当前地址表示 |
| `sig-0052` | `BootInitFlow -> Soc.Preset` | drives / 0020 | 执行平台早期初始化边界并从 Base 迁移到 Prepared；具体平台语义当前为 Deferred |

`sig-0020` 的 handler 进入前必须逐项验证 Kernel acceptance、RISC-V/SBI/OpenSBI、BootCpuRegisters、
Config/Lds、BootTask `OnCpu/Live/Invalid`、initial-flow binding、task concurrency 和入口 `satp=0`。
它还必须观察 0018 已绑定的有效 CpuRef。它只进入和退出一对 `SingleTaskContext`；该 context 覆盖全部
同步 child drives，并在 0020 提交前退出。本组没有 Lock acquire/release 或新的运行期 instance；
CPU0 的唯一 indexed declaration 已在第 1 组完成，不得用最终 `context_is(SystemExclusive)` 事实虚构
第二个 context、Lock 或实例事件。

最后一个 child 0052 完成后，0020 检查 CPU0 的 CurrentTask/CurrentStack binding 仍解析为
BootTask/BootTask.stack 及 `task_flow_started(BootInitFlow)`，再把
BootInitFlow 从 Base 提交到 Prepared。此时 Kernel.Enable 的同步 drives 列表下一项是
`BootInitFlow.Setup`；有界推导必须在创建该 Signal 之前 reached。boundary provenance 的 source/target
必须是 `Kernel -> BootInitFlow`、canonical signal 必须是 `BootInitFlow.Setup`、delivery 必须是 drives、
cause 必须是 0016，snapshot 必须等于 0020 的 after snapshot。边界处 Kernel 为 Ready、BootInitFlow
为 Prepared；CpuGroup、InterruptType、ExceptionType、Soc、KernelAddrSpace 为 Prepared/Ready 边界规定的状态，`CpuGroup.cpus[0]` 为
Ready 且 active controller 是 EarlyVm，KernelImage 为 Online，TrapType/Vm/RawDtb/FixMap、
TrampolineVm/EarlyVm 为 Ready。不存在独立入口 binding snapshot，也不存在 controller Cleanup/Destroyed。`CurrentCPU` 的 selector resolution 必须记录 source flow
`BootInitFlow`、source ref `BootCPURef` 与 canonical target `CpuGroup.cpus[0]`；它本身不得拥有 state、
instance 或 lifecycle。不得创建 `BootInitFlow.Setup` identity，也不得出现它的 send、receive、handler
或 Context 事件。

Linux 6.12 的 RISC-V `head.S` 依次关闭 BootCPU 的全部中断分路门控并清空其待决中断信号、建立 `gp`、关闭 BootCPU 的浮点与向量执行状态、清 BSS、保存 boot
hart、建立 `tp/sp/stvec`，再由 `setup_vm()` 建立 trampoline、early kernel mapping 和 FDT fixmap，
`relocate_enable_mmu()` 完成 trampoline 到 early page table 的切换。现有 entry checkpoint 与实现把
Rust 前不可延迟的动作作为同一 BootInitFlow.Preset 的 head segment adoption，并在 VM continuation
返回、TrapType、task/stack 地址表示刷新与 Soc 完成后提交 `BootInitFlow.Prepared`；`EntrySuccessorPhase.Started`
只能出现在其后，属于第 3 组 Setup。`BootInitFlow.Started` 必须观察 Preset 已在前三项 Kernel child
完成后被接受，并位于首个 Preset child action 之前；Rust 可以稍后验证和采用 head handoff facts，但
这种物理 adoption 时点不得重排 Signal 账本、伪造提前 child commit 或提前提交 BootInitFlow.Prepared。

失败账本按首个边界停止：缺失 Kernel acceptance、BootTask `OnCpu/Live/Invalid` 或 initial-flow binding
时，0020 必须拒绝且不得创建 0021；缺失/悬空 CpuRef 必须在 0018 写入或 0027 解引用边界拒绝，缺失
入口 CSR/VM 条件时必须在消费它的 Kernel/BootInit/child handler 处拒绝；任一 child guard 失败都使
包含它的同步 ancestor failed，短路该 child 之后的所有
drives，并保持 BootInitFlow Base、Kernel Ready，不提交 `task_flow_started(BootInitFlow)` 或
BootInitFlow.Prepared。重复 BootInitFlow.Preset、绕过真实 Kernel.Enable、陈旧 model fingerprint 和
缺失 canonical scenario 都必须严格拒绝，不能回溯 emitter、合成事实或重试。

## 单阶段因果账本

每个阶段和关键 action 都必须维护并评审下列顺序的因果账本：

```text
source Signal → target System → handler → guards → child Signals → state commit → facts/emits
```

账本至少回答：

1. `source Signal`：谁在什么已提交状态下创建 Signal；canonical 名称、payload、delivery、cause 和
   FIFO/source order 是否稳定；是否存在不应发送或遗漏的 Signal。
2. `target System`：动态 receiver、有效 parent、运行期 identity 和到达前 lifecycle 是否正确；目标
   是否确实是 Linux 控制流中承担该责任的系统边界。
3. `handler`：解析到的 Transition/Action 及 override 是否唯一；Action 是否被错误地显示或实现为
   lifecycle 迁移；handler 的执行主体和栈是否与当前 Task/Flow 一致。
4. `guards`：depends_on、invariant、Context、Lock、中断、抢占、RCU 和执行权条件是否在进入时真实
   成立；guard 的进入、继承、嵌套和退出是否配对，失败是否在提交前终止。
5. `child Signals`：同步 drives、ordered choice 和异步 emits 是否完整且顺序正确；未选候选不得产生
   Signal；子失败是否沿完整 cause chain 回传。
6. `state commit`：target 的 before/after 状态及提交时点是否准确；不可逆 task/flow/payload 切换前
   的最后可逆边界是否明确；ancestor 未提交时不得被快照伪装为完成。
7. `facts/emits`：ensures、引用更新、动态实例、checkpoint 和 post-commit emits 是否来自真实动作；
   不得用宽泛事实掩盖缺失的状态迁移、资源操作或失败分支。

账本以结构化 JSON 为逐字段证据，以文本保留可审查的线性路径，以动画检查全局时序和结构。任何一栏
无法用当前产物回答，都表示该阶段尚未完成语义校准，而不是允许从最终 Online 状态反推中间过程正确。

## 重点检查项

### Context 与 Lock

Context 不能只是 handler 上的静态标签，Lock 也不能只靠最终 `lock_held` 类事实表示。校准必须核对
Context 的进入/退出 Signal 或其它正式 guard 动作、词法继承与嵌套、obj_refs 可见范围、中断/抢占
状态，以及具体 lock instance、acquire/release 方式和失败边界。进入与退出必须在因果账本中配对；
Context 结束后不得残留权限，Lock guard 也不得被 handler 的最终成功隐式补齐。

当前观察到的第一个校准案例正是 Lock/Context：完整闭包和最终状态可以成功，但现有输出尚不足以对
若干 `within` 边界逐一证明 guard 的真实进入、继承、嵌套、退出及其与具体 Lock 操作的对应关系。这一
现象证明“闭包成功不等于语义完整”。它在本章程中只作为待定位案例，不预判缺陷最终属于 charter、
model、tools2 还是旧工具，也不把“增加隐式 Signal”“合成事实”或某种固定 lowering 直接写成修复
设计。后续必须先用账本和 checkpoint 把第一个失败边界定位清楚，再按 charter-first 决定表达和实现。

### 运行期迁移

Task、TaskFlow、CPU、CurrentTask 解析、栈和 active-flow 的迁移必须按 precommit、commit、真实入口
和后继 continuation 分开核对。特别是 BootTask→KernelInitTask、KernelInitFlow→UserAppFlow 以及
AP pointwise Flow，不能只比较前后两个 Online 状态；必须证明执行权、owner/parent、current、stack、
Context 和不可逆边界在同一因果链上同步变化，旧 Flow 不会继续以原权限执行。

### 失败传播

每个已发送的严格 Signal 都必须得到 completed、rejected、failed、truncated 或 stopped 等明确结果；
guard 不成立、handler 失败、同步子 Signal 失败和异步 FIFO 失败都要保留完整 cause chain，并传播到
根结果。校准必须主动覆盖错误路径，不能只运行成功闭包。precommit 失败不得留下部分 lifecycle、引用、
锁、上下文或动态实例提交，post-commit 失败则必须按该阶段 charter 的不可逆语义处理，不能伪装回滚。

### 动态实例

`declare` 或其它 fresh instance 建立必须记录声明点、类型、稳定 runtime identity、词法 alias、parent、
owner 和生命周期起点。多次执行、循环或按 CPU/Task 复制时，实例之间不得复用 identity 或状态；后续
Signal 必须指向实际创建的实例，snapshot/view/animation 也必须保留同一个 identity。静态具名 object、
类型占位符和运行期实例事实不得互相替代，失败路径不得发布未提交实例。

## 四方问题判定

问题由 charter、Linux/checkpoint、tools2 和旧 `tools/` 四方证据共同定位，但不采用多数表决，也不
把任一工具输出当作自然正确：

| 证据方 | 回答的问题 | 不能承担的责任 |
| --- | --- | --- |
| charter | 项目希望该对象、阶段和失败边界具有什么设计语义 | 不能用笼统目标替代可验证边界，也不能否定已经复现的 Linux/checkpoint 事实而不作明确取舍 |
| Linux 6.12 / checkpoint | 参考实现真实按什么顺序、执行主体、guard 和提交点运行 | 单段源码名称不是项目对象边界；观察不足时不能从日志缺失推断动作不存在 |
| tools2 | 当前正式 model 在 tools2 语义下实际推导出什么因果链和 snapshot | 闭包成功不能证明未建模语义；展示结果不能修改 derive 含义 |
| 旧 `tools/` | 既有规则、产物、CLI 和根门禁目前依赖什么，可提供哪些回归对照 | 不是 charter、Linux 或新工具的真值 oracle，也不能因历史测试通过冻结错误语义 |

判定顺序如下：先以可复现 checkpoint 或其它诊断确定第一个行为边界，再判断 charter 是否已经明确该
边界的设计意图。若意图缺失或与已确认目标冲突，先修 charter；若 charter 已明确而正式 model 不符，
按 charter-first 修 model 及下游层；若 model 的结构化含义正确而 tools2 推导或 view 不符，归为
tools2 缺陷；若只有旧工具与已闭合的 charter/model/Linux/tools2 证据冲突，归为旧工具缺陷。证据仍
不足时结论只能是“诊断不足”，必须先补长期有用的 checkpoint，不能猜修复。

旧工具不是真值 oracle。校准允许发现并修正规格错误，也允许发现并修正旧工具错误；Linux 对齐本身
若与项目明确设计取舍不同，也应在 charter 中显式记录，而不是让任一工具静默选择。双工具结果一致
只能增加回归信心，不能把共同遗漏升级为正确语义。

## tools2 接管与旧工具退役顺序

旧工具退役采用逐责任接管，不以一次完整闭包或一次 CLI 替换直接删除：

1. 先完成上述启动阶段校准，使 tools2 的 derive/view 成为新语义的主验收路径；在覆盖和差异清单
   关闭前，旧 derive/view 只作 shadow comparison 和既有产物维护。
2. derive/view 接管后迁移 prove。每项证明义务都必须能追溯到 charter/model，并以正反例验证；不能
   为匹配旧工具结论而复制其未说明的规则。
3. prove 稳定后迁移 codegen。生成输入、产物协议、确定性、失败退出和生成后测试必须先由 tools2
   路径覆盖，旧生成器仍保留可比较的回退窗口。
4. codegen 接管后统一 CLI。逐项迁移参数、默认值、输出协议、退出码和工作目录约束，明确兼容别名
   的期限；不能让新 CLI 外壳暗中调用旧实现而宣称已经接管。
5. CLI 稳定后才迁移仓库根门禁。先把同等或更强的 tools2 检查加入根回归，观察稳定性并清除对旧
   产物的隐式依赖，再移除旧门禁。
6. 只有 derive/view、prove、codegen、CLI 和根门禁都已完成责任迁移，且旧工具独有调用者、文档、
   fixture 和生成产物已经盘点处理，才可删除旧 `tools/`。删除必须作为独立 charter-first 变更，保留
   可审计的迁移记录和最终根回归结果。

任一步发现差异，都回到对应启动阶段的因果账本判定问题方；不得为了推进退役而以旧输出覆盖新
snapshot，也不得为了证明 tools2 正确而删除仍能暴露差异的旧门禁。

## 完成标准

某一启动校准批次只有在以下条件全部成立后才可标为完成：canonical 入口 snapshot 可从真实上游稳定
重建；组内每个阶段的因果账本闭合；动画、文本和结构化 JSON 的职责分别验收；Linux/checkpoint 证据
足以定位关键边界；Context、Lock、运行期迁移、失败传播和动态实例的适用检查项有明确结论；charter、
model、coding、适用 compose、实现和 testing 按 authority 顺序一致；focused validation 与仓库根
回归通过。未满足的项目必须保留为明确问题或 deferred 边界，不能由成功闭包代替。

## 相关文档

- [系统 Signal 语义](../system-signal.md)
- [tools2 Signal 动画章程](signal-animation.md)
- [BootInitFlow](../phases/boot-init-flow.md)
- [KernelInitFlow 的 SMP/runtime 叶子阶段](../phases/smp-runtime.md)
- [KernelInitFlow 的 payload 准备与提交](../phases/payload.md)
- [运行期实例声明](../objects/dynamic-instance-declaration.md)
