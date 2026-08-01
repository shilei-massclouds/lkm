# BootInitFlow.Setup direct-object coding

本文件约束 `BootInitFlow.Setup` 从 `start_kernel()` 到 sibling Linux 6.12 `setup_arch()` 返回的直接对象
编排。实现落点是 `impl/arceos_ex/src/flows/boot_init_flow/setup.rs` 私有 helper；它不是 PhaseObject，
不保存 state，不发出 Started/Prepared/Ready/Online checkpoint，也不拥有 sibling continuation。

## Linux 顶层函数映射

实现和审查必须以 Charter
[`start_kernel()` 到 `setup_arch()` 返回的顶层函数/系统清单](../../../charter/phases/boot-init-flow.md#start_kernel-到-setup_arch-返回的顶层函数系统清单)
为权威顺序。每个顶层函数映射到明确系统/对象；trimmed/deferred 函数保留结构化位置事实，不生成
虚构 lifecycle。`setup_arch()` 只代表 BootInitFlow 的嵌套编排范围。

## 直接 drives

`start_kernel()` 的 Setup guard 通过后，helper 严格按模型顺序推进：

| # | Model drive | Impl |
| --- | --- | --- |
| 1 | `BootTask.Action::EnableStackGuard` | `ctx.init_stack.enable()` |
| 2 | `EarlyDtb.Preset` | `ctx.early_dtb.preset(...)` |
| 3 | `CurrentCPU.Enable` | `ctx.cpu_group.enable_boot_cpu()` |
| 4 | `PrintkBuffer.Preset` | `printk::preset()`；随后 banner write action |
| 5 | `EarlyDtb.Setup` | `ctx.early_dtb.setup(...)` |
| 6 | `InitMM.Setup` | `ctx.init_mm.setup(...)` |
| 7 | `EarlyIoremap.Setup` | `ctx.early_ioremap.setup(...)` |
| 8 | `SBI.Setup` | `ctx.sbi.setup()` |
| 9 | `Params.Preset` | `ctx.params.preset(...)` |
| 10 | `MemBlock.Setup` | `ctx.memblock.setup(...)` |
| 11 | `Vm.Enable` | `ctx.vm.enable(...)` |
| 12 | `MemBlock.Enable` | `ctx.memblock.enable(...)` |
| 13 | `EarlyDtb.Cleanup` | `ctx.early_dtb.cleanup(...)` |
| 14 | `DeviceTree.Setup` | `ctx.device_tree.setup(...)` |
| 15 | `Zones.Setup` | `ctx.zones.setup(...)` |
| 16 | `PageMetadataMap.Setup` | `ctx.page_metadata_map.setup(...)` |
| 17–18 | `ResourceLock.Preset/Setup` | `preset_static()` / `setup()` |
| 19 | `ResourceTree.Setup` | `ctx.resource_tree.setup(...)` |
| 20 | `CpuGroup.Setup` | `ctx.cpu_group.setup_smp(...)` |
| 21 | `CacheBlockInfo.Setup` | `ctx.cache_block_info.setup(...)` |
| 22 | `CpuCapabilities.Setup` | `ctx.cpu_capabilities.setup(...)` |
| 23 | `DmaCachePolicy.Setup` | `ctx.dma_cache_policy.setup(...)` |

完成第 23 项后必须立即回到 BootInitFlow Setup continuation，由 BootInitFlow 发送
`CorePreparePhase.Preset`。不得在二者之间插入 Phase、Action、checkpoint 或公开函数边界。

## 边界 invariant

发送 `CorePreparePhase.Preset` 前，BootInitFlow 保持 Prepared、CorePreparePhase 保持 Base；完整内核
地址空间、MemBlock、正式 DeviceTree、zone/page metadata、resource、CPU topology、CBO/hwcap 和
DMA/cache policy 已完成。PerCpuStorage、StaticBranch、saved/static command line、普通参数、
PrintkBuffer.Setup、ExceptionTable 与正式 ExceptionType.Setup 均未推进。

start-kernel/EFI 与 setup-arch-tail 的 deferred/trimmed facts 归属 BootInitFlow.Setup。实现只需以私有
bitset/检查函数保存对应事实，不得为事实建立 wrapper state 或 checkpoint。
