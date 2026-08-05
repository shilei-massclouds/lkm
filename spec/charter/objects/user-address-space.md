# UserAddressSpace

`UserAddressSpace` 表示一个用户执行上下文可观察的低半地址空间、VMA 集合、页表和 SATP
身份。内核高半映射继续共享 `SwapperVm`，用户 leaf 必须设置 `U`，内核 leaf 不得设置 `U`。
本对象拥有统一用户缺页策略；`UserStack`、ELF loader、brk 和匿名 mmap 只提供 VMA 与 backing
操作，不得各自重新解释 fault。

## VMA 与首轮 fault core

- ELF segment、向下增长的用户栈、brk heap 和匿名 `MAP_PRIVATE` 都以不重叠 VMA 表示。
  VMA 独立保存范围以及 read/write/execute 权限；页是否已有 backing 不改变 VMA 是否存在。
- brk 新扩展区和匿名私有映射可以只有 VMA、没有 leaf。它们的首次合法 load/store fault 只为
  fault page 分配清零 backing 并安装 leaf，不预填跨越的中间页。
- 成功解除完整匿名 `MAP_PRIVATE` VMA 必须先撤销其所有 leaf，再释放 backing 引用和 VMA 槽位，
  并使该范围可被后续匿名映射复用；解除后访问该范围必须按无 VMA 分类。首轮不接受拆分 ELF、
  stack、brk 或匿名 VMA 的部分 `munmap`，拒绝不得改变既有 VMA、PTE 或 backing 所有权。
- 每次 user fault 请求必须绑定当前 Task、当前 `UserAddressSpace`/SATP、fault address、原始
  `sepc` 和 instruction/load/store access。错误 Task、错误 mm 或 stale SATP 不能消费请求。
- fault 互斥分类为：合法 VMA 内缺 leaf 的 `NotPresent`；已有 leaf、store access、VMA 原本可写且
  私有、PTE 为 RO+COW、backing 引用有效的 `CowWriteProtect`；已有 leaf 但 access 不被 VMA/PTE
  允许且不满足完整 COW 资格的 `Protection`；以及不存在 VMA 的 `Unmapped`。只读 leaf、NX leaf、
  非私有页、错误 Task/mm、stale PTE 或缺失 COW 标志不得进入 COW 分支。
- `NotPresent` 成功结果只能是 `RetrySameInstruction`：保持原 `sepc`，安装 leaf 后对 fault VA
  执行定点 `sfence.vma`，由 trap return 重试原指令。分配、页表扩展或 leaf 安装任一步失败都
  保持原 VMA、PTE、backing 所有权和可见字节不变。
- `CowWriteProtect` 在 refcount 大于一时分配并复制完整页，以新 frame/PTE/backing 引用原子替换
  当前 leaf 后释放旧引用；refcount 等于一时不复制，只清除 COW 并恢复原写权限。两条成功路径都只在
  定点 `sfence.vma` 后返回保持原 `sepc` 的 `RetrySameInstruction`。分配、复制或 commit 失败必须
  保持原 PTE、frame 引用和父子可见字节不变。
- 来自真实用户 instruction/load/store trap 的 `Unmapped` 必须形成同步致命
  `SIGSEGV/SEGV_MAPERR`，`Protection` 必须形成同步致命 `SIGSEGV/SEGV_ACCERR`。signal info 固定
  `signo=11`，`si_code` 与分类一致，`si_addr` 精确等于本次请求的 fault address；不得推进 `sepc`、
  伪装普通 exit 或进入 kernel exception-table fixup。错误 Task/mm、stale SATP 和其它
  `InvalidContext` 是内核上下文不变量失败，不得伪装成用户 SIGSEGV。
- 同步致命 SIGSEGV 首片只记录当前 Task terminal，并沿既有 child exit/wait/SIGCHLD 生命周期形成
  低 signal bits 为 11 的 wait word、唤醒 parent wait 和生成 SIGCHLD。它不构造用户 signal frame、
  不进入已登记 handler、不实现 `rt_sigreturn` 或 core dump。faulting Task 的 mm/PTE/`UserFrame`
  引用在 terminal handoff 释放一次，reap 只释放 Task record；parent 和 sibling mm 不受影响。
- 诊断在固定处理边界记录 Task/mm 身份、地址、`sepc`、access、分类、结果、fault 前后 frame
  refcount、复制次数和唯一引用快路径次数。这些诊断不得插入、删除或重排既有外部 checkpoint。
- COW 分配或 commit 资源失败产生 `TaskTerminalReason::OutOfMemory`：只终止当前 faulting Task，
  parent wait 按 signal 9 编码并产生 SIGCHLD；不得伪装成 SIGSEGV、普通 exit、系统 panic 或 OOM
  killer 选择。该 terminal 不构造用户 signal frame，也不进入已登记 handler。

文件后备缺页、page cache、`MAP_SHARED`、swap、页面迁移和 SMP 页表并发不属于当前对象范围。
kernel-origin exception-table fixup 属于 `PageFaultExceptionType` 的独立分支，不得读取或修改用户
VMA fault 状态。

## 普通 fork 的独立 mm

- 每个普通 fork parent/child Task 各自拥有一个 `UserAddressSpace` 身份、低半根页表和 SATP；高半
  仍只引用共享 `SwapperVm`。地址空间物理保存在对应 `UserProcessAggregate`，不得移入全局 active
  carrier 或通过交换整个对象来选择当前进程。trap、syscall 与 usercopy 只可经 generation/CPU-checked
  `UserProcessLease` 访问当前 mm。
- child 发布前先建立独立稀疏 VMA 元数据、页表和用户栈引用。所有可失败的 child 页表分配、leaf
  inventory 校验和共享引用 acquire 必须在 parent PTE commit 前完成；随后把双方原本可写的私有 leaf
  降低为 RO+COW，把原本只读 leaf 建立为共享只读非 COW，并刷新 parent 的受影响 TLB。parent/child
  初始 PFN 相同但根页表、SATP 和 mm identity 不同；未驻留 VMA 仍未驻留。任一步失败必须释放全部
  未发布 child 资源，parent 的 PTE、frame 引用、SATP 和可见字节保持不变。
- schedule commit 选择 next Task 所拥有的 mm，写 SATP 并执行本地 `sfence.vma`，不得通过保存/恢复
  parent writable page、stack 或整个 address-space 的字节快照实现隔离。trap 和 usercopy 必须按当前
  TaskRef/CPU lease 解析并校验 mm；错误 Task、错误 owner 或 stale SATP 不能访问。
- exec 在 point-of-no-return 原子替换当前 Task 的 mm；失败继续保留旧 mm，成功后旧 mm 只释放一次。
  exit 释放 exiting Task 的 leaf、页表和 backing，reap 只释放 Task record，不得再次释放同一 mm。
  fork 失败的 Task/mm 在可见发布前共同回滚，禁止留下可调度 child 或双重释放。

普通 fork 的共享 frame 不改变上述 per-Task mm ownership、切换、exec 或 teardown 边界。
vfork/首轮接受的 `CLONE_VM` 保持 parent-blocking 串行 handoff并固定父核；共享 mm 不会并发运行。
vfork child 在自己的 aggregate 中保留一个空闲且身份稳定的 `UserAddressSpace`/`UserStack` 对象，并以
generation-checked 直接链接解析仍存活的最终 mm owner；page fault、usercopy、brk、mmap、mprotect、
munmap 以及从该 child 发起的普通 fork 都必须先验证 current child、owner、同一 CpuRef 和 live SATP，
再对 owner mm 操作。成功 exec 把 staging image 安装到 child 的空闲对象，令 child 成为自己的 mm owner，
且不得 retire、释放或改写原 shared parent mm；exec 前 exit 也不得释放该 parent mm。链接中的 stale
generation、错误 CPU 或 parent 已非 live 必须在暴露 mm 地址之前失败。
完整 thread group 和共享 mm 跨 CPU 继续独立 deferred。

## Mapping

- Model: `spec/model/objects/user_boot.spec`
- Coding: `spec/coding/objects/user-boot.md`
- Implementation: `impl/arceos_ex/src/objects/user_boot.rs`
