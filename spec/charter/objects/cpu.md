# CPU

`CPU` 是每个逻辑处理器实例的统一类型。它保存独立 lifecycle、物理 `hartid`、由容器索引派生的
`logical_id`，possible/present/active/online 状态，以及可选的 `active_translation_controller` association。系统中不存在 boot/AP 专用 CPU 类型；
`BootCPU`、`ApCPU(i)` 只是同一实例集合上的角色别名：

```text
BootCPU := CpuGroup.cpus[0]
ApCPU(i) := CpuGroup.cpus[i], i > 0
```

角色名不得拥有 lifecycle、字段或存储。CPU0 与 AP 元素均只能由唯一 `CpuGroup` 的 indexed-owned
collection 建立；其它对象只能保存 `CpuRef`，不得复制 CPU state 或建立 `CpuView`。

## 身份与引用

`CpuRef` 是指向 `CpuGroup.cpus[logical_id]` 的类型化稳定引用。lowering 只需保存紧凑的 `LogicId`，
但每次解引用都必须同时验证索引范围、目标元素已发布且元素的派生 logical ID 与该索引一致。不存在的
元素、失败创建留下的 key 和越界 key 均不得解引用；PID、hartid、角色名或数组地址都不能替代
`CpuRef`。

logical ID 仅由 owned collection 的 key 派生，不在 CPU 内维护可漂移的第二份权威值。`hartid` 是
CPU 实例属性；它被赋值后，CpuGroup 必须维护已赋值元素间的一一映射，并拒绝重复
hartid。对已赋值元素，按 logical ID 查找 hartid 与按 hartid 查找 CpuRef 必须互为反向映射。

## Lifecycle 与集合状态

CPU 的通用 lifecycle 为 Base → Prepared → Ready → Online。CpuGroup.Preset 在同一原子发布中创建
`cpus[0]` 并推进它到 Prepared；BootInitFlow.Preset 保存入口第一个参数，但不在汇编入口中直接写入
CPU 对象。后续 `smp_setup_processor_id()` 对应阶段再把保存值写为 BootCPU 的 `hartid`。
BootInitFlow.Preset 通过 CurrentCPU 把 CPU0 推进到 Ready，后续平台验证再使它 Online。CpuGroup.Setup 根据已验证拓扑创建 AP 元素；AP 的后续
Ready/Online 仍由对应 CPU 的入口和 hotplug 因果推进。

possible/present/active/online 集合只能从每个已发布 CPU 的状态或属性派生。实现可以缓存位图，但缓存
必须可由 `cpus[]` 重建且不得保存 CpuRef/CpuView 副本数组。

## 浮点与向量运算能力

关闭 BootCPU 的浮点运算和向量运算能力。内核态默认禁止使用这些能力，只在明确受控的执行区间内才
允许临时打开，随即关闭。用户态根据任务需要和系统策略打开。

这些执行状态属于对应 CPU，不属于启动 Flow 或 CpuGroup。CpuGroup 只负责 CPU 实例的 ownership、
index 与 reference 关系；BootInitFlow 只按入口顺序驱动 BootCPU 完成初始关闭。

## CPU-local 子对象

每个已发布 CPU 恰好拥有一个独立 `TrapType` 资源；Trap 再拥有独立 `InterruptType` 与
`ExceptionType`，Exception 拥有 page-fault、syscall、breakpoint、unexpected 四个具体资源。完整
结构为：

```text
CPU.trap: TrapType
TrapType.interrupt: InterruptType
TrapType.exception: ExceptionType
ExceptionType.{page_fault, syscall, breakpoint, unexpected}
```

这些 resident 资源随 CPU 建立并保持各自 lifecycle、入口容量与 handler/gate 状态，任何一个 CPU 的
状态都不得由全局对象或其它 CPU 的缓存副本替代。CurrentTask 不属于 CPU 子对象，也不存在
`CurrentTaskSlot` lifecycle；它是 CPU 执行上下文在入口或 scheduler commit 提交的 task binding，解析
时与 effective TaskFlow 的 parent/owner/active 关系交叉校验。每个 CPU 的 binding 独立，不能由全局
单例或其它 CPU 的缓存副本替代。

`active_translation_controller` 是 CPU-local 的 optional typed association，不是另一份页表状态。
尚未进入内核的 stopped AP 必须保持 association absent，且没有该 CPU 的 live SATP 事实；启动协议中
预先发布的期望 SATP 只是入口输入，不得当成 active association 或 live CSR。正在执行内核入口的 CPU
必须恰有关联一个 controller。首次从 absent 建立关联是 `InitialActivation`，已有 controller 间切换是
`Handoff`；二者都只能由 `PhysicalDirect`、`TrampolineVm`、`EarlyVm` 或 `SwapperVm` 的
`Action::ActivateOnCpu(cpu_ref)` 原子提交。Handoff 提交前必须证明旧 controller 与 live SATP 一致。
完整协议见
[`KernelAddrSpace 与启动期 translation controller`](kernel-address-space.md)。

## CurrentCPU capability

`CurrentCPU` 是保留的上下文选择器，不是 object、instance、owner 或 lifecycle：

```text
CurrentCPU := dereference(effective_task_flow.cpu_ref)
```

它只能从具有有效 CpuRef 的当前 TaskFlow 构造，包括 BootTask 首次 BindTask 之前的 BootInitFlow。
CurrentCPU 不依赖 CurrentTask 已经绑定。TaskFlow 及其同步 `drives` 子孙继承同一个解析结果；
异步 `emits` 不继承，接收方必须从自己的 effective TaskFlow 重新解析。每次 trace 必须记录 canonical
target（如 `CpuGroup.cpus[0]`）和解析来源 Flow/CpuRef，不能只记录字符串 `CurrentCPU`。

## Mapping

- Model: `spec/model/objects/cpu.spec`
- Coding: `spec/coding/objects/cpu.md`
- Implementation: `impl/arceos_ex/src/objects/cpu.rs`
