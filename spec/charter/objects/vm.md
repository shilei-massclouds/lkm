# Vm

`Vm` 是 `Kernel` 直接拥有的地址转换控制面，用于协调执行环境在不同 translation controller 之间的
切换；它不是内核地址空间本体，也不拥有 `KernelAddrSpace` 中的虚拟地址区域。`Vm` 拥有共享的
`PhysicalDirect`、`TrampolineVm`、`EarlyVm` 与 `SwapperVm`。controller 的准备状态是全局事实，某个
controller 是否正在承载执行则是所属 CPU 的局部事实。

`Vm.Preset` 准备入口前导期所需的地址转换环境。它协调 `KernelAddrSpace`、`RawDtb`、`FixMap`、
`TrampolineVm` 与 `EarlyVm` 完成各自的准备工作，但不改变当前 CPU 正在使用的 controller；成功后
`Vm` 进入 Prepared。

`Vm.Setup` 只在 `Preset` 已完成且当前 CPU 仍由 `PhysicalDirect` 承载时执行。它先让
`TrampolineVm` 接管当前 CPU，保证执行能够跨越物理地址到虚拟地址的边界，再让 `EarlyVm` 接管，
使后续入口代码可以访问完整 `KernelImage` 和经 `FixMap` 映射的 `RawDtb`。第一次切换期间可以临时
借用 `TrapType.Preset` 已占用的 `stvec` 入口位置，把它暂时指向地址转换 continuation，但必须在继续
普通启动执行前恢复为原保护入口；这不推进 `TrapType` 生命周期。成功后 `Vm` 进入 Ready，两个
controller 仍保持可复用，
其它 CPU 不受影响。

`Vm.Enable` 留给后续建立并启用 `SwapperVm` 的阶段；本段不补充其具体语义。`Vm.Ready` 只表示启动
CPU 已进入 EarlyVm，不表示最终完整内核页表已经建立。

## Mapping

- Model: `spec/model/objects/vm.spec`
- Coding: `spec/coding/objects/vm.md`
- Implementation: `impl/arceos_ex/src/objects/vm.rs`
