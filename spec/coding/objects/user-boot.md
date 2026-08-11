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

Credential syscalls resolve the exact current TaskRef and fixed CpuRef, acquire a `UserProcessLease`, then take
the dedicated aggregate-credentials IRQ-safe lock before reading or mutating ids or supplementary groups. A
dynamic slot owns its credential value. While the child remains private in `Reserved`, a preparer-only registry
query may resolve that reservation's exact parent TaskRef after checking the child generation and `Reserved` state;
ordinary TaskRef/FlowRef lookup must still reject the child. The preparer acquires a lease on the published parent,
then copies its credentials before the child becomes visible. PID 1 follows the same lease/lock route to its stable
aggregate; no current child may read
or mutate `Context.kernel_init_user_state` as a global credential carrier. UID/GID getters, res-id getters,
`getgroups`, and the bounded root-euid setters all use this route.

Signal syscalls use the same occurrence resolution rule but a distinct IRQ-safe signal lock. Every published
aggregate owns one `SignalRuntime` containing 64 three-word RISC-V `rt_sigaction` entries, a 64-bit blocked mask,
a 64-bit pending mask, one `ITIMER_REAL { deadline_ticks, interval_ticks }`, and one active-frame record. PID 1
stores the same value in its stable registry aggregate; no dynamic process may read or mutate the legacy
`Context.kernel_init_user_state` signal fields. Ordinary fork copies only the action table and blocked mask while
the child is still private; its pending mask, timer and active-frame record start empty. Remote pending-bit
publication holds a live target `UserProcessLease` and the signal lock, but never an owner-CPU aggregate write
capability. The lock order is registry lease, signal lock, then—only after dropping the signal lock—scheduler inbox
publication. Calling Scheduler or sending an IPI while the signal lock is held is forbidden.

RISC-V syscalls 134 and 135 retain their existing 24-byte kernel action and 8-byte kernel sigset layouts,
respectively, but route all reads and commits to that current aggregate. The RISC-V kernel action words are
`handler, flags, mask`; unlike obsolete-restorer architectures, RISC-V does not expose a restorer word in this
kernel ABI. Signal bits use `1 << (signal - 1)`; `SIGKILL` 9 and `SIGSTOP` 19 are removed from masks and reject
installed handlers. Delivery implements `SA_SIGINFO=0x4`, `SA_RESTART=0x10000000`,
`SA_NODEFER=0x40000000` and `SA_RESETHAND=0x80000000`; other Linux-accepted disposition flags may be retained but
have no effect outside their represented semantics. Unsupported flag bits are cleared before storing the action,
matching the bounded Linux `do_sigaction()` route.

RISC-V syscall 103 decodes `setitimer(which, new_value, old_value)`. Only signed `which == ITIMER_REAL(0)` is
accepted; other values return `EINVAL`. `new_value`, when non-null, imports exactly 32 bytes as four signed i64
fields in `{ interval_sec, interval_usec, value_sec, value_usec }` order. Seconds must be nonnegative and each
microsecond field must be in `0..1_000_000`; bad values return `EINVAL`, address failure returns `EFAULT`, and
tick conversion rounds a nonzero duration up so it cannot become an immediate zero interval. Under the signal
lock, snapshot the old remaining/interval value against the current monotonic tick, then atomically install,
replace or cancel the timer. A null `new_value` is query-only. Copy the staged old value to `old_value`, when
non-null, only after the new state is committed; an `EFAULT` from that copyout does not roll the new timer back.
Reprogram the local clockevent mux after releasing the signal lock. Expiration merges `SIGALRM` bit 14, advances a
periodic deadline by enough whole intervals to lie in the future, or clears a one-shot timer.

RISC-V syscall 129 decodes `kill(pid, signal)` as signed 32-bit values. This slice accepts a positive PID naming an
exact published occurrence; absent or stale occurrences return `ESRCH`, signal outside `0..64` returns `EINVAL`,
and the root-effective-UID permission check succeeds. Signal zero performs only existence and permission checks.
A successful nonzero send merges the target pending bit while the target lease remains live, then publishes a
generation-checked `Wake` record to its immutable CpuRef and requests that CPU's IPI after dropping the signal
lock. Inbox consumption records a pending wake when the target is still `OnCpu` or has merely declared sleep, and
enqueues exactly once when it is already blocked `Online`; `PreparePrev` consumes a matching earlier wake instead
of sleeping. Thus the final pending-mask recheck and sleep transition have no lost-wake window.

Every address space maps one private page at fixed VA `0x3e00_0000`, outside ELF/interpreter, heap/mmap and the
minimum randomized stack/guard range. The mapping kind is `SignalTrampoline`; it owns one frame, is user-readable
and executable but not writable (RX/NW), and contains little-endian instructions `0x08b00893` (`addi a7,zero,139`)
and `0x00000073` (`ecall`) followed by zeroes. Setup, fork COW duplication, exec replacement, failure rollback and
teardown must account for that frame exactly once. It is never made COW/writable, and the user stack remains RW/NX.

Before a user return, delivery selects the lowest unblocked pending signal under the signal lock. `SIG_IGN`
dequeues it; a default-terminal action records the signal-form wait status and uses the existing terminal handoff.
For a handler, delivery requires no active frame, snapshots the action and old mask, applies the action mask plus
the delivered signal unless `SA_NODEFER`, resets the action when `SA_RESETHAND`, and reserves a 16-byte-aligned
1088-byte frame below the current user SP. Nested deliverable signals remain pending until that frame returns.
The frame layout is Linux RISC-V `rt_sigframe`: 128-byte `siginfo_t` at offset 0 and 960-byte `ucontext_t` at
offset 128; `uc_sigmask` is at absolute offset 168 and `uc_mcontext` at absolute offset 304. The mcontext begins
with 32 little-endian u64 integer values in `pc, x1, ..., x31` order. Delivery checks and writes the whole range,
sets `sp` to the frame base, `a0` to the signal, `a1` to the siginfo address, `a2` to the ucontext address, `ra` to
`0x3e00_0000`, and `sepc` to the handler. A signal interrupting the bounded blocking wait4 restores the same ecall
PC and original argument registers when its action has `SA_RESTART`; otherwise the saved continuation returns
`-EINTR`. No generic restart block is represented.

RISC-V syscall 139 is accepted only while the exact current aggregate owns an active frame whose recorded base
equals the current frame base. It imports the saved mask and integer context from that 1088-byte user frame,
clears `SIGKILL`/`SIGSTOP` from the restored mask, validates a user `sepc` and aligned user `sp`, clears the active
record, and restores `pc` plus x1..x31 without ordinary syscall-result assignment or four-byte `sepc` advance.
An invalid/missing frame follows the Linux bad-frame terminal path rather than returning forged success.

`fchownat(AT_FDCWD, path, uid, gid, flags)` first performs checked path usercopy, validates the bounded flags and
relative-dirfd slice, then snapshots the current aggregate credentials through that lease/lock route. Only the
root-euid proxy may mutate ownership. Under the files/fs guard it resolves the path and updates the transient
writable inode atomically; `u32::MAX` preserves the corresponding id. Failure leaves both ids unchanged. Unknown
flags return `EINVAL`; bad relative dirfd, empty/overlong/faulting paths, lookup failures, read-only backing and
authorization failure retain distinct errno classes. Directory-fd traversal and `AT_EMPTY_PATH` remain deferred.

Legacy `fchmodat(AT_FDCWD, path, mode)` syscall 53 applies the same checked path, current-credential lease and
files/fs guard order. Absolute paths ignore dirfd; a relative path with any other dirfd returns `EBADF`. The VFS
operation follows the final symlink, rejects read-only backing, and commits `(old_mode & S_IFMT) |
(mode & 07777)` in one mutation so both path stat and opened-file stat observe the new permissions without losing
the inode type. EFAULT, ENOENT, ENAMETOOLONG, ENOTDIR, ELOOP, EROFS and EPERM remain distinct; all failures retain
the old mode. The current authorization slice admits the exact current root-euid aggregate only.

Native RISC-V `statfs(path, buf)` syscall 43 performs checked pathname import and, under the files/fs lock,
follows the path through `VfsCore` to its actual superblock before examining `buf`. The first supported slice is
an ext2-bound path, including a transient overlay inode whose superblock is ext2. Its 120-byte little-endian
native layout contains 64-bit words at Linux asm-generic offsets, reports `EXT2_SUPER_MAGIC`, the parsed ext2
block size and total block/inode counts, `VFS_NAME_MAX`, equal fragment/block sizes, and `ST_VALID`; free-space,
free-inode and fsid fields remain zero because this VFS does not maintain those accounting facts. Missing,
overlong, non-directory, symlink-loop and backend path failures retain their distinct errno mapping. Path
resolution precedes output-pointer validation as in Linux 6.12 `user_statfs()`/`do_statfs_native()`. The complete
result is staged in kernel memory and copied only after successful resolution, so `EFAULT` never publishes a
partially constructed result. Other filesystem kinds and live ext2 free-space accounting remain unsupported.

For an existing read-only ordinary pathname, `openat(AT_FDCWD, ...)` accepts
`O_NOFOLLOW` together with the already admitted `O_DIRECTORY`, `O_LARGEFILE`
and `O_CLOEXEC` bits. The flag is carried explicitly into the VFS lookup: all
parent components use the ordinary symlink-follow walk, while the final
component is looked up without following it. A final symlink maps to `ELOOP`;
`O_DIRECTORY` still maps a resolved non-directory to `ENOTDIR`. `O_NOFOLLOW`
does not widen the bounded create, TTY or `/dev/null` flag shapes.

For an ordinary pathname, the first writable `openat` slice accepts only
`AT_FDCWD` plus `O_CREAT|O_EXCL|O_RDWR`, with optional `O_LARGEFILE` and
`O_CLOEXEC`, and a mode whose low `07777` bits are stored on the new inode.
After checked pathname import it snapshots fsuid/fsgid through the current
generation/CPU-checked process lease and aggregate credential lock, then takes
the files/fs/VFS guard. `FilesStruct` owns a bounded pool of two independent
regular-file OFDs. It must select an OFD with no live fd-table references and
reserve the lowest free fd slot before calling the VFS exclusive-create
transaction. The existing `/opt/ltp/runtest/syscalls` input may therefore
remain inherited on fd 0 while the LTP IPC file receives a distinct OFD. Only
exhaustion of both OFDs or the fd table maps to `EMFILE`; the implementation
must not close, overwrite or reuse a still-referenced OFD. Both a transient existing name and an ext2-backed existing
name map to `EEXIST`; VFS allocation exhaustion maps to `ENOSPC`, fd exhaustion
maps to `EMFILE`, and failures before fd installation leave no new pathname.
The installed fd is readable and writable according to `O_RDWR`, even though
general regular-file write/truncate persistence remains outside this slice.
The bounded current umask is zero; general umask, `O_CREAT` without `O_EXCL`,
ordinary-path `O_TRUNC` and other create flag combinations remain deferred.

RISC-V syscall 46 (`ftruncate`) first interprets the length as signed `off_t`:
a negative value returns `EINVAL` before fd lookup. The current aggregate's
files/VFS resource guard then resolves the fd. A missing fd returns `EBADF`; a
non-regular or non-writable entry returns `EINVAL`. Only a transient
memory-backed Regular0/Regular1 file is mutable in this slice. Lengths through
the fixed 64 KiB regular-file capacity resize the VFS inode atomically,
zero-fill extension, discard a shrunken suffix, preserve every shared OFD file
position and update the selected `FilesStruct` cache length. A larger length
returns `EFBIG`; reserve failure returns `ENOSPC`; neither failure changes inode
size, bytes or cached length. There is no user pointer and therefore no
`EFAULT` path for this syscall.

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

After publication, the target CPU may consume the reserved activation and move the child between `Online` and
`OnCpu` while the publishing CPU completes legacy compatibility bookkeeping. A compatibility enqueue mark accepts
either state, but its eligibility predicate must read the child lifecycle exactly once and test that one snapshot;
it must not combine repeated unlocked reads across a concurrent `OnCpu -> Online` or `Online -> OnCpu`
transition. PID and active-record checks follow the same captured lifecycle decision before the compatibility
`enqueued` bit is published.

The constructor accepts only a current `OnCpu/Live` parent whose Flow and Runtime are Online. The parent remains
`OnCpu/Online/Online`; the child is published `Online/Online/Online`, with return register 0 while the parent gets
the child PID. The child's context contains a fresh FlowRef/generation and rebuilt post-fork continuation, but no
parent TrapFlowRef, YieldToken, CurrentTask binding, CPU execution authority, or trap stack. Validation failure at
any stage returns `EAGAIN` for PID/slot/inbox exhaustion or `ENOMEM` for aggregate/mm/frame allocation failure,
releases the staged Task slot, PID, page tables, frame references and inbox reservation, and leaves parent PTEs,
counters, runqueues and all public registries unchanged.

For the CPU0 compatibility publication path, runqueue selection and enqueue use the child PID and TaskRef returned
by that completed snapshot. The caller must not mutate Scheduler selection from `next_child_pid()` before the
snapshot chooses its actual registry occurrence. A remote child remains owned by its reserved inbox path and does
not leave a speculative CPU0 selection record behind.

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

Each live Regular0/Regular1 fd-table entry likewise owns one reference to a generation-checked shared regular
OpenFileDescription slot. The bounded static registry stores the authoritative file position and a total reference
count; the aggregate keeps only the slot reference plus its private fd table and bounded content/path cache. A new
open allocates a fresh slot even when another open names the same inode. `dup` acquires one reference for the new
entry, ordinary fork acquires one reference for every copied entry, and close, CLOEXEC and process teardown release
exactly the entries they remove. A saved parent-fd table acquires its own references; restore transfers those held
references into the restored table without restoring or rewinding the shared position, while discard releases them.
All multi-reference acquisition paths roll back earlier acquisitions on failure before publication. A slot is
reusable only after its count reaches zero and its nonzero generation advances on the next allocation.

Regular-file read and lseek, and directory getdents/lseek, resolve the exact shared slot while holding the existing
files/fs resource lock, then serialize position validation and update under the shared-OFD registry lock. Read
reserves and advances only the bytes it actually returns; EOF does not advance. Invalid/stale references are
`EBADF`-class failures and failed seek validation leaves the position unchanged. No code path may use a copied
`FilesStruct.regular*_offset` or `directory0_offset` as the authoritative post-fork position.

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
The selected child's UserAppRuntime, fixed TaskFlow and Task lifecycle are already Destroyed by terminal exit
before scheduler quiescence grants reap. Wait4 validates those terminal states and releases the aggregate record;
it must not repeat Runtime, Flow or Task Disable/Cleanup.

After registration the current owner CPU declares Task scheduler sleep and calls its local Scheduler. Inbox
consumption at user-return/idle safe points distinguishes the race sides: a wake for the current sleep-declared
Task becomes its one pending wake signal, while a wake for an already blocked Online Task restores runqueue
eligibility. PreparePrev consumes a matching pending signal instead of blocking. An already runnable/enqueued or
generation-stale wake is consumed without a second enqueue. No Scheduler call occurs while the files or pipe lock
is held.

`mkdirat` in the first transient-writable-namespace slice accepts only
`AT_FDCWD` and imports the pathname through mapping- and permission-checked
usercopy. The importer must distinguish an inaccessible address (`EFAULT`), an
empty pathname (`ENOENT`) and a pathname with no terminator inside the fixed
buffer (`ENAMETOOLONG`). After import, syscall dispatch resolves the current
generation/CPU-checked aggregate, acquires the shared files/fs/VFS IRQ-safe
resource lock, and routes its `FsStruct` plus the global `VfsCore` to the
transient directory create operation specified by `vfs.md`. It may ignore mode
and umask only as the explicitly deferred metadata/permission part of this
slice; it must not return success unless a lookup-visible directory was
actually published. Other dirfd shapes remain unsupported.

`unlinkat` syscall 35 uses the same checked pathname importer, current
generation/CPU-checked aggregate resolution and files/fs/VFS lock as
`mkdirat`. The first slice accepts `AT_FDCWD` and exactly either flags zero or
`AT_REMOVEDIR`. Unknown flag bits return `EINVAL`; other dirfds remain outside
the slice. It routes flags zero to transient regular-file removal and
`AT_REMOVEDIR` to transient empty-directory removal. The VFS operation must
finish target type, read-only-backing and directory-emptiness validation before
detaching the name. Usercopy and lookup failures map distinctly to `EFAULT`,
`ENOENT`, `ENAMETOOLONG`, `ENOTDIR`, `ELOOP` or `EROFS`; mismatched types map to
`EISDIR`/`ENOTDIR`, and a nonempty directory maps to `ENOTEMPTY`. No error may
change the namespace. Success returns only after the dentry is unreachable by
path; it must not invalidate an existing open description or the independent
frame already owned by a shared mapping.

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
first quiesces and cleans up the current UserAppRuntime, then disables and cleans up the fixed TaskFlow. It installs
`SwapperVm` SATP on the exiting CPU and performs a local `sfence.vma`, then publishes the zombie status and SIGCHLD
with release ordering. A vfork child may consume and publish its separately reserved immediate-parent completion
wake at this point. The ordinary wait/reap wake is not emitted yet: the exit leaf first enters a one-way
terminal Scheduler handoff using the exact current TaskRef and owner CpuRef. This handoff is not the fixed Flow's
ordinary resumable Schedule signal: it requires the registry zombie, Destroyed Runtime and Destroyed fixed Flow,
keeps the Task breakpoint invalid, and commits `Task.OnCpu -> Task.Offline` without publishing a resumable context.
Its terminal prepare step discards any stale pending wake and deactivates the Task before next-task selection, so
the zombie cannot regain runnable eligibility. The ordinary Flow-sender Schedule entry and all of its sender
validation remain unchanged.

Before zombie publication, exit holds the aggregate files-resource lock and releases every live inherited
open-file-description/pipe endpoint reference exactly once. A successful release must clear the aggregate's
`process_resources_present` ownership marker in the same critical section, analogous to Linux 6.12 `exit_files()`
clearing `task_struct.files` before `put_files_struct()`. Wait/reap treats a cleared marker as already released: it
may reset the destroyed slot carrier and credentials, but must not call the shared-reference decrement path again.
The slot cannot be reused until that carrier reset and the registry reap claim both complete.

After the physical switch, the selected next Task's stack cleans up the Offline terminal Task by resolving the
registry zombie identity, never a compatibility active/last-exited carrier. Only after that cleanup does the next
stack release-publish the terminal Task's scheduler-quiesced acknowledgement. That publication returns the
parent CpuRef captured under the same registry lock; the next stack then requests the parent CPU's reschedule IPI.
Thus the ordinary wait/reap wake follows the complete acquire-visible predicate and cannot be consumed between
zombie publication and quiescence. A parent already running may reap before the redundant IPI arrives, which is
harmless; slot reuse cannot erase the captured CpuRef. Zombie publication alone is not a
reap grant; the acknowledgement proves that the old mm, CurrentTask binding and kernel stack are no longer live on
the owner CPU. Wait4 acquire-observes both completion and this acknowledgement and is the only reap owner. A
caller-supplied boolean must not substitute for the acknowledgement. Repeated terminal, acknowledgement or reap
operations are rejected.

The blocking wait4 loop must preserve a wake across its final registry recheck and sleep instruction. On RISC-V,
the CPU-local SSIE class gate stays open while the loop keeps the global `sstatus.SIE` gate closed across `wfi`;
an SSIP arriving after the last recheck therefore remains pending and wakes `wfi`. Only after that wake may the
global gate reopen and allow the handler to clear SSIP, followed by another acquire recheck. Opening the global
gate before `wfi` is forbidden because the handler could consume the sole pending wake and return directly to the
sleep instruction. This is the raw-`wfi` lowering of the same prepare/recheck/sleep ordering guaranteed by a
waitqueue; it is not a timeout or polling substitute.

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
parent CpuRef and blocks the parent until child exec/exit completes. RISC-V syscall 123 provides only the read-only
`sched_getaffinity(pid, len, mask)` slice required to observe the current process. Decode `pid` as signed 32-bit and
`len` as unsigned 32-bit. Validate that `len * 8` covers the runtime `possible_cpu_count` and that `len` is aligned
to an 8-byte RISC-V `unsigned long` before resolving a process; either failure returns `EINVAL`. PID 0 and the
positive PID of the exact current registry occurrence select that occurrence; a negative or other PID returns
`ESRCH` in this bounded slice. Hold its generation/CPU-checked `UserProcessLease` while snapshotting the
`CpuGroup` active bits by logical ID. Because `MAX_CPUS <= 64`, the exported cpumask is one zero-initialized 8-byte
native word; copy exactly `min(len, 8)` bytes, return that byte count, and leave any trailing user buffer untouched.
An inaccessible output range returns `EFAULT`. The action neither rewrites the Task's fixed CpuRef nor calls the
Scheduler. `sched_setaffinity`, migration, runtime hotplug, automatic balancing and concurrent shared-mm cross-CPU
execution remain unrepresented.

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

The first file-backed `mmap` slice accepts exactly one page of a live transient
regular file, a page-aligned offset, `MAP_SHARED` without anonymous/fixed or
unmodeled flags, and readable `PROT_READ|PROT_WRITE` permissions. Syscall
dispatch resolves the generation/CPU-checked current process lease and holds
the user-memory lock before the files/fs/VFS lock. The fd lookup returns
`EBADF`; missing read access or a writable shared mapping without write access
returns `EACCES`; a non-regular/unmodeled backend returns `ENODEV`; malformed
length, offset, protection or flags return `EINVAL`; address-space or page
allocation exhaustion returns `ENOMEM`.

The VFS range read that seeds this mapping is positioned and must not advance
the open description. Mapping prepare allocates and zeroes one `UserFrameRef`,
copies the file range into it, prepares any required L0 table, installs the
leaf PTE, and only then publishes the VMA; failure releases staged ownership
and leaves the prior fd, VFS and address space unchanged. The VMA owns the
frame and stores the stable VFS `FileRef` plus file offset independently of the
fd and pathname, so close or unlink cannot invalidate it. Ordinary fork copies
that backing identity, acquires the same frame reference for this VMA,
and installs a writable non-COW leaf in parent and child. Whole-VMA munmap
clears the leaf and releases exactly that address-space reference. Full page
cache coherence, partial mappings, offsets beyond EOF and additional mapping
flag combinations remain deferred.

RISC-V syscall 227 implements the Linux 6.12 `msync(start, len, flags)` boundary.
It accepts only `MS_ASYNC|MS_INVALIDATE|MS_SYNC`, rejects simultaneous ASYNC and
SYNC and an unaligned start with `EINVAL`, rounds a nonzero length upward with
checked arithmetic, returns success for zero length, and returns `ENOMEM` if any
part of the rounded range is outside a current VMA. Hold the user-memory lock
while validating the current generation/CPU-checked address space, then the
files/VFS lock while syncing file-backed ranges. `MS_SYNC` on the bounded shared
file VMA copies its current frame bytes to the VMA's stable `FileRef` at the
stored file offset through a positioned VFS range write; this survives fd close
and pathname unlink and must not change `File.position`. Anonymous/private VMA
ranges and accepted non-SYNC flag combinations have no file side effect.
Backing lookup or positioned-write failure is reported as `EIO`, never converted
to success.

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
no user instruction trap occurred. OOM remains the disjoint signal-9 Task terminal. The bounded asynchronous
handler path above does not change the existing synchronous SIGSEGV/OOM terminal classification; user handlers
for those synchronous faults, core images and alternate signal stacks remain deferred.
