# CorePreparePhase Coding

## Overview

核心准备子阶段，对应 model `spec/model/phases/boot/core-prepare/phase.spec` 中 `CorePreparePhase` 对象的 Preset → Online 生命周期。

本阶段从 `EntrySuccessorPhase.Online` 开始，驱动 DeviceTree、Zones、PageMetadataMap、ResourceTree、CpuGroup、PerCpuStorage、StaticBranch、CommandLine、Params、PrintkBuffer、ExceptionStream 等对象的建立，对应 Linux `paging_init()` 之后到 `trap_init()` 之间的核心准备路径。

按[阶段范式代码映射](../../phase-paradigm.md)，每个迁移对应一个概念函数。

## preset()

### 1. depends_on

由 `BootInitFlow.setup_after_entry_successor()` 启动，并在 `preset()` 中逐项检查：
- `EntrySuccessorPhase.state == Online`
- `Vm.state == Online`、`SwapperVm.state == Online`
- `MemBlock.state == Online`
- `Params.state == Prepared`、`EarlyParam.state == Ready`
- `CommandLine.state == Prepared`
- `BootCPU.state == Online`、`BootCpuLocalInterrupt.state == Ready`
- `PrintkBuffer.state == Prepared`
- `ExceptionStream.state == Prepared`

`preset()` 还必须先检查 `CORE_PREPARE_PHASE_STATE == Base`；全部检查通过后才发出
`CorePreparePhase.Started`。

### 2. drives

模型 `CorePreparePhase.Preset` 按以下顺序驱动：

| # | Model drives | Impl |
|---|---|---|
| 1 | `DeviceTree.Transition::Setup` | `ctx.device_tree.setup()` |
| 2 | `Zones.Transition::Setup` | `ctx.zones.setup()` |
| 3 | `PageMetadataMap.Transition::Setup` | `ctx.page_metadata_map.setup()` |
| 4 | `ResourceLock.Transition::Preset` | `ctx.resource_lock.preset_static()` |
| 5 | `ResourceLock.Transition::Setup` | `ctx.resource_lock.setup()` |
| 6 | `ResourceTree.Transition::Setup` | `ctx.resource_tree.setup()` |
| 7 | `CpuGroup.Transition::Setup` | `ctx.cpu_group.setup_smp()` |
| 8 | `CacheBlockInfo.Transition::Setup` | `ctx.cache_block_info.setup()` |
| 9 | `CpuCapabilities.Transition::Setup` | `ctx.cpu_capabilities.setup()` |
| 10 | `DmaCachePolicy.Transition::Setup` | `ctx.dma_cache_policy.setup()` |
| 11 | `PerCpuStorage.Transition::Preset` | `ctx.per_cpu_storage.preset()` |
| 12 | `CpuHotplugLock.Transition::Preset` | `ctx.cpu_hotplug_lock.preset_static_with_per_cpu_storage()` |
| 13 | `CpuHotplugLock.Transition::Setup` | `ctx.cpu_hotplug_lock.setup()` |
| 14 | `JumpLabelMutex.Transition::Preset` | `ctx.jump_label_mutex.preset_static()` |
| 15 | `JumpLabelMutex.Transition::Setup` | `ctx.jump_label_mutex.setup()` |
| 16 | `StaticBranch.Transition::Setup` | `ctx.static_branch.setup()` |
| 17 | `CommandLine.Transition::Setup` | `ctx.command_line.setup()` |
| 18 | `PerCpuStorage.Transition::Setup` | `ctx.per_cpu_storage.setup()` |
| 19 | `CpuHotplugState.Transition::Setup` | `ctx.cpu_hotplug_state.setup()` |
| 20 | `Params.Transition::Setup` | `ctx.params.setup()` |
| 21 | `Randomness.Transition::Preset` | `ctx.randomness.preset()` |
| 22 | `PrintkBuffer.Transition::Setup` | `printk::setup()` |
| 23 | `ExceptionTable.Transition::Setup` | `ctx.exception_table.setup()` |
| 24 | `ExceptionStream.Transition::Setup` | `ctx.exception_stream.setup()` |

### 3. ensures

驱动完成后检查：
- `interrupt_concurrency_closed()`
- `task_concurrency_closed()`
- `smp_concurrency_closed()`
- `context_is(SystemExclusive)`
- `early_boot_irqs_disabled_true()`

### 4. emits

提交 Base → Prepared，读回后发出 `CorePreparePhase.Prepared`，再调用 `setup()`。

## setup()

### 1. depends_on

由 `preset()` 保证。

### 2. drives

无。

### 3. ensures

检查所有被驱动对象已到达模型约定的状态（参见下文 Invariant 表）。

### 4. emits

检查精确 Prepared，提交 Prepared → Ready，读回后发出 `CorePreparePhase.Ready`，再调用
`enable()`。

## enable()

### 1. depends_on

由 `setup()` 保证。

### 2. drives

无。

### 3. ensures

检查精确 Ready并重新确认 Online invariant，提交 Ready → Online，发出
`CorePreparePhase.Online` checkpoint。

### 4. emits

→ `BootInitFlow.setup_after_core_prepare()` 父 continuation

## 迁移间调用关系

```
preset()  ← 由 BootInitFlow.setup_after_entry_successor() 调用
  │
  ├─ preset_objects()  ← 按模型 drives 顺序驱动全部对象 transition
  │
  ├─ adopt_prepared_with_check()  ← 检查并发关闭事实，标记 Prepared + checkpoint
  │
  setup()  ← emits
  │
  ├─ adopt_ready()  ← 检查 Ready invariant，标记 Ready + checkpoint
  │
  enable()  ← emits
  │
  ├─ enable_event()  ← 检查 invariant，标记 Online + checkpoint
  │
  └─ BootInitFlow.setup_after_core_prepare()
```

## Invariant（模型 CorePreparePhase.Ready / Online）

| Object | Required State |
|---|---|
| EntrySuccessorPhase | Online |
| DeviceTree | Ready |
| Zones | Ready |
| PageMetadataMap | Ready |
| ResourceLock | Ready |
| ResourceTree | Ready |
| CpuGroup | Ready |
| CacheBlockInfo | Ready |
| CpuCapabilities | Ready |
| DmaCachePolicy | Ready |
| CpuHotplugLock | Ready |
| JumpLabelMutex | Ready |
| StaticBranch | Ready |
| CommandLine | Ready |
| SavedCommandLine | Ready |
| StaticCommandLine | Ready |
| PerCpuStorage | Ready |
| CpuHotplugState | Ready |
| Params | Ready |
| BootParam | Ready |
| PayloadParam | Ready |
| Randomness | Prepared |
| PrintkBuffer | Ready |
| ExceptionTable | Ready |
| ExceptionStream | Ready |
| PageFaultException | Ready |
| SyscallException | Prepared |
| BreakpointException | Ready |
| UnexpectedException | Ready |

## Checkpoints

| Checkpoint | Phase State | Position |
|---|---|---|
| `CorePreparePhase.Started` | Base | `preset()` source/dependency 检查后 |
| `CorePreparePhase.Prepared` | Prepared | `adopt_prepared_with_check()` |
| `CorePreparePhase.Ready` | Ready | `adopt_ready()` |
| `CorePreparePhase.Online` | Online | `enable_event()`、父 continuation 前 |

## Coding Constraints

- CorePreparePhase 运行在 `paging_init()` 完成后、`trap_init()`/`mm_core_init()` 之前的系统独占上下文中：中断、任务并发、SMP 并发均未打开。
- `StaticBranch.setup()` 必须保持 `CpuHotplugLock.ReadLock/ReadUnlock` 和 `JumpLabelMutex.Lock/Unlock` 的 guard 协议可见，不得因 SingleTaskContext 事实而隐藏或擦除。
- `PrintkBuffer.setup()` 中的 `local_irq_save/restore` 临界区必须绑定到 `BootCpuLocalInterrupt` 对象，如 `PrintkBufferSetupLocalInterruptContext` 模型上下文所示。
- `Randomness.preset()` 对应的 `random_init_early()` 中的 `_mix_pool_bytes()` 不持有 `input_pool.lock`，实现不得添加该锁。
- 所有 deferred 路径（如 `acpi_boot_table_init()`、`early_memtest()`、`sparse_init()`、`kasan_init()`、`bootconfig`、`VFS caches` 等）保留调用位置标记，不得在当前实现中提前推进或隐含状态。
- `checkpoint_setup_nr_cpu_ids` 仅验证 `CpuGroup` 的 `possible_cpu_boundary_ready` 和 `BootCPU == CpuGroup[0]`，不推进额外对象状态。
- `checkpoint_second_parse_early_param` 仅检查 `EarlyParam.state == Ready`（标志第二次 `parse_early_param()` 调用位置），不重新推进 `EarlyParam` 或 `EarlyCon`。
- `checkpoint_print_unknown_bootoptions` 仅输出 `BootParam` 收集到的未知选项，不改变 `BootParam` 状态。
- 所有对象推进必须严格按模型 drives 顺序执行，不得因实现方便提前打开中断或 SMP 并发。

`CORE_PREPARE_PHASE_STATE` 必须持久记录四状态；公开查询为 `is_online()`，且只在精确 Online
时返回 true。

## 迁移自 legacy formal index 的 MUST

原 `core-prepare.spec` 的有效规则全部由本文件承接：必须保持 early IRQ、task 和 SMP 并发关闭
事实；StaticBranch 必须记录并实际驱动独立 `CpuHotplugLock`/`JumpLabelMutex` guard，运行期 text
patch synchronization 继续显式 deferred；ResourceTree 必须记录并实际驱动独立 ResourceLock
write guard，通用 RwLock 暴露 reader/writer 协议；PrintkBuffer.Setup 必须记录 local irq
save/restore 并绑定 BootCpuLocalInterrupt；Randomness.Preset 不得无条件获取 input-pool lock，未来
展开条件 reseed/credit 路径时必须使用 base-crng irqsave lock。
