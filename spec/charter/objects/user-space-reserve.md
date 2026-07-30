# UserSpaceReserve

`UserSpaceReserve` 是 `KernelAddrSpace` 拥有的体系结构 canonical 用户虚拟地址预留区域。它表达不可被
内核映射占用的地址范围，不是用户进程地址空间，也不建立页表。

`UserSpaceReserve.Preset` 建立该预留范围，使 `KernelAddrSpace` 的其它区域以及后续
`SwapperVm` 映射都能验证与它互不重叠。本段只建立范围约束，不创建任何用户映射。

## Mapping

- Model: `spec/model/objects/user_space_reserve.spec`
- Coding: `spec/coding/objects/user-space-reserve.md`
- Implementation: `impl/arceos_ex/src/objects/user_space_reserve.rs`
