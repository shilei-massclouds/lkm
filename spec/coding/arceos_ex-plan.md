# arceos_ex 第一轮实现计划

本文记录 `arceos_ex` 第一轮实现任务清单。当前执行路线已经调整为：先在本仓库内直接完成对象级实现实验，源码放在
`impl/arceos_ex/`，使用 `Makefile` 编译和运行；暂时不进入 `tgoskits`、`xtask`、ArceOS crate 兼容和 feature 传递问题。

`tgoskits`/ArceOS 组件兼容属于后续 `Composition Phase`，只有对象级实现闭环后再恢复讨论。此前在
`tgoskits` 中实现过的 `arceos_ex` 仍具有参考价值，尤其是 RISC-V64 入口、链接脚本、checkpoint 字符输出、SBI/FDT
平台细节和 overlay 经验；但当前对象级实现不得直接继承其 ArceOS 组件边界、feature 传递、`axlog` 或 `ax-alloc` 接入。

## 目标边界

第一轮目标是让 `arceos_ex` 作为规格驱动的对象级内核原型运行，并完成当前模型中的两个子阶段：

- `EntryPreludePhase.Ready`
- `EntrySuccessorPhase.Ready`

最小可见结果是通过独立早期输出路径打印启动 banner 和 `Hello, world!`，随后通过 SBI 关机。

实现推进分为两个逻辑阶段：

- `Object Coding Phase`：优先完成对象级语义，包括对象状态、事件推进、依赖检查、checkpoint 和必要的最小运行路径。
- `Composition Phase`：在对象级语义明确之后，再整理 crate/module 边界、公开接口、adapter、overlay workspace 和与 ArceOS 组件体系的兼容关系。

当前只推进 `Object Coding Phase`。不得因为未来 ArceOS 组件封装需要而反向改变模型对象语义。

实现分层上，Phase 对象只作为过程编排存在；非 Phase 对象原则上应在 Rust 中有明确承载，例如 struct、静态单例或启动上下文字段。

## 当前执行计划

| 优先级 | 状态 | 任务 |
| --- | --- | --- |
| P0 | 完成 | 建立 `impl/arceos_ex/` 独立实验目录，包含 `Makefile`、RISC-V64 linker script、入口汇编和 no-alloc Rust 源码骨架。 |
| P0 | 完成 | 定义对象级公共基础：规范状态集合、事件集合、`EventResult`、生命周期事件唯一性检查和 checkpoint hook。 |
| P0 | 完成 | 分类 `make verify` 当前 obligations/deferred，明确进入 `EntryPreludePhase` 对象实现前的处理策略。 |
| P0 | 完成 | 纠正入口实现与规格顺序不一致的问题：`_start` 现在作为严格 head prefix，按规格顺序完成进入 Rust 前必须用汇编实现并由入口前导期拥有的 `EntryPreludePhase.Setup` 起始边界以及 `InterruptStream.Preset`、`KernelImage.Preset`、`RootStream.Preset`、`KernelImage.Setup`、`CpuGroup.Preset`、`InitTask.Preset`、`InitStack.Preset`；Rust 续段只认领这些状态并从 `EventStream.Preset` 继续。`StartupTimeline/PreparePhase/BootPhase` 等父阶段或准备期边界不由入口汇编代发 checkpoint，而是在各自映射实现中提交或采用 head-prefix adoption。 |
| P0 | 完成 | 实现 `EntryPreludePhase` 第一批不依赖页表切换的 foundation 对象：`InterruptStream.Preset`、`KernelImage.Preset/Setup`、`RootStream.Preset`、`BootCPU.Preset`、`CpuGroup.Preset`、`InitTask.Preset`、`InitStack.Preset`、`EventStream.Preset`。 |
| P0 | 完成 | 实现 `EntryPreludePhase` 最小闭环：`_start`、`__global_pointer$`、head text 布局约束、BootArgs、RootStream、KernelImage、BootCPU、InitStack、RawDtb、FixMap、TrampolineVm、EarlyVm、VM 三段切换；`make run LOG=trace` 已到达 banner 与本地 `app_main()`。 |
| P0 | 完成 | 实现 `EntrySuccessorPhase` 最小闭环：EarlyDtb、PlatformCpuInfo、PhysicalMemory、CpuIdMap、InterruptStream、BootCPU setup/enable、PrintkBuffer、KernelCmdline、KernelParam、SBI、EarlyCon、MemBlock、InitMM、EarlyIoremap、SwapperVm；`make run LOG=trace` 已到达 `EntrySuccessorPhase.Ready`。 |
| P0 | 完成 | 建立 no-alloc 输出路径：启动期内部 `printk`/`println-like` 前端和应用侧最小 `println!` 前端都写入 `PrintkBuffer`，再由 `EarlyCon(SBI)` drain。 |
| P1 | 完成 | 实现最小 FDT 解析，不引入外部 crate，不使用 `Vec`、`String`、`Box`；只解析当前闭环必要的 `/cpus`、`/memory`、`/chosen`、`/memreserve/` 和必要 `/reserved-memory`。 |
| P1 | 完成 | 用顶层 Makefile 提供 `build`、`run`、`verify`、`clean` 等入口，暂时脱离 `xtask`。 |
| P1 | 完成 | 对照 `startup-timeline.trace.svg` 和 checkpoint 输出逐段复查规格、推导和实现一致性；当前 `make verify REPORT=graph`、`make run LOG=trace` 和实现阶段顺序一致，运行期 checkpoint 单字符映射已修正为无重复。 |
| P1 | 待办 | 按 Object Coding Phase 映射规则整理源码结构：`main.rs` 承载 `startup-timeline`，`phases/` 按 Phase 包含层次拆分过程文件，`objects/` 按对象类别逐步拆成一对象一文件，并把资源对象统一收敛到全局 `Context`。 |
| P1 | 待办 | 将当前直接生成完整 `riscv64.lds` 的实验收敛为 Linux 风格的 `riscv64.lds.S` 方案：`codegen` 基于 `Config` 生成 `generated/config.lds.h` 等配置头，`.lds.S` 保留链接布局结构并通过预处理生成最终 `.lds`；同时规划生成 Rust 侧配置，避免 `.lds`、Rust 常量和 codegen profile 各自维护同一配置值。 |
| P1 | 完成 | 整理规格规则强度分层，为 `MUST`/硬约束、`SHOULD`/强建议、`MAY`/允许项和 `NOTE`/说明建立统一标注与解释规则，并把全局 `Context` 映射记录为 `SHOULD`。 |
| P1 | 待办 | 建立 GitHub Actions 快速 CI，覆盖推导工具质量、核心规格推导和 `impl/arceos_ex` 最小构建。 |
| P1 | 待办 | 建立 nightly/manual 测试流水线，生成 trace、系统测试日志、对象覆盖表和项目主页展示产物。 |
| P2 | 延期 | 组件封装阶段：恢复 ArceOS 组件接口、crate 边界、`ax-std` 接入、overlay workspace、`xtask`、feature 传递、`axlog` 和 `ax-alloc` facade 等问题。 |

## 入口命令

当前入口统一使用仓库顶层 `Makefile`，默认内核为 `arceos_ex`。

第一轮优先支持：

```bash
make build
make run
make run LOG=trace
make verify
make verify REPORT=graph
make clean
```

`KERNEL ?= arceos_ex` 选择默认内核。`build` 负责编译内核镜像；`run` 使用 QEMU/OpenSBI 运行；`run LOG=trace`
启用 checkpoint 字符输出。`verify` 调用 `pyveri` 对当前启动时间轴规格做推导验证；`verify REPORT=graph`
生成带注释的 trace SVG 报告。

当前对象级实现已经能通过 `make run` 和 `make run LOG=trace` 完成 `EntryPreludePhase.Ready` 与
`EntrySuccessorPhase.Ready`，输出启动 banner 和 `Hello, world!` 后通过 SBI 关机。

## `make verify` obligation 分类

`80966ec` 曾暴露 `13 obligation / 2 deferred`。这些条目不能作为实现可忽略的提示；处理原则是：实现某个对象事件前，必须先通过“推导义务门禁”。能由模型、推导工具、链接脚本、ISA、固件规范或已证明前序事实推出的，应优先补齐推导证明；只能由外部交付保证支撑的，应明确作为 source assumption，并在后续对象事件中尽快转化为运行期检查；无法归类的应作为规格缺口或显式 deferred。

当前已补齐 Lds、OpenSBI DTB handoff 和 BootCPU 前序事实的推导规则，`make verify` 报告为 `0 obligation / 2 deferred`。后续若再次出现 obligation，应先回到本节分类处理，不得直接继续实现。

| 分类 | 条目 | 影响范围 | 当前处理策略 |
| --- | --- | --- | --- |
| 固件交付假设，运行期逐步确认 | `firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)`；`firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)` | `OpenSbiFirmware.Online`、`RawDtb.Preset`、`RawDtb.Setup` | 已作为 OpenSBI handoff source fact 进入推导；`RawDtb` 仍必须逐步读取 header、检查 magic、读取 totalsize 并确定完整范围。若确认失败，事件返回 `Failed`，不得继续推进。 |
| 链接/入口布局硬约束 | `text_start == kernel_start`；`elf_entry == kernel_start`；`entry_head_text_layout_ready(Lds)`；`pre_mmu_access_discipline_ready(Lds)`；`trampoline_access_discipline_ready(Lds)` | `Lds.Online`、`KernelImage.Preset/Setup`、`TrampolineVm`、`EarlyVm` | 已作为 linker script source fact 进入推导；实现必须继续由 linker script、入口段布局和构建期/启动期检查维持这些事实。推进真实页表前，还要复查 head/trampoline 安全范围和访问纪律。 |
| 对象前序事实 | `boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid)`；`boot_cpu_present(BootCPU)`；`boot_cpu_active(BootCPU)` | `BootCPU.Preset`、`BootCPU.Setup`、`BootCPU.Enable` | 已由 `BootCPU` 事件 `ensures` 和前序事实推导消化；实现方面，`BootCPU.Preset` 已记录启动 hartid，`Setup/Enable` 仍需在入口后继期基于 `PlatformCpuInfo` 和 FDT `/cpus` 完成。 |
| 显式 deferred | `Soc` 早期平台状态点；`jump_label_init()`；`efi_init()` | `Soc.Preset` 和后续非最小启动路径 | 保持 deferred，不在第一轮对象级最小闭环中隐式实现；后续扩展 SoC、StaticKey 或 EFI 对象时再展开。 |

由此得到后续 `EntryPreludePhase` 实现顺序：

1. 持续保持 `Lds`/`KernelImage` 相关符号和布局检查面，确保 `_start`、`kernel_start`、text 起点、ELF entry、head text 范围在实现中可对应。
2. 继续实现 `RawDtb.Preset/Setup` 的最小确认路径，把 OpenSBI handoff 假设转化为 header/magic/totalsize/range 的运行期检查。
3. 在上述事实具备后，推进 `FixMap`、`TrampolineVm`、`EarlyVm` 和 `Vm` 三段切换。

## 应用复用

当前对象级实验不复用现有 ArceOS Unikernel 应用，不依赖 `ax-std`、`ax-api`、`ax-feat` 或 `arceos-rust`。

第一轮只保留一个内建的最小 payload：在对象级初始化完成后调用本地 `app_main()`，由它通过最小 `println!` 前端输出
`Hello, world!`。该 `println!` 不等同于 `axstd::println!`；后者属于后续组件封装阶段。

在未来 Composition Phase 中，再恢复“Unikernel app 引领内核形态”的 ArceOS 设计，并讨论如何接入 `ax-std`、测试 payload 和宏内核 payload。

## 新增核心 crate

当前不新增 crate。源码先集中在 `impl/arceos_ex/src/`，可按对象和架构分目录组织：

```text
impl/arceos_ex/
  Makefile
  README.md
  linker/riscv64.lds
  src/
    arch/riscv64/
    objects/
    phases/
    trace/
```

目录结构服务于对象级实现清晰性，不承担最终组件边界。

对象级实现优先采用“一个模型对象对应一个 `.rs` 文件”的组织方式。该要求可以逐步落实，不要求一次性重排既有文件。随着文件数量增加，可再按对象类别建立目录层级；架构强相关对象可以在类别目录下进一步区分架构。目录层级只用于源码管理，不等同于后续 Composition Phase 的 crate/module 公开边界。

## Cargo/xtask 策略

当前不使用 Cargo workspace、overlay workspace 或 `xtask`。如果需要 Rust 编译，Makefile 直接调用 `rustc` 或一个局部最小
`Cargo.toml`，但不得引入 ArceOS feature 传递链。

Cargo/xtask 策略整体延期到 Composition Phase。

## CI 与项目主页

GitHub workflow 分为测试和展示两类，但展示内容应主要来自测试流水线产物，不建立另一套独立生成来源。主页展示不要求高即时性，优先展示最近一次 nightly 或手动 workflow 成功生成的结果。

### 快速 CI

快速 CI 用于 pull request 和 push，目标是在较短时间内发现关键问题。第一轮应覆盖：

- 推导工具本身的格式检查、lint、单元测试和关键边界测试。
- 核心规格推导验证，例如 `spec/model/main.spec --derive --strict`。
- trace 生成 smoke test：输出到临时目录，确认命令成功，不要求把生成图提交回仓库。
- 顶层 `make verify`。
- 当对象级内核骨架具备可编译状态后，加入顶层 `make build`。

快速 CI 不发布 GitHub Pages，不运行耗时长或依赖模拟器稳定性的全量任务。

### Nightly 与手动触发

Nightly workflow 用于定时日构建，也支持 `workflow_dispatch` 手动触发。它可以执行耗时更长的任务：

- 推导工具全量测试、系统测试和 fixture 回归。
- 全规格批量推导验证。
- trace SVG 全量生成，并作为 artifact 保存。
- `impl/arceos_ex` 的完整 `make build`、`make run`、`make run LOG=trace`。
- QEMU smoke test，检查 `Hello, world!` 或 checkpoint 序列。
- 生成 unresolved obligations、deferred items、对象覆盖表、QEMU 日志等报告。

手动触发可用于规格大改后立即刷新展示结果，也可后续增加参数，例如指定 spec、指定 kernel implementation、是否运行 QEMU、是否发布 Pages。

### 项目主页展示

项目主页应作为测试流水线结果的发布视图，而不是独立测试来源。第一阶段采用轻量 GitHub Pages 方案即可，例如从 `docs/` 或 workflow artifact 发布静态页面。

主页展示内容优先包括：

- 项目目标和当前阶段。
- 主规格、`spec/model`、`spec/coding`、`spec/compose` 的入口。
- 最新成功 nightly/manual 生成的 startup trace SVG。
- 推导摘要、unresolved obligations 和 deferred items。
- `impl/arceos_ex` 对象级实现进展、Makefile 命令和 smoke 测试结果。
- 生成时间、commit id 和 workflow run id。

自动生成内容应放在清晰的 generated 区域，例如 `docs/generated/` 或 Pages artifact。普通 PR/push 只检查生成脚本可运行，不直接发布主页；nightly 或手动触发成功后再发布 GitHub Pages。

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
- reserved-memory：必须至少处理 FDT header `/memreserve/` 与 `/reserved-memory` 中当前启动闭环必要的保留范围，
  供 `MemBlock.setup()` 在 allocator 可用前排除；不得把 OpenSBI 或 QEMU virt 固定物理范围作为最终硬编码规则
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

单字符 checkpoint id 必须在当前后端中保持一一对应，避免运行期 trace 解码歧义。

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
