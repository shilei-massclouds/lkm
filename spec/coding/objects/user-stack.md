# UserStack coding contract

Model source: [`user_stack.spec`](../../model/objects/user_stack.spec). Implementation:
`impl/arceos_ex/src/objects/user_stack.rs`.

`UserStack` is one of four independent objects: stack backing owner, `UserAddressSpace` VMA/page tables,
`UserTrapFrame`, and `ExecTransaction`. Its fallible `Vec<UserStackPage>` is sorted by virtual page address
and owns every sparse stack `PageRef`. `UserMappingKind::Stack` must not copy a `PageRef`; it stores only
range, RW/NX permissions and an ownership token referring to the current stack object.

The Linux-default `UserStackConfig` values are 128 KiB initial downward expansion, 8 MiB rlimit, a 256-page
(1 MiB) guard gap, fixed top `0x40000000`, and 16 random bytes. Initial-stack construction copies strings,
the HWRNG block and auxv from high to low, includes `AT_RANDOM`, and aligns SP to 16 bytes. The VMA covers
the argument pages plus the configured expansion, capped by rlimit; only pages containing initial data are
allocated and mapped.

`resolve_user_stack_fault` accepts only the current SATP and load/store access. A missing page inside the VMA
allocates one sparse slot. An address below it may move the VMA base only after rlimit, guard-gap and adjacent
mapping checks. Multi-page jumps do not populate intermediate pages. Backing allocation, new L0 allocation
and leaf installation form one rollback unit; failure preserves base, slots, PTE accounting and allocator
counts. Success performs targeted `sfence.vma` and does not advance `sepc`.

Usercopy resolves every stack page in a valid range through the same mechanism and reports failure as
`EFAULT`; the detailed rejection remains observable through `UserStack.GrowRejected`. Release and exec/parent
snapshot transfers use Rust move/swap. Raw byte copies of `UserStack` are forbidden. Snapshot byte copy uses
the sparse-page read/write API.

Exec obtains the complete 16-byte value from the current virtio HWRNG before point-of-no-return. No timestamp
or early-mix fallback is allowed. Unavailable/short entropy is `EntropyUnavailable`; runtime maps it to
`EAGAIN`, while boot treats it as terminal and does not try a later init candidate.
