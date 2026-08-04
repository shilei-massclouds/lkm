# User address-space and fault acceptance testing

本文件规定用户 VMA fault、进程地址空间、COW 和同步致命 SIGSEGV 的验收责任。核心语义来自
Charter、Model 和 Coding；Testing 只选择可复现观察、场景与门禁，不以测试实现反向定义语义。

## Common user-fault core

Rule IDs (MUST):

- `arceos_ex_must_user_fault_classes_be_exclusive_and_complete`
- `arceos_ex_must_user_fault_retry_preserve_sepc`
- `arceos_ex_must_user_fault_failure_preserve_mapping_state`
- `arceos_ex_must_user_fault_diagnostic_identify_task_mm_access_and_result`
- `arceos_ex_must_keep_kernel_extable_independent_from_user_fault_policy`

对象 smoke 必须对同一统一入口覆盖 `NotPresent`、`CowWriteProtect`、`Protection` 和 `Unmapped`。请求必须带当前 Task、mm/SATP、fault VA、`sepc` 与 read/write/execute access；测试
必须证明分类互斥，成功结果只有 `RetrySameInstruction`，且返回的 `sepc` 与请求完全相同。错误 Task/mm、
未知 access、权限不足和无 VMA 必须返回确定结果；分配或 PTE 安装注入失败不得留下 backing page、leaf
PTE 或页表页泄漏。稳定诊断必须可观察 Task/mm、VA、`sepc`、access、class、result 和 COW 计数。

真实 RISC-V user fixture 必须通过普通指令首次触及增长后的 stack、brk 和匿名 `MAP_PRIVATE` 页，验证
零页内容、写后读和 trap 后同指令完成。它不得调用测试专用缺页 API。native 与 Linux-object provider
运行同一 fixture；provider 差异不得改变用户输出和退出状态。

对象 smoke 还必须反复建立、触页并整段解除匿名私有 VMA，验证 leaf、backing 和 VMA 槽位回收，
解除后的 fault 分类为 `Unmapped`，且后续 mmap 可以复用地址与槽位。部分范围或非匿名 VMA 的首轮
`munmap` 拒绝必须保持页表、引用和 VMA 不变。真实 BusyBox/LTP 路径覆盖短生命周期 mmap/munmap
循环，不能以固定 VMA 表耗尽形成伪 `ENOMEM`。

内核 fault acceptance 继续单独覆盖正式 `__ex_table` fixup、nested/atomic terminal 路径。用户 VMA/COW
场景不得消费 extable state；内核 fixup 也不得读取用户 fault 分类、稀疏 backing 或 COW 诊断。

## Independent mm and COW closure

Rule IDs (MUST after the corresponding implementation phase):

- `arceos_ex_must_plain_fork_publish_independent_mm_atomically`
- `arceos_ex_must_cow_refcounts_equal_all_live_pte_references`
- `arceos_ex_must_cow_failure_leave_parent_and_child_unchanged`
- `arceos_ex_must_exec_exit_and_reap_release_mm_once`
- `arceos_ex_must_cow_oom_terminate_only_faulting_task_with_signal9_wait_word`

普通 fork 的对象与真实 guest 测试必须证明父子 mm identity、根页表和 SATP 不同；child 发布前的任一
dup_mm 分配失败都完整回滚，且不得保存/恢复父 writable page、stack 或 address-space 字节快照。双 provider 运行的同一真实 fixture 必须让 child 分别修改 ELF
可写 data/BSS、当前 stack、已触页匿名 `MAP_PRIVATE` 和增长后的 brk，再由 parent wait 验证退出状态、
全部父字节及父 brk 边界均保持不变；child exit/reap 后必须仍能继续执行并解除父匿名映射。

COW 阶段必须对已装入的 ELF 私有可写页、用户栈、brk 和匿名 `MAP_PRIVATE` 检查 fork 后同 PFN、双方
RO+COW 和引用加一。child 与 parent 写、refcount=1 快路径、嵌套 fork、child exec、父先退、子先退及
失败注入后，物理页引用数必须与所有 live PTE 精确相等。只读页可共享但不得获得 COW 写权限；stale
PTE、重复释放和错误 mm 必须确定失败。exec、exit、失败 fork 和 reap 各自只释放一次 mm/PTE/frame。
COW 分配失败注入还必须证明原 PTE、引用数与双方数据不变，只有 faulting Task 进入 OOM terminal，parent
得到 signal 9 wait word 与 SIGCHLD；不得出现 SIGSEGV、用户 signal frame、OOM killer 或系统 panic。

## Fatal SIGSEGV closure

Rule IDs (MUST after the corresponding implementation phase):

- `arceos_ex_must_user_unmapped_fault_exit_with_segv_maperr`
- `arceos_ex_must_user_protection_fault_exit_with_segv_accerr`
- `arceos_ex_must_user_segv_produce_wait_status_and_sigchld`

无 VMA fault 必须形成带 fault VA 的 `SIGSEGV/SEGV_MAPERR`；权限、NX 和非 COW 写保护必须形成
`SIGSEGV/SEGV_ACCERR`。首片只验收同步致命 delivery：task 以 signal 11 状态退出、wait 被唤醒并得到
对应 status、parent 产生 SIGCHLD；不得以推进 `sepc`、panic 或伪装普通 exit 代替。用户 handler
frame、`rt_sigreturn`、core dump 和 OOM killer 不在本闭环；COW 分配失败以独立 task OOM terminal
验收，不能伪装成 SIGSEGV。

## Gates

每阶段至少运行对象 smoke、两个 provider 的真实 user fixture、Model/tools2 验证和仓库根直接
`make test`。最终阶段再运行相关压力、paired difftest、格式、Clippy、coding-spec、Charter lock 和
`git diff --check`；测试运行不得遗留额外 tracked 修改。
