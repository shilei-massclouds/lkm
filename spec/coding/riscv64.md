# RISC-V64 Coding 规格

本文记录把当前模型规格落到 RISC-V64 内核实现时需要遵守的补充约束。

## 适用范围

本文件补充 `spec/model` 中与 RISC-V64 启动、CSR、页表、SBI、FDT 和中断异常入口相关的编码约束。它不重新定义对象生命周期，也不改变 `Preset`、`Setup`、`Enable`、`Cleanup` 的状态迁移语义。

第一轮实现只支持 RISC-V64，不要求提供其它架构的兼容层或空实现。

## 启动入口

- 内核入口应显式接收并保存启动 ABI 传入的 `a0` 和 `a1`。
- `a0` 对应 boot hart id，进入模型中的 `BootArgs.boot_hartid`。
- `a1` 对应 DTB 物理地址，进入模型中的 `BootArgs.dtb_pa`。
- 入口代码不得跳过 `BootArgs` 抽象直接让后续对象长期依赖裸寄存器值。
- 链接脚本必须显式提供 `__global_pointer$`，并保证 `_start` 同时是内核 text 起点和 ELF entry。

## 入口前导期地址访问纪律

RISC-V64 入口前导期实现必须按地址空间阶段区分可执行代码位置和可访问数据范围。

1. MMU 关闭阶段：
   - 入口最前段代码应集中放在内核映像起始处的 boot/head text 区域。实现可采用 `.head.text`、`.text.boot` 或其它等价段名，但链接脚本必须保证该区域紧接 `_start`，且位于固件跳入后可直接取指的物理映像范围内。
   - 该阶段访问内核符号必须使用 PC-relative 或等价 position-independent 方式。若使用 Rust/C 函数，必须确保对应编译选项和代码形态不会生成依赖最终虚拟地址的绝对寻址。
   - 该阶段不得访问普通 `.data/.bss/.init.data`，除非该访问已经被证明使用当前物理地址语义且不依赖尚未启用的虚拟映射。
   - 该阶段函数不得引入 ftrace、sanitizer、coverage、stack protector 或其它可能访问普通数据段或运行时设施的 instrumentation，除非逐项证明其访问路径满足本阶段约束。
2. trampoline 临时映射阶段：
   - 切换到 trampoline 页表后，只允许执行 trampoline 映射覆盖的代码，且代码应尽快切换到 `EarlyVm` 页表。
   - 该阶段不得访问普通 `.data/.bss/.init.data`，不得调用可能访问普通数据段、锁、日志、allocator 或未映射静态对象的函数。
   - checkpoint hook 若位于该阶段，必须是极小的 head/trampoline-safe 实现。
3. EarlyVm 阶段：
   - `EarlyVm` 必须至少映射整个 `KernelImage`，包括入口后续需要访问的 `.text/.rodata/.data/.bss/.init.data` 等内核映像范围。
   - 进入 EarlyVm 后应重新设置 `gp = __global_pointer$` 的当前虚拟地址，使 `gp-relative` 访问重新有效。
   - 只有在该阶段之后，入口前导期代码才可以按普通早期虚拟地址访问内核静态数据。

该约束参考 Linux RISC-V64 的 `__HEAD`/`HEAD_TEXT_SECTION`、`setup_vm()` 的 `medany` 要求，以及 trampoline 到 early page table 的两次 `satp` 切换流程；`arceos_ex` 不要求复制 Linux 宏名，但必须提供等价的链接布局和访问纪律。

## 链接脚本参考建议

- `__global_pointer$` 的存在和入口可达性属于强制约束；其在链接脚本中的精确位置当前作为实现建议处理。
- 可参考 Linux RISC-V64 的链接布局，把 `__global_pointer$` 放在 `.sbss` 与 `.sdata` 之间。这有助于让 `gp` 相对寻址覆盖小 BSS 与小数据段，并进一步使 `.sbss`、`.sdata` 的相对顺序和距离约束在链接脚本中显式化。
- 若具体内核沿用不同布局，只要能证明 `gp` 初始化后的可寻址范围覆盖所有依赖 `gp` 相对寻址的符号，即可视为满足当前强制规格；后续若模型需要表达小数据段布局，可再把该建议提升为更精细的约束。

## CSR 与屏障

- CSR 访问应集中封装，调用点表达具体语义，例如关闭中断、设置 `stvec`、切换 `satp`。
- 页表切换相关代码必须显式处理 `sfence.vma` 要求。模型规格可以不逐条展开该细节，但实现规格要求保留该边界。
- 早期入口对中断 pending/enable 状态的防御性清理应对应 `InterruptStream.Preset` 或 `InterruptStream.Setup` 的实现边界。

## 地址空间与页表

- `TrampolineVm`、`EarlyVm`、`SwapperVm` 应在代码中保持可区分的实现边界。
- 第一轮应真实拆分入口前导期和入口后继期页表推进过程，而不是只把现有 boot page table 代码改名为多个模型事件。
- `EarlyVm` 的实现必须覆盖规格要求的 `KernelImage` 和 `RawDtb` 映射前提。
- `FixMap` 槽位布局应由配置或架构常量统一定义，不应在多个对象实现中分散硬编码。
- 第一轮 `Config.fixmap.fdt` 的 FDT 槽位容量按 2MiB 配置，用于覆盖 Linux RISC-V64 `FIX_FDT`/`FIX_FDT_SIZE` 级别的早期 FDT 映射窗口；`FixMap` 只能消费该配置并执行容量检查，不应自行定义槽位大小。
- 完整内核页表启用后，`EarlyVm` 退出服务应有明确的代码边界，对应 `EarlyVm.Cleanup`。

## FDT 与物理内存

- `RawDtb` 的物理地址、头部范围和完整范围应在进入后续解析前被记录并检查。
- OpenSBI 固件提供 RawDtb 已完整加载到 S-mode 可访问内存中的交付保证；实现仍应在 `RawDtb` 推进过程中逐步确认 header、magic、totalsize 和完整范围。
- `PhysicalMemory` 和 `PlatformCpuInfo` 不是准备期事实。它们来自 `RawDtb`，由入口后继期 `EarlyDtb.Preset` 解析 FDT `/memory` 与 `/cpus` 后建立并发布。
- `EarlyDtb` 只表示入口后继期短暂存在的早期解析服务，不能被实现为后续正式 DeviceTree 对象的无边界延续。实现中应区分 `EarlyDtb.Preset` 的基础平台事实抽取和 `EarlyDtb.Setup` 的命令行、MemBlock 候选区段等后续解析用途。
- `PhysicalMemory`、`PlatformCpuInfo`、`MemBlock` 第一轮应优先由实际 FDT 解析建立。若解析能力不足，应停止并报告缺口，不得静默回退到 QEMU virt 固定内存范围。

## 多核与内存模型

- 当前 RISC-V64 generic 平台按 UMA 架构建模，所有 CPU/hart 共享同一物理内存地址空间。
- 将来扩展多核时，应基于 SMP 架构推进 `CPUGroup`、secondary CPU、IPI、timer 和调度相关对象；不得引入 NUMA
  节点、本地内存距离或 per-node allocator 语义，除非模型规格先显式扩展。
- FDT 中解析出的多个 hart 只表示 SMP CPU 拓扑事实，不改变 `PhysicalMemory` 的 UMA 语义。

## SBI

- SBI 能力探测应集中形成 `SBI` 能力视图。
- `SBI` 对象负责记录能力事实，不把每一次具体 SBI 调用都建模为自身生命周期事件。
- Early console、timer、IPI、rfence 等对象应依赖 SBI 能力事实，而不是各自重复探测固件能力。
- 第一轮 `EarlyCon` 使用 SBI early console 后端，不继承 ArceOS RISC-V64 QEMU virt 当前的 NS16550 UART console 路径。

## Checkpoint Trace

- 状态一致点在实现中应映射为 checkpoint hook，默认实现为空。
- checkpoint trace 是独立路径，不属于 `EarlyCon` 或正式 `Console`，不得依赖 allocator、锁、字符串缓冲区、FixMap 或线性映射已经可用。
- RISC-V64 第一轮可提供 SBI legacy putchar 单字符后端，用于极早期定位；该后端只输出稳定 checkpoint id 对应的一个字节。
- 单字符 trace 只用于调试和状态差分采集的最低层观测，不改变对象状态，也不能作为规格事件或状态迁移的组成部分。
- 页表切换前后的 checkpoint hook 必须显式考虑当前代码地址是否已被正在使用的页表覆盖；必要时使用极小的 inline/boot text 实现。

## RISC-V64 Generic 平台

`arceos_ex` 使用新的 RISC-V64 generic SBI/FDT 平台实现，而不是复用现有 `riscv64-qemu-virt` 平台实现。

该平台的定位是：

- 当前以 QEMU virt 作为前期测试目标。
- 长期面向遵守启动 ABI、SBI 和 FDT 描述约束的 RISC-V64 平台。
- boot hart、DTB 地址来自启动 ABI。
- CPU 描述、物理内存、reserved-memory、chosen bootargs 等事实来自 FDT。
- timer、IPI、HSM、shutdown、early console 等能力来自 SBI 能力视图。
- 不把 QEMU virt 的 UART、PLIC、固定 RAM range 或 MMIO range 作为默认语义硬编码；需要使用时应来自 FDT 或明确的配置事实。

## 待补充

- 具体页表模式和地址布局常量。
- RISC-V64 链接脚本符号列表。
- QEMU virt 与真实硬件之间的差异处理。
