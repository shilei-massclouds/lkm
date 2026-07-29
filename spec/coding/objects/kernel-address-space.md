# KernelAddrSpace 与 translation controller coding contract

`Kernel` 的直接子对象 `KernelAddrSpace` 是唯一的内核地址空间资源；`Vm` 是平级的地址转换控制面，
不是第二个地址空间。Rust `Context` 分别保存两者，不能用 `Vm` lifecycle 替代区域布局状态。

## 地址空间区域

`KernelAddrSpace` 直接拥有 `KernelImage`、`FixMap`、`LinearMap` 和 `UserSpaceReserve`。实现必须保存并
验证每个虚拟范围的 parent、端点、页对齐和两两不重叠：

- `KernelImage` 只表示固件交接时已加载在内存中的内核映像，保存物理装载范围、链接虚拟范围和段信息。
  含混的独立 `KernelImageMap` 投影不得保留；所有映像映射检查直接使用 `KernelImage.virt_range`。
- `FixMap` 保存配置给出的固定槽位。本轮 FDT slot 容量仍为 2 MiB；它只临时映射 `RawDtb`，不拥有
  该物理 blob。
- `LinearMap` 从 `PAGE_OFFSET` 开始，表示物理内存的线性映射虚拟区域；入口前导期 Ready 只表示布局
  已保留，不能冒充完整 RAM bank 已映射。
- `UserSpaceReserve` 是体系结构 canonical 用户地址保留范围，任何 kernel region 或最终 Swapper 映射
  都不得占用它。

`KernelAddrSpace::preset` 建立区域布局并推进 KernelImage；`setup` 在 FixMap Ready 后验证全部区域约束，
提交 Ready；`enable` 只在 Swapper 页表 Ready 后发布最终映射并提交 Online。Online 是全局地址空间发布
事实，不表示所有 CPU 已切换到 Swapper。

`KernelImageFile` 是未来文件对象的保留名称。本轮不得新增这个类型、实例、生命周期或兼容 alias；构建
层事实使用 kernel ELF 和 kernel boot artifact，运行时 `KernelImage` 只描述内存中映像。

## 共享 controller 与每 CPU 接管

`Vm` 直接拥有 `PhysicalDirect`、`TrampolineVm`、`EarlyVm`、`SwapperVm`。四个对象是共享
translation controller；页表/映射准备状态是全局 `Ready`，激活状态只保存在各 CPU 的
`active_translation_owner`。不为 Trampoline/Early/Swapper 建立全局 Online/Destroyed 状态，也不保留
`TrampolineVm.Cleanup`、`EarlyVm.Cleanup` 或同义 checkpoint/snapshot。

每个 controller 实现统一的 `take_over(cpu_ref)` 事务：

1. 解析 CpuRef 并验证 controller Ready、允许的旧 owner 以及旧 owner 对应的 live `satp`。
2. 按 controller 规则写入目标 `satp` 并执行所需 `sfence.vma`；PhysicalDirect 的目标 SATP 为 0。
3. 原子替换该 CPU 的 owner，并记录 CPU、旧/新 owner、SATP 和同步事实。任一前置检查失败不得修改
   owner、SATP 或 trace；提交期不一致必须 fail-stop。

BP 接管链固定为 `PhysicalDirect -> TrampolineVm -> EarlyVm -> SwapperVm`；AP 链固定为
`PhysicalDirect -> TrampolineVm -> SwapperVm`。从 Trampoline 切到 Early 只使前者对该 CPU 退役；
静态 trampoline 页表继续 Ready，供随后 AP 使用。下游阶段检查 `KernelAddrSpace/Vm` Online、相关
controller Ready，并只在语义确实依赖当前执行 CPU 时校验其 owner/live SATP。

## RawDtb 边界

`RawDtb::preset/setup` 依次检查入口物理地址非零、固定头部范围加法不溢出、头部可访问、magic、
`total_size >= sizeof(DtbHeader)`、完整范围加法不溢出和 FDT FixMap slot 临界容量。完整 blob 可访问性
来自 OpenSBI handoff 契约；实现不得伪装成逐字节物理探测。该对象只记录 header 与完整物理范围，
不得解析 `/memory`、`/cpus` 或其它节点；节点解析仍由后续 `EarlyDtb`/`DeviceTree` 对象承担。
