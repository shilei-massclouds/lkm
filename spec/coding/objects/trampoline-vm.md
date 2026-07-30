# TrampolineVm Coding

`TrampolineVm` 必须由独立静态页表存储和 controller 实现边界承载。`setup` 只建立从当前物理执行点
跨到虚拟 continuation 所需的最小映射并提交 Ready，不写 `satp`，也不发布 CPU association。

`ActivateOnCpu(cpu_ref)` 只接受准确由 `PhysicalDirect` 承载且 live `satp == 0` 的 CPU。进入不可回退的
提交段前必须完成所有可失败校验，并预先算好 trampoline SATP、虚拟 continuation 和保护入口的虚拟
地址。RISC-V 提交顺序必须是：

1. 临时把 `stvec` 指向 trampoline 映射覆盖的虚拟 continuation；
2. 执行 `sfence.vma`，再以 `csrw satp, trampoline_satp` 切换页表；
3. 在 trampoline 映射中的虚拟 continuation 继续执行，验证 live SATP；
4. 记录并原子发布 `PhysicalDirect -> TrampolineVm` 的 per-CPU Handoff receipt。

该临时 `stvec` 只服务地址转换边界，不推进 `TrapType` 生命周期。保护入口的恢复由同一外层
`Vm.Setup` 在继续普通代码前完成。CPU 离开后静态页表和 Ready 状态继续保留，不能产生 Cleanup、
Destroyed 或全局 Online。

稳定规则 ID（MUST）：

- `riscv64_must_trampoline_vm_activation_flush_tlb_before_satp`
- `riscv64_must_trampoline_vm_activation_commit_per_cpu_sync_fact`

Rust 的完成接口只能核对汇编已经提交的完整 receipt、association 与 live SATP，再发布
`trampoline_vm_translation_sync_complete`；不得二次写 association、追加 receipt 或覆盖 trace。
