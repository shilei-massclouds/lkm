# ExecTransaction

`ExecTransaction` 是一次 `kernel_execve` 或用户态 `execve(221)` 的 Context-owned 工作单元。Context
只持有一个可重复使用的 active slot；首轮不声称支持并发 exec。它不是持久服务对象 `ExecCore`，也不拥有
候选 init 策略。

## 责任

- 在进入任何会分配 staging 资源的步骤前，接收已经归一化的绝对路径、`argv`、`envp` 和 `ExecOwner`。
- 通过 `Config.exec_argument_limits` 接受 argv/envp，不为当前 BusyBox 样本硬编码项数。
  Linux 6.12 默认值为 `MAX_ARG_STRINGS=0x7fffffff`、`MAX_ARG_STRLEN=32*PAGE_SIZE`、
  `_STK_LIM=RLIMIT_STACK=8 MiB` 和 `ARG_MAX=128 KiB`；因此 `bprm_stack_limits()` 在该默认
  rlimit 下得到 2 MiB 字符串预算，再扣除 `max(argc,1)+envc` 的指针表字节。
  `ExecArguments` 按这一总预算动态保存数据，分配失败为 `ENOMEM`，超过计数、
  单字符串或总预算为 `E2BIG`。boot 至少传入 `argv[0]=selected_path`、`HOME=/`、
  `TERM=linux`，runtime 的 usercopy 在 transaction 启动前完成。
- 经 `BinaryFormatRegistry` 选择 handler，持有 main/interpreter `ElfObject`、staging address space、stack、
  trap frame、失败事实和 point-of-no-return。
- 首轮仍使用固定大小用户栈，但 boot/runtime transaction 的 staging stack 映射 128 KiB。当前 Alpine
  BusyBox `/bin/sh` 在 `0x96d86..0x96d94` 可复现地逐页探测一个 64 KiB 栈帧，初始 argv/envp/auxv 与
  调用帧还需要额外空间；动态栈扩展、guard page 和内核侧 stack canary/protector 留待后续对象化。
- 在 point-of-no-return 前完成文件读取、格式识别、解释器读取、所有 staging 分配、页表安装和 bounded
  CLOEXEC 预检。失败释放全部 staging backing 并保持 current mm、SATP、trap frame、fd table 和进程身份不变。
- 成功时依次标记 point-of-no-return、交换 current/staging image、执行已预检 CLOEXEC、安装映像/stack/
  trap frame、完成 owner-specific handoff、释放 retired mm，然后 reset slot。
- `UserChildProcess` 的 bounded fork/vfork 替代路径若仍持有 parent mm，runtime exec 不得把该 parent backing
  当作无引用 retired mm 释放；transaction 将 address-space/stack 所有权交给 parent snapshot，child exit
  释放被替换的 child image 后再恢复 parent。无 parent snapshot 的 boot/runtime 路径仍在 handoff 后立即释放。

## 错误与提交边界

普通失败只能发生在 point-of-no-return 前：usercopy fault 为 `EFAULT`，参数容量超限为 `E2BIG`，文件或
解释器缺失为 `ENOENT`，没有 handler 接受映像为 `ENOEXEC`，分配失败为 `ENOMEM`。point-of-no-return
后不得返回普通 errno；内部不变量破坏进入 terminal panic，完整 fatal-signal 语义后续展开。

`ExecOwner` 只决定输入来源、成功交接和 checkpoint namespace。boot/runtime 共用同一 prepare/dispatch/
commit/abort 管线，但继续发出稳定的 `UserBoot.*` 或 `UserExec.*` checkpoint；既有 ID、名称和顺序不变。

## 非目标

首轮不引入并发 transaction、credentials/signals/LSM 完整切换、COW/VMA、完整 task graph 或持久
`ExecCore`。`ExecArguments`、`ExecError`、`ExecOwner` 是值类型，不建立独立生命周期文件。
