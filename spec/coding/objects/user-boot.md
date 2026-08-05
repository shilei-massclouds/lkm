# User boot coding constraints

KernelInitTask retains KernelInitFlow for its entire lifetime. PID 1 occupies registry slot zero, fixed to CPU0,
and its stable aggregate owns KernelInitFlow and one UserAppRuntime. PID 1 exec replaces only aggregate contents
and `runtime.application`; successful replacement preserves the identities and generations of Task, TaskRef,
Flow, FlowRef, Runtime, CpuRef and the address-space object.

At the no-return first user-entry handoff, PID 1 adopts the prepared user kernel-trap stack as its TaskThreadContext
stack and installs the Task stack guard there before the CPU trap entry context is refreshed. The earlier kernel-init
startup stack is no longer a resumable continuation. Consequently a syscall that blocks in Scheduler may save and
later restore PID 1 only through the same Task-owned trap stack used by every subsequent user trap.

`UserProcessRegistry` has exactly 32 slots including PID 1. A reusable slot contains state, nonzero generation,
fixed CpuRef, lease count and an optional heap-allocated `UserProcessAggregate`. The aggregate owns Task,
TaskFlow, Runtime/Application, `UserAddressSpace`, `UserStack`, `UserTrapFrame`, files/fs, credentials, signal,
wait/zombie and exec state. Published lookup returns a `UserProcessLease` only after TaskRef slot/generation and
caller CPU match; drop decrements the lease count. Reap increments generation and frees the aggregate only after
the Task is absent from every runqueue, inbox and CurrentTask binding and the lease count is zero.

The registry lock also protects the task-list identity index needed by the first-slice process/session ABI:
parent PID, process-group ID, session ID, session-leader state and controlling-TTY ownership. PID 1 starts as
session and process-group leader. A fork reservation copies those fields from the exact generation-checked parent
slot before publication. `setsid` validates the current published TaskRef and owner CPU, rejects a current session
leader or any still-live process group whose ID equals the caller PID, and otherwise changes the caller's session
and process group to its PID while clearing controlling-TTY ownership in the same critical section. Current-task
`getppid`, `getpgid`, `getsid`, `setpgid` and `setsid` must resolve this index. `setpgid` may update only the exact
current occurrence or its exact published child; a pgrp equal to the target PID is created directly, while any
other target pgrp requires a live same-session registry member. Validation and mutation occur under the same
registry guard. During the temporary CPU0 serial vfork handoff, a successful legacy child identity update is
mirrored to that exact generation-checked registry occurrence before it can reserve a descendant; the legacy
snapshot must not stand in for a different published registry occurrence.

For the single first-slice console TTY, the same registry guard protects the TTY session ID and foreground
process-group ID. `TIOCSCTTY` validates that the generation-checked current process is a session leader without a
controlling TTY, then binds the TTY to that process's session and initial foreground pgrp. `TIOCSPGRP` accepts only
a live registry pgrp in the TTY/current session and updates the shared foreground ID; `TIOCGPGRP` and `TIOCGSID`
read that shared state only for a current process that owns the controlling TTY. These operations must not read or
update the CPU0 legacy child's foreground/session snapshot for an independent current aggregate.

The bounded `writev` representation imports at most four vectors of at most 256 bytes each into one fixed
1024-byte staging buffer before the `FilesStruct -> FileDescriptorTable -> OpenFileDescription -> FileBackend`
write. A console-backed vector is therefore one write batch, matching Linux's single imported `iov_iter` under
the TTY atomic-write lock rather than one independently interleavable write per iovec. `Printk` acquires its
IRQ-safe write lock before either the UART route or the `stress_mem` observation route. Named checkpoints and
runtime diagnostics first format one bounded record and submit it through that same locked batch path; no direct
multi-call SBI fragment sequence may interleave with a user console batch.

Fork stages a fresh Task, TaskRef, ordinary TaskFlow, FlowRef, UserAppRuntime and ApplicationInstance in one
private aggregate candidate. A checked `materialize_fork_child_snapshot` constructor fills the embedded Flow,
owner/parent links, independent generations, stack, post-fork TaskThreadContext, Runtime/application binding,
PID, target CPU and resource references, then performs exactly one publication after scheduler-eligibility and
target-inbox reservation. It commits the
child directly as `Online/Online/Online`; it must not call Task, Flow, or Runtime Preset/Setup/Enable. No child
shares any of these identities or storage with its parent or siblings.

The constructor accepts only a current `OnCpu/Live` parent whose Flow and Runtime are Online. The parent remains
`OnCpu/Online/Online`; the child is published `Online/Online/Online`, with return register 0 while the parent gets
the child PID. The child's context contains a fresh FlowRef/generation and rebuilt post-fork continuation, but no
parent TrapFlowRef, YieldToken, CurrentTask binding, CPU execution authority, or trap stack. Validation failure at
any stage returns `EAGAIN` for PID/slot/inbox exhaustion or `ENOMEM` for aggregate/mm/frame allocation failure,
releases the staged Task slot, PID, page tables, frame references and inbox reservation, and leaves parent PTEs,
counters, runqueues and all public registries unchanged.

Plain-fork parent resolution starts from the generation-checked current Task occurrence and then resolves that
occurrence's effective current-mm binding. An ordinary aggregate resolves its own stable address-space field; a
same-CPU serial vfork/CLONE_VM child resolves the still-live shared parent mm selected by its parent-blocking
handoff. The latter must not be rejected merely because its Task slot has no independent mm. In both cases the
plain-fork child receives a fresh independent COW address space, and the source mm must equal the live SATP before
any fork preparation or parent PTE mutation begins.

Every published aggregate stores either `mm_present == true` with no shared owner, or `mm_present == false` with
a direct `shared_mm_owner_ref` naming the final generation-checked aggregate that owns the live mm. A vfork child
also stores its exact `vfork_parent_ref` and one pre-reserved `Wake` inbox record for that parent. These fields are
initialized only in the unpublished candidate; no lookup may fall back to `Context.user_task_set` for a published
registry occurrence. Effective-mm resolution validates the current Task identity and owner CPU first, then the
owner registry generation, same CpuRef, Online address-space state and current SATP before returning stable field
capabilities. Process resources always resolve from the current aggregate, not from the effective mm owner.

The vfork prepare transaction reserves the same-CPU child `Activate` record followed by the parent's `Wake`
record before registry publication. It copies the current aggregate's files/fs references, materializes a fresh
Task/Flow/Runtime/kernel-stack snapshot with the child return register set to zero, publishes and locally consumes
the activation, then declares the parent asleep and calls that CPU's Scheduler. Failure before publication rolls
back both reservations and all acquired resources. Nested vfork repeats this transaction with a fresh child and
its own immediate-parent wake; the shared owner link is flattened to the final mm owner rather than chained.

Runtime `execve` user-copy applies the same current occurrence/effective-mm resolution to the filename, every
`argv`/`envp` pointer slot, and every referenced string. No vector-slot precheck may consult the legacy PID1
`Context.user_address_space` when a registry Task is current; the range proof and the volatile copy must resolve
the same generation-checked Task/mm and live SATP identity.

Registry storage distinguishes a private `Reserved` slot from a `Published` slot; TaskRef and FlowRef
resolution never sees the former. Runtime storage address plus a nonzero application generation realizes an
ApplicationInstance identity. Fork starts a fresh Runtime at application generation 1; exec preserves Task,
TaskFlow and Runtime storage/refs and advances only that generation.

Each aggregate owns a private fixed-size fd table. An inherited pipe entry stores a generation-checked
`SharedPipeRef` into a bounded static pipe registry rather than embedding or copying pipe bytes in `FilesStruct`.
The referenced slot owns the ring buffer and checked reader/writer endpoint counts under an IRQ-safe lock.
Ordinary fork and fd duplication acquire every copied endpoint before publication; close, CLOEXEC removal and
process teardown release exactly the removed endpoint. A failed fork or duplicate reverses all acquired endpoint
references before releasing its unpublished aggregate or fd reservation. A pipe slot may be reused with an
advanced nonzero generation only after both endpoint counts reach zero. Empty reads return EOF only after the
last writer is released, and writes return `EPIPE` only after the last reader is released. The legacy serial
parent-fd snapshot may preserve its table bookkeeping, but it neither owns pipe storage nor substitutes for these
cross-process references.

Each shared pipe slot also contains a bounded waiter array sized for all 32 user Tasks. A waiter owns a
generation-checked `{TaskRef, CpuRef}` and a pre-reserved `Wake` inbox record. Registration occurs under the pipe
lock only after a second empty-buffer/live-writer check. A read that finds data or EOF during that check cancels
the reservation without sleeping. The first write that changes empty to readable, and the release that changes
the writer count to zero, detach matching waiters under the pipe lock; publication and IPI occur only after both
pipe and per-process files guards are dropped. Resumed reads always repeat fd lookup and shared-backing read.
Waiter removal on fd close, task teardown or stale generation rolls its reservation back exactly once.

The wait4 implementation normalizes PID `-1` to an all-child selector and a positive PID to an exact-child
selector; PID zero and other negative selectors remain outside the first slice. Registry child-existence,
zombie-selection and same-owner-CPU runnable-child queries take the same selector and always combine it with the
generation-checked current parent TaskRef and parent PID. Completed-child records and compatibility continuation
handoffs apply that selector before status copyout, scheduling or reap. An unrelated completed record must not
mask an exact matching registry child, and an exact selector with no matching child returns `ECHILD` even when
the parent has other children. Registry reap clears only the selected zombie's slot, scheduler and resource
references. It must not reset the current parent's compatibility identity, PID, continuation or active-record
fields merely because both occurrences share the bounded task-storage manager.

After registration the current owner CPU declares Task scheduler sleep and calls its local Scheduler. Inbox
consumption at user-return/idle safe points distinguishes the race sides: a wake for the current sleep-declared
Task becomes its one pending wake signal, while a wake for an already blocked Online Task restores runqueue
eligibility. PreparePrev consumes a matching pending signal instead of blocking. An already runnable/enqueued or
generation-stale wake is consumed without a second enqueue. No Scheduler call occurs while the files or pipe lock
is held.

Before runqueue or Task publication, ordinary fork also creates a fresh `UserAddressSpace`, root page table,
SATP and `UserStack`. Resident private pages are shared through checked `UserFrameRef`s while holes remain holes.
For a VMA that was originally private-writable, both parent and child leaf PTEs are read-only with `PTE_COW`;
originally read-only pages are shared read-only without COW promotion. Allocation, reference acquisition or PTE
installation failure releases the unpublished child mm and Task slot in reverse order without changing the parent.

Fork snapshot materialization is a prepare/commit transaction. Prepare allocates every child page-table page, acquires every child frame
reference, builds all child leaves, and revalidates each parent leaf/physical-frame/VMA tuple. Commit lowers the
validated parent writable leaves to RO+COW, performs targeted `sfence.vma`, and only then publishes the child.
Nested fork accepts an already RO+COW parent leaf when its original private VMA is writable. No fallible operation
may occur after the first parent PTE change; rollback before that point releases only staged child resources.

Each address-space and stack value remains at a stable address inside its owning aggregate for its entire life;
there is no active carrier and no whole-mm swap. Syscall, trap, page-fault and usercopy code reach them only
through the current `UserProcessLease`. Switching in installs the destination aggregate's SATP and executes local
`sfence.vma`; switching out leaves all aggregate values in place. Ordinary fork/wait/exit must not retain or
restore writable-page, stack-byte or whole-address-space snapshots.

Exec stages replacement contents against the leased address-space object. A successful commit releases the
retired image once while preserving object identity; a failed commit leaves the Task-owned mm untouched. Exit
first installs `SwapperVm` SATP on the exiting CPU and performs a local `sfence.vma`, then publishes the zombie
status and SIGCHLD with release ordering and queues a wake record to the parent's fixed CPU. Zombie publication
alone is not a reap grant. After the physical switch, code running on the selected next Task's stack
release-publishes the terminal Task's scheduler-quiesced acknowledgement; only that acknowledgement proves that
the old mm, CurrentTask binding and kernel stack are no longer live on the owner CPU. Wait4 acquire-observes both
completion and this acknowledgement and is the only reap owner. A caller-supplied boolean must not substitute for
the acknowledgement. Repeated terminal, acknowledgement or reap operations are rejected.

For a vfork child, exec stages into and commits the child's currently Base address-space and stack fields. It does
not copy the effective parent mm into the exec transaction's retired fields. After owner rebinding, SATP switch and
return-frame preparation, commit sets `mm_present`, clears `shared_mm_owner_ref`, takes the parent wake reservation
and publishes/locally consumes it exactly once. Exit takes and publishes the same reservation without releasing an
effective mm it does not own. Slot rollback/reap requires that no unconsumed parent wake or shared-owner link can
survive reuse. Same-CPU inbox consumption supports CPU0 and AP schedulers and resolves dynamic registry Tasks even
when CPU0 also carries the legacy PID1 task set.

Wait4 derives its parent `{TaskRef, pid, CpuRef}` from the current generation-checked process occurrence. Every
zombie lookup, same-CPU child dispatch decision and reap therefore uses that exact parent identity and its owner
Scheduler; a dynamic parent must never substitute PID1, `KernelInitTask`, CPU0 or `Cpu0Scheduler` for itself.

SMP online freezes the ordered online CpuRef array by logical id. Ordinary fork selects
`online_cpus[child_pid % cpu_count]`; that CpuRef is immutable for the child's lifetime. vfork/CLONE_VM selects the
parent CpuRef and blocks the parent until child exec/exit completes. No affinity ABI, migration, runtime hotplug,
automatic balancing or concurrent shared-mm cross-CPU execution is represented.

Syscall and user-mode traps are effective-flow children above the lifetime TaskFlow. They may schedule;
on return, the scheduler restores the saved trap leaf from TaskThreadContext before resuming the
TaskFlow-level continuation.

Exit quiesces the ApplicationInstance, disables and cleans the Runtime, disables and cleans the fixed Flow, then disables and
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

For a real user instruction/load/store trap, `SegvMaperr` lowers to
`TaskTerminalReason::SegmentationFault` plus `TaskSegvInfo { code: Maperr, address: stval }`;
`SegvAccerr` lowers identically with `code: Accerr`. The signal number is the constant 11, and
`TaskSegvInfo.address` is copied from the same `UserFaultRequest.address` that produced the result. The trap handler
does not advance `sepc`: after recording the terminal it invokes the existing child-exit handoff with logical exit
status zero, while `task_wait_status` selects the low-bit signal-11 encoding from the terminal reason. The handoff
releases the faulting mm once, resumes/copies status to the parent wait and queues SIGCHLD; reap releases only the
destroyed Task slot. A stable terminal diagnostic exposes task identity, signal number, code and address without
adding or reordering checkpoints.

`InvalidContext` remains a kernel invariant failure and must not be relabeled as SIGSEGV. Syscall usercopy may use
the same classifier for range preparation, but `SegvMaperr`/`SegvAccerr` there remain a false/EFAULT result because
no user instruction trap occurred. OOM remains the disjoint signal-9 Task terminal. This slice does not inspect the
registered action table, create a signal frame, enter a handler, implement `rt_sigreturn`, or write a core image.
