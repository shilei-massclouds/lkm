# User boot and process integration coding

本文件是 `spec/model/objects/user_boot.spec` 与 `files.spec` 的权威 coding 映射。VFS object/backend
语义由 [`vfs.md`](vfs.md) 承载；本文件承载 init 候选策略、address-space/trap return、syscall table、
fd/OFD/backend dispatch、Task 用户资源以及当前 bounded user-task 集合的集成边界。共享 exec 对象分别由
[`ExecTransaction`](exec-transaction.md)、[`BinaryFormatRegistry`](binary-format-registry.md)、
[`ExecSyncBoundaries`](exec-sync-boundaries.md) 和 [`ElfObject`](elf-object.md) 承载。

## Ownership and entry

`UserBootPayload`、首个 `UserAddressSpace` 和 `UserTrapFrame` 均属于
`KernelInitTask` 的执行线。PID 1 的 files、credentials、signal、process-group 和 user trap 状态
直接保存在 `KernelInitTask` 的 user-state lowering 中，不建立 exec 后 persona carrier。
首次 exec 声明的 fresh `UserAppFlow` 独立保存用户应用 continuation lifecycle；它不是第二个 Task，
也不拥有上述 Task 资源。`SyscallException` 继续属于 `ExceptionType`；`SyscallTable` 是独立表对象，
不增加 `SyscallDispatcher`。

用户栈 backing、initial stack 与增长语义由独立 [`UserStack`](user-stack.md) coding contract 承载；
本文件只保留 address-space、trap、usercopy/syscall 和 process 集成边界。

`UserBootPayload` owns only requested/default/fallback candidate selection. It normalizes the selected boot
arguments and invokes the shared transaction; ELF handler search, parser, staging and commit are not
implemented in `user_boot.rs`. User entry writes the prepared satp, performs the required fence and returns
through the modeled trap frame.

The first-user handoff completes `SyscallExceptionType.Setup/Enable`, then enables the owning CPU-local
`ExceptionType` and `TrapType` before committing user entry. These parent enables do not create global
exception/trap objects and do not alter the persistent `Task.active_flow`.

## Trap and exception mapping

The RISC-V user trap frame records `scause`, `sepc`, `sstatus`, `stval` and the integer register state
needed for transparent return. Breakpoint dispatch gives architecture single-step/probe hooks an
opportunity before falling back to signal/unsupported handling. These are implementation mappings of
the model trap boundary, not permission to invent a second exception resource or Flow chain.

The user kernel stack is a VMALLOC mapping with the modeled alignment and unmapped guard gap. Large
exec/mm objects stay out of trap-stack frames. `sscratch` always points to the owning CPU's
`TrapEntryContext`; the context carries the usable kernel-stack bounds, current Task identity, optional root
TrapFlowRef and that CPU's emergency-stack state. The user-origin prelude preserves user `tp` in the ordinary
`TrapFrame`, selects the installed kernel-stack top and loads Task identity into kernel `tp`. Kernel-origin
entry keeps its interrupted `sp`, but uses the same context and capacity check. Both paths compare the complete
aligned `TrapFrame + TrapExecutionRecord` range against the installed bounds before saving the ordinary frame.

Insufficient capacity switches to the context's CPU-local, 16-byte-aligned 4 KiB emergency stack before
constructing a complete `TrapFrame`. Emergency reentry requests shutdown directly without reusing the active
emergency stack. The non-returning handler uses only the SBI console to report stable CPU, Task, bad-SP,
task-stack, emergency-stack and CSR diagnostics before terminal shutdown; it does not use ordinary checkpoints,
allocation, printk locking or user-signal delivery. Object smoke covers normal, boundary, overflow and reentry
classification, while an ordinary user `ecall` with callee-saved sentinels covers transparent register return.
Per-task stack ownership is supplied by `TaskThreadContext`; IRQ hardirq stack switching remains deferred.

## Files and syscall dispatch

`FilesStruct` is task-owned alongside `FsStruct`; `FileDescriptorTable` entries reference
`OpenFileDescription`, which selects a `FileBackend`. stdio uses the console/TTY backend and ordinary
read-only files use the VFS backend. Syscalls validate user memory and dispatch through these objects;
they do not special-case fixture paths or add test-only kernel APIs.

`FileDescriptorTable` maps successful single-entry creation through the private
`install_new_fd_entry(fd, entry)` implementation primitive. Callers retain their own lifecycle,
dependency, fd-range and entry-kind validation; first-free callers select the lowest unused slot at or
above their existing lower bound through `lowest_free_fd_from(min_fd)` before installing. The install
primitive writes exactly one previously free entry and advances `fd_installed` exactly once. This is a
coding mapping shared by the existing model actions, not a new public API or formal action. Targeted
`dup3` replacement must bypass it because an occupied `newfd` may be replaced and the dup3-specific facts
are recorded separately. Atomic pipe-pair installation must also remain separate so capacity failure
cannot expose a partial pair and success advances the installed-entry count by two.

Current production slices include the modeled write/exit, read-only open/read/close/stat, ELF memory
management, process/credential, TTY, time/random, signal and observed AF_UNIX pathname-error operations.
Each slice follows the local Linux 6.12 RISC-V syscall ABI and preserves explicitly modeled errno and
deferred boundaries. Unsupported socket success paths, complete credentials/namespaces/LSM, general
task graphs, complete COW/mm, signals, networking and full fd sharing remain deferred.

The single read-only regular-file staging buffer is bounded at 64 KiB. This covers the canonical
`/opt/ltp/runtest/syscalls` input (33,110 bytes) used by list-before-run while retaining the existing one-open
regular OFD model. Larger files still fail at the bounded staging boundary; page cache, streaming VFS reads,
multiple simultaneous regular files and writable files remain deferred.

Linux RISC-V syscall 59 is a bounded `pipe2` first slice routed through `FilesStruct`. It accepts only
`flags == 0`; any nonzero flag, including `O_CLOEXEC` and `O_NONBLOCK`, returns `EINVAL`. Success atomically
installs the lowest two free descriptors as a read-only end followed by a write-only end and copies the two
32-bit fd values to user memory. User-copy failure or insufficient fd capacity leaves neither descriptor
installed and makes the single staged pipe reusable.

The staged pipe owns one small bounded FIFO buffer. Existing `read`, `write`, `close`, `dup3` and fcntl-dup
paths operate on the pipe-end fd entries with direction checks. Buffered bytes survive the current observed
child fd-table snapshot/restore: child close/dup/write mutations are rolled back for the parent fd view, but
pipe data is shared handoff state and must not be restored from the parent snapshot. Reads return available
bytes; after the last writer closes, an empty read returns EOF. The first slice supports only one live pipe
and the observed child-write/parent-read handoff used by BusyBox command substitution. General blocking and
wakeup, concurrent or multiple live pipes, expandable buffers, full OFD/files/task refcount graphs,
`O_CLOEXEC`/`O_NONBLOCK`, signals and signal-producing `EPIPE` remain deferred.

## Clone, wait and exec

The supported `clone(220)` plain-fork shape decodes ABI flags in `SyscallTable` and delegates Task creation
to `TaskCreationCore.CopyUserProcess`. Production state is owned by `UserTaskSet`: each successful fork
allocates a fresh logical Task identity and monotonically increasing PID, while parent state and writable
pages are saved in a continuation record for the handoff. The child receives `a0=0`; wait/exit restores the
parent view before status copyout. A fixed-capacity Rust record may be recycled only after the prior Task and
all its Flow instances are destroyed; record reuse must increment the allocation generation and must never
reuse the prior TaskRef or lifecycle state. Nested BusyBox-init/login records are bounded lowering, not a
single reusable child object or a claim of general COW.

For the observed BusyBox init login-shell plain fork, `UserTaskSet` additionally owns one pending
grandchild record from clone return until the shell's wait4 handoff. It records the fresh grandchild Task identity, pid and
the shell parent's inherited process group/session without creating another runnable task. While that
identity is parent-visible and has not entered the handoff/exec continuation, the shell parent may issue
only `setpgid(child_pid, child_pid)`; the update is consumed into the visible child identity at wait4
handoff. Unknown pid remains `ESRCH`, negative pgid remains `EINVAL`, and unsupported or cross-session
group selection remains `EPERM`. Parent-side setpgid after handoff/exec, multiple pending children and a
general process-group/task lookup stay deferred. Grandchild exit restores the saved shell pgrp/session
and clears any pending identity. The corresponding user Task remains the `Ready`, enqueued shell continuation;
the restore clears only the completed grandchild round's trap-frame, stack/address-space snapshot, wait
frame/status and exit facts. A later sequential observed plain fork must allocate a new Task identity and
receive the next pid; an implementation record may be reclaimed only after the prior Task/Flow teardown.

The PID 1 parent wait handoff accepts both observed BusyBox forms:
`wait4(-1, status, WUNTRACED, NULL)` from the interactive shell and
`wait4(-1, status, 0, NULL)` from a non-interactive rc.local shell. Both use the same saved parent
frame/address-space/stack ownership and child-exit restore path; options=0 is not rejected merely because it
omits stop reporting.

The LTP list-stage command substitution adds one narrower second-level shape. It accepts only
`flags == SIGCHLD`, `newsp == 0`, and a current script continuation that came from an unfinished
PID1-originated plain fork. The bounded `UserTaskSet` slice may hold at most one pending builtin-only
grandchild record. Clone returns a monotonically allocated pid to the script and records a distinct Task identity, Flow, trap/
stack/fd view; the grandchild receives zero from clone only when the script reaches wait4. A second pending
child, a clone from the builtin grandchild, or any deeper/concurrent shape returns the existing unsupported
boundary and does not consume a pid.

That second-level scheduling boundary owns a separate bounded parent-continuation snapshot: script parent trap frame and
status pointer, fs/fd view, stack bytes, and writable user pages. Before an inner exec it does not copy another exec
address-space or `UserStack` object. The boundary is either the script's wait4 or, as observed in BusyBox command
substitution, its first blocking read from the empty pipe after closing the parent write end. Handoff restores
the clone-time child stack/fd view; close/dup/write then operate on
that live child fd view while pipe bytes remain shared. Grandchild exit restores only the script parent
continuation and fs/fd view. A wait4-origin handoff completes that wait directly; a pipe-read-origin handoff
retries the restored read, keeps the exited child waitable, and lets the later wait4 copy status and reap it.
The outer PID1 wait frame, address-space snapshot, fd snapshot, stack snapshot,
and writable-page snapshot keep their original ownership until the script itself exits. Allocation, copy,
or fd-snapshot failure releases every second-level page/reference and leaves no pending identity or consumed
pid.

Runtime `execve` from the active builtin-only grandchild uses the common bounded exec transaction. Retired-image
ownership is classified before the point of no return as `None`, `OuterChild`, or `BuiltinGrandchild`, with the
builtin source checked first. The first inner exec transfers the retired script address space and `UserStack` to
the builtin continuation. Its binding owns the transaction's preallocated retired address-space and stack slots
until restore, independently of the outer PID1 snapshot; it must not overwrite that outer address-space or stack
snapshot. A later exec by
the same grandchild keeps the original script image unchanged and releases the immediately replaced executable.
Close-on-exec scans only the live grandchild fd view; the saved script and outer PID1 fd snapshots remain owners of
their entries until their respective restore boundaries.

Grandchild exit first releases its current exec image, restores the retained script address space and `UserStack`,
and switches to that SATP. Only then may the existing writable-page, stack-byte, fs/fd, wait4 or pipe-read restore
sequence run. When no inner exec occurred, that exec-object restore is an explicit no-op fact. The script's later
exit still restores the original outer PID1 image. Staging, argument, ELF, address-space, entropy, or close-on-exec
precheck failure before the point of no return preserves the current executable, both parent snapshots, fd
references, and pending identity. A retention or restore invariant failure after commit is terminal and emits a
stable diagnostic containing owner, first/subsequent exec count, current/parent SATP, saved/restored state, and the
last exec failure stage. The existing vfork-origin observed grandchild exec slice is unchanged. General COW/task
graphs, multiple pending children, deeper nesting, signals, and complete pipe scheduling remain deferred.

The same builtin-only slice permits its observed stderr redirection
`openat(AT_FDCWD, "/dev/null", O_WRONLY|O_CREAT|O_TRUNC|O_LARGEFILE, 0666)`. `O_CREAT`/`O_TRUNC` are ignored
only for the built-in null device, whose writes are discarded; they remain invalid for TTY, regular-file and
directory paths, so this does not add a general writable VFS/open-create slice.

Builtin `pwd` uses the inherited/restorable `FsStruct.pwd` dentry and the existing bounded dentry parent
chain to serialize a NUL-terminated absolute path such as `/opt/ltp`. The syscall still returns the byte
count including NUL, `ERANGE` for a short buffer and `EFAULT` for failed copyout. Mount crossings,
disconnected dentries, concurrent cwd updates and namespace-aware `d_path()` remain deferred.

The canonical rc-local shutdown path supports the observed BusyBox `sync(2)` then `reboot(2)` sequence.
`sync` is a successful barrier for the current in-memory rootfs. `reboot` reaches SBI shutdown only for
Linux `MAGIC1`, `MAGIC2`, and `CMD_POWER_OFF`; invalid magic or other commands are rejected and never power
off the guest.

Runtime `execve(221)` accepts the current PID 1 process as well as the observed child continuation, copies
filename/argv/envp through `SyscallTable`, then invokes the same Context-owned transaction used by boot. PID 1
self-exec preserves its process identity, installs the new return frame and releases retired image backing;
the child path retains the parent snapshot ownership exception. The transaction owns staging rollback,
repeated slot reset, bounded CLOEXEC, old-mm reclamation and point-of-no-return. Credential/signal/LSM and the
complete Linux lock protocol remain deferred.

## Test and observation boundary

KUnit/checkpoint handlers are read-only observers of production facts. Fixture building and delayed
stdin live in [`../../testing/rootfs.md`](../../testing/rootfs.md); image construction lives in
[`../rootfs-image.md`](../rootfs-image.md). The ordinary acceptance boundary is guest
output plus modeled checkpoint facts, never mutation of a user/process object by a handler.
