# RISC-V64 Coding 规格

本文记录把当前模型规格落到 RISC-V64 内核实现时需要遵守的补充约束。

## 适用范围

本文件补充 `spec/model` 中与 RISC-V64 启动、CSR、页表、SBI、FDT 和中断异常入口相关的编码约束。它不重新定义对象生命周期，也不改变 `Preset`、`Setup`、`Enable`、`Cleanup` 的状态迁移语义。

## 启动入口

- 内核入口应显式接收并保存启动 ABI 传入的 `a0` 和 `a1`。
- `a0` 对应 boot hart id，进入模型中的 `BootArgs.boot_hartid`。
- `a1` 对应 DTB 物理地址，进入模型中的 `BootArgs.dtb_pa`。
- 入口代码不得跳过 `BootArgs` 抽象直接让后续对象长期依赖裸寄存器值。

## CSR 与屏障

- CSR 访问应集中封装，调用点表达具体语义，例如关闭中断、设置 `stvec`、切换 `satp`。
- 页表切换相关代码必须显式处理 `sfence.vma` 要求。模型规格可以不逐条展开该细节，但实现规格要求保留该边界。
- 早期入口对中断 pending/enable 状态的防御性清理应对应 `InterruptStream.Preset` 或 `InterruptStream.Setup` 的实现边界。

## 地址空间与页表

- `TrampolineVm`、`EarlyVm`、`SwapperVm` 应在代码中保持可区分的实现边界。
- `EarlyVm` 的实现必须覆盖规格要求的 `KernelImage` 和 `RawDtb` 映射前提。
- `FixMap` 槽位布局应由配置或架构常量统一定义，不应在多个对象实现中分散硬编码。
- 完整内核页表启用后，`EarlyVm` 退出服务应有明确的代码边界，对应 `EarlyVm.Cleanup`。

## FDT 与物理内存

- `RawDtb` 的物理地址、头部范围和完整范围应在进入后续解析前被记录并检查。
- `PhysicalMemory` 和 `PlatformCpuInfo` 这类准备期事实可以来源于 FDT/platform 描述，但代码中应区分“描述来源”和“使用该事实的对象”。
- `EarlyDtb` 只表示入口后继期短暂存在的早期解析服务，不能被实现为后续正式 DeviceTree 对象的无边界延续。

## SBI

- SBI 能力探测应集中形成 `SBI` 能力视图。
- `SBI` 对象负责记录能力事实，不把每一次具体 SBI 调用都建模为自身生命周期事件。
- Early console、timer、IPI、rfence 等对象应依赖 SBI 能力事实，而不是各自重复探测固件能力。

## 待补充

- 具体页表模式和地址布局常量。
- RISC-V64 链接脚本符号列表。
- QEMU virt 与真实硬件之间的差异处理。
