# KernelImage

Model 的 `gp_relative_addressing_ready(KernelImage)` 降低为以下 RISC-V64 约束：

- 将 linker 提供的 `__global_pointer$` 符号地址装入 `gp/x3`。
- 用 `.option push`、`.option norelax` 和 `.option pop` 保护这段初始化，防止 linker relaxation
  把建立寻址基准的指令改写为依赖尚未建立的 `gp`。
- 如果执行环境变化会使原有基准失效，必须在下一次 `gp` 相对访问前重新建立它；直接从 linker symbol
  装载时仍须使用上述 `norelax` 保护，使用预先计算的地址时则必须先验证它对应同一 anchor。这只是
  保持同一项 Model 能力，不建立物理/虚拟两种对象状态。
- 所有实际生成的 `gp` 相对引用都必须有链接或构建检查证明其相对 `__global_pointer$` 可编码；不能从
  anchor 的存在推断整个 `.sdata`、`.sbss` 或其它 section 都由 `gp` 覆盖。

正式参考配置不启用 `CONFIG_SHADOW_CALL_STACK`，因此 `gp/x3` 可用于 global-pointer 约定。若配置改为
由 Shadow Call Stack 占用 `gp/x3`，必须重新决定本 Model 能力的 lowering，不能继续执行上述初始化。

当前 formal 非 XIP 路径的 `KernelImage.Setup` 必须由入口汇编为 BSS 段清零。XIP 路径不属于当前
formal lowering，记录为 deferred。

Mapping: charter [`kernel-image.md`](../../charter/objects/kernel-image.md), model
[`kernel_image.spec`](../../model/objects/kernel_image.spec), implementation
[`kernel_image.rs`](../../../impl/arceos_ex/src/objects/kernel_image.rs) and entry/translation assembly.
