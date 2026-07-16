# UserStack

`UserStack` 表示 exec 创建并由当前用户映像独占的可向下增长用户栈。它与
`UserAddressSpace` 的 VMA 元数据、页表对象和 `UserTrapFrame` 分离：栈对象是稀疏物理 backing
的唯一 owner，stack mapping 只保存 VMA 范围、RW/NX 权限和 ownership token。

## 配置与初始映像

- `Config.user_stack` 固定本轮 Linux-like 默认值：`stack_top=0x40000000`、
  `initial_expand=128 KiB`、`rlimit_stack=8 MiB`、`guard_gap=256*PAGE_SIZE`、
  `random_bytes=16`。
- exec 从栈顶向下复制 filename/argv/envp、16 字节 HWRNG 随机块和 auxv；`AT_RANDOM` 指向该
  栈内随机块，最终 SP 16 字节对齐。
- 初始 VMA 在实际参数页下方额外预扩展 128 KiB，但只为实际写入的页分配 backing。栈保持 RW、NX。

## 增长与失败原子性

当前 SATP 的用户 load/store fault 可以 fault-in VMA 内未分配页，也可以在 8 MiB rlimit、1 MiB
guard gap 和相邻 mapping 允许时向下移动 VMA base。跨页跳跃只分配 fault page。新 backing、L0
页表或 PTE 安装任一步失败都回滚新资源和 VMA metadata；成功只对 fault VA 执行定点
`sfence.vma` 并重试原指令。instruction/权限/越界/冲突/非 stack fault 保持 terminal diagnostic。

usercopy 对合法栈范围使用同一 range resolver 逐页 fault-in；失败仍返回 `EFAULT`。完整 SIGSEGV、
通用 VMA fault core、COW、`setrlimit`、stack ASLR 和内核 compiler stack protector 不在本轮。
