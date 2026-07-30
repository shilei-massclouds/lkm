# SwapperVm Coding

`SwapperVm` 必须保留独立的 controller 类型、静态页表存储和实现文件，不能与 `EarlyVm` 合并，也不能
以 `KernelAddrSpace` 的 Ready 状态替代。

本入口前导段不得建立、激活或发布 SwapperVm，也不得从 `Vm.Preset` 或 `Vm.Setup` 写入 swapper SATP。
其页表建立、最终映射权限、per-CPU activation 和同步规则留给后续 `Vm.Enable` 校准；现有实现路径在
该校准完成前不能反向决定本段语义。

下列既有 RISC-V lowering 约束保留在本系统文件中，供后续 `Vm.Enable` 校准使用，本段不据此发送
SwapperVm Signal：

- `riscv64_must_swapper_vm_activation_flush_tlb_after_satp`（MUST）
- `riscv64_must_swapper_vm_activation_commit_per_cpu_sync_fact`（MUST）

后续 `ActivateOnCpu` 应在写 swapper SATP 后执行本地 `sfence.vma`，再验证 live SATP、提交 per-CPU
receipt/association，并由 Rust 只读核对后发布 `swapper_vm_translation_sync_complete`。这一边界对应
Linux RISC-V `setup_vm_final()` 的最终 `local_flush_tlb_all()`，不是本段 `Vm.Setup` 的组成部分。
