# UserStack coding contract

Model source: [`user_stack.spec`](../../model/objects/user_stack.spec). Implementation:
`impl/arceos_ex/src/objects/user_stack.rs`.

`UserStack` is one of four independent objects: stack backing owner, `UserAddressSpace` VMA/page tables,
`UserTrapFrame`, and `ExecTransaction`. Its fallible `Vec<UserStackPage>` is sorted by virtual page address
and owns every sparse stack `UserFrameRef`. `UserMappingKind::Stack` must not copy a frame reference; it stores only
range, RW/NX permissions and an ownership token referring to the current stack object.

The Linux-default `UserStackConfig` values are 128 KiB initial downward expansion, 8 MiB rlimit, a 256-page
(1 MiB) guard gap, `stack_top_max=0x40000000`, an 8 MiB ASLR window, and 16 user-visible random bytes.
`ExecTransaction` reads exactly 24 HWRNG bytes once before point-of-no-return. Bytes 0..16 are the only
`AT_RANDOM` input. Bytes 16..24 form a little-endian `u64` seed used only by `select_stack_top`: mask it by
`aslr_window / PAGE_SIZE - 1`, multiply by `PAGE_SIZE`, and subtract that offset from `stack_top_max`.
The window is page-sized, power-of-two, no larger than the top/layout gap, and the selected top and rlimit
base must remain above the fixed heap/mmap arena. The ASLR bytes are never copied into user memory.

Initial-stack construction copies the exec filename separately from argv/envp, followed by the HWRNG block
and auxv from high to low. `AT_EXECFN` points to the filename copy even when it differs from `argv[0]`.
The complete supported auxv set is `AT_HWCAP`, `AT_PAGESZ`, `AT_CLKTCK`, `AT_PHDR`, `AT_PHENT`, `AT_PHNUM`,
`AT_BASE`, `AT_FLAGS`, `AT_ENTRY`, `AT_UID`, `AT_EUID`, `AT_GID`, `AT_EGID`, `AT_SECURE`, `AT_RANDOM`,
`AT_EXECFN`, and the final `AT_NULL`. `AT_HWCAP` is derived from `CpuCapabilities.common_isa()` using the
Linux RISC-V letter-bit ABI; clock tick is 100, flags and secure are zero, and credentials are the pre-exec
snapshot (root for boot). Unsupported platform/HWCAP2/rseq/vDSO entries are absent. SP remains 16-byte
aligned. The VMA covers the argument pages plus configured expansion, capped by the selected-top rlimit;
only pages containing initial data are allocated and mapped.

The common `UserAddressSpace` resolver is the only policy entry point. Its stack branch delegates sparse-page
allocation and downward-growth checks to `UserStack`; the stack object does not classify non-stack,
instruction, permission or stale-mm faults. A missing page inside the VMA allocates one sparse slot. An
address below it may move the VMA base only after rlimit, guard-gap and adjacent mapping checks. Multi-page
jumps do not populate intermediate pages. Backing allocation, new L0 allocation and leaf installation form
one rollback unit; failure preserves base, slots, PTE accounting and allocator counts. Success performs
targeted `sfence.vma` and returns `RetrySameInstruction` without advancing `sepc`.

Usercopy resolves every stack page in a valid range through the same mechanism and reports failure as
`EFAULT`; the detailed rejection remains observable through `UserStack.GrowRejected`. Release and exec/current
Task transfers use Rust move/swap. Raw byte copies of `UserStack` are forbidden. Ordinary fork acquires one
checked reference per resident stack page into the unpublished child stack, preserving sorted virtual-page order;
both leaves become RO+COW because the stack VMA is originally writable. Rollback releases staged child references,
and wait/exit never copy stack bytes.

Exec obtains the complete 24-byte value from the current virtio HWRNG before point-of-no-return. No timestamp
or early-mix fallback is allowed. Unavailable/short entropy is `EntropyUnavailable`; runtime maps it to
`EAGAIN`, while boot treats it as terminal and does not try a later init candidate. Existing boot and runtime
checkpoint owners expose top max, selected top, ASLR offset, execfn pointer and a computed auxv-complete fact
without changing checkpoint IDs, names or order.

Dynamic rlimit syscalls and `SIGSEGV/si_code`, multiple thread stacks, `MAP_STACK`/
`MAP_GROWSDOWN`, a complete CRNG, kernel compiler stack protector and
per-task canaries remain separate roadmap work.
