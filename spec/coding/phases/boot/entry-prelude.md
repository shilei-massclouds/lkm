# EntryPreludePhase Coding

## Overview

入口前导子阶段，对应 model `spec/model/phases/boot/entry-prelude/phase.spec` 中 `EntryPreludePhase` 对象的 Preset → Online 生命周期。

本阶段从内核入口 `_start` 到 EarlyVm.Online（早期虚拟地址空间可访问），职责：

- 关闭 S 模式中断总开关（sie/sip）
- 建立 gp-relative 寻址
- 禁用内核 FPU/Vector
- 清零 BSS
- 记录 boot hartid，建立 BootCPU 对象
- 安装 init_task/pt_regs/trap 入口
- 建立跳板页表 → 早期页表，完成物理→虚拟地址切换

本阶段是首个子阶段，没有前驱阶段。`_start` 是启动入口，不由上级函数调用。

按[阶段链式映射规则](../../mapping.md#阶段链式映射规则)，每个迁移对应一个概念函数。preset 因汇编 → 地址空间切换的技术约束，跨多段实现；setup 和 enable 为空，仅做 emits 推进。

## preset()

入口符号：`_start`（不是 Rust fn，由 OpenSBI 跳转至此）

### 1. depends_on

首个子阶段，无运行时检查。前置条件由 Prepare 层保证（硬件规格、OpenSBI、Lds、Config 均 Online）。

### 2. drives

模型 EntryPreludePhase.Preset 驱动全部子对象迁移。impl 分为三段：

**段 1 — 汇编 head（`_start` 到 `entry_prelude_rust_entry`）**

| Model drives | Impl |
|---|---|
| `InterruptStream.Transition::Preset` | `csrw sie, zero; csrw sip, zero` |
| `KernelImage.Transition::Preset` | `la gp, __global_pointer$` |
| `RootStream.Transition::Preset` | `csrrc zero, sstatus, #FPU_VECTOR_MASK` |
| `KernelImage.Transition::Setup` | BSS zero loop |
| `BootCurrentCPU.Transition::Preset` | 保存 `a0`(hartid) 到 `head_boot_hartid` |
| `BootInitTask.Transition::Preset` | `la tp, init_task` |
| `BootInitStack.Transition::Preset` | `la sp, init_stack_end`; 预留 `pt_size_on_stack`; `csrw stvec, head_trap_entry` |

**段 2 — Rust `setup_until_vm_switch`（`entry_prelude_rust_entry` → `Vm.Setup`）**

| Model drives | Impl |
|---|---|
| `BootCurrentCPU.Transition::Setup` → `CpuGroup.Transition::Preset` → `BootCurrentCPU.Transition::Enable` | `cpu_group.preset()`, `boot_current_cpu.enable()` |
| `EventStream.Transition::Preset` | `event_stream.preset()` |
| `ExceptionStream.Transition::Preset` | `exception_stream.preset()` |
| `Vm.Transition::Preset`（含 `TrampolineVm.Setup` → `EarlyVm.Preset` → `EarlyVm.Setup`） | `vm.preset()` |

**段 3 — Rust 续接 `after_vm_setup`（地址空间切换后）**

| Model drives | Impl |
|---|---|
| `Vm.Transition::Setup`（含 `TrampolineVm.Enable` → `EarlyVm.Enable` → `TrampolineVm.Cleanup` → `KernelImage.Enable`） | 在 `vm.setup()` 内完成，回调 `after_vm_setup_continuation` |
| `EventStream.Transition::Setup` | `event_stream.setup()` |
| `BootInitTask.Transition::Enable` | `init_task.enable()` |
| `BootInitStack.Transition::Setup` | `init_stack.setup()` |
| `Soc.Transition::Preset` | `Soc::preset()` |

### 3. ensures

调用 `checkpoint_ready()`，检查模型 Online invariant（见下文 invariant 表）。成功后推进自身状态。

### 4. emits

推进 Base → Prepared（概念上；impl 中 Prepared 隐式到达，`checkpoint_ready` 标记 Ready 作为下一个可见状态点）。调用 `setup()`。

## setup()

### 1. depends_on

由 preset 保证。

### 2. drives

无。

### 3. ensures

Ready 状态（`checkpoint_ready` 已确认对象状态满足 Online invariant）。

### 4. emits

调用 `enable()`。

## enable()

### 1. depends_on

由 setup 保证。

### 2. drives

无。

### 3. ensures

调用 `handoff_event()`，标记 Ready → Online，发出 `EntryPreludePhaseOnline`  checkpoint。

### 4. emits

→ `EntrySuccessorPhase.preset()`

## 迁移间调用关系

```
_start (preset 入口)
  │
  ├─ [汇编 head] → drives(InterruptStream.Preset, KernelImage.Preset, ...)
  │
  ├─ [Rust setup_until_vm_switch] → drives(EventStream.Preset, Vm.Preset, ...)
  │
  ├─ [Rust after_vm_setup] → drives(EventStream.Setup, InitTask.Enable, ...)
  │
  └─ checkpoint_ready()  ← ensures + emits
       │
       setup() ← emits（空壳）
       │
       enable() ← emits
       │
       handoff_event() → EntrySuccessorPhase.preset()
```

## Invariant（模型 EntryPreludePhase.Online）

EntryPreludePhase 的 ensures 检查以下对象状态：

| Object | Required State |
|---|---|
| RootStream | Prepared |
| InterruptStream | Prepared |
| EventStream | Ready |
| ExceptionStream | Prepared |
| PageFaultException | Prepared |
| SyscallException | Prepared |
| BreakpointException | Prepared |
| UnexpectedException | Prepared |
| KernelImage | Online |
| RawDtb | Ready |
| BootInitTask | Online |
| BootInitStack | Ready |
| Vm | Ready |
| TrampolineVm | Destroyed |
| EarlyVm | Online |
| BootCurrentCPU | Online |
| BootCPU | Prepared |
| CpuGroup | Prepared |
| Soc | Prepared |

## Checkpoints

| Checkpoint | Phase State | Position |
|---|---|---|
| `EntryPreludePhaseStarted` | Base | head 段 `trace_adopt_begin` |
| `EntryPreludePhaseReady` | Ready | `checkpoint_ready()` |
| `EntryPreludePhaseOnline` | Online | `handoff_event()` |

## Coding Constraints

- RISC-V early alternatives (`apply_early_boot_alternatives`) 在本阶段 Vm.Preset 中 deferred，不得假装已实现，必须保持 observable deferral。
- head 段汇编必须在 Rust 运行前完成所有物理地址阶段的 CSR/GPR 操作。
- TrampolineVm 到 EarlyVm 切换必须在同一个 Vm.Setup event 内完成，不跨阶段边界。
- preset 跨汇编/Rust 多段是因地址空间切换的技术约束，不是模型层面有多个 Preset。
