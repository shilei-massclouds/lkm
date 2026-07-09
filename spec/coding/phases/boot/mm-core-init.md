# MmCoreInitPhase coding

本文件承载 `spec/coding/phases/boot/mm-core-init.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/boot/mm-core-init.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`mm-core-init.spec`](mm-core-init.spec)。

### ArceosExMmCoreInitCodingMust

#### Memory topology view

MemoryTopology.setup() must only project already Ready Zones into
allocator-visible MemoryNode/ZoneSet views. It must not repartition
zones, allocate mem_map/page metadata, or claim new ownership of
Linux node_zones.

#### PageAllocator preset/setup split

PageAllocator.preset() must only establish zonelist topology and
page allocator CPU hotplug step registration. It must not release
MemBlock pages to buddy/free page sets.

#### MemBlock handoff

PageAllocator.setup() must perform the memblock_free_all() handoff:
account managed pages, populate buddy free page sets, and advance
MemBlock through Disable to Offline.

#### Minimal buddy free lists

PageAllocator.setup() must build buddy free lists as PageAllocator
internal storage. The first implementation round uses per-zone,
per-order free_area-like lists and a single migratetype.

#### MemBlock range splitting

The buddy setup must iterate MemBlock usable ranges, exclude
reserved ranges, and split the remaining pages into order-aligned
buddy blocks before linking them into the free areas.

#### Metadata-backed free nodes

Buddy free-list nodes must be represented through PageMetadata
fields on the block head page. The buddy structure must not depend
on Vec, Box, heap, SLUB, or kmalloc storage.

#### Linux-like buddy API

Once PageAllocator reaches Ready it must expose the formal
PageAllocatorType runtime actions as production APIs named after
Linux's buddy boundary: alloc_pages(order, gfp), alloc_page(gfp) as
an order-0 convenience, and free_pages(page_ref, order). These APIs
must be used by later SLUB/kmalloc and smoke paths instead of
private test-only hooks.

#### PageRef contract

alloc_pages(order, gfp) must return a caller-owned PageRef covering
2^order buddy pages that are reachable through the established
linear mapping. The PageRef is the object-model handle used by later
SLUB and smoke code; it must not expose allocator internals.

#### Free order contract

free_pages(page_ref, order) must release only PageRefs allocated from
PageAllocator, and the order must match the allocation order.

#### mm_core_init synchronization surface

SingleTaskContext contributes contextual facts for boot-time call
sites, but it must not make Linux lock, irq, preempt, RCU, per-cpu,
or TLB/cache requirements disappear from the formal model. For every
mm_core_init object that publishes a later runtime API, the
model/coding surface must say whether the sync protocol is executed
in setup, is a pure context fact with no runtime protocol, or is
explicitly deferred to the runtime consumer.

#### Zonelist update protocol

Linux __build_all_zonelists(NULL) runs inside
write_seqlock_irqsave(&zonelist_update_seq, flags) and
printk_deferred_enter()/exit(). The current target must still
execute and observe a balanced setup-time irqsave seqlock section
and printk-deferred section. Full seqlock reader/retry behavior
remains a later runtime refinement.

#### Page allocator runtime locking

PageAllocator.Ready means alloc_pages/free_pages are callable, not
that the complete Linux zone lock, PCP lock, irqsave and preempt
protocol has already been implemented. Until a consumer requires
those paths outside boot-exclusive context, the runtime locking
requirements must remain explicit deferred facts.

#### Page allocator smoke

The first allocator smoke coverage must allocate pages through the
formal alloc_pages API, perform bounded read/write through the
mapped page reference, and release them through free_pages().

#### MemBlock remains Offline

mm_core_init() must not destroy or discard MemBlock metadata. That
later discard belongs to page_alloc_init_late()/memblock_discard().

#### SWIOTLB ordering

Swiotlb.setup() must complete before MemBlock.Disable so any early
default pool or resolved no-pool fact is established while MemBlock
allocation/reservation is still available.

#### Static branch policy

MemoryDebugHardening.setup() must bind to the existing StaticBranch
registry and scanned EarlyParam boundary. Current mm hardening
early-param policy is trimmed to the default policy; it must not
initialize a private static-key mechanism or imply full parameter
support.

#### SLUB object hierarchy

SlubSubsystem is the single SLUB facade object, not a cache
instance and not a reusable SlubSubsystemType. The formal cache type
name is SlubCache, not SlubCacheType. SlubCacheRegistry must be the
single registry/ownership collection for boot caches, formal
kmem_cache/kmem_cache_node, kmalloc size-class caches, and later
named caches. KmallocCaches is only a size-class reference/index
view over registered SlubCache instances; it must not own a second
set of cache instances. Named phase objects and Linux
KMEM_CACHE(...) sites such as page->ptl, vmap_area, mm_struct,
radix tree node, maple node, and pool_workqueue caches must
register their underlying SlubCache instance in SlubCacheRegistry
instead of keeping only private ready flags. A KMEM_CACHE(...) site
may only remain outside the registry if it has an explicit deferred
classification with a later owner.

#### SLUB bootstrap

SlubSubsystem.setup() must bootstrap kmem_cache/kmem_cache_node
before KmallocCaches is considered Ready.

#### Linux-like kmalloc API

Once SlubSubsystem reaches Ready it must expose production
kmalloc(size, gfp), kzalloc(size, gfp), and kfree(ref) APIs. The
first round may use a minimal page-backed slab implementation, but
it must allocate backing pages through PageAllocator rather than
test hooks or MemBlock.

#### SLUB bootstrap synchronization

Linux kmem_cache_init() explicitly runs before slab_mutex is needed
for the early bootstrap caches, but SLUB runtime allocation/free,
per-cpu/node partial lists, global list mutation, slab sysfs, and
FULL state still have synchronization semantics. The target must
record the pre-FULL slab_mutex boundary and keep the runtime SLUB
locking model as an explicit deferred contract instead of treating
SingleTaskContext as a proof that no SLUB locks exist.

#### SLUB/kmalloc smoke

The kmalloc smoke coverage must use the formal kmalloc/kzalloc/kfree
APIs to check writable allocations, zeroed kzalloc storage,
independent same-size allocations, and free-list reuse.

#### Global allocator

KernelGlobalAllocator.setup() must run after SlubSubsystem.Ready and
expose the Rust GlobalAlloc boundary through SLUB/kmalloc. Ordinary
dynamic containers must depend on this boundary rather than using
MemBlock, PageAllocator internals, or SLUB private structures.

#### Dynamic container smoke

The first dynamic-container smoke must use Vec through the ordinary
allocator path, force at least one growth, validate stored values,
and drop the Vec without direct SLUB or PageAllocator access.

#### Page table lock cache

PageTableLockCache.setup() must create the split page-table lock
cache corresponding to Linux "page->ptl".

#### Vmalloc boundary

VmallocAllocator.setup() must manage vmalloc/vmap virtual address
resources and metadata. VmallocAllocator.map_page_range() is the
vmap mapping executor and must install VA/PA mappings using the
PageTableCaches/SwapperVm capability already prepared for the
vmalloc range.

#### Vmap subobjects

VmallocAllocator.setup() must establish VmapAreaCache,
VmapAddressSpace, VmapNodeSet, VmapBlockQueues, and VfreeDeferredSet
before VmallocAllocator.Ready is emitted.

#### Mapping record boundary

Every successful map_page_range() action must create a distinct
VmapMapping record bound to the reserved VmapArea, caller-supplied
physical range/PFN/pages, protection, and installed swapper page
table entries. Reusing a global "mapping ready" bit is not enough.

#### Runtime mapping window pool

VmallocAllocator.map_page_range() must support mappings within the
runtime vmalloc address space, including ranges that cross from one
supported L0/PTE window into the next. It must allocate additional
L1/L0 page-table pages and sparse slot metadata on demand through
PageTableCaches/PageAllocator when a reserved vmap area touches an
uninstalled window, reject ranges beyond VMALLOC_END, and reject a
second mapping record for an area that already has an installed
mapping. VmapArea/VmapMapping record storage must use dynamic
container backing and must not fail at the former fixed test-slot
capacity.

#### Vmalloc synchronization contracts

vmalloc_init() initializes per-cpu vmap block queues, deferred vfree
work, vmap nodes and their locks. The current setup path can publish
those structures under SystemExclusive, but runtime get/map/unmap/
free actions must keep their guard, RCU/lazy purge and TLB/cache
contracts visible. Local RISC-V sfence.vma lowering may satisfy the
current single-CPU mapping visibility boundary; remote shootdown,
lazy purge and RCU freeing remain explicit deferred facts.

#### Unmap/free boundary

VmallocAllocator must tear down installed page table mappings before
releasing the corresponding vmap area metadata. The ioremap layer may
request this flow, but the actual VA/PTE teardown remains owned by
vmalloc/vmap.

#### Ioremap/vmalloc split

Ioremap owns the device physical resource source and MMIO attribute
policy. VmallocAllocator owns vmap VA allocation and mapping
execution only; it must not parse DeviceTree/PCI BARs, decide
whether a physical range is mappable, or choose device/non-cache/
write-combine/normal memory attributes.

#### Iounmap boundary

Ioremap.iounmap() must retire the ioremap cookie by requesting
VmallocAllocator.unmap_page_range() followed by free_vm_area().
Ioremap must not clear PTEs itself or maintain vmap free-space
metadata.

#### MMIO attributes

Ioremap must model device/non-cache/write-combine/normal-memory
mapping attributes explicitly. The current RISC-V implementation may
expose only plain device ioremap as supported; the remaining
attributes must be recorded as deferred rather than inferred from a
mapping that happens to be accessible.

#### mm_struct only

MmStructCache.setup() must only establish the "mm_struct" cache as
a named cache instance registered under SlubSubsystem/
SlubCacheRegistry. MmStructCache must not become a separate
allocator type. vm_area_struct, vma lock cache, and mmap_init()
remain later proc_caches_init()/process-memory work.

### ArceosExMmCoreInitCodingShould

#### Checkpoints

mm_core_init() implementation should keep compile-time, report and
trimmed-path checkpoints observable in trace/logging without turning
them into lifecycle states.

<!-- formal-predicate-notes:spec/coding/phases/boot/mm-core-init.spec END -->
