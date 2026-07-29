# KernelAddrSpace 与启动期 translation controller

`KernelAddrSpace` 是 `Kernel` 直接拥有的唯一内核地址空间资源。它把地址布局与映射区域的
长期事实同写 SATP 的控制面分开：`Vm` 只协调 translation controller，不再代表地址空间本体。

```text
Kernel
|- KernelAddrSpace
|  |- KernelImage
|  |- FixMap
|  |- LinearMap
|  `- UserSpaceReserve
|- Vm
|  |- PhysicalDirect
|  |- TrampolineVm
|  |- EarlyVm
|  `- SwapperVm
|- Soc
`- CpuGroup
```

`KernelAddrSpace.Ready` 表示四个区域的布局、范围和互不冲突约束成立。`KernelImage` 是固件装载后
已经存在于内存中的内核映像，其对象语义由 [`kernel-image.md`](kernel-image.md) 规定；本文件只规定
它是 `KernelAddrSpace` 拥有的区域及其布局关系，不再建立独立映像映射准对象。
`FixMap` 是固定虚拟槽位区域，当前由 FDT slot 为物理 `RawDtb` 提供临时映射。
`LinearMap` 是从 `PAGE_OFFSET` 开始的物理内存线性映射区域。
`UserSpaceReserve` 是体系结构 canonical 用户地址预留范围，内核区域和最终内核页表不得占用它。
`RawDtb` 是固件提供的物理 blob，不是 `KernelAddrSpace` 的子区域。

`KernelAddrSpace.Online` 表示最终 `SwapperVm` 映射已发布到这一地址空间；它不表示所有 CPU 已经
切换 SATP。`Vm.Online` 是控制面已经发布最终 controller 的全局边界，也不代替任何 CPU-local
activation。controller 对象和它们的页表全局共享；activation relation、live SATP、同步完成事实与
activation journal 都按 CPU 独立。

## 每 CPU active translation controller

`CPU.active_translation_controller` 是可选的 typed association。已经执行内核入口的 CPU 必须
恰有关联一个 controller；仅被发现、尚未进入内核的 stopped AP 必须为 absent，而且不得预置该 CPU
的 live SATP。AP 启动协议可以预先发布入口预期使用的 SATP 值，但该值在 AP 真正进入架构入口前只是
boot-data expectation。`PhysicalDirect`、`TrampolineVm`、`EarlyVm` 与 `SwapperVm` 是共享
translation controller：页表/控制器 Ready 状态是全局事实，激活关联是每 CPU 事实。

四个 controller 使用统一的 `Action::ActivateOnCpu(cpu_ref)`。当 association absent 时，action kind
必须是 `InitialActivation`，新 controller 必须是 `PhysicalDirect`，并验证入口 live SATP 为零；当
association present 时，action kind 必须是 `Handoff`，并验证准确的旧 controller 与 live SATP
一致。随后 action 按目标 controller 规则写 SATP、完成所需 fence，并把 kind、canonical CPU、
optional old controller、新 controller、写入的 SATP 与同步事实作为一个原子 activation commit
发布。失败必须在 SATP、association、journal 和 committed count 出现任何部分提交前记录诊断并
fail-stop；旧 controller 只是不再被该 CPU 使用，不进入 Cleanup/Destroyed。

BP 在 Kernel 真实入口接受边界建立
`absent -InitialActivation-> PhysicalDirect -Handoff-> TrampolineVm -Handoff-> EarlyVm -Handoff-> SwapperVm`。
AP 在真实 `ApEntryPreludePhase` 架构入口建立
`absent -InitialActivation-> PhysicalDirect -Handoff-> TrampolineVm -Handoff-> SwapperVm`，不经过
EarlyVm。EarlyVm activation 完成时 trampoline 只对该 CPU 退役，但静态 trampoline 页表保持
Ready，供以后 AP 复用。EarlyVm 同样保持 Ready；BP 切换到 SwapperVm 不销毁共享 controller。
任何 controller、`KernelAddrSpace` 或 `Vm` 的 Ready/Online 聚合状态都不能推出某个 CPU 已激活它。

## KernelImage ownership 与构建产物命名

`KernelImage` 的访问机制和 lifecycle 由独立对象规格规定。本文件只保留命名边界：它不表示磁盘文件、
ELF 文件或 build pipeline artifact。构建层事实分别称为 kernel ELF build fact 与 kernel boot artifact build fact。
`KernelImageFile` 保留给未来可能建模的文件对象；本轮不创建该类型、实例或生命周期。

## RawDtb 边界

`RawDtb` 只验证入口物理地址非零、固定 header 范围可访问、magic、`total_size` 至少覆盖 header、
加法不溢出，并派生完整物理范围供 FixMap 容量检查。完整 blob 的可访问性来自 OpenSBI handoff
契约，不伪装成逐字节物理探测。`RawDtb` 不解析 `/memory`、`/cpus` 或其它节点；节点解析仍属于
后续 `EarlyDtb`/`DeviceTree` 对象。

## Mapping

- Model: `spec/model/objects/kernel_image.spec` 与 `spec/model/phases/boot-init/preset.spec`
- Coding: `spec/coding/objects/kernel-image.md`、`spec/coding/objects/kernel-address-space.md` 与
  `spec/coding/phases/boot-init/preset.md`
- Implementation: `impl/arceos_ex/src/objects/kernel_addr_space.rs`、`vm.rs` 及各 controller module
