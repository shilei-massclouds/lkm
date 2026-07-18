# ExecTransaction coding contract

Model source: [`exec_transaction.spec`](../../model/objects/exec_transaction.spec). Implementation:
`impl/arceos_ex/src/objects/exec_transaction.rs`.

`Context` owns exactly one `ExecTransaction`, one current `UserAddressSpace`, and one transaction-owned
staging `UserAddressSpace`. Large ELF/address-space values must not be placed on the syscall trap stack.
`ExecArguments` stores an absolute filename plus dynamically allocated argv/envp vectors. Their limits come
only from `Config.exec_argument_limits`; no fixture-specific count may be embedded in the transaction or
usercopy code. The default mirrors Linux 6.12: `MAX_ARG_STRINGS=0x7fffffff`,
`MAX_ARG_STRLEN=32*PAGE_SIZE`, `_STK_LIM=RLIMIT_STACK=8 MiB`, and `ARG_MAX=128 KiB`. The effective default
string budget uses the `bprm_stack_limits()` calculation and is 2 MiB, less
`(max(argc, 1) + envc) * sizeof(void *)`. The terminating NUL counts toward both the per-string and aggregate
limits. Runtime `SyscallTable.Execve` permits the current PID 1 process or the observed child continuation,
then performs complete usercopy before `begin`; boot builds the same
value with `argv[0]`, `HOME=/`, and `TERM=linux`.

The staging `UserStack` follows the independent [`UserStack`](user-stack.md) contract: sparse physical pages,
128 KiB initial VMA expansion, 8 MiB rlimit, 1 MiB guard gap, an 8 MiB page-granular top-ASLR window and a
16-byte HWRNG-backed `AT_RANDOM` block. Prepare snapshots the absolute filename, common-ISA ELF HWCAP, and
the current real/effective UID/GID before commit; boot uses root values. It then issues one exact 24-byte
HWRNG read. The first 16 bytes and last 8-byte ASLR seed are passed as distinct inputs, so neither can affect
or reveal the other. Kernel stack canaries and compiler stack-protector policy remain deferred.

The only allowed call direction is:

```text
UserBootPayload or SyscallTable
  -> ExecTransaction
  -> BinaryFormatRegistry
  -> ElfBinaryFormat / ElfObject
  -> UserStack + staging UserAddressSpace + UserTrapFrame
  -> commit or abort
```

Prepare owns all fallible work. Its error mapping is `EFAULT` for pre-transaction usercopy, `E2BIG` for
argument/string capacity, `ENOENT` for main/interpreter lookup, `ENOEXEC` for format rejection, and `ENOMEM`
for backing/page-table allocation, and `EAGAIN` for unavailable or any non-24-byte HWRNG input. Entropy acquisition is
pre-commit; boot treats this failure as terminal while runtime preserves the old image. Abort releases every staging page/page-table and resets the active slot;
current address space, SATP, trap frame, fd table and process identity are immutable on this path.

When bounded usercopy reaches a configured count, string, or aggregate limit, the syscall-error diagnostic may
inspect the accepted prefix plus the first rejected entry. This observation happens before `begin` and must not
truncate the vector, mutate user memory, or turn the rejected call into a transaction. `Vec::try_reserve` or an
equivalent fallible allocation path is required; allocator failure maps to `ENOMEM`, not panic or `E2BIG`.

Commit first verifies all fallible facts and CLOEXEC capacity, then sets point-of-no-return. It swaps the
current/staging image, applies the prechecked bounded CLOEXEC pass, installs main/interpreter/stack/trap-frame,
does the owner handoff, releases retired backing after SATP no longer references it, and resets the slot.
Returning `ExecError` after point-of-no-return is forbidden; an invariant failure is terminal.

The bounded `UserChildProcess` fork/vfork surrogate is an ownership exception, not a leak: when its saved
parent snapshot has the same SATP as the retired image, commit transfers the retired address-space and
`UserStack` ownership to that snapshot instead of freeing parent pages. Child exit releases the replacement
child image (including page tables and stack backing), restores both parent objects, and only then resumes the
parent. A transaction may reset only after either release or this explicit ownership transfer succeeds.
For PID 1 self-exec there is no saved parent owner: commit preserves PID/process identity and releases the
retired address space/stack after switching to the replacement image.

Checkpoint ownership stays owner-scoped. The transaction dispatches the existing stable `UserBoot.*` or
`UserExec.*` sequence without renaming, reordering or double-emitting checkpoints. Observation fields live
with the existing checkpoint owner and may be populated through narrow crate-internal hooks. Appended fields
cover stack top max, selected top, ASLR offset, execfn pointer and the stack-computed auxv-complete fact.
