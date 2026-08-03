# 用户缺页、独立 mm、COW 与 SIGSEGV 执行边界

> 当前优先级、状态和下一步顺序只由 [`../ROADMAP.md`](../ROADMAP.md) 维护。本文保存已完成基线、
> 后续语义边界、执行顺序和验收清单，不维护第二套活跃优先级表。

## 已完成基线：第一与第二阶段

第一阶段已把普通用户缺页统一到 Task/mm-aware VMA fault core：请求显式携带 TaskRef、SATP、fault
address、`sepc` 和 access；成功分配/映射后执行目标 TLB flush 并返回同指令重试，kernel extable
分支保持隔离。

第二阶段在 COW 前建立语义完整的 eager 过渡基线：每个普通 fork child 拥有独立
`UserAddressSpace`、根页表、SATP、用户栈与已驻留私有 backing page；未驻留 VMA 继续保留 hole。
child 只在 mm、PTE 和 fd 准备全部成功后发布，失败会释放全部保留槽位和页而不改变 parent。调度、
trap 和 usercopy 按 TaskRef/owner/SATP 校验当前 carrier；exec 原子替换本 Task mm，exit 释放 mm，reap
只释放 Task record。普通 fork/wait/exit 不再保存或恢复 parent writable-page、stack-byte 或整个
address-space snapshot；vfork 的既有共享-VM 兼容片不由本阶段泛化。

真实双 provider fixture 要求 child 修改 ELF data/BSS、当前 stack、brk 和已触页匿名
`MAP_PRIVATE`，parent 在 wait 后验证退出状态、全部原字节和 brk 不变，并能继续 munmap。对象 smoke
另以 eager-copy 后故障注入验证 page、未发布 Task 槽、PID 分配和 parent SATP 的原子回滚。

## 第三阶段：真实 COW（最高优先级）

下一执行窗口必须 charter-first 从本阶段开始，不能以 eager copy 长期替代 COW，也不能恢复 snapshot
回退路径。

实现责任：

- 为用户物理 frame 建立受检查的共享引用生命周期；所有 PTE 引用总数必须与 frame refcount 一致，
  stale PTE、重复释放和错误 Task/mm 必须确定拒绝。
- 使用 RISC-V PTE software-reserved bit 表示 COW。只有“VMA 原本可写”且属于当前支持范围的私有页
  能标记 COW：已装入 ELF 私有可写 data/BSS、用户栈、brk 和匿名 `MAP_PRIVATE`。只读页可以共享，
  但绝不能因写 fault 获得写权限。
- fork 把 parent/child 对应私有可写 leaf 同时变为 RO+COW并增加 frame 引用；双方 mm、根页表和 SATP
  仍独立。PTE lowering 完成前 child 不可发布。
- store-page-fault 必须校验当前 Task/mm、VMA 原写权限、PTE COW 状态和 frame 引用。refcount 大于一时
  分配并复制完整页、原子替换 leaf 后减旧引用；等于一时走不复制快路径，清 COW 并恢复 W。成功只
  `sfence.vma` 后重试原 `sepc`。
- frame 分配或页表更新失败必须保持原 PTE、引用计数和父子可见字节不变；当前无 OOM killer，失败以
  明确 task OOM terminal 收口，不得伪装成 SIGSEGV。
- exec、exit、失败 fork 和 reap 覆盖 child 先退、parent 先退、嵌套 fork 和多轮复用，禁止悬空 PTE、
  泄漏引用和 double release。完成后关闭 Model deferred `user_clone.004`；thread group
  `user_clone.002` 与 `CLONE_VM`/完整 vfork `user_clone.003` 保持 deferred。

验收责任：

- Model/tools2 证明 fault 分类互斥完整、COW 资格、PTE/refcount 守恒、失败原子性、exec/exit/reap
  teardown，以及 kernel extable 与用户 COW 状态隔离。
- 真实 RISC-V 验证 fork 后同 PFN、双方 RO+COW；child 写获得私有页且 parent 不变，parent 后写覆盖
  refcount=1 快路径；覆盖 ELF data/BSS、stack、brk、匿名私有映射、嵌套 fork、child exec、两种退出
  顺序和多轮 fork/wait，退出后页/frame/page-table 计数回到基线。
- 对分配与 PTE commit 注入失败，逐项验证原 PTE、refcount、Task 槽、PID、parent/child 字节不变。

## 第四阶段：同步致命 SIGSEGV 与最终收口

只有第三阶段全部门禁通过并独立提交后才进入本阶段。

实现责任：

- 无 VMA 的用户 fault 生成携带 fault address 的 `SIGSEGV/SEGV_MAPERR`；VMA 权限、NX、只读写入和
  非 COW 写保护生成 `SIGSEGV/SEGV_ACCERR`。COW 成功仍只重试同一指令，不进入 signal 分支。
- 第一片只实现同步致命 delivery：形成 signal 11 退出状态、唤醒 wait 并生成 SIGCHLD；不构造用户
  signal frame，不进入登记 handler，不实现 `rt_sigreturn`、core dump 或 OOM killer。
- signal terminal 必须释放 faulting Task 的 mm/PTE/frame 引用并只释放一次，随后由 wait/reap 保持
  与普通 signal death 一致的状态。

最终验收责任：

- 真实 guest 覆盖只读写、NX execute 和 unmapped access，分别观察 ACCERR/MAPERR、`si_addr` 与
  wait status signal 11；覆盖 parent/child fault 后另一方数据与 mm 继续有效。
- native 与 Linux-object provider 运行相同程序，比较输出、父子隔离、退出和 wait 状态；只比较双方
  共有的外部 checkpoint，不伪造 Linux 内部 COW checkpoint。
- 运行 tools2 全套与完整模型推导、`make verify`、`make coding-spec-check`、格式/Clippy、相关
  user/rootfs/LTP smoke、默认四组压力各 10 次、额外多轮 fork/COW/exit 引用回收压力、
  `make difftest`、`git diff --check`，并以仓库根直接 `make test` 作为每次代码变更的最终门禁。

## 明确不扩展的边界

本专题不实现 thread group、`CLONE_VM`、完整 vfork mm sharing、用户 handler delivery、
`rt_sigreturn`、page cache、文件后备缺页、`MAP_SHARED`、swap、页面迁移、OOM killer、ASLR 扩展或
SMP 页表并发。上述责任不能被第三或第四阶段的实现便利隐式吸收。
