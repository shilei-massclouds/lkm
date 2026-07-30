# ExceptionType

`ExceptionType` 是 `TrapType.exception` 的 CPU-local resident 分类资源。它拥有 page-fault、syscall、
breakpoint 和 unexpected 四个具体资源，先建立覆盖全部原因的 fatal fallback，再绑定正式分类，最后
使异常机制在线。

`ExceptionType.Preset` 由所属 `TrapType.Setup` 驱动，并继续驱动四个具体资源各自的 `Preset`。成功后
`ExceptionType` 与四个子资源进入 Prepared，表示当前已经支持的异常能力具有确定的初始兜底；它不
表示正式 handler、恢复机制或 syscall 服务已经就绪。当前尚未支持的分类、处理或恢复能力保留为
deferred obligation，不得用 Prepared 或 TrapType Ready 冒充已经完成。

每次异常创建 fresh `ExceptionFlowType`，再由分类结果同步创建一个具体异常 Flow。未覆盖、歧义或
过期引用必须 fail-and-shutdown，不得回退到全局资源或缓存副本。

## Mapping

- Model: `spec/model/objects/exception_type.spec`
- Coding: `spec/coding/objects/exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/exception_type.rs`
