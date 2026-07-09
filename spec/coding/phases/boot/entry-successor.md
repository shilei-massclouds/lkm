# EntrySuccessorPhase coding

本文件承载 `spec/coding/phases/boot/entry-successor.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/boot/entry-successor.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`entry-successor.spec`](entry-successor.spec)。

### ArceosExEntrySuccessorCodingMust

#### start_kernel() deferred calls

Linux start_kernel() calls init_vmlinux_build_id() before disabling
local IRQs, then page_address_init() after boot_cpu_init() and
before setup_arch(). EntrySuccessorPhase may defer both objects, but
the phase ready check and implementation-visible facts must preserve
those call positions instead of treating them as absent.

#### RISC-V setup_bootmem() facts

MemBlock.setup() must record the RISC-V setup_bootmem() facts needed
by later boot objects: phys_ram_base, kernel va-pa offset, DMA32
limit/zone input and the hugetlb CMA reserve call position. The
current implementation may defer hugetlb/CMA details, but the
deferral must remain observable.

#### SwapperVm permission boundary

SwapperVm.setup()/enable() must keep the setup_vm_final() SATP/TLB
synchronization fact while recording that CONFIG_STRICT_KERNEL_RWX
final text/rodata/data permission splitting is still deferred. It
must not claim the final RW/RO/NX protection split merely because
the complete kernel address space is online.

<!-- formal-predicate-notes:spec/coding/phases/boot/entry-successor.spec END -->
