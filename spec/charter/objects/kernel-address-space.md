# KernelAddrSpace

`KernelAddrSpace` 是 `Kernel` 直接拥有的唯一内核地址空间资源。它拥有 `KernelImage`、`FixMap`、
`LinearMap` 与 `UserSpaceReserve` 四个区域，并维护这些区域在同一个虚拟地址空间中的归属、布局和
互不冲突关系。它不负责选择 CPU 当前使用的页表；该控制职责属于 `Vm` 及其 translation controller。

`KernelAddrSpace.Preset` 建立四个区域的早期布局，其中 `LinearMap` 在本段只保留区域，
`UserSpaceReserve` 只建立不可被内核映射占用的范围。`KernelAddrSpace.Setup` 在 `FixMap` 的 FDT 槽位
和 `KernelImage` 范围都已确定后，确认早期布局完整且互不冲突，使其可以被 `EarlyVm` 使用。

`KernelAddrSpace.Ready` 只表示启动早期所需的地址空间布局成立，不表示完整线性映射已经发布，也不
表示任一 CPU 已经切换到某个 controller。最终 `SwapperVm` 映射发布和 `KernelAddrSpace.Online` 的
语义留给后续 `Vm.Enable` 校准。

各下级系统的权威定义分别见 [`kernel-image.md`](kernel-image.md)、[`fix-map.md`](fix-map.md)、
[`linear-map.md`](linear-map.md) 与 [`user-space-reserve.md`](user-space-reserve.md)。`RawDtb` 是固件
物理 blob，不是 `KernelAddrSpace` 的子区域。

## Mapping

- Model: `spec/model/objects/kernel_addr_space.spec`
- Coding: `spec/coding/objects/kernel-address-space.md`
- Implementation: `impl/arceos_ex/src/objects/kernel_addr_space.rs`
