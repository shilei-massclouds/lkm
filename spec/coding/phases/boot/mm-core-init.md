# MmCoreInitPhase Coding

## Overview

核心内存初始化子阶段，对应 model `spec/model/phases/boot/mm-core-init/phase.spec` 中 `MmCoreInitPhase` 对象的 Preset → Online 生命周期。

本阶段从 `CorePreparePhase.Online` 开始，驱动 MemoryTopology、PageAllocator、SlubSubsystem、VmallocAllocator、Ioremap 等内存子系统的建立，对应 Linux `mm_core_init()` 核心路径。

按[阶段链式映射规则](../../mapping.md#阶段链式映射规则)，每个迁移对应一个概念函数。

## preset()

### 1. depends_on

由 `CorePreparePhase.enable()` 保证：
- `CorePreparePhase.state == Online`
- `ExceptionStream.state == Ready`
- `MemBlock.state == Online`、`Zones.state == Ready`
- `PageMetadataMap.state == Ready`
- `CpuGroup.state == Ready`、`DmaCachePolicy.state == Ready`
- `PerCpuStorage.state == Ready`、`PrintkBuffer.state == Ready`
- `StaticBranch.state == Ready`
- `Vm.state == Online`、`SwapperVm.state == Online`

### 2. drives

模型 `MmCoreInitPhase.Preset` 按以下顺序驱动：

| # | Model drives | Impl |
|---|---|---|
| 1 | `MemoryTopology.Transition::Setup` | `ctx.memory_topology.setup()` |
| 2 | `PageAllocator.Transition::Preset` | `ctx.page_allocator.preset()` |
| 3 | `MemoryDebugHardening.Transition::Setup` | `ctx.memory_debug_hardening.setup()` |
| 4 | `StackDepot.Transition::Setup` | `ctx.stack_depot.setup()` |
| 5 | `Swiotlb.Transition::Setup` | `ctx.swiotlb.setup()` |
| 6 | `PageAllocator.Transition::Setup` | `ctx.page_allocator.setup()` |
| 7 | `SlubSubsystem.Transition::Preset` | `ctx.slub_subsystem.preset()` |
| 8 | `SlubSubsystem.Transition::Setup` | `ctx.slub_subsystem.setup()` |
| 9 | `KernelGlobalAllocator.Transition::Setup` | `ctx.kernel_global_allocator.setup()` |
| 10 | `DynamicContainerRuntime.Transition::Setup` | `ctx.dynamic_container_runtime.setup()` |
| 11 | `PageTableCaches.Transition::Setup` | `ctx.page_table_caches.setup()` |
| 12 | `VmallocAllocator.Transition::Setup` | `ctx.vmalloc_allocator.setup()` |
| 13 | `Ioremap.Transition::Setup` | `ctx.ioremap.setup()` |
| 14 | `MmStructCache.Transition::Setup` | `ctx.mm_struct_cache.setup()` |
| 15 | `MmCoreTrimmedPaths.Transition::Setup` | `ctx.mm_core_trimmed_paths.setup()` |

### 3. ensures

驱动完成后检查：
- `mm_core_init_ready()`
- `interrupt_concurrency_closed()`
- `task_concurrency_closed()`
- `context_is(SystemExclusive)`

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

推进 Ready → Online，发出 `MmCoreInitPhaseOnline` checkpoint。

### 4. emits

→ `SchedInitPhase.preset()`

## 迁移间调用关系

```
preset()  ← 由 CorePreparePhase.enable() 调用
  │
  ├─ preset_objects()  ← 按模型 drives 顺序驱动全部对象 transition
  │
  ├─ adopt_prepared_with_check()  ← 检查并发关闭事实，标记 Prepared
  │
  setup()  ← emits
  │
  ├─ adopt_ready()  ← 检查 Online invariant，标记 Ready
  │
  enable()  ← emits
  │
  ├─ enable_event()  ← 标记 Online
  │
  └─ SchedInitPhase.preset()
```

## Invariant（模型 MmCoreInitPhase.Ready / Online）

| Object | Required State |
|---|---|
| CorePreparePhase | Online |
| ExceptionStream | Ready |
| MemoryTopology | Ready |
| MemoryNode | Ready |
| ZoneSet | Ready |
| ZonelistSet | Ready |
| ZonelistUpdateSeq | Ready |
| ZonelistPrintkDeferredSection | Ready |
| PageMetadataMap | Ready |
| PageAllocatorBuddyFreePageSets | Ready |
| PageAllocator | Ready |
| MemBlock | Offline |
| MemoryDebugHardening | Ready |
| Swiotlb | Ready |
| StackDepot | Ready |
| SlubSubsystem | Ready |
| SlubCacheRegistry | Ready |
| KmallocCaches | Ready |
| KernelGlobalAllocator | Ready |
| DynamicContainerRuntime | Ready |
| PageTableCaches | Ready |
| PageTableLockCache | Ready |
| VmallocAllocator | Ready |
| VmapAreaCache | Ready |
| VmapAddressSpace | Ready |
| VmapNodeSet | Ready |
| VmapBlockQueues | Ready |
| VfreeDeferredSet | Ready |
| Ioremap | Ready |
| MmStructCache | Ready |
| MmCoreTrimmedPaths | Ready |

## Checkpoints

| Checkpoint | Phase State | Position |
|---|---|---|
| `MmCoreInitPhaseStarted` | Base | `preset()` 入口 |
| `MmCoreInitPhaseReady` | Prepared | `adopt_prepared_with_check()` |
| `MmCoreInitPhaseOnline` | Online | `enable_event()` |

## Coding Constraints

- `PageAllocator.setup()` 必须完成 `memblock_free_all()` 交接（MemBlock → Offline），不得在此前或此后额外推进 MemBlock。
- `SlubSubsystem.setup()` 必须引导 `kmem_cache` / `kmem_cache_node` 自举缓存，然后建立 `KmallocCaches`。
- `VmallocAllocator.setup()` 必须在 VmallocAllocator.Ready 前建立 `VmapAreaCache`、`VmapAddressSpace`、`VmapNodeSet`、`VmapBlockQueues`、`VfreeDeferredSet`。
- `Ioremap.setup()` 依赖 VmallocAllocator 的 VA 管理和映射执行能力，不得持有独立页表操作。
- 所有 mm_core_init 裁剪路径（PageExt/KFENCE/KMSAN/Kmemleak/DebugObjects/ExecMemory）必须在 `MmCoreTrimmedPaths` 中保留 observable 位置标记，不得假定为不存在。
- `PageAllocator.setup()` 中的 `zonelist_update_seq` 写侧 seqlock + `printk_deferred` 的协议必须保持 observable。
- `KernelGlobalAllocator.setup()` 必须在 `SlubSubsystem.Ready` 之后运行，暴露通过 SLUB/kmalloc 的 Rust `GlobalAlloc` 边界。
