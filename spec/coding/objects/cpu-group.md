# CpuGroup coding contract

`CpuGroup` is a direct child of `Kernel`, alongside `Soc`; required boot ordering between them is an explicit
signal/guard dependency and never structural ownership. `Context` owns one `CpuGroup`; it does not store a
separate boot/current CPU or secondary CPU store.
`CpuGroup` owns `[Option<Cpu>; MAX_CPUS]`, which is the only authoritative CPU collection.

AP construction must lower into the authoritative destination `Option<Cpu>` slot with lazy in-place insertion.
It must not first materialize a complete resident `Cpu` (including its Scheduler and Trap emergency stack) as an
automatic value on the current Task stack and then move it into the slot. Slot absence and all topology/key checks
remain prerequisites; destination construction is still unpublished candidate work until the parent handler's
non-failing publication suffix commits the completed AP collection. This stack-safety constraint does not permit a
parallel CPU store, a smaller identity-bearing projection, or deferred construction of any CPU-owned resident
resource.

Creating an indexed element is a parent operation. It validates the logical-ID key, bounds, duplicate slot and
child construction before publishing the element or changing parent facts. Failure leaves the stable collection
and all derived masks unchanged. CPU0 must be created and reach Prepared before the kernel Enable handoff; this
publication does not require its `hartid` to have been assigned. Assigning a hartid later validates uniqueness
against every already assigned element. `setup_smp()` creates AP elements through the same operation.

Possible/present/active/online queries iterate the owned slots or use bitmaps that are provably rebuildable from
the `Cpu` facts. Parallel `cpu_refs`, `CpuView`, or `SecondaryCpuStore` arrays are forbidden. `boot_cpu()` is only
an index-0 convenience accessor and may not own or cache another body.
