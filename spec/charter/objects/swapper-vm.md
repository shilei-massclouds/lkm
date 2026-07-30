# SwapperVm

`SwapperVm` 是承载最终完整内核页表的 translation controller。它与 `TrampolineVm`、`EarlyVm` 一样
是由 `Vm` 拥有的共享 controller，但它依赖后续启动阶段建立物理内存线性映射和其它最终内核映射。

入口前导期只保留 `SwapperVm` 的独立系统身份，不建立或激活它。`Vm.Enable` 将在后续校准中负责
协调 `SwapperVm` 的建立和当前 CPU 的切换；本段不提前定义该 lifecycle 的具体条件和结果。

## Mapping

- Model: `spec/model/objects/swapper_vm.spec`
- Coding: `spec/coding/objects/swapper-vm.md`
- Implementation: `impl/arceos_ex/src/objects/swapper_vm.rs`
