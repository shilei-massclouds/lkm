# EarlyVm

`EarlyVm` 是入口前导期后半段使用的 translation controller。它提供完整 `KernelImage` 的早期映射，
并通过 `FixMap` 的 FDT 槽位提供对完整 `RawDtb` 的临时访问；它不建立物理内存的最终线性映射，也不
代表后续 `SwapperVm` 承载的完整内核地址空间。

`EarlyVm.Preset` 收拢建立早期映射前必须成立的 `RawDtb` 与 `FixMap` 事实。`EarlyVm.Setup` 在这些
事实和 `KernelAddrSpace` 的早期布局成立后建立早期映射，使 controller 进入 Ready；这两个步骤都不
改变任何 CPU 当前使用的 controller。

`Vm.Setup` 调用 `EarlyVm.ActivateOnCpu(cpu_ref)`，把准确由 `TrampolineVm` 承载的目标 CPU 交给
`EarlyVm`。成功后该 CPU 可以继续普通入口执行并访问完整内核映像与 RawDtb；`EarlyVm` 保持 Ready，
并承载该 CPU。后续 `SwapperVm` 接管时如何处理 `EarlyVm`，留给 `Vm.Enable` 阶段校准。

## Mapping

- Model: `spec/model/objects/early_vm.spec`
- Coding: `spec/coding/objects/early-vm.md`
- Implementation: `impl/arceos_ex/src/objects/early_vm.rs`
