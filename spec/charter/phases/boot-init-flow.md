# BootInitFlow

`BootInitFlow` 是静态 `BootTask.flow` 指向的终身 TaskFlow。它从 `_start` 编排 boot execution，提交
Online 后仍承载同一 PID 0 的 idle setup、首次 Schedule 返回、`BootIdleEntryPhase` 和 idle loop。
不存在第二个 boot idle Flow；`BootTask` 首次真实切出时才保存 context并进入 Online，未来通过统一
Continue 返回同一个 BootInitFlow continuation。

## 入口与 Preset

OpenSBI 发出 `Kernel.Enable` 后，Kernel 在 Ready 的同一 handler 中依次完成：

1. `Kernel.Action::AcceptEnable`；
2. `BootInitFlow.Action::AssignCpuRef(BootCPURef)`；
3. `PhysicalDirect.Action::ActivateOnCpu(BootCPURef)` 的 InitialActivation；
4. canonical `BootInitFlow.Transition::Preset`。

Preset 接受时 Flow 必须 Base、parent 必须是 OnCpu/Live BootTask；`Started` 只是在第一个 child 前记录的
checkpoint。入口依次完成 BootCPU interrupt route mask/pending clear、浮点/向量关闭、BSS 清零、hartid
记录、PhysicalDirect 下 boot-only `CurrentTask.BindTaskStack(BootTask, BootTask.stack)`、临时 trap、
Vm.Preset/Setup、正式 TrapType.Setup、EarlyVm 下 `RefreshTaskStack`、Soc.Preset，最后提交 Prepared。
失败不得部分提交 Flow、CurrentTask/CurrentStack 或 translation controller。

## Setup 与发布

Setup 从 `start_kernel()` 起直接编排入口 C 初始化与 `setup_arch()`：先推进根栈保护、EarlyDtb、
BootCPU、PrintkBuffer、InitMM、EarlyIoremap、SBI、早期参数、MemBlock、Vm/SwapperVm 和 EarlyDtb
退出，再按 sibling Linux 6.12 的顺序推进正式 DeviceTree、Zones/PageMetadataMap、ResourceTree、
CpuGroup、CacheBlockInfo、CpuCapabilities 与 DmaCachePolicy。`setup_arch()` 返回只是直接编排中的
校准边界，不产生新的 Phase、lifecycle 或 checkpoint。

返回后 Setup 顺序驱动 `CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase`、
`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase` 和 `BootInitRestInitPhase`，随后提交
Ready。所有保留的叶子 parent 均直接是 BootInitFlow；物理目录中的 `boot` 只是 namespace。

### `start_kernel()` 到 `setup_arch()` 返回的顶层函数/系统清单

下表以 sibling Linux 6.12 的源码调用顺序为准。每个顶层函数都对应一个明确的系统/对象责任；
`setup_arch()` 对应 BootInitFlow 的嵌套编排本身，其内部顶层函数继续逐项列出。一个系统可以由多个
连续顶层函数推进，但不得用 wrapper Phase 代替这些系统，也不得遗漏 deferred/trimmed 调用位置。

| # | Linux 顶层函数 | 对应系统/对象 | 本轮边界处理 |
| --- | --- | --- | --- |
| 1 | `set_task_stack_end_magic()` | `InitStack` / `BootTask.stack` guard | formal：`BootTask.EnableStackGuard` |
| 2 | `smp_setup_processor_id()` | `PlatformCpuInfo` / BootCPU identity | formal：写入保存的 boot hartid，建立启动 CPU 平台身份 |
| 3 | `debug_objects_early_init()` | `DebugObjects` | trimmed：sibling `CONFIG_DEBUG_OBJECTS=n` |
| 4 | `init_vmlinux_build_id()` | `KernelBuildId` | deferred：保留 build-id 发布位置 |
| 5 | `cgroup_init_early()` | `Cgroup` | trimmed：sibling `CONFIG_CGROUPS=n` |
| 6 | `local_irq_disable()` | BootCPU 的 `InterruptType` | formal：关闭总门控并保持 early-IRQ-disabled 事实 |
| 7 | `boot_cpu_init()` | `CpuGroup.cpus[0]` | formal：推进 possible/present/active/online 语义 |
| 8 | `page_address_init()` | `PageAddressMetadata` | deferred：保留 page-address metadata 初始化位置 |
| 9 | `pr_notice(linux_banner)` | `PrintkBuffer` | formal action：Preset 后写入启动 banner |
| 10 | `setup_arch()` | `BootInitFlow` | 直接嵌套编排；自身不创建 Phase、state 或 checkpoint |
| 10.1 | `parse_dtb()` | `EarlyDtb` | formal：建立平台/物理内存、raw command line 与 MemBlock candidate facts |
| 10.2 | `setup_initial_init_mm()` | `InitMM` | formal：建立 init-mm 映像边界与 BootTask active-mm 关系 |
| 10.3 | `early_ioremap_setup()` | `EarlyIoremap` | formal：建立 FIX_BTMAP 临时映射服务 |
| 10.4 | `sbi_init()` | `SBI` | formal：建立内核可见的 SBI capability view |
| 10.5 | `jump_label_init()` | `StaticBranch` | 保留系统调用位置；本轮 lifecycle 由返回后的通用调用推进 |
| 10.6 | `parse_early_param()` | `Params` / `EarlyParam` | formal：解析 early params，并驱动 `EarlyCon` |
| 10.7 | `efi_init()` | `EFI` | deferred：sibling EFI enabled path 尚未形式化 |
| 10.8 | `paging_init()` | `Vm` | formal：内部推进 MemBlock、SwapperVm、KernelAddrSpace、Vm 与 EarlyDtb 退出 |
| 10.9 | `acpi_boot_table_init()` | `AcpiBootTables` | trimmed：sibling `CONFIG_ACPI=n` |
| 10.10 | `unflatten_device_tree()` | `DeviceTree` | formal：建立正式 OF tree；BUILTIN_DTB 分支未选中 |
| 10.11 | `misc_mem_init()` | `Zones` / `PageMetadataMap` | formal 主线，并保留 early-memtest、sparse/vmemmap/crashkernel 分支分类 |
| 10.12 | `init_resources()` | `ResourceTree` | formal：经 `ResourceLock` write guard 建立资源树 |
| 10.13 | `kasan_init()` | `Kasan` | trimmed：sibling `CONFIG_KASAN=n` |
| 10.14 | `setup_smp()` | `CpuGroup` | formal：建立 secondary CPU 候选与拓扑，不开放 SMP 并发 |
| 10.15 | `acpi_init_rintc_map()` | `AcpiRintcMap` | trimmed：sibling `CONFIG_ACPI=n` |
| 10.16 | `acpi_map_cpus_to_nodes()` | `AcpiCpuNumaMap` | trimmed：sibling `CONFIG_ACPI=n`、`CONFIG_NUMA=n` |
| 10.17 | `riscv_init_cbo_blocksizes()` | `CacheBlockInfo` | formal：发布 CBOM/CBOZ block-size facts |
| 10.18 | `riscv_fill_hwcap()` | `CpuCapabilities` | formal：汇总 all-harts common ISA/hwcap facts |
| 10.19 | `init_rt_signal_env()` | `RtSignalEnv` | deferred：用户 RT signal environment 尚未形式化 |
| 10.20 | `apply_boot_alternatives()` | `BootAlternatives` | deferred：保留通用 boot text-patch protocol 位置 |
| 10.21 | `riscv_noncoherent_supported()` | `DmaCachePolicy` | formal 条件 action：发布 non-coherent support fact |
| 10.22 | `riscv_set_dma_cache_alignment()` | `DmaCachePolicy` | formal：收敛 dma-cache-alignment fact |
| 10.23 | `riscv_user_isa_enable()` | `UserIsaExposure` | deferred：用户 ISA 暴露尚未形式化 |

`setup_arch()` 返回后，上表中 formal 系统必须已完成其目标状态；BootInitFlow 仍为 Prepared，
CorePreparePhase 仍为 Base。PerCpuStorage、普通 Boot/Payload 参数解析与正式 ExceptionType Setup 尚未推进。

`BootInitRestInitPhase` 完整创建、发布 `KernelInitTask`/`KernelInitFlow` 和
`KthreaddTask`/`KthreaddFlow`。Task 发布时已是 Online/None/Valid，固定 Flow 也已 Online；首次派发不再
发送 Startup 或 Activate，而是恢复其首个 TaskThreadContext、提交 Task.Continue，再交付 contextual
TaskFlow.Continue。

Enable 驱动 `BootInitScheduleHandoffPhase`，只完成 CPU0 Scheduler idle/curr metadata、runqueue 与首次
调度的可逆预检；不创建 Flow、active binding 或 dispatch kind。随后提交 BootInitFlow.Online。

## Online Actions、首次调度与 idle

BootInitFlow.Online 后按固定顺序执行同一 Flow 的 Actions：

1. 完成 boot idle setup 和退出 inherited preempt-disabled guard；
2. `yields CpuGroup.cpus[0].scheduler.Action::Schedule()`；
3. identity 时由目标完成后的通用 resume attempt 立即从 yields 后继续；
4. non-identity 时 Scheduler 显式保存 BootTask context、Task.Suspend、恢复 next context、提交 bindings 和
   next Task.Continue；BootInitFlow lane token 保持 pending；
5. 未来 Scheduler 恢复 BootTask 后，contextual BootInitFlow.Continue 校验 context epoch/token，先回到
   `schedule()` 返回 continuation，再驱动 `BootIdleEntryPhase` 和 idle loop。

本轮 canonical before-send 边界固定在第 2 步 token/Signal 尚未创建的位置。发送 Schedule 本身不改变
BootTask 或 Flow；只有 non-identity SwitchTo 的显式步骤改变 Task/CPU binding。`BootIdleEntryPhase`
现在是 BootInitFlow 的 Online child，而不是另一个 Flow 的子对象。

陷入期间 BootTask 保持 OnCpu、BootInitFlow 保持 Online；effective-flow 栈切到 Trap/Interrupt/
Exception leaf。若陷入中调度切出，恢复先落到该 leaf，再回到 BootInitFlow continuation。

## 生命周期与边界

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

每个 transition/action 都重新校验固定 parent、FlowRef/generation、CpuRef 与 effective-flow guard。
BootInitFlow 不退出，终身属于 BootTask。完整 SMP arbitration、迁移与 replay 保持 P2 延期。

## 引用

- [阶段范式](../phase-paradigm.md)
- [BootInitFlow model](../../model/phases/boot-init/phase.spec)
- [BootInitFlow coding](../../coding/phases/boot-init.md)
- [Kernel 系统](../systems/kernel.md)
