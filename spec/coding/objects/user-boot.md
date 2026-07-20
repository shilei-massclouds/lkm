# User boot and process integration coding

本文件是 `spec/model/objects/user_boot.spec` 与 `files.spec` 的权威 coding 映射。VFS object/backend
语义由 [`vfs.md`](vfs.md) 承载；本文件承载 init 候选策略、address-space/trap return、syscall table、
fd/OFD/backend dispatch、进程身份以及当前单 child slice 的集成边界。共享 exec 对象分别由
[`ExecTransaction`](exec-transaction.md)、[`BinaryFormatRegistry`](binary-format-registry.md)、
[`ExecSyncBoundaries`](exec-sync-boundaries.md) 和 [`ElfObject`](elf-object.md) 承载。

## Ownership and entry

`UserBootPayload`、首个 `UserAddressSpace` 和 `UserTrapFrame` 均属于
`KernelInitTask` 的执行线。`UserInitProcess` 是同一 PID 1 task 经 exec/user-entry 后的身份视图，
不是第二个 task。`SyscallException` 继续属于 `ExceptionStream`；`SyscallTable` 是独立表对象，
不增加 `SyscallDispatcher`。

用户栈 backing、initial stack 与增长语义由独立 [`UserStack`](user-stack.md) coding contract 承载；
本文件只保留 address-space、trap、usercopy/syscall 和 process 集成边界。

`UserBootPayload` owns only requested/default/fallback candidate selection. It normalizes the selected boot
arguments and invokes the shared transaction; ELF handler search, parser, staging and commit are not
implemented in `user_boot.rs`. User entry writes the prepared satp, performs the required fence and returns
through the modeled trap frame.

## Trap and exception mapping

The RISC-V user trap frame records `scause`, `sepc`, `sstatus`, `stval` and the integer register state
needed for transparent return. Breakpoint dispatch gives architecture single-step/probe hooks an
opportunity before falling back to signal/unsupported handling. These are implementation mappings of
the model trap boundary, not permission to invent a second exception stream.

The user kernel stack is a VMALLOC mapping with the modeled alignment and unmapped guard gap. Large
exec/mm objects stay out of trap-stack frames. For `APP=user-boot`, the RISC-V formal entry MUST keep the
user-origin path on the `sscratch`-provided safe kernel stack and MUST bypass the VMAP bit-test on that
path. A kernel-origin trap MUST classify the prospective 288-byte frame before saving any general
register, using only `sp` and `sscratch` and the Linux-shaped `((sp - frame_size) >> THREAD_SHIFT) & 1`
test (`THREAD_SHIFT=14`). Its normal branch MUST restore the original `sp`, clear `sscratch`, and preserve
all other registers before entering the common frame-save path.

The overflow stack MUST be a linker-visible 4 KiB static region aligned to 16 bytes. The overflow branch
MUST preserve the bad stack pointer and original `t6` through the `t6`/`sscratch` exchange, switch before
constructing a complete `TrapFrame`, and save all integer registers plus `sepc`, `scause`, `stval` and
`sstatus`. The non-returning handler MUST use only the SBI console to report stable bad-SP, task-stack,
overflow-stack and CSR diagnostics before terminal shutdown; it MUST NOT use ordinary checkpoints,
allocation, printk locking or user-signal delivery. The shared classification constants and frame
construction contract MUST have object smoke coverage, while an ordinary user `ecall` with callee-saved
sentinels covers transparent register return. Per-task stack ownership, per-CPU overflow stacks and IRQ
hardirq stack switching remain explicit follow-up boundaries in the active roadmap.

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

The supported `clone(220)` plain-fork shape decodes ABI flags in `SyscallTable` and delegates object
creation to `TaskCreationCore.CopyUserProcess`. The current implementation deliberately uses one
observed child slot: parent state and writable pages are saved for handoff, the child receives `a0=0`,
and wait/exit restores the parent view before status copyout. Nested BusyBox-init/login slices reuse only the
explicitly modeled continuation records; this is not a claim of a general runnable task graph or COW.
For a PID1-originated plain fork, successful wait status copyout completes reaping: the exited internal
`UserChild` task is removed from the runqueue, its slot becomes `Prepared`, and the next sequential fork
reuses that task ref with a monotonically increasing user-visible pid. This path does not use the vfork
completed-record archive.

For the observed BusyBox init login-shell plain fork, `UserInitProcess` additionally owns one pending
grandchild identity from clone return until the shell's wait4 handoff. It records the grandchild pid and
the shell parent's inherited process group/session without creating another runnable task. While that
identity is parent-visible and has not entered the handoff/exec continuation, the shell parent may issue
only `setpgid(child_pid, child_pid)`; the update is consumed into the visible child identity at wait4
handoff. Unknown pid remains `ESRCH`, negative pgid remains `EINVAL`, and unsupported or cross-session
group selection remains `EPERM`. Parent-side setpgid after handoff/exec, multiple pending children and a
general process-group/task lookup stay deferred. Grandchild exit restores the saved shell pgrp/session
and clears any pending identity. The internal `UserChild` remains the `Ready`, enqueued shell continuation;
the restore clears only the completed grandchild round's trap-frame, stack/address-space snapshot, wait
frame/status and exit facts. A later sequential observed plain fork may reuse that shell slot and must
receive the next pid. This is distinct from vfork active-slot reuse, which releases a completed execution
slot to `Prepared`.

The PID 1 parent wait handoff accepts both observed BusyBox forms:
`wait4(-1, status, WUNTRACED, NULL)` from the interactive shell and
`wait4(-1, status, 0, NULL)` from a non-interactive rc.local shell. Both use the same saved parent
frame/address-space/stack ownership and child-exit restore path; options=0 is not rejected merely because it
omits stop reporting.

The LTP list-stage command substitution adds one narrower second-level shape. It accepts only
`flags == SIGCHLD`, `newsp == 0`, and a current script continuation that came from an unfinished
PID1-originated plain fork. The single internal `UserChild` slot may own at most one pending builtin-only
grandchild. Clone returns a monotonically allocated pid to the script and records a distinct child trap/
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
[`../projects/rootfs-image.md`](../projects/rootfs-image.md). The ordinary acceptance boundary is guest
output plus modeled checkpoint facts, never mutation of a user/process object by a handler.
