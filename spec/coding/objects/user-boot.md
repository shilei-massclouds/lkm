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
exec/mm objects stay out of trap-stack frames. Early overflow checking, per-task generalization and IRQ
hardirq stacks remain explicit follow-up boundaries in the active roadmap.

## Files and syscall dispatch

`FilesStruct` is task-owned alongside `FsStruct`; `FileDescriptorTable` entries reference
`OpenFileDescription`, which selects a `FileBackend`. stdio uses the console/TTY backend and ordinary
read-only files use the VFS backend. Syscalls validate user memory and dispatch through these objects;
they do not special-case fixture paths or add test-only kernel APIs.

Current production slices include the modeled write/exit, read-only open/read/close/stat, ELF memory
management, process/credential, TTY, time/random, signal and observed AF_UNIX pathname-error operations.
Each slice follows the local Linux 6.12 RISC-V syscall ABI and preserves explicitly modeled errno and
deferred boundaries. Unsupported socket success paths, complete credentials/namespaces/LSM, general
task graphs, complete COW/mm, signals, networking and full fd sharing remain deferred.

## Clone, wait and exec

The supported `clone(220)` plain-fork shape decodes ABI flags in `SyscallTable` and delegates object
creation to `TaskCreationCore.CopyUserProcess`. The current implementation deliberately uses one
observed child slot: parent state and writable pages are saved for handoff, the child receives `a0=0`,
and wait/exit restores the parent view before status copyout. Nested OpenRC/login slices reuse only the
explicitly modeled continuation records; this is not a claim of a general runnable task graph or COW.
For a PID1-originated plain fork, successful wait status copyout completes reaping: the exited internal
`UserChild` task is removed from the runqueue, its slot becomes `Prepared`, and the next sequential fork
reuses that task ref with a monotonically increasing user-visible pid. This path does not use the vfork
completed-record archive.

For the observed OpenRC login-shell plain fork, `UserInitProcess` additionally owns one pending
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

Child `execve(221)` copies filename/argv/envp through `SyscallTable`, then invokes the same Context-owned
transaction used by boot. The transaction owns staging rollback, repeated slot reset, bounded CLOEXEC,
old-mm reclamation and point-of-no-return. Credential/signal/LSM and the complete Linux lock protocol remain
deferred.

## Test and observation boundary

KUnit/checkpoint handlers are read-only observers of production facts. Fixture building and delayed
stdin live in [`../../testing/rootfs.md`](../../testing/rootfs.md); image construction lives in
[`../projects/rootfs-image.md`](../projects/rootfs-image.md). The ordinary acceptance boundary is guest
output plus modeled checkpoint facts, never mutation of a user/process object by a handler.
