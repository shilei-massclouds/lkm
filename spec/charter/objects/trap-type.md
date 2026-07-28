# TrapType

`TrapType` 是每个 CPU 独立拥有的公共陷入资源，不存在全局实例。它拥有同一 CPU 的
`InterruptType` 与 `ExceptionType`，保存公共入口服务状态、入口容量和紧急失败状态；具体原因和下级
handler 状态仍由对应子资源拥有。

服务 lifecycle 为受控早期入口 → 正式公共入口 → 服务在线。正式服务建立前的陷入只进入 fatal
prelude，不创建任何 Flow occurrence。正式入口必须先验证当前 CPU、入口上下文和实际剩余栈容量；
容量不足时切换到该 CPU 的 emergency stack 并 fail-stop，emergency stack 再入直接停机。

每次正式陷入以 `declare + Bind` 建立 fresh `TrapFlowType` occurrence。资源只接收、验证和绑定它，
不得缓存一个全局 Flow 或固定深度池。

## Mapping

- Model: `spec/model/objects/trap_type.spec`
- Coding: `spec/coding/objects/trap-type.md`
- Implementation: `impl/arceos_ex/src/objects/trap_type.rs`
