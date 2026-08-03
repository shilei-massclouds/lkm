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
- fault 互斥分类为：合法 VMA 内缺 leaf 的 `NotPresent`、已有 leaf 但 access 不被 VMA/PTE
  允许的 `Protection`，以及不存在 VMA 的 `Unmapped`。COW 写保护在真实 COW 阶段提升为独立
  分类；在此之前只读 leaf 的 store 属于 `Protection`。
- `NotPresent` 成功结果只能是 `RetrySameInstruction`：保持原 `sepc`，安装 leaf 后对 fault VA
  执行定点 `sfence.vma`，由 trap return 重试原指令。分配、页表扩展或 leaf 安装任一步失败都
  保持原 VMA、PTE、backing 所有权和可见字节不变。
- `Protection` 与 `Unmapped` 必须形成稳定非法 fault 结果，不得被伪装为可恢复缺页；正式
  `SIGSEGV/SEGV_ACCERR/SEGV_MAPERR` task terminal delivery 在后续阶段闭合。
- 诊断在固定处理边界记录 Task/mm 身份、地址、`sepc`、access、分类、结果以及 COW 计数（首轮为
  零）。这些诊断不得插入、删除或重排既有外部 checkpoint。

文件后备缺页、page cache、`MAP_SHARED`、swap、页面迁移和 SMP 页表并发不属于当前对象范围。
kernel-origin exception-table fixup 属于 `PageFaultExceptionType` 的独立分支，不得读取或修改用户
VMA fault 状态。

## 普通 fork 的独立 mm

- 每个普通 fork parent/child Task 各自拥有一个 `UserAddressSpace` 身份、低半根页表和 SATP；高半
  仍只引用共享 `SwapperVm`。实现可以在调度切换期间把当前 Task 的 mm 暂存于唯一 active carrier，
  但 carrier 只是经 TaskRef 和 SATP 校验的借用位置，不能成为跨 Task 的全局地址空间所有者。
- 当前 eager 过渡阶段在 child 发布前复制每个已驻留私有 backing page、稀疏 VMA 元数据和用户栈
  backing，并用复制页建立 child 自己的 leaf PTE。父子初始字节相同，但 PFN、根页表和 SATP 均不同；
  未驻留 VMA 仍未驻留。任一 page 或页表分配失败必须释放全部未发布 child 资源，parent 的 PTE、
  backing、SATP 和可见字节保持不变。
- wait/schedule handoff 只切换 Task 所拥有的 mm 和 SATP，不得通过保存/恢复 parent writable page、
  stack 或整个 address-space 的字节快照实现隔离。trap 和 usercopy 必须按当前 TaskRef 解析并校验 active
  mm；错误 Task、错误 owner 或 stale SATP 不能访问 carrier。
- exec 在 point-of-no-return 原子替换当前 Task 的 mm；失败继续保留旧 mm，成功后旧 mm 只释放一次。
  exit 释放 exiting Task 的 leaf、页表和 backing，reap 只释放 Task record，不得再次释放同一 mm。
  fork 失败的 Task/mm 在可见发布前共同回滚，禁止留下可调度 child 或双重释放。

本阶段尚不共享普通 fork page。真实 COW 阶段会把 eager private-page copy 降低为 RO+COW 共享引用，
但不改变上述 per-Task mm ownership、切换、exec 或 teardown 边界。`CLONE_VM`、完整 vfork mm sharing
与 thread group 继续独立 deferred。

## Mapping

- Model: `spec/model/objects/user_boot.spec`
- Coding: `spec/coding/objects/user-boot.md`
- Implementation: `impl/arceos_ex/src/objects/user_boot.rs`
