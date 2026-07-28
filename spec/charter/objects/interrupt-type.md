# InterruptType

`InterruptType` 是 `TrapType.interrupt` 的 CPU-local resident 资源。它直接拥有本 CPU 的总门控、分类
门控与 pending、handler binding，以及 save/disable/restore 动作；设备级 source gate 继续由 IRQ chip
资源拥有。

服务 lifecycle 为默认关闭和 fallback → handler 分派就绪 → 总入口开放。hardirq 执行期间禁止调度和
普通 IRQ 再入；只有 handler 明确重新开放的返回或 softirq 区间才允许嵌套。每次到达正式分派边界时
声明独立 `InterruptFlowType`，其 parent 固定为本资源。

## Mapping

- Model: `spec/model/objects/interrupt_type.spec`
- Coding: `spec/coding/objects/interrupt-type.md`
- Implementation: `impl/arceos_ex/src/objects/interrupt_type.rs`
