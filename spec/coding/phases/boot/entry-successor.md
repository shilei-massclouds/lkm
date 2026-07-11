# EntrySuccessorPhase Coding

## Overview

入口后继子阶段，对应 model `spec/model/phases/boot/entry-successor/phase.spec` 中 `EntrySuccessorPhase` 对象的 Preset → Online 生命周期。

本阶段从 `EntryPreludePhase.Online` 开始，推进到 `SwapperVm.Online`、`MemBlock.Online`、`EarlyDtb.Destroyed`，对应 Linux `start_kernel()` 中从 `setup_arch()`/`paging_init()` 完成后到 `mm_core_init()` 之前的核心路径。

按[阶段链式映射规则](../../mapping.md#阶段链式映射规则)，每个迁移对应一个概念函数。

## preset()

### 1. depends_on

由 `EntryPreludePhase.enable()` 保证：
- `EntryPreludePhase.state == Online`
- `Vm.state == Ready`
- `EarlyVm.state == Online`
- `BootInitTask.state == Online`
- `BootInitStack.state == Ready`
- `InterruptStream.state == Prepared`
- `RawDtb.state == Ready`
- `FixMap.state == Ready`
- `KernelImage.state == Online`

### 2. drives

模型 `EntrySuccessorPhase.Preset` 按以下顺序驱动：

| # | Model drives | Impl |
|---|---|---|
| 1 | `BootInitStack.Transition::Enable` | `ctx.init_stack.enable()` |
| 2 | `EarlyDtb.Transition::Preset` | `ctx.early_dtb.preset()`，内部驱动 `PlatformCpuInfo.Preset→Enable`、`PhysicalMemory.Preset→Enable` |
| 3 | `InterruptStream.Transition::Setup` | `ctx.interrupt_stream.setup()` |
| 4 | `BootCPU.Transition::Setup` | `ctx.boot_current_cpu.setup_boot_cpu()` |
| 5 | `BootCPU.Transition::Enable` | `ctx.boot_current_cpu.enable_boot_cpu()` + `ctx.cpu_group.register_boot_cpu()` |
| 6 | `PrintkBuffer.Transition::Preset` | `printk::preset()`、`printk::write_str()` |
| 7 | `EarlyDtb.Transition::Setup` | `ctx.early_dtb.setup()`，内部驱动 `MemBlock.Preset`、`CommandLine.Preset` |
| 8 | `InitMM.Transition::Setup` | `ctx.init_mm.setup()` |
| 9 | `EarlyIoremap.Transition::Setup` | `ctx.early_ioremap.setup()` |
| 10 | `SBI.Transition::Setup` | `ctx.sbi.setup()` |
| 11 | `Params.Transition::Preset` | `ctx.params.preset()`，内部驱动 `EarlyParam.Setup` → `EarlyCon.Preset→Setup→Enable` |
| 12 | `MemBlock.Transition::Setup` | `ctx.memblock.setup()` |
| 13 | `Vm.Transition::Enable` | `ctx.vm.enable()` |
| 14 | `MemBlock.Transition::Enable` | `ctx.memblock.enable()` |
| 15 | `EarlyDtb.Transition::Cleanup` | `ctx.early_dtb.cleanup()` |

### 3. ensures

驱动完成后检查 deferred facts：
- `vmlinux_build_id_deferred`
- `page_address_init_deferred`
- `entry_successor_start_kernel_position_preserved`

### 4. emits

推进 Base → Prepared，调用 `setup()`。

## setup()

### 1. depends_on

由 `preset()` 保证。

### 2. drives

无。

### 3. ensures

检查所有被驱动对象已到达模型约定的状态（参见下文 Invariant 表）。

### 4. emits

推进 Prepared → Ready，调用 `enable()`。

## enable()

### 1. depends_on

由 `setup()` 保证。

### 2. drives

无。

### 3. ensures

推进 Ready → Online，发出 `EntrySuccessorPhaseOnline` checkpoint。

### 4. emits

→ `CorePreparePhase.preset()`

## 迁移间调用关系

```
preset()  ← 由 EntryPreludePhase.enable() 调用
  │
  ├─ preset_objects()  ← 按模型 drives 顺序驱动全部对象 transition
  │
  ├─ adopt_prepared_with_check()  ← 检查 deferred facts，标记 Prepared
  │
  setup()  ← emits
  │
  ├─ adopt_ready()  ← 检查 Online invariant，标记 Ready
  │
  enable()  ← emits
  │
  ├─ enable_event()  ← 标记 Online
  │
  └─ CorePreparePhase.preset()
```

## Invariant（模型 EntrySuccessorPhase.Ready / Online）

| Object | Required State |
|---|---|
| EntryPreludePhase | Online |
| BootInitStack | Online |
| BootCPU | Online |
| InterruptStream | Ready |
| PrintkBuffer | Prepared |
| EarlyDtb | Destroyed |
| KernelCmdline | Ready |
| InitMM | Ready |
| EarlyIoremap | Ready |
| SBI | Ready |
| Params | Prepared |
| EarlyParam | Ready |
| EarlyCon | Online |
| MemBlock | Online |
| Vm | Online |
| SwapperVm | Online |
| EarlyVm | Destroyed |

## Checkpoints

| Checkpoint | Phase State | Position |
|---|---|---|
| `EntrySuccessorPhaseStarted` | Base | `preset()` 入口 |
| `EntrySuccessorPhaseReady` | Prepared | `adopt_prepared_with_check()` |
| `EntrySuccessorPhaseOnline` | Online | `enable_event()` |

## Coding Constraints

- `init_vmlinux_build_id()` 暂缓，保留调用位置作为 deferred fact。
- `page_address_init()` 暂缓，保留调用位置作为 deferred fact。
- `setup_bootmem()` 中 `hugetlb_cma_reserve()` 暂缓，保留调用位置作为 deferred fact。
- 所有 elf 映射、页表切换、物理内存采集和 SBI/console 能力探测必须按模型 drives 顺序推进，不得提前打开中断或任务并发。
- `printk::write_str("arceos_ex object kernel\n")` 是启动 banner 输出，不是生命周期 transition，属于 `PrintkBuffer.Preset` 之后的 `PrintkBuffer.Write` action。
