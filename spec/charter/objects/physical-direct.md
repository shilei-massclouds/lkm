# PhysicalDirect

`PhysicalDirect` 表示 CPU 在没有启用分页地址转换时直接按物理地址执行的 translation controller。
它不是一套页表，也不是另一个内核地址空间；它只描述进入内核时已经存在的直接执行环境。

CPU 首次进入内核时，通过 `PhysicalDirect.ActivateOnCpu(cpu_ref)` 建立该 CPU 的初始 controller 关联。
该动作只允许用于尚未关联 controller、且确实处于直接物理执行环境的 CPU。后续 `Vm.Setup` 从这一
明确起点把启动 CPU 交给 `TrampolineVm`；离开后 `PhysicalDirect` 本身不被销毁。

## Mapping

- Model: `spec/model/objects/physical_direct.spec`
- Coding: `spec/coding/objects/physical-direct.md`
- Implementation: `impl/arceos_ex/src/objects/physical_direct.rs`
