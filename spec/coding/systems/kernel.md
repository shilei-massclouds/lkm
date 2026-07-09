# Kernel System Coding

本文件承载 `systems/kernel.spec` 的说明性正文。Formal 文件只保留 Kernel system 层级以及 arceos_ex startup 系统约束的 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/systems/kernel.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/systems/kernel.spec` 的长注释迁移而来。`*.spec` 文件只保留 rule ID、type 分组、MUST/SHOULD/MAY/NOTE 层级和最短标签；解释、背景、参考路径、阶段性取舍与例子在这里维护。

### KernelSystemCoding

#### Mapping path

Kernel system implementation belongs under impl/arceos_ex/src/systems/.
The Kernel object maps to impl/arceos_ex/src/systems/kernel.rs.

#### Kernel lifecycle boundary

systems/kernel.rs must provide the system lifecycle entry points that
preserve the model Kernel lifecycle states. Kernel.Preset completes
only after BootPhase.Ready and moves Kernel from Base to Prepared;
Kernel.Setup completes only after InterruptPhase.Ready and moves
Kernel from Prepared to Ready; Kernel.Enable drives
UpMultitaskPhase, SmpRuntimePhase and PayloadPhase, and marks Kernel
Online only after PayloadPhase.Online.

#### Runtime choreography

Kernel owns the ordering relationship among BootPhase,
InterruptPhase, UpMultitaskPhase, SmpRuntimePhase and PayloadPhase.
Individual phase setup/handoff code remains in the mapped phase
files.

#### Kernel state

The system module owns the Kernel lifecycle state and emits Kernel
system checkpoints. Crate-root entry code must not retain separate
startup timeline state.

#### Behavior preservation

Kernel system migration must not reorder existing phase calls or
checkpoint emission order.

#### Phase ownership

System-level mapping records the lifecycle owner but must not absorb
phase-local checks, diagnostics or checkpoints out of their phase
modules.

#### Payload handoff

The selected payload remains a sibling phase handoff after
SMP/runtime readiness, not a project-level build action and not a
nested runtime-core side effect.

### ArceosExStartupPhaseCodingMust

#### Startup phase order

The arceos_ex startup chain must follow the formal top-level model
order PreparePhase -> BootPhase -> InterruptPhase ->
UpMultitaskPhase -> SmpRuntimePhase -> PayloadPhase.
PayloadPhase setup/enable must not run directly after BootPhase
without first completing the intervening phase chain.

#### Payload dependency

PayloadPhase setup and enable must require InterruptPhase.Ready in
addition to BootPhase.Ready. A selected payload may rely on the
boot CPU interrupt gate and IRQ/time acceptance facts established by
InterruptPhase.

#### Payload placement

PayloadPhase is a Kernel child that follows
SmpRuntimePhase. It must not be implemented as the last subphase
nested under SmpRuntimePhase.

#### Finalize handoff

PayloadPhase setup/enable must require FinalizePhase.Ready and the
FinalizeBoundary next-boundary fact. This makes the direct handoff
from kernel_init() finalization to payload explicit.

#### KernelInitTask execution line

KernelInitTask execution starts at the PreSmpInitPhase entry after
rest_init() dispatch facts, then proceeds through the ordered
phase chain to FinalizePhase and PayloadPhase. Payload ownership is
derived from that continuous execution line instead of from an
isolated payload-only fact.

#### UserBootPayload selected variant

The first user-mode program path is a selected payload variant named
UserBootPayload. It is driven by PayloadPhase setup/enable and must
not be implemented as an unrelated phase or as a second root
startup chain.
The current Linux PayloadPhase.Online paired anchor is on the
default fallback try_to_run_init_process("/sbin/init") branch.
Requested-init cases that pass init= return before that branch and
must not include PayloadPhase.Online in a hard diff scope without an
equivalent requested-init anchor.

#### Syscall ownership

User-mode ecall/syscall handling must extend the existing
SyscallException branch under ExceptionStream. Code generation must
not introduce a separate root Syscall object that bypasses
ExceptionStream dispatch.

#### ELF object boundary

The user executable model object is ElfObject. Code generation must
not create a separate ElfLoader resource object for this slice.
ElfObject.Setup parses ELF header/program headers and builds the
PT_LOAD mapping plan; the actual user-address-space mapping belongs
to UserAddressSpace.Setup. ElfObject.Enable only confirms user-entry
preconditions and hands them to UserBootPayload.

Dynamic libc support still uses ElfObject. PT_INTERP on the main
executable binds an interpreter-path fact and causes UserBootPayload
to read the interpreter as another ElfObject role. That interpreter
may be ET_DYN, but it is not an ElfLoader resource object and it does
not introduce a separate Load lifecycle stage.

ELF type handling follows Linux 6.12 fs/binfmt_elf.c
load_elf_binary(): ET_DYN is not synonymous with dynamic linking,
and ET_EXEC is not synonymous with static linking. PT_INTERP is the
current dynamic/interpreter-path discriminator. Main executables must
support ET_EXEC and must also support ET_DYN + PT_INTERP as PIE main
with a fixed non-overlapping load bias for this first slice. Linux's
ELF_ET_DYN_BASE + ASLR + mmap search is trimmed for now. ET_DYN main
without PT_INTERP is the direct-loader form and remains deferred.

#### User init ELF validation boundary

Smoke/KUnit may verify the temporary /sbin/init overlay ELF's normal
metadata and content facts through ElfObject state, such as entry,
PT_LOAD segment count, permissions and expected embedded bytes.
Implementations MUST NOT add test-only methods or APIs to ordinary
objects for this validation.

#### User address-space boundary

UserAddressSpace is the multi-instance user address-space object.
Its low half is per user process; its high half shares or references
SwapperVm. SwapperVm remains the single kernel shared address-space
instance and must not be mechanically converted into a user address
space type with arbitrary instances.

The first UserAddressSpace instance belongs to the KernelInitTask
execution line that reached PayloadPhase. Preset must require
KernelInitTask.Online and record the first-instance binding before
stack, ELF mappings or trap-frame setup consume the address space.
This expresses the Linux-like kernel_init/kernel_execve handoff
without claiming that satp has already switched to a user page
table.

UserAddressSpace.Setup consumes ElfObject's PT_LOAD mapping plan and
records segment/stack mapping facts, user-page U permission facts and
kernel-page U=0 facts. For PT_INTERP executables it must consume both
the main executable ElfObject and the interpreter ElfObject mapping
plans into the same user address space. It must also provide a
minimal user heap/anonymous mapping arena for the dynamic loader's
early brk/mmap-style allocations. If the main executable is
ET_DYN + PT_INTERP PIE, ElfObject.Setup must bind a fixed
non-overlapping main load bias before UserAddressSpace.Setup
consumes the load plan, so the main PIE, interpreter, heap and stack
windows do not overlap. It may allocate backing pages, copy ELF file
bytes, zero .bss/stack/heap bytes and build a page-table-shaped
view. ELF segment backing and low-half user leaf PTEs are
page-granular over align_down(p_vaddr)..align_up(p_vaddr + p_memsz);
mprotect/munmap mapped-range checks must use that same page range so
GNU_RELRO whole-page protection requests are accepted.

UserAddressSpace.Enable is the real pre-switch satp-ready boundary.
It may allocate real Sv39 user page-table pages, install low-half
user leaf PTEs, copy/share SwapperVm high-half root entries, produce
a satp token and mark runtime_ready as "ready for the next trap
return consumer". It MUST also preserve a prepared-but-not-current
fact, and MUST NOT write satp, perform the address-space switching
sfence.vma, execute sret, or set up syscall/UserInitProcess state
before the explicit trap/syscall round is modeled.

The explicit trap/syscall round must consume the prepared
UserTrapFrame through the existing EventStream trap-return shape.
The first user entry may write satp, execute the required sfence.vma
boundary and sret to U-mode. The corresponding trap entry must not
trust the user stack as a kernel trap frame stack: it must switch to
a kernel-owned trap stack, for example through sscratch, before
saving the full trap frame.

RISC-V user FPU support is a first-slice user-mode status contract
based on local Linux 6.12 arch/riscv/kernel/process.c::start_thread(),
arch/riscv/include/asm/csr.h and arch/riscv/include/asm/switch_to.h.
UserTrapFrame setup must set the user status to SR_PIE with
SR_FS_INITIAL and SPP clear so hard-float musl/BusyBox code can
execute ordinary user FPU instructions. The corresponding trap entry
must save the user status first, then clear live sstatus FS/VS
before entering kernel/Rust handling, and restore the saved user
status only for sret back to user mode. Full thread.fstate storage,
fstate_save()/fstate_restore(), lazy/clean/dirty optimization,
fork/signal/ptrace fpstate and multitask FPU/vector context switch
remain deferred.

User ecall must enter the existing ExceptionStream ->
SyscallException branch and install a concrete syscall policy there.
Code generation MUST NOT create a new root Syscall object or bypass
ExceptionStream dispatch. It also MUST NOT create a separate
SyscallDispatcher object: SyscallException owns syscall entry,
source validation, argument extraction and dispatch selection.
SyscallTable is the independent table object and concrete syscalls
are SyscallTable actions. The current table handles the first user
program's console/file syscalls, vector console writes through
writev, the dynamic loader memory actions brk, mmap, mprotect and
munmap by routing them to UserAddressSpace, directory-capable
openat plus getdents64 by routing through FilesStruct fd position
and Ext2/VFS directory records, readlinkat through the VFS no-follow
final symlink path, getrandom(278) through HwRngCore current-device
reads rather than /dev or VFS, and set_tid_address by recording the
current PID1 UserInitProcess clear_child_tid pointer.
clone(220) plain fork first slice must reference local Linux 6.12
kernel/fork.c::SYSCALL_DEFINE5(clone), kernel_clone(),
copy_process(), include/uapi/asm-generic/unistd.h and
arch/riscv/kernel/process.c::copy_thread(). The current observed
BusyBox /bin/sh "ls" request is a0=0x11, a1=0, a2=0, a3=8,
a4=0x20096580, a5=1; implementations must decode this as
exit_signal=SIGCHLD with no CLONE_* flags after CSIGNAL removal.
The OpenRC login shell focused baseline and the rc.local
direct-inittab shell both observe the same plain-fork ABI shape for
staged /bin/ls from an existing UserChild continuation:
clone_flags=0x11, newsp=0, current_child=1, and active_slot_reusable=0.
The login-shell case is a nested-vfork child; the direct rc.local
sysinit shell is a non-nested vfork child. Neither is the supported
PID1 plain-fork slice or a new vfork clone. The supported
child-continuation shape is a bounded observed child plain fork:
only SIGCHLD with no other CLONE_* flags, newsp=0, current vfork
child continuation as parent, and no deeper observed child.
It must save the shell parent pid/frame/stack/fd/address-space facts,
allocate next_child_pid for the grandchild, prepare a copied child
trap frame with a0=0 and inherited TLS, then return that pid to the
shell parent. It must not create a second runnable UserChild task ref
or enqueue; the grandchild continuation is taken only when the shell
later reaches wait4(-1, status, allowed_options, NULL). Unsupported
child-context shapes must still print stable clone_plain diagnostics.
SyscallTable.Action::Clone must drive TaskCreationCore.CopyUserProcess
for PID1 plain fork and non-nested vfork rather than manufacturing a
pid return. The observed child plain-fork exception is explicitly not
a CopyUserProcess/runqueue path; it is a UserChildProcess single-slot
continuation fact. The child process must have
a copied user trap frame with a0=0, inherited user sp because
newsp=0, inherited TLS because CLONE_SETTLS is absent, copied
files/fs/credentials/signal first-slice facts, a PID, and a
wake_up_new_task-shaped SelectRunQueue -> SetTaskCpu -> EnqueueTask
boundary. The parent returns the child pid. After clone, the same
shell "ls" evidence reaches parent wait4(260) with pid=-1,
status pointer, options=WUNTRACED and rusage=NULL. wait4(260) first
slice must reference local Linux 6.12
kernel/exit.c::SYSCALL_DEFINE4(wait4), kernel_wait4() and
do_wait(); kernel_wait4() adds WEXITED and do_wait() reaches the
wait_chldexit interruptible boundary because the cloned child exists
but is not yet waitable. The first slice may record that parent wait
boundary and yield to the already-created child continuation; it
must not synthesize a child exit, write the status pointer, reap a
zombie or claim full scheduler sleep/wakeup semantics. After this
first slice, the same guest no longer stops at unsupported wait4;
the next observation is an instruction page fault in the child
continuation with a7=135, which makes copied trap-frame-adjacent
child address-space/stack snapshot the next boundary instead of an
ad-hoc child exec shortcut. OpenRC /sbin/init additionally observes
legacy clone flags 0x4111 with flags_without_csignal=0x4100:
SIGCHLD | CLONE_VM | CLONE_VFORK, not CLONE_PIDFD.  The first slice
for that shape saves the parent clone frame, hands off directly to
the child continuation, and lets bounded child exit resume parent
clone with a real SIGCHLD pending/wake.  The OpenRC login focused
run later observes BusyBox login issuing the same vfork shape while
the getty/login UserChild continuation is still active; this first
slice only permits that single nested takeover by reusing the same
internal UserChild task ref with a new user-visible pid, and still
rejects deeper nesting, pidfd nested vfork, parent/child
concurrency and multiple runnable user task refs.  The following
post-auth credential/tty ownership boundary at fchown(55) on fd 0
and fchmod(52) on fd 0 now has a fd-local first slice that records
owner/mode metadata through FilesStruct and the fd table.  Focused
trace after that slice confirms both syscalls return 0, and records
the next post-auth boundary as socket(198) with
AF_UNIX/SOCK_STREAM/SOCK_CLOEXEC/protocol 0.  The current socket
first slice only creates an unconnected fd for that stream shape;
complete inode ownership, TTY ownership, complete credentials,
connect, sendto, network sockets and full supplementary group
semantics remain out of scope here.  The later observed
boundary is clone_vfork stage=child_records_full with
completed_records=8, record_capacity=8, active_slot_reusable=1,
next_child_pid=11 and no first_unreaped_pid.  Therefore completed
record capacity is the current occupied unreaped/diagnostic slot
count, not a monotonic lifetime history.  Successful wait4 reaping
of a completed vfork record must preserve last/total diagnostics
while releasing the slot for later sequential vfork reuse; EFAULT
status copyout and rt_sigtimedwait(SIGCHLD) consumption must not
release it.  A true vfork shape that includes CLONE_PIDFD(0x1000)
additionally uses parent_tidptr as the pidfd copyout address and
installs a pidfd-like fd table entry.
Unsupported clone diagnostics must remain generic rather than
vfork-only: every clone ENOSYS detail must print clone_kind as one
of plain_fork, vfork_vm, vfork_pidfd or unsupported_shape, then a
shape-specific stage key such as clone_plain stage=... or
clone_vfork stage=.... The detail must include flags,
flags_without_csignal, exit_signal, newsp,
current_child_continuation, active slot state/reusable, child pid,
nested parent pid, next_child_pid and completed-record counters,
and must be diagnostic-only.
clone3, thread groups, full CLONE_VM/vfork completion scheduling,
COW mm, full zombie/release_task lifecycle, pid hash,
resource accounting, wait queues, multi-child concurrency,
full pidfd file operations,
ptrace/seccomp/cgroup/audit, namespace, robust futex,
clear-child futex wake, full exit/reap/status copyout, failure rollback
and unobserved flag combinations must be recorded by
UserCloneDeferredBoundaries or SyscallTable wait4 facts as deferred,
trimmed or unsupported-first-slice.
This is a formal runtime boundary, not a test-only API. It does not
implement a full VMA tree, fd table, devfs console file, TTY line
discipline, futex/clone/thread-group semantics, complete fork/wait
or signal semantics beyond the bounded rt_sigtimedwait SIGCHLD
pending/dequeue/wake first slice.

mmap(222) must reference local Linux 6.12
arch/riscv/kernel/sys_riscv.c::SYSCALL_DEFINE6(mmap),
mm/mmap.c::ksys_mmap_pgoff()/do_mmap()/mmap_region(), and the
asm-generic/linux mman UAPI headers. The current first slice keeps
ordinary MAP_PRIVATE|MAP_ANONYMOUS allocation in the staged arena
and additionally accepts the observed BusyBox/musl shape
mmap(USER_HEAP_BASE, 4096, PROT_NONE,
MAP_PRIVATE|MAP_FIXED|MAP_ANONYMOUS, -1, 0). The anonymous path
must not consume fd; the request must be page-aligned and wholly
contained in the pre-mapped anonymous arena, returning the fixed
address. Full MAP_FIXED replacement
unmap, PROT_NONE PTEs, VMA split/merge, file-backed mmap,
overcommit/accounting, ASLR area selection, MAP_FIXED_NOREPLACE,
mmap locks and complete errno behavior remain trimmed.

The first directory slice must reference local Linux 6.12
fs/open.c::do_sys_openat2()/sys_openat(),
fs/readdir.c::sys_getdents64()/iterate_dir()/filldir64(), and
fs/file.c::fdget_pos(). It may trim Linux permission, LSM,
fsnotify/file_accessed, inode i_rwsem, RCU/f_pos_lock contention,
mount namespace and complete errno behavior, but it must preserve
the production path shape: openat(O_DIRECTORY) installs a readable
directory fd, getdents64 serializes linux_dirent64 records from the
ext2 directory data, returns copied bytes, and advances the fd
offset only at complete record boundaries.

The first readlinkat slice must reference local Linux 6.12
fs/stat.c::do_readlinkat()/sys_readlinkat(),
fs/namei.c::vfs_readlink()/readlink_copy(), and
include/uapi/asm-generic/unistd.h::__NR_readlinkat=78. It must
preserve the Linux core order: reject bufsiz <= 0 with EINVAL,
copy the user pathname, look up the path without following the final
symlink, reject a non-symlink final dentry with EINVAL, copy at most
bufsiz target bytes without appending NUL, and return the copied
byte count. The current slice may support only AT_FDCWD plus
absolute paths on read-only ext2 fast symlinks; relative dirfd,
AT_EMPTY_PATH, slow symlink page/block bodies, security hooks,
atime, RCU/retry_estale and complete errno details remain trimmed.

The first getrandom slice must reference local Linux 6.12
drivers/char/random.c::sys_getrandom(),
include/uapi/linux/random.h and
include/uapi/asm-generic/unistd.h::__NR_getrandom=278. It must keep
the syscall as a kernel random-core interface, not a /dev pathname
operation. The current implementation may support only flags == 0
and small USER_COPY_MAX-bounded buffers, routing successful reads
through the existing HwRngCore current provider backed by
virtio-rng; full CRNG pool readiness, blocking wait queues,
GRND_NONBLOCK, GRND_RANDOM, GRND_INSECURE, signal interruption,
large-buffer iteration and random-quality policy remain trimmed.

The first stdin read slice must reference local Linux 6.12
fs/read_write.c::ksys_read()/vfs_read(), fs/file.c::fdget_pos(),
drivers/tty/tty_io.c::tty_read(), drivers/tty/n_tty.c::n_tty_read()
and include/uapi/asm-generic/unistd.h::__NR_read=63. It may only
support fd0 char-device reads when bounded TTY ready data already
exists. The path must still go through FilesStruct, FileDescriptorTable,
OpenFileDescription and FileBackend::CharDevice; SyscallTable must
not copy a test string directly. The ready-data fixture is allowed
only when the selected ELF is the controlled user_smoke fixture,
including make test's temporary requested-init overlay case; it must
not be injected for distro init/sh/ls images. Blocking wait queues,
canonical N_TTY
line discipline, job control, poll/ppoll, signal interruption/restart,
controlling tty state and real IRQ wakeup remain deferred.
PROBE=user-read-trace is the explicit diagnostic for read success
and failure results that are invisible to user-syscall-error. It may
print supported read(63) fd, requested length, USER_COPY_MAX capped
length, success result or failure reason, trap mode, a0..a2, sepc
and stval, plus an internal FileError/copy reason when available.
It must not change the read return value, errno mapping, checkpoint
ordering, user-smoke pass/fail policy or default make test output.
A no-ready-data char-device read returning 0 is diagnostic evidence
for the current trimmed implementation, not proof that Linux
blocking N_TTY read semantics are complete.

The /dev/tty, fd-dup, termios and foreground-pgrp slice must reference local Linux
6.12 fs/open.c::build_open_flags()/do_sys_openat2(),
fs/file.c::ksys_dup3()/do_dup2(), fs/fcntl.c::do_fcntl()/f_dupfd(),
drivers/tty/tty_io.c::tty_open()/tty_ioctl()/tiocgwinsz(),
drivers/tty/tty_ioctl.c::tty_mode_ioctl(),
drivers/tty/tty_port.c::tty_port_block_til_ready(),
drivers/tty/tty_jobctrl.c::tty_jobctrl_ioctl()/tiocgpgrp()/tiocspgrp() and the
asm-generic fcntl/ioctls/termbits UAPI headers. O_RDWR is a valid open access
mode and O_NONBLOCK=00004000 is a file status flag; neither may be
rejected as an invalid flag before pathname copy when the path may
be a TTY. The current slice may special-case
openat(AT_FDCWD, "/dev/tty" or "/dev/tty[0-9]+",
O_RDWR|O_NONBLOCK|O_LARGEFILE) to install a console-like
FileBackend::CharDevice fd entry in the lowest free fixed-capacity
fd table slot. With stdio still installed this is normally 3
through 15; after close(0/1/2), the vacated low-numbered slot may
be reused by the next TTY open. Multiple /dev/ttyN aliases may
therefore coexist as separate fd entries with independent status
flags and close-on-exec bits, but they still share the same
console-like Tty0 backend and do not allocate real VT instances.
Full fd table returns EMFILE; invalid fd paths still return EBADF.
close(fd) follows Linux close_fd() only far enough to clear the
current fd entry and return EBADF for invalid descriptors. Closing
fd0/fd1/fd2 does not destroy the shared console-like backend and
does not implement complete stdio rebinding, filp_flush/fput,
fdtable locks, refcounts or cross-task files-copy semantics.
O_NONBLOCK is preserved in each opened fd entry for F_GETFL
diagnostics. Regular and directory paths do not gain nonblocking
read/write semantics or generalized multi-open support from this
slice. The TTY path must still route through FilesStruct,
FileDescriptorTable and OpenFileDescription. This is not devtmpfs,
/dev/console, real VT allocation, multiple TTY instances,
major/minor device lookup, controlling tty allocation or canonical
N_TTY readiness.

fcntl(F_DUPFD) and fcntl(F_DUPFD_CLOEXEC) must duplicate the fd
entry to the lowest free fixed-capacity slot at or above arg,
keeping the same OpenFileDescription and setting the new fd's
close-on-exec bit only for F_DUPFD_CLOEXEC. If the original fd is
closed while the duplicate still points at that staged OFD, close
must not clear the shared Regular0 metadata; it may be released only
after the last fd-table alias for that OFD is gone. Invalid source
fd returns EBADF; arg outside the fixed fdtable range returns
EINVAL; no free slot returns EMFILE. Runtime execve success scans
the current fixed fd table and closes entries whose close-on-exec bit
is set, recording scanned/closed/first-closed/remaining-open
diagnostics. dup3(24) supports targeted fd table replacement:
flags may be only 0 or O_CLOEXEC, oldfd == newfd returns EINVAL,
invalid oldfd or newfd outside the fixed fdtable range returns
EBADF, and success makes newfd point at the same
OpenFileDescription/backend as oldfd while setting the new fd
close-on-exec bit from flags & O_CLOEXEC. If newfd is already
open, this first slice replaces only the fd table entry; Linux
expand_files(), rlimit, EBUSY larval-fd detection, get_file(),
filp_close(), fput(), refcounts and concurrent fdtable locking stay
trimmed. Because the current single-child runtime still reuses
one FilesStruct object, clone/vfork must save a bounded parent fd
table and single regular/pidfd slot metadata snapshot, and child
exit must restore it before parent resume. This prevents child execve
close-on-exec from clearing the parent's fd entries, as observed by
`/bin/sh` later using fd10 for TIOCSPGRP. Dynamic fdtable growth,
dup(23), dup2, file refcounts, full copy_files/CLONE_FILES, complete
files unshare and concurrent fdtable locking remain trimmed.

fcntl(F_SETFL) must reference local Linux 6.12
include/uapi/asm-generic/fcntl.h and fs/fcntl.c::setfl(). This
first slice accepts only the current console-like TTY char-device
fd and only mutates the persisted O_NONBLOCK status bit in that fd
entry according to arg & O_NONBLOCK. It must preserve the original
access mode and already persisted flags such as O_LARGEFILE/
O_DIRECTORY, must not persist O_CLOEXEC as a file status flag, and
must keep multiple /dev/ttyN alias fd entries' F_GETFL/F_SETFL
state independent even though they share the same Tty0 backend. It
must leave regular files, directories, stdio fds, O_APPEND,
O_DIRECT, O_NOATIME, FASYNC, owner/signal state, locks, leases and
complete TTY/N_TTY nonblocking read semantics trimmed.

ioctl(TCGETS/TCSETS) must be accepted only on char-device fds and
use the riscv64/generic 36-byte struct termios described by
include/uapi/asm-generic/termbits.h. The first slice initializes a
console-like current termios from a Linux tty_std_termios-like
snapshot. TCGETS copies that current value to the user pointer.
TCSETS follows Linux 6.12 tty_mode_ioctl(TCSETS) /
set_termios(..., TERMIOS_OLD) user-visible shape for the immediate
path: copy one old struct termios from user memory, update the
current console-like termios state, and return 0. User copy failure
returns EFAULT; non char-device fds and unknown tty ioctls return
ENOTTY. Fixed-length copy_from_user/copy_to_user helpers must
precheck the current UserAddressSpace mapping and load/store
permissions before touching the address, so unmapped, overflowing
or permission-failing user pointers return EFAULT instead of a
supervisor page fault. The next TCGETS must observe the updated
value. TCSETSW, TCSETSF, output drain/wait, flush, driver and
line-discipline set_termios callbacks, locked termios, real TTY
locking, line-discipline behavior and isatty remain trimmed.

ioctl(TIOCGPGRP) must be accepted only on char-device fds that match
the current UserInitProcess controlling tty facts. It copies a
riscv64 pid_t/int foreground process-group id to the user pointer.
The first slice records PID1 as session leader, process-group
leader and foreground pgrp of the console-like controlling tty, so
success writes 1; if the foreground pgrp fact is absent, it should
follow Linux pid_vnr(NULL) shape and write 0 rather than inventing a
new errno.

ioctl(TIOCSPGRP) must follow the Linux tiocspgrp() user-visible
order for the current first slice: validate the fd as the current
console-like controlling tty, copy a riscv64 pid_t/int pgrp number
from the user pointer, reject negative pgrp with EINVAL, reject
unknown pgrp with ESRCH, reject a pgrp outside the current session
with EPERM, then update the foreground pgrp fact. The current
single-PID slice has pgrp 1 plus the observed child pgrp in the
current bounded identity view, so TIOCSPGRP can record either
supported foreground pgrp.

ioctl(TIOCGSID) follows Linux 6.12 tiocgsid(): validate the fd as a
console-like char-device tty, require the current syscall identity
to own that controlling tty, and copy the tty session id as a
riscv64 pid_t/int to the user pointer. No controlling tty/session
returns ENOTTY; user copy failure returns EFAULT.

ioctl(TIOCSCTTY) follows Linux 6.12 tiocsctty() only for the
observed getty first slice: after child setsid() succeeds, the
current child is a session leader with no controlling tty and may
bind a console-like TTY fd. Success records child tty session id as
child SID and foreground pgrp as child PGID. Non session leaders,
existing child controlling tty, non tty fds and incompatible tty
ownership return EPERM/ENOTTY. The observed arg == 1 is accepted but
tty steal/CAP_SYS_ADMIN is not implemented.

This is not full TTY job control: pty, TIOCNOTTY, real tty
refcount/locks, session_clear_tty, multi-session contention, orphan
pgrp, canonical N_TTY behavior and job-control signal delivery
remain trimmed.

nanosleep(101) must reference local Linux 6.12
kernel/time/hrtimer.c::sys_nanosleep(),
kernel/time/time.c::get_timespec64(), include/linux/time64.h and
include/uapi/asm-generic/unistd.h. The implementation must first
copy the 64-bit struct __kernel_timespec from rqtp and return
EFAULT on copy failure, then validate tv_sec >= 0 and
0 <= tv_nsec < 1e9 and return EINVAL on invalid values. The current
first slice is evidence driven by BusyBox /bin/sh's observed
req={0, 20ms}; it may support only bounded short relative sleeps
with duration <= 100ms by reading RiscvTimerProvider time until the
computed CLOCK_MONOTONIC-relative target tick is reached. Successful
completion returns 0 and does not write rmtp, matching Linux's
completed-sleep path. Longer sleeps, unavailable timer provider,
tick conversion overflow, signal interruption, rmtp remaining-time
copyout, restart blocks, timer slack, scheduler wait queues,
clock_nanosleep and multi-task sleep remain out of slice and must
not be presented as complete hrtimer semantics.

Unsupported syscall diagnostics must remain Linux-like and
low-side-effect: unknown or not-yet-supported syscall numbers return
ENOSYS and must not create successful-path checkpoints or mutate
user-visible state. When a real distro path reaches
nanosleep(101), the diagnostic may use the already captured user
argument registers to best-effort copy the 64-bit
struct __kernel_timespec at rqtp and print rqtp, rmtp, req_sec and
req_nsec, or req_copy=failed. This follows Linux 6.12
kernel/time/hrtimer.c::sys_nanosleep() argument layout and is
diagnostic evidence only; it must not change the ENOSYS return,
advance sepc differently, treat the syscall as supported, or guess
whether zero-duration, short sleep or full hrtimer/scheduler sleep
is required before the observed values are reviewed.
The early OpenRC/login socket(AF_UNIX,
SOCK_DGRAM|SOCK_CLOEXEC, 0) and sendto(206) path is tolerated
unsupported noise because returning ENOSYS does not prevent the
guest from reaching getty/login. The post-auth
socket(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC, 0) path is different:
focused evidence shows returning ENOSYS causes immediate login
failure before setgroups(159). The current Linux-differential
socket slice may therefore install only an unconnected AF_UNIX
stream socket fd via FilesStruct. Focused rerun after that slice
observes the post-auth stream socket returning fd 3, then the new
first boundary is connect(203) with fd=3 and addrlen=0x18. DGRAM,
sendto(206), successful connect and the network/socket operation
backend remain unsupported. The current connect slice accepts only
copied AF_UNIX pathname sockaddr_un values that satisfy Linux 6.12
unix_validate_addr() length/family checks, are not abstract paths,
and whose fd currently points at UnixSocket0. It must route pathname
existence lookup through current FsStruct.root and VFS path walk
without allocating FileRef, opening a file, or mutating the fd table.
The read-only lookup may follow existing fast symlinks and handle
. / .. components needed by Alpine /var/run -> ../run, clamped at the
current FsStruct.root; this still does not create or connect sockets.
Missing pathname returns ENOENT; existing pathname returns
ECONNREFUSED because no Unix socket/listener model is present. The
missing /var/run/nscd/socket focused rerun now reaches the next
direct boundary: setgroups(159) with gidsetsize=1 and a user gid_t
list pointer. User sockaddr copy fault returns EFAULT. Unsupported connect diagnostics
still name connect, print fd, sockaddr pointer, addrlen,
copy/family/validate/path classification and fd_unix_socket0, then
return ENOSYS for zero/oversized addrlen, invalid family/length,
non-AF_UNIX, abstract path, non-UnixSocket0 fd, non-pathname shape or
unmapped VFS failures. It must not install fds, mutate fd state,
write user memory, create a Unix socket peer/listener lookup,
implement sendto(206), network stack or complete
credentials. Focused evidence after the diagnostic classifies the
early addrlen=110 connects as pathname AF_UNIX sockets under
/run/utmps, and the post-auth fchown/fchmod boundary as addrlen=24
path /var/run/nscd/socket, now returning ENOENT when that path is
absent from the current rootfs and exposing setgroups(159).

The supported-syscall error diagnostic must stay behind the explicit
PROBE=user-syscall-error path. It observes supported syscall error
returns and must not change return values, errno mapping,
checkpoint ordering or user-smoke pass/fail policy. Path syscalls
that fail after a pathname has already been copied should print the
internal FileError class and printable path bytes. If a syscall such
as openat rejects dirfd, access mode or out-of-slice flags before
normal pathname copy, the diagnostic may print dirfd, flags,
unsupported flag bits, access mode and a best-effort pathname only
after the errno decision has been made. That best-effort copy must
be guarded by UserAddressSpace mapped-range facts; skipped or
failed copies are diagnostic output, not alternate errno behavior.
Supported execve(221) EFAULT diagnostics must print the filename,
argv and envp user pointers, child-continuation fact, stable
failure stage/reason/detail, and best-effort filename/argv/envp
prefix copies. Execve filename/argv C-string copies, including
these diagnostic prefix copies, use bounded per-byte readability up
to the first NUL and do not require the whole USER_PATH_MAX range to
be mapped; non-execve path syscall copies keep their existing
mapped-range guard. Filename-copy failure is reported as
filename_copy; argv pointer or string-copy failure is reported as
argv_copy. These fields are diagnostic only and must not change the
returned errno, checkpoint sequence, user memory or default output.
fcntl and ioctl error diagnostics should decode command names using
the local Linux 6.12 UAPI constants, including F_DUPFD,
F_DUPFD_CLOEXEC, F_GETFD, F_SETFD, F_GETFL, F_SETFL, TCGETS/TCSETS,
TIOCGWINSZ and TIOCSCTTY/TIOCGPGRP/TIOCSPGRP/TIOCGSID. Syscall name decoding should
include dup3 so nr 24 traces no longer appear as unknown.

A success-path trace such as PROBE=user-read-trace is separate from
user-syscall-error. It is required when a distro symptom can be
caused by a successful syscall result, for example read(63)
returning 0 and making /bin/sh treat stdin as EOF. Such a probe must
remain explicit, low noise and side-effect free.

PROBE=user-syscall-trace is the explicit syscall return/exit order
diagnostic for distro paths that have no visible error return. It
may print nr, decoded syscall name, return value, trap mode, a0..a5,
sepc and stval for syscall returns, and must separately print
exit/exit_group status before shutdown because those calls do not
return to the normal syscall return path. The trace may observe
unsupported or error returns, but it must not replace the
user-syscall-error detail probe, read or write user memory for
diagnostics, change return values, errno mapping, checkpoint
ordering, user-smoke pass/fail policy or default make test output.
Syscall-specific details such as ppoll nfds, timeout, sigmask,
ready count and bounded pollfd fd/events/revents may be printed only
from local facts already copied by the normal syscall path.

The first process-identity/UTS/getcwd slice must reference local
Linux 6.12 kernel/sys.c::sys_getpid()/sys_getppid()/
do_getpgid()/sys_getpgid()/SYSCALL_DEFINE1(getsid)/
sys_setpgid()/sys_geteuid()/sys_getegid()/ksys_setsid()/
sys_getresuid()/sys_getresgid()/sys_newuname(),
fs/d_path.c::sys_getcwd(), init/main.c::rest_init(),
init/version-timestamp.c::init_uts_ns and
include/uapi/linux/utsname.h. getpid must return the current
syscall identity's thread-group id: PID1 current returns 1, while a
visible observed-child continuation returns the child pid. Missing
child identity remains an explicit unsupported boundary rather than
silently returning PID1. getppid must model PID1's visible boot
idle/init_task parent as 0, without introducing a full task tree.
getpgid(0) must read the current UserInitProcess process group
and return the current PID1/observed-child pgrp. The first bounded
identity slice may also accept getpgid(1) as PID1 and the current
visible child pid as the child task; unknown or negative pid values
must follow Linux's failed lookup shape and return ESRCH. getsid(0)
must read the current syscall identity sid; getsid(1) returns PID1's
sid, the current visible child pid returns the child sid, and unknown
or negative pid values return ESRCH.
setpgid(154) must preserve Linux sys_setpgid() argument
normalization for pid/pgid zero: pid 0 means current syscall
identity and pgid 0 means the selected pid. The current bounded
slice may accept setpgid(0,0), setpgid(0,1), setpgid(1,0) and
setpgid(1,1), all leaving PID1 in pgrp 1, plus observed-child
setpgid shapes already proven by OpenRC/getty. Negative pgid must
return EINVAL; unknown pid must return ESRCH; unknown target pgrp in
the current single
session must return EPERM. Full tasklist lookup, RCU protection,
security_task_getpgid()/security_task_setpgid(), pid namespaces,
PF_FORKNOEXEC/EACCES and multi-process process-group state remain
trimmed. setsid(157) must follow Linux ksys_setsid()'s conservative
failure rule for PID1: because PID1 is already recorded as a session
leader and process-group leader with pgrp 1, PID1 setsid() returns
EPERM and must not create a new session, change SID/PGID or detach
the controlling tty. The observed getty child continuation first
slice may succeed when the child is visible, has a nonzero pid, is
not already a session leader and no process group equal to the child
pid exists; success returns the child pid and sets child SID/PGID to
that pid. Repeated child setsid() or an existing same-pid child pgrp
returns EPERM. Full tasklist/RCU, security_task_getsid(), pid
namespaces, successful PID1 setsid, pty, controlling-tty detach,
orphan pgrp and complete job-control semantics remain out of slice.
geteuid/getegid/getresuid/getresgid must read the existing root
credential substate and write uid_t/gid_t user results for getres*.
setgroups(159) must use local Linux 6.12
include/uapi/asm-generic/unistd.h::__NR_setgroups=159 and
kernel/groups.c::SYSCALL_DEFINE2(setgroups) as the reference. The
current first slice stores only UserInitProcess's bounded
supplementary group view: effective uid 0 may clear it with size 0
or copy one 32-bit gid_t from userspace with size 1; copy fault
returns EFAULT, non-root returns EPERM, and size > 1 remains an
unsupported ENOSYS diagnostic rather than claiming NGROUPS_MAX.
The focused OpenRC login rerun after this slice observes
/var/run/nscd/socket still returning ENOENT, setgroups(159)
returning 0, setgid(144) with gid=100 returning 0, and the direct
boundary at setuid(146) with uid=1000 returning EPERM. The current
setgid slice treats effective uid 0 as the bounded CAP_SETGID proxy
and synchronizes only UserInitProcess.gid/egid/sgid/fsgid to the
target gid. The current setuid slice treats effective uid 0 as the
bounded CAP_SETUID proxy and synchronizes only
UserInitProcess.uid/euid/suid/fsuid to a 32-bit target uid; non-root
effective uid still returns EPERM. This slice must not broaden
capabilities, user namespace, LSM, credential COW/RCU, permission
matrix, TTY ownership, inode ownership semantics, or saved-id/root
regain after setuid(1000).
Focused evidence after setuid(1000), setpgid(0,7) and TIOCSPGRP
succeed reaches getgroups(158) with gidsetsize=32 and a user
gid_t * before the login shell prompt. The current getgroups slice
only reads back the same fixed-capacity supplementary group view:
gidsetsize 0 returns the current count, too-small nonzero buffers
return EINVAL, large-enough buffers copy saved 32-bit gid_t values,
and copyout faults return EFAULT. Full group_info allocation/sort,
capabilities, user namespaces, LSM, task cred COW/RCU, file
permission checks, TTY ownership and inode ownership/mode semantics
remain trimmed.
uname must copy the six-field 65-byte new_utsname layout using the
static local Linux 6.12 generated UTS values. getcwd must use the
inherited FsStruct root/pwd view; the current slice covers only root
cwd and returns "/\\0" with the Linux return length that includes
the trailing NUL. User pointer failures must return EFAULT and
too-small getcwd buffers must return ERANGE. Writable UTS
namespaces, personality release override, hostname/domainname
mutation, full parent/child task graph, pid namespaces and non-root
credential/user namespace semantics remain trimmed.

Signal runtime first slices must reference local Linux 6.12
kernel/signal.c::sys_rt_sigprocmask(),
kernel/signal.c::sys_rt_sigaction()/do_sigaction(),
include/uapi/asm-generic/signal.h and
arch/riscv/include/uapi/asm/signal.h. The model boundary is Task ->
SignalRuntime, with ProcessSignalState for process/thread-group
shared signal state, ThreadSignalState for per-thread blocked and
pending state, and SignalActionTable for sighand-style
action[sig - 1] entries. The current single PID1 implementation may
fold these fields into UserInitProcess, but it must preserve the
separate facts so later clone/fork/thread-group support can split
them without changing syscall semantics. rt_sigprocmask(135)
updates ThreadSignalState.blocked; rt_sigaction(134) reads and
writes SignalActionTable entries.

rt_sigaction(134) must follow the Linux 6.12 order: reject
sigsetsize values other than riscv64 sizeof(sigset_t) == 8; copy the
optional user sigaction using the riscv64 kernel ABI layout
{ handler: usize, flags: usize, mask: usize }; reject invalid signal
numbers and attempts to install actions for SIGKILL or SIGSTOP;
snapshot the old action before applying the new action; clear
unsupported userspace flags and remove SIGKILL/SIGSTOP from the
stored action mask; then copy the old action to user memory when the
old-action pointer is non-null. User copy failures must return
EFAULT, invalid signal/sigsetsize/kernel-only set attempts must
return EINVAL. Full signal delivery, shared pending queues,
sighand/siglock/RCU locking, restartable syscalls, signal frame
construction, handler entry and rt_sigreturn remain trimmed.

user-smoke should keep signal syscall coverage as a dedicated
syscall-level case rather than embedding all signal checks in the
/bin/sh probe. That case may cover rt_sigprocmask and rt_sigaction
ABI, success, old-value writeback and first-slice error handling,
but it must not assert real handler delivery until SignalRuntime
delivery, frame and rt_sigreturn are specified.

APP=user-boot validation in make test must not treat QEMU/SBI
shutdown success as sufficient. The host harness must parse the
ordinary guest output line "user exit status=N" and require N == 0
for native and every configured provider. A nonzero status, missing
status line, or failed QEMU command is a visible test failure. The
guest exit syscall path still owns status printing and shutdown; the
harness only interprets the output. The default checked-in overlay
map must install the combined user_smoke fixture as /sbin/init, and
the harness must use a case-local disk image so the result cannot
pass by reusing stale build/virtio-blk.raw contents.

Long-term user-boot checkpoints should be expanded when they support
future Linux differential debugging. Existing boundaries such as
UserBoot.InitAttemptFailed, UserBoot.MainElfReady,
UserBoot.InterpreterReady and UserAddressSpace.Ready should expose
stable ELF/load-bias/mapping diagnostics before adding another
checkpoint. New checkpoints are appropriate only when a real distro
path reaches a stable behavior boundary not covered by the existing
set.

#### Whole-disk ext2 input

The current rootfs image is a whole-disk ext2 filesystem. The
UserBootPayload path must not require PartitionTable or
BlockPartition objects until the disk image format is changed to
include a partition table.

<!-- formal-predicate-notes:spec/coding/systems/kernel.spec END -->
