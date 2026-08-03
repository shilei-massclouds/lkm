# User boot coding constraints

KernelInitTask retains KernelInitFlow for its entire lifetime. KernelInitFlow owns one stable
UserAppRuntime. PID 1 exec replaces only `runtime.application`; successful replacement preserves the
identities and generations of Task, TaskRef, Flow, FlowRef and Runtime.

Fork allocates a fresh Task, TaskRef, UserTaskFlow, FlowRef, UserAppRuntime and ApplicationInstance. The
creation order is structural bind, Task context setup, Runtime publication, Flow publication, runqueue
publication, then Task publication. No child shares Runtime or Flow storage with its parent or siblings.

Before runqueue or Task publication, ordinary fork also creates a fresh `UserAddressSpace`, root page table,
SATP and `UserStack`. The eager transition copies every resident private page into a fresh frame and preserves
holes as holes. Allocation or PTE installation failure releases the unpublished child mm and Task slot in
reverse order without changing the parent.

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
`UserFaultClass` is a closed representation: `NotPresent`, `Protection`, or `Unmapped` in this slice; the later
COW slice adds `CowWriteProtect` without changing existing meanings. The only successful result is
`RetrySameInstruction { sepc }`, and its `sepc` must equal the request value.
Ordinary trap and syscall usercopy resolve that Task through validated `CurrentTask`. A synchronous terminal
child-to-parent handoff may instead pass the parent TaskRef already validated and committed by the switch
transaction while the full parent TaskFlow selector is not yet observable; the explicit TaskRef is scoped to
that copyout/resumed read and is still rejected unless the live SATP matches the target mm.

VMA extent and sparse backing extent are separate fields. ELF, stack, heap and anonymous-private VMAs are
non-overlapping; brk expansion and anonymous mmap create or enlarge VMAs without eagerly allocating leaves.
Each sparse non-stack backing slot stores both its virtual page and `PageRef`. A successful NotPresent fault
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

The stable fault diagnostic records Task/mm, VA, `sepc`, access, class, result, and zero COW counters for this
slice. It is internal diagnostic state and does not create, reorder or rename external checkpoints. Kernel
exception-table recovery never calls this classifier and never consumes its state.
