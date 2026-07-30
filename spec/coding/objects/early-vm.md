# EarlyVm Coding

`EarlyVm` 必须由独立静态页表存储和 controller 实现边界承载。`preset` 驱动 `RawDtb.Preset`、
`RawDtb.Setup` 与 `FixMap.Preset`，只汇集映射前提；`setup` 建立覆盖完整 `KernelImage` 以及 FDT fixed
slot 的页表映射并提交 Ready。两者都不得写 `satp` 或改变 CPU association。

`ActivateOnCpu(cpu_ref)` 只接受准确由 `TrampolineVm` 承载、且 live SATP 等于 trampoline SATP 的
CPU。RISC-V 提交段必须写入 early SATP，随后执行 `sfence.vma`，验证完整 KernelImage 与 FDT slot
已经可访问，再记录并原子发布 `TrampolineVm -> EarlyVm` 的 per-CPU Handoff receipt。成功只使
TrampolineVm 对该 CPU 退役，不清除其静态页表。

本对象不得建立最终 RAM 线性映射，也不得把 Ready 或 activation 解释为 `SwapperVm` 已建立。

稳定规则 ID（MUST）：

- `riscv64_must_early_vm_activation_flush_tlb_after_satp`
- `riscv64_must_early_vm_activation_commit_per_cpu_sync_fact`

Rust 的完成接口只能核对汇编已经提交的完整 BP receipt 链与 live SATP，再发布
`early_vm_translation_sync_complete`；不得重写该链。
