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

## Implementation Flow

### Head Segment (assembly)

文件 `impl/arceos_ex/src/phases/boot/entry_prelude.rs`，`_start` 标签。
model 顺序与 impl 对应关系：

| Model EntryPreludePhase.Preset drives | Impl head segment |
|---|---|
| `InterruptStream.Transition::Preset` | `csrw sie, zero; csrw sip, zero` |
| `KernelImage.Transition::Preset` | `la gp, __global_pointer$` |
| `RootStream.Transition::Preset` | `csrrc zero, sstatus, #FPU_VECTOR_MASK` |
| `KernelImage.Transition::Setup` | BSS zero loop |
| `BootCurrentCPU.Transition::Preset` | 保存 `a0`(hartid) 到 `head_boot_hartid` |
| `BootInitTask.Transition::Preset` | `la tp, init_task` |
| `BootInitStack.Transition::Preset` | `la sp, init_stack_end`; 预留 `pt_size_on_stack`; `csrw stvec, head_trap_entry` |

### Rust Segment — `setup_until_vm_switch`

head 段完成后进入 `entry_prelude_rust_entry`，调用 `adopt_head_prefix` 把汇编段已完成的 Preset 写入 Rust 对象状态，随后继续：

| Model drives | Impl `setup_until_vm_switch` |
|---|---|
| `BootCurrentCPU.Transition::Setup` → `CpuGroup.Transition::Preset` → `BootCurrentCPU.Transition::Enable` | `cpu_group.preset()`, `boot_current_cpu.enable()` |
| `EventStream.Transition::Preset` | `event_stream.preset()` |
| `ExceptionStream.Transition::Preset` | `exception_stream.preset()` |
| `Vm.Transition::Preset` (包括 `TrampolineVm.Setup` → `EarlyVm.Preset` → `EarlyVm.Setup`) | `vm.preset()` |

### Rust Segment — `after_vm_setup` (虚拟地址回跳)

`Vm.Setup` 切换到早期虚拟地址空间后通过 continuation 回到此函数：

| Model drives | Impl `after_vm_setup` |
|---|---|
| `Vm.Transition::Setup` (包括 `TrampolineVm.Enable` → `EarlyVm.Enable` → `TrampolineVm.Cleanup` → `KernelImage.Enable`) | 在 `vm.setup()` 内完成，回调 `after_vm_setup_continuation` |
| `EventStream.Transition::Setup` | `event_stream.setup()` |
| `BootInitTask.Transition::Enable` | `init_task.enable()` |
| `BootInitStack.Transition::Setup` | `init_stack.setup()` |
| `Soc.Transition::Preset` | `Soc::preset()` |
| EntryPreludePhase → Ready | `checkpoint_ready()` |

### Handoff — `handoff`

EntryPreludePhase 推进到 Online 后调用 `entry_successor::setup()` 进入下一子阶段。

## Ready Invariant (model Online invariant)

EntryPreludePhase.Online 时以下所有对象必须处于指定状态：

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

- RISC-V early alternatives (apply_early_boot_alternatives) 在本阶段 Vm.Preset 中 deferred，不得假装已实现，必须保持 observable deferral。
- head 段汇编必须在 Rust 运行前完成所有物理地址阶段的 CSR/GPR 操作。
- TrampolineVm 到 EarlyVm 切换必须在同一个 Vm.Setup event 内完成，不跨阶段边界。
