# User boot coding constraints

KernelInitTask retains KernelInitFlow for its entire lifetime. KernelInitFlow owns one stable
UserAppRuntime. PID 1 exec replaces only `runtime.application`; successful replacement preserves the
identities and generations of Task, TaskRef, Flow, FlowRef and Runtime.

Fork allocates a fresh Task, TaskRef, UserTaskFlow, FlowRef, UserAppRuntime and ApplicationInstance. The
creation order is structural bind, Task context setup, Runtime publication, Flow publication, runqueue
publication, then Task publication. No child shares Runtime or Flow storage with its parent or siblings.

Before runqueue or Task publication, ordinary fork also creates a fresh `UserAddressSpace`, root page table,
SATP and `UserStack`. Resident private pages are shared through checked `UserFrameRef`s while holes remain holes.
For a VMA that was originally private-writable, both parent and child leaf PTEs are read-only with `PTE_COW`;
originally read-only pages are shared read-only without COW promotion. Allocation, reference acquisition or PTE
installation failure releases the unpublished child mm and Task slot in reverse order without changing the parent.

Fork is a prepare/commit transaction. Prepare allocates every child page-table page, acquires every child frame
reference, builds all child leaves, and revalidates each parent leaf/physical-frame/VMA tuple. Commit lowers the
validated parent writable leaves to RO+COW, performs targeted `sfence.vma`, and only then publishes the child.
Nested fork accepts an already RO+COW parent leaf when its original private VMA is writable. No fallible operation
may occur after the first parent PTE change; rollback before that point releases only staged child resources.

The concrete current-mm fields may act as a single active carrier so existing syscall and trap code can borrow
one stable address. Ownership is nevertheless keyed by TaskRef: switching out moves the complete mm/stack value
to that Task's storage, switching in moves the destination Task's value to the carrier, installs its distinct
SATP and executes `sfence.vma`. A carrier is valid only when its owner TaskRef and SATP match CurrentTask. PID 1
has an equally explicit inactive storage when a child is running. Ordinary fork/wait/exit must not retain or
restore writable-page, stack-byte or whole-address-space snapshots. The fd-table compatibility snapshot is a
separate FilesStruct responsibility and is unaffected by this mm rule.

Exec staging atomically replaces the active Task's carrier mm. A successful commit releases the retired image
once; a failed commit leaves the Task-owned mm untouched. Exit releases the active mm before returning to the
parent mm; reap releases only the destroyed Task record. Inactive or unpublished mm storage must be empty after
move/release, making repeated teardown a deterministic rejection.

Syscall and user-mode traps are effective-flow children above the lifetime TaskFlow. They may schedule;
on return, the scheduler restores the saved trap leaf from TaskThreadContext before resuming the
TaskFlow-level continuation.

Exit quiesces the ApplicationInstance and Runtime, disables and cleans the fixed Flow, then disables and
cleans the Task. A pending yield must be resolved or terminated before Flow cleanup.

`UserAddressSpace` owns the common user-fault classifier. `UserFaultRequest` carries the current Task identity,
the current address-space/SATP identity, fault VA, original `sepc`, and instruction/load/store access.
`UserFaultClass` is a closed representation: `NotPresent`, `CowWriteProtect`, `Protection`, or `Unmapped`.
`CowWriteProtect` requires a store fault, a present RO leaf carrying `PTE_COW`, and an originally writable private
VMA whose sparse backing resolves to the same `UserFrameRef`; no other write-protected leaf enters COW. The only successful result is
`RetrySameInstruction { sepc }`, and its `sepc` must equal the request value.
Ordinary trap and syscall usercopy resolve that Task through validated `CurrentTask`. A synchronous terminal
child-to-parent handoff may instead pass the parent TaskRef already validated and committed by the switch
transaction while the full parent TaskFlow selector is not yet observable; the explicit TaskRef is scoped to
that copyout/resumed read and is still rejected unless the live SATP matches the target mm.

VMA extent and sparse backing extent are separate fields. ELF, stack, heap and anonymous-private VMAs are
non-overlapping; brk expansion and anonymous mmap create or enlarge VMAs without eagerly allocating leaves.
Each sparse non-stack backing slot stores both its virtual page and `UserFrameRef`. A successful NotPresent fault
allocates and zeroes exactly the requested page, installs one leaf, then executes targeted `sfence.vma`.
Backing insertion, optional L0 allocation and PTE publication are one transaction: on failure, undo them in
reverse order without changing the prior VMA, leaf, allocator count, or bytes.

The first anonymous-private `munmap` slice accepts exactly one complete, page-aligned anonymous VMA. It
prevalidates every resident leaf against the VMA backing before mutation, clears those leaves, performs a
targeted `sfence.vma` for each resident page, releases the backing `PageRef`s, and clears the descriptor in
place. Anonymous mmap prefers empty descriptor slots and first-fit virtual holes;
trailing empty slots also reduce the table high-water mark. Partial ranges and
non-anonymous VMAs fail before mutation. Empty L0 page-table pages remain mm-owned until address-space
teardown; removing their last leaf does not transfer or leak that ownership.

RISC-V leaf bit 8 (the low software-reserved RSW bit) is `PTE_COW`; bit 9 remains zero and reserved for future use.
A shared COW write allocates and copies one page, prepares a writable non-COW replacement leaf, atomically swaps
the backing owner and PTE, flushes that VA, then releases the old reference. When the checked old count is one,
the fast path only clears COW, restores write permission and flushes that VA. Allocation or commit failure leaves
the old owner/count/PTE/data intact and terminates only the faulting Task as `OutOfMemory`; its parent observes a
signal-9 wait word and SIGCHLD, with no SIGSEGV, user signal frame, OOM killer selection or kernel panic.

The stable fault diagnostic records Task/mm, VA, `sepc`, access, class, result, checked refcount, copy count and
unique fast-path count. It is internal diagnostic state and does not create, reorder or rename external checkpoints. Kernel
exception-table recovery never calls this classifier and never consumes its state.
