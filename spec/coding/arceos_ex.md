# arceos_ex 对象级实现说明

本文记录 `arceos_ex` 第一轮对象级实现的设计背景、命令、工程边界和专题方案。统一任务优先级和状态以
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) 为准；本文不维护独立计划表。

当前执行路线已经调整为：先在本仓库内直接完成对象级实现实验，源码放在
`impl/arceos_ex/`，使用 `Makefile` 编译和运行；暂时不进入 `tgoskits`、`xtask`、ArceOS crate 兼容和 feature 传递问题。

`tgoskits`/ArceOS 组件兼容属于后续 `Composition Phase`，只有对象级实现闭环后再恢复讨论。此前在
`tgoskits` 中实现过的 `arceos_ex` 仍具有参考价值，尤其是 RISC-V64 入口、链接脚本、checkpoint 字符输出、SBI/FDT
平台细节和 overlay 经验；但当前对象级实现不得直接继承其 ArceOS 组件边界、feature 传递、`axlog` 或 `ax-alloc` 接入。

## 目标边界

第一轮目标是让 `arceos_ex` 作为规格驱动的对象级内核原型运行，并完成当前模型中的两个引导子阶段和最终 payload 交接阶段：

- `EntryPreludePhase.Ready`
- `EntrySuccessorPhase.Ready`
- `PayloadPhase.Online`

最小可见结果是通过独立早期输出路径打印启动 banner，进入默认 smoke payload，执行 smoke 用例并通过 SBI 关机。

实现推进分为两个逻辑阶段：

- `Object Coding Phase`：优先完成对象级语义，包括对象状态、事件推进、依赖检查、checkpoint 和必要的最小运行路径。
- `Composition Phase`：在对象级语义明确之后，再整理 crate/module 边界、公开接口、adapter、overlay workspace 和与 ArceOS 组件体系的兼容关系。

当前只推进 `Object Coding Phase`。不得因为未来 ArceOS 组件封装需要而反向改变模型对象语义。

实现分层上，Phase 对象只作为过程编排存在；非 Phase 对象原则上应在 Rust 中有明确承载，例如 struct、静态单例或启动上下文字段。

## 入口命令

当前入口统一使用仓库顶层 `Makefile`，默认内核为 `arceos_ex`。

第一轮优先支持：

```bash
make build
make build APP=smoke
make build APP=hello
make run
make run APP=smoke
make run APP=hello
make run LOG=trace
make verify
make verify REPORT=graph
make clean
```

`KERNEL ?= arceos_ex` 选择默认内核，`APP ?= smoke` 选择默认 selected payload。`build` 负责编译内核镜像；`run` 使用 QEMU/OpenSBI 运行；`run LOG=trace`
启用 checkpoint 字符输出。`verify` 调用 `pyveri` 对当前启动时间轴规格做推导验证；`verify REPORT=graph`
生成带注释的 trace SVG 报告。

当前对象级实现已经能通过 `make run` 和 `make run LOG=trace` 完成 `EntryPreludePhase.Ready`、
`EntrySuccessorPhase.Ready` 与 `CorePreparePhase.Ready`，随后通过 `PayloadPhase` 进入默认 `smoke` payload，执行 smoke 用例后通过 SBI 关机。`MmCoreInitPhase` 已进入正式规格；实现侧仍按后续对象级任务推进。

## `make verify` obligation 分类

`80966ec` 曾暴露 `13 obligation / 2 deferred`。这些条目不能作为实现可忽略的提示；处理原则是：实现某个对象事件前，必须先通过“推导义务门禁”。能由模型、推导工具、链接脚本、ISA、固件规范或已证明前序事实推出的，应优先补齐推导证明；只能由外部交付保证支撑的，应明确作为 source assumption，并在后续对象事件中尽快转化为运行期检查；无法归类的应作为规格缺口或显式 deferred。

当前已补齐 Lds、OpenSBI DTB handoff 和 BootCPU 前序事实的推导规则，`make verify` 报告为 `0 obligation / 3 deferred`。后续若再次出现 obligation，应先回到本节分类处理，不得直接继续实现。

| 分类 | 条目 | 影响范围 | 当前处理策略 |
| --- | --- | --- | --- |
| 固件交付假设，运行期逐步确认 | `firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)`；`firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)` | `OpenSbiFirmware.Online`、`RawDtb.Preset`、`RawDtb.Setup` | 已作为 OpenSBI handoff source fact 进入推导；`RawDtb` 仍必须逐步读取 header、检查 magic、读取 totalsize 并确定完整范围。若确认失败，事件返回 `Failed`，不得继续推进。 |
| 链接/入口布局硬约束 | `text_start == kernel_start`；`elf_entry == kernel_start`；`entry_head_text_layout_ready(Lds)`；`pre_mmu_access_discipline_ready(Lds)`；`trampoline_access_discipline_ready(Lds)` | `Lds.Online`、`KernelImage.Preset/Setup`、`TrampolineVm`、`EarlyVm` | 已作为 linker script source fact 进入推导；实现必须继续由 linker script、入口段布局和构建期/启动期检查维持这些事实。推进真实页表前，还要复查 head/trampoline 安全范围和访问纪律。 |
| 对象前序事实 | `boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid)`；`boot_cpu_present(BootCPU)`；`boot_cpu_active(BootCPU)` | `BootCPU.Preset`、`BootCPU.Setup`、`BootCPU.Enable` | 已由 `BootCPU` 事件 `ensures` 和前序事实推导消化；实现方面，`BootCPU.Preset` 已记录启动 hartid，`Setup/Enable` 仍需在入口后继期基于 `PlatformCpuInfo` 和 FDT `/cpus` 完成。 |
| 显式 deferred | `Soc` 早期平台状态点；`efi_init()` | `Soc.Preset` 和后续非最小启动路径 | 保持 deferred，不在第一轮对象级最小闭环中隐式实现；`StaticBranch` 已进入 formal model，后续 EFI 对象扩展时再展开 EFI 路径。 |

### 待迁移到 model 的 deferred 标记

`deferred` 的归属首先是 `spec/model`。本节只是实现说明中的审计清单，用来记录哪些参考 Linux
启动流程尚未迁移到模型文件中的显式 `deferred` 块。完成迁移后，模型才是这些 deferred 的事实源；
`impl/arceos_ex` 是否实现、何时实现，是后续单独的展开任务。

以下条目来自对照 `linux-6.12.37/default_config` 后的规格缺口复查。它们不要求立即实现，但后续应先补入
`spec/model` 对应对象或 phase 的 `deferred`，避免读者把当前最小模型误解为已经完整覆盖参考 Linux
启动路径。

| 候选项 | 参考配置/路径 | 建议归属 | 说明 |
| --- | --- | --- | --- |
| RISC-V Linux boot image header / boot protocol header | RISC-V 入口协议 | `PreparePhase` 或 `Lds` | 当前由 `BootArgs`、`Lds` 前置事实吸收，但 header 本身没有说明不展开。 |
| EFI stub / PE header 入口细节 | `CONFIG_EFI=y`、`CONFIG_EFI_STUB=y` | `PreparePhase` 或 `Lds` | 现有 deferred 只覆盖 `efi_init()`，未覆盖 EFI stub/header 入口。 |
| SATP mode 探测与页表层级降级 | `CONFIG_PGTABLE_LEVELS=5` | `Config` 或 `Vm.Preset` | 当前 `Config.satp_mode` 是既定事实，未描述 Linux 的运行时探测/降级过程。 |
| `apply_early_boot_alternatives()` | `CONFIG_RISCV_ALTERNATIVE_EARLY=y` | `Vm.Preset` 或 `EarlyVm.Setup` | 早期 alternatives/errata patch 尚未作为对象或 deferred 标记。 |
| `set_task_stack_end_magic()` | `CONFIG_SCHED_STACK_END_CHECK=y` | `InitStack` | 可折叠进栈保护语义，但应说明当前不展开 Linux 的具体检查标记。 |
| `init_vmlinux_build_id()` | `start_kernel()` early generic path | `EntrySuccessorPhase` | 当前未建模 build id 初始化，也未标记 deferred。 |
| `page_address_init()` | `start_kernel()` before `setup_arch()` | `EntrySuccessorPhase` | 当前没有 page address 元数据对象。 |
| `setup_command_line()` / saved cmdline | `start_kernel()` after `setup_arch()` | `CommandLine` | `CommandLine` 管理 raw/saved/static 三个文本视图；Param 解析对象由 `Params` 管理并保持原时序。 |
| DT unflatten | `CONFIG_OF_FLATTREE=y` | `EarlyDtb` 或后续 DT 对象 | 当前只覆盖 early scan 所需事实，未标记 unflatten 阶段。 |
| `phys_ram_base` / `kernel_map.va_pa_offset` 建立 | `CONFIG_64BIT=y`、`CONFIG_MMU=y` | `MemBlock.Setup` 或 `SwapperVm.Setup` | 当前折叠进映射正确性谓词，未单独说明。 |
| `ZONE_DMA32` / zone 边界初始化前置事实 | `CONFIG_ZONE_DMA32=y` | `MemBlock.Setup` | 当前未抽象 zone 边界与 DMA32 限制。 |
| hugetlb 早期保留 | `CONFIG_HUGETLB_PAGE=y` | `MemBlock.Setup` | 当前只保留 memblock 高层结果，未展开 hugetlb reserve。 |
| final page table 权限细分 RW/RO/NX | `CONFIG_STRICT_KERNEL_RWX=y` | `SwapperVm.Setup` | 当前 `SwapperVm` 只要求映射 ready，未细化 text/rodata/data 权限域。 |
| `riscv_fill_hwcap()` / ISA 能力发布 | FPU/V/Zicbom 等启用 | 后续 `CpuFeature` / `UserIsa` 对象 | 当前边界没有 CPU feature/hwcap 发布对象。 |
| `apply_boot_alternatives()` | `CONFIG_RISCV_ALTERNATIVE=y` | 后续 `Alternative/Patch` 对象 | boot alternatives 未建模，且不同于 early alternatives。 |
| `riscv_user_isa_enable()` | RISC-V ISA 配置相关 | 后续 `UserIsa` 对象 | 用户态 ISA 暴露语义当前不属于最小闭环。 |

由此得到后续 `EntryPreludePhase` 实现顺序：

1. 持续保持 `Lds`/`KernelImage` 相关符号和布局检查面，确保 `_start`、`kernel_start`、text 起点、ELF entry、head text 范围在实现中可对应。
2. 继续实现 `RawDtb.Preset/Setup` 的最小确认路径，把 OpenSBI handoff 假设转化为 header/magic/totalsize/range 的运行期检查。
3. 在上述事实具备后，推进 `FixMap`、`TrampolineVm`、`EarlyVm` 和 `Vm` 三段切换。

## 应用复用

当前对象级实验不复用现有 ArceOS Unikernel 应用，不依赖 `ax-std`、`ax-api`、`ax-feat` 或 `arceos-rust`。

第一轮保留两个内建 payload：默认 `APP=smoke` 和最小独立 `APP=hello`。对象级初始化完成后，启动链进入 `PayloadPhase`，在 `PayloadPhase.Enable` 提交后调用 selected payload 的 `run() -> !`。当前 `smoke` payload 在 `impl/arceos_ex/src/apps/smoke/cases/` 下维护可返回测试用例，首批覆盖输出路径、格式化输出、MemBlock 分配和 FDT 查询。`APP=hello` 仍作为最小独立 payload，输出 `Hello, world!` 后通过 SBI 关机。

所有 payload 的入口约定为 `run() -> !`。这表示控制流不返回启动编排链：Unikernel payload 可以进入服务循环或停机，未来宏内核 payload 可以加载首个用户态程序并完成用户态切换。若某个 payload 意外返回，应视为违反 `PayloadPhase.Enable` 的 no-return handoff 契约。

当前 payload 选择由 Makefile 变量控制，`APP` 会转换为 Rust `--cfg app_<name>`，例如 `APP=smoke` 对应 `app_smoke`。后续新增 payload 时，应在 `impl/arceos_ex/src/apps/` 下新增模块，并在 `apps/mod.rs` 中加入对应静态选择分支。后续新增 smoke 用例时，应放在 `impl/arceos_ex/src/apps/smoke/cases/` 下，并返回 `SmokeResult`，不得使用 payload 级 `run() -> !` 契约。

在未来 Composition Phase 中，再恢复“Unikernel app 引领内核形态”的 ArceOS 设计，并讨论如何接入 `ax-std`、测试 payload 和宏内核 payload。

### smoke 测试边界

smoke payload 用于覆盖 QEMU 运行期可观察行为，以及规格推导不能单独替代的实现效果，例如控制台输出、格式化输出、内存分配动作、FDT/DeviceTree 公开查询接口、资源摘要和 payload 关机路径。若某个性质仅仅是在重复对象状态、生命周期顺序、内部副本一致性或谓词不变量，并且已经能由 `make verify` 的规格推导闭合，则不应为它新增 smoke 用例。

实现也不应为了 smoke 暴露原本不需要公开的内部状态查询接口。若某个对象同时有可验证的不变量和用户可观察行为，smoke 应测试后者；前者保留在模型谓词、推导验证和对象事件推进检查中。例如 `CommandLine` 的 raw/saved/static 文本视图一致性属于规格和实现状态推进约束，不需要单独增加只读取内部状态的 smoke case。

### 启动与 smoke 输出风格

启动日志和 smoke 用例输出主要服务人工审阅，SHOULD 优先采用接近 Linux 启动日志的清晰文本格式，而不是大量
`key=value` 调试字段。机器可解析的状态序列应通过 checkpoint trace 或后续结构化报告承载，不应挤进普通启动日志。

建议格式如下：

- 用简短标题标明当前对象或测试主题，例如 `Resource tree:`。
- 多项事实分行输出，左侧使用稳定的人类可读标签，冒号对齐，右侧放结果值。
- 物理地址范围采用 `[mem start-end]` 风格，输出为闭区间；内部实现仍可继续使用半开区间。
- 容量优先用 `KiB`、`MiB` 等可读单位，避免只输出裸字节数。
- 汇总行应说明事实类别和数量，例如 `1 region`、`5 regions`、`4 segments`；单复数可读性优先于完全机器化。
- 测试失败时可以直接输出一行具体失败原因，不要求套用对齐格式。

示例：

```text
Resource tree:
  Entries      : 12
  Root         : I/O memory
  System RAM   : 1 region, 131072 KiB
  Reserved     : 5 regions
  Kernel image : [mem 0x80200000-0x80221fff], 4 segments
```

### per-cpu 链接段约束

`PerCpuStaticImage` 由链接脚本中的 `.data..percpu` 输出段提供。链接脚本必须先收集显式 per-cpu 输入段，再收集普通
`.data.*` 尾部段；不得让 `.data .data.*` 的普通 wildcard 提前吞掉 `.data..percpu`、`.data..percpu.*` 等输入段。运行期
`Lds.entry_layout_ready()` 会检查 `__per_cpu_start < __per_cpu_end`、起点页对齐，以及 `__per_cpu_load` 到模板大小的加载范围仍处于内核镜像内。

### 静态 per-cpu 访问接口

当前实现 SHOULD 提供两类接近 Linux 静态 percpu 使用方式的接口：

- 定义接口：通过宏封装 `#[link_section = ".data..percpu"]` 和 `#[used]`，只声明静态 percpu 模板变量。
- 访问接口：通过 `PerCpuStorage` 的 offset table 按 logical CPU 计算实例地址，并提供 `per_cpu_ptr` 风格地址计算以及类型化读/写 action；调用者不得直接依赖 first chunk 的 dynamic/reserved 内部布局。

从规格语义看，静态 percpu 访问是 action：它锚定在 `PerCpuStorage.Ready` / `PerCpuOffsetTable.Ready` 状态上执行。若对象尚未处于这些状态，action 应失败或返回空结果；若 action 成功，也仍停留在原 Ready 状态，不推进任何对象生命周期状态，也不得伪造成新的 `Setup/Enable` 事件。

smoke 可以覆盖静态 percpu 访问的真实运行期行为，例如对每个 possible logical CPU 的实例分别写入并读回，确认实例互不影响。完整动态
percpu allocator 暂缓，在规格中只有 `dynamic reserve ready` 布局事实时，不应新增动态分配或动态区裸写 smoke。

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
- QEMU smoke test，检查 smoke 汇总输出、独立 `APP=hello` 输出或 checkpoint 序列。
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

实现必须覆盖当前模型中 `EntryPreludePhase`、`EntrySuccessorPhase` 和 `PayloadPhase` 所需对象。Phase 对象可以是编排过程；非 Phase 对象原则上应有 Rust struct、静态单例或启动上下文字段承载。

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
- `CommandLine` / `KernelCmdline`
- `Params` / `EarlyParam`
- `SBI`
- `PrintkBuffer`
- `EarlyCon`
- `MemBlock`
- `InitMM`
- `EarlyIoremap`
- `CorePreparePhase` 编排的最小对象骨架：`DeviceTree`、`Zones`、`ResourceTree`、`CacheBlockInfo`、`CpuCapabilities`、`CommandLine`、`PerCpuStorage`、`CpuHotplugState`、`Params`、`BootParam`、`PayloadParam`、`Randomness`、`ExceptionTable`。这些对象不是 `CorePreparePhase` 的下级对象；phase 只驱动其生命周期事件。`SavedCommandLine` / `StaticCommandLine` 是 `CommandLine` 的子视图对象，不是独立顶级对象；`EarlyParam` / `BootParam` / `PayloadParam` 是 `Params` 的子对象。`PerCpuStaticImage`、`PerCpuFirstChunk` 和 `PerCpuOffsetTable` 是 `PerCpuStorage` 的子对象；first chunk 的 unit 个数和 offset table 边界必须基于 `CpuGroup` 的 possible CPU 集合，而不是 online CPU 集合。

`Randomness.preset()` 对应 `random_init_early(command_line)` 的早期语义。实现应建立 early seed material，并至少混入 `StaticCommandLine`；具体 mix/hash 算法不由 coding 规格限定。当前 RISC-V64 最小实现允许记录 arch entropy 为 0，且 `Randomness.Prepared` 不得暴露正式随机数接口或表示完整 RNG ready。

`PrintkBuffer.setup()` 对应 `setup_log_buf()` 的运行期准备语义。它不重新建立早期输出能力，而是在 `MemBlock`、`PerCpuStorage` 和 `BootParam` 已就绪后完成运行期日志缓冲准备；当前最小实现可以不扩容、不切换动态缓冲，继续使用静态 ring buffer，但必须标记运行期 printk 数据可用，且不得清空或丢弃 setup 时仍未 drain 的记录。

`ExceptionTable.setup()` 对应 `sort_main_extable()`。链接脚本必须提供 `__start___ex_table` / `__stop___ex_table` 边界，运行时必须基于该范围完成主异常表排序并提供按 faulting instruction address 查询的路径。当前表为空时也必须显式确认空表范围有效、排序事实成立且 lookup 返回 miss，不得仅以 checkpoint 代替该语义。

`EventStream.setup()` 安装的 formal trap entry 只负责读取 cause 并做第一层分流：interrupt cause 必须进入
`InterruptStream`，exception cause 必须进入 `ExceptionStream`。不得让 `ExceptionStream` 直接承担中断过滤职责。

`ExceptionStream` 和 `InterruptStream` 应维护 cause 到 handler policy 的静态分发表。当前最小实现可以使用固定大小数组或等价的静态表；每个表项初始必须绑定到 fallback panic/halt handler，不能留下空项、未初始化项或落到未定义行为。具体异常或中断对象完成 `Setup` / `Enable` 时，只能替换它所覆盖 cause 的表项。

异常 cause 映射至少应满足：

1. instruction/load/store page fault 绑定到 `PageFaultException`。
2. breakpoint 绑定到 `BreakpointException`。
3. user/supervisor ecall 绑定到 `SyscallException`，在 syscall 机制未启用前仍必须落到受控 panic/halt 或 disabled policy。
4. 其它同步异常绑定到 `UnexpectedException`。

`formal_event_entry` 必须先建立共享 trap context，再进入 `EventStream` 的第一层分流。该 context 至少应保存
`scause`、`sepc`、`sstatus`、`stval`，以及可透明返回所需的通用寄存器；当前最小实现 SHOULD 保存全部整数寄存器。
具体异常 handler 不应直接执行 `sret`，而应通过统一的 trap outcome 或等价约定表达 `Resume` / `Panic`：
`Resume` 由统一出口写回 `sepc`、恢复寄存器并 `sret`，`Panic` 仍走受控 panic/halt。

`BreakpointException.Setup` 安装的是 breakpoint trap 的 hook 分发入口，而不是“任意 #BR 均跳过返回”的终点
handler。其处理顺序 SHOULD 参考 Linux/RISC-V 的 `handle_break()`：先给 single-step/probe 类 hook 机会，再给
breakpoint hit hook 机会，后续可扩展 KGDB、BUG、CFI 等 hook。hook 输入为共享 trap context，输出应能表达：
已接管并 resume、已接管并 panic/halt、未接管。只有明确接管并要求 resume 的 hook 可以修改 `sepc` 并返回；
未知 #BR 或无人接管的 #BR 必须落入受控 panic/halt，不得默认跳过。

`Preset` 阶段仍只安装受控 fallback panic/halt；`Setup` 阶段才把 breakpoint cause 切换到 hook 分发入口；
`Enable` 保留给后续完整调试机制，例如断点管理、kprobe/kgdb 或等价设施。当前 smoke 若需要验证可返回路径，
应通过一个明确的 smoke/probe hook 接管测试断点，而不是依赖未知 #BR 的默认跳过行为。该 hook 若需要跳过
`ebreak` 或 `c.ebreak`，指令长度判断参考 Linux/RISC-V 的 `GET_INSN_LENGTH` 规则：读取 `sepc` 处半字，
若低两位为 `0b11` 则跳过 4 字节，否则跳过 2 字节。

其它 handler 当前仍可以返回 `!` 并执行 panic/halt；后续支持 syscall、timer interrupt 或 page fault recovery 时，
应复用同一套 trap context、handler outcome 和统一出口路径。

## DeviceTree unflatten 编码约束

正式 `DeviceTree` 对应模型中的 `DeviceTree.setup()`，参考 Linux 的 `unflatten_device_tree()` /
`unflatten_and_copy_device_tree()` 路径。它不同于早期 `EarlyDtb` 扫描：`EarlyDtb` 只提取启动早期事实，
`DeviceTree` 要建立后续运行期可遍历、可查询的树结构。

`DeviceTree.setup()` 必须把 `MemBlock.Online` 作为可分配早期物理内存的能力来使用，而不是只检查状态。
展开树的节点和属性元数据必须来自 `MemBlock` 早期分配；不得使用普通 heap、`Vec`/`Box`，也不得用固定静态数组作为正式展开存储。属性原始值可以引用生命周期受保护的 `RawDtb` 或内建 DTB 拷贝，但这种引用关系必须在对象状态中可解释，不能依赖已经销毁的 `EarlyDtb` 临时结构。

实现应采用两次遍历 `RawDtb` 的流程：

1. 第一遍校验 FDT 结构，并计算展开后节点、属性和必要元数据所需空间；若遇到格式错误、深度越界、大小溢出或无法映射的地址范围，事件必须失败，不能提交 `Ready`。
2. 通过 `MemBlock.alloc_phys(size, align)` 或等价的 `MemBlock` 事件接口申请物理存储，并通过 `SwapperVm`/`Config` 已建立的线性映射取得可写地址。
3. 第二遍填充 `DeviceNode`、property、root、parent/children 和查询索引或等价关系。

`DeviceTree.Ready` checkpoint 只能在第二遍完成，并且 root 唯一、非 root 节点 parent 唯一、parent/children 一致、路径查询和 property 查询均可用之后发出。涉及裸指针写入 `MemBlock` 分配存储的代码应封装在小的内部 unsafe 边界内，对外优先暴露安全的状态推进和查询接口。

## `mm_core_init()` 编码约束

`MmCoreInitPhase` 已正式落到 `spec/model/boot/mm-core-init/`。实现侧必须保持与模型一致的阶段边界：入口是 `CorePreparePhase.Ready`、`ExceptionStream.Ready`、`MemBlock.Online`、`DmaCachePolicy.Ready`、`StaticBranch.Ready` 和 `SystemExclusive`；出口是 `PageAllocator.Ready`、`MemBlock.Offline`、`SlubAllocator.Ready`、`PageTableCaches.Ready`、`VmallocAllocator.Ready`、`MmStructCache.Ready`。

`PageAllocator.preset()` 只做 `build_all_zonelists(NULL)` 和 `page_alloc_init_cpuhp()` 对应的拓扑与 hook 建立：`BootZonelistSet.Ready`、fallback zoneref 顺序、NULL sentinel、`CPUHP_PAGE_ALLOC` step 注册。它不得释放 MemBlock 页，也不得把自身推进到 `Ready`。

`PageAllocator.setup()` 才对应 `memblock_free_all()`。该事件必须先要求 `Swiotlb.Ready` 和 `MemoryDebugHardening.Ready`，再把 MemBlock free ranges 交给 buddy/free page sets，并通过 `MemBlock.Disable` 使 `MemBlock.state == Offline`。本阶段不得执行 `memblock_discard()`，不得把 `MemBlock` 推进到 `Destroyed`。

`VmallocAllocator` 的边界是 vmalloc/vmap 虚拟地址资源管理，不是页表映射器。`PageTableCaches.setup()` 承担 RISC-V 当前主线的 `VMALLOC_START..VMALLOC_END` 页表范围预分配事实；`VmallocAllocator.setup()` 负责 `VmapAreaCache`、`VmapAddressSpace`、`VmapNodeSet`、`VmapBlockQueues` 和 `VfreeDeferredSet`，并导入已有 `vmlist` 作为 busy areas、建立 free vmap space。

`MmStructCache.setup()` 只建立 `"mm_struct"` cache。`vm_area_struct` cache、`vma_lock_cachep` 和 `mmap_init()` 属于后续 `proc_caches_init()` 或进程地址空间初始化路径，不得为了填满本阶段而提前塞进 `MmStructCache`。

`PageExt`、`KFENCE`、`KMSAN`、`Kmemleak`、`DebugObjectsMemory` 和 `ExecMemory` 当前按 `linux-6.12.37/default_config` 记录为 model `deferred`/trimmed 路径。实现若遇到这些调用位置，应输出 checkpoint 或保留 no-op 分支说明，不得散落 TODO 来替代正式规格记录。

## CacheBlockInfo 编码约束

`CacheBlockInfo.setup()` 对应 Linux 6.12.37 的 `riscv_init_cbo_blocksizes()`。当前实现只发布平台级 `CBOM` 和 `CBOZ` block size 事实；`CBOP` 虽然存在 DeviceTree binding，但不在该 Linux 初始化点发布，暂不进入当前对象状态。

实现必须从正式 `DeviceTree` 的 `/cpus` CPU nodes 读取 `riscv,cbom-block-size` 和 `riscv,cboz-block-size`，并结合 `CpuGroup` 只收集当前拓扑中的 hart。缺失属性表示 unavailable，不应导致启动失败。多个 hart 值不一致时只记录诊断事实，保持 first value wins 的收敛策略，不 panic，也不阻止 `CacheBlockInfo` 进入 `Ready`。

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

checkpoint trace 独立于 `EarlyCon` 和正式 `Console`。极早期地址空间阶段可以使用 RISC-V64 SBI legacy putchar 输出单个字符，用于定位 `EarlyVm` 切换生效前的最小事件；该路径不得依赖 allocator、锁、字符串地址、FixMap 或线性映射状态。`EarlyVm` 切换生效、完整 `KernelImage` 映射可访问后，trace 后端应输出稳定 checkpoint 名称字符串，而不是继续消耗单字符 id。

单字符 checkpoint id 只服务 `EarlyVm` 切换前的最低层观测，必须在该极早期后端中保持一一对应，避免运行期 trace 解码歧义。`EarlyVm` 切换后的 checkpoint 新增时只需提供稳定名称，不应再分配单字符 id。

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
