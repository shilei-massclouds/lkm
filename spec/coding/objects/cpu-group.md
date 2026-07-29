# CpuGroup coding contract

`CpuGroup` is a direct child of `Kernel`, alongside `Soc`; required boot ordering between them is an explicit
signal/guard dependency and never structural ownership. `Context` owns one `CpuGroup`; it does not store a
separate boot/current CPU or secondary CPU store.
`CpuGroup` owns `[Option<Cpu>; MAX_CPUS]`, which is the only authoritative CPU collection.

Creating an indexed element is a parent operation. It validates the logical-ID key, bounds, duplicate slot and
child construction before publishing the element or changing parent facts. Failure leaves the stable collection
and all derived masks unchanged. CPU0 must be created and reach Prepared before the kernel Enable handoff; this
publication does not require its `hartid` to have been assigned. Assigning a hartid later validates uniqueness
against every already assigned element. `setup_smp()` creates AP elements through the same operation.

Possible/present/active/online queries iterate the owned slots or use bitmaps that are provably rebuildable from
the `Cpu` facts. Parallel `cpu_refs`, `CpuView`, or `SecondaryCpuStore` arrays are forbidden. `boot_cpu()` is only
an index-0 convenience accessor and may not own or cache another body.
