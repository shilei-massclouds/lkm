# 用户态 payload 历史

> 本文从 `docs/ROADMAP.md` 拆出，只保留专题历史、证据和细节。当前优先级、状态和下一步顺序以 [`../ROADMAP.md`](../ROADMAP.md) 为唯一入口。

## 当前进度与后续高优先级计划：用户态 payload

用户态 `helloworld` 快速路径已经完成，并已推进到 dynamic musl libc `/sbin/init` 综合 smoke：`APP=user-boot` 能从当前 ext2 rootfs 读取构造期 overlay 的 `/sbin/init`，识别 `PT_INTERP=/lib/ld-musl-riscv64.so.1`，读取并映射 musl interpreter，建立 `UserAddressSpace` / `UserTrapFrame` / `UserInitProcess`，写入 `satp`、执行 `sfence.vma` 并通过 `sret` 进入 U-mode。真实运行已支持 `set_tid_address`、阶段性 `brk/mmap/mprotect/munmap`、`writev` 错误输出、read-only `openat/read/close/newfstatat`、directory `openat/getdents64`、stdio fd table 到 console char-device backend，以及 `write/exit`；默认 overlay 的 `user_smoke` dynamic musl 产物输出逐项 `syscall ... ok`、`user hello` 和 `user exit status=0`。

本轮已经完成的关键闭环：

1. **Payload / exec 对象边界**。`PayloadPhase` 已确认为 `StartupTimeline` 末尾阶段；`UserBootPayload`、`ElfObject`、`UserAddressSpace`、`UserStack`、`UserTrapFrame`、`SyscallException` / `SyscallTable`、`FilesStruct` 和 `UserInitProcess` 的模型、coding 与实现事实已经首轮收口。`SyscallException` 沿用 `ExceptionStream`，不建立独立 `SyscallDispatcher`；`UserInitProcess` 表示 PID 1 的 `KernelInitTask` 经 exec/user entry 后获得用户态身份，不创建第二个 task。
2. **用户 ELF 与地址空间**。`ElfObject` 支持主程序和 interpreter 两种 role，`UserAddressSpace` 在同一低地址用户区映射主 ELF、musl interpreter、用户栈和阶段性 heap/mmap arena；ELF segment backing 和 low-half leaf PTE 按页粒度覆盖，已支持 GNU_RELRO 所需的页粒度 `mprotect`。
3. **U-mode 与 syscall 首片**。RISC-V trap return 已用 `within UserModeTrapReturnContext { ... }` 表达不可返回交接；用户态 trap 入口通过 `sscratch` 切回内核 trap 栈，user ecall 进入 `SyscallException -> SyscallTable`。当前 syscall 覆盖 `write/writev`、read-only `openat/read/close/newfstatat`、`brk/mmap/mprotect/munmap`、`set_tid_address` 和 `exit/exit_group`。
4. **rootfs 与文件读取支撑**。VFS path walk 已从 `FsStruct.root` 出发解析绝对路径，rootfs 已切到 ext2，Ext2/VFS/BufferHead 路径已支持 regular file 多 direct-block 和 single-indirect read，因此能读取 Alpine rootfs 中约 600KiB 的 musl loader。
5. **缺陷回归证据**。DF-0001 原始 `/sbin/init` 间歇读取失败已通过 virtio-blk 同步请求生命周期修复并归档；后续普通和 `stress-mem` user-boot 回归均未再复现 `read user ELF failed`。DF-0002 近期失败已定位为 PLIC/UART probe 观察约束过强或相邻诊断缺口，继续由 stress 回归覆盖。

后续高优先级计划按证据和前置依赖排序：

1. **P0：继续保留 DF-0001/DF-0002 stress 回归并分析 source-scoped 失败事实**。短期不再添加缺陷专用 checkpoint；`Serial8250RxBatchLoopbackProbe.setup` 内部稳定 first-failed predicate 已补齐到 source claim/complete delta matched。若再次出现 DF-0002，优先沿既有 `failure_diagnostic` / `ready_check_failed` 中的 source-scoped PLIC/UART IRQ cycle 事实分析，不回退到全局 claim/complete equality。
2. **P0：OpenRC getty/login shell 当前计划**。真正阻断闭环的是 login 交互验收：TTY 输入链路已能消费 host 输入，但一次性 delayed stdin 会把后续命令交给 `Password:`；同时 bare Alpine minirootfs 的 `root:*` 被锁定，`ROOTFS_OVERLAY=none` 不应自然登录。当前推进片新增显式 account-only `ROOTFS_FILE_OVERLAY_DIR`，用专用测试用户和 test-only 密码 hash 提供登录凭据，同时保持未设置该 overlay 时的 bare rootfs 账号语义不变。focused run 已越过 `login:` / `Password:` 输入和认证；旧直接阻断是 BusyBox login 认证后在现有 OpenRC/getty child continuation 内调用 `clone(220)` vfork，返回 `ENOSYS` 并打印 `login: vfork: Function not implemented`。该单层 nested vfork takeover 已闭合，登录后 fd-local `fchown(55)` / `fchmod(52)` 也已闭合：`PROBE=user-syscall-trace` 复现 `fchown(fd=0, uid=1000, gid=100)` 和 `fchmod(fd=0, mode=0600)` 均返回 0。本片增强 `socket(198)` unsupported 诊断后完成分类：早期 `socket(AF_UNIX, SOCK_DGRAM|SOCK_CLOEXEC, 0)` 返回 `ENOSYS` 后还能继续到 `sendto(206)`，属于 tolerated/noisy syslog-like path；真正 post-auth 阻断是 `fchmod` 后的 `socket(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC, 0)`，返回 `ENOSYS` 后未观察到 `setgroups(159)`，而是写出 `login: can't set groups: Function not implemented` 并 `exit_group(1)`。该 case 暂保留为 `TEST_OPENRC_LOGIN=1` opt-in 诊断；下一片若继续推进，应规格化最小 AF_UNIX/socket 首片，`sendto(206)`、`connect`、`setgroups(159)`、完整 credentials、TTY ownership 和 inode 持久化继续 deferred。
3. **P0：发行版 rootfs smoke 与 overlay 分层**。overlay 继续作为 `user_smoke` 和 staged probe fixture；发行版路径已新增 `ROOTFS_OVERLAY=none init=/bin/ls` 与 delayed-input `init=/bin/sh` smoke，并已覆盖 `/bin/sh -> /bin/ls` 外部命令闭包。下一步只沿真实证据推进原生 OpenRC 的 rc.local/local service 路径，不把 overlay 机制提前删除，也不通过更换输入形态或测试专用 kernel API 制造通过结果；rc.local 初期保持 manual/focused case，不进入默认 `make test` 强制门禁。
4. **P1：Rootfs/VFS/Ext2 正式化补强**。补 `/dev` 在 ext2 root 下的挂接策略、mount namespace/chroot/cwd/pwd 规则、VFS inode/dentry cache 边界、negative lookup/refcount/evict deferred，以及 page cache / `AddressSpace` / folio read path；这些不阻塞 dynamic fixture，但会成为发行版用户态和长期文件系统语义的前置。
5. **P1：用户态 I/O 与 TTY 分层**。保持当前 stdio fd 到 console char-device backend 的最小路径；完整 `/dev/console`、TTY/N_TTY、line discipline、stdin/stdout 语义、poll、pipes 和普通文件写路径继续后置，需在 printk console 与 serial8250 runtime 层职责稳定后推进。
