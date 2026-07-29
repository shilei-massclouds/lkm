# KernelImage

`KernelImage` 表示固件已经装载到内存中的内核映像，不表示磁盘文件、ELF 文件或构建流水线产物。

入口前导期为 `KernelImage` 建立相对 `gp` 寻址基准，支持对内核映像中全局数据的快速访问机制。
该机制只表达当前执行环境中的全局数据访问能力；非特殊情况，不区分寻址发生在物理地址空间还是
虚拟地址空间。

`KernelImage.Setup` 负责处理映像的 BSS 段；其边界与完成事实由后续 BSS 专题校准。

## Mapping

- Model: `spec/model/objects/kernel_image.spec`
- Coding: `spec/coding/objects/kernel-image.md`
- Implementation: `impl/arceos_ex/src/objects/kernel_image.rs`
