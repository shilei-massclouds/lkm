# LinearMap

`LinearMap` 是 `KernelAddrSpace` 拥有的物理内存线性映射虚拟区域，从体系结构规定的内核页偏移开始。
它描述的是内核地址空间中的区域身份和布局，不是独立页表或 translation controller。

`LinearMap.Preset` 建立并保留该区域的布局边界，使其它内核区域可以证明不会与它冲突。本入口前导
阶段尚未把物理内存映射到该区域；真正发布线性映射属于后续 `SwapperVm` 建立过程。

## Mapping

- Model: `spec/model/objects/linear_map.spec`
- Coding: `spec/coding/objects/linear-map.md`
- Implementation: `impl/arceos_ex/src/objects/linear_map.rs`
