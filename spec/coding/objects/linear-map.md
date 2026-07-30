# LinearMap Coding

`LinearMap` 必须由独立区域类型保存从体系结构 `PAGE_OFFSET` 开始的虚拟范围，并作为
`KernelAddrSpace` 的子区域参与边界、页对齐与不重叠检查。它不是页表或 translation controller。

`preset` 只提交区域已保留以及它与 `FixMap` 的布局关系。本段不得创建 RAM bank PTE、发布物理内存
可线性访问事实，或把 Ready 解释为完整线性映射已经建立；这些动作留给后续 `SwapperVm` 建立过程。
