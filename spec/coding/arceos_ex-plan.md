# arceos_ex 第一轮实现计划

本文记录 `arceos_ex` 第一轮实现任务清单。当前执行路线已经调整为：先在本仓库内直接完成对象级实现实验，源码放在
`impl/arceos_ex/`，使用 `Makefile` 编译和运行；暂时不进入 `tgoskits`、`xtask`、ArceOS crate 兼容和 feature 传递问题。

`tgoskits`/ArceOS 组件兼容属于后续 `Composition Phase`，只有对象级实现闭环后再恢复讨论。

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
| P0 | 进行中 | 建立 `impl/arceos_ex/` 独立实验目录，包含 `Makefile`、RISC-V64 linker script、入口汇编和 no-alloc Rust 源码骨架。 |
| P0 | 待办 | 定义对象级公共基础：规范状态集合、事件集合、`EventResult`、生命周期事件唯一性检查和 checkpoint hook。 |
| P0 | 待办 | 实现 `EntryPreludePhase` 最小闭环：`_start`、`__global_pointer$`、head text 布局约束、BootArgs、RootStream、KernelImage、BootCPU、InitStack、RawDtb、FixMap、TrampolineVm、EarlyVm、VM 三段切换。 |
| P0 | 待办 | 实现 `EntrySuccessorPhase` 最小闭环：EarlyDtb、PlatformCpuInfo、PhysicalMemory、CpuIdMap、InterruptStream、BootCPU setup/enable、PrintkBuffer、KernelCmdline、KernelParam、SBI、EarlyCon、MemBlock、InitMM、EarlyIoremap、SwapperVm。 |
| P0 | 待办 | 建立 no-alloc 输出路径：启动期内部 `printk`/`println-like` 前端和应用侧最小 `println!` 前端都写入 `PrintkBuffer`，再由 `EarlyCon(SBI)` drain。 |
| P1 | 待办 | 实现最小 FDT 解析，不引入外部 crate，不使用 `Vec`、`String`、`Box`；只解析当前闭环必要的 `/cpus`、`/memory`、`/chosen`、`/memreserve/` 和必要 `/reserved-memory`。 |
| P1 | 待办 | 用 Makefile 提供 `build`、`run`、`trace`、`check-spec`、`clean` 等入口，暂时脱离 `xtask`。 |
| P1 | 待办 | 对照 `startup-timeline.trace.svg` 和 checkpoint 输出逐段复查规格、推导和实现一致性。 |
| P2 | 延期 | 组件封装阶段：恢复 ArceOS 组件接口、crate 边界、`ax-std` 接入、overlay workspace、`xtask`、feature 传递、`axlog` 和 `ax-alloc` facade 等问题。 |

## 入口命令

当前入口统一使用 `impl/arceos_ex/Makefile`。

第一轮优先支持：

```bash
make -C impl/arceos_ex build
make -C impl/arceos_ex run
make -C impl/arceos_ex trace
make -C impl/arceos_ex check-spec
```

`build` 负责编译 RISC-V64 内核镜像；`run` 使用 QEMU/OpenSBI 运行；`trace` 启用 checkpoint 字符输出；`check-spec` 调用 `pyveri`
检查当前启动时间轴规格。

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

## Cargo/xtask 策略

当前不使用 Cargo workspace、overlay workspace 或 `xtask`。如果需要 Rust 编译，Makefile 直接调用 `rustc` 或一个局部最小
`Cargo.toml`，但不得引入 ArceOS feature 传递链。

Cargo/xtask 策略整体延期到 Composition Phase。

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
