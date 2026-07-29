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
已经存在于内存中的内核映像，其虚拟范围是自身属性；不再建立独立映像映射准对象。
`FixMap` 是固定虚拟槽位区域，当前由 FDT slot 为物理 `RawDtb` 提供临时映射。
`LinearMap` 是从 `PAGE_OFFSET` 开始的物理内存线性映射区域。
`UserSpaceReserve` 是体系结构 canonical 用户地址预留范围，内核区域和最终内核页表不得占用它。
`RawDtb` 是固件提供的物理 blob，不是 `KernelAddrSpace` 的子区域。

`KernelAddrSpace.Online` 表示最终 `SwapperVm` 映射已发布到这一地址空间；它不表示所有 CPU 已经
切换 SATP。`Vm.Online` 是控制面已经发布最终 controller 的全局边界，也不代替 CPU-local owner。

## 每 CPU active translation owner

`CPU.active_translation_owner` 是可选的 typed controller reference。已经执行内核入口的 CPU 必须
恰有一个 owner；仅被发现、尚未进入内核的 AP 可以为空。`PhysicalDirect`、`TrampolineVm`、
`EarlyVm` 与 `SwapperVm` 是共享 translation controller：页表/控制器 Ready 状态是全局事实，
激活归属是每 CPU 事实。

四个 controller 使用统一的 `Action::TakeOver(cpu_ref)`：先验证 controller Ready、CpuRef 有效、
旧 owner 与 live SATP 一致，再按控制器规则写 SATP 并完成所需 fence，最后原子替换目标 CPU owner。
trace 必须记录 canonical CPU、旧/新 owner、写入的 SATP 与同步事实。失败必须在 SATP 和 owner 修改前
记录诊断并 fail-stop；旧 controller 只是不再被该 CPU 使用，不进入 Cleanup/Destroyed。

BP 的 owner 链是 `PhysicalDirect -> TrampolineVm -> EarlyVm -> SwapperVm`；AP 的链是
`PhysicalDirect -> TrampolineVm -> SwapperVm`。`EarlyVm.TakeOver` 完成时 trampoline 对当前 CPU
同步退役，但静态 trampoline 页表保持 Ready，供以后 AP 复用。`EarlyVm` 同样保持 Ready；BP 切换
到 SwapperVm 不销毁共享 controller。

## KernelImage 与构建产物命名

`KernelImage` 仅表示入口时已经加载在内存中的映像，不表示磁盘文件、ELF 文件或 build pipeline
artifact。构建层事实分别称为 kernel ELF build fact 与 kernel boot artifact build fact。
`KernelImageFile` 保留给未来可能建模的文件对象；本轮不创建该类型、实例或生命周期。

## RawDtb 边界

`RawDtb` 只验证入口物理地址非零、固定 header 范围可访问、magic、`total_size` 至少覆盖 header、
加法不溢出，并派生完整物理范围供 FixMap 容量检查。完整 blob 的可访问性来自 OpenSBI handoff
契约，不伪装成逐字节物理探测。`RawDtb` 不解析 `/memory`、`/cpus` 或其它节点；节点解析仍属于
后续 `EarlyDtb`/`DeviceTree` 对象。

## Mapping

- Model: `spec/model/phases/boot-init/preset.spec`
- Coding: `spec/coding/phases/boot-init/preset.md` 与 `spec/coding/riscv64.md`
- Implementation: `impl/arceos_ex/src/objects/kernel_addr_space.rs`、`vm.rs` 及各 controller module
