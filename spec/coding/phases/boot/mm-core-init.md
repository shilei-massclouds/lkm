# MmCoreInitPhase Coding

## Overview

核心内存初始化子阶段，对应 model `spec/model/phases/boot/mm-core-init/phase.spec` 中 `MmCoreInitPhase` 对象的 Preset → Online 生命周期。

本阶段从 `CorePreparePhase.Online` 开始，驱动 MemoryTopology、PageAllocator、SlubSubsystem、VmallocAllocator、Ioremap 等内存子系统的建立，对应 Linux `mm_core_init()` 核心路径。

按[阶段范式代码映射](../../phase-paradigm.md)，每个迁移对应一个概念函数。

## preset()

### 1. depends_on

由 `BootInitFlow.setup_after_core_prepare()` 启动，并在 `preset()` 中逐项检查：
- `CorePreparePhase.state == Online`
- `ExceptionStream.state == Ready`
- `MemBlock.state == Online`、`Zones.state == Ready`
- `PageMetadataMap.state == Ready`
- `CpuGroup.state == Ready`、`DmaCachePolicy.state == Ready`
- `PerCpuStorage.state == Ready`、`PrintkBuffer.state == Ready`
- `StaticBranch.state == Ready`
- `Vm.state == Online`、`SwapperVm.state == Online`

`preset()` 必须先检查 `MM_CORE_INIT_PHASE_STATE == Base`；全部依赖通过后才发出
`MmCoreInitPhase.Started`。

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

提交 Base → Prepared，读回并发出 `MmCoreInitPhase.Prepared`，再调用 `setup()`。

## setup()

### 1. depends_on

由 `preset()` 保证。

### 2. drives

无。

### 3. ensures

检查所有被驱动对象已到达模型约定的状态（参见下文 Invariant 表）。

### 4. emits

检查精确 Prepared，提交 Prepared → Ready，读回并发出 `MmCoreInitPhase.Ready`，再调用
`enable()`。

## enable()

### 1. depends_on

由 `setup()` 保证。

### 2. drives

无。

### 3. ensures

检查精确 Ready并重新确认 Online invariant，提交 Ready → Online，发出
`MmCoreInitPhase.Online` checkpoint。

### 4. emits

→ `BootInitFlow.setup_after_mm_core_init()` 父 continuation

## 迁移间调用关系

```
preset()  ← 由 BootInitFlow.setup_after_core_prepare() 调用
  │
  ├─ preset_objects()  ← 按模型 drives 顺序驱动全部对象 transition
  │
  ├─ adopt_prepared_with_check()  ← 检查 Preset ensures，标记 Prepared + checkpoint
  │
  setup()  ← emits
  │
  ├─ adopt_ready()  ← 检查 Ready invariant，标记 Ready + checkpoint
  │
  enable()  ← emits
  │
  ├─ enable_event()  ← 检查 invariant，标记 Online + checkpoint
  │
  └─ BootInitFlow.setup_after_mm_core_init()
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
| `MmCoreInitPhase.Started` | Base | `preset()` source/dependency 检查后 |
| `MmCoreInitPhase.Prepared` | Prepared | `adopt_prepared_with_check()` |
| `MmCoreInitPhase.Ready` | Ready | `adopt_ready()` |
| `MmCoreInitPhase.Online` | Online | `enable_event()`、父 continuation 前 |

## Coding Constraints

- `PageAllocator.setup()` 必须完成 `memblock_free_all()` 交接（MemBlock → Offline），不得在此前或此后额外推进 MemBlock。
- `SlubSubsystem.setup()` 必须引导 `kmem_cache` / `kmem_cache_node` 自举缓存，然后建立 `KmallocCaches`。
- `VmallocAllocator.setup()` 必须在 VmallocAllocator.Ready 前建立 `VmapAreaCache`、`VmapAddressSpace`、`VmapNodeSet`、`VmapBlockQueues`、`VfreeDeferredSet`。
- `Ioremap.setup()` 依赖 VmallocAllocator 的 VA 管理和映射执行能力，不得持有独立页表操作。
- 所有 mm_core_init 裁剪路径（PageExt/KFENCE/KMSAN/Kmemleak/DebugObjects/ExecMemory）必须在 `MmCoreTrimmedPaths` 中保留 observable 位置标记，不得假定为不存在。
- `PageAllocator.setup()` 中的 `zonelist_update_seq` 写侧 seqlock + `printk_deferred` 的协议必须保持 observable。
- `KernelGlobalAllocator.setup()` 必须在 `SlubSubsystem.Ready` 之后运行，暴露通过 SLUB/kmalloc 的 Rust `GlobalAlloc` 边界。

`MM_CORE_INIT_PHASE_STATE` 必须持久记录四状态；公开查询为 `is_online()`，且只在精确 Online 时
返回 true。

## 迁移自 legacy formal index 的 MUST/SHOULD

原 `mm-core-init.spec` 的规则按下列主题全部由本文件承接：

- MemoryTopology 只投影既有 zones；PageAllocator.Preset 只建立 topology/hooks，Setup 完成
  MemBlock 到 buddy 的 page handoff，MemBlock 最终为 Offline 而不是 Destroyed，且 SWIOTLB 必须
  在 handoff 前建立。
- buddy free lists 归 PageAllocator 所有，使用 zone/order free-area、首轮单 migratetype、拆分
  MemBlock free ranges、PageMetadata nodes 和 intrusive list；free area 只保存 head/count，node 是
  block-head metadata，不得依赖 heap storage。
- 暴露 Linux-like alloc/free pages API；分配返回 owned linear-mapped PageRef，free order 必须匹配
  alloc order；smoke 覆盖 alloc/free/read/write。
- 即使 boot lowering 可省略机器指令，model synchronization 仍保留；zonelist irqsave seqlock 与
  printk-deferred 协议必须可观察，PageAllocator 运行期 locking 必须显式 deferred。
- MemoryDebugHardening 使用 StaticBranch registry。SlubSubsystem 是唯一 facade 而不是 cache
  instance；类型名为 SlubCache，registry 拥有所有实例，KmallocCaches 引用已注册 cache，所有
  `kmem_cache` 创建点必须注册 named cache 或显式 deferred。
- SLUB bootstrap 必须早于 KmallocCaches.Ready；kmalloc/kzalloc/kfree 使用 PageAllocator backing
  pages、固定 size classes 和 slab-slot freelist，kzalloc 清零，kfree 回收；复杂 Linux 路径、
  runtime locking 和 slab-mutex/FULL 边界保持显式，smoke 覆盖三类 API。
- GlobalAlloc 只能在 SLUB Ready 后建立，并实现 `core::alloc::GlobalAlloc`；alloc 走 kmalloc，
  alloc_zeroed 走 kzalloc 或等价清零，dealloc 从 pointer 恢复 kmalloc object，只支持文档化 layout
  子集。动态容器依赖 GlobalAlloc Ready，smoke 覆盖 Vec growth/drop 和 layout pressure boundary。
- PageTableLockCache 的 Linux 名必须是 `page->ptl`。Vmalloc 同时管理 vmap addresses 和执行映射，
  Setup 建立所有 vmap subobjects，每次 map page range 记录动作，支持预分配窗口、拒绝重复映射并
  可按需分配 L0 window；unmap 必须早于 free vmap area，锁、RCU、TLB/cache synchronization
  contract 不能被隐藏。
- Ioremap 把 physical resource 与 MMIO policy 保持在 Vmalloc 之外；iounmap 只请求 Vmalloc
  teardown，并显式记录 MMIO attribute policy。MmStructCache 只创建 `mm_struct` cache。
- `MmCoreInitPhase` 的 Started、Prepared、Ready、Online checkpoint 均应保持可观察。
