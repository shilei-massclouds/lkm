# UserStack

`UserStack` 表示 exec 创建并由当前用户映像独占的可向下增长用户栈。它与
`UserAddressSpace` 的 VMA 元数据、页表对象和 `UserTrapFrame` 分离：栈对象拥有本 mm 的稀疏
`UserFrame` backing 引用，普通 fork 后 parent/child 栈对象可以引用同一物理 frame，但两者的引用、
VMA、页表和 ownership token 独立。

## 配置与初始映像

- `Config.user_stack` 固定本轮 Linux-like 默认值：`stack_top_max=0x40000000`、
  `aslr_window=8 MiB`、
  `initial_expand=128 KiB`、`rlimit_stack=8 MiB`、`guard_gap=256*PAGE_SIZE`、
  `random_bytes=16`。
- 每次 exec 在 point-of-no-return 前一次性取得 24 字节 HWRNG：前 16 字节只复制为
  `AT_RANDOM` 数据，后 8 字节只按 little-endian seed 计算页粒度 ASLR offset，二者不得复用或向
  用户态泄露。offset 是 `seed & (8 MiB / PAGE_SIZE - 1)` 个页面，因此覆盖
  `0..8 MiB-PAGE_SIZE`；本次 `top=stack_top_max-offset`，rlimit、guard、增长和 ownership token
  全部绑定该 top。short/unavailable entropy 在 point-of-no-return 前失败。
- exec 从选定栈顶向下独立复制 filename、argv、envp、16 字节随机块和 auxv；`AT_EXECFN` 指向
  filename 副本而不复用 `argv[0]`，最终 SP 保持 16 字节对齐。
- 当前可准确表达的 Linux RISC-V initial auxv 基线为 `AT_HWCAP`、`AT_PAGESZ`、`AT_CLKTCK`、
  `AT_PHDR`、`AT_PHENT`、`AT_PHNUM`、`AT_BASE`、`AT_FLAGS`、`AT_ENTRY`、
  `AT_UID/EUID/GID/EGID`、`AT_SECURE`、`AT_RANDOM`、`AT_EXECFN`、`AT_NULL`。
  HWCAP 来自 exec 前已建立的公共 ISA facts，`AT_CLKTCK=100`、`AT_FLAGS=0`，credentials 使用
  exec 前快照（boot 为 root），当前无 secure-exec，故 `AT_SECURE=0`。没有支撑的
  `AT_PLATFORM`、`AT_HWCAP2`、rseq 和 vDSO auxv 不得伪造。
- 初始 VMA 在实际参数页下方额外预扩展 128 KiB，但只为实际写入的页分配 backing。栈保持 RW、NX。
- 8 MiB ASLR 窗口下最小 rlimit base 仍高于当前 heap/mmap arena；setup 必须拒绝未页对齐、窗口
  非页粒度/非 2 的幂、offset 越界或会与低地址映射区碰撞的配置。

## 增长与失败原子性

`UserAddressSpace` 的统一用户 fault core 可以 fault-in 当前栈 VMA 内未分配页，也可以在 8 MiB
rlimit、1 MiB guard gap 和相邻 mapping 允许时请求本对象向下移动 VMA base。跨页跳跃只分配
fault page。新 backing、L0 页表或 PTE 安装任一步失败都回滚新资源和 VMA metadata；成功只对
fault VA 执行定点 `sfence.vma` 并以 `RetrySameInstruction` 重试原 `sepc`。本对象只实现稀疏
backing 与增长约束，不自行分类 instruction、权限、越界、冲突或非 stack fault。

usercopy 对合法栈范围使用同一 range resolver 逐页 fault-in；失败仍返回 `EFAULT`。checkpoint
诊断在既有稳定时点记录 top max、选定 top、offset、execfn pointer 和 auxv 完整性，不新增、插入或
重排 checkpoint。普通 fork 在 child 发布前为每个已驻留 stack page acquire 独立引用，并把双方 leaf
降低为 RO+COW；任一方写缺页按 `UserAddressSpace` 统一策略解共享。parent/child 栈字节不得通过
wait/exit snapshot 回滚实现隔离。栈范围的无映射或权限 fault 由 `UserAddressSpace` 按同步致命
SIGSEGV 统一收口；动态 `RLIMIT_STACK`、多线程栈、`MAP_STACK/MAP_GROWSDOWN`、用户 handler
delivery、完整 CRNG 和内核 compiler stack protector 不在本轮。
