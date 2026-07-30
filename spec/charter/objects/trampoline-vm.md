# TrampolineVm

`TrampolineVm` 是从物理地址执行进入虚拟地址执行时使用的临时 translation controller。它只提供
跨越切换边界所需的最小可执行映射，不代表完整的早期内核地址空间，也不拥有被映射的
`KernelImage`。

`TrampolineVm.Setup` 建立这组最小映射并使 controller 可用于切换。`Vm.Setup` 随后调用
`TrampolineVm.ActivateOnCpu(cpu_ref)`，把准确由 `PhysicalDirect` 承载的目标 CPU 交给
`TrampolineVm`。切换必须能够在对应的虚拟 continuation 继续执行，并在继续普通启动路径前恢复
`TrapType` 的临时保护入口。

`EarlyVm` 接管 CPU 后，`TrampolineVm` 只是不再承载该 CPU；它的准备状态和静态映射继续保留，供
以后其它 CPU 的入口路径复用，不进入 Cleanup 或 Destroyed。

## Mapping

- Model: `spec/model/objects/trampoline_vm.spec`
- Coding: `spec/coding/objects/trampoline-vm.md`
- Implementation: `impl/arceos_ex/src/objects/trampoline_vm.rs`
