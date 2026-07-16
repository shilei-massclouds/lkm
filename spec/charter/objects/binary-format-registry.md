# BinaryFormatRegistry

`BinaryFormatRegistry` 是 exec 格式分派的 Context-owned 注册表。它在 Initcall 阶段进入 `Ready`，维护固定
容量的 handler entry 表；首轮恰好注册一个 `ElfBinaryFormat` entry。

Registry 接收 kernel-owned image bytes 和 active `ExecTransaction`，按注册顺序调用 handler probe/load。
它不读取 user pointer，不拥有 address space，也不提交 exec。没有 handler 接受输入时返回 `ENOEXEC`。

`BinaryFormatHandler` 是 entry contract，`ElfBinaryFormat` 是首个实现类型，二者都不是新的生命周期对象。
shebang/script、binfmt_misc、动态注册和 module retry 保持 deferred；当前 `CONFIG_MODULES=n` 的 module
retry 是 trimmed/no-op。后续新增 handler 必须保持 registry 的 dispatch/errno/transaction ownership
边界，不得把格式搜索重新放回 syscall 或 `UserBootPayload`。
