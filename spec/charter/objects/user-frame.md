# UserFrame

`UserFrame` 表示由页分配器提供、当前作为用户低半地址空间 leaf backing 的一个物理页。页表页、内核栈、
slab backing 和其它内核私有页不属于本对象。`UserFrame` 的物理 identity 在最后一个引用释放前稳定；
每个 live user leaf 必须对应恰好一个 live frame 引用，反之每个 live frame 引用必须由一个已发布 leaf
或一个尚未发布事务的暂存 owner 持有。

## 共享引用生命周期

- 新分配 frame 以 refcount 1 建立，引用事实的唯一来源是 `PageMetadataMap`；单个
  `UserAddressSpace`、`UserStack` 或映射槽不得维护可与之分歧的第二份 authoritative count。
- acquire 必须验证 frame identity、用户 backing 类型、当前引用非零且计数不会溢出；release 必须验证
  调用者拥有对应引用并拒绝 stale、underflow 和重复释放。refcount 降为零时才把物理页归还页分配器，
  且只归还一次。
- refcount 等于一表示唯一 leaf owner，可以在合法 COW 写缺页中原地恢复写权限；大于一表示 frame
  仍共享，写入前必须建立私有副本。计数本身不授予读、写或执行权限，权限仍只来自 VMA 与 PTE。
- 普通 fork 只允许共享已驻留的私有用户页：原本可写页由双方 RO+COW leaf 持有，原本只读页由双方
  只读非 COW leaf 持有。未驻留 VMA 不创建 frame 引用，文件后备、`MAP_SHARED`、swap 和页面迁移
  不由本对象隐式引入。
- fork、COW fault、exec、munmap、exit 和失败回滚都必须使 refcount 与全部 live PTE 引用守恒。事务可
  暂存引用，但 child 发布或 fault commit 前失败必须释放全部暂存引用并保持既有 leaf、计数和可见字节
  不变。

## Mapping

- Model: `spec/model/objects/user_frame.spec`
- Coding: `spec/coding/objects/user-frame.md`
- Implementation: `impl/arceos_ex/src/objects/mm_core.rs`
