# Vm Coding

`Vm` 必须由独立控制面类型承载，并与 `KernelAddrSpace` 分开保存。它拥有四个共享 translation
controller，但只协调准备与每 CPU handoff；虚拟区域范围仍由 `KernelAddrSpace` 及其子对象保存。

`preset` 对应 Linux RISC-V `setup_vm(dtb_pa)` 的语义边界：依次准备 TrampolineVm、RawDtb/FixMap、
KernelAddrSpace 与 EarlyVm。调用前后当前 CPU 必须仍由 PhysicalDirect 承载且 live `satp == 0`；任何
页表 helper、Rust 状态提交或 checkpoint 都不得提前写 `satp`。

`setup` 对应 Linux RISC-V `relocate_enable_mmu(early_pg_dir)` 的执行切换，必须作为一次不允许普通
Rust/C 介入的汇编 handoff 链完成：

1. 在 PhysicalDirect 下完成两次 activation 所需的全部可失败校验，计算虚拟 continuation、保护入口
   虚拟地址、trampoline SATP 与 early SATP；
2. 临时把 `stvec` 写为 trampoline 可达的虚拟 continuation，执行 `sfence.vma`，写 trampoline SATP，
   并提交 `TrampolineVm.ActivateOnCpu`；
3. 到达虚拟 continuation 后，把 `stvec` 恢复为 `TrapType.Preset` 的汇编保护入口，并重新装载
   `gp/x3`，确保下一次普通全局数据访问前已恢复相对寻址基准；
4. 写 early SATP，执行 `sfence.vma`，提交 `EarlyVm.ActivateOnCpu`，然后才返回普通入口代码。

汇编提交期间不得调用 Rust/C、日志、allocator 或 checkpoint。进入提交段前的失败不得改变 live
`satp`、`stvec`、association 或 journal；进入后若硬件事实与预计算不一致必须 fail-stop，不能带着
TrampolineVm 的半完成上下文返回。成功时仅目标 CPU 改为 EarlyVm，`Vm` 提交 Ready，且 `stvec`
仍指向原保护入口。

本段不 lower `Vm.Enable`，也不得在 `setup` 中建立或激活 `SwapperVm`。Vm.Setup 为使地址切换
continuation 可执行而临时调整 live `sp` 时，该调整只属于 handoff transport，不发布 CurrentStack
binding，也不能被记作 `CurrentTask.RefreshTaskStack` 已完成。BootTask 的正式 `tp/sp` pair 刷新仍属于
后续独立的 `CurrentTask.RefreshTaskStack` 汇编提交块；该块必须在 TrapType.Setup 后重新装载两者。

后续阶段既有的跨-controller 约束继续保留：BP 完整链为
`PhysicalDirect -> TrampolineVm -> EarlyVm -> SwapperVm`，AP 链为
`PhysicalDirect -> TrampolineVm -> SwapperVm`；每次汇编 activation 都要在写 receipt 前核对旧
association、activation kind 和 committed count。根回归的反汇编检查最终必须覆盖真实
`csrw satp`、`sfence.vma`、receipt、association 与 release committed-count 的顺序。本段只校准前两次
BP handoff，不提前校准 SwapperVm。
