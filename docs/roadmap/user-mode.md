# 用户态 payload 历史

> 本文从 `docs/ROADMAP.md` 拆出，只保留专题历史、证据和细节。当前优先级、状态和下一步顺序以 [`../ROADMAP.md`](../ROADMAP.md) 为唯一入口。

> 下文“当前”“后续高优先级”“P0/P1”等措辞是当时执行快照，不构成活跃计划。

## 用户态 payload 阶段归档

用户态 `helloworld` 快速路径已经完成，并已推进到 dynamic musl libc `/sbin/init` 综合 smoke：`APP=user-boot` 能从当前 ext2 rootfs 读取构造期 overlay 的 `/sbin/init`，识别 `PT_INTERP=/lib/ld-musl-riscv64.so.1`，读取并映射 musl interpreter，建立 `UserAddressSpace` / `UserTrapFrame` / `UserInitProcess`，写入 `satp`、执行 `sfence.vma` 并通过 `sret` 进入 U-mode。真实运行已支持 `set_tid_address`、阶段性 `brk/mmap/mprotect/munmap`、`writev` 错误输出、read-only `openat/read/close/newfstatat`、directory `openat/getdents64`、stdio fd table 到 console char-device backend，以及 `write/exit`；默认 overlay 的 `user_smoke` dynamic musl 产物输出逐项 `syscall ... ok`、`user hello` 和 `user exit status=0`。

本轮已经完成的关键闭环：

1. **Payload / exec 对象边界**。`PayloadPhase` 已确认为 `Kernel` 生命周期末尾阶段；`UserBootPayload`、`ElfObject`、`UserAddressSpace`、`UserStack`、`UserTrapFrame`、`SyscallException` / `SyscallTable`、`FilesStruct` 和 `UserInitProcess` 的模型、coding 与实现事实已经首轮收口。`SyscallException` 沿用 `ExceptionStream`，不建立独立 `SyscallDispatcher`；`UserInitProcess` 表示 PID 1 的 `KernelInitTask` 经 exec/user entry 后获得用户态身份，不创建第二个 task。
2. **用户 ELF 与地址空间**。`ElfObject` 支持主程序和 interpreter 两种 role，`UserAddressSpace` 在同一低地址用户区映射主 ELF、musl interpreter、用户栈和阶段性 heap/mmap arena；ELF segment backing 和 low-half leaf PTE 按页粒度覆盖，已支持 GNU_RELRO 所需的页粒度 `mprotect`。
3. **U-mode 与 syscall 首片**。RISC-V trap return 已用 `within UserModeTrapReturnContext { ... }` 表达不可返回交接；用户态 trap 入口通过 `sscratch` 切回内核 trap 栈，user ecall 进入 `SyscallException -> SyscallTable`。boot CPU `UserInitProcess` 的 VMAP kernel trap stack 已补 kernel-context early overflow bit-test、寄存器保持、静态 overflow-stack 完整 frame 和 SBI terminal diagnostic；per-task/per-CPU 泛化与 IRQ hardirq stack switch 后续展开。当前 syscall 覆盖 `write/writev`、read-only `openat/read/close/newfstatat`、有界 `pipe2(flags=0)`、`brk/mmap/mprotect/munmap`、`set_tid_address` 和 `exit/exit_group`。
4. **rootfs 与文件读取支撑**。VFS path walk 已从 `FsStruct.root` 出发解析绝对路径，rootfs 已切到 ext2，Ext2/VFS/BufferHead 路径已支持 regular file 多 direct-block 和 single-indirect read，因此能读取 Alpine rootfs 中约 600KiB 的 musl loader。
5. **缺陷回归证据**。DF-0001 原始 `/sbin/init` 间歇读取失败已通过 virtio-blk 同步请求生命周期修复并归档；后续普通和 `stress-mem` user-boot 回归均未再复现 `read user ELF failed`。DF-0002 近期失败已定位为 PLIC/UART probe 观察约束过强或相邻诊断缺口，继续由 stress 回归覆盖。

LTP 自动验收首轮取证在 native 与 linux-object 两侧得到相同边界：
`run-syscalls.sh --list -- 'uname*'` 尚未枚举测试，就在第 30 行用于解析安装目录的 BusyBox
command substitution 触发 RISC-V syscall 59，并因 `pipe2` 返回 ENOSYS 报
`can't create pipe: Function not implemented`。因此当前片只规格化 flags=0、最低两个 fd、失败原子
回滚、单有界 pipe、dup/close/EOF 和 observed-child 写入数据跨 fd snapshot/restore 保留；完整 flags、
阻塞等待、多 pipe、refcount/task graph、信号化 EPIPE，以及后续可能出现的 LTP ELF/VFS 或
`personality(2)` 边界继续 deferred，不从静态风险直接猜修。

实现后 `kernel-smoke-native` 的 pipe fd pair、方向、round-trip、dup/close、EOF、snapshot handoff、
EFAULT/EMFILE 回滚和非法 flags 覆盖通过。正式 `make run TEST=ltp` 与 `TEST=ltp-lo` 都已越过 syscall
59；两侧新的首个运行时边界一致为同一第 30 行 command substitution 中的
`clone(220) stage=child_context_unsupported`，参数为 `flags=0x11`、`newsp=0`、
`current_child_continuation=1`，guest status 2。list 尚未输出任何 uname 条目，因此本轮不继续修改
clone、ELF/VFS 或 personality；两份 result/manifest/qemu.log 保留该红灯和完整 cleanup 事实。

2026-07-19 扩展 clone 失败诊断并由根 `make test` 在两种 provider 重现：两侧均记录
`current_child_source=pid1_plain_fork`，外层 PID1 wait frame/address/fd/stack/writable snapshot 所有权
全部为 1，且唯一命中的具体拒绝条件是 `reject_outer_not_vfork=1`，其余 178/180 门禁通过。当前片
因此规格化为单 builtin-only grandchild：独立保存内层 child 与 script-parent continuation，保持外层
PID1 snapshot 所有权，inner exec 明确 `ENOSYS`；list 后的新首个执行边界作为下一轮证据。
实现首轮由 syscall trace 进一步观察到 BusyBox parent 在 wait4 前先关闭 pipe 写端并阻塞读取空 pipe；
旧 TTY 自旋等待无法调度 child。当前规格因此把该首个阻塞 pipe read 与 wait4 都作为有界 handoff，
child exit 恢复并重试 parent read，随后 wait4 只 reap 已完成 grandchild，不扩展通用 pipe 调度。

该有界二层 plain-fork 片完成后，native 的
`20260719T010657.225045Z-ltp-10960` 与 linux-object 的
`20260719T011021.698067Z-ltp-lo-11145` 都在 list 阶段各精确输出一次
`uname01\tuname01`、`uname02\tuname02`、`uname04\tuname04`，随后真实进入
`=== uname01: uname01`。两侧第一个新执行边界相同：builtin grandchild 请求
`execve("/bin/sh", ["/bin/sh", "-c", "uname01"], envp)`，实现以
`exec_boundary=builtin_grandchild_enosys` 明确拒绝。严格 3/3 acceptance 保持不变，因此两份正式
case 仍为预期红灯，但 list 首片已闭合，完整 inner exec 与后续 uname ELF/VFS/personality 只按该新
边界进入下一片；两份 result 均记录 private disk removed 和 process group reaped。

新增对象 smoke 初次复跑没有用探针猜修：QEMU GDB 在稳定停机后确认唯一运行 hart 递归 fault 于
`formal_event_entry`，原始调用链为 canonical-ELF scenario -> builtin-grandchild helper ->
`copy_plain_fork_from_parent` -> `save_parent_fd_snapshot`。目标函数入口真实
`sp=0xffffffc8006082c0`，距 16 KiB kernel-init 栈底 `0xffffffc800608000` 仅 704 字节，而函数 prologue
固定需要 944 字节。测试随后按 testing spec 拆成独立 builtin-grandchild scenario，并把原有
child-lifecycle coverage 放到后置独立 scenario；native 与 linux-object 对象 smoke 均恢复为 55/55。

后续高优先级计划按证据和前置依赖排序：

1. **P0：继续保留 DF-0001/DF-0002 stress 回归并分析 source-scoped 失败事实**。短期不再添加缺陷专用 checkpoint；`Serial8250RxBatchLoopbackProbe.setup` 内部稳定 first-failed predicate 已补齐到 source claim/complete delta matched。若再次出现 DF-0002，优先沿既有 `failure_diagnostic` / `ready_check_failed` 中的 source-scoped PLIC/UART IRQ cycle 事实分析，不回退到全局 claim/complete equality。
2. **BusyBox init getty/login shell bounded job-control 验收**。真正阻断闭环的是 login 交互验收：TTY 输入链路已能消费 host 输入，但一次性 delayed stdin 会把后续命令交给 `Password:`；同时 bare Alpine minirootfs 的 `root:*` 被锁定，`ROOTFS_OVERLAY=none` 不应自然登录。显式 account-only `ROOTFS_FILE_OVERLAY_DIR` 继续提供专用测试用户和 test-only 密码 hash，未设置 overlay 时保持 bare rootfs 账号语义。focused run 已越过 `login:` / `Password:`、认证、单层 nested vfork takeover、fd-local `fchown(55)` / `fchmod(52)`、AF_UNIX stream socket fd 创建、`/var/run/nscd/socket` pathname `ENOENT`、root-only `setgroups(159)`、`setgid(144, gid=100)`、`setuid(146, uid=1000)` root-euid drop、登录 shell `execve(221)` filename/argv bounded C-string copy 和 MOTD 输出；登录 shell 已进入 `~ $`。本片前 focused baseline 使用 account overlay、`PROBE=user-syscall-trace,user-syscall-error` 和 staged `/bin/ls\nexit\n`，确认 `/bin/ls` 触发 `clone(220)`，`flags=0x11`、`flags_wo_csignal=0`、`exit_signal=17`、`newsp=0`、`current_child=1`、`child_pid=7`、`nested_parent_pid=6`、`active_slot_reusable=0`、`completed_records_total_archived=3`、`completed_records_reaped=3`、`next_child_pid=8`，旧行为返回 `ENOSYS` 并打印 `can't fork: Function not implemented`。该边界不是 execve filename copy，也不是已支持的 BusyBox-init nested vfork，而是 observed child continuation 内的 plain fork；本片把它规格化为单 internal `UserChild` slot 的 observed grandchild：只接受 SIGCHLD-only、`newsp=0`、当前登录 shell child continuation，clone 保存 shell parent facts 并返回 `next_child_pid`，shell `wait4(-1, status, allowed_options, NULL)` 时才 handoff 到 `/bin/ls` grandchild，grandchild exit 后恢复 shell wait4。最终 focused 1/1 复跑记录父侧 `setpgid(8, 8)` 的稳定 `target=pending_child` 成功事实，不再出现 `reason=pid_not_visible`；handoff 后 `ioctl(TIOCSPGRP)`（`a1=0x5410`）返回 0，`/bin/ls` 输出 `lost+found`，并以 `user exit status=0` 结束。该闭环只增加单 pending grandchild 的 clone-return 到 handoff/exec 窗口，不引入正式多 runnable user task graph、完整 COW/mm、通用 wait/reap、post-exec parent setpgid、job-control signal 或多任务并发。该 case 继续作为 opt-in 诊断，不进入默认 `make test`；早期 DGRAM syslog path、`sendto(206)`、成功 `connect(203)` 和 `getgroups(158)` 仍由后续证据分别规格化。
3. **P0：发行版 rootfs smoke 与 overlay 分层**。overlay 继续作为 `user_smoke` 和 staged probe fixture；发行版路径已新增 `ROOTFS_OVERLAY=none init=/bin/ls` 与 delayed-input `init=/bin/sh` smoke，并已覆盖 `/bin/sh -> /bin/ls` 外部命令闭包。下一步只沿真实证据推进BusyBox init 的 rc.local/local service 路径，不把 overlay 机制提前删除，也不通过更换输入形态或测试专用 kernel API 制造通过结果；rc.local 初期保持 manual/focused case，不进入默认 `make test` 强制门禁。
4. **P1：Rootfs/VFS/Ext2 正式化补强**。补 `/dev` 在 ext2 root 下的挂接策略、mount namespace/chroot/cwd/pwd 规则、VFS inode/dentry cache 边界、negative lookup/refcount/evict deferred，以及 page cache / `AddressSpace` / folio read path；这些不阻塞 dynamic fixture，但会成为发行版用户态和长期文件系统语义的前置。
5. **P1：用户态 I/O 与 TTY 分层**。保持当前 stdio fd 到 console char-device backend 的最小路径；完整 `/dev/console`、TTY/N_TTY、line discipline、stdin/stdout 语义、poll、pipes 和普通文件写路径继续后置，需在 printk console 与 serial8250 runtime 层职责稳定后推进。

## 用户栈 initial ABI 与布局首片（2026-07-16）

本片把 exec initial stack 收敛到当前能够准确表达的 Linux RISC-V 基线。每次 exec 在
point-of-no-return 前只读取一次 24 字节 HWRNG：前 16 字节仅写入 `AT_RANDOM`，后 8 字节仅用于
在 `stack_top_max=0x40000000` 以下的 8 MiB 窗口选择页对齐栈顶；initial SP 继续 16 字节对齐，
本次选定 top 同时约束 rlimit、guard、demand growth 和 ownership。initial stack 独立复制 exec
filename，`AT_EXECFN` 不复用 `argv[0]`。auxv 现包含 HWCAP、PAGESZ、CLKTCK、PHDR/PHENT/PHNUM、
BASE、FLAGS、ENTRY、exec 前 UID/EUID/GID/EGID、SECURE、RANDOM、EXECFN 和 NULL；没有事实来源的
platform、HWCAP2、rseq 与 vDSO 条目仍不生成。short/unavailable entropy 在 commit 前返回
`EntropyUnavailable`，runtime 映射为 `EAGAIN` 并保留旧映像。

稳定的 `UserAddressSpace.Ready` checkpoint 没有插入或重排，诊断已扩展为 top max、选定 top、
ASLR offset、execfn pointer 和 auxv completeness，并改为在 boot commit 前读取 transaction staging
对象。对象 smoke 覆盖配置端点、布局不碰撞、seed 可预测性、两段熵独立性、filename 与
`argv[0]` 不同、完整 auxv、24 字节熵失败回滚，以及随机 top 下既有 growth/guard/rlimit/usercopy/
snapshot 行为；guest smoke 通过 `getauxval()` 验证当前 invocation 的 `AT_EXECFN`、HWCAP、
credentials 和非零 `AT_RANDOM`，并保留 stack protector、192 KiB cross-page usercopy 和 256 KiB
栈增长。

DF-0003 的首轮 30 次复现给出同一稳定边界：boot 和第一次 exec 各消费 24 字节后，virtio-rng
缓存只剩 16 字节，第二次 exec 因 short read 返回 `EAGAIN`。因此 virtio-rng 规格与实现增加长期
可用的 request low-watermark refill：一次完整读取后，若余量小于本次请求长度，就丢弃尾部并
重新提交 full buffer；真正的 short completion 仍向 exec 暴露实际长度并失败。修正后 DF-0003
30/30；默认 stress 的 DF-0001、DF-0002、DF-0003 各 10/10；Linux exact baseline 与默认
rc.local paired difftest 各 1/1。对象 smoke 55/55，真实 user guest 与 address-space checkpoint
probe 通过，最终仓库根 `make test` 为 171/171。动态 rlimit/越界 signal、COW/多线程栈与通用
VMA fault、完整 CRNG 和内核 compiler protector 继续由主 roadmap 的 P1/P2 行承载。
