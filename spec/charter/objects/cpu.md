# CPU

`CPU` 是每个逻辑处理器实例的统一类型。它保存独立 lifecycle、物理 `hartid`、由容器索引派生的
`logical_id`，以及 possible/present/active/online 状态。系统中不存在 boot/AP 专用 CPU 类型；
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
CPU 实例属性；CpuGroup 必须维护已发布元素间的一一映射，并拒绝重复 hartid。按 logical ID 查找
hartid 与按 hartid 查找 CpuRef 必须互为反向映射。

## Lifecycle 与集合状态

CPU 的通用 lifecycle 为 Base → Prepared → Ready → Online。CpuGroup.Preset 在同一原子发布中创建
`cpus[0]`、记录入口 hartid 并推进它到 Prepared；BootInitFlow.Preset 通过 CurrentCPU 把 CPU0 推进
到 Ready，后继平台验证再使它 Online。CpuGroup.Setup 根据已验证拓扑创建 AP 元素；AP 的后续
Ready/Online 仍由对应 CPU 的入口和 hotplug 因果推进。

possible/present/active/online 集合只能从每个已发布 CPU 的状态或属性派生。实现可以缓存位图，但缓存
必须可由 `cpus[]` 重建且不得保存 CpuRef/CpuView 副本数组。

## CPU-local 子对象

`CurrentTaskSlot`、本地中断控制和其它真正 per-CPU 的对象属于相应 `CPU` 实例。当前轮次保留
CurrentTaskSlot 的既有语义，只把其存储归属迁到 `CpuGroup.cpus[0]`；它仍是唯一 OnCpu Task 的只读
投影，不是 Task 执行权来源。

## CurrentCPU capability

`CurrentCPU` 是保留的上下文选择器，不是 object、instance、owner 或 lifecycle：

```text
CurrentCPU := dereference(effective_task_flow.cpu_ref)
```

它只能从具有有效 CpuRef 的当前 TaskFlow 构造。TaskFlow 及其同步 `drives` 子孙继承同一个解析结果；
异步 `emits` 不继承，接收方必须从自己的 effective TaskFlow 重新解析。每次 trace 必须记录 canonical
target（如 `CpuGroup.cpus[0]`）和解析来源 Flow/CpuRef，不能只记录字符串 `CurrentCPU`。

## Mapping

- Model: `spec/model/objects/cpu.spec`
- Coding: `spec/coding/objects/cpu.md`
- Implementation: `impl/arceos_ex/src/objects/cpu.rs`
