# arceos_ex 第一轮实现计划

本文记录 `arceos_ex` 第一轮实现任务清单。该计划用于把 `spec/model`、主规格文档和 coding 规格转化为 `tgoskits` 中可执行的实现步骤。

## 目标边界

第一轮目标是让 `arceos_ex` 以 ArceOS Unikernel 形态运行 `helloworld`，并完成当前模型中的两个子阶段：

- `EntryPreludePhase.Ready`
- `EntrySuccessorPhase.Ready`

最小可见结果是通过 SBI early console 打印 `Hello, world!`，随后关机。

## 入口命令

新增 `cargo xtask arceos-ex ...` 子命令。

第一轮优先支持：

```bash
cargo xtask arceos-ex test qemu --test-case helloworld --arch riscv64
```

该命令应复用 ArceOS 现有测试发现、构建和 QEMU 运行机制，但内部选择 `arceos_ex` 的 `_ex` 核心组件和 RISC-V64 generic 平台。

## 应用复用

第一轮复用现有 ArceOS Unikernel 应用，不默认复制应用源码。

优先目标：

- `test-suit/arceos/std/qemu-smp1/helloworld`

可作为参考的现有示例：

- `os/arceos/examples/helloworld`

应用通过 `ax-std` 正式接入，而不是直接依赖 `ax-runtime-ex`。

## 新增核心 crate

第一轮建议新增：

- `os/arceos_ex/modules/axhal`，包名 `ax-hal-ex`
- `os/arceos_ex/modules/axruntime`，包名 `ax-runtime-ex`
- `components/axplat_crates/platforms/axplat-riscv64-generic`，包名 `ax-plat-riscv64-generic`

若需要额外支撑 crate，应优先放在合适的 `components` 层级，并使用 `_ex` 后缀。不得修改现有 `os/arceos` 和已有 `components` crate 的行为。

## Cargo/xtask 策略

短期采用 `xtask` 驱动的 overlay workspace 方案，让现有 `ax-std` / `ax-api` / `ax-feat` 依赖链在 `arceos-ex` 构建中指向 `_ex` 核心组件。

`cargo xtask arceos-ex ...` 先在 `tmp/axbuild/arceos-ex-workspace/` 下生成独立的 `Cargo.toml`，并通过符号链接映射原仓库的 `os/`、`components/`、`drivers/`、`platform/`、`scripts/`、`test-suit/` 等源码目录。后续内部 Cargo 调用显式使用该 overlay manifest。顶层 `Cargo.toml` 不应被手工修改或替换。

已验证限制：

- Cargo 的 `--manifest-path` 目标文件名必须是 `Cargo.toml`。
- workspace member 必须位于 workspace root 之下，不能直接从临时 workspace 引用外部成员目录。
- 通过 overlay workspace 内部符号链接映射源码目录后，Cargo 可以把这些成员视为位于 overlay workspace 内部。

若未来 overlay workspace 无法稳定表达依赖切换，再讨论由 `xtask` 管理正式的多 workspace manifest 或受控顶层 manifest 切换。该过程必须由工具管理，不要求开发者手工来回修改顶层 `Cargo.toml`。

## 外部 crate 整改

`arceos_ex` 必须遵守 Rust coding 规格中的 crate 信任边界。当前实现中已发现的直接外部 crate 使用需要整改：

- `fdt-parser`：不得作为黑盒依赖保留。后续应改为本项目维护的最小 FDT 解析实现，或先把可参考源码引入
  `components/` 后审查、裁剪和改造。
- `sbi-rt`：不得作为 `SBI.setup()` 的实现依赖扩大使用范围。后续 SBI 能力视图优先由本项目维护的最小 SBI ecall
  wrapper 建立；现有 checkpoint SBI 字符输出和平台关机路径也应逐步收口到本项目维护的 SBI 封装。
- 对上述 crate 的传递依赖也必须按同一规则处理，不能留下未审查的黑盒依赖。

## 第一轮最小对象覆盖

实现必须覆盖当前模型中 `EntryPreludePhase` 和 `EntrySuccessorPhase` 所需对象。Phase 对象可以是编排过程；非 Phase 对象原则上应有 Rust struct、静态单例或启动上下文字段承载。

重点对象包括：

- `BootArgs`
- `BootCPU`
- `CpuIdMap`
- `RootStream`
- `InterruptStream`
- `KernelImage`
- `RawDtb`
- `PhysicalMemory`
- `PlatformCpuInfo`
- `InitTask`
- `InitStack`
- `Vm`
- `TrampolineVm`
- `EarlyVm`
- `SwapperVm`
- `FixMap`
- `EarlyDtb`
- `KernelCmdline`
- `KernelParam`
- `SBI`
- `PrintkBuffer`
- `EarlyCon`
- `MemBlock`
- `InitMM`
- `EarlyIoremap`

## RISC-V64 generic 平台任务

`ax-plat-riscv64-generic` 应以 SBI/FDT 为主要事实来源。

第一轮必须解析或建立：

- boot hart id：来自启动 ABI `a0`
- DTB 物理地址：来自启动 ABI `a1`
- CPU 描述：来自 FDT `/cpus`
- 物理内存：来自 FDT `/memory`
- bootargs：来自 FDT `/chosen`
- reserved-memory：仅处理当前启动闭环必要信息
- SBI 能力视图：至少覆盖 early console、timer、HSM/shutdown 相关能力边界
- 多 hart 平台按 UMA/SMP 处理：`/cpus` 描述 SMP CPU 拓扑，`/memory` 描述共享物理内存地址空间；第一轮不引入 NUMA 语义

第一轮不要求支持：

- initrd
- memory limit
- 多个 memory bank 的完整策略
- NUMA 节点、内存距离和 per-node allocator
- 非 QEMU 的板级差异处理

若 FDT 解析能力不足，应停止并报告缺口，不得静默回退到 QEMU virt 固定内存范围。

## 页表任务

必须严格遵循规格，实现并保持以下边界：

- `TrampolineVm`
- `EarlyVm`
- `SwapperVm`

第一轮不得把现有 boot page table 代码简单改名为多个模型事件。每次页表切换必须显式处理 RISC-V64 所需的 `sfence.vma` 边界。

## checkpoint

第一轮预留 checkpoint hook 接口，但不要求实现完整状态差分输出。hook 默认为空实现，可通过编译/链接选项接入具体 trace 后端。

checkpoint trace 独立于 `EarlyCon` 和正式 `Console`。当前最小后端可以使用 RISC-V64 SBI legacy putchar 输出单个字符，用于最早期启动定位；该路径不得依赖 allocator、锁、字符串地址、FixMap 或线性映射状态。

checkpoint 命名应沿用模型对象和状态名称，例如：

- `BootCPU.Prepared`
- `EarlyVm.Ready`
- `Vm.Online`
- `EntrySuccessorPhase.Ready`

## 待确认

- `ax-hal-ex` 与现有 `ax-hal` 的第一轮 public API 对照表。
- `ax-runtime-ex` 与现有 `ax-runtime` 的第一轮 public API 对照表。
- `xtask arceos-ex` 的具体参数和快照格式。
- overlay workspace 生成内容的最小成员集合和依赖替换表。
