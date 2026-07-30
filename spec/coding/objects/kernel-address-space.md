# KernelAddrSpace Coding

`KernelAddrSpace` 必须由独立实现类型承载，并与 `Vm` 控制面分开保存。它是 `Kernel` 唯一的内核地址
空间资源，直接聚合 `KernelImage`、`FixMap`、`LinearMap` 与 `UserSpaceReserve` 的范围事实；controller
association、live `satp` 和每 CPU activation receipt 不得存入本对象。

`preset` 只建立早期布局：验证 `KernelImage` 已确定，并驱动 `LinearMap.Preset` 与
`UserSpaceReserve.Preset`。`setup` 在 `FixMap` 已准备后检查四个区域的 parent、端点、页对齐和两两
不重叠，再提交 Ready。Ready 不得伪装成完整 RAM 线性映射已发布，也不得据此断言任一 CPU 已启用
分页。

最终映射发布与 Online 属于后续 `Vm.Enable` 校准。本段实现不得借 `setup` 提前调用
`SwapperVm`、写 `satp` 或发布 Online。

`KernelImage` 的装载范围和 `gp` 寻址 lowering 由 [`kernel-image.md`](kernel-image.md) 负责；其它
下级区域分别由各自 Coding 文件负责。
