# BootInitFlow.Setup

Setup 从 `start_kernel()` 起直接编排入口 C 初始化与 `setup_arch()`：先推进根栈保护、EarlyDtb、
BootCPU、PrintkBuffer、InitMM、EarlyIoremap、SBI、早期参数、MemBlock、Vm/SwapperVm 和 EarlyDtb
退出，再按 sibling Linux 6.12 的顺序推进正式 DeviceTree、Zones/PageMetadataMap、ResourceTree、
CpuGroup、CacheBlockInfo、CpuCapabilities 与 DmaCachePolicy。`setup_arch()` 返回只是直接编排中的
校准边界，不产生新的 Phase、lifecycle 或 checkpoint。

返回后 Setup 顺序驱动 `CorePreparePhase`、`MmCoreInitPhase`、`SchedInitPhase`、`IrqTimeInitPhase`、
`LocalIrqEnablePhase`、`IrqOpenPreparePhase`、`ProcessPreparePhase` 和 `BootInitRestInitPhase`，随后提交
Ready。所有保留的叶子 parent 均直接是 BootInitFlow；物理目录中的 `boot` 只是 namespace。

## `start_kernel()` 到 `setup_arch()` 返回的顶层函数/系统清单

下表以 sibling Linux 6.12 提交 `a73fa7d6e` 的源码调用顺序和该工作树 `.config` 为核对基线。每个顶层
函数都对应一个明确的系统/对象责任；`setup_arch()` 对应 BootInitFlow 的嵌套编排本身，其内部顶层函数
继续逐项列出。一个系统可以由多个连续顶层函数推进，但不得用 wrapper Phase 代替这些系统，也不得
遗漏 deferred/trimmed 调用位置。

表中的 **trimmed** 表示当前配置或固定输入使调用编译为空、被条件编译删除或确定不执行实质工作，
并必须对应 Model 的 `trimmed` 记录；**deferred** 只用于当前配置下未裁剪、真实调用仍存在但语义尚未
形式化的责任，并必须对应 Model 的 `deferred` 记录。**formal/推进型** 表示当前调用在本边界直接推进
对应系统；**formal/位置型** 表示调用位置和幂等关系已形式化，但系统 lifecycle 按 Charter 明确绑定到
另一处真实调用。`.config` 中未出现的布尔选项按未启用处理。

| # | Linux 顶层函数 | 对应系统/对象 | sibling `.config` / 实际路径 | Charter / Model 当前处理 |
| --- | --- | --- | --- | --- |
| 1 | `set_task_stack_end_magic()` | `Task`（`BootTask` 实例） | 无条件真实调用 | formal：Setup 第一项调用 `BootTask.EnableStackGuard`；Action 语义由 Task owner 定义 |
| 2 | `smp_setup_processor_id()` | `PlatformCpuInfo` / BootCPU identity | `CONFIG_SMP=y`，选择 RISC-V 实现 | formal：写入保存的 boot hartid，建立启动 CPU 平台身份 |
| 3 | `debug_objects_early_init()` | `DebugObjects` | `CONFIG_DEBUG_OBJECTS=n`，inline 空实现 | **trimmed**：`boot_init_setup.004`，处理一致 |
| 4 | `init_vmlinux_build_id()` | `KernelBuildId` | `CONFIG_STACKTRACE_BUILD_ID=n` 且 `CONFIG_VMCORE_INFO` 未启用，inline 空实现 | **trimmed**：`boot_init_setup.002`，不建立 build-id 发布责任 |
| 5 | `cgroup_init_early()` | `Cgroup` | `CONFIG_CGROUPS=n`，inline 空实现 | **trimmed**：`boot_init_setup.005`，处理一致 |
| 6 | `local_irq_disable()` | BootCPU 的 `InterruptType` | 无条件真实调用 | formal：关闭总门控并保持 early-IRQ-disabled 事实 |
| 7 | `boot_cpu_init()` | `CpuGroup.cpus[0]` | 无条件通用实现；当前 `CONFIG_SMP=y` | formal：推进 possible/present/active/online 语义 |
| 8 | `page_address_init()` | `PageAddressMetadata` | RISC-V 当前既无 `HASHED_PAGE_VIRTUAL` 也无 `WANT_PAGE_VIRTUAL`，展开为空宏 | **trimmed**：`boot_init_setup.003`，不建立 page-address metadata 责任 |
| 9 | `pr_notice(linux_banner)` | `PrintkBuffer` | `CONFIG_PRINTK=y`，真实 banner write | formal action：Preset 后写入启动 banner |
| 10 | `setup_arch()` | `BootInitFlow` | RISC-V 无条件架构入口 | formal：直接嵌套编排；自身不创建 Phase、state 或 checkpoint |
| 10.1 | `parse_dtb()` | `EarlyDtb` | `CONFIG_OF_EARLY_FLATTREE=y` | formal：建立平台/物理内存、raw command line 与 MemBlock candidate facts |
| 10.2 | `setup_initial_init_mm()` | `InitMM` | `CONFIG_MMU=y`，真实调用 | formal：建立 init-mm 映像边界与 BootTask active-mm 关系 |
| 10.3 | `early_ioremap_setup()` | `EarlyIoremap` | `CONFIG_GENERIC_EARLY_IOREMAP=y` | formal：建立 FIX_BTMAP 临时映射服务 |
| 10.4 | `sbi_init()` | `SBI` | `CONFIG_RISCV_SBI=y` | formal：建立内核可见的 SBI capability view |
| 10.5 | `jump_label_init()` | `StaticBranch` | `CONFIG_JUMP_LABEL=y`；RISC-V `setup_arch()` 与其返回后的通用 `start_kernel()` 各有一次真实调用 | **formal/位置型**：保留第一次调用位置，`StaticBranch.Setup` 按主 Charter 绑定第二次调用；不是 trimmed/deferred，与 Model/CorePrepare 一致 |
| 10.6 | `parse_early_param()` | `Params` / `EarlyParam` | 真实调用；early-param 表可用 | formal：解析 early params，并驱动 `EarlyCon` |
| 10.7 | `efi_init()` | `EFI` | `CONFIG_EFI=y`，真实符号存在；运行期可因 FDT 无 EFI 参数提前返回 | **deferred**：`boot_init_setup.001`，enabled alternate path 分类一致 |
| 10.8 | `paging_init()` | `Vm` | `CONFIG_MMU=y`，真实调用 | formal：内部推进 MemBlock、SwapperVm、KernelAddrSpace、Vm 与 EarlyDtb 退出 |
| 10.9 | `acpi_boot_table_init()` | `AcpiBootTables` | `CONFIG_ACPI=n`，inline 空实现 | **trimmed**：`boot_init_setup.006`，处理一致 |
| 10.10 | `unflatten_device_tree()` | `DeviceTree` | `CONFIG_OF=y` 且 `CONFIG_BUILTIN_DTB` 未启用，选择本分支 | formal：建立正式 OF tree |
| 10.11 | `misc_mem_init()` | `Zones` / `PageMetadataMap` | `CONFIG_FLATMEM=y`；主线真实执行；`CONFIG_MEMTEST=y` 但固定输入无 `memtest=`，`SPARSEMEM`/`SPARSEMEM_VMEMMAP`/`CRASH_RESERVE` 均未启用 | formal 主线；内部 early-memtest、sparse、vmemmap flush、crashkernel 分别 **trimmed** 为 `boot_init_setup.007`–`.010`，处理一致 |
| 10.12 | `init_resources()` | `ResourceTree` | 无条件真实调用 | formal：经 `ResourceLock` write guard 建立资源树 |
| 10.13 | `kasan_init()` | `Kasan` | `CONFIG_KASAN=n`，调用被条件编译删除 | **trimmed**：`boot_init_setup.011`，处理一致 |
| 10.14 | `setup_smp()` | `CpuGroup` | `CONFIG_SMP=y`，调用被编入 | formal：建立 secondary CPU 候选与拓扑，不开放 SMP 并发 |
| 10.15 | `acpi_init_rintc_map()` | `AcpiRintcMap` | `CONFIG_ACPI=n`，`acpi_disabled=true` 且接口为空实现 | **trimmed**：`boot_init_setup.012`，处理一致 |
| 10.16 | `acpi_map_cpus_to_nodes()` | `AcpiCpuNumaMap` | `CONFIG_ACPI=n`、`CONFIG_NUMA=n`，`CONFIG_ACPI_NUMA` 未启用 | **trimmed**：`boot_init_setup.013`，处理一致 |
| 10.17 | `riscv_init_cbo_blocksizes()` | `CacheBlockInfo` | `CONFIG_RISCV_ISA_ZICBOM=y`、`CONFIG_RISCV_ISA_ZICBOZ=y`、`CONFIG_ACPI=n`，选择 DT 路径 | formal：发布 CBOM/CBOZ；CBOP 绑定细节 **deferred** 为 `boot_init_setup.014`，分类一致 |
| 10.18 | `riscv_fill_hwcap()` | `CpuCapabilities` | 无空实现配置分支，真实调用 | formal：汇总 all-harts common ISA/hwcap facts |
| 10.19 | `init_rt_signal_env()` | `RtSignalEnv` | RISC-V 真实实现存在，不因当前配置折叠为空 | **deferred**：`boot_init_setup.016`，分类一致 |
| 10.20 | `apply_boot_alternatives()` | `BootAlternatives` | `CONFIG_RISCV_ALTERNATIVE=y`，真实实现存在 | **deferred**：`boot_init_setup.015`，分类一致 |
| 10.21 | `riscv_noncoherent_supported()` | `DmaCachePolicy` | `CONFIG_RISCV_ISA_ZICBOM=y`、`CONFIG_RISCV_DMA_NONCOHERENT=y`；另有运行期 ZICBOM guard | formal 条件 action：发布 non-coherent support fact |
| 10.22 | `riscv_set_dma_cache_alignment()` | `DmaCachePolicy` | `CONFIG_RISCV_DMA_NONCOHERENT=y`，真实实现存在 | formal：收敛 dma-cache-alignment fact |
| 10.23 | `riscv_user_isa_enable()` | `UserIsaExposure` | `CONFIG_RISCV_ISA_ZICBOZ=y`，真实实现存在并按运行期 capability 设置 CBZE | **deferred**：`boot_init_setup.017`，分类一致 |

`setup_arch()` 返回后，上表中 formal 系统必须已完成其目标状态；BootInitFlow 仍为 Prepared，
CorePreparePhase 仍为 Base。PerCpuStorage、普通 Boot/Payload 参数解析与正式 ExceptionType Setup 尚未推进。

`BootInitRestInitPhase` 完整创建、发布 `KernelInitTask`/`KernelInitFlow` 和
`KthreaddTask`/`KthreaddFlow`。Task 发布时已是 Online/None/Valid，固定 Flow 也已 Online；首次派发不再
发送 Startup 或 Activate，而是恢复其首个 TaskThreadContext、提交 Task.Continue，再交付 contextual
TaskFlow.Continue。

## 引用

- [BootInitFlow](README.md)
- [BootInitFlow.Setup model](../../../model/flows/boot_init_flow/main.spec)
- [BootInitFlow.Setup coding](../../../coding/flows/boot_init_flow/setup.md)
