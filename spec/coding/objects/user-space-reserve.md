# UserSpaceReserve Coding

`UserSpaceReserve` 必须由独立区域类型保存 RISC-V canonical 用户虚拟地址范围，并作为
`KernelAddrSpace` 的子区域参加不重叠验证。它不是用户进程地址空间，也不拥有页表。

`preset` 只发布该范围已保留以及内核区域不得占用它。不得在本对象中创建用户 PTE、用户 address-space
实例或进程生命周期；后续 `SwapperVm` 的任何内核映射也必须重新验证不进入该范围。
