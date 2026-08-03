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

## Mapping

- Model: `spec/model/objects/user_boot.spec`
- Coding: `spec/coding/objects/user-boot.md`
- Implementation: `impl/arceos_ex/src/objects/user_boot.rs`
