# CpuGroup

`CpuGroup` 是 `Kernel` 的直接子对象，也是内核中唯一的 CPU 实例 owner；它与 `Soc` 平级，二者之间
需要的启动先后关系只通过 signal/guard 表达，不通过 parent 表达。系统只有一个 `CpuGroupObject` 实例 `CpuGroup`，其唯一
权威集合为：

```text
owned {
    indexed cpus[key: LogicId]: CPU;
}
```

`CpuGroup.cpus[i]` 同时是对象树 canonical identity、Signal target 和 snapshot state key。任何
`BootCPU`、`CPU1`、`ApCPU(i)` 名称只允许作为显示/规格角色别名，不能进入状态表成为独立对象。

## Indexed ownership

只有 CpuGroup handler 可以向 `cpus` 插入元素。创建采用父 handler 的原子发布：handler 在临时
candidate 中创建 child、推进所需 child/parent 状态并验证所有 invariant；只有全部成功后 child 与
父状态才一起进入稳定 snapshot。重复 key、错误 key 类型、非 owner 插入、越界、child transition
失败或父 invariant 失败都拒绝整个 candidate，且不得留下元素、索引、引用或 identity occurrence。

Preset 原子创建 `cpus[0]`、从 BootArgs 记录 hartid，并把 CPU0 与 CpuGroup 一起发布为 Prepared；
OpenSBI 只有在观察这一 checkpoint 后才发送 Kernel.Enable。Setup 在 DeviceTree/SBI 与 boot CPU
online 前置满足后创建其余 AP 元素并发布完整拓扑。已发布 key 不得被覆盖或重新编号。

## 派生视图与引用

CpuGroup 允许按 logical ID 或 hartid 返回经验证的 CpuRef，并从 `cpus[]` 派生 possible、present、
active、online 集合。缓存位图仅是派生加速；禁止 `cpu_refs[]`、`CpuView[]`、`SecondaryCpuStore` 或
其它平行实例集合。CPU 状态变化后，任何集合查询必须观察同一 owned element。

`CpuRef` 不拥有目标。TaskFlow 是任务执行路径中唯一保存 CpuRef 的对象；入口 adoption 和调度提交
边界可以写它，Flow 本体执行及其同步 drives 子孙只能读取。Flow 离开 OnCpu 后保留已分配/最后归属
CpuRef；迁移只在调度 commit 修改。Flow handoff 必须在激活新 Flow 前复制旧 Flow 的 CpuRef，避免
handoff 窗口内 CurrentCPU 无法解析。

## 启动因果

1. `OpenSBI.Enable` 同步驱动 `CpuGroup.Preset`。
2. CpuGroup 原子发布 `CpuGroup.cpus[0]` 与自身 Prepared，然后 OpenSBI 异步发送 `Kernel.Enable`。
3. Kernel 接受 Enable 后把 `ref(CpuGroup.cpus[0])` 绑定到 `BootInitFlow.cpu_ref`。
4. `BootInitFlow.Preset` 通过 CurrentCPU 推进 CPU0 到 Ready并记录入口 hartid。
5. 后继平台验证使 CPU0 Online；`CpuGroup.Setup` 再建立 AP 元素和完整拓扑。

外部 OpenSBI 固件不需要实现内核对象；真实内核入口在接受 Kernel Enable 前采用并验证第 1-2 步的
模型效果，并发布长期 checkpoint。

## Mapping

- Model: `spec/model/objects/cpu_group.spec`
- Coding: `spec/coding/objects/cpu-group.md`
- Implementation: `impl/arceos_ex/src/objects/cpu_group.rs`
